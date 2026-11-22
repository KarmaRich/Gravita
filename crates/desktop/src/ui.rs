//! 仿经典 IDM (Internet Download Manager) 桌面主界面（已接入真实 aria2 后台引擎）

use crate::engine::{DownloadEngine, EngineCommand, EngineSnapshot};
use crate::settings_panel::{SettingsInputs, open_directory, render_settings_bar};
use crate::state::{DesktopState, SidebarFilter, format_speed};
use crate::ui_parts::{
    IdmPalette, idm_tool_button, render_idm_table_header, render_idm_task_row, render_menubar,
};
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use idm_core::{AppSettings, DownloadStatus, DownloadTask, FileCategory};

/// IDM 主窗口视图实体
pub struct IdmMainView {
    state: DesktopState,
    engine: DownloadEngine,
    snapshot: EngineSnapshot,
    show_add_bar: bool,
    show_settings: bool,
    url_input: Entity<InputState>,
    settings_inputs: SettingsInputs,
    notice: Option<String>,
}

impl IdmMainView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = DesktopState::with_demo_tasks();
        // 优先从本地配置文件加载持久化设置
        let cfg_path = AppSettings::default_config_path();
        if let Ok(loaded) = AppSettings::load_from_file(&cfg_path) {
            state.settings = loaded;
        }

        let engine = DownloadEngine::start(state.settings.aria2.clone());
        // 启动后立即将本地已保存的限速/代理配置同步给内置 aria2c
        engine.send(EngineCommand::ApplyGlobalOptions {
            max_overall_download_limit: state.settings.max_overall_download_limit.clone(),
            proxy: state.settings.proxy.clone(),
        });

        let url_input = cx.new(|cx| InputState::new(window, cx).placeholder("请输入 HTTP/HTTPS/磁力下载链接..."));
        let settings_inputs = SettingsInputs::from_settings(&state.settings, window, cx);

        // 启动轻量 UI 定时刷新循环：每 500ms 同步一次后台 aria2 引擎快照
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                let ok = this.update(cx, |view, cx| {
                    view.snapshot = view.engine.snapshot();
                    cx.notify();
                });
                if ok.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            state,
            engine,
            snapshot: EngineSnapshot::default(),
            show_add_bar: false,
            show_settings: false,
            url_input,
            settings_inputs,
            notice: None,
        }
    }

    /// 返回当前视图应展示的任务列表（已连接 aria2 时展示真实任务，否则展示空或离线提示）
    fn visible_tasks(&self) -> Vec<DownloadTask> {
        let source: Vec<DownloadTask> = if self.snapshot.connected {
            self.snapshot.tasks.clone()
        } else {
            Vec::new()
        };

        source
            .into_iter()
            .filter(|t| match self.state.active_filter {
                SidebarFilter::All => true,
                SidebarFilter::Active => t.status == DownloadStatus::Active,
                SidebarFilter::Paused => t.status == DownloadStatus::Paused,
                SidebarFilter::Completed => t.status == DownloadStatus::Completed,
                SidebarFilter::Category(cat) => t.category == cat,
            })
            .collect()
    }

    /// 统计某个分类下的真实任务数
    fn count_for(&self, filter: SidebarFilter) -> usize {
        if !self.snapshot.connected {
            return 0;
        }
        self.snapshot
            .tasks
            .iter()
            .filter(|t| match filter {
                SidebarFilter::All => true,
                SidebarFilter::Active => t.status == DownloadStatus::Active,
                SidebarFilter::Paused => t.status == DownloadStatus::Paused,
                SidebarFilter::Completed => t.status == DownloadStatus::Completed,
                SidebarFilter::Category(cat) => t.category == cat,
            })
            .count()
    }

    /// 渲染经典 IDM 大图标工具栏（全部绑定真实 aria2 控制指令）
    fn render_toolbar(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let divider = || div().w(px(1.0)).h(px(36.0)).mx_1().bg(p.border);

        div()
            .flex()
            .items_center()
            .px_2()
            .py_1()
            .bg(p.toolbar_bg)
            .border_b_1()
            .border_color(p.border)
            .child(
                idm_tool_button("tb-add", "＋", rgb(0x16a34a), "新建任务", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.show_add_bar = !this.show_add_bar;
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-resume", "▶", rgb(0x16a34a), "继续", p).on_click(
                    cx.listener(|this, _, _, _| {
                        if let Some(gid) = this.state.selected_task_id.clone() {
                            this.engine.send(EngineCommand::Unpause(gid));
                        }
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-stop", "■", rgb(0xdc2626), "停止", p).on_click(
                    cx.listener(|this, _, _, _| {
                        if let Some(gid) = this.state.selected_task_id.clone() {
                            this.engine.send(EngineCommand::Pause(gid));
                        }
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-stop-all", "⏹", rgb(0xb91c1c), "全部停止", p).on_click(
                    cx.listener(|this, _, _, _| {
                        this.engine.send(EngineCommand::PauseAll);
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-del", "✕", rgb(0xef4444), "删除", p).on_click(cx.listener(
                    |this, _, _, _| {
                        if let Some(gid) = this.state.selected_task_id.take() {
                            this.engine.send(EngineCommand::Remove(gid));
                        }
                    },
                )),
            )
            .child(divider())
            .child(
                idm_tool_button("tb-folder", "📁", rgb(0xd97706), "打开目录", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        let target_dir = this
                            .state
                            .selected_task_id
                            .as_deref()
                            .and_then(|id| this.snapshot.tasks.iter().find(|t| t.id == id))
                            .map(|t| t.save_dir.clone())
                            .unwrap_or_else(|| this.state.settings.default_download_dir.clone());
                        let _ = open_directory(&target_dir);
                        this.notice = Some(format!("已打开目录: {}", target_dir.display()));
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-opt", "⚙", rgb(0x2563eb), "选项", p).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_settings = !this.show_settings;
                        cx.notify();
                    },
                )),
            )
            .child(
                idm_tool_button("tb-sync", "↻", rgb(0x0284c7), "刷新", p).on_click(cx.listener(
                    |this, _, _, _| {
                        this.engine.send(EngineCommand::SyncNow);
                    },
                )),
            )
    }

    /// 渲染新建下载 URL 输入条
    fn render_add_bar(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(p.menu_bg)
            .border_b_1()
            .border_color(p.border)
            .child(div().text_xs().text_color(p.text).child("下载链接:"))
            .child(div().flex_1().child(Input::new(&self.url_input)))
            .child(
                Button::new("btn-submit-url")
                    .primary()
                    .label("开始下载")
                    .on_click(cx.listener(|this, _, window, cx| {
                        let url = this.url_input.read(cx).value().trim().to_string();
                        if !url.is_empty() {
                            let cat = FileCategory::from_filename(&url);
                            let save_dir = this.state.settings.resolve_save_dir(cat);
                            this.engine.send(EngineCommand::AddUri {
                                url,
                                filename: None,
                                save_dir: Some(save_dir),
                                split: this.state.settings.default_split,
                            });
                            this.url_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                            this.show_add_bar = false;
                            cx.notify();
                        }
                    })),
            )
    }

    /// 渲染左侧经典 IDM 树形分类栏
    fn render_category_tree(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_filter;
        let nodes: Vec<(usize, SidebarFilter, &str, bool)> = vec![
            (0, SidebarFilter::All, "▾ 全部下载", false),
            (1, SidebarFilter::Category(FileCategory::Compressed), "压缩文件", true),
            (2, SidebarFilter::Category(FileCategory::Document), "文档", true),
            (3, SidebarFilter::Category(FileCategory::Music), "音乐", true),
            (4, SidebarFilter::Category(FileCategory::Program), "程序", true),
            (5, SidebarFilter::Category(FileCategory::Video), "视频", true),
            (6, SidebarFilter::Category(FileCategory::Other), "其他", true),
            (7, SidebarFilter::Active, "▸ 正在下载", false),
            (8, SidebarFilter::Paused, "▸ 已暂停", false),
            (9, SidebarFilter::Completed, "▸ 已完成", false),
        ];

        div()
            .flex()
            .flex_col()
            .w(px(185.0))
            .bg(p.panel_bg)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .bg(p.header_bg)
                    .border_b_1()
                    .border_color(p.border)
                    .text_xs()
                    .font_medium()
                    .text_color(p.text)
                    .child("分类"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p_1()
                    .children(nodes.into_iter().map(|(id, filter, label, indent)| {
                        let is_active = filter == active;
                        let count = self.count_for(filter);
                        div()
                            .id(("tree", id))
                            .flex()
                            .items_center()
                            .justify_between()
                            .pl(if indent { px(22.0) } else { px(6.0) })
                            .pr_2()
                            .py_1()
                            .cursor_pointer()
                            .bg(if is_active { p.selected_bg } else { p.panel_bg })
                            .text_xs()
                            .text_color(p.text)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.active_filter = filter;
                                cx.notify();
                            }))
                            .child(label)
                            .child(div().text_color(p.muted).child(format!("({count})")))
                    })),
            )
    }

    /// 渲染右侧真实任务列表表格
    fn render_task_list(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = self.visible_tasks();
        let selected_id = self.state.selected_task_id.clone();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .bg(p.panel_bg)
            .child(render_idm_table_header(p))
            .child(if !self.snapshot.connected {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(p.muted)
                    .child("未连接到 aria2c，请先启动 aria2c --enable-rpc（默认端口 6800）")
                    .into_any_element()
            } else if tasks.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(p.muted)
                    .child("当前分类暂无下载任务，点击左上角「新建任务」添加真实下载链接")
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .children(tasks.into_iter().enumerate().map(|(idx, task)| {
                        let task_id = task.id.clone();
                        let is_selected = selected_id.as_deref() == Some(task.id.as_str());
                        render_idm_task_row(idx, &task, is_selected, p).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.state.selected_task_id = Some(task_id.clone());
                                cx.notify();
                            },
                        ))
                    }))
                    .into_any_element()
            })
    }

    /// 渲染底部经典状态栏（显示真实 aria2 在线状态与全局速度）
    fn render_statusbar(&self, p: IdmPalette) -> impl IntoElement {
        let status_text = if let Some(ver) = &self.snapshot.aria2_version {
            format!("● 已连接 aria2 v{ver}")
        } else {
            "○ 未连接 aria2c".to_string()
        };
        let speed = format_speed(self.snapshot.global_download_speed);
        let msg = self
            .snapshot
            .last_error
            .clone()
            .unwrap_or_else(|| "就绪".to_string());

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .bg(p.menu_bg)
            .border_t_1()
            .border_color(p.border)
            .text_xs()
            .text_color(p.muted)
            .child(msg)
            .child(format!("速度: {speed}  |  {status_text}"))
    }
}

impl Render for IdmMainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = IdmPalette::classic();
        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel_bg)
            .child(render_menubar(p))
            .child(self.render_toolbar(p, cx));

        if self.show_add_bar {
            root = root.child(self.render_add_bar(p, cx));
        }
        if self.show_settings {
            root = root.child(render_settings_bar(
                &self.settings_inputs,
                p,
                cx.listener(|this, _, _, cx| {
                    this.settings_inputs.apply_to(&mut this.state.settings, cx);
                    let path = AppSettings::default_config_path();
                    let _ = this.state.settings.save_to_file(&path);
                    this.engine.send(EngineCommand::ApplyGlobalOptions {
                        max_overall_download_limit: this
                            .state
                            .settings
                            .max_overall_download_limit
                            .clone(),
                        proxy: this.state.settings.proxy.clone(),
                    });
                    this.notice = Some("设置已保存并实时应用到 aria2".into());
                    this.show_settings = false;
                    cx.notify();
                }),
            ));
        }

        root.child(
            div()
                .flex()
                .flex_1()
                .overflow_hidden()
                .child(self.render_category_tree(p, cx))
                .child(self.render_task_list(p, cx)),
        )
        .child(self.render_statusbar(p))
    }
}
