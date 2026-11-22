//! 桌面端视图状态管理与数据格式化模块

use idm_core::{AppSettings, DownloadStatus, DownloadTask, FileCategory, NewDownloadRequest, TaskFilter, TaskStore};

/// 左侧分类导航树选项（仿 IDM 左侧栏）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFilter {
    All,
    Active,
    Completed,
    Category(FileCategory),
}

impl SidebarFilter {
    /// 返回所有默认导航项（按 IDM 习惯顺序排列）
    pub fn all_items() -> Vec<(Self, &'static str)> {
        vec![
            (Self::All, "全部下载"),
            (Self::Active, "正在下载"),
            (Self::Completed, "已完成"),
            (Self::Category(FileCategory::Compressed), "压缩文件"),
            (Self::Category(FileCategory::Document), "文档"),
            (Self::Category(FileCategory::Music), "音乐"),
            (Self::Category(FileCategory::Program), "程序"),
            (Self::Category(FileCategory::Video), "视频"),
            (Self::Category(FileCategory::Other), "其他"),
        ]
    }
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
    /// 初始化桌面端状态（预置若干示例任务便于 MVP 阶段直观预览布局）
    pub fn with_demo_tasks() -> Self {
        let settings = AppSettings::default();
        let mut store = TaskStore::new();
        store.upsert(DownloadTask {
            id: "task-1".into(),
            gid: Some("demo-gid-1".into()),
            url: "https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso".into(),
            filename: "ubuntu-24.04-desktop-amd64.iso".into(),
            save_dir: settings.resolve_save_dir(FileCategory::Other),
            category: FileCategory::Other,
            status: DownloadStatus::Active,
            total_bytes: 5_800_000_000,
            completed_bytes: 2_320_000_000,
            speed_bytes_per_sec: 12_500_000,
            headers: vec![],
        });
        store.upsert(DownloadTask {
            id: "task-2".into(),
            gid: Some("demo-gid-2".into()),
            url: "https://example.com/archive/rust-book-zh.zip".into(),
            filename: "rust-book-zh.zip".into(),
            save_dir: settings.resolve_save_dir(FileCategory::Compressed),
            category: FileCategory::Compressed,
            status: DownloadStatus::Completed,
            total_bytes: 42_000_000,
            completed_bytes: 42_000_000,
            speed_bytes_per_sec: 0,
            headers: vec![],
        });

        Self {
            settings,
            store,
            active_filter: SidebarFilter::All,
            selected_task_id: Some("task-1".into()),
            next_seq: 3,
        }
    }

    /// 根据当前左侧栏筛选条件获取任务列表
    pub fn filtered_tasks(&self) -> Vec<&DownloadTask> {
        let filter = match self.active_filter {
            SidebarFilter::All => TaskFilter::default(),
            SidebarFilter::Active => TaskFilter {
                status: Some(DownloadStatus::Active),
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
        };
        self.store.query(&filter)
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
            total_bytes: 0,
            completed_bytes: 0,
            speed_bytes_per_sec: 0,
            headers: req.headers,
        });
        self.selected_task_id = Some(id.clone());
        id
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

/// 将字节数格式化为人类可读字符串（如 `12.5 MB`）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_and_speed() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(format_speed(0), "—");
        assert_eq!(format_speed(1024 * 1024), "1.00 MB/s");
    }

    #[test]
    fn test_desktop_state_filter_add_and_delete() {
        let mut state = DesktopState::with_demo_tasks();
        assert_eq!(state.filtered_tasks().len(), 2);

        state.active_filter = SidebarFilter::Completed;
        assert_eq!(state.filtered_tasks().len(), 1);

        state.active_filter = SidebarFilter::All;
        let new_id = state.add_from_request(NewDownloadRequest {
            url: "https://example.com/demo.mp4".into(),
            filename: None,
            save_dir: None,
            headers: vec![],
            auto_start: true,
        });
        assert_eq!(state.selected_task_id.as_deref(), Some(new_id.as_str()));
        assert_eq!(state.filtered_tasks().len(), 3);

        assert!(state.delete_selected());
        assert_eq!(state.filtered_tasks().len(), 2);
    }
}
