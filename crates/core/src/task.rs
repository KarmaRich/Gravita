//! 下载任务模型、筛选查询与本地 JSON 持久化仓库

use crate::{DownloadStatus, FileCategory};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 来自桌面 UI 或浏览器插件的新建下载请求
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewDownloadRequest {
    /// 目标下载 URL
    pub url: String,
    /// 可选自定义文件名（为空时从 URL 路径推断）
    #[serde(default)]
    pub filename: Option<String>,
    /// 可选自定义保存目录（为空时按文件分类自动匹配）
    #[serde(default)]
    pub save_dir: Option<PathBuf>,
    /// 需要透传给 aria2 的 HTTP 请求头（如 Cookie / Referer / User-Agent）
    #[serde(default)]
    pub headers: Vec<String>,
    /// 是否立即开始下载
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

fn default_true() -> bool {
    true
}

impl NewDownloadRequest {
    /// 推断最终文件名：优先使用显式传入的 filename，否则从 URL 末尾提取并进行 URL 解码
    pub fn inferred_filename(&self) -> String {
        if let Some(name) = self.filename.as_deref().filter(|s| !s.trim().is_empty()) {
            return percent_decode(name.trim());
        }
        let path_part = self.url.split('?').next().unwrap_or(&self.url);
        let raw = path_part
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("download.bin");
        percent_decode(raw)
    }
}

/// 轻量 URL 百分号解码（例如 `%E6%B5%8B%E8%AF%95.zip` -> `测试.zip`）
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

/// 桌面端展示与持久化的下载任务实体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadTask {
    pub id: String,
    /// 关联的 aria2 任务 GID
    pub gid: Option<String>,
    pub url: String,
    pub filename: String,
    pub save_dir: PathBuf,
    pub category: FileCategory,
    pub status: DownloadStatus,
    pub total_bytes: u64,
    pub completed_bytes: u64,
    pub speed_bytes_per_sec: u64,
    /// 当前活跃并发连接数
    #[serde(default)]
    pub connections: u32,
    /// 十六进制分片位图（来自 aria2 `bitfield`，用于绘制 IDM 风格分片区块图）
    #[serde(default)]
    pub bitfield: Option<String>,
    /// 分片总数
    #[serde(default)]
    pub num_pieces: u32,
    /// 单分片大小（字节）
    #[serde(default)]
    pub piece_length: u64,
    #[serde(default)]
    pub headers: Vec<String>,
    /// 下载失败时来自 aria2 的具体错误信息
    #[serde(default)]
    pub error_message: Option<String>,
    /// 任务创建时间（Unix 毫秒时间戳，用于列表按最新优先倒序排列）
    #[serde(default)]
    pub created_at_ms: u64,
}

impl DownloadTask {
    /// 返回本地文件的完整保存路径
    pub fn full_path(&self) -> PathBuf {
        self.save_dir.join(&self.filename)
    }

    /// 将十六进制 `bitfield` 解析为固定槽位数（如 32 格）的分片完成状态数组，供 UI 渲染 IDM 分片进度格
    pub fn piece_slots(&self, slot_count: usize) -> Vec<bool> {
        if slot_count == 0 {
            return Vec::new();
        }
        if self.status == DownloadStatus::Completed {
            return vec![true; slot_count];
        }
        let Some(hex) = self.bitfield.as_deref().filter(|s| !s.is_empty()) else {
            // 若 aria2 未返回 bitfield，则按总进度比例折算
            let ratio = if self.total_bytes > 0 {
                (self.completed_bytes as f64 / self.total_bytes as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let filled = (ratio * slot_count as f64).round() as usize;
            return (0..slot_count).map(|i| i < filled).collect();
        };

        let mut bits = Vec::with_capacity(hex.len() * 4);
        for ch in hex.chars() {
            if let Some(nibble) = ch.to_digit(16) {
                for shift in (0..4).rev() {
                    bits.push(((nibble >> shift) & 1) == 1);
                }
            }
        }
        let total_bits = if self.num_pieces > 0 {
            (self.num_pieces as usize).min(bits.len()).max(1)
        } else {
            bits.len().max(1)
        };
        bits.truncate(total_bits);

        (0..slot_count)
            .map(|slot| {
                let idx = (slot * total_bits / slot_count).min(bits.len().saturating_sub(1));
                bits.get(idx).copied().unwrap_or(false)
            })
            .collect()
    }
}

/// 任务列表筛选条件（对应 IDM 左侧分类树与顶部搜索框）
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub status: Option<DownloadStatus>,
    pub category: Option<FileCategory>,
    pub keyword: Option<String>,
}

/// 任务本地存储管理器
#[derive(Debug, Default)]
pub struct TaskStore {
    tasks: Vec<DownloadTask>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 新增或按 id 覆盖更新任务
    pub fn upsert(&mut self, task: DownloadTask) {
        if let Some(existing) = self.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            self.tasks.push(task);
        }
    }

