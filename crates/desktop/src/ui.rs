//! 仿经典 IDM (Internet Download Manager) 桌面主界面（已接入真实 aria2 后台引擎）

use crate::batch_dialog::{BatchTaskInputs, preview_batch_urls, render_batch_dialog};
use crate::dialogs::{AddTaskInputs, AddTaskTab};
use crate::engine::{DownloadEngine, EngineCommand, EngineSnapshot};
use crate::settings_dialog::{render_add_task_dialog, render_settings_dialog};
use crate::settings_panel::{
    SettingsInputs, SettingsTab, bring_window_to_front, open_directory, pick_directory,
};
use crate::state::{DesktopState, SidebarFilter, format_speed};
use crate::system::{apply_autostart, send_system_notification};
use crate::ui_parts::{
    IdmPalette, idm_tool_button, render_idm_table_header, render_idm_task_row,
};
use gpui_kit::base::StyledExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use idm_core::{AppSettings, DownloadStatus, DownloadTask, FileCategory};
use std::collections::HashMap;
use std::path::PathBuf;

/// IDM 主窗口视图实体
pub struct IdmMainView {
    state: DesktopState,
    engine: DownloadEngine,
    snapshot: EngineSnapshot,
    show_add_modal: bool,
    show_batch_modal: bool,
    show_settings_modal: bool,
    add_tab: AddTaskTab,
    settings_tab: SettingsTab,
    add_inputs: AddTaskInputs,
    batch_inputs: BatchTaskInputs,
    settings_inputs: SettingsInputs,
    search_input: Entity<InputState>,
    last_task_statuses: HashMap<String, DownloadStatus>,
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

        let engine = DownloadEngine::start(
            state.settings.aria2.clone(),
            state.settings.to_aria2_startup_args(),
            state.settings.bridge_port,
            state.settings.bridge_token.clone(),
        );
        // 启动后立即将本地已保存的全局配置同步给内置 aria2c
        Self::sync_global_options(&engine, &state.settings);

        let preferred_dir = state
            .settings
            .last_save_dir
            .as_ref()
            .unwrap_or(&state.settings.default_download_dir)
            .to_string_lossy()
            .into_owned();
        let add_inputs =
            AddTaskInputs::new(&preferred_dir, state.settings.allow_overwrite, window, cx);
        let batch_inputs = BatchTaskInputs::new(&preferred_dir, window, cx);
        let settings_inputs = SettingsInputs::from_settings(&state.settings, window, cx);
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索文件名或路径..."));

        // 启动轻量 UI 定时刷新循环：每 500ms 同步一次后台 aria2 引擎快照，并检查插件待确认下载请求
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                let ok = this.update_in(cx, |view, window, cx| {
                    let next = view.engine.snapshot();
                    if view.state.settings.enable_notifications {
                        for t in &next.tasks {
                            if let Some(prev) = view.last_task_statuses.get(&t.id) {
                                if *prev != DownloadStatus::Completed
                                    && t.status == DownloadStatus::Completed
                                {
                                    send_system_notification(
                                        "IDM-Aria2-RS 下载完成",
                                        format!("已完成下载: {}", t.filename),
                                    );
                                } else if *prev != DownloadStatus::Error
                                    && t.status == DownloadStatus::Error
                                {
                                    send_system_notification(
                                        "IDM-Aria2-RS 下载失败",
                                        format!("下载出错: {}", t.filename),
                                    );
                                }
                            }
                        }
                    }
                    view.last_task_statuses =
                        next.tasks.iter().map(|t| (t.id.clone(), t.status)).collect();
                    view.snapshot = next;
                    if let Some(req) = view.engine.pop_pending_request() {
                        view.open_add_modal_with_request(req, window, cx);
                        window.activate_window();
                        cx.activate(true);
                        bring_window_to_front("IDM-Aria2-RS 下载器");
                    }
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
            show_add_modal: false,
            show_batch_modal: false,
            show_settings_modal: false,
            add_tab: AddTaskTab::Basic,
            settings_tab: SettingsTab::General,
            add_inputs,
            batch_inputs,
            settings_inputs,
            search_input,
            last_task_statuses: HashMap::new(),
            notice: None,
        }
    }

    fn sync_global_options(engine: &DownloadEngine, s: &AppSettings) {
        engine.send(EngineCommand::ApplyGlobalOptions(s.to_aria2_global_options()));
    }

