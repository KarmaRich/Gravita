//! 桌面端选项设置输入状态与系统目录选择/打开辅助模块

use gpui_kit::component::input::InputState;
use gpui_kit::*;
use idm_core::AppSettings;
use std::path::{Path, PathBuf};

/// 选项弹窗分类标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Connection,
    Network,
    Engine,
}

impl SettingsTab {
    pub fn all() -> [(Self, &'static str); 4] {
        [
            (Self::General, "常规与保存"),
            (Self::Connection, "连接与限速"),
            (Self::Network, "代理与网络"),
            (Self::Engine, "aria2 引擎"),
        ]
    }
}

/// 选项弹窗全量输入状态集合
pub struct SettingsInputs {
    pub download_dir: Entity<InputState>,
    pub max_concurrent: Entity<InputState>,
    pub allow_overwrite: bool,
    pub split: Entity<InputState>,
    pub max_conn_per_server: Entity<InputState>,
    pub min_split_size: Entity<InputState>,
    pub speed_limit: Entity<InputState>,
    pub upload_limit: Entity<InputState>,
    pub proxy: Entity<InputState>,
    pub user_agent: Entity<InputState>,
    pub timeout_secs: Entity<InputState>,
    pub max_tries: Entity<InputState>,
    pub rpc_secret: Entity<InputState>,
    pub disk_cache: Entity<InputState>,
    pub file_allocation: Entity<InputState>,
}

impl SettingsInputs {
    /// 根据当前配置初始化各输入框
    pub fn from_settings(settings: &AppSettings, window: &mut Window, cx: &mut App) -> Self {
        let mk = |val: String, ph: &'static str, window: &mut Window, cx: &mut App| {
            cx.new(|cx| InputState::new(window, cx).placeholder(ph).default_value(val))
        };

        Self {
            download_dir: mk(
                settings.default_download_dir.to_string_lossy().into_owned(),
                "默认下载保存目录",
                window,
                cx,
            ),
            max_concurrent: mk(
                settings.max_concurrent_downloads.to_string(),
                "1 ~ 16",
                window,
                cx,
            ),
            allow_overwrite: settings.allow_overwrite,
            split: mk(settings.default_split.to_string(), "1 ~ 64", window, cx),
            max_conn_per_server: mk(
                settings.max_connection_per_server.to_string(),
                "1 ~ 16",
                window,
                cx,
            ),
            min_split_size: mk(settings.min_split_size.clone(), "如 1M / 4M", window, cx),
            speed_limit: mk(
                settings.max_overall_download_limit.clone(),
                "0(不限) / 5M / 500K",
                window,
                cx,
            ),
            upload_limit: mk(
                settings.max_overall_upload_limit.clone(),
                "0(不限) / 1M",
                window,
                cx,
            ),
            proxy: mk(
                settings.proxy.clone().unwrap_or_default(),
                "如 http://127.0.0.1:7890（留空直连）",
                window,
                cx,
            ),
            user_agent: mk(
                settings.user_agent.clone().unwrap_or_default(),
                "留空使用默认 User-Agent",
                window,
                cx,
            ),
            timeout_secs: mk(settings.timeout_secs.to_string(), "秒，默认 60", window, cx),
            max_tries: mk(settings.max_tries.to_string(), "0 表示无限重试", window, cx),
            rpc_secret: mk(
                settings.aria2.secret.clone().unwrap_or_default(),
                "可选 RPC 鉴权密钥",
                window,
                cx,
            ),
            disk_cache: mk(settings.disk_cache.clone(), "如 16M / 32M", window, cx),
            file_allocation: mk(
                settings.file_allocation.clone(),
                "none / prealloc / falloc",
                window,
                cx,
            ),
        }
    }

    /// 读取用户输入并更新到 `AppSettings`
    pub fn apply_to(&self, settings: &mut AppSettings, cx: &App) {
        let val = |e: &Entity<InputState>| e.read(cx).value().trim().to_string();

        let dir = val(&self.download_dir);
        if !dir.is_empty() {
            settings.default_download_dir = PathBuf::from(dir);
        }
        if let Ok(v) = val(&self.max_concurrent).parse::<u32>() {
            settings.max_concurrent_downloads = v.clamp(1, 32);
        }
        settings.allow_overwrite = self.allow_overwrite;
        if let Ok(v) = val(&self.split).parse::<u32>() {
            settings.default_split = v.clamp(1, 64);
        }
        if let Ok(v) = val(&self.max_conn_per_server).parse::<u32>() {
            settings.max_connection_per_server = v.clamp(1, 16);
        }
        let min_split = val(&self.min_split_size);
        if !min_split.is_empty() {
            settings.min_split_size = min_split;
        }
        let dl = val(&self.speed_limit);
        settings.max_overall_download_limit = if dl.is_empty() { "0".into() } else { dl };
        let ul = val(&self.upload_limit);
        settings.max_overall_upload_limit = if ul.is_empty() { "0".into() } else { ul };

        let proxy = val(&self.proxy);
        settings.proxy = if proxy.is_empty() { None } else { Some(proxy) };
        let ua = val(&self.user_agent);
        settings.user_agent = if ua.is_empty() { None } else { Some(ua) };

        if let Ok(v) = val(&self.timeout_secs).parse::<u32>() {
            settings.timeout_secs = v.clamp(5, 600);
        }
        if let Ok(v) = val(&self.max_tries).parse::<u32>() {
            settings.max_tries = v;
        }
        let secret = val(&self.rpc_secret);
        settings.aria2.secret = if secret.is_empty() { None } else { Some(secret) };
        let cache = val(&self.disk_cache);
        if !cache.is_empty() {
            settings.disk_cache = cache;
        }
        let alloc = val(&self.file_allocation);
        if !alloc.is_empty() {
            settings.file_allocation = alloc;
        }
    }
}

/// 弹出系统原生目录选择对话框
pub fn pick_directory(initial: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(dir) = initial.filter(|p| p.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder()
}

/// 在系统文件管理器中打开指定目录（Windows 使用 `explorer`）
pub fn open_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}
