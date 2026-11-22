//! Windows 系统托盘图标与右键快捷菜单模块

use crate::engine::EngineCommand;
use crate::settings_panel::{bring_window_to_front, hide_window};
use tokio::sync::mpsc;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

const WINDOW_TITLE: &str = "IDM-Aria2-RS 下载器";

/// 生成 32x32 蓝色下载箭头 RGBA 托盘图标（无需外部图片文件）
pub fn build_tray_rgba_icon() -> (Vec<u8>, u32, u32) {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            // 蓝色圆底 + 白色向下箭头
            if dx * dx + dy * dy <= 14 * 14 {
                let in_shaft = (13..=18).contains(&x) && (7..=19).contains(&y);
                let in_head = (17..=24).contains(&y)
                    && (x as i32 - 16).abs() <= (24 - y as i32);
                if in_shaft || in_head {
                    rgba[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
                } else {
                    rgba[idx..idx + 4].copy_from_slice(&[37, 99, 235, 255]);
                }
            }
        }
    }
    (rgba, size, size)
}

/// 在独立后台线程启动系统托盘图标与右键菜单事件循环
pub fn start_system_tray(cmd_tx: mpsc::UnboundedSender<EngineCommand>) {
    #[cfg(windows)]
    std::thread::spawn(move || {
        let (rgba, w, h) = build_tray_rgba_icon();
        let Ok(icon) = Icon::from_rgba(rgba, w, h) else {
            return;
        };

        let menu = Menu::new();
        let item_show = MenuItem::new("显示主窗口", true, None);
        let item_hide = MenuItem::new("隐藏到托盘", true, None);
        let item_pause_all = MenuItem::new("全部暂停", true, None);
        let item_resume_all = MenuItem::new("全部继续", true, None);
        let item_quit = MenuItem::new("退出", true, None);

        let _ = menu.append(&item_show);
        let _ = menu.append(&item_hide);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&item_pause_all);
        let _ = menu.append(&item_resume_all);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&item_quit);

        let Ok(_tray) = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("IDM-Aria2-RS 下载器")
            .with_icon(icon)
            .build()
        else {
            return;
        };

        let show_id = item_show.id().clone();
        let hide_id = item_hide.id().clone();
        let pause_id = item_pause_all.id().clone();
        let resume_id = item_resume_all.id().clone();
        let quit_id = item_quit.id().clone();

        // 后台轮询托盘事件（左键单击唤起主窗口，右键菜单触发对应操作）
        std::thread::spawn(move || {
            let menu_rx = MenuEvent::receiver();
            let tray_rx = TrayIconEvent::receiver();
            loop {
                if let Ok(ev) = tray_rx.try_recv() {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        bring_window_to_front(WINDOW_TITLE);
                    }
                }
                if let Ok(ev) = menu_rx.try_recv() {
                    if ev.id == show_id {
                        bring_window_to_front(WINDOW_TITLE);
                    } else if ev.id == hide_id {
                        hide_window(WINDOW_TITLE);
                    } else if ev.id == pause_id {
                        let _ = cmd_tx.send(EngineCommand::PauseAll);
                    } else if ev.id == resume_id {
                        let _ = cmd_tx.send(EngineCommand::UnpauseAll);
                    } else if ev.id == quit_id {
                        let _ = cmd_tx.send(EngineCommand::ShutdownAndExit);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        // 运行轻量 Win32 消息循环以保持托盘图标响应
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetMessageW(msg: *mut [u8; 48], hwnd: isize, min: u32, max: u32) -> i32;
            fn TranslateMessage(msg: *const [u8; 48]) -> i32;
            fn DispatchMessageW(msg: *const [u8; 48]) -> isize;
        }
        let mut msg = [0u8; 48];
        unsafe {
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    #[cfg(not(windows))]
    {
        let _ = cmd_tx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tray_rgba_icon_dimensions() {
        let (rgba, w, h) = build_tray_rgba_icon();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(rgba.len(), (32 * 32 * 4) as usize);
    }
}
