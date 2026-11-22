//! 远程下载链接元数据预检模块（仿 IDM 新建下载弹窗自动探测文件大小、真实文件名与断点续传支持）

use crate::FileCategory;
use crate::task::NewDownloadRequest;
use std::time::Duration;

/// 远程 URL 预检结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlProbeInfo {
    /// 最终重定向后的有效 URL
    pub final_url: String,
    /// 探测出的真实文件名（优先 `Content-Disposition`，其次重定向后 URL 路径）
    pub filename: String,
    /// 文件总大小（字节，`None` 表示服务器未返回 `Content-Length`）
    pub total_bytes: Option<u64>,
    /// 服务器是否支持 `Accept-Ranges: bytes` 或 `206 Partial Content` 多线程分片下载
    pub resumable: bool,
    /// 推断的文件分类
    pub category: FileCategory,
}

/// 异步探测目标下载链接的文件大小、文件名与多线程支持能力
pub async fn probe_download_url(
    url: &str,
    headers: &[String],
    proxy: Option<&str>,
) -> Result<UrlProbeInfo, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL 不能为空".into());
    }
    if trimmed.starts_with("magnet:?") {
        let filename = NewDownloadRequest {
            url: trimmed.to_string(),
            filename: None,
            save_dir: None,
            headers: vec![],
            auto_start: true,
        }
        .inferred_filename();
        return Ok(UrlProbeInfo {
            final_url: trimmed.to_string(),
            category: FileCategory::from_filename(&filename),
            filename,
            total_bytes: None,
            resumable: true,
        });
    }

    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .connect_timeout(Duration::from_secs(4));

    if let Some(p) = proxy.filter(|s| !s.trim().is_empty()) {
        if let Ok(px) = reqwest::Proxy::all(p) {
            builder = builder.proxy(px);
        }
    }

    let client = builder.build().map_err(|e| e.to_string())?;

    // 优先发送带 Range: bytes=0-0 的 GET 请求（兼容性远好于 HEAD，许多防盗链网盘/签名直链拒绝 HEAD）
    let mut req = client.get(trimmed).header("Range", "bytes=0-0");
    for h in headers {
        if let Some((k, v)) = h.split_once(':') {
            req = req.header(k.trim(), v.trim());
        }
    }

    let resp = req.send().await.map_err(|e| format!("探测失败: {e}"))?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    let resp_headers = resp.headers();

    let resumable = status.as_u16() == 206
        || resp_headers
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("bytes"));

    // 解析文件总大小：优先从 Content-Range: bytes 0-0/123456 提取，其次从 Content-Length 提取
    let total_bytes = resp_headers
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range_total)
        .or_else(|| {
            if status.as_u16() != 206 {
                resp_headers
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok())
            } else {
                None
            }
        });

    let disposition_name = resp_headers
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_disposition_filename);

    let filename = disposition_name.unwrap_or_else(|| {
        NewDownloadRequest {
            url: final_url.clone(),
            filename: None,
            save_dir: None,
            headers: vec![],
            auto_start: true,
        }
        .inferred_filename()
    });

    let category = FileCategory::from_filename(&filename);

    Ok(UrlProbeInfo {
        final_url,
        filename,
        total_bytes,
        resumable,
        category,
    })
}

/// 从 `Content-Range: bytes 0-0/1048576` 中提取总字节数 `1048576`
pub fn parse_content_range_total(header_val: &str) -> Option<u64> {
    let (_, total_str) = header_val.rsplit_once('/')?;
    let trimmed = total_str.trim();
    if trimmed == "*" {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// 从 `Content-Disposition` 响应头中解析文件名（支持 `filename*=UTF-8''...` 与 `filename="..."`）
pub fn parse_content_disposition_filename(header_val: &str) -> Option<String> {
    for part in header_val.split(';') {
        let trimmed = part.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("filename*=") {
            let raw = &trimmed["filename*=".len()..];
            let cleaned = raw
                .trim_matches('"')
                .strip_prefix("UTF-8''")
                .or_else(|| raw.trim_matches('"').strip_prefix("utf-8''"))
                .unwrap_or(raw.trim_matches('"'));
            let req = NewDownloadRequest {
                url: format!("https://example.com/{cleaned}"),
                filename: None,
                save_dir: None,
                headers: vec![],
                auto_start: true,
            };
            let decoded = req.inferred_filename();
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }
    for part in header_val.split(';') {
        let trimmed = part.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("filename=") {
            let raw = trimmed["filename=".len()..].trim_matches('"').trim_matches('\'');
            let req = NewDownloadRequest {
                url: raw.to_string(),
                filename: Some(raw.to_string()),
                save_dir: None,
                headers: vec![],
                auto_start: true,
            };
            let decoded = req.inferred_filename();
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_range_total() {
        assert_eq!(parse_content_range_total("bytes 0-0/5242880"), Some(5242880));
        assert_eq!(parse_content_range_total("bytes 0-1023/*"), None);
    }

    #[test]
    fn test_parse_content_disposition_filename() {
        assert_eq!(
            parse_content_disposition_filename("attachment; filename=\"setup_v2.exe\""),
            Some("setup_v2.exe".into())
        );
        assert_eq!(
            parse_content_disposition_filename(
                "attachment; filename=\"fallback.zip\"; filename*=UTF-8''%E6%B5%8B%E8%AF%95%E5%8C%85.zip"
            ),
            Some("测试包.zip".into())
        );
    }
}
