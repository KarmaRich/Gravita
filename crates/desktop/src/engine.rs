//! 桌面端异步下载引擎控制器：负责后台连接 aria2 RPC、定时同步任务进度与执行控制指令

use aria2_rpc::{AddUriOptions, Aria2Client, Aria2Config, Aria2TaskStatus};
use idm_core::{DownloadStatus, DownloadTask, FileCategory};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 发往后台引擎的控制指令
#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// 提交新下载任务到 aria2
    AddUri {
        url: String,
        filename: Option<String>,
        save_dir: Option<PathBuf>,
        split: u32,
    },
    /// 暂停单个任务
    Pause(String),
    /// 恢复单个任务
    Unpause(String),
    /// 暂停全部任务
    PauseAll,
    /// 删除单个任务（同时清理停止记录）
    Remove(String),
    /// 立即触发一次状态同步
    SyncNow,
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

/// 桌面端后台下载引擎句柄
pub struct DownloadEngine {
    cmd_tx: mpsc::UnboundedSender<EngineCommand>,
    snapshot: Arc<Mutex<EngineSnapshot>>,
}

impl DownloadEngine {
    /// 启动后台 Tokio 线程并开始定时同步 aria2 状态
    pub fn start(config: Aria2Config) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let snapshot = Arc::new(Mutex::new(EngineSnapshot::default()));
        let snap_clone = Arc::clone(&snapshot);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("创建后台 Tokio 运行时失败");

            rt.block_on(async move {
                let client = match Aria2Client::new(config) {
                    Ok(c) => c,
                    Err(e) => {
                        if let Ok(mut s) = snap_clone.lock() {
                            s.last_error = Some(e.to_string());
                        }
                        return;
                    }
                };

                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(800));
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            sync_snapshot(&client, &snap_clone).await;
                        }
                        Some(cmd) = cmd_rx.recv() => {
                            handle_command(&client, cmd, &snap_clone).await;
                            sync_snapshot(&client, &snap_clone).await;
                        }
                    }
                }
            });
        });

        Self { cmd_tx, snapshot }
    }

    /// 发送异步控制指令（非阻塞，不卡 UI 线程）
    pub fn send(&self, cmd: EngineCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// 读取最新引擎快照
    pub fn snapshot(&self) -> EngineSnapshot {
        self.snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
}

async fn handle_command(
    client: &Aria2Client,
    cmd: EngineCommand,
    snap: &Arc<Mutex<EngineSnapshot>>,
) {
    let result: Result<(), String> = match cmd {
        EngineCommand::AddUri {
            url,
            filename,
            save_dir,
            split,
        } => {
            let opts = AddUriOptions {
                dir: save_dir.map(|p| p.to_string_lossy().into_owned()),
                out: filename.filter(|s| !s.trim().is_empty()),
                split: Some(split.to_string()),
                max_connection_per_server: Some(split.min(16).to_string()),
                ..Default::default()
            };
            client
                .add_uri(vec![url], Some(opts))
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        EngineCommand::Pause(gid) => client.pause(&gid).await.map(|_| ()).map_err(|e| e.to_string()),
        EngineCommand::Unpause(gid) => client.unpause(&gid).await.map(|_| ()).map_err(|e| e.to_string()),
        EngineCommand::PauseAll => client.pause_all().await.map(|_| ()).map_err(|e| e.to_string()),
        EngineCommand::Remove(gid) => {
            let _ = client.remove(&gid).await;
            client
                .remove_download_result(&gid)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        EngineCommand::SyncNow => Ok(()),
    };

    if let Err(err) = result {
        if let Ok(mut s) = snap.lock() {
            s.last_error = Some(err);
        }
    }
}

async fn sync_snapshot(client: &Aria2Client, snap: &Arc<Mutex<EngineSnapshot>>) {
    match client.get_version().await {
        Ok(ver) => {
            let active = client.tell_active().await.unwrap_or_default();
            let waiting = client.tell_waiting(0, 100).await.unwrap_or_default();
            let stopped = client.tell_stopped(0, 100).await.unwrap_or_default();
            let stat = client.get_global_stat().await.ok();

            let mut tasks = Vec::new();
            for item in active.into_iter().chain(waiting).chain(stopped) {
                tasks.push(aria2_status_to_download_task(item));
            }

            if let Ok(mut s) = snap.lock() {
                s.connected = true;
                s.aria2_version = Some(ver.version);
                s.tasks = tasks;
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

    let raw_path = item
        .files
        .first()
        .map(|f| f.path.as_str())
        .unwrap_or("");
    let filename = PathBuf::from(raw_path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("task-{}", item.gid));

    let category = FileCategory::from_filename(&filename);

    DownloadTask {
        id: item.gid.clone(),
        gid: Some(item.gid.clone()),
        url: raw_path.to_string(),
        filename,
        save_dir: PathBuf::from(&item.dir),
        category,
        status,
        total_bytes: item.total_length_u64(),
        completed_bytes: item.completed_length_u64(),
        speed_bytes_per_sec: item.download_speed_u64(),
        headers: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria2_rpc::Aria2File;

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
            }],
            error_code: None,
            error_message: None,
        };

        let task = aria2_status_to_download_task(raw);
        assert_eq!(task.id, "g001");
        assert_eq!(task.filename, "archive.zip");
        assert_eq!(task.category, FileCategory::Compressed);
        assert_eq!(task.status, DownloadStatus::Active);
        assert_eq!(task.total_bytes, 2048);
        assert_eq!(task.completed_bytes, 1024);
        assert_eq!(task.speed_bytes_per_sec, 512);
    }
}
