//! 桌面端异步下载引擎控制器：负责后台连接 aria2 RPC、定时同步任务进度与执行控制指令

use aria2_rpc::{
    AddUriOptions, Aria2Client, Aria2Config, Aria2Process, Aria2ProcessConfig, Aria2TaskStatus,
};
use idm_core::{AppSettings, DownloadStatus, DownloadTask, FileCategory, NewDownloadRequest, TaskStore};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 发往后台引擎的控制指令
#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// 提交新下载任务到 aria2（支持覆盖策略、换链无缝续传、单任务限速、自定义 HTTP 头与代理）
    AddUri {
        url: String,
        filename: Option<String>,
        save_dir: Option<PathBuf>,
        split: u32,
        max_connection_per_server: u32,
        min_split_size: String,
        allow_overwrite: bool,
        max_download_limit: Option<String>,
        headers: Vec<String>,
        proxy: Option<String>,
        /// 若为「刷新过期链接/续传」，指定要替换并清理的旧任务 GID
        replace_gid: Option<String>,
    },
    /// 暂停单个任务
    Pause(String),
    /// 恢复单个任务（若该任务为重启后不在 aria2 内存中的历史记录，则自动携带原 URL 与目录重新拉起续传）
    Unpause(String),
    /// 暂停全部任务
    PauseAll,
    /// 恢复全部暂停任务
    UnpauseAll,
    /// 删除单个任务（可选同时删除本地已下载的文件及 `.aria2` 控制文件）
    Remove {
        gid: String,
        delete_local_file: bool,
    },
    /// 实时更新 aria2 全局参数（由 AppSettings::to_aria2_global_options 生成）
    ApplyGlobalOptions(aria2_rpc::GlobalOptions),
    /// 当修改了带 `*` 的引擎启动级参数（如磁盘缓存、预分配、证书校验、RPC Secret、DHT 等）时，热重启内置 `aria2c` 进程并无缝重连
    RestartEngine {
        config: Aria2Config,
        startup_args: Vec<String>,
        global_options: aria2_rpc::GlobalOptions,
    },
    /// 立即触发一次状态同步
    SyncNow,
    /// 保存会话与历史记录，优雅关闭 aria2c 子进程并退出应用
    ShutdownAndExit,
}

/// 后台引擎同步给 UI 的只读快照
#[derive(Debug, Clone, Default)]
pub struct EngineSnapshot {
    pub connected: bool,
    pub aria2_version: Option<String>,
    pub tasks: Vec<DownloadTask>,
    pub global_download_speed: u64,
    pub last_error: Option<String>,
}

/// 桌面端后台下载引擎句柄（同时托管内置 `aria2c` 子进程生命周期与插件待确认请求队列）
pub struct DownloadEngine {
    cmd_tx: mpsc::UnboundedSender<EngineCommand>,
    snapshot: Arc<Mutex<EngineSnapshot>>,
    /// 来自浏览器插件的待确认下载请求队列（由 UI 弹出「新建下载任务」窗口确认）
    pending_requests: Arc<Mutex<VecDeque<NewDownloadRequest>>>,
    /// 持有内置 aria2c 进程句柄，供优雅退出或 Drop 时清理
    managed_process: Arc<Mutex<Option<Aria2Process>>>,
}

impl DownloadEngine {
    /// 自动拉起内置 `aria2c` 守护进程、启动本地 `BridgeServer` 并开启后台 Tokio 状态同步线程
    pub fn start(
        mut config: Aria2Config,
        startup_args: Vec<String>,
        bridge_port: u16,
        bridge_token: Option<String>,
    ) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let tasks_path = AppSettings::default_tasks_path();
        let mut initial_store = TaskStore::load_from_file(&tasks_path).unwrap_or_default();
        // 启动时将上次残留的 Active 状态归位为 Paused，待 aria2 同步后刷新真实状态
        for t in initial_store.all().to_vec() {
            if t.status == DownloadStatus::Active {
                let mut paused = t;
                paused.status = DownloadStatus::Paused;
                paused.speed_bytes_per_sec = 0;
                initial_store.upsert(paused);
            }
        }

