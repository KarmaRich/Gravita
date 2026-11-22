//! 桌面端选项设置面板与系统文件夹打开辅助模块

use crate::ui_parts::IdmPalette;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use idm_core::AppSettings;
use std::path::{Path, PathBuf};

/// 选项面板输入状态集合
pub struct SettingsInputs {
    pub download_dir: Entity<InputState>,
    pub split: Entity<InputState>,
    pub speed_limit: Entity<InputState>,
    pub proxy: Entity<InputState>,
}

impl SettingsInputs {
    /// 根据当前配置初始化各输入框
    pub fn from_settings(settings: &AppSettings, window: &mut Window, cx: &mut App) -> Self {
        let dir_str = settings.default_download_dir.to_string_lossy().into_owned();
        let split_str = settings.default_split.to_string();
        let limit_str = settings.max_overall_download_limit.clone();
        let proxy_str = settings.proxy.clone().unwrap_or_default();

        Self {
            download_dir: cx.new(|cx| InputState::new(window, cx).default_value(dir_str)),
            split: cx.new(|cx| InputState::new(window, cx).default_value(split_str)),
            speed_limit: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("如 0(不限) / 5M / 500K")
                    .default_value(limit_str)
            }),
            proxy: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("如 http://127.0.0.1:7890（留空直连）")
                    .default_value(proxy_str)
            }),
        }
    }

    /// 读取用户输入并更新到 `AppSettings`
    pub fn apply_to(&self, settings: &mut AppSettings, cx: &App) {
        let dir = self.download_dir.read(cx).value().trim().to_string();
        if !dir.is_empty() {
            settings.default_download_dir = PathBuf::from(dir);
        }
        if let Ok(split) = self.split.read(cx).value().trim().parse::<u32>() {
            settings.default_split = split.clamp(1, 64);
        }
        let limit = self.speed_limit.read(cx).value().trim().to_string();
        settings.max_overall_download_limit = if limit.is_empty() { "0".into() } else { limit };

        let proxy = self.proxy.read(cx).value().trim().to_string();
        settings.proxy = if proxy.is_empty() { None } else { Some(proxy) };
    }
}

/// 渲染经典选项设置条（位于工具栏下方）
pub fn render_settings_bar(
    inputs: &SettingsInputs,
    p: IdmPalette,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let field = |label: &'static str, input: &Entity<InputState>, width: f32| {
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .child(div().text_xs().text_color(p.text).child(label))
            .child(div().w(px(width)).child(Input::new(input)))
    };

    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_3()
        .px_3()
        .py_2()
        .bg(p.menu_bg)
        .border_b_1()
        .border_color(p.border)
        .child(field("下载目录:", &inputs.download_dir, 180.0))
        .child(field("分片数:", &inputs.split, 60.0))
        .child(field("全局限速:", &inputs.speed_limit, 90.0))
        .child(field("代理:", &inputs.proxy, 190.0))
        .child(
            Button::new("btn-save-settings")
                .primary()
                .label("保存并应用")
                .on_click(on_save),
        )
}

/// 在系统文件管理器中打开指定目录（Windows 使用 `explorer`）
pub fn open_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}
