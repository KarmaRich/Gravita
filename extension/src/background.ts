/**
 * 浏览器插件后台 Service Worker 骨架
 * 负责监听下载事件与右键菜单，并将符合条件的任务转发给桌面端本地 RPC 服务
 */

import { DEFAULT_INTERCEPT_RULE, shouldInterceptDownload } from "./filter.ts";

/** 默认桌面端本地 RPC 接收地址 */
export const DESKTOP_RPC_ENDPOINT = "http://127.0.0.1:16800/api/downloads";

/**
 * 评估下载项是否需要接管（后续阶段一将接入 chrome.downloads.onCreated 与 fetch 转发）
 */
export function evaluateDownloadItem(url: string, filename: string, fileSize?: number): boolean {
  return shouldInterceptDownload({ url, filename, fileSize }, DEFAULT_INTERCEPT_RULE);
}
