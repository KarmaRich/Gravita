//! aria2 JSON-RPC 2.0 客户端与进程托管库

pub mod client;
pub mod process;
pub mod types;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub use client::Aria2Client;
pub use process::{Aria2Process, Aria2ProcessConfig};
pub use types::{
    AddUriOptions, Aria2File, Aria2TaskStatus, Aria2Uri, Aria2Version, GlobalOptions, GlobalStat,
};

/// aria2 RPC 通信与协议错误定义
#[derive(Debug, Error)]
pub enum Aria2Error {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("序列化或反序列化 JSON 失败: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("aria2 返回业务错误 (code={code}): {message}")]
    Rpc { code: i64, message: String },

    #[error("aria2 响应缺少 result 字段")]
    MissingResult,
}

/// JSON-RPC 2.0 请求结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<Value>,
}

/// JSON-RPC 2.0 错误对象
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
}

/// JSON-RPC 2.0 响应结构
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcErrorObject>,
}

/// aria2 客户端连接配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Aria2Config {
    /// RPC 服务地址，例如 `http://127.0.0.1:6800/jsonrpc`
    pub endpoint: String,
    /// 可选的 RPC 密钥（对应 aria2c 的 `--rpc-secret`）
    pub secret: Option<String>,
}

impl Default for Aria2Config {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:6800/jsonrpc".to_string(),
            secret: None,
        }
    }
}

impl JsonRpcRequest {
    /// 构造标准 aria2 JSON-RPC 请求，若配置了 secret 则自动注入 `token:<secret>` 作为首个参数
    pub fn new(
        id: impl Into<String>,
        method: impl Into<String>,
        mut params: Vec<Value>,
        secret: Option<&str>,
    ) -> Self {
        if let Some(token) = secret.filter(|s| !s.is_empty()) {
            params.insert(0, Value::String(format!("token:{token}")));
        }

        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_request_without_secret() {
        let req = JsonRpcRequest::new("req-1", "aria2.getVersion", vec![], None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, "req-1");
        assert_eq!(req.method, "aria2.getVersion");
        assert!(req.params.is_empty());
    }

    #[test]
    fn test_build_request_with_secret_prepends_token() {
        let req = JsonRpcRequest::new(
            "req-2",
            "aria2.addUri",
            vec![json!(["https://example.com/app.zip"])],
            Some("my-secret"),
        );
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0], json!("token:my-secret"));
        assert_eq!(req.params[1], json!(["https://example.com/app.zip"]));
    }
}
