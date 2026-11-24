//! 仿经典 IDM (Internet Download Manager) 桌面主界面（已接入真实 aria2 后台引擎）

use crate::about_dialog::{render_about_dialog, render_update_dialog};
use crate::batch_dialog::{BatchTaskInputs, preview_batch_urls, render_batch_dialog};
use crate::dialogs::{AddTaskInputs, AddTaskTab};
use crate::engine::{DownloadEngine, EngineCommand, EngineSnapshot};
use crate::settings_dialog::{render_add_task_dialog, render_settings_dialog};
use crate::settings_panel::{
    SettingsInputs, SettingsTab, bring_window_to_front, hide_window, open_directory, open_file,
    open_url_in_browser, pick_directory, reveal_file_in_directory,
};
use crate::state::{DesktopState, SidebarFilter, format_bytes, format_speed};
use crate::system::{apply_autostart, send_system_notification};
use crate::tray::start_system_tray;
use crate::ui_parts::{
    IdmPalette, idm_tool_button, render_idm_table_header, render_idm_task_row,
    render_task_inspector,
};
use gpui_kit::base::StyledExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use idm_core::{
    AppSettings, CURRENT_VERSION, DownloadStatus, DownloadTask, FileCategory, GITHUB_REPO_URL,
    UpdateInfo, check_github_update, probe_download_url,
};
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
    show_about_modal: bool,
    update_prompt: Option<UpdateInfo>,
    checking_update: bool,
    about_update_status: Option<String>,
    pending_update_installer: Option<PathBuf>,
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
        let mut state = DesktopState::new();
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
        // 启动后立即将本地已保存的全局配置同步给内置 aria2c，并启动系统托盘
        Self::sync_global_options(&engine, &state.settings);
        start_system_tray(engine.command_sender());

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
                                        "Gravita 下载完成",
                                        format!("已完成下载: {}", t.filename),
                                    );
                                } else if *prev != DownloadStatus::Error
                                    && t.status == DownloadStatus::Error
                                {
                                    send_system_notification(
                                        "Gravita 下载失败",
                                        format!("下载出错: {}", t.filename),
                                    );
                                }
                            }
                        }
                    }
                    view.last_task_statuses =
                        next.tasks.iter().map(|t| (t.id.clone(), t.status)).collect();
                    // 若正在等待自动更新安装包下载完成，一旦完成则自动拉起安装向导
                    if let Some(installer_path) = view.pending_update_installer.clone() {
                        if next.tasks.iter().any(|t| {
                            t.status == DownloadStatus::Completed && t.full_path() == installer_path
                        }) {
                            view.pending_update_installer = None;
                            #[cfg(windows)]
                            if installer_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
                            {
                                // 将当前安装目录通过 NSIS /D= 参数直接传给更新安装包，确保自动定位到原目录
                                let mut cmd = std::process::Command::new(&installer_path);
                                if let Ok(exe_path) = std::env::current_exe() {
                                    if let Some(install_dir) = exe_path.parent() {
                                        cmd.arg(format!("/D={}", install_dir.display()));
                                    }
                                }
                                let _ = cmd.spawn();
                                view.engine.send(EngineCommand::ShutdownAndExit);
                            } else {
                                let _ = open_file(&installer_path);
                            }
                            #[cfg(not(windows))]
                            {
                                let _ = open_file(&installer_path);
                            }
                            view.notice = Some(format!(
                                "新版本安装包已下载完成并启动: {}",
                                installer_path.display()
                            ));
                        }
                    }
                    view.snapshot = next;
                    if let Some(req) = view.engine.pop_pending_request() {
                        view.open_add_modal_with_request(req, window, cx);
                        window.activate_window();
                        cx.activate(true);
                        bring_window_to_front("Gravita 下载器");
                    }
                    cx.notify();
                });
                if ok.is_err() {
                    break;
                }
            }
        })
        .detach();

        let mut view = Self {
            state,
            engine,
            snapshot: EngineSnapshot::default(),
            show_add_modal: false,
            show_batch_modal: false,
            show_settings_modal: false,
            show_about_modal: false,
            update_prompt: None,
            checking_update: false,
            about_update_status: None,
            pending_update_installer: None,
            add_tab: AddTaskTab::Basic,
            settings_tab: SettingsTab::General,
            add_inputs,
            batch_inputs,
            settings_inputs,
            search_input,
            last_task_statuses: HashMap::new(),
            notice: None,
        };

        if view.state.settings.auto_check_update {
            view.trigger_update_check(false, window, cx);
        }

        view
    }

    fn sync_global_options(engine: &DownloadEngine, s: &AppSettings) {
        engine.send(EngineCommand::ApplyGlobalOptions(s.to_aria2_global_options()));
    }

    /// 异步检查 GitHub Releases 最新版本（`manual = false` 时若命中用户已忽略版本则静默跳过）
    fn trigger_update_check(
        &mut self,
        manual: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.checking_update {
            return;
        }
        self.checking_update = true;
        if manual {
            self.about_update_status = Some("正在连接 GitHub Releases 检查更新...".into());
            self.notice = Some("正在检查 GitHub 最新版本...".into());
        }
        cx.notify();

        let proxy = self.state.settings.proxy.clone();
        cx.spawn_in(window, async move |this, cx| {
            if !manual {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(2))
                    .await;
            }
            let res = cx
                .background_executor()
                .spawn(async move {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| e.to_string())?;
                    rt.block_on(check_github_update(proxy.as_deref()))
                })
                .await;

            let _ = this.update_in(cx, |view, _, cx| {
                view.checking_update = false;
                match res {
                    Ok(Some(info)) => {
                        let is_ignored = !manual
                            && view
                                .state
                                .settings
                                .ignored_update_version
                                .as_deref()
                                == Some(info.latest_version.as_str());
                        if !is_ignored {
                            view.about_update_status =
                                Some(format!("发现新版本: {}", info.latest_version));
                            view.notice = Some(format!(
                                "发现新版本 Gravita {}，请确认是否更新",
                                info.latest_version
                            ));
                            view.show_about_modal = false;
                            view.update_prompt = Some(info);
                        }
                    }
                    Ok(None) => {
                        if manual {
                            let msg = format!("✓ 当前已是最新版本 (v{CURRENT_VERSION})");
                            view.about_update_status = Some(msg.clone());
                            view.notice = Some(msg);
                        }
                    }
                    Err(err) => {
                        if manual {
                            view.about_update_status = Some(err.clone());
                            view.notice = Some(err);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 收到浏览器插件拦截请求后，预填全部字段并弹出「新建下载任务」确认窗口
    fn open_add_modal_with_request(
        &mut self,
        req: idm_core::NewDownloadRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let filename = req.inferred_filename();
        let cat = FileCategory::from_filename(&filename);
        let auto_cat_dir = self.state.settings.resolve_save_dir(cat);
        let preferred_dir = req
            .save_dir
            .as_ref()
            .or(self.state.settings.last_save_dir.as_ref())
            .unwrap_or(&auto_cat_dir)
            .to_string_lossy()
            .into_owned();
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
        self.add_inputs.use_proxy = false;
        self.add_inputs.replace_gid = None;
        self.add_inputs.probe_status = Some("正在自动探测服务器文件大小与断点续传支持...".into());
        self.add_tab = AddTaskTab::Basic;
        self.show_batch_modal = false;
        self.show_settings_modal = false;
        self.show_about_modal = false;
        self.show_add_modal = true;
        self.notice = Some("已捕获浏览器下载请求，请确认目录与设置后点击「开始下载」".into());
        self.trigger_url_probe(window, cx);
    }

    /// 异步发 Range 请求探测当前输入 URL 的真实文件名、文件大小与断点续传能力（带序号防竞态覆盖）
    fn trigger_url_probe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.add_inputs.url.read(cx).value().trim().to_string();
        if url.is_empty() {
            self.add_inputs.probe_status = Some("请先输入有效的下载链接".into());
            cx.notify();
            return;
        }
        self.add_inputs.probe_seq += 1;
        let current_seq = self.add_inputs.probe_seq;
        let is_replacing = self.add_inputs.replace_gid.is_some();
        let headers = self.add_inputs.build_headers(cx);
        let task_proxy = self.add_inputs.proxy.read(cx).value().trim().to_string();
        let proxy = self
            .state
            .settings
            .resolve_task_proxy(self.add_inputs.use_proxy, Some(&task_proxy));
        self.add_inputs.probe_status = Some("正在连接服务器探测文件信息...".into());
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    static PROBE_RT: std::sync::OnceLock<tokio::runtime::Runtime> =
                        std::sync::OnceLock::new();
                    let rt = PROBE_RT.get_or_init(|| {
                        tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(1)
                            .enable_all()
                            .build()
                            .expect("初始化探测运行时失败")
                    });
                    rt.block_on(probe_download_url(&url, &headers, proxy.as_deref()))
                })
                .await;

            let _ = this.update_in(cx, |view, window, cx| {
                // 若已有更新的探测请求发出，丢弃过期响应
                if view.add_inputs.probe_seq != current_seq {
                    return;
                }
                match res {
                    Ok(info) => {
                        let size_str = info
                            .total_bytes
                            .map(format_bytes)
                            .unwrap_or_else(|| "未知大小".into());
                        let resume_str = if info.resumable {
                            "支持多线程分片/续传"
                        } else {
                            "单线程流式下载"
                        };
                        view.add_inputs.probe_status = Some(format!(
                            "大小: {size_str}  |  分类: {}  |  {resume_str}",
                            info.category.display_name()
                        ));
                        if !is_replacing {
                            let cur_name =
                                view.add_inputs.filename.read(cx).value().trim().to_string();
                            if cur_name.is_empty() || cur_name == "download.bin" {
                                view.add_inputs.filename.update(cx, |i, cx| {
                                    i.set_value(info.filename.clone(), window, cx)
                                });
                            }
                            // 仅当用户从未设置过上次保存目录且当前目录为空时，才自动填充分类子目录
                            if view.state.settings.last_save_dir.is_none()
                                && view.add_inputs.save_dir.read(cx).value().trim().is_empty()
                            {
                                let auto_dir = view
                                    .state
                                    .settings
                                    .resolve_save_dir(info.category)
                                    .to_string_lossy()
                                    .into_owned();
                                view.add_inputs
                                    .save_dir
                                    .update(cx, |i, cx| i.set_value(auto_dir, window, cx));
                            }
                        }
                    }
                    Err(err) => {
                        view.add_inputs.probe_status = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 返回当前视图应展示的任务列表（按创建时间倒序排列：最新创建的任务排在最前面）
    fn visible_tasks(&self, cx: &App) -> Vec<DownloadTask> {
        let source: Vec<DownloadTask> = self.snapshot.tasks.clone();
        let kw = self.search_input.read(cx).value().trim().to_ascii_lowercase();

        let mut list: Vec<(usize, DownloadTask)> = source
            .into_iter()
            .enumerate()
            .filter(|(_, t)| match self.state.active_filter {
                SidebarFilter::All => true,
                SidebarFilter::Active => t.status == DownloadStatus::Active,
                SidebarFilter::Waiting => t.status == DownloadStatus::Waiting,
                SidebarFilter::Paused => t.status == DownloadStatus::Paused,
                SidebarFilter::Completed => t.status == DownloadStatus::Completed,
                SidebarFilter::Error => t.status == DownloadStatus::Error,
                SidebarFilter::Category(cat) => t.category == cat,
            })
            .filter(|(_, t)| {
                kw.is_empty()
                    || t.filename.to_ascii_lowercase().contains(&kw)
                    || t.url.to_ascii_lowercase().contains(&kw)
            })
            .collect();

        // 按 created_at_ms 降序排列；对于旧记录时间戳同为 0 的，按原始插入索引逆序（越后加入的越靠前）
        list.sort_by(|(idx_a, a), (idx_b, b)| {
            b.created_at_ms
                .cmp(&a.created_at_ms)
                .then_with(|| idx_b.cmp(idx_a))
        });

        list.into_iter().map(|(_, t)| t).collect()
    }

    /// 统计某个分类下的真实任务数
    fn count_for(&self, filter: SidebarFilter) -> usize {
        self.snapshot
            .tasks
            .iter()
            .filter(|t| match filter {
                SidebarFilter::All => true,
                SidebarFilter::Active => t.status == DownloadStatus::Active,
                SidebarFilter::Waiting => t.status == DownloadStatus::Waiting,
                SidebarFilter::Paused => t.status == DownloadStatus::Paused,
                SidebarFilter::Completed => t.status == DownloadStatus::Completed,
                SidebarFilter::Error => t.status == DownloadStatus::Error,
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
                        // 优先使用上次选择的下载保存目录，若从未设置过则回退到默认目录
                        let mut default_dir = this
                            .state
                            .settings
                            .last_save_dir
                            .as_ref()
                            .unwrap_or(&this.state.settings.default_download_dir)
                            .to_string_lossy()
                            .into_owned();

                        // 自动检测系统剪贴板中的下载链接并预填，触发远程大小探测
                        let mut auto_probed = false;
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
                                if this.state.settings.last_save_dir.is_none() {
                                    let cat = FileCategory::from_filename(&inferred);
                                    default_dir = this
                                        .state
                                        .settings
                                        .resolve_save_dir(cat)
                                        .to_string_lossy()
                                        .into_owned();
                                }
                                this.add_inputs
                                    .url
                                    .update(cx, |i, cx| i.set_value(trimmed.to_string(), window, cx));
                                this.add_inputs
                                    .filename
                                    .update(cx, |i, cx| i.set_value(inferred, window, cx));
                                auto_probed = true;
                            }
                        }

                        this.add_inputs
                            .save_dir
                            .update(cx, |i, cx| i.set_value(default_dir, window, cx));
                        this.add_inputs
                            .proxy
                            .update(cx, |i, cx| i.set_value("", window, cx));
                        this.add_inputs.allow_overwrite = this.state.settings.allow_overwrite;
                        this.add_inputs.use_proxy = false;
                        this.add_inputs.replace_gid = None;
                        this.add_inputs.probe_status = None;
                        this.add_tab = AddTaskTab::Basic;
                        this.show_batch_modal = false;
                        this.show_settings_modal = false;
                        this.show_about_modal = false;
                        this.show_add_modal = true;
                        if auto_probed {
                            this.trigger_url_probe(window, cx);
                        }
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
                        this.batch_inputs.use_proxy = false;
                        this.show_add_modal = false;
                        this.show_settings_modal = false;
                        this.show_about_modal = false;
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
                    |this, _, _, cx| {
                        if let Some(gid) = this.state.selected_task_id.take() {
                            this.engine.send(EngineCommand::Remove {
                                gid,
                                delete_local_file: false,
                            });
                            this.notice = Some("已从列表移除任务（保留本地已下载文件）".into());
                            cx.notify();
                        }
                    },
                )),
            )
            .child(divider())
            .child(
                idm_tool_button("tb-folder", "📁", rgb(0xd97706), "打开目录", p).on_click(
                    cx.listener(|this, _, _, cx| {
                        let selected = this
                            .state
                            .selected_task_id
                            .as_deref()
                            .and_then(|id| this.snapshot.tasks.iter().find(|t| t.id == id));
                        if let Some(task) = selected {
                            let _ = reveal_file_in_directory(&task.full_path(), &task.save_dir);
                            this.notice = Some(format!("已定位文件: {}", task.full_path().display()));
                        } else {
                            let target_dir = this.state.settings.default_download_dir.clone();
                            let _ = open_directory(&target_dir);
                            this.notice = Some(format!("已打开目录: {}", target_dir.display()));
                        }
                        cx.notify();
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-opt", "⚙", rgb(0x2563eb), "选项", p).on_click(cx.listener(
                    |this, _, window, cx| {
                        // 每次打开设置弹窗时从当前已生效的 settings 重新构建表单，防止上次点「取消」的未保存脏数据残留
                        this.settings_inputs =
                            SettingsInputs::from_settings(&this.state.settings, window, cx);
                        this.show_add_modal = false;
                        this.show_batch_modal = false;
                        this.show_about_modal = false;
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
            .child(
                idm_tool_button("tb-tray", "⬇", rgb(0x475569), "隐藏托盘", p).on_click(
                    cx.listener(|_, _, _, _| {
                        hide_window("Gravita 下载器");
                    }),
                ),
            )
            .child(
                idm_tool_button("tb-about", "ⓘ", rgb(0x7c3aed), "关于", p).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_add_modal = false;
                        this.show_batch_modal = false;
                        this.show_settings_modal = false;
                        this.show_about_modal = true;
                        cx.notify();
                    },
                )),
            )
            .child(div().flex_1())
            .child(div().w(px(220.0)).pr_2().child(Input::new(&self.search_input)))
    }

    /// 渲染左侧经典 IDM 树形分类栏（支持点击折叠/展开分类子树，并完整覆盖全部 5 种任务状态）
    fn render_category_tree(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_filter;
        let expanded = self.state.categories_expanded;
        let all_label = if expanded {
            "▾ 全部下载"
        } else {
            "▸ 全部下载"
        };

        let mut nodes: Vec<(usize, SidebarFilter, &str, bool)> =
            vec![(0, SidebarFilter::All, all_label, false)];

        if expanded {
            nodes.extend([
                (1, SidebarFilter::Category(FileCategory::Compressed), "压缩文件", true),
                (2, SidebarFilter::Category(FileCategory::Document), "文档", true),
                (3, SidebarFilter::Category(FileCategory::Music), "音乐", true),
                (4, SidebarFilter::Category(FileCategory::Program), "程序", true),
                (5, SidebarFilter::Category(FileCategory::Video), "视频", true),
                (6, SidebarFilter::Category(FileCategory::Other), "其他", true),
            ]);
        }

        nodes.extend([
            (7, SidebarFilter::Active, "● 正在下载", false),
            (8, SidebarFilter::Waiting, "◔ 等待队列", false),
            (9, SidebarFilter::Paused, "❚❚ 已暂停", false),
            (10, SidebarFilter::Completed, "✓ 已完成", false),
            (11, SidebarFilter::Error, "⚠ 下载出错", false),
        ]);

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
                    .child("分类与状态"),
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
                                if filter == SidebarFilter::All && this.state.active_filter == SidebarFilter::All {
                                    this.state.categories_expanded = !this.state.categories_expanded;
                                } else {
                                    this.state.active_filter = filter;
                                }
                                cx.notify();
                            }))
                            .child(label)
                            .child(div().text_color(p.muted).child(format!("({count})")))
                    })),
            )
    }

    /// 渲染右侧真实任务列表表格（支持垂直滚动、双击直接打开文件、底部选中任务分片详情抽屉）
    fn render_task_list(&self, p: IdmPalette, cx: &mut Context<Self>) -> impl IntoElement {
        let tasks = self.visible_tasks(cx);
        let selected_id = self.state.selected_task_id.clone();
        let selected_task = selected_id
            .as_deref()
            .and_then(|id| self.snapshot.tasks.iter().find(|t| t.id == id))
            .cloned();

        let list_body = if tasks.is_empty() && !self.snapshot.connected {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(p.muted)
                .child("未连接到 aria2c，请检查内置引擎或启动 aria2c --enable-rpc")
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
                .id("task-scroll-container")
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .children(tasks.into_iter().enumerate().map(|(idx, task)| {
                    let task_id = task.id.clone();
                    let full_path = task.full_path();
                    let save_dir = task.save_dir.clone();
                    let status = task.status;
                    let is_selected = selected_id.as_deref() == Some(task.id.as_str());
                    render_idm_task_row(idx, &task, is_selected, p).on_click(cx.listener(
                        move |this, ev: &ClickEvent, _, cx| {
                            this.state.selected_task_id = Some(task_id.clone());
                            // 双击任务行：若已完成且文件存在则直接打开文件，否则定位目录
                            if ev.click_count() >= 2 {
                                if status == DownloadStatus::Completed && full_path.exists() {
                                    let _ = open_file(&full_path);
                                    this.notice =
                                        Some(format!("已打开文件: {}", full_path.display()));
                                } else {
                                    let _ = reveal_file_in_directory(&full_path, &save_dir);
                                    this.notice =
                                        Some(format!("已定位目录: {}", save_dir.display()));
                                }
                            }
                            cx.notify();
                        },
                    ))
                }))
                .into_any_element()
        };

        let mut panel = div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .bg(p.panel_bg)
            .child(render_idm_table_header(p))
            .child(list_body);

        if let Some(sel) = selected_task {
            let open_path = sel.full_path();
            let reveal_path = sel.full_path();
            let reveal_dir = sel.save_dir.clone();
            let copy_url = sel.url.clone();
            let refresh_req = idm_core::NewDownloadRequest {
                url: sel.url.clone(),
                filename: Some(sel.filename.clone()),
                save_dir: Some(sel.save_dir.clone()),
                headers: sel.headers.clone(),
                auto_start: true,
            };
            let refresh_gid = sel.id.clone();
            let del_gid = sel.id.clone();
            let collapsed = self.state.inspector_collapsed;

            panel = panel.child(render_task_inspector(
                &sel,
                collapsed,
                p,
                cx.listener(|this, _, _, cx| {
                    this.state.inspector_collapsed = !this.state.inspector_collapsed;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.state.selected_task_id = None;
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    if open_path.exists() {
                        let _ = open_file(&open_path);
                        this.notice = Some(format!("已打开文件: {}", open_path.display()));
                    } else {
                        this.notice = Some("本地文件尚未生成或已被移除".into());
                    }
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    let _ = reveal_file_in_directory(&reveal_path, &reveal_dir);
                    this.notice = Some(format!("已定位文件: {}", reveal_path.display()));
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copy_url.clone()));
                    this.notice = Some(format!("已复制下载链接: {copy_url}"));
                    cx.notify();
                }),
                cx.listener(move |this, _, window, cx| {
                    this.open_add_modal_with_request(refresh_req.clone(), window, cx);
                    this.add_inputs.allow_overwrite = true;
                    this.add_inputs.replace_gid = Some(refresh_gid.clone());
                    this.notice =
                        Some("请在弹窗中粘贴更新后的下载直链或 Cookie，点击「开始下载」将自动替换旧任务并无缝续传".into());
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    this.engine.send(EngineCommand::Remove {
                        gid: del_gid.clone(),
                        delete_local_file: true,
                    });
                    if this.state.selected_task_id.as_deref() == Some(del_gid.as_str()) {
                        this.state.selected_task_id = None;
                    }
                    this.notice = Some("已彻底删除该任务及本地文件".into());
                    cx.notify();
                }),
            ));
        }

        panel
    }

    /// 渲染底部经典状态栏（显示简洁就绪状态、选中任务错误原因与全局速度）
    fn render_statusbar(&self, p: IdmPalette) -> impl IntoElement {
        let status_text = if self.snapshot.connected {
            "● 已就绪".to_string()
        } else {
            "○ 引擎启动中...".to_string()
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
            let entity_proxy = cx.entity().clone();
            let configured_proxy = self.state.settings.proxy.clone();
            root = root.child(render_add_task_dialog(
                &self.add_inputs,
                self.add_tab,
                configured_proxy.as_deref(),
                p,
                move |tab, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.add_tab = tab;
                        cx.notify();
                    });
                },
                cx.listener(|this, _, window, cx| {
                    this.trigger_url_probe(window, cx);
                }),
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
                move |_, window, cx| {
                    entity_proxy.update(cx, |this, cx| {
                        this.add_inputs.use_proxy = !this.add_inputs.use_proxy;
                        this.trigger_url_probe(window, cx);
                        cx.notify();
                    });
                },
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
                        let resolved_proxy = this
                            .state
                            .settings
                            .resolve_task_proxy(this.add_inputs.use_proxy, Some(&task_proxy));
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
                            proxy: resolved_proxy,
                            replace_gid: this.add_inputs.replace_gid.take(),
                        });
                        this.add_inputs.url.update(cx, |i, cx| i.set_value("", window, cx));
                        this.add_inputs
                            .filename
                            .update(cx, |i, cx| i.set_value("", window, cx));
                        this.notice = Some("已提交下载任务到 aria2".into());
                        this.show_add_modal = false;
                        cx.notify();
                    }
                }),
            ));
        }

        if self.show_batch_modal {
            let preview_count = preview_batch_urls(&self.batch_inputs, cx).len();
            let configured_proxy = self.state.settings.proxy.clone();
            root = root.child(render_batch_dialog(
                &self.batch_inputs,
                preview_count,
                configured_proxy.as_deref(),
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
                    this.batch_inputs.use_proxy = !this.batch_inputs.use_proxy;
                    cx.notify();
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
                        let default_root = this
                            .state
                            .settings
                            .default_download_dir
                            .to_string_lossy()
                            .into_owned();
                        let use_auto_category = custom_dir.is_empty() || custom_dir == default_root;
                        if !custom_dir.is_empty() {
                            this.state.settings.last_save_dir = Some(PathBuf::from(&custom_dir));
                            let _ = this
                                .state
                                .settings
                                .save_to_file(&AppSettings::default_config_path());
                        }
                        let batch_proxy = this
                            .state
                            .settings
                            .resolve_task_proxy(this.batch_inputs.use_proxy, None);

                        let count = urls.len();
                        for url in urls {
                            let save_dir = if use_auto_category {
                                let inferred = idm_core::NewDownloadRequest {
                                    url: url.clone(),
                                    filename: None,
                                    save_dir: None,
                                    headers: vec![],
                                    auto_start: true,
                                }
                                .inferred_filename();
                                let cat = FileCategory::from_filename(&inferred);
                                this.state.settings.resolve_save_dir(cat)
                            } else {
                                PathBuf::from(&custom_dir)
                            };
                            this.engine.send(EngineCommand::AddUri {
                                url,
                                filename: None,
                                save_dir: Some(save_dir),
                                split: this.state.settings.default_split,
                                max_connection_per_server: this
                                    .state
                                    .settings
                                    .max_connection_per_server,
                                min_split_size: this.state.settings.min_split_size.clone(),
                                allow_overwrite: this.state.settings.allow_overwrite,
                                max_download_limit: None,
                                headers: vec![],
                                proxy: batch_proxy.clone(),
                                replace_gid: None,
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
                    let prev_startup_args = this.state.settings.to_aria2_startup_args();
                    let prev_secret = this.state.settings.aria2.secret.clone();

                    this.settings_inputs.apply_to(&mut this.state.settings, cx);
                    let path = AppSettings::default_config_path();
                    let _ = this.state.settings.save_to_file(&path);
                    let _ = apply_autostart(this.state.settings.auto_start);

                    let next_startup_args = this.state.settings.to_aria2_startup_args();
                    let next_secret = this.state.settings.aria2.secret.clone();

                    if prev_startup_args != next_startup_args || prev_secret != next_secret {
                        this.engine.send(EngineCommand::RestartEngine {
                            config: this.state.settings.aria2.clone(),
                            startup_args: next_startup_args,
                            global_options: this.state.settings.to_aria2_global_options(),
                        });
                        this.notice =
                            Some("检测到带 * 引擎级参数变更，已自动热重启内置 aria2c 并应用新配置".into());
                    } else {
                        Self::sync_global_options(&this.engine, &this.state.settings);
                        this.notice = Some("设置已保存并实时热更新到 aria2".into());
                    }
                    this.show_settings_modal = false;
                    cx.notify();
                }),
            ));
        }

        if self.show_about_modal {
            let aria2_ver = self.snapshot.aria2_version.clone();
            let upd_status = self.about_update_status.clone();
            root = root.child(render_about_dialog(
                aria2_ver.as_deref(),
                upd_status.as_deref(),
                self.checking_update,
                p,
                cx.listener(|this, _, _, cx| {
                    let _ = open_url_in_browser(GITHUB_REPO_URL);
                    this.notice = Some(format!("已在浏览器打开 GitHub 仓库: {GITHUB_REPO_URL}"));
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(GITHUB_REPO_URL.to_string()));
                    this.notice = Some(format!("已复制 GitHub 仓库地址: {GITHUB_REPO_URL}"));
                    cx.notify();
                }),
                cx.listener(|this, _, window, cx| {
                    this.trigger_update_check(true, window, cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_about_modal = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.show_about_modal = false;
                    cx.notify();
                }),
            ));
        }

        if let Some(info) = self.update_prompt.clone() {
            let ver_to_ignore = info.latest_version.clone();
            let release_url_open = info.html_url.clone();
            let release_url_fallback = info.html_url.clone();
            let dl_url = info.download_url.clone();
            let asset_name = info.asset_name.clone();

            root = root.child(render_update_dialog(
                &info,
                p,
                cx.listener(move |this, _, _, cx| {
                    this.state.settings.ignored_update_version = Some(ver_to_ignore.clone());
                    let _ = this
                        .state
                        .settings
                        .save_to_file(&AppSettings::default_config_path());
                    this.update_prompt = None;
                    this.notice = Some(format!("已忽略版本 {ver_to_ignore} 的自动更新提醒"));
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    let _ = open_url_in_browser(&release_url_open);
                    this.notice = Some("已在浏览器打开 GitHub Release 页面".into());
                    cx.notify();
                }),
                cx.listener(move |this, _, _, cx| {
                    if let (Some(url), Some(filename)) = (dl_url.clone(), asset_name.clone()) {
                        let save_dir =
                            this.state.settings.resolve_save_dir(FileCategory::Program);
                        let target_path = save_dir.join(&filename);
                        this.pending_update_installer = Some(target_path);
                        this.engine.send(EngineCommand::AddUri {
                            url,
                            filename: Some(filename.clone()),
                            save_dir: Some(save_dir),
                            split: this.state.settings.default_split,
                            max_connection_per_server: this
                                .state
                                .settings
                                .max_connection_per_server,
                            min_split_size: this.state.settings.min_split_size.clone(),
                            allow_overwrite: true,
                            max_download_limit: None,
                            headers: vec![],
                            proxy: this.state.settings.proxy.clone(),
                            replace_gid: None,
                        });
                        this.update_prompt = None;
                        this.notice = Some(format!(
                            "正在高速下载新版本安装包 {filename}，下载完成后将自动为您启动安装程序"
                        ));
                    } else {
                        let _ = open_url_in_browser(&release_url_fallback);
                        this.update_prompt = None;
                    }
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.update_prompt = None;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.update_prompt = None;
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
