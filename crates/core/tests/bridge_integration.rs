//! BridgeServer 本地 HTTP 接收服务集成测试

use idm_core::{BridgeServer, NewDownloadRequest};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_bridge_health_and_download_dispatch() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let server = BridgeServer::start("127.0.0.1:0", Some("secret-123".into()), tx)
        .await
        .unwrap();
    let base = format!("http://{}", server.local_addr());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    // 1. 健康检查接口
    let health = client.get(format!("{base}/api/health")).send().await.unwrap();
    assert_eq!(health.status(), 200);

    // 2. 缺少 Token 应返回 401
    let payload = NewDownloadRequest {
        url: "https://example.com/demo.zip".into(),
        filename: Some("demo.zip".into()),
        save_dir: None,
        headers: vec!["Referer: https://example.com".into()],
        auto_start: true,
    };
    let unauth = client
        .post(format!("{base}/api/downloads"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), 401);

    // 3. 携带正确 Token 应返回 202 并成功投递到通道
    let ok = client
        .post(format!("{base}/api/downloads"))
        .header("X-Idm-Token", "secret-123")
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 202);

    let received = rx.recv().await.unwrap();
    assert_eq!(received, payload);

    // 4. 模拟 TCP 拆包（Header 与 Body 分两次到达），验证服务端能完整拼包
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut raw_stream = tokio::net::TcpStream::connect(server.local_addr())
        .await
        .unwrap();
    let body_json = serde_json::to_string(&payload).unwrap();
    let header_part = format!(
        "POST /api/downloads HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Idm-Token: secret-123\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body_json.len()
    );
    raw_stream.write_all(header_part.as_bytes()).await.unwrap();
    raw_stream.flush().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    raw_stream.write_all(body_json.as_bytes()).await.unwrap();
    raw_stream.flush().await.unwrap();

    let mut resp_buf = vec![0u8; 1024];
    let n = raw_stream.read(&mut resp_buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
    assert!(resp_str.starts_with("HTTP/1.1 202 Accepted"));

    let split_received = rx.recv().await.unwrap();
    assert_eq!(split_received, payload);

    server.shutdown();
}