        let snapshot = Arc::new(Mutex::new(EngineSnapshot {
            tasks: initial_store.all().to_vec(),
            ..Default::default()
        }));
        let pending_requests = Arc::new(Mutex::new(VecDeque::new()));

        // 优先静默启动项目内置的 aria2c（使用专属端口 16801，避免与外部 aria2 冲突）
        let proc_cfg = Aria2ProcessConfig {
            binary_path: Aria2ProcessConfig::resolve_bundled_binary(),
            rpc_port: 16801,
            rpc_secret: config.secret.clone(),
            extra_args: startup_args,
            ..Default::default()
        };
        let spawned_process = match Aria2Process::spawn(proc_cfg.clone()) {
            Ok(proc) => {
                config.endpoint = proc_cfg.rpc_endpoint();
                Some(proc)
            }
            Err(err) => {
                if let Ok(mut s) = snapshot.lock() {
                    s.last_error = Some(format!("拉起内置 aria2c 失败，尝试连接外部 RPC: {err}"));
                }
                None
            }
        };
        let managed_process = Arc::new(Mutex::new(spawned_process));

        let snap_clone = Arc::clone(&snapshot);
        let pending_clone = Arc::clone(&pending_requests);
        let proc_clone = Arc::clone(&managed_process);
        let store_arc = Arc::new(Mutex::new(initial_store));
        let url_cache: Arc<Mutex<HashMap<String, (String, Vec<String>)>>> =
            Arc::new(Mutex::new(HashMap::new()));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("创建后台 Tokio 运行时失败");

            rt.block_on(async move {
                let mut client = match Aria2Client::new(config) {
                    Ok(c) => c,
                    Err(e) => {
                        if let Ok(mut s) = snap_clone.lock() {
                            s.last_error = Some(e.to_string());
                        }
                        return;
                    }
                };

                // 启动本地 HTTP 桥接服务，收到插件拦截的下载请求后入队，交由 UI 弹窗确认
                let (bridge_tx, mut bridge_rx) = mpsc::unbounded_channel();
                let bind_addr = format!("127.0.0.1:{bridge_port}");
                let _bridge = idm_core::BridgeServer::start(&bind_addr, bridge_token, bridge_tx)
                    .await
                    .ok();

                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(800));
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            sync_snapshot(&client, &snap_clone, &store_arc, &url_cache, &tasks_path).await;
                        }
                        Some(cmd) = cmd_rx.recv() => {
                            match cmd {
                                EngineCommand::ShutdownAndExit => {
                                    let _ = client.save_session().await;
                                    let _ = client.shutdown().await;
                                    if let Ok(store) = store_arc.lock() {
                                        let _ = store.save_to_file(&tasks_path);
                                    }
                                    if let Ok(mut guard) = proc_clone.lock() {
                                        if let Some(mut p) = guard.take() {
                                            let _ = p.stop();
                                        }
                                    }
                                    std::process::exit(0);
                                }
                                EngineCommand::RestartEngine {
                                    mut config,
                                    startup_args,
                                    global_options,
                                } => {
                                    let _ = client.save_session().await;
                                    let _ = client.shutdown().await;
                                    if let Ok(mut guard) = proc_clone.lock() {
                                        if let Some(mut p) = guard.take() {
                                            let _ = p.stop();
                                        }
                                        let new_cfg = Aria2ProcessConfig {
                                            binary_path: Aria2ProcessConfig::resolve_bundled_binary(),
                                            rpc_port: 16801,
                                            rpc_secret: config.secret.clone(),
                                            extra_args: startup_args,
                                            ..Default::default()
                                        };
                                        match Aria2Process::spawn(new_cfg.clone()) {
                                            Ok(proc) => {
                                                config.endpoint = new_cfg.rpc_endpoint();
                                                *guard = Some(proc);
                                            }
                                            Err(err) => {
                                                if let Ok(mut s) = snap_clone.lock() {
                                                    s.last_error = Some(format!("重启内置 aria2c 失败: {err}"));
                                                }
                                            }
                                        }
                                    }
                                    if let Ok(new_client) = Aria2Client::new(config) {
                                        client = new_client;
                                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                        let _ = client.change_global_option(global_options).await;
                                    }
                                }
                                other => {
                                    handle_command(&client, other, &snap_clone, &store_arc, &url_cache, &tasks_path).await;
                                }
                            }
                            sync_snapshot(&client, &snap_clone, &store_arc, &url_cache, &tasks_path).await;
                        }
                        Some(req) = bridge_rx.recv() => {
                            if let Ok(mut q) = pending_clone.lock() {
                                q.push_back(req);
                            }
                        }
                    }
                }
            });
        });

        Self {
            cmd_tx,
            snapshot,
            pending_requests,
            managed_process,
        }
    }

    /// 取出一条来自浏览器插件的待确认下载请求（供 UI 弹出「新建下载任务」窗口）
    pub fn pop_pending_request(&self) -> Option<NewDownloadRequest> {
        self.pending_requests.lock().ok()?.pop_front()
    }

    /// 发送异步控制指令（非阻塞，不卡 UI 线程）
    pub fn send(&self, cmd: EngineCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// 克隆命令发送端（供系统托盘菜单等外部模块控制引擎）
    pub fn command_sender(&self) -> mpsc::UnboundedSender<EngineCommand> {
        self.cmd_tx.clone()
    }

    /// 读取最新引擎快照
    pub fn snapshot(&self) -> EngineSnapshot {
        self.snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
}

impl Drop for DownloadEngine {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.managed_process.lock() {
            if let Some(mut p) = guard.take() {
                let _ = p.stop();
            }
        }
    }
}

