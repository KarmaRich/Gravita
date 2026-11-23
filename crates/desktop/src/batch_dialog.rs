//! 批量下载 / 通配符导入居中模态弹窗

use crate::dialogs::{checkbox_row, modal_shell};
use crate::ui_parts::IdmPalette;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use idm_core::batch::expand_batch_urls;

/// 批量下载弹窗输入状态
pub struct BatchTaskInputs {
    pub urls: Entity<InputState>,
    pub save_dir: Entity<InputState>,
    /// 批量任务是否使用代理（默认 false 不使用代理）
    pub use_proxy: bool,
}

impl BatchTaskInputs {
    pub fn new(default_dir: &str, window: &mut Window, cx: &mut App) -> Self {
        let dir = default_dir.to_string();
        Self {
            urls: cx.new(|cx| {
                InputState::new(window, cx).placeholder(
                    "输入通配符如 https://example.com/ep_[01-10].mp4，或多条链接（支持换行 / 逗号 / 分号分隔）",
                )
            }),
            save_dir: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("选择或输入保存目录")
                    .default_value(dir)
            }),
            use_proxy: false,
        }
    }
}

/// 渲染「批量下载」居中弹窗
pub fn render_batch_dialog(
    inputs: &BatchTaskInputs,
    preview_count: usize,
    configured_proxy: Option<&str>,
    p: IdmPalette,
    on_browse: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_proxy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let proxy_label = match configured_proxy.filter(|s| !s.trim().is_empty()) {
        Some(px) => format!("使用代理下载 ({px})"),
        None => "使用代理下载".to_string(),
    };

    let body = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_xs()
                .text_color(p.muted)
                .child("支持多条链接（换行、英文逗号 , 或分号 ; 分隔），以及多重通配符（如 [01-10]、[a-d]）："),
        )
        .child(div().h(px(140.0)).child(Input::new(&inputs.urls)))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(72.0)).text_xs().text_color(p.text).child("保存目录:"))
                .child(div().flex_1().child(Input::new(&inputs.save_dir)))
                .child(Button::new("btn-batch-browse").label("浏览...").on_click(on_browse)),
        )
        .child(checkbox_row(
            "chk-batch-proxy",
            proxy_label,
            inputs.use_proxy,
            p,
            on_toggle_proxy,
        ))
        .child(
            div()
                .text_xs()
                .text_color(p.text)
                .child(format!("预计展开下载任务数：{preview_count} 条")),
        );

    let footer = div()
        .flex()
        .items_center()
        .gap_2()
        .child(Button::new("btn-batch-cancel").label("取消").on_click(on_cancel))
        .child(
            Button::new("btn-batch-ok")
                .primary()
                .label(format!("批量开始 ({preview_count})"))
                .on_click(on_submit),
        );

    modal_shell("批量下载 / 通配符导入", 580.0, p, on_close, body, footer)
}

/// 便捷预览展开后的 URL 列表
pub fn preview_batch_urls(inputs: &BatchTaskInputs, cx: &App) -> Vec<String> {
    expand_batch_urls(&inputs.urls.read(cx).value())
}
