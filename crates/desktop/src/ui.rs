//! 基于 GPUI + gpui-kit 的现代 IDM 主界面

use crate::state::{DesktopState, SidebarFilter, format_speed};
use crate::ui_parts::{Palette, render_detail_panel, render_table_header, render_task_row};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::base::StyledExt;
use gpui_kit::*;
use idm_core::NewDownloadRequest;

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

    /// 渲染顶部工具栏（新建、暂停/继续、全部暂停、删除、深浅色主题切换）
    fn render_toolbar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_label = if self.state.is_dark {
            "☀ 浅色"
        } else {
            "🌙 深色"
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2p5()
            .bg(p.surface)
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("btn-new")
                            .primary()
                            .label("+ 新建任务")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.add_from_request(NewDownloadRequest {
                                    url: "https://example.com/release/sample-app.zip".into(),
                                    filename: None,
                                    save_dir: None,
                                    headers: vec![],
                                    auto_start: true,
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("btn-toggle-pause")
                            .label("⏯ 继续 / 暂停")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.toggle_pause_selected();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("btn-pause-all")
                            .label("⏹ 全部暂停")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.pause_all();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("btn-delete")
                            .label("🗑 删除选中")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.delete_selected();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                Button::new("btn-theme")
                    .label(theme_label)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.is_dark = !this.state.is_dark;
                        cx.notify();
                    })),
            )
    }

    /// 渲染左侧分类导航树（带分区标题与实时数量角标）
    fn render_sidebar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_filter;

        let render_group =
            |title: &'static str, items: Vec<(SidebarFilter, &'static str)>, offset: usize, cx: &mut Context<Self>| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .px_3()
                            .pt_2()
                            .pb_1()
                            .text_xs()
                            .font_semibold()
                            .text_color(p.muted)
                            .child(title),
                    )
                    .children(items.into_iter().enumerate().map(|(i, (filter, label))| {
                        let is_active = filter == active;
                        let count = self.state.count_for_filter(filter);
                        div()
                            .id(("nav-item", offset + i))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if is_active { p.row_selected } else { p.sidebar_bg })
                            .text_color(if is_active { p.primary } else { p.text })
                            .text_sm()
                            .font_medium()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.active_filter = filter;
                                cx.notify();
                            }))
                            .child(label)
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_full()
                                    .bg(p.surface)
                                    .text_xs()
                                    .text_color(p.muted)
                                    .child(count.to_string()),
                            )
                    }))
            };

        div()
            .flex()
            .flex_col()
            .w(px(210.0))
            .bg(p.sidebar_bg)
            .border_r_1()
            .border_color(p.border)
            .p_2()
            .gap_3()
            .child(render_group("下载状态", SidebarFilter::status_items(), 0, cx))
            .child(render_group("文件分类", SidebarFilter::category_items(), 100, cx))
    }

    /// 渲染主区域任务表格
    fn render_task_table(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = self.state.filtered_tasks();
        let selected_id = self.state.selected_task_id.clone();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .bg(p.surface)
            .child(render_table_header(p))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .children(tasks.into_iter().enumerate().map(|(idx, task)| {
                        let task_id = task.id.clone();
                        let is_selected = selected_id.as_deref() == Some(task.id.as_str());
                        render_task_row(idx, task, is_selected, p).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.state.selected_task_id = Some(task_id.clone());
                                cx.notify();
                            },
                        ))
                    })),
            )
            .child(render_detail_panel(self.state.selected_task(), p))
    }

    /// 渲染底部全局状态栏
    fn render_statusbar(&self, p: Palette) -> impl IntoElement {
        let active_count = self.state.count_for_filter(SidebarFilter::Active);
        let speed = format_speed(self.state.total_download_speed());
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_1p5()
            .bg(p.sidebar_bg)
            .border_t_1()
            .border_color(p.border)
            .text_xs()
            .text_color(p.muted)
            .child(format!("活跃任务: {active_count}   |   全局下载速度: {speed}"))
            .child(format!("● aria2 RPC: {}", self.state.settings.aria2.endpoint))
    }
}

impl Render for IdmMainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::for_mode(self.state.is_dark);
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.bg)
            .child(self.render_toolbar(p, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_sidebar(p, cx))
                    .child(self.render_task_table(p, cx)),
            )
            .child(self.render_statusbar(p))
    }
}
