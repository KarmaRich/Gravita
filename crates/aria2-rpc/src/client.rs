//! 基于 HTTP 的 aria2 异步 JSON-RPC 2.0 客户端

use crate::types::{AddUriOptions, Aria2TaskStatus, Aria2Version, GlobalOptions, GlobalStat};
use crate::{Aria2Config, Aria2Error, JsonRpcRequest, JsonRpcResponse};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

/// aria2 异步 RPC 客户端
#[derive(Debug)]
pub struct Aria2Client {
    http: reqwest::Client,
    config: Aria2Config,
    next_id: AtomicU64,
}

impl Aria2Client {
    /// 根据配置创建新的客户端实例（本地 RPC 默认绕过系统代理）
    pub fn new(config: Aria2Config) -> Result<Self, Aria2Error> {
        let http = reqwest::Client::builder().no_proxy().build()?;
        Ok(Self {
            http,
            config,
            next_id: AtomicU64::new(1),
        })
    }

    /// 通用 JSON-RPC 调用封装
    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Vec<Value>,
    ) -> Result<T, Aria2Error> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let req = JsonRpcRequest::new(id, method, params, self.config.secret.as_deref());

        let resp: JsonRpcResponse = self
            .http
            .post(&self.config.endpoint)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.error {
            return Err(Aria2Error::Rpc {
                code: err.code,
                message: err.message,
            });
        }

        let result = resp.result.ok_or(Aria2Error::MissingResult)?;
        Ok(serde_json::from_value(result)?)
    }

    /// 获取 aria2 版本信息（常用于健康检查）
    pub async fn get_version(&self) -> Result<Aria2Version, Aria2Error> {
        self.call("aria2.getVersion", vec![]).await
    }

    /// 添加新的 HTTP/HTTPS/FTP 下载任务，返回任务 GID
    pub async fn add_uri(
        &self,
        uris: Vec<String>,
        options: Option<AddUriOptions>,
    ) -> Result<String, Aria2Error> {
        let mut params = vec![json!(uris)];
        if let Some(opts) = options {
            params.push(serde_json::to_value(opts)?);
        }
        self.call("aria2.addUri", params).await
    }

    /// 暂停指定任务
    pub async fn pause(&self, gid: &str) -> Result<String, Aria2Error> {
        self.call("aria2.pause", vec![json!(gid)]).await
    }

    /// 暂停全部任务
    pub async fn pause_all(&self) -> Result<String, Aria2Error> {
        self.call("aria2.pauseAll", vec![]).await
    }

    /// 恢复指定任务
    pub async fn unpause(&self, gid: &str) -> Result<String, Aria2Error> {
        self.call("aria2.unpause", vec![json!(gid)]).await
    }

    /// 恢复全部任务
    pub async fn unpause_all(&self) -> Result<String, Aria2Error> {
        self.call("aria2.unpauseAll", vec![]).await
    }

    /// 移除指定正在下载或等待中的任务
    pub async fn remove(&self, gid: &str) -> Result<String, Aria2Error> {
        self.call("aria2.remove", vec![json!(gid)]).await
    }

    /// 清除已完成/出错/已移除任务的记录
    pub async fn remove_download_result(&self, gid: &str) -> Result<String, Aria2Error> {
        self.call("aria2.removeDownloadResult", vec![json!(gid)])
            .await
    }

    /// 查询单个任务的详细状态
    pub async fn tell_status(&self, gid: &str) -> Result<Aria2TaskStatus, Aria2Error> {
        self.call("aria2.tellStatus", vec![json!(gid)]).await
    }

    /// 查询所有活跃下载任务
    pub async fn tell_active(&self) -> Result<Vec<Aria2TaskStatus>, Aria2Error> {
        self.call("aria2.tellActive", vec![]).await
    }

    /// 查询等待中的任务列表
    pub async fn tell_waiting(&self, offset: i64, num: i64) -> Result<Vec<Aria2TaskStatus>, Aria2Error> {
        self.call("aria2.tellWaiting", vec![json!(offset), json!(num)])
            .await
    }

    /// 查询已停止（完成/错误/移除）的任务列表
    pub async fn tell_stopped(&self, offset: i64, num: i64) -> Result<Vec<Aria2TaskStatus>, Aria2Error> {
        self.call("aria2.tellStopped", vec![json!(offset), json!(num)])
            .await
    }

    /// 获取全局下载/上传速度与任务统计
    pub async fn get_global_stat(&self) -> Result<GlobalStat, Aria2Error> {
        self.call("aria2.getGlobalStat", vec![]).await
    }

    /// 动态修改 aria2 全局参数（如全局限速 `max-overall-download-limit`）
    pub async fn change_global_option(&self, options: GlobalOptions) -> Result<String, Aria2Error> {
        self.call("aria2.changeGlobalOption", vec![json!(options)])
            .await
    }

    /// 立即将当前未完成会话持久化写入磁盘（对应 `aria2.saveSession`）
    pub async fn save_session(&self) -> Result<String, Aria2Error> {
        self.call("aria2.saveSession", vec![]).await
    }

    /// 请求 aria2 优雅关闭（对应 `aria2.shutdown`）
    pub async fn shutdown(&self) -> Result<String, Aria2Error> {
        self.call("aria2.shutdown", vec![]).await
    }
}
