//! 桌面端居中模态弹窗组件（仿 IDM「新建下载任务」与「选项设置」弹窗）

use crate::settings_panel::SettingsInputs;
use crate::ui_parts::IdmPalette;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;

/// 新建下载弹窗的输入状态集合
pub struct AddTaskInputs {
    pub url: Entity<InputState>,
    pub filename: Entity<InputState>,
    pub save_dir: Entity<InputState>,
}

impl AddTaskInputs {
    pub fn new(default_dir: &str, window: &mut Window, cx: &mut App) -> Self {
        Self {
            url: cx.new(|cx| {
                InputState::new(window, cx).placeholder("请输入 HTTP / HTTPS / 磁力下载链接")
            }),
            filename: cx.new(|cx| {
                InputState::new(window, cx).placeholder("留空则自动从链接识别文件名")
            }),
            save_dir: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("留空则按文件分类自动归档")
                    .default_value(default_dir.to_string())
            }),
        }
    }
}

/// 通用居中模态窗口外壳（半透明遮罩 + 标题栏 + 内容区 + 底部按钮区）
fn modal_shell(
    title: &'static str,
    width: f32,
    p: IdmPalette,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    body: impl IntoElement,
    footer: impl IntoElement,
) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000055))
        .child(
            div()
                .w(px(width))
                .bg(p.panel_bg)
                .border_1()
                .border_color(p.border)
                .rounded_md()
                .shadow_lg()
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_2p5()
                        .bg(p.header_bg)
                        .border_b_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(p.text)
                                .child(title),
                        )
                        .child(
                            div()
                                .id("modal-close-x")
                                .px_2()
                                .py_0p5()
                                .rounded_sm()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(p.muted)
                                .on_click(on_close)
                                .child("✕"),
                        ),
                )
                .child(div().p_4().flex().flex_col().gap_3().child(body))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_2()
                        .px_4()
                        .py_2p5()
                        .bg(p.menu_bg)
                        .border_t_1()
                        .border_color(p.border)
                        .child(footer),
                ),
        )
}

/// 表单单行（左标签 + 右输入框）
fn form_row(label: &'static str, input: &Entity<InputState>, p: IdmPalette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(div().w(px(88.0)).text_xs().text_color(p.text).child(label))
        .child(div().flex_1().child(Input::new(input)))
}

/// 渲染「新建下载任务」居中弹窗
pub fn render_add_task_modal(
    inputs: &AddTaskInputs,
    p: IdmPalette,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let body = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(form_row("下载地址:", &inputs.url, p))
        .child(form_row("保存文件名:", &inputs.filename, p))
        .child(form_row("保存目录:", &inputs.save_dir, p));

    let footer = div()
        .flex()
        .items_center()
        .gap_2()
        .child(Button::new("btn-add-cancel").label("取消").on_click(on_cancel))
        .child(
            Button::new("btn-add-ok")
                .primary()
                .label("开始下载")
                .on_click(on_submit),
        );

    modal_shell("新建下载任务", 520.0, p, on_close, body, footer)
}

/// 渲染「选项设置」居中弹窗
pub fn render_settings_modal(
    inputs: &SettingsInputs,
    p: IdmPalette,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let body = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(form_row("默认下载目录:", &inputs.download_dir, p))
        .child(form_row("单任务分片数:", &inputs.split, p))
        .child(form_row("全局下载限速:", &inputs.speed_limit, p))
        .child(form_row("代理服务器:", &inputs.proxy, p));

    let footer = div()
        .flex()
        .items_center()
        .gap_2()
        .child(Button::new("btn-opt-cancel").label("取消").on_click(on_cancel))
        .child(
            Button::new("btn-opt-save")
                .primary()
                .label("保存并应用")
                .on_click(on_save),
        );

    modal_shell("选项设置", 500.0, p, on_close, body, footer)
}