    /// 按 id 删除任务，返回是否成功移除
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < before
    }

    /// 返回所有任务的只读切片
    pub fn all(&self) -> &[DownloadTask] {
        &self.tasks
    }

    /// 将来自 aria2 的实时任务列表合并进本地仓库，并返回是否有持久化关键状态变更
    pub fn merge_live_tasks(&mut self, live_tasks: Vec<DownloadTask>) -> bool {
        let mut changed = false;
        for live in live_tasks {
            if let Some(existing) = self.tasks.iter_mut().find(|t| t.id == live.id) {
                let status_changed = existing.status != live.status
                    || existing.filename != live.filename
                    || existing.total_bytes != live.total_bytes
                    || existing.error_message != live.error_message;
                let preserved_url = if live.url.is_empty()
                    || (!live.url.starts_with("http://")
                        && !live.url.starts_with("https://")
                        && !live.url.starts_with("magnet:?")
                        && !live.url.starts_with("ftp://"))
                {
                    existing.url.clone()
                } else {
                    live.url.clone()
                };
                let preserved_headers = if live.headers.is_empty() {
                    existing.headers.clone()
                } else {
                    live.headers.clone()
                };
                let preserved_created_at = if existing.created_at_ms > 0 {
                    existing.created_at_ms
                } else {
                    live.created_at_ms
                };
                *existing = DownloadTask {
                    url: preserved_url,
                    headers: preserved_headers,
                    created_at_ms: preserved_created_at,
                    ..live
                };
                if status_changed {
                    changed = true;
                }
            } else {
                let mut new_task = live;
                if new_task.created_at_ms == 0 {
                    new_task.created_at_ms = current_unix_millis();
                }
                self.tasks.push(new_task);
                changed = true;
            }
        }
        changed
    }

    /// 按条件筛选任务列表
    pub fn query(&self, filter: &TaskFilter) -> Vec<&DownloadTask> {
        self.tasks
            .iter()
            .filter(|t| filter.status.is_none_or(|s| t.status == s))
            .filter(|t| filter.category.is_none_or(|c| t.category == c))
            .filter(|t| {
                filter.keyword.as_deref().is_none_or(|kw| {
                    let kw = kw.to_ascii_lowercase();
                    t.filename.to_ascii_lowercase().contains(&kw)
                        || t.url.to_ascii_lowercase().contains(&kw)
                })
            })
            .collect()
    }

    /// 持久化任务列表到 JSON 文件
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.tasks)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// 从 JSON 文件加载任务列表
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let tasks = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Self { tasks })
    }
}

/// 获取当前 Unix 毫秒时间戳
pub fn current_unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inferred_filename_from_url() {
        let req = NewDownloadRequest {
            url: "https://example.com/files/setup_v1.exe?token=abc".into(),
            filename: None,
            save_dir: None,
            headers: vec![],
            auto_start: true,
        };
        assert_eq!(req.inferred_filename(), "setup_v1.exe");

        let cn_req = NewDownloadRequest {
            url: "https://example.com/%E6%B5%8B%E8%AF%95.zip".into(),
            filename: None,
            save_dir: None,
            headers: vec![],
            auto_start: true,
        };
        assert_eq!(cn_req.inferred_filename(), "测试.zip");
    }

    #[test]
    fn test_task_store_query_and_remove() {
        let mut store = TaskStore::new();
        store.upsert(DownloadTask {
            id: "t1".into(),
            gid: Some("g1".into()),
            url: "https://example.com/movie.mp4".into(),
            filename: "movie.mp4".into(),
            save_dir: PathBuf::from("Downloads/Video"),
            category: FileCategory::Video,
            status: DownloadStatus::Active,
            total_bytes: 1000,
            completed_bytes: 500,
            speed_bytes_per_sec: 100,
            connections: 4,
            bitfield: Some("f0".into()),
            num_pieces: 8,
            piece_length: 125,
            headers: vec![],
            error_message: None,
            created_at_ms: 1_700_000_000_000,
        });

        let matched = store.query(&TaskFilter {
            category: Some(FileCategory::Video),
            keyword: Some("MOVIE".into()),
            ..Default::default()
        });
        assert_eq!(matched.len(), 1);
        assert!(store.remove("t1"));
        assert!(store.query(&TaskFilter::default()).is_empty());
    }
}
