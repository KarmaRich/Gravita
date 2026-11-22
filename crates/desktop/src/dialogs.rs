//! 桌面端居中模态弹窗通用外壳与「新建下载任务」弹窗

use crate::ui_parts::IdmPalette;
use gpui_kit::base::StyledExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;

/// 新建任务弹窗标签页（基础 / 高级 HTTP）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddTaskTab {
    Basic,
    HttpAdvanced,
}

/// 新建下载弹窗的输入状态集合
pub struct AddTaskInputs {
    pub url: Entity<InputState>,
    pub filename: Entity<InputState>,
    pub save_dir: Entity<InputState>,
    pub speed_limit: Entity<InputState>,
    pub allow_overwrite: bool,
    /// 当前任务是否启用代理（默认 false 即直连；勾选后优先用 `proxy` 输入框，留空则用选项中的预设代理）
    pub use_proxy: bool,
    pub referer: Entity<InputState>,
    pub user_agent: Entity<InputState>,
    pub cookie: Entity<InputState>,
    pub proxy: Entity<InputState>,
    /// 远程链接探测状态提示（如 `文件大小: 128.50 MB | 支持多线程断点续传`）
    pub probe_status: Option<String>,
    /// 探测请求递增序号（防止旧的并发探测结果覆盖新结果）
    pub probe_seq: u64,
    /// 若当前弹窗由「刷新链接/重下」唤起，记录待替换清理的旧任务 GID
    pub replace_gid: Option<String>,
}

impl AddTaskInputs {
    pub fn new(default_dir: &str, allow_overwrite: bool, window: &mut Window, cx: &mut App) -> Self {
        let mk = |ph: &'static str, val: &str, window: &mut Window, cx: &mut App| {
            let v = val.to_string();
            cx.new(|cx| InputState::new(window, cx).placeholder(ph).default_value(v))
        };
        Self {
            url: mk("请输入 HTTP / HTTPS / 磁力下载链接", "", window, cx),
            filename: mk("留空则自动从链接识别文件名", "", window, cx),
            save_dir: mk("选择或输入保存目录", default_dir, window, cx),
            speed_limit: mk("0(不限) / 2M / 500K", "", window, cx),
            allow_overwrite,
            use_proxy: false,
            referer: mk("可选 Referer，如 https://example.com/", "", window, cx),
            user_agent: mk("可选自定义 User-Agent", "", window, cx),
            cookie: mk("可选 Cookie 请求头（用于需登录态下载）", "", window, cx),
            proxy: mk("留空则使用「选项」中的预设代理；填写则使用此独立代理", "", window, cx),
            probe_status: None,
            probe_seq: 0,
            replace_gid: None,
        }
    }

    /// 组装高级 HTTP 请求头列表（供 aria2 `header` 参数使用）
    pub fn build_headers(&self, cx: &App) -> Vec<String> {
        let mut headers = Vec::new();
        let referer = self.referer.read(cx).value().trim().to_string();
        if !referer.is_empty() {
            headers.push(format!("Referer: {referer}"));
        }
        let ua = self.user_agent.read(cx).value().trim().to_string();
        if !ua.is_empty() {
            headers.push(format!("User-Agent: {ua}"));
        }
        let cookie = self.cookie.read(cx).value().trim().to_string();
        if !cookie.is_empty() {
            headers.push(format!("Cookie: {cookie}"));
        }
        headers
    }
}

/// 通用居中模态窗口外壳
pub fn modal_shell(
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
                        .child(div().text_sm().font_semibold().text_color(p.text).child(title))
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
pub fn form_row(label: &'static str, input: &Entity<InputState>, p: IdmPalette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(div().w(px(96.0)).text_xs().text_color(p.text).child(label))
        .child(div().flex_1().child(Input::new(input)))
}

/// 勾选开关行
pub fn checkbox_row(
    id: &'static str,
    label: impl IntoElement,
    checked: bool,
    p: IdmPalette,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .pl(px(108.0))
        .cursor_pointer()
        .on_click(on_toggle)
        .child(
            div()
                .w(px(16.0))
                .h(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(p.border)
                .bg(if checked { rgb(0x2563eb) } else { p.panel_bg })
                .text_xs()
                .text_color(rgb(0xffffff))
                .child(if checked { "✓" } else { "" }),
        )
        .child(div().text_xs().text_color(p.text).child(label))
}
