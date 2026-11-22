//! IDM-Aria2-RS 桌面客户端入口（基于 Rust + GPUI + gpui-kit）

mod dialogs;
mod engine;
mod settings_dialog;
mod settings_panel;
mod state;
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
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1080.0), px(680.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| IdmMainView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("打开 IDM-Aria2-RS 主窗口失败");

        cx.activate(true);
    });
}
