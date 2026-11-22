/**
 * Manifest V3 后台 Service Worker
 * 负责监听浏览器下载事件、右键菜单，并将任务（含 Cookie/Referer/UA）转发至桌面端
 */

import { DEFAULT_CONFIG, buildDownloadHeaders, shouldInterceptDownload } from "./filter.js";
import { dispatchToDesktop } from "./bridge.js";

const MENU_ID = "idm-aria2-download-link";

async function loadConfig() {
  const stored = await chrome.storage.local.get("config");
  return { ...DEFAULT_CONFIG, ...(stored.config || {}) };
}

async function getCookieHeader(url) {
  try {
    const cookies = await chrome.cookies.getAll({ url });
    return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch {
    return "";
  }
}

// 插件安装/更新时注册右键菜单
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: MENU_ID,
      title: "使用 IDM-Aria2-RS 下载",
      contexts: ["link", "image", "video", "audio"],
    });
  });
});

// 右键菜单点击：直接转发给桌面端
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== MENU_ID) return;
  const url = info.linkUrl || info.srcUrl;
  if (!url) return;

  const config = await loadConfig();
  const cookie = await getCookieHeader(url);
  const headers = buildDownloadHeaders({
    referer: tab?.url || info.pageUrl,
    userAgent: navigator.userAgent,
    cookie,
  });

  await dispatchToDesktop(config.desktopEndpoint, config.token, {
    url,
    filename: null,
    save_dir: null,
    headers,
    auto_start: true,
  });
});

// 监听 onDeterminingFilename：此时浏览器已完成 302 重定向并解析出 Content-Disposition 真实文件名（兼容 GitHub Release / 网盘直链）
chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
  const url = item.finalUrl || item.url;
  if (!url) {
    suggest();
    return;
  }

  (async () => {
    const config = await loadConfig();
    const candidate = {
      url,
      filename: item.filename || "",
      fileSize: typeof item.fileSize === "number" ? item.fileSize : -1,
    };

    if (!shouldInterceptDownload(candidate, config)) {
      suggest();
      return;
    }

    const cookie = await getCookieHeader(url);
    const headers = buildDownloadHeaders({
      referer: item.referrer,
      userAgent: navigator.userAgent,
      cookie,
    });

    const accepted = await dispatchToDesktop(config.desktopEndpoint, config.token, {
      url,
      filename: item.filename ? item.filename.split(/[\\/]/).pop() : null,
      save_dir: null,
      headers,
      auto_start: true,
    });

    // 仅当桌面端确认接收后才取消浏览器原生下载，否则自动放行给浏览器
    if (accepted) {
      try {
        await chrome.downloads.cancel(item.id);
        await chrome.downloads.erase({ id: item.id });
      } catch {
        // 忽略已完成的取消异常
      }
    }
    suggest();
  })();

  // 返回 true 告知 Chrome 异步调用 suggest()
  return true;
});
