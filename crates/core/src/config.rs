//! 应用配置与分类目录映射持久化模块

use crate::FileCategory;
use aria2_rpc::Aria2Config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 桌面端应用全局设置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// 默认下载根目录
    pub default_download_dir: PathBuf,
    /// 各文件分类对应的子目录名
    pub category_subdirs: BTreeMap<String, String>,
    /// 单任务默认分片数
    pub default_split: u32,
    /// aria2 RPC 连接配置
    pub aria2: Aria2Config,
    /// 供浏览器插件连接的本地 HTTP 接收服务端口
    pub bridge_port: u16,
    /// 可选的本地接收服务鉴权 Token
    pub bridge_token: Option<String>,
    /// 全局下载限速（如 "0" 表示不限速，"5M" 表示 5MB/s）
    #[serde(default = "default_zero_limit")]
    pub max_overall_download_limit: String,
    /// 可选 HTTP/SOCKS 代理地址（如 "http://127.0.0.1:7890"）
    #[serde(default)]
    pub proxy: Option<String>,
}

fn default_zero_limit() -> String {
    "0".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut category_subdirs = BTreeMap::new();
        category_subdirs.insert("Compressed".into(), "Compressed".into());
        category_subdirs.insert("Document".into(), "Documents".into());
        category_subdirs.insert("Music".into(), "Music".into());
        category_subdirs.insert("Program".into(), "Programs".into());
        category_subdirs.insert("Video".into(), "Video".into());
        category_subdirs.insert("Other".into(), "Other".into());

        Self {
            default_download_dir: PathBuf::from("Downloads"),
            category_subdirs,
            default_split: 16,
            aria2: Aria2Config::default(),
            bridge_port: 16800,
            bridge_token: None,
            max_overall_download_limit: "0".into(),
            proxy: None,
        }
    }
}

impl AppSettings {
    /// 返回默认配置文件路径（优先用户主目录下 `.idm-aria2-rs/settings.json`）
    pub fn default_config_path() -> PathBuf {
        let base = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(".idm-aria2-rs").join("settings.json")
    }
    /// 根据文件分类解析最终保存路径
    pub fn resolve_save_dir(&self, category: FileCategory) -> PathBuf {
        let key = format!("{category:?}");
        match self.category_subdirs.get(&key) {
            Some(sub) if !sub.is_empty() => self.default_download_dir.join(sub),
            _ => self.default_download_dir.clone(),
        }
    }

    /// 将配置序列化并写入 JSON 文件
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// 从 JSON 文件读取配置
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_save_dir_by_category() {
        let settings = AppSettings::default();
        assert_eq!(
            settings.resolve_save_dir(FileCategory::Video),
            PathBuf::from("Downloads").join("Video")
        );
        assert_eq!(
            settings.resolve_save_dir(FileCategory::Document),
            PathBuf::from("Downloads").join("Documents")
        );
    }

    #[test]
    fn test_settings_roundtrip_persistence() {
        let mut settings = AppSettings::default();
        settings.bridge_port = 17890;
        settings.bridge_token = Some("local-secret".into());

        let tmp = std::env::temp_dir().join("idm_aria2_rs_test_settings.json");
        settings.save_to_file(&tmp).unwrap();
        let loaded = AppSettings::load_from_file(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(loaded, settings);
    }
}
