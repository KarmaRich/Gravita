//! 桌面端 5 分类选项设置输入状态与系统目录选择/打开辅助模块

use gpui_kit::component::input::InputState;
use gpui_kit::*;
use idm_core::AppSettings;
use std::path::{Path, PathBuf};

/// 选项弹窗 5 大分类标签页（严格对齐 ARIA2_OPTIONS.md）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Connection,
    HttpProxy,
    BitTorrent,
    EngineSession,
}

impl SettingsTab {
    pub fn all() -> [(Self, &'static str); 5] {
        [
            (Self::General, "常规与保存"),
            (Self::Connection, "连接与限速"),
            (Self::HttpProxy, "HTTP 与代理"),
            (Self::BitTorrent, "BT 与磁力"),
            (Self::EngineSession, "引擎与会话"),
        ]
    }
}

/// 选项弹窗布尔开关枚举（用于统一点击切换）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFlag {
    ContinueDownload,
    AllowOverwrite,
    AutoFileRenaming,
    RemoteTime,
    CheckCertificate,
    HttpAcceptGzip,
    EnableDht,
    EnablePeerExchange,
    BtSaveMetadata,
    EnableSessionPersistence,
    DisableIpv6,
    EnableNotifications,
    AutoStart,
}

/// 选项弹窗全量输入状态集合
pub struct SettingsInputs {
    // 1. 常规与保存
    pub download_dir: Entity<InputState>,
    pub file_allocation: Entity<InputState>,
    pub disk_cache: Entity<InputState>,
    pub continue_download: bool,
    pub allow_overwrite: bool,
    pub auto_file_renaming: bool,
    pub remote_time: bool,
    pub enable_notifications: bool,
    pub auto_start: bool,
    // 2. 连接与限速
    pub max_concurrent: Entity<InputState>,
    pub split: Entity<InputState>,
    pub max_conn_per_server: Entity<InputState>,
    pub min_split_size: Entity<InputState>,
    pub speed_limit: Entity<InputState>,
    pub upload_limit: Entity<InputState>,
    pub lowest_speed_limit: Entity<InputState>,
    // 3. HTTP 与代理
    pub proxy: Entity<InputState>,
    pub no_proxy: Entity<InputState>,
    pub user_agent: Entity<InputState>,
    pub connect_timeout: Entity<InputState>,
    pub timeout_secs: Entity<InputState>,
    pub max_tries: Entity<InputState>,
    pub retry_wait: Entity<InputState>,
    pub check_certificate: bool,
    pub http_accept_gzip: bool,
    // 4. BT 与磁力
    pub bt_max_peers: Entity<InputState>,
    pub seed_ratio: Entity<InputState>,
    pub seed_time: Entity<InputState>,
    pub bt_tracker: Entity<InputState>,
    pub enable_dht: bool,
    pub enable_peer_exchange: bool,
    pub bt_save_metadata: bool,
    // 5. 引擎与会话
    pub rpc_secret: Entity<InputState>,
    pub save_session_interval: Entity<InputState>,
    pub enable_session_persistence: bool,
    pub disable_ipv6: bool,
}

