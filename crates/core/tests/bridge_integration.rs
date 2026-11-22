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

    server.shutdown();
}
