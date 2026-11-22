//! 仿经典 IDM (Internet Download Manager) 桌面主界面

use crate::state::{DesktopState, SidebarFilter, format_speed};
use crate::ui_parts::{
    IdmPalette, idm_tool_button, render_idm_table_header, render_idm_task_row, render_menubar,
};
use gpui_kit::base::StyledExt;
use gpui_kit::*;
use idm_core::{FileCategory, NewDownloadRequest};

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

    /// 渲染经典 IDM 大图标工具栏
    fn render_toolbar(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let divider = || div().w(px(1.0)).h(px(36.0)).mx_1().bg(p.border);

        div()
            .flex()
            .items_center()
            .px_2()
            .py_1()
            .bg(p.toolbar_bg)
            .border_b_1()
            .border_color(p.border)
            .child(
                idm_tool_button("tb-add", "＋", rgb(0x16a34a), "新建任务", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.add_from_request(NewDownloadRequest {
                            url: "https://example.com/release/idm-demo-package.zip".into(),
                            filename: None,
                            save_dir: None,
                            headers: vec![],
                            auto_start: true,
                        });
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-resume", "▶", rgb(0x16a34a), "继续", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.toggle_pause_selected();
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-stop", "■", rgb(0xdc2626), "停止", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.toggle_pause_selected();
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-stop-all", "⏹", rgb(0xb91c1c), "全部停止", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.pause_all();
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-del", "✕", rgb(0xef4444), "删除", p).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.delete_selected();
                        cx.notify();
                    },
                )),
            )
            .child(divider())
            .child(idm_tool_button("tb-opt", "⚙", rgb(0x2563eb), "选项", p))
            .child(idm_tool_button("tb-sched", "🕒", rgb(0x0284c7), "计划任务", p))
    }

    /// 渲染左侧经典 IDM 树形分类栏
    fn render_category_tree(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_filter;
        let nodes: Vec<(usize, SidebarFilter, &str, bool)> = vec![
            (0, SidebarFilter::All, "▾ 全部下载", false),
            (1, SidebarFilter::Category(FileCategory::Compressed), "压缩文件", true),
            (2, SidebarFilter::Category(FileCategory::Document), "文档", true),
            (3, SidebarFilter::Category(FileCategory::Music), "音乐", true),
            (4, SidebarFilter::Category(FileCategory::Program), "程序", true),
            (5, SidebarFilter::Category(FileCategory::Video), "视频", true),
            (6, SidebarFilter::Category(FileCategory::Other), "其他", true),
            (7, SidebarFilter::Active, "▸ 正在下载", false),
            (8, SidebarFilter::Paused, "▸ 已暂停", false),
            (9, SidebarFilter::Completed, "▸ 已完成", false),
        ];

        div()
            .flex()
            .flex_col()
            .w(px(185.0))
            .bg(p.panel_bg)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .bg(p.header_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .text_xs()
                    .font_medium()
                    .text_color(p.text)
                    .child("分类"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p_1()
                    .children(nodes.into_iter().map(|(id, filter, label, indent)| {
                        let is_active = filter == active;
                        let count = self.state.count_for_filter(filter);
                        div()
                            .id(("tree", id))
                            .flex()
                            .items_center()
                            .justify_between()
                            .pl(if indent { px(22.0) } else { px(6.0) })
                            .pr_2()
                            .py_1()
                            .cursor_pointer()
                            .bg(if is_active { p.selected_bg } else { p.panel_bg })
                            .text_xs()
                            .text_color(p.text)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.active_filter = filter;
                                cx.notify();
                            }))
                            .child(label)
                            .child(div().text_color(p.muted).child(format!("({count})")))
                    })),
            )
    }

    /// 渲染右侧任务列表表格
    fn render_task_list(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = self.state.filtered_tasks();
        let selected_id = self.state.selected_task_id.clone();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .bg(p.panel_bg)
            .child(render_idm_table_header(p))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .children(tasks.into_iter().enumerate().map(|(idx, task)| {
                        let task_id = task.id.clone();
                        let is_selected = selected_id.as_deref() == Some(task.id.as_str());
                        render_idm_task_row(idx, task, is_selected, p).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.state.selected_task_id = Some(task_id.clone());
                                cx.notify();
                            },
                        ))
                    })),
            )
    }

    /// 渲染底部经典状态栏
    fn render_statusbar(&self, p: IdmPalette) -> impl IntoElement {
        let count = self.state.filtered_tasks().len();
        let speed = format_speed(self.state.total_download_speed());
        let selected_url = self
            .state
            .selected_task()
            .map(|t| t.url.as_str())
            .unwrap_or("就绪");

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .bg(p.menu_bg)
            .border_t_1()
            .border_color(p.border)
            .text_xs()
            .text_color(p.muted)
            .child(format!("{selected_url}"))
            .child(format!(
                "项目: {count}  |  速度: {speed}  |  aria2: {}",
                self.state.settings.aria2.endpoint
            ))
    }
}

impl Render for IdmMainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = IdmPalette::classic();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel_bg)
            .child(render_menubar(p))
            .child(self.render_toolbar(p, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_category_tree(p, cx))
                    .child(self.render_task_list(p, cx)),
            )
            .child(self.render_statusbar(p))
    }
}
