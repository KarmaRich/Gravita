//! 经典 IDM 风格配色、工具栏大图标按钮与表格行渲染组件

use crate::state::{format_bytes, format_eta, format_speed, task_progress};
use gpui_kit::base::StyledExt;
use gpui_kit::*;
use idm_core::{DownloadStatus, DownloadTask};

/// 经典 Windows / IDM 界面调色板
#[derive(Clone, Copy)]
pub struct IdmPalette {
    pub menu_bg: Rgba,
    pub toolbar_bg: Rgba,
    pub panel_bg: Rgba,
    pub header_bg: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub selected_bg: Rgba,
    pub selected_border: Rgba,
    pub progress_fill: Rgba,
    pub progress_bg: Rgba,
}

impl IdmPalette {
    pub fn classic() -> Self {
        Self {
            menu_bg: rgb(0xf5f6f7),
            toolbar_bg: rgb(0xf0f4f9),
            panel_bg: rgb(0xffffff),
            header_bg: rgb(0xf3f4f6),
            border: rgb(0xd0d7de),
            text: rgb(0x111827),
            muted: rgb(0x57606a),
            selected_bg: rgb(0xcce8ff),
            selected_border: rgb(0x99d1ff),
            progress_fill: rgb(0x2da44e),
            progress_bg: rgb(0xe5e7eb),
        }
    }
}

/// 渲染 IDM 经典“上图标 + 下文字”工具栏按钮
pub fn idm_tool_button(
    id: &'static str,
    icon: &'static str,
    icon_color: Rgba,
    label: &'static str,
    p: IdmPalette,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .w(px(64.0))
        .py_1p5()
        .rounded_sm()
        .cursor_pointer()
        .child(div().text_lg().text_color(icon_color).child(icon))
        .child(div().text_xs().text_color(p.text).child(label))
}

/// 渲染经典 IDM 任务列表表头（带列分隔线）
pub fn render_idm_table_header(p: IdmPalette) -> impl IntoElement {
    let col = |w: Option<f32>, label: &'static str| {
        let base = div()
            .px_2()
            .py_1()
            .border_r_1()
            .border_color(p.border)
            .text_xs()
            .font_medium()
            .text_color(p.text)
            .child(label);
        match w {
            Some(width) => base.w(px(width)),
            None => base.flex_1(),
        }
    };

    div()
        .flex()
        .items_center()
        .bg(p.header_bg)
        .border_b_1()
        .border_color(p.border)
        .child(col(None, "文件名"))
        .child(col(Some(88.0), "大小"))
        .child(col(Some(145.0), "状态"))
        .child(col(Some(82.0), "剩余时间"))
        .child(col(Some(96.0), "传输速率"))
        .child(col(Some(100.0), "分类"))
}

/// 渲染经典 IDM 紧凑型任务行
pub fn render_idm_task_row(
    idx: usize,
    task: &DownloadTask,
    is_selected: bool,
    p: IdmPalette,
) -> Stateful<Div> {
    let pct = task_progress(task.completed_bytes, task.total_bytes);
    let status_text = match task.status {
        DownloadStatus::Active => format!("{pct:.1}%"),
        DownloadStatus::Waiting => "等待中".to_string(),
        DownloadStatus::Paused => format!("已暂停 ({pct:.1}%)"),
        DownloadStatus::Completed => "已完成".to_string(),
        DownloadStatus::Error => "下载出错".to_string(),
    };

    div()
        .id(("idm-row", idx))
        .flex()
        .items_center()
        .py_1()
        .bg(if is_selected { p.selected_bg } else { p.panel_bg })
        .border_b_1()
        .border_color(if is_selected { p.selected_border } else { rgb(0xf0f0f0) })
        .text_xs()
        .text_color(p.text)
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .px_2()
                .overflow_hidden()
                .child(task.filename.clone()),
        )
        .child(div().w(px(88.0)).px_2().child(format_bytes(task.total_bytes)))
        .child(
            div()
                .w(px(145.0))
                .px_2()
                .flex()
                .items_center()
                .gap_1p5()
                .child(
                    div()
                        .w(px(56.0))
                        .h(px(10.0))
                        .bg(p.progress_bg)
                        .border_1()
                        .border_color(p.border)
                        .child(div().h_full().w(relative(pct / 100.0)).bg(p.progress_fill)),
                )
                .child(status_text),
        )
        .child(
            div()
                .w(px(82.0))
                .px_2()
                .text_color(p.muted)
                .child(format_eta(
                    task.completed_bytes,
                    task.total_bytes,
                    task.speed_bytes_per_sec,
                )),
        )
        .child(
            div()
                .w(px(96.0))
                .px_2()
                .child(format_speed(task.speed_bytes_per_sec)),
        )
        .child(
            div()
                .w(px(100.0))
                .px_2()
                .text_color(p.muted)
                .child(task.category.display_name()),
        )
}

