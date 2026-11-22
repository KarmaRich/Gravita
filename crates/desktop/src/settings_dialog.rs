//! 弹窗内容组装：「新建任务」双 Tab 弹窗与「选项设置」4 分类 Tab 弹窗

use crate::dialogs::{AddTaskInputs, AddTaskTab, checkbox_row, form_row, modal_shell};
use crate::settings_panel::{SettingsFlag, SettingsInputs, SettingsTab};
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

/// 渲染「选项设置」5 分类标签页弹窗（严格对齐 ARIA2_OPTIONS.md）
pub fn render_settings_dialog(
    inputs: &SettingsInputs,
    active_tab: SettingsTab,
    p: IdmPalette,
    on_tab: impl Fn(SettingsTab, &mut Window, &mut App) + Clone + 'static,
    on_browse_dir: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_flag: impl Fn(SettingsFlag, &mut Window, &mut App) + Clone + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let chk = |id: &'static str, label: &'static str, checked: bool, flag: SettingsFlag| {
        let on_toggle = on_toggle_flag.clone();
        checkbox_row(id, label, checked, p, move |_, w, cx| on_toggle(flag, w, cx))
    };

    let tabs = div()
        .flex()
        .flex_wrap()
        .gap_1p5()
        .pb_2()
        .border_b_1()
        .border_color(p.border)
        .children(SettingsTab::all().into_iter().enumerate().map(|(i, (tab, label))| {
            let on_tab = on_tab.clone();
            let active = active_tab == tab;
            div()
                .id(("opt-tab", i))
                .px_2p5()
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
            .gap_2p5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(96.0)).text_xs().text_color(p.text).child("默认下载目录:"))
                    .child(div().flex_1().child(Input::new(&inputs.download_dir)))
                    .child(Button::new("btn-opt-browse").label("浏览...").on_click(on_browse_dir)),
            )
            .child(form_row("文件预分配*:", &inputs.file_allocation, p))
            .child(form_row("磁盘缓存*:", &inputs.disk_cache, p))
            .child(chk("f-cont", "启用断点续传 (continue)", inputs.continue_download, SettingsFlag::ContinueDownload))
            .child(chk("f-over", "允许覆盖已存在同名文件 (allow-overwrite)", inputs.allow_overwrite, SettingsFlag::AllowOverwrite))
            .child(chk("f-ren", "同名文件自动重命名 (auto-file-renaming)", inputs.auto_file_renaming, SettingsFlag::AutoFileRenaming))
            .child(chk("f-rtime", "保留服务器修改时间 (remote-time)", inputs.remote_time, SettingsFlag::RemoteTime)),
        SettingsTab::Connection => div()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(form_row("最大同时下载:", &inputs.max_concurrent, p))
            .child(form_row("单任务分片数:", &inputs.split, p))
            .child(form_row("单服最大连接:", &inputs.max_conn_per_server, p))
            .child(form_row("最小分片大小:", &inputs.min_split_size, p))
            .child(form_row("全局下载限速:", &inputs.speed_limit, p))
            .child(form_row("全局上传限速:", &inputs.upload_limit, p))
            .child(form_row("最低速度断连:", &inputs.lowest_speed_limit, p)),
        SettingsTab::HttpProxy => div()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(form_row("全局代理地址:", &inputs.proxy, p))
            .child(form_row("代理白名单:", &inputs.no_proxy, p))
            .child(form_row("默认 UA:", &inputs.user_agent, p))
            .child(form_row("连接超时(秒):", &inputs.connect_timeout, p))
            .child(form_row("读写超时(秒):", &inputs.timeout_secs, p))
            .child(form_row("最大重试次数:", &inputs.max_tries, p))
            .child(form_row("重试间隔(秒):", &inputs.retry_wait, p))
            .child(chk("f-cert", "校验 HTTPS 证书* (check-certificate)", inputs.check_certificate, SettingsFlag::CheckCertificate))
            .child(chk("f-gzip", "启用 HTTP Gzip 压缩 (http-accept-gzip)", inputs.http_accept_gzip, SettingsFlag::HttpAcceptGzip)),
        SettingsTab::BitTorrent => div()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(form_row("最大连接节点:", &inputs.bt_max_peers, p))
            .child(form_row("分享率上限:", &inputs.seed_ratio, p))
            .child(form_row("做种时间(分):", &inputs.seed_time, p))
            .child(form_row("附加 Tracker:", &inputs.bt_tracker, p))
            .child(chk("f-dht", "启用 IPv4 DHT 网络* (enable-dht)", inputs.enable_dht, SettingsFlag::EnableDht))
            .child(chk("f-pex", "启用 PEX 节点交换* (enable-peer-exchange)", inputs.enable_peer_exchange, SettingsFlag::EnablePeerExchange))
            .child(chk("f-meta", "保存磁力 .torrent 元数据 (bt-save-metadata)", inputs.bt_save_metadata, SettingsFlag::BtSaveMetadata)),
        SettingsTab::EngineSession => div()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(form_row("RPC Secret*:", &inputs.rpc_secret, p))
            .child(form_row("会话保存间隔*:", &inputs.save_session_interval, p))
            .child(chk("f-sess", "启用会话持久化*（自动保存/恢复未完成任务）", inputs.enable_session_persistence, SettingsFlag::EnableSessionPersistence))
            .child(chk("f-ipv6", "禁用 IPv6* (disable-ipv6)", inputs.disable_ipv6, SettingsFlag::DisableIpv6))
            .child(
                div()
                    .pt_1()
                    .text_xs()
                    .text_color(p.muted)
                    .child("注：带 * 号的配置项将在下次重启应用 / 引擎时生效，其余项立即热更新生效。"),
            ),
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

    modal_shell("选项设置", 600.0, p, on_close, body, footer)
}