async fn handle_command(
    client: &Aria2Client,
    cmd: EngineCommand,
    snap: &Arc<Mutex<EngineSnapshot>>,
    store: &Arc<Mutex<TaskStore>>,
    url_cache: &Arc<Mutex<HashMap<String, (String, Vec<String>)>>>,
    tasks_path: &std::path::Path,
) {
    let result: Result<(), String> = match cmd {
        EngineCommand::AddUri {
            url,
            filename,
            save_dir,
            split,
            max_connection_per_server,
            min_split_size,
            allow_overwrite,
            max_download_limit,
            headers,
            proxy,
            replace_gid,
        } => {
            if let Some(old_gid) = replace_gid {
                let _ = client.remove(&old_gid).await;
                let _ = client.remove_download_result(&old_gid).await;
                if let Ok(mut cache) = url_cache.lock() {
                    cache.remove(&old_gid);
                }
                if let Ok(mut st) = store.lock() {
                    if st.remove(&old_gid) {
                        let _ = st.save_to_file(tasks_path);
                    }
                }
            }
            let opts = AddUriOptions {
                dir: save_dir.map(|p| p.to_string_lossy().into_owned()),
                out: filename.filter(|s| !s.trim().is_empty()),
                split: Some(split.to_string()),
                max_connection_per_server: Some(max_connection_per_server.min(64).to_string()),
                min_split_size: Some(min_split_size),
                header: headers.clone(),
                all_proxy: proxy.filter(|s| !s.trim().is_empty()),
                max_download_limit: max_download_limit.filter(|s| !s.trim().is_empty()),
                allow_overwrite: Some(if allow_overwrite { "true" } else { "false" }.into()),
                auto_file_renaming: Some(if allow_overwrite { "false" } else { "true" }.into()),
                continue_download: Some("true".into()),
            };
            client
                .add_uri(vec![url.clone()], Some(opts))
                .await
                .map(|gid| {
                    if let Ok(mut cache) = url_cache.lock() {
                        cache.insert(gid, (url, headers));
                    }
                })
                .map_err(|e| e.to_string())
        }
        EngineCommand::Pause(gid) => client.pause(&gid).await.map(|_| ()).map_err(|e| e.to_string()),
        EngineCommand::Unpause(gid) => {
            match client.unpause(&gid).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    // 若该任务是上次运行留存于 tasks.json 但已不在 aria2 内存中的历史任务，自动使用原 URL 和目录重新拉起断点续传
                    let fallback_task = store
                        .lock()
                        .ok()
                        .and_then(|st| st.all().iter().find(|t| t.id == gid).cloned());
                    if let Some(task) = fallback_task.filter(|t| {
                        t.url.starts_with("http://")
                            || t.url.starts_with("https://")
                            || t.url.starts_with("magnet:?")
                            || t.url.starts_with("ftp://")
                    }) {
                        let opts = AddUriOptions {
                            dir: Some(task.save_dir.to_string_lossy().into_owned()),
                            out: Some(task.filename.clone()),
                            header: task.headers.clone(),
                            allow_overwrite: Some("true".into()),
                            auto_file_renaming: Some("false".into()),
                            continue_download: Some("true".into()),
                            ..Default::default()
                        };
                        match client.add_uri(vec![task.url.clone()], Some(opts)).await {
                            Ok(new_gid) => {
                                if let Ok(mut cache) = url_cache.lock() {
                                    cache.remove(&gid);
                                    cache.insert(new_gid, (task.url, task.headers));
                                }
                                if let Ok(mut st) = store.lock() {
                                    st.remove(&gid);
                                    let _ = st.save_to_file(tasks_path);
                                }
                                Ok(())
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    } else {
                        Err(err.to_string())
                    }
                }
            }
        }
        EngineCommand::PauseAll => client.pause_all().await.map(|_| ()).map_err(|e| e.to_string()),
        EngineCommand::UnpauseAll => client
            .unpause_all()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        EngineCommand::Remove {
            gid,
            delete_local_file,
        } => {
            let _ = client.remove(&gid).await;
            let _ = client.remove_download_result(&gid).await;
            if let Ok(mut cache) = url_cache.lock() {
                cache.remove(&gid);
            }
            if let Ok(mut st) = store.lock() {
                if delete_local_file {
                    if let Some(task) = st.all().iter().find(|t| t.id == gid) {
                        let file_path = task.full_path();
                        let aria2_meta = PathBuf::from(format!("{}.aria2", file_path.display()));
                        let _ = std::fs::remove_file(&file_path);
                        let _ = std::fs::remove_file(&aria2_meta);
                    }
                }
                if st.remove(&gid) {
                    let _ = st.save_to_file(tasks_path);
                }
            }
            Ok(())
        }
        EngineCommand::ApplyGlobalOptions(opts) => client
            .change_global_option(opts)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        EngineCommand::SyncNow
        | EngineCommand::ShutdownAndExit
        | EngineCommand::RestartEngine { .. } => Ok(()),
    };

    if let Err(err) = result {
        if let Ok(mut s) = snap.lock() {
            s.last_error = Some(err);
        }
    }
}

