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
