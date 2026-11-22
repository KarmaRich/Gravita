//! 弹窗内容组装：「新建任务」双 Tab 弹窗与「选项设置」4 分类 Tab 弹窗

use crate::dialogs::{AddTaskInputs, AddTaskTab, checkbox_row, form_row, modal_shell};
use crate::settings_panel::{SettingsInputs, SettingsTab};
use crate::ui_parts::IdmPalette;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::*;

/// 渲染「新建下载任务」完整弹窗（支持浏览目录、覆盖开关、限速、高级 HTTP）
pub fn render_add_task_dialog(
    inputs: &AddTaskInputs,
    active_tab: AddTaskTab,
    p: IdmPalette,
    on_tab: impl Fn(AddTaskTab, &mut Window, &mut App) + Clone + 'static,
    on_browse: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_overwrite: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tab_btn = |id: &'static str, tab: AddTaskTab, label: &'static str| {
        let on_tab = on_tab.clone();
        let active = active_tab == tab;
        div()
            .id(id)
            .px_3()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .bg(if active { p.selected_bg } else { p.menu_bg })
            .border_1()
            .border_color(if active { p.selected_border } else { p.border })
            .text_color(p.text)
            .on_click(move |_, w, cx| on_tab(tab, w, cx))
            .child(label)
    };

    let tabs = div()
        .flex()
        .gap_2()
        .pb_2()
        .border_b_1()
        .border_color(p.border)
        .child(tab_btn("add-tab-basic", AddTaskTab::Basic, "常规下载"))
        .child(tab_btn("add-tab-http", AddTaskTab::HttpAdvanced, "高级 HTTP / 代理"));

    let content = match active_tab {
        AddTaskTab::Basic => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(form_row("下载地址:", &inputs.url, p))
            .child(form_row("保存文件名:", &inputs.filename, p))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(96.0)).text_xs().text_color(p.text).child("保存目录:"))
                    .child(div().flex_1().child(Input::new(&inputs.save_dir)))
                    .child(Button::new("btn-add-browse").label("浏览...").on_click(on_browse)),
            )
            .child(form_row("单任务限速:", &inputs.speed_limit, p))
            .child(checkbox_row(
                "chk-add-overwrite",
                "若本地已存在同名文件，允许直接覆盖",
                inputs.allow_overwrite,
                p,
                on_toggle_overwrite,
            )),
        AddTaskTab::HttpAdvanced => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(form_row("Referer:", &inputs.referer, p))
            .child(form_row("User-Agent:", &inputs.user_agent, p))
            .child(form_row("Cookie:", &inputs.cookie, p))
            .child(form_row("单任务代理:", &inputs.proxy, p)),
    };

    let body = div().flex().flex_col().gap_3().child(tabs).child(content);
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

    modal_shell("新建下载任务", 560.0, p, on_close, body, footer)
}

/// 渲染「选项设置」4 分类标签页弹窗
pub fn render_settings_dialog(
    inputs: &SettingsInputs,
    active_tab: SettingsTab,
    p: IdmPalette,
    on_tab: impl Fn(SettingsTab, &mut Window, &mut App) + Clone + 'static,
    on_browse_dir: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_overwrite: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tabs = div()
        .flex()
        .gap_2()
        .pb_2()
        .border_b_1()
        .border_color(p.border)
        .children(SettingsTab::all().into_iter().enumerate().map(|(i, (tab, label))| {
            let on_tab = on_tab.clone();
            let active = active_tab == tab;
            div()
                .id(("opt-tab", i))
                .px_3()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .text_xs()
                .bg(if active { p.selected_bg } else { p.menu_bg })
                .border_1()
                .border_color(if active { p.selected_border } else { p.border })
                .text_color(p.text)
                .on_click(move |_, w, cx| on_tab(tab, w, cx))
                .child(label)
        }));

    let content = match active_tab {
        SettingsTab::General => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(96.0)).text_xs().text_color(p.text).child("默认下载目录:"))
                    .child(div().flex_1().child(Input::new(&inputs.download_dir)))
                    .child(Button::new("btn-opt-browse").label("浏览...").on_click(on_browse_dir)),
            )
            .child(form_row("最大同时下载:", &inputs.max_concurrent, p))
            .child(checkbox_row(
                "chk-opt-overwrite",
                "默认允许覆盖已存在的同名文件",
                inputs.allow_overwrite,
                p,
                on_toggle_overwrite,
            )),
        SettingsTab::Connection => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(form_row("单任务分片数:", &inputs.split, p))
            .child(form_row("单服最大连接:", &inputs.max_conn_per_server, p))
            .child(form_row("最小分片大小:", &inputs.min_split_size, p))
            .child(form_row("全局下载限速:", &inputs.speed_limit, p))
            .child(form_row("全局上传限速:", &inputs.upload_limit, p)),
        SettingsTab::Network => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(form_row("全局代理地址:", &inputs.proxy, p))
            .child(form_row("默认 UA:", &inputs.user_agent, p))
            .child(form_row("连接超时(秒):", &inputs.timeout_secs, p))
            .child(form_row("最大重试次数:", &inputs.max_tries, p)),
        SettingsTab::Engine => div()
            .flex()
            .flex_col()
            .gap_3()
            .child(form_row("RPC Secret:", &inputs.rpc_secret, p))
            .child(form_row("磁盘缓存大小:", &inputs.disk_cache, p))
            .child(form_row("文件预分配:", &inputs.file_allocation, p)),
    };

    let body = div().flex().flex_col().gap_3().child(tabs).child(content);
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

    modal_shell("选项设置", 560.0, p, on_close, body, footer)
}
