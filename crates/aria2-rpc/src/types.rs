//! aria2 JSON-RPC 2.0 业务数据类型定义

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// aria2 版本信息
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Aria2Version {
    pub version: String,
    #[serde(default)]
    pub enabled_features: Vec<String>,
}

/// 添加下载任务时的可选参数（对应 aria2.addUri 的 options 对象）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddUriOptions {
    /// 保存目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// 自定义保存文件名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    /// 单任务分片数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<String>,
    /// 单服务器最大连接数
    #[serde(rename = "max-connection-per-server", skip_serializing_if = "Option::is_none")]
    pub max_connection_per_server: Option<String>,
    /// 最小分片大小（如 "1M"）
    #[serde(rename = "min-split-size", skip_serializing_if = "Option::is_none")]
    pub min_split_size: Option<String>,
    /// 自定义 HTTP 请求头（如 Cookie / Referer / User-Agent）
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub header: Vec<String>,
    /// 代理地址（如 `http://127.0.0.1:7890`）
    #[serde(rename = "all-proxy", skip_serializing_if = "Option::is_none")]
    pub all_proxy: Option<String>,
    /// 单任务限速（如 "2M"、"500K"、"0" 表示不限速）
    #[serde(rename = "max-download-limit", skip_serializing_if = "Option::is_none")]
    pub max_download_limit: Option<String>,
    /// 是否允许覆盖已存在的同名文件（"true" / "false"）
    #[serde(rename = "allow-overwrite", skip_serializing_if = "Option::is_none")]
    pub allow_overwrite: Option<String>,
    /// 同名文件自动重命名（"true" / "false"，换链续传或允许覆盖时需设为 "false"）
    #[serde(rename = "auto-file-renaming", skip_serializing_if = "Option::is_none")]
    pub auto_file_renaming: Option<String>,
    /// 是否启用断点续传（"true" / "false"）
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_download: Option<String>,
}

/// 任务关联的单个 URI 信息
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Aria2Uri {
    pub uri: String,
    #[serde(default)]
    pub status: String,
}

/// 任务关联的单个文件信息
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Aria2File {
    pub index: String,
    pub path: String,
    pub length: String,
    pub completed_length: String,
    #[serde(default)]
    pub uris: Vec<Aria2Uri>,
}

/// aria2 任务状态详情（对应 tellStatus / tellActive / tellWaiting / tellStopped）
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Aria2TaskStatus {
    pub gid: String,
    pub status: String,
    #[serde(default)]
    pub total_length: String,
    #[serde(default)]
    pub completed_length: String,
    #[serde(default)]
    pub download_speed: String,
    #[serde(default)]
    pub upload_speed: String,
    #[serde(default)]
    pub connections: String,
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub files: Vec<Aria2File>,
    /// 十六进制分片位图（用于可视化分片进度条）
    #[serde(default)]
    pub bitfield: Option<String>,
    /// 分片总数
    #[serde(default)]
    pub num_pieces: Option<String>,
    /// 单个分片字节大小
    #[serde(default)]
    pub piece_length: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

impl Aria2TaskStatus {
    /// 解析总字节数
    pub fn total_length_u64(&self) -> u64 {
        self.total_length.parse().unwrap_or(0)
    }

    /// 解析已完成字节数
    pub fn completed_length_u64(&self) -> u64 {
        self.completed_length.parse().unwrap_or(0)
    }

    /// 解析当前下载速度（字节/秒）
    pub fn download_speed_u64(&self) -> u64 {
        self.download_speed.parse().unwrap_or(0)
    }

    /// 解析当前连接数
    pub fn connections_u32(&self) -> u32 {
        self.connections.parse().unwrap_or(0)
    }

    /// 解析分片总数
    pub fn num_pieces_u32(&self) -> u32 {
        self.num_pieces
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// 解析单分片大小（字节）
    pub fn piece_length_u64(&self) -> u64 {
        self.piece_length
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// 计算下载进度百分比（0.0 ~ 100.0）
    pub fn progress_percent(&self) -> f64 {
        let total = self.total_length_u64();
        if total == 0 {
            return 0.0;
        }
        (self.completed_length_u64() as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

/// aria2 全局统计信息（对应 aria2.getGlobalStat）
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalStat {
    pub download_speed: String,
    pub upload_speed: String,
    pub num_active: String,
    pub num_waiting: String,
    pub num_stopped: String,
}

/// 全局配置项键值对（对应 aria2.changeGlobalOption）
pub type GlobalOptions = BTreeMap<String, String>;
