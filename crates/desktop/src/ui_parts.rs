//! 界面调色板与任务表格/详情辅助渲染组件

use crate::state::{format_bytes, format_eta, format_speed, task_progress};
use gpui_kit::base::StyledExt;
use gpui_kit::*;
use idm_core::{DownloadStatus, DownloadTask};

/// 桌面端浅色/深色主题调色板
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Rgba,
    pub surface: Rgba,
    pub sidebar_bg: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub primary: Rgba,
    pub row_selected: Rgba,
    pub row_alt: Rgba,
    pub progress_track: Rgba,
}

impl Palette {
    pub fn for_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                bg: rgb(0x111827),
                surface: rgb(0x1f2937),
                sidebar_bg: rgb(0x161f2e),
                border: rgb(0x374151),
                text: rgb(0xf3f4f6),
                muted: rgb(0x9ca3af),
                primary: rgb(0x3b82f6),
                row_selected: rgb(0x1e3a5f),
                row_alt: rgb(0x182132),
                progress_track: rgb(0x374151),
            }
        } else {
            Self {
                bg: rgb(0xf8fafc),
                surface: rgb(0xffffff),
                sidebar_bg: rgb(0xf1f5f9),
                border: rgb(0xe2e8f0),
                text: rgb(0x0f172a),
                muted: rgb(0x64748b),
                primary: rgb(0x2563eb),
                row_selected: rgb(0xdbeafe),
                row_alt: rgb(0xf8fafc),
                progress_track: rgb(0xe2e8f0),
            }
        }
    }
}

/// 根据下载状态返回中文标签与对应的强调色
pub fn status_badge_style(status: DownloadStatus) -> (&'static str, Rgba, Rgba) {
    match status {
        DownloadStatus::Active => ("下载中", rgb(0x1d4ed8), rgb(0xdbeafe)),
        DownloadStatus::Waiting => ("等待中", rgb(0x475569), rgb(0xe2e8f0)),
        DownloadStatus::Paused => ("已暂停", rgb(0xb45309), rgb(0xfef3c7)),
        DownloadStatus::Completed => ("已完成", rgb(0x15803d), rgb(0xdcfce7)),
        DownloadStatus::Error => ("出错", rgb(0xb91c1c), rgb(0xfee2e2)),
    }
}

/// 渲染任务表格表头
pub fn render_table_header(p: Palette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px_4()
        .py_2()
        .bg(p.sidebar_bg)
        .border_b_1()
        .border_color(p.border)
        .text_xs()
        .font_semibold()
        .text_color(p.muted)
        .child(div().flex_1().child("文件名"))
        .child(div().w(px(96.0)).child("大小"))
        .child(div().w(px(170.0)).child("进度"))
        .child(div().w(px(100.0)).child("速度"))
        .child(div().w(px(80.0)).child("剩余时间"))
        .child(div().w(px(76.0)).child("状态"))
}

/// 渲染单行任务（含可视化进度条与彩色状态徽章）
pub fn render_task_row(
    idx: usize,
    task: &DownloadTask,
    is_selected: bool,
    p: Palette,
) -> Stateful<Div> {
    let pct = task_progress(task.completed_bytes, task.total_bytes);
    let (status_text, status_fg, status_bg) = status_badge_style(task.status);
    let row_bg = if is_selected {
        p.row_selected
    } else if idx % 2 == 1 {
        p.row_alt
    } else {
        p.surface
    };

    div()
        .id(("task-row", idx))
        .flex()
        .items_center()
        .px_4()
        .py_2p5()
        .bg(row_bg)
        .border_b_1()
        .border_color(p.border)
        .text_sm()
        .text_color(p.text)
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .pr_3()
                .overflow_hidden()
                .child(div().font_medium().child(task.filename.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(p.muted)
                        .child(task.category.display_name()),
                ),
        )
        .child(
            div()
                .w(px(96.0))
                .text_xs()
                .text_color(p.muted)
                .child(format_bytes(task.total_bytes)),
        )
        .child(
            div()
                .w(px(170.0))
                .pr_4()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .w_full()
                        .h(px(6.0))
                        .rounded_full()
                        .bg(p.progress_track)
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(relative(pct / 100.0))
                                .bg(if task.status == DownloadStatus::Completed {
                                    rgb(0x16a34a)
                                } else {
                                    p.primary
                                }),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(p.muted)
                        .child(format!("{pct:.1}%")),
                ),
        )
        .child(
            div()
                .w(px(100.0))
                .text_xs()
                .child(format_speed(task.speed_bytes_per_sec)),
        )
        .child(
            div()
                .w(px(80.0))
                .text_xs()
                .text_color(p.muted)
                .child(format_eta(
                    task.completed_bytes,
                    task.total_bytes,
                    task.speed_bytes_per_sec,
                )),
        )
        .child(
            div().w(px(76.0)).child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_full()
                    .bg(status_bg)
                    .text_color(status_fg)
                    .text_xs()
                    .font_medium()
                    .child(status_text),
            ),
        )
}

/// 渲染底部选中任务详情条
pub fn render_detail_panel(task: Option<&DownloadTask>, p: Palette) -> impl IntoElement {
    let content = match task {
        Some(t) => format!(
            "保存路径: {}   |   来源: {}",
            t.save_dir.display(),
            t.url
        ),
        None => "点击列表中的下载任务以查看详细路径与来源链接".to_string(),
    };
    div()
        .px_4()
        .py_2()
        .bg(p.surface)
        .border_t_1()
        .border_color(p.border)
        .text_xs()
        .text_color(p.muted)
        .child(content)
}
