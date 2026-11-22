//! IDM-Aria2-RS 桌面客户端入口（基于 Rust + GPUI + gpui-kit）

use aria2_rpc::Aria2Config;
use idm_core::FileCategory;

/// 桌面端启动配置
#[derive(Debug, Clone)]
pub struct AppBootstrapConfig {
    /// 应用窗口标题
    pub title: String,
    /// aria2 RPC 配置
    pub aria2: Aria2Config,
}

impl Default for AppBootstrapConfig {
    fn default() -> Self {
        Self {
            title: "IDM-Aria2-RS 下载器".to_string(),
            aria2: Aria2Config::default(),
        }
    }
}

fn main() {
    let config = AppBootstrapConfig::default();
    let sample_category = FileCategory::from_filename("demo.zip");
    println!(
        "{} 启动骨架就绪 (aria2={}, 示例分类={})",
        config.title,
        config.aria2.endpoint,
        sample_category.display_name()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bootstrap_config() {
        let cfg = AppBootstrapConfig::default();
        assert_eq!(cfg.title, "IDM-Aria2-RS 下载器");
        assert_eq!(cfg.aria2.endpoint, "http://127.0.0.1:6800/jsonrpc");
    }
}