async fn sync_snapshot(
    client: &Aria2Client,
    snap: &Arc<Mutex<EngineSnapshot>>,
    store: &Arc<Mutex<TaskStore>>,
    url_cache: &Arc<Mutex<HashMap<String, (String, Vec<String>)>>>,
    tasks_path: &std::path::Path,
) {
    match client.get_version().await {
        Ok(ver) => {
            let active = client.tell_active().await.unwrap_or_default();
            let waiting = client.tell_waiting(0, 200).await.unwrap_or_default();
            let stopped = client.tell_stopped(0, 200).await.unwrap_or_default();
            let stat = client.get_global_stat().await.ok();

            let mut live_tasks = Vec::new();
            for item in active.into_iter().chain(waiting).chain(stopped) {
                if item.status == "removed" {
                    continue;
                }
                let mut task = aria2_status_to_download_task(item);
                if let Ok(cache) = url_cache.lock() {
                    if let Some((cached_url, cached_headers)) = cache.get(&task.id) {
                        if !cached_url.is_empty() {
                            task.url = cached_url.clone();
                        }
                        if task.headers.is_empty() {
                            task.headers = cached_headers.clone();
                        }
                    }
                }
                live_tasks.push(task);
            }

            let all_tasks = if let Ok(mut st) = store.lock() {
                if st.merge_live_tasks(live_tasks) {
                    let _ = st.save_to_file(tasks_path);
                }
                // 清理已完成或出错任务的 url_cache 内存缓存，防止长期运行内存无限增长
                if let Ok(mut cache) = url_cache.lock() {
                    for t in st.all() {
                        if matches!(t.status, DownloadStatus::Completed | DownloadStatus::Error) {
                            cache.remove(&t.id);
                        }
                    }
                }
                st.all().to_vec()
            } else {
                live_tasks
            };

            if let Ok(mut s) = snap.lock() {
                s.connected = true;
                s.aria2_version = Some(ver.version);
                s.tasks = all_tasks;
                s.global_download_speed = stat
                    .and_then(|st| st.download_speed.parse().ok())
                    .unwrap_or(0);
                s.last_error = None;
            }
        }
        Err(e) => {
            if let Ok(mut s) = snap.lock() {
                s.connected = false;
                s.aria2_version = None;
                s.last_error = Some(format!("未连接到 aria2: {e}"));
            }
        }
    }
}

