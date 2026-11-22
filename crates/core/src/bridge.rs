//! 本地 HTTP 接收服务：供浏览器插件检测桌面端在线状态并推送拦截的下载任务

use crate::task::NewDownloadRequest;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// 本地插件桥接服务句柄
pub struct BridgeServer {
    local_addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl BridgeServer {
    /// 绑定指定地址并启动本地 HTTP 接收服务，收到的下载请求会通过 `tx` 转发给桌面端
    pub async fn start(
        bind_addr: &str,
        expected_token: Option<String>,
        tx: mpsc::UnboundedSender<NewDownloadRequest>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let token = expected_token.clone();
                let sender = tx.clone();
                tokio::spawn(async move {
                    let _ = handle_connection(stream, token.as_deref(), sender).await;
                });
            }
        });

        Ok(Self { local_addr, handle })
    }

    /// 返回实际监听地址（传入端口 0 时可用于获取系统分配的随机端口）
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// 停止本地接收服务
    pub fn shutdown(self) {
        self.handle.abort();
    }
}

/// 完整读取 HTTP 请求头与 Content-Length 指定的请求体（处理 TCP 拆包/半包，限制最大 1MB 防止 OOM）
async fn read_full_http_request(stream: &mut TcpStream) -> std::io::Result<Option<(String, String)>> {
    const MAX_HEADER_BYTES: usize = 64 * 1024;
    const MAX_BODY_BYTES: usize = 1024 * 1024;

    let mut buf = Vec::with_capacity(8192);
    let mut temp = [0u8; 4096];
    let header_end;

    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "HTTP header 尚未读取完整即断开",
            ));
        }
        buf.extend_from_slice(&temp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos;
            break;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP header 超过 64KB 限制",
            ));
        }
    }

    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let content_length = head
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    if content_length > MAX_BODY_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "HTTP body 超过 1MB 限制",
        ));
    }

    let body_start = header_end + 4;
    let total_needed = body_start + content_length;
    while buf.len() < total_needed {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&temp[..n]);
    }

    let body_end = total_needed.min(buf.len());
    let body = String::from_utf8_lossy(&buf[body_start..body_end]).into_owned();
    Ok(Some((head, body)))
}

/// 处理单个本地 HTTP 请求
async fn handle_connection(
    mut stream: TcpStream,
    expected_token: Option<&str>,
    tx: mpsc::UnboundedSender<NewDownloadRequest>,
) -> std::io::Result<()> {
    let Some((head, body)) = read_full_http_request(&mut stream).await? else {
        return Ok(());
    };

    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut req_parts = request_line.split_whitespace();
    let method = req_parts.next().unwrap_or("");
    let path = req_parts.next().unwrap_or("");

    // 提取请求头中的 X-Idm-Token（不区分大小写）
    let provided_token = lines.find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if k.trim().eq_ignore_ascii_case("x-idm-token") {
            Some(v.trim())
        } else {
            None
        }
    });

    // CORS 预检请求直接放行
    if method == "OPTIONS" {
        return write_json_response(&mut stream, "204 No Content", "").await;
    }

    match (method, path) {
        ("GET", "/api/health") => {
            write_json_response(
                &mut stream,
                "200 OK",
                r#"{"status":"ok","service":"idm-aria2-rs"}"#,
            )
            .await
        }
        ("POST", "/api/downloads") => {
            if let Some(secret) = expected_token.filter(|s| !s.is_empty()) {
                if provided_token != Some(secret) {
                    return write_json_response(
                        &mut stream,
                        "401 Unauthorized",
                        r#"{"error":"invalid token"}"#,
                    )
                    .await;
                }
            }

            match serde_json::from_str::<NewDownloadRequest>(&body) {
                Ok(req) => {
                    let _ = tx.send(req);
                    write_json_response(&mut stream, "202 Accepted", r#"{"accepted":true}"#).await
                }
                Err(_) => {
                    write_json_response(
                        &mut stream,
                        "400 Bad Request",
                        r#"{"error":"invalid download payload"}"#,
                    )
                    .await
                }
            }
        }
        _ => write_json_response(&mut stream, "404 Not Found", r#"{"error":"not found"}"#).await,
    }
}

async fn write_json_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Content-Type, X-Idm-Token\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await
}
