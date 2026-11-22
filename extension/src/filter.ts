/**
 * 浏览器下载拦截过滤规则模块
 */

export interface InterceptRule {
  /** 是否开启拦截 */
  enabled: boolean;
  /** 允许拦截的扩展名列表（小写，不含点号） */
  extensions: string[];
  /** 最小拦截文件大小（字节），小于该值的文件交由浏览器原生下载 */
  minFileSizeBytes: number;
  /** 不拦截的域名黑名单 */
  excludedDomains: string[];
}

export interface DownloadCandidate {
  url: string;
  filename: string;
  /** 文件总大小，未知时为 -1 或 undefined */
  fileSize?: number;
}

export const DEFAULT_INTERCEPT_RULE: InterceptRule = {
  enabled: true,
  extensions: ["zip", "rar", "7z", "exe", "msi", "iso", "mp4", "mkv", "pdf"],
  minFileSizeBytes: 1024 * 1024, // 默认 1 MB
  excludedDomains: [],
};

/**
 * 判断某个下载请求是否应当被插件拦截并转发给桌面端
 */
export function shouldInterceptDownload(
  candidate: DownloadCandidate,
  rule: InterceptRule = DEFAULT_INTERCEPT_RULE
): boolean {
  if (!rule.enabled) {
    return false;
  }

  // 仅拦截 http/https 协议，忽略 blob: / data: 等本地链接
  let parsedUrl: URL;
  try {
    parsedUrl = new URL(candidate.url);
  } catch {
    return false;
  }
  if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
    return false;
  }

  // 域名排除列表检查
  const host = parsedUrl.hostname.toLowerCase();
  if (rule.excludedDomains.some((d) => host === d.toLowerCase() || host.endsWith(`.${d.toLowerCase()}`))) {
    return false;
  }

  // 若已知文件大小且小于阈值，则不拦截
  if (typeof candidate.fileSize === "number" && candidate.fileSize >= 0 && candidate.fileSize < rule.minFileSizeBytes) {
    return false;
  }

  // 扩展名匹配检查
  const dotIdx = candidate.filename.lastIndexOf(".");
  if (dotIdx <= 0 || dotIdx === candidate.filename.length - 1) {
    return false;
  }
  const ext = candidate.filename.slice(dotIdx + 1).toLowerCase();
  return rule.extensions.includes(ext);
}