impl SettingsInputs {
    pub fn from_settings(s: &AppSettings, window: &mut Window, cx: &mut App) -> Self {
        let mk = |val: String, ph: &'static str, window: &mut Window, cx: &mut App| {
            cx.new(|cx| InputState::new(window, cx).placeholder(ph).default_value(val))
        };
        Self {
            download_dir: mk(s.default_download_dir.to_string_lossy().into_owned(), "默认下载目录", window, cx),
            file_allocation: mk(s.file_allocation.clone(), "none / prealloc / falloc", window, cx),
            disk_cache: mk(s.disk_cache.clone(), "如 16M / 32M", window, cx),
            continue_download: s.continue_download,
            allow_overwrite: s.allow_overwrite,
            auto_file_renaming: s.auto_file_renaming,
            remote_time: s.remote_time,
            enable_notifications: s.enable_notifications,
            auto_start: s.auto_start,
            max_concurrent: mk(s.max_concurrent_downloads.to_string(), "1 ~ 32", window, cx),
            split: mk(s.default_split.to_string(), "1 ~ 64", window, cx),
            max_conn_per_server: mk(s.max_connection_per_server.to_string(), "1 ~ 16", window, cx),
            min_split_size: mk(s.min_split_size.clone(), "如 1M / 4M", window, cx),
            speed_limit: mk(s.max_overall_download_limit.clone(), "0(不限) / 5M", window, cx),
            upload_limit: mk(s.max_overall_upload_limit.clone(), "0(不限) / 1M", window, cx),
            lowest_speed_limit: mk(s.lowest_speed_limit.clone(), "0(禁用) / 50K", window, cx),
            proxy: mk(s.proxy.clone().unwrap_or_default(), "如 http://127.0.0.1:7890", window, cx),
            no_proxy: mk(s.no_proxy.clone().unwrap_or_default(), "逗号分隔域名白名单", window, cx),
            user_agent: mk(s.user_agent.clone().unwrap_or_default(), "留空使用默认 UA", window, cx),
            connect_timeout: mk(s.connect_timeout_secs.to_string(), "秒，默认 60", window, cx),
            timeout_secs: mk(s.timeout_secs.to_string(), "秒，默认 60", window, cx),
            max_tries: mk(s.max_tries.to_string(), "0 表示无限重试", window, cx),
            retry_wait: mk(s.retry_wait_secs.to_string(), "秒，默认 3", window, cx),
            check_certificate: s.check_certificate,
            http_accept_gzip: s.http_accept_gzip,
            bt_max_peers: mk(s.bt_max_peers.to_string(), "默认 55", window, cx),
            seed_ratio: mk(s.seed_ratio.clone(), "0.0 表示不做种", window, cx),
            seed_time: mk(s.seed_time.clone(), "分钟，0 表示下完即停", window, cx),
            bt_tracker: mk(s.bt_tracker.clone().unwrap_or_default(), "逗号分隔 Tracker URL", window, cx),
            enable_dht: s.enable_dht,
            enable_peer_exchange: s.enable_peer_exchange,
            bt_save_metadata: s.bt_save_metadata,
            rpc_secret: mk(s.aria2.secret.clone().unwrap_or_default(), "可选 RPC Secret", window, cx),
            save_session_interval: mk(s.save_session_interval_secs.to_string(), "秒，默认 60", window, cx),
            enable_session_persistence: s.enable_session_persistence,
            disable_ipv6: s.disable_ipv6,
        }
    }

    pub fn toggle_flag(&mut self, flag: SettingsFlag) {
        match flag {
            SettingsFlag::ContinueDownload => self.continue_download = !self.continue_download,
            SettingsFlag::AllowOverwrite => self.allow_overwrite = !self.allow_overwrite,
            SettingsFlag::AutoFileRenaming => self.auto_file_renaming = !self.auto_file_renaming,
            SettingsFlag::RemoteTime => self.remote_time = !self.remote_time,
            SettingsFlag::CheckCertificate => self.check_certificate = !self.check_certificate,
            SettingsFlag::HttpAcceptGzip => self.http_accept_gzip = !self.http_accept_gzip,
            SettingsFlag::EnableDht => self.enable_dht = !self.enable_dht,
            SettingsFlag::EnablePeerExchange => self.enable_peer_exchange = !self.enable_peer_exchange,
            SettingsFlag::BtSaveMetadata => self.bt_save_metadata = !self.bt_save_metadata,
            SettingsFlag::EnableSessionPersistence => self.enable_session_persistence = !self.enable_session_persistence,
            SettingsFlag::DisableIpv6 => self.disable_ipv6 = !self.disable_ipv6,
            SettingsFlag::EnableNotifications => self.enable_notifications = !self.enable_notifications,
            SettingsFlag::AutoStart => self.auto_start = !self.auto_start,
        }
    }

    pub fn apply_to(&self, s: &mut AppSettings, cx: &App) {
        let val = |e: &Entity<InputState>| e.read(cx).value().trim().to_string();
        let non_empty = |v: String| (!v.is_empty()).then_some(v);

        if let Some(d) = non_empty(val(&self.download_dir)) { s.default_download_dir = PathBuf::from(d); }
        if let Some(a) = non_empty(val(&self.file_allocation)) { s.file_allocation = a; }
        if let Some(c) = non_empty(val(&self.disk_cache)) { s.disk_cache = c; }
        s.continue_download = self.continue_download;
        s.allow_overwrite = self.allow_overwrite;
        s.auto_file_renaming = self.auto_file_renaming;
        s.remote_time = self.remote_time;
        s.enable_notifications = self.enable_notifications;
        s.auto_start = self.auto_start;

        if let Ok(v) = val(&self.max_concurrent).parse::<u32>() { s.max_concurrent_downloads = v.clamp(1, 32); }
        if let Ok(v) = val(&self.split).parse::<u32>() { s.default_split = v.clamp(1, 64); }
        if let Ok(v) = val(&self.max_conn_per_server).parse::<u32>() { s.max_connection_per_server = v.clamp(1, 16); }
        if let Some(v) = non_empty(val(&self.min_split_size)) { s.min_split_size = v; }
        s.max_overall_download_limit = non_empty(val(&self.speed_limit)).unwrap_or_else(|| "0".into());
        s.max_overall_upload_limit = non_empty(val(&self.upload_limit)).unwrap_or_else(|| "0".into());
        s.lowest_speed_limit = non_empty(val(&self.lowest_speed_limit)).unwrap_or_else(|| "0".into());

        s.proxy = non_empty(val(&self.proxy));
        s.no_proxy = non_empty(val(&self.no_proxy));
        s.user_agent = non_empty(val(&self.user_agent));
        if let Ok(v) = val(&self.connect_timeout).parse::<u32>() { s.connect_timeout_secs = v.clamp(5, 600); }
        if let Ok(v) = val(&self.timeout_secs).parse::<u32>() { s.timeout_secs = v.clamp(5, 600); }
        if let Ok(v) = val(&self.max_tries).parse::<u32>() { s.max_tries = v; }
        if let Ok(v) = val(&self.retry_wait).parse::<u32>() { s.retry_wait_secs = v; }
        s.check_certificate = self.check_certificate;
        s.http_accept_gzip = self.http_accept_gzip;

        if let Ok(v) = val(&self.bt_max_peers).parse::<u32>() { s.bt_max_peers = v; }
        if let Some(v) = non_empty(val(&self.seed_ratio)) { s.seed_ratio = v; }
        s.seed_time = non_empty(val(&self.seed_time)).unwrap_or_else(|| "0".into());
        s.bt_tracker = non_empty(val(&self.bt_tracker));
        s.enable_dht = self.enable_dht;
        s.enable_peer_exchange = self.enable_peer_exchange;
        s.bt_save_metadata = self.bt_save_metadata;

        s.aria2.secret = non_empty(val(&self.rpc_secret));
        if let Ok(v) = val(&self.save_session_interval).parse::<u32>() { s.save_session_interval_secs = v.clamp(10, 3600); }
        s.enable_session_persistence = self.enable_session_persistence;
        s.disable_ipv6 = self.disable_ipv6;
    }
}

