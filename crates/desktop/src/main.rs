#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Gravita (引力下载器) 桌面客户端入口（基于 Rust + GPUI + gpui-kit）

mod batch_dialog;
mod dialogs;
mod engine;
mod settings_dialog;
mod settings_panel;
mod state;
mod system;
mod tray;
mod ui;
mod ui_parts;

use gpui_kit::component::Root;
use gpui_kit::*;
use ui::IdmMainView;

fn main() {
    let app = gpui_kit::application();
    app.run(move |cx| {
        gpui_kit::init(cx);

        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Gravita 下载器".into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1080.0), px(680.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            // 点击窗口右上角关闭按钮时默认隐藏到系统托盘，不退出程序（由托盘右键「退出」彻底结束）
            window.on_window_should_close(cx, |_, _| {
                settings_panel::hide_window("Gravita 下载器");
                false
            });
            let view = cx.new(|cx| IdmMainView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("打开 Gravita 主窗口失败");

        cx.activate(true);
    });
}
