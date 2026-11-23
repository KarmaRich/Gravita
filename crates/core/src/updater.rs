//! GitHub Releases 自动更新检测模块（支持直连与预设代理自动回退、SemVer 比对及跨平台安装包匹配）

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 当前编译的客户端版本号（来自 Cargo.toml）
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 项目官方 GitHub 仓库主页
pub const GITHUB_REPO_URL: &str = "https://github.com/KarmaRich/Gravita";

/// GitHub Releases 最新版本查询 API
pub const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/KarmaRich/Gravita/releases/latest";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseResponse {
    pub tag_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub body: String,
    pub html_url: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

/// 检测到新版本时的结构化更新信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_name: String,
    pub release_notes: String,
    pub html_url: String,
    pub published_at: String,
    /// 自动匹配的当前操作系统安装包下载直链
    pub download_url: Option<String>,
    /// 自动匹配的安装包文件名
    pub asset_name: Option<String>,
}

/// 解析版本号字符串（支持带 `v`/`V` 前缀或 `-beta` 后缀，如 `"v0.2.1"` -> `Some((0, 2, 1))`）
pub fn parse_semver(ver: &str) -> Option<(u32, u32, u32)> {
    let cleaned = ver.trim().trim_start_matches(['v', 'V']);
    let core = cleaned.split(['-', '+']).next().unwrap_or(cleaned);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// 判断远端版本 `latest` 是否比本地版本 `current` 更新
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => false,
    }
}

/// 根据当前编译的目标操作系统，从 Release Assets 中挑选最合适的安装包
pub fn select_platform_asset(assets: &[GitHubAsset]) -> Option<(String, String)> {
    let find_matching = |pred: fn(&str) -> bool| -> Option<(String, String)> {
        assets
            .iter()
            .find(|a| pred(&a.name.to_ascii_lowercase()))
            .map(|a| (a.name.clone(), a.browser_download_url.clone()))
    };

    if cfg!(target_os = "windows") {
        find_matching(|n| n.ends_with("-setup.exe") || n.ends_with(".exe"))
            .or_else(|| find_matching(|n| n.contains("windows") && n.ends_with(".zip")))
    } else if cfg!(target_os = "macos") {
        find_matching(|n| n.ends_with(".dmg"))
    } else {
        find_matching(|n| n.ends_with(".deb"))
            .or_else(|| find_matching(|n| n.contains("linux") && n.ends_with(".tar.gz")))
    }
}

async fn fetch_release_with_proxy(proxy: Option<&str>) -> Result<GitHubReleaseResponse, String> {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(5));

    if let Some(p) = proxy.map(str::trim).filter(|s| !s.is_empty()) {
        let px = reqwest::Proxy::all(p).map_err(|e| e.to_string())?;
        builder = builder.proxy(px);
    }

    let client = builder.build().map_err(|e| e.to_string())?;
    let resp = client
        .get(GITHUB_LATEST_RELEASE_API)
        .header("User-Agent", format!("Gravita/{CURRENT_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub Release 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回状态码 {}", resp.status()));
    }

    resp.json::<GitHubReleaseResponse>()
        .await
        .map_err(|e| format!("解析 Release 响应失败: {e}"))
}

/// 检查 GitHub 最新 Release：优先直连，若直连失败且配置了预设代理则自动通过代理重试
pub async fn check_github_update(configured_proxy: Option<&str>) -> Result<Option<UpdateInfo>, String> {
    let release = match fetch_release_with_proxy(None).await {
        Ok(r) => r,
        Err(first_err) => {
            if let Some(px) = configured_proxy.map(str::trim).filter(|s| !s.is_empty()) {
                fetch_release_with_proxy(Some(px)).await?
            } else {
                return Err(first_err);
            }
        }
    };

    if !is_newer_version(CURRENT_VERSION, &release.tag_name) {
        return Ok(None);
    }

    let matched = select_platform_asset(&release.assets);
    let (asset_name, download_url) = match matched {
        Some((name, url)) => (Some(name), Some(url)),
        None => (None, None),
    };

    let release_name = if release.name.trim().is_empty() {
        format!("Gravita {}", release.tag_name)
    } else {
        release.name
    };
    let notes = if release.body.trim().is_empty() {
        "该版本未填写详细更新说明，可前往 GitHub Release 页面查看提交记录。".to_string()
    } else {
        release.body
    };
    let pub_date = release
        .published_at
        .split('T')
        .next()
        .unwrap_or(&release.published_at)
        .to_string();

    Ok(Some(UpdateInfo {
        current_version: format!("v{CURRENT_VERSION}"),
        latest_version: release.tag_name,
        release_name,
        release_notes: notes,
        html_url: release.html_url,
        published_at: pub_date,
        download_url,
        asset_name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparison() {
        assert_eq!(parse_semver("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver("1.2.3-beta.1"), Some((1, 2, 3)));
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "v0.2.0"));
        assert!(!is_newer_version("0.1.0", "v0.1.0"));
        assert!(!is_newer_version("0.2.0", "v0.1.9"));
    }

    #[test]
    fn test_select_platform_asset() {
        let assets = vec![
            GitHubAsset {
                name: "Gravita-v0.2.0-windows-x86_64-setup.exe".into(),
                browser_download_url: "https://example.com/setup.exe".into(),
            },
            GitHubAsset {
                name: "Gravita-v0.2.0-macos-arm64.dmg".into(),
                browser_download_url: "https://example.com/app.dmg".into(),
            },
            GitHubAsset {
                name: "gravita_0.2.0_amd64.deb".into(),
                browser_download_url: "https://example.com/app.deb".into(),
            },
        ];
        let matched = select_platform_asset(&assets);
        assert!(matched.is_some());
    }
}
