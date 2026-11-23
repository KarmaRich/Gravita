//! 「关于 Gravita」软件信息弹窗与「发现新版本」自动更新提示弹窗

use crate::dialogs::modal_shell;
use crate::ui_parts::IdmPalette;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use idm_core::{CURRENT_VERSION, GITHUB_REPO_URL, UpdateInfo};

/// 渲染「关于 Gravita」居中模态窗口
pub fn render_about_dialog(
    aria2_version: Option<&str>,
    update_status: Option<&str>,
    is_checking: bool,
    p: IdmPalette,
    on_open_github: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_github: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_check_update: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let engine_ver = aria2_version
        .map(|v| format!("aria2 v{v} (OpenSSL 128-Thread Enhanced)"))
        .unwrap_or_else(|| "未连接".into());

    let info_row = |label: &'static str, value: String| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .py_1()
            .border_b_1()
            .border_color(rgb(0xf0f0f0))
            .text_xs()
            .child(div().text_color(p.muted).child(label))
            .child(div().font_medium().text_color(p.text).child(value))
    };

    let status_text = if is_checking {
        "正在连接 GitHub Releases 检查最新版本...".to_string()
    } else {
        update_status
            .unwrap_or("点击下方「检查更新」可检测 GitHub Release 最新版本")
            .to_string()
    };

    let body = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .p_3()
                .rounded_md()
                .bg(p.header_bg)
                .border_1()
                .border_color(p.border)
                .child(
                    div()
                        .w(px(44.0))
                        .h(px(44.0))
                        .rounded_md()
                        .bg(rgb(0x1e1b4b))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_lg()
                        .font_bold()
                        .text_color(rgb(0x06b6d4))
                        .child("⬇"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(p.text)
                                .child(format!("Gravita (引力下载器)  v{CURRENT_VERSION}")),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.muted)
                                .child("基于 Rust + GPUI + Aria2 构建的高性能多线程桌面下载器"),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .px_1()
                .child(info_row("软件版本", format!("v{CURRENT_VERSION}")))
                .child(info_row("下载引擎", engine_ver))
                .child(info_row("UI 渲染引擎", "Rust GPUI (GPU 硬件加速)".into()))
                .child(info_row("开源许可证", "MIT License".into()))
                .child(info_row("GitHub 仓库", GITHUB_REPO_URL.into())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_2p5()
                .py_1p5()
                .rounded_sm()
                .bg(p.toolbar_bg)
                .border_1()
                .border_color(p.border)
                .text_xs()
                .text_color(rgb(0x2563eb))
                .child(status_text),
        );

    let check_label = if is_checking { "检查中..." } else { "🔄 检查更新" };
    let footer = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            Button::new("btn-about-copy-gh")
                .label("📋 复制链接")
                .on_click(on_copy_github),
        )
        .child(
            Button::new("btn-about-open-gh")
                .label("🌐 GitHub 主页")
                .on_click(on_open_github),
        )
        .child(
            Button::new("btn-about-check-upd")
                .primary()
                .label(check_label)
                .on_click(on_check_update),
        )
        .child(
            Button::new("btn-about-close")
                .label("关闭")
                .on_click(on_cancel),
        );

    modal_shell("关于 Gravita", 500.0, p, on_close, body, footer)
}

/// 渲染「发现新版本」自动更新提示模态弹窗（支持忽略此版本、打开 GitHub Release 或立即下载安装包）
pub fn render_update_dialog(
    info: &UpdateInfo,
    p: IdmPalette,
    on_ignore_version: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_release: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_download_now: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_later: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let asset_hint = match &info.asset_name {
        Some(name) => format!("已匹配当前系统安装包: {name}"),
        None => "未找到当前平台直接安装包，可点击「前往 GitHub 下载」手动选择".into(),
    };

    let body = div()
        .flex()
        .flex_col()
        .gap_2p5()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .p_2p5()
                .rounded_sm()
                .bg(p.selected_bg)
                .border_1()
                .border_color(p.selected_border)
                .text_xs()
                .child(
                    div()
                        .font_semibold()
                        .text_color(p.text)
                        .child(format!("发现新版本: {} ({})", info.release_name, info.latest_version)),
                )
                .child(
                    div()
                        .text_color(p.muted)
                        .child(format!("当前版本: {}  |  发布日期: {}", info.current_version, info.published_at)),
                ),
        )
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(p.text)
                .child("版本更新说明 (Release Notes):"),
        )
        .child(
            div()
                .id("update-notes-scroll")
                .h(px(160.0))
                .p_2p5()
                .rounded_sm()
                .bg(p.header_bg)
                .border_1()
                .border_color(p.border)
                .overflow_y_scroll()
                .text_xs()
                .text_color(p.text)
                .flex()
                .flex_col()
                .gap_1()
                .children(info.release_notes.lines().map(|line| {
                    div().child(if line.trim().is_empty() {
                        " ".to_string()
                    } else {
                        line.to_string()
                    })
                })),
        )
        .child(div().text_xs().text_color(rgb(0x2563eb)).child(asset_hint));

    let download_label = if info.download_url.is_some() {
        "⬇ 立即下载并更新"
    } else {
        "🌐 前往 GitHub 下载"
    };

    let footer = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            Button::new("btn-upd-ignore")
                .label(format!("忽略此版本 ({})", info.latest_version))
                .on_click(on_ignore_version),
        )
        .child(
            Button::new("btn-upd-later")
                .label("稍后提醒")
                .on_click(on_later),
        )
        .child(
            Button::new("btn-upd-github")
                .label("查看 Release 页")
                .on_click(on_open_release),
        )
        .child(
            Button::new("btn-upd-now")
                .primary()
                .label(download_label)
                .on_click(on_download_now),
        );

    modal_shell("发现新版本更新", 560.0, p, on_close, body, footer)
}
