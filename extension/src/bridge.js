/**
 * 与 IDM-Aria2-RS 桌面端本地 BridgeServer 的 HTTP 通信模块
 */

/**
 * 检测桌面端本地接收服务是否在线
 */
export async function checkDesktopHealth(endpoint) {
  const base = endpoint.replace(/\/+$/, "");
  try {
    const resp = await fetch(`${base}/api/health`, { method: "GET" });
    return resp.ok;
  } catch {
    return false;
  }
}

/**
 * 将拦截的下载任务推送给桌面端
 * @returns {Promise<boolean>} 桌面端是否成功接收
 */
export async function dispatchToDesktop(endpoint, token, payload) {
  const base = endpoint.replace(/\/+$/, "");
  const headers = { "Content-Type": "application/json" };
  if (token) {
    headers["X-Idm-Token"] = token;
  }

  try {
    const resp = await fetch(`${base}/api/downloads`, {
      method: "POST",
      headers,
      body: JSON.stringify(payload),
    });
    return resp.status === 202 || resp.ok;
  } catch {
    return false;
  }
}