/// 渲染底部经典 IDM 风格选中任务详情与 32 槽动态分片位图面板（支持收起/展开伸缩与关闭，原始链接支持水平滚动且绝不折行撑高面板）
pub fn render_task_inspector(
    task: &DownloadTask,
    collapsed: bool,
    p: IdmPalette,
    on_toggle_collapse: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_file: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_reveal_dir: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_url: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_refresh_url: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete_with_file: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let slots = task.piece_slots(32);
    let pct = task_progress(task.completed_bytes, task.total_bytes);
    let action_btn = |id: &'static str, label: &'static str, color: Rgba| {
        div()
            .id(id)
            .px_2p5()
            .py_0p5()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.panel_bg)
            .text_xs()
            .text_color(color)
            .cursor_pointer()
            .child(label)
    };

    let collapse_label = if collapsed {
        "▴ 展开详情"
    } else {
        "▾ 收起详情"
    };

    let mut root = div()
        .flex()
        .flex_col()
        .gap_1p5()
        .px_3()
        .py_2()
        .bg(p.header_bg)
        .border_t_1()
        .border_color(p.border)
        .text_xs()
        .text_color(p.text)
        .child(
            // 第一行：左侧文件名（超长单行不折行），右侧统计信息 + 展开/收起按钮 + 关闭按钮
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .h(px(18.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .font_semibold()
                        .child(format!("任务详情: {}", task.filename)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_color(p.muted).child(format!(
                            "已下载 {} / {} ({pct:.1}%)  |  连接: {}  |  分片: {}",
                            format_bytes(task.completed_bytes),
                            format_bytes(task.total_bytes),
                            task.connections,
                            task.num_pieces.max(1)
                        )))
                        .child(
                            action_btn("insp-toggle", collapse_label, p.text)
                                .on_click(on_toggle_collapse),
                        )
                        .child(action_btn("insp-close", "✕ 关闭", p.muted).on_click(on_close)),
                ),
        )
        .child(
            // 第二行：独立操作按钮栏
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1p5()
                .child(action_btn("insp-open", "▶ 打开文件", rgb(0x16a34a)).on_click(on_open_file))
                .child(action_btn("insp-dir", "📁 定位文件", rgb(0xd97706)).on_click(on_reveal_dir))
                .child(action_btn("insp-copy", "🔗 复制链接", rgb(0x2563eb)).on_click(on_copy_url))
                .child(
                    action_btn("insp-refresh", "↻ 刷新链接/重下", rgb(0x0284c7))
                        .on_click(on_refresh_url),
                )
                .child(
                    action_btn("insp-del-file", "🗑 彻底删除(含本地文件)", rgb(0xdc2626))
                        .on_click(on_delete_with_file),
                ),
        );

    if !collapsed {
        root = root
            .child(
                // 第三行：32 槽动态分片进度图
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(64.0)).text_color(p.muted).child("分片进度:"))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .gap_0p5()
                            .p_0p5()
                            .bg(p.panel_bg)
                            .border_1()
                            .border_color(p.border)
                            .children(slots.into_iter().map(|done| {
                                div()
                                    .flex_1()
                                    .h(px(10.0))
                                    .bg(if done { rgb(0x2563eb) } else { p.progress_bg })
                            })),
                    ),
            )
            .child(
                // 第四行：保存路径与原始链接（单行固定高度 + 水平滚动框，彻底解决超长链接折行撑高面板问题）
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_color(p.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .w(px(300.0))
                            .child(div().child("保存路径:"))
                            .child(
                                div()
                                    .id("insp-path-scroll")
                                    .flex_1()
                                    .h(px(22.0))
                                    .px_1p5()
                                    .flex()
                                    .items_center()
                                    .bg(p.panel_bg)
                                    .border_1()
                                    .border_color(p.border)
                                    .rounded_sm()
                                    .overflow_x_scroll()
                                    .whitespace_nowrap()
                                    .child(task.full_path().display().to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .flex_1()
                            .overflow_hidden()
                            .child(div().child("原始链接:"))
                            .child(
                                div()
                                    .id("insp-url-scroll")
                                    .flex_1()
                                    .h(px(22.0))
                                    .px_1p5()
                                    .flex()
                                    .items_center()
                                    .bg(p.panel_bg)
                                    .border_1()
                                    .border_color(p.border)
                                    .rounded_sm()
                                    .overflow_x_scroll()
                                    .whitespace_nowrap()
                                    .child(task.url.clone()),
                            ),
                    ),
            );
    }

    root
}