pub fn pick_directory(initial: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(dir) = initial.filter(|p| p.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder()
}

pub fn open_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    { std::process::Command::new("explorer").arg(path).spawn()?; }
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(path).spawn()?; }
    #[cfg(all(unix, not(target_os = "macos")))]
    { std::process::Command::new("xdg-open").arg(path).spawn()?; }
    Ok(())
}

/// 在系统文件资源管理器中打开所在目录并高亮选中目标文件（若文件尚不存在则直接打开父目录）
pub fn reveal_file_in_directory(file_path: &Path, fallback_dir: &Path) -> std::io::Result<()> {
    if file_path.exists() {
        #[cfg(windows)]
        {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", file_path.display()))
                .spawn()?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", &file_path.to_string_lossy()])
                .spawn()?;
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            open_directory(fallback_dir)?;
        }
        Ok(())
    } else {
        open_directory(fallback_dir)
    }
}

/// 使用系统默认关联程序直接打开已下载完成的文件
pub fn open_file(file_path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(file_path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(file_path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(file_path).spawn()?;
    }
    Ok(())
}

/// 将指定标题的桌面窗口立刻恢复并置顶到前台（解决 Windows 后台弹窗无感问题）
pub fn bring_window_to_front(title: &str) {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "user32")]
        unsafe extern "system" {
            fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
            fn ShowWindow(hwnd: isize, cmd_show: i32) -> i32;
            fn SetWindowPos(
                hwnd: isize,
                insert_after: isize,
                x: i32,
                y: i32,
                cx: i32,
                cy: i32,
                flags: u32,
            ) -> i32;
            fn SetForegroundWindow(hwnd: isize) -> i32;
        }

        const SW_RESTORE: i32 = 9;
        const HWND_TOPMOST: isize = -1;
        const HWND_NOTOPMOST: isize = -2;
        const SWP_NOMOVE: u32 = 0x0002;
        const SWP_NOSIZE: u32 = 0x0001;
        const SWP_SHOWWINDOW: u32 = 0x0040;

        let wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
            if hwnd != 0 {
                ShowWindow(hwnd, SW_RESTORE);
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                SetForegroundWindow(hwnd);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = title;
    }
}

/// 将指定标题的桌面窗口隐藏到系统托盘
pub fn hide_window(title: &str) {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "user32")]
        unsafe extern "system" {
            fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
            fn ShowWindow(hwnd: isize, cmd_show: i32) -> i32;
        }

        const SW_HIDE: i32 = 0;
        let wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
            if hwnd != 0 {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = title;
    }
}