/// 将 aria2 返回的任务状态转换为桌面端统一的 `DownloadTask`
pub fn aria2_status_to_download_task(item: Aria2TaskStatus) -> DownloadTask {
    let status = match item.status.as_str() {
        "active" => DownloadStatus::Active,
        "waiting" => DownloadStatus::Waiting,
        "paused" => DownloadStatus::Paused,
        "complete" => DownloadStatus::Completed,
        _ => DownloadStatus::Error,
    };

    let first_file = item.files.first();
    let raw_path = first_file.map(|f| f.path.as_str()).unwrap_or("");
    let real_uri = first_file
        .and_then(|f| f.uris.first())
        .map(|u| u.uri.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(raw_path);

    let filename = PathBuf::from(raw_path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            if !real_uri.is_empty() {
                Some(
                    NewDownloadRequest {
                        url: real_uri.to_string(),
                        filename: None,
                        save_dir: None,
                        headers: vec![],
                        auto_start: true,
                    }
                    .inferred_filename(),
                )
            } else {
                None
            }
        })
        .unwrap_or_else(|| format!("task-{}", item.gid));

    let category = FileCategory::from_filename(&filename);

    let error_message = if status == DownloadStatus::Error {
        let code = item.error_code.as_deref().unwrap_or("");
        let msg = item.error_message.as_deref().unwrap_or("未知错误");
        Some(if code.is_empty() {
            msg.to_string()
        } else {
            format!("错误码 {code}: {msg}")
        })
    } else {
        None
    };

    DownloadTask {
        id: item.gid.clone(),
        gid: Some(item.gid.clone()),
        url: real_uri.to_string(),
        filename,
        save_dir: PathBuf::from(&item.dir),
        category,
        status,
        total_bytes: item.total_length_u64(),
        completed_bytes: item.completed_length_u64(),
        speed_bytes_per_sec: item.download_speed_u64(),
        connections: item.connections_u32(),
        bitfield: item.bitfield.clone(),
        num_pieces: item.num_pieces_u32(),
        piece_length: item.piece_length_u64(),
        headers: vec![],
        error_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria2_rpc::{Aria2File, Aria2Uri};

    #[test]
    fn test_aria2_status_to_download_task_conversion() {
        let raw = Aria2TaskStatus {
            gid: "g001".into(),
            status: "active".into(),
            total_length: "2048".into(),
            completed_length: "1024".into(),
            download_speed: "512".into(),
            upload_speed: "0".into(),
            connections: "4".into(),
            dir: "D:/Downloads".into(),
            files: vec![Aria2File {
                index: "1".into(),
                path: "D:/Downloads/archive.zip".into(),
                length: "2048".into(),
                completed_length: "1024".into(),
                uris: vec![Aria2Uri {
                    uri: "https://example.com/archive.zip".into(),
                    status: "used".into(),
                }],
            }],
            bitfield: Some("f0".into()),
            num_pieces: Some("8".into()),
            piece_length: Some("256".into()),
            error_code: None,
            error_message: None,
        };

        let task = aria2_status_to_download_task(raw);
        assert_eq!(task.id, "g001");
        assert_eq!(task.url, "https://example.com/archive.zip");
        assert_eq!(task.filename, "archive.zip");
        assert_eq!(task.category, FileCategory::Compressed);
        assert_eq!(task.status, DownloadStatus::Active);
        assert_eq!(task.total_bytes, 2048);
        assert_eq!(task.completed_bytes, 1024);
        assert_eq!(task.speed_bytes_per_sec, 512);
        assert_eq!(task.connections, 4);
    }
}
