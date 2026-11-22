//! 基于 GPUI + gpui-kit 的 IDM 经典布局主界面

use crate::state::{DesktopState, SidebarFilter, format_bytes, format_speed};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use idm_core::{DownloadStatus, NewDownloadRequest};

/// IDM 主窗口视图实体
pub struct IdmMainView {
    state: DesktopState,
}

impl IdmMainView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            state: DesktopState::with_demo_tasks(),
        }
    }

    /// 渲染顶部工具栏（新建下载、删除选中等核心按钮）
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .child(
                Button::new("btn-new")
                    .primary()
                    .label("+ 新建下载")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.add_from_request(NewDownloadRequest {
                            url: "https://example.com/sample-package.zip".into(),
                            filename: None,
                            save_dir: None,
                            headers: vec![],
                            auto_start: true,
                        });
                        cx.notify();
                    })),
            )
            .child(
                Button::new("btn-delete")
                    .label("删除选中")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.delete_selected();
                        cx.notify();
                    })),
            )
    }

    /// 渲染左侧分类导航树
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_filter;
        let items = SidebarFilter::all_items();

        div()
            .flex()
            .flex_col()
            .w(px(180.0))
            .border_r_1()
            .p_2()
            .gap_1()
            .children(items.into_iter().enumerate().map(|(idx, (filter, label))| {
                let is_active = filter == active;
                Button::new(("nav", idx))
                    .label(if is_active {
                        format!("▸ {label}")
                    } else {
                        label.to_string()
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.active_filter = filter;
                        cx.notify();
                    }))
            }))
    }

    /// 渲染主区域任务列表
    fn render_task_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = self.state.filtered_tasks();
        let selected_id = self.state.selected_task_id.clone();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .p_3()
            .gap_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .pb_2()
                    .border_b_1()
                    .child("文件名")
                    .child("大小 / 速度 / 状态"),
            )
            .children(tasks.into_iter().enumerate().map(|(idx, task)| {
                let task_id = task.id.clone();
                let is_selected = selected_id.as_deref() == Some(task.id.as_str());
                let status_label = match task.status {
                    DownloadStatus::Active => "下载中",
                    DownloadStatus::Waiting => "等待中",
                    DownloadStatus::Paused => "已暂停",
                    DownloadStatus::Completed => "已完成",
                    DownloadStatus::Error => "出错",
                };
                let summary = format!(
                    "{} | {} | {}",
                    format_bytes(task.total_bytes),
                    format_speed(task.speed_bytes_per_sec),
                    status_label
                );

                div()
                    .id(("task-row", idx))
                    .flex()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.selected_task_id = Some(task_id.clone());
                        cx.notify();
                    }))
                    .child(if is_selected {
                        format!("● {}", task.filename)
                    } else {
                        task.filename.clone()
                    })
                    .child(summary)
            }))
    }

    /// 渲染底部状态栏
    fn render_statusbar(&self) -> impl IntoElement {
        let count = self.state.filtered_tasks().len();
        let speed = format_speed(self.state.total_download_speed());
        div()
            .flex()
            .justify_between()
            .px_4()
            .py_1()
            .border_t_1()
            .child(format!("当前视图任务数: {count}"))
            .child(format!(
                "全局速度: {speed} | aria2: {}",
                self.state.settings.aria2.endpoint
            ))
    }
}

impl Render for IdmMainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(self.render_sidebar(cx))
                    .child(self.render_task_list(cx)),
            )
            .child(self.render_statusbar())
    }
}
