//! IDM-Aria2-RS 核心领域模型、任务管理、配置持久化与本地插件桥接服务

pub mod aria2_opts;
pub mod batch;
pub mod bridge;
pub mod config;
pub mod probe;
pub mod task;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use bridge::BridgeServer;
pub use config::AppSettings;
pub use probe::{UrlProbeInfo, probe_download_url};
pub use task::{DownloadTask, NewDownloadRequest, TaskFilter, TaskStore};

/// 下载任务状态（与 IDM 视图分类对应）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    /// 正在下载
    Active,
    /// 等待队列中
    Waiting,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 下载出错
    Error,
}

/// 文件自动分类枚举（对应 IDM 左侧分类树）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileCategory {
    Compressed,
    Document,
    Music,
    Program,
    Video,
    Other,
}

impl FileCategory {
    /// 根据文件名扩展名自动推断所属分类
    pub fn from_filename(filename: &str) -> Self {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        match ext.as_deref() {
            Some("zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz") => Self::Compressed,
            Some("pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md") => {
                Self::Document
            }
            Some("mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a") => Self::Music,
            Some("exe" | "msi" | "dmg" | "pkg" | "deb" | "rpm" | "apk") => Self::Program,
            Some("mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm") => Self::Video,
            _ => Self::Other,
        }
    }

    /// 返回用于界面展示的中文分类名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Compressed => "压缩文件",
            Self::Document => "文档",
            Self::Music => "音乐",
            Self::Program => "程序",
            Self::Video => "视频",
            Self::Other => "其他",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_category_inference() {
        assert_eq!(FileCategory::from_filename("archive.7z"), FileCategory::Compressed);
        assert_eq!(FileCategory::from_filename("REPORT.PDF"), FileCategory::Document);
        assert_eq!(FileCategory::from_filename("song.flac"), FileCategory::Music);
        assert_eq!(FileCategory::from_filename("installer.exe"), FileCategory::Program);
        assert_eq!(FileCategory::from_filename("movie.mkv"), FileCategory::Video);
        assert_eq!(FileCategory::from_filename("README"), FileCategory::Other);
    }

    #[test]
    fn test_file_category_display_name() {
        assert_eq!(FileCategory::Compressed.display_name(), "压缩文件");
        assert_eq!(FileCategory::Video.display_name(), "视频");
    }
}