    /// 收到浏览器插件拦截请求后，预填全部字段并弹出「新建下载任务」确认窗口
    fn open_add_modal_with_request(
        &mut self,
        req: idm_core::NewDownloadRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let preferred_dir = req
            .save_dir
            .as_ref()
            .or(self.state.settings.last_save_dir.as_ref())
            .unwrap_or(&self.state.settings.default_download_dir)
            .to_string_lossy()
            .into_owned();
        let filename = req.inferred_filename();
        let referer = extract_header(&req.headers, "referer").unwrap_or_default();
        let ua = extract_header(&req.headers, "user-agent").unwrap_or_default();
        let cookie = extract_header(&req.headers, "cookie").unwrap_or_default();

        self.add_inputs
            .url
            .update(cx, |i, cx| i.set_value(req.url, window, cx));
        self.add_inputs
            .filename
            .update(cx, |i, cx| i.set_value(filename, window, cx));
        self.add_inputs
            .save_dir
            .update(cx, |i, cx| i.set_value(preferred_dir, window, cx));
        self.add_inputs
            .referer
            .update(cx, |i, cx| i.set_value(referer, window, cx));
        self.add_inputs
            .user_agent
            .update(cx, |i, cx| i.set_value(ua, window, cx));
        self.add_inputs
            .cookie
            .update(cx, |i, cx| i.set_value(cookie, window, cx));

        self.add_inputs.allow_overwrite = self.state.settings.allow_overwrite;
        self.add_tab = AddTaskTab::Basic;
        self.show_batch_modal = false;
        self.show_settings_modal = false;
        self.show_add_modal = true;
        self.notice = Some("已捕获浏览器下载请求，请确认目录与设置后点击「开始下载」".into());
    }

