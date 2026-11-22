//! 桌面端视图状态管理与数据格式化模块

use idm_core::{AppSettings, FileCategory};

/// 左侧分类导航树选项（仿 IDM 左侧栏，覆盖全部 5 种任务状态与 6 大文件分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFilter {
    All,
    Active,
    Waiting,
    Paused,
    Completed,
    Error,
    Category(FileCategory),
}

/// 桌面端核心视图状态（已移除冗余僵尸字段 `store`）
pub struct DesktopState {
    pub settings: AppSettings,
    pub active_filter: SidebarFilter,
    pub selected_task_id: Option<String>,
    /// 左侧「全部下载」分类子树是否展开
    pub categories_expanded: bool,
    /// 底部「任务详情」面板是否收起为单行紧凑模式
    pub inspector_collapsed: bool,
}

impl DesktopState {
    /// 初始化桌面端状态
    pub fn new() -> Self {
        Self {
            settings: AppSettings::default(),
            active_filter: SidebarFilter::All,
            selected_task_id: None,
            categories_expanded: true,
            inspector_collapsed: false,
        }
    }
}

/// 将字节数格式化为人类可读字符串（如 `12.50 MB`）
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// 将每秒字节数格式化为速度字符串（如 `11.92 MB/s`）
pub fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        "—".to_string()
    } else {
        format!("{}/s", format_bytes(bytes_per_sec))
    }
}

/// 计算任务进度百分比（0.0 ~ 100.0）
pub fn task_progress(completed: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    ((completed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32
}

/// 计算预估剩余时间 (ETA)
pub fn format_eta(completed: u64, total: u64, speed: u64) -> String {
    if speed == 0 || completed >= total {
        return "—".to_string();
    }
    let secs = (total - completed) / speed;
    if secs >= 3600 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_helpers() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(format_speed(1024 * 1024), "1.00 MB/s");
        assert_eq!(format_eta(0, 120, 2), "1m 0s");
        assert_eq!(format_eta(100, 100, 10), "—");
    }

    #[test]
    fn test_state_init() {
        let state = DesktopState::new();
        assert_eq!(state.active_filter, SidebarFilter::All);
        assert!(state.categories_expanded);
    }
}
