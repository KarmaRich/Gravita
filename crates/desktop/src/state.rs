//! 桌面端视图状态管理、任务控制与数据格式化模块

use idm_core::{
    AppSettings, DownloadStatus, DownloadTask, FileCategory, NewDownloadRequest, TaskFilter,
    TaskStore,
};

/// 左侧分类导航树选项（仿 IDM 左侧栏）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFilter {
    All,
    Active,
    Paused,
    Completed,
    Category(FileCategory),
}

/// 桌面端核心视图状态
pub struct DesktopState {
    pub settings: AppSettings,
    pub store: TaskStore,
    pub active_filter: SidebarFilter,
    pub selected_task_id: Option<String>,
    pub next_seq: u64,
}

impl DesktopState {
    /// 初始化桌面端状态（预置多类型示例任务，直观展示进度条、速度、状态标签与分类统计）
    pub fn with_demo_tasks() -> Self {
        let settings = AppSettings::default();
        let mut store = TaskStore::new();
        let demos = [
            (
                "task-1",
                "ubuntu-24.04-desktop-amd64.iso",
                "https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso",
                FileCategory::Other,
                DownloadStatus::Active,
                5_800_000_000,
                2_610_000_000,
                14_200_000,
            ),
            (
                "task-2",
                "rust-1.96-x86_64-pc-windows-msvc.msi",
                "https://static.rust-lang.org/dist/rust-1.96.msi",
                FileCategory::Program,
                DownloadStatus::Active,
                285_000_000,
                210_900_000,
                8_650_000,
            ),
            (
                "task-3",
                "4K-Nature-Documentary.mkv",
                "https://cdn.example.com/videos/4K-Nature-Documentary.mkv",
                FileCategory::Video,
                DownloadStatus::Paused,
                1_450_000_000,
                435_000_000,
                0,
            ),
            (
                "task-4",
                "gpui-kit-design-assets.zip",
                "https://example.com/archive/gpui-kit-design-assets.zip",
                FileCategory::Compressed,
                DownloadStatus::Completed,
                42_000_000,
                42_000_000,
                0,
            ),
        ];

        for (id, name, url, cat, status, total, done, speed) in demos {
            store.upsert(DownloadTask {
                id: id.into(),
                gid: Some(format!("gid-{id}")),
                url: url.into(),
                filename: name.into(),
                save_dir: settings.resolve_save_dir(cat),
                category: cat,
                status,
                total_bytes: total,
                completed_bytes: done,
                speed_bytes_per_sec: speed,
                headers: vec![],
            });
        }

        Self {
            settings,
            store,
            active_filter: SidebarFilter::All,
            selected_task_id: Some("task-1".into()),
            next_seq: 5,
        }
    }

    fn to_task_filter(filter: SidebarFilter) -> TaskFilter {
        match filter {
            SidebarFilter::All => TaskFilter::default(),
            SidebarFilter::Active => TaskFilter {
                status: Some(DownloadStatus::Active),
                ..Default::default()
            },
            SidebarFilter::Paused => TaskFilter {
                status: Some(DownloadStatus::Paused),
                ..Default::default()
            },
            SidebarFilter::Completed => TaskFilter {
                status: Some(DownloadStatus::Completed),
                ..Default::default()
            },
            SidebarFilter::Category(cat) => TaskFilter {
                category: Some(cat),
                ..Default::default()
            },
        }
    }

    /// 根据当前左侧栏筛选条件获取任务列表
    pub fn filtered_tasks(&self) -> Vec<&DownloadTask> {
        self.store.query(&Self::to_task_filter(self.active_filter))
    }

    /// 统计某个导航项下的任务数量（用于左侧栏角标）
    pub fn count_for_filter(&self, filter: SidebarFilter) -> usize {
        self.store.query(&Self::to_task_filter(filter)).len()
    }

    /// 获取当前选中的任务详情
    pub fn selected_task(&self) -> Option<&DownloadTask> {
        let id = self.selected_task_id.as_deref()?;
        self.store
            .query(&TaskFilter::default())
            .into_iter()
            .find(|t| t.id == id)
    }

    /// 从新建下载请求创建并加入任务列表
    pub fn add_from_request(&mut self, req: NewDownloadRequest) -> String {
        let id = format!("task-{}", self.next_seq);
        self.next_seq += 1;
        let filename = req.inferred_filename();
        let category = FileCategory::from_filename(&filename);
        let save_dir = req
            .save_dir
            .unwrap_or_else(|| self.settings.resolve_save_dir(category));

        self.store.upsert(DownloadTask {
            id: id.clone(),
            gid: None,
            url: req.url,
            filename,
            save_dir,
            category,
            status: if req.auto_start {
                DownloadStatus::Active
            } else {
                DownloadStatus::Paused
            },
            total_bytes: 128_000_000,
            completed_bytes: 0,
            speed_bytes_per_sec: if req.auto_start { 4_200_000 } else { 0 },
            headers: req.headers,
        });
        self.selected_task_id = Some(id.clone());
        id
    }

    /// 切换当前选中任务的暂停/继续状态
    pub fn toggle_pause_selected(&mut self) -> bool {
        let Some(mut task) = self.selected_task().cloned() else {
            return false;
        };
        match task.status {
            DownloadStatus::Active => {
                task.status = DownloadStatus::Paused;
                task.speed_bytes_per_sec = 0;
            }
            DownloadStatus::Paused | DownloadStatus::Waiting => {
                task.status = DownloadStatus::Active;
                task.speed_bytes_per_sec = 6_400_000;
            }
            _ => return false,
        }
        self.store.upsert(task);
        true
    }

    /// 暂停所有正在下载的任务
    pub fn pause_all(&mut self) {
        let actives: Vec<DownloadTask> = self
            .store
            .query(&TaskFilter {
                status: Some(DownloadStatus::Active),
                ..Default::default()
            })
            .into_iter()
            .cloned()
            .collect();
        for mut task in actives {
            task.status = DownloadStatus::Paused;
            task.speed_bytes_per_sec = 0;
            self.store.upsert(task);
        }
    }

    /// 删除当前选中的任务
    pub fn delete_selected(&mut self) -> bool {
        if let Some(id) = self.selected_task_id.take() {
            return self.store.remove(&id);
        }
        false
    }

    /// 计算当前所有活跃任务的全局下载速度（字节/秒）
    pub fn total_download_speed(&self) -> u64 {
        self.store
            .query(&TaskFilter {
                status: Some(DownloadStatus::Active),
                ..Default::default()
            })
            .into_iter()
            .map(|t| t.speed_bytes_per_sec)
            .sum()
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
    fn test_pause_resume_and_pause_all() {
        let mut state = DesktopState::with_demo_tasks();
        assert_eq!(state.count_for_filter(SidebarFilter::Active), 2);
        assert!(state.toggle_pause_selected());
        assert_eq!(state.count_for_filter(SidebarFilter::Active), 1);
        assert_eq!(state.count_for_filter(SidebarFilter::Paused), 2);
        state.pause_all();
        assert_eq!(state.count_for_filter(SidebarFilter::Active), 0);
    }
}
