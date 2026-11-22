/**
 * 浏览器下载拦截过滤规则与请求头组装模块
 */

export const DEFAULT_CONFIG = {
  /** 是否开启自动下载拦截 */
  enabled: true,
  /** 桌面端本地 BridgeServer 地址 */
  desktopEndpoint: "http://127.0.0.1:16800",
  /** 可选鉴权 Token（对应桌面端 bridge_token） */
  token: "",
  /** 最小拦截文件大小（字节），默认 1 MB */
  minFileSizeBytes: 1024 * 1024,
  /** 允许拦截的扩展名列表（小写，不含点号） */
  extensions: [
    "zip", "rar", "7z", "tar", "gz", "xz",
    "exe", "msi", "dmg", "apk", "iso",
    "mp4", "mkv", "avi", "mov", "mp3", "flac", "pdf",
  ],
  /** 不拦截的域名黑名单 */
  excludedDomains: [],
};

/**
 * 判断某个下载请求是否应当被插件拦截并转发给桌面端
 */
export function shouldInterceptDownload(candidate, config = DEFAULT_CONFIG) {
  if (!config.enabled) {
    return false;
  }

  let parsedUrl;
  try {
    parsedUrl = new URL(candidate.url);
  } catch {
    return false;
  }
  if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
    return false;
  }

  const host = parsedUrl.hostname.toLowerCase();
  if (
    (config.excludedDomains || []).some(
      (d) => d && (host === d.toLowerCase() || host.endsWith(`.${d.toLowerCase()}`))
    )
  ) {
    return false;
  }

  if (
    typeof candidate.fileSize === "number" &&
    candidate.fileSize >= 0 &&
    candidate.fileSize < config.minFileSizeBytes
  ) {
    return false;
  }

  const name = candidate.filename || parsedUrl.pathname.split("/").pop() || "";
  const dotIdx = name.lastIndexOf(".");
  if (dotIdx <= 0 || dotIdx === name.length - 1) {
    return false;
  }
  const ext = name.slice(dotIdx + 1).toLowerCase();
  return (config.extensions || []).includes(ext);
}

/**
 * 组装透传给桌面端 / aria2 的 HTTP 请求头数组
 */
export function buildDownloadHeaders({ referer, userAgent, cookie } = {}) {
  const headers = [];
  if (referer) headers.push(`Referer: ${referer}`);
  if (userAgent) headers.push(`User-Agent: ${userAgent}`);
  if (cookie) headers.push(`Cookie: ${cookie}`);
  return headers;
}
