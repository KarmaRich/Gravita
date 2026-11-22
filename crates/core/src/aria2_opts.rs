//! 将 AppSettings 映射为 aria2 热更新全局参数与启动命令行参数

use crate::AppSettings;
use std::collections::BTreeMap;

impl AppSettings {
    /// 生成可通过 `aria2.changeGlobalOption` 实时热更新的参数映射
    pub fn to_aria2_global_options(&self) -> BTreeMap<String, String> {
        let mut opts = BTreeMap::new();
        // 1. 常规与保存
        opts.insert(
            "dir".into(),
            self.default_download_dir.to_string_lossy().into_owned(),
        );
        opts.insert("continue".into(), bool_str(self.continue_download));
        opts.insert("allow-overwrite".into(), bool_str(self.allow_overwrite));
        opts.insert(
            "auto-file-renaming".into(),
            bool_str(self.auto_file_renaming),
        );
        opts.insert("remote-time".into(), bool_str(self.remote_time));

        // 2. 连接与限速
        opts.insert(
            "max-concurrent-downloads".into(),
            self.max_concurrent_downloads.to_string(),
        );
        opts.insert("split".into(), self.default_split.to_string());
        opts.insert(
            "max-connection-per-server".into(),
            self.max_connection_per_server.min(64).to_string(),
        );
        opts.insert("min-split-size".into(), self.min_split_size.clone());
        opts.insert(
            "max-overall-download-limit".into(),
            self.max_overall_download_limit.clone(),
        );
        opts.insert(
            "max-overall-upload-limit".into(),
            self.max_overall_upload_limit.clone(),
        );
        opts.insert(
            "lowest-speed-limit".into(),
            self.lowest_speed_limit.clone(),
        );

        // 3. HTTP 与代理（全局默认直连不挂代理，由新建任务按需勾选启用预设代理或独立代理）
        opts.insert("all-proxy".into(), String::new());
        opts.insert("no-proxy".into(), self.no_proxy.clone().unwrap_or_default());
        if let Some(ua) = self.user_agent.as_deref().filter(|s| !s.is_empty()) {
            opts.insert("user-agent".into(), ua.to_string());
        }
        opts.insert("connect-timeout".into(), self.connect_timeout_secs.to_string());
        opts.insert("timeout".into(), self.timeout_secs.to_string());
        opts.insert("max-tries".into(), self.max_tries.to_string());
        opts.insert("retry-wait".into(), self.retry_wait_secs.to_string());
        opts.insert("http-accept-gzip".into(), bool_str(self.http_accept_gzip));

        // 4. BT 与磁力
        opts.insert("bt-max-peers".into(), self.bt_max_peers.to_string());
        opts.insert("seed-ratio".into(), self.seed_ratio.clone());
        if !self.seed_time.trim().is_empty() {
            opts.insert("seed-time".into(), self.seed_time.clone());
        }
        opts.insert("bt-save-metadata".into(), bool_str(self.bt_save_metadata));
        if let Some(trackers) = self.bt_tracker.as_deref().filter(|s| !s.is_empty()) {
            opts.insert("bt-tracker".into(), trackers.to_string());
        }

        opts
    }

    /// 生成仅在启动 `aria2c` 时生效的命令行参数（含磁盘缓存、预分配、TLS、DHT、会话持久化等）
    pub fn to_aria2_startup_args(&self) -> Vec<String> {
        let mut args = vec![
            format!("--file-allocation={}", self.file_allocation),
            format!("--disk-cache={}", self.disk_cache),
            format!("--check-certificate={}", bool_str(self.check_certificate)),
            format!("--enable-dht={}", bool_str(self.enable_dht)),
            format!(
                "--enable-peer-exchange={}",
                bool_str(self.enable_peer_exchange)
            ),
            format!("--disable-ipv6={}", bool_str(self.disable_ipv6)),
        ];

        if self.enable_session_persistence {
            let session_path = Self::default_session_path();
            if let Some(parent) = session_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if session_path.is_file() {
                args.push(format!("--input-file={}", session_path.display()));
            }
            args.push(format!("--save-session={}", session_path.display()));
            args.push(format!(
                "--save-session-interval={}",
                self.save_session_interval_secs
            ));
        }

        args
    }

    /// 解析单个下载任务最终使用的代理地址：
    /// - `use_proxy == false`（默认）：不使用任何代理（直连）
    /// - `use_proxy == true`：优先使用任务自定义的 `custom_proxy`，若为空则使用全局设置中的预设代理 `self.proxy`
    pub fn resolve_task_proxy(&self, use_proxy: bool, custom_proxy: Option<&str>) -> Option<String> {
        if !use_proxy {
            return None;
        }
        if let Some(custom) = custom_proxy.map(str::trim).filter(|s| !s.is_empty()) {
            return Some(custom.to_string());
        }
        self.proxy
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    }
}

fn bool_str(v: bool) -> String {
    if v { "true" } else { "false" }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_aria2_global_options_contains_all_categories() {
        let mut s = AppSettings::default();
        s.max_overall_download_limit = "5M".into();
        s.proxy = Some("http://127.0.0.1:7890".into());
        s.seed_ratio = "0.0".into();

        let opts = s.to_aria2_global_options();
        assert_eq!(opts.get("max-overall-download-limit").map(String::as_str), Some("5M"));
        assert_eq!(opts.get("all-proxy").map(String::as_str), Some(""));
        assert_eq!(
            s.resolve_task_proxy(false, None),
            None
        );
        assert_eq!(
            s.resolve_task_proxy(true, None).as_deref(),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(
            s.resolve_task_proxy(true, Some("http://127.0.0.1:1080")).as_deref(),
            Some("http://127.0.0.1:1080")
        );
        assert_eq!(opts.get("seed-ratio").map(String::as_str), Some("0.0"));
        assert_eq!(opts.get("continue").map(String::as_str), Some("true"));
    }

    #[test]
    fn test_to_aria2_startup_args_includes_engine_flags() {
        let s = AppSettings::default();
        let args = s.to_aria2_startup_args();
        assert!(args.iter().any(|a| a == "--file-allocation=none"));
        assert!(args.iter().any(|a| a == "--disk-cache=16M"));
        assert!(args.iter().any(|a| a == "--enable-dht=true"));
        assert!(args.iter().any(|a| a.starts_with("--save-session=")));
    }
}