    /// 返回当前视图应展示的任务列表（已连接 aria2 时展示真实任务，支持左侧分类 + 顶部搜索过滤）
    fn visible_tasks(&self, cx: &App) -> Vec<DownloadTask> {
        let source: Vec<DownloadTask> = if self.snapshot.connected {
            self.snapshot.tasks.clone()
        } else {
            Vec::new()
        };
        let kw = self.search_input.read(cx).value().trim().to_ascii_lowercase();

        source
            .into_iter()
            .filter(|t| match self.state.active_filter {
                SidebarFilter::All => true,
                SidebarFilter::Active => t.status == DownloadStatus::Active,
                SidebarFilter::Paused => t.status == DownloadStatus::Paused,
                SidebarFilter::Completed => t.status == DownloadStatus::Completed,
                SidebarFilter::Category(cat) => t.category == cat,
            })
            .filter(|t| {
                kw.is_empty()
                    || t.filename.to_ascii_lowercase().contains(&kw)
                    || t.url.to_ascii_lowercase().contains(&kw)
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
                    cx.listener(|this, _, window, cx| {
                        // 打开新建任务弹窗时优先回填上次选择的保存目录
                        let preferred = this
                            .state
                            .settings
                            .last_save_dir
                            .as_ref()
                            .unwrap_or(&this.state.settings.default_download_dir)
                            .to_string_lossy()
                            .into_owned();
                        this.add_inputs
                            .save_dir
                            .update(cx, |i, cx| i.set_value(preferred, window, cx));

                        // 自动检测系统剪贴板中的下载链接并预填
                        if let Some(clip) = cx.read_from_clipboard().and_then(|c| c.text()) {
                            let trimmed = clip.trim();
                            if trimmed.starts_with("http://")
                                || trimmed.starts_with("https://")
                                || trimmed.starts_with("magnet:?")
                            {
                                let inferred = idm_core::NewDownloadRequest {
                                    url: trimmed.to_string(),
                                    filename: None,
                                    save_dir: None,
                                    headers: vec![],
                                    auto_start: true,
                                }
                                .inferred_filename();
                                this.add_inputs
                                    .url
                                    .update(cx, |i, cx| i.set_value(trimmed.to_string(), window, cx));
                                this.add_inputs
                                    .filename
                                    .update(cx, |i, cx| i.set_value(inferred, window, cx));
                            }
                        }

                        this.add_inputs.allow_overwrite = this.state.settings.allow_overwrite;
                        this.add_tab = AddTaskTab::Basic;
                        this.show_settings_modal = false;
                        this.show_add_modal = true;
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-batch", "▤", rgb(0x0d9488), "批量任务", p).on_click(
                    cx.listener(|this, _, window, cx| {
                        let preferred = this
                            .state
                            .settings
                            .last_save_dir
                            .as_ref()
                            .unwrap_or(&this.state.settings.default_download_dir)
                            .to_string_lossy()
                            .into_owned();
                        this.batch_inputs
                            .save_dir
                            .update(cx, |i, cx| i.set_value(preferred, window, cx));
                        this.show_add_modal = false;
                        this.show_settings_modal = false;
                        this.show_batch_modal = true;
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
                idm_tool_button("tb-resume-all", "⏩", rgb(0x15803d), "全部继续", p).on_click(
                    cx.listener(|this, _, _, _| {
                        this.engine.send(EngineCommand::UnpauseAll);
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
                        this.show_add_modal = false;
                        this.show_settings_modal = true;
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
            .child(div().flex_1())
            .child(div().w(px(220.0)).pr_2().child(Input::new(&self.search_input)))
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
        let tasks = self.visible_tasks(cx);
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

    /// 渲染底部经典状态栏（显示真实 aria2 在线状态、选中任务错误原因与全局速度）
    fn render_statusbar(&self, p: IdmPalette) -> impl IntoElement {
        let status_text = if let Some(ver) = &self.snapshot.aria2_version {
            format!("● 已连接 aria2 v{ver}")
        } else {
            "○ 未连接 aria2c".to_string()
        };
        let speed = format_speed(self.snapshot.global_download_speed);
        let selected_err = self
            .state
            .selected_task_id
            .as_deref()
            .and_then(|id| self.snapshot.tasks.iter().find(|t| t.id == id))
            .and_then(|t| t.error_message.clone());
        let msg = selected_err
            .or_else(|| self.snapshot.last_error.clone())
            .or_else(|| self.notice.clone())
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
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel_bg)
            .child(self.render_toolbar(p, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_category_tree(p, cx))
                    .child(self.render_task_list(p, cx)),
            )
            .child(self.render_statusbar(p));

        if self.show_add_modal {
            let entity = cx.entity().clone();
            root = root.child(render_add_task_dialog(
                &self.add_inputs,
                self.add_tab,
                p,
                move |tab, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.add_tab = tab;
                        cx.notify();
                    });
                },
                cx.listener(|this, _, window, cx| {
                    let cur = this.add_inputs.save_dir.read(cx).value().trim().to_string();
                    let init = (!cur.is_empty()).then(|| PathBuf::from(&cur));
                    cx.spawn_in(window, async move |this, cx| {
                        let chosen = cx
                            .background_executor()
                            .spawn(async move { pick_directory(init.as_deref()) })
                            .await;
                        if let Some(dir) = chosen {
                            let s = dir.to_string_lossy().into_owned();
                            let _ = this.update_in(cx, |view, window, cx| {
                                view.add_inputs
                                    .save_dir
                                    .update(cx, |i, cx| i.set_value(s, window, cx));
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }),
                cx.listener(|this, _, _, cx| {
                    this.add_inputs.allow_overwrite = !this.add_inputs.allow_overwrite;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_add_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_add_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, window, cx| {
                    let url = this.add_inputs.url.read(cx).value().trim().to_string();
                    if !url.is_empty() {
                        let custom_name =
                            this.add_inputs.filename.read(cx).value().trim().to_string();
                        let custom_dir =
                            this.add_inputs.save_dir.read(cx).value().trim().to_string();
                        let speed_limit =
                            this.add_inputs.speed_limit.read(cx).value().trim().to_string();
                        let task_proxy =
                            this.add_inputs.proxy.read(cx).value().trim().to_string();
                        let headers = this.add_inputs.build_headers(cx);

                        let filename = (!custom_name.is_empty()).then_some(custom_name);
                        let save_dir = if custom_dir.is_empty() {
                            let cat = FileCategory::from_filename(
                                filename.as_deref().unwrap_or(&url),
                            );
                            this.state.settings.resolve_save_dir(cat)
                        } else {
                            PathBuf::from(custom_dir)
                        };

                        // 记住本次保存目录并持久化
                        this.state.settings.last_save_dir = Some(save_dir.clone());
                        let _ = this
                            .state
                            .settings
                            .save_to_file(&AppSettings::default_config_path());

                        this.engine.send(EngineCommand::AddUri {
                            url,
                            filename,
                            save_dir: Some(save_dir),
                            split: this.state.settings.default_split,
                            max_connection_per_server: this
                                .state
                                .settings
                                .max_connection_per_server,
                            min_split_size: this.state.settings.min_split_size.clone(),
                            allow_overwrite: this.add_inputs.allow_overwrite,
                            max_download_limit: (!speed_limit.is_empty()).then_some(speed_limit),
                            headers,
                            proxy: (!task_proxy.is_empty()).then_some(task_proxy),
                        });
                        this.add_inputs.url.update(cx, |i, cx| i.set_value("", window, cx));
                        this.add_inputs
                            .filename
                            .update(cx, |i, cx| i.set_value("", window, cx));
                        this.notice = Some("已提交新下载任务到 aria2".into());
                        this.show_add_modal = false;
                        cx.notify();
                    }
                }),
            ));
        }

        if self.show_batch_modal {
            let preview_count = preview_batch_urls(&self.batch_inputs, cx).len();
            root = root.child(render_batch_dialog(
                &self.batch_inputs,
                preview_count,
                p,
                cx.listener(|this, _, window, cx| {
                    let cur = this
                        .batch_inputs
                        .save_dir
                        .read(cx)
                        .value()
                        .trim()
                        .to_string();
                    let init = (!cur.is_empty()).then(|| PathBuf::from(&cur));
                    cx.spawn_in(window, async move |this, cx| {
                        let chosen = cx
                            .background_executor()
                            .spawn(async move { pick_directory(init.as_deref()) })
                            .await;
                        if let Some(dir) = chosen {
                            let s = dir.to_string_lossy().into_owned();
                            let _ = this.update_in(cx, |view, window, cx| {
                                view.batch_inputs
                                    .save_dir
                                    .update(cx, |i, cx| i.set_value(s, window, cx));
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_batch_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_batch_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, window, cx| {
                    let urls = preview_batch_urls(&this.batch_inputs, cx);
                    if !urls.is_empty() {
                        let custom_dir = this
                            .batch_inputs
                            .save_dir
                            .read(cx)
                            .value()
                            .trim()
                            .to_string();
                        let save_dir = if custom_dir.is_empty() {
                            this.state.settings.default_download_dir.clone()
                        } else {
                            PathBuf::from(custom_dir)
                        };
                        this.state.settings.last_save_dir = Some(save_dir.clone());
                        let _ = this
                            .state
                            .settings
                            .save_to_file(&AppSettings::default_config_path());

                        let count = urls.len();
                        for url in urls {
                            this.engine.send(EngineCommand::AddUri {
                                url,
                                filename: None,
                                save_dir: Some(save_dir.clone()),
                                split: this.state.settings.default_split,
                                max_connection_per_server: this
                                    .state
                                    .settings
                                    .max_connection_per_server,
                                min_split_size: this.state.settings.min_split_size.clone(),
                                allow_overwrite: this.state.settings.allow_overwrite,
                                max_download_limit: None,
                                headers: vec![],
                                proxy: None,
                            });
                        }
                        this.batch_inputs
                            .urls
                            .update(cx, |i, cx| i.set_value("", window, cx));
                        this.notice = Some(format!("已批量提交 {count} 条下载任务到 aria2"));
                        this.show_batch_modal = false;
                        cx.notify();
                    }
                }),
            ));
        }

        if self.show_settings_modal {
            let entity_tab = cx.entity().clone();
            let entity_flag = cx.entity().clone();
            root = root.child(render_settings_dialog(
                &self.settings_inputs,
                self.settings_tab,
                p,
                move |tab, _, cx| {
                    entity_tab.update(cx, |this, cx| {
                        this.settings_tab = tab;
                        cx.notify();
                    });
                },
                cx.listener(|this, _, window, cx| {
                    let cur = this
                        .settings_inputs
                        .download_dir
                        .read(cx)
                        .value()
                        .trim()
                        .to_string();
                    let init = (!cur.is_empty()).then(|| PathBuf::from(&cur));
                    cx.spawn_in(window, async move |this, cx| {
                        let chosen = cx
                            .background_executor()
                            .spawn(async move { pick_directory(init.as_deref()) })
                            .await;
                        if let Some(dir) = chosen {
                            let s = dir.to_string_lossy().into_owned();
                            let _ = this.update_in(cx, |view, window, cx| {
                                view.settings_inputs
                                    .download_dir
                                    .update(cx, |i, cx| i.set_value(s, window, cx));
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }),
                move |flag, _, cx| {
                    entity_flag.update(cx, |this, cx| {
                        this.settings_inputs.toggle_flag(flag);
                        cx.notify();
                    });
                },
                cx.listener(|this, _, _, cx| {
                    this.show_settings_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_settings_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.settings_inputs.apply_to(&mut this.state.settings, cx);
                    let path = AppSettings::default_config_path();
                    let _ = this.state.settings.save_to_file(&path);
                    let _ = apply_autostart(this.state.settings.auto_start);
                    Self::sync_global_options(&this.engine, &this.state.settings);
                    this.notice = Some("设置已保存并实时应用到 aria2".into());
                    this.show_settings_modal = false;
                    cx.notify();
                }),
            ));
        }

        root
    }
}

/// 从 `Key: Value` 请求头列表中按名称（不区分大小写）提取值
fn extract_header(headers: &[String], name: &str) -> Option<String> {
    headers.iter().find_map(|h| {
        let (k, v) = h.split_once(':')?;
        if k.trim().eq_ignore_ascii_case(name) {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}
