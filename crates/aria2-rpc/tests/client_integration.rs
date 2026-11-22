//! Aria2Client 集成测试：使用本地 Mock HTTP RPC 服务器验证真实请求与响应解析

use aria2_rpc::{AddUriOptions, Aria2Client, Aria2Config, Aria2Error, JsonRpcRequest};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// 启动一次性 Mock HTTP JSON-RPC 服务器，返回监听地址与请求校验结果
async fn spawn_mock_rpc_server(
    response_body: String,
) -> (String, tokio::task::JoinHandle<JsonRpcRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = format!("http://{addr}/jsonrpc");

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let raw = String::from_utf8_lossy(&buf[..n]);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("");
        let parsed: JsonRpcRequest = serde_json::from_str(body).unwrap();

        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
        parsed
    });

    (endpoint, handle)
}

#[tokio::test]
async fn test_add_uri_and_token_injection() {
    let mock_resp = json!({
        "jsonrpc": "2.0",
        "id": "1",
        "result": "2089b05ecca3d829"
    })
    .to_string();

    let (endpoint, handle) = spawn_mock_rpc_server(mock_resp).await;
    let client = Aria2Client::new(Aria2Config {
        endpoint,
        secret: Some("test-token".to_string()),
    })
    .unwrap();

    let opts = AddUriOptions {
        dir: Some("D:/Downloads".to_string()),
        out: Some("demo.zip".to_string()),
        split: Some("16".to_string()),
        header: vec!["Referer: https://example.com".to_string()],
        ..Default::default()
    };

    let gid = client
        .add_uri(vec!["https://example.com/demo.zip".to_string()], Some(opts))
        .await
        .unwrap();

    assert_eq!(gid, "2089b05ecca3d829");

    let received = handle.await.unwrap();
    assert_eq!(received.method, "aria2.addUri");
    assert_eq!(received.params[0], json!("token:test-token"));
    assert_eq!(received.params[1], json!(["https://example.com/demo.zip"]));
    assert_eq!(received.params[2]["out"], json!("demo.zip"));
}

#[tokio::test]
async fn test_tell_status_and_progress_calculation() {
    let mock_resp = json!({
        "jsonrpc": "2.0",
        "id": "1",
        "result": {
            "gid": "abc123",
            "status": "active",
            "totalLength": "1000",
            "completedLength": "250",
            "downloadSpeed": "102400",
            "uploadSpeed": "0",
            "connections": "8",
            "dir": "D:/Downloads",
            "files": []
        }
    })
    .to_string();

    let (endpoint, _handle) = spawn_mock_rpc_server(mock_resp).await;
    let client = Aria2Client::new(Aria2Config {
        endpoint,
        secret: None,
    })
    .unwrap();

    let status = client.tell_status("abc123").await.unwrap();
    assert_eq!(status.gid, "abc123");
    assert_eq!(status.total_length_u64(), 1000);
    assert_eq!(status.completed_length_u64(), 250);
    assert_eq!(status.download_speed_u64(), 102400);
    assert!((status.progress_percent() - 25.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_rpc_error_response() {
    let mock_resp = json!({
        "jsonrpc": "2.0",
        "id": "1",
        "error": {
            "code": 1,
            "message": "Unauthorized"
        }
    })
    .to_string();

    let (endpoint, _handle) = spawn_mock_rpc_server(mock_resp).await;
    let client = Aria2Client::new(Aria2Config {
        endpoint,
        secret: Some("wrong".to_string()),
    })
    .unwrap();

    let err = client.get_version().await.unwrap_err();
    match err {
        Aria2Error::Rpc { code, message } => {
            assert_eq!(code, 1);
            assert_eq!(message, "Unauthorized");
        }
        other => panic!("期望 Rpc 错误，实际得到: {other:?}"),
    }
}
