/**
 * Manifest V3 后台 Service Worker
 * 负责监听浏览器下载事件、右键菜单，并将任务（含 Cookie/Referer/UA）转发至桌面端
 */

import { DEFAULT_CONFIG, buildDownloadHeaders, shouldInterceptDownload } from "./filter.js";
import { dispatchToDesktop } from "./bridge.js";

const MENU_ID = "idm-aria2-download-link";
const MENU_BATCH_ID = "idm-aria2-download-all-links";

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
      title: "使用 IDM-Aria2-RS 下载此链接",
      contexts: ["link", "image", "video", "audio"],
    });
    chrome.contextMenus.create({
      id: MENU_BATCH_ID,
      title: "使用 IDM-Aria2-RS 下载当前选中/页面全部可下载链接",
      contexts: ["page", "selection"],
    });
  });
});

async function forwardSingleUrl(url, referer, filename = null) {
  const config = await loadConfig();
  const cookie = await getCookieHeader(url);
  const headers = buildDownloadHeaders({
    referer,
    userAgent: navigator.userAgent,
    cookie,
  });

  return dispatchToDesktop(config.desktopEndpoint, config.token, {
    url,
    filename,
    save_dir: null,
    headers,
    auto_start: true,
  });
}

// 接收来自网页 content_script 的视频浮动下载条或批量划词下载请求
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message !== "object") return false;

  if (message.type === "IDM_DOWNLOAD_URLS" && Array.isArray(message.urls)) {
    const referer = sender?.tab?.url || message.referer || "";
    (async () => {
      let count = 0;
      for (const item of message.urls) {
        const u = typeof item === "string" ? item : item?.url;
        const name = typeof item === "object" ? item?.filename || null : null;
        if (u && (await forwardSingleUrl(u, referer, name))) {
          count += 1;
        }
      }
      sendResponse({ ok: true, count });
    })();
    return true;
  }
  return false;
});

// 右键菜单点击：单个直链转发或唤醒页面链接批量提取
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId === MENU_ID) {
    const url = info.linkUrl || info.srcUrl;
    if (!url) return;
    await forwardSingleUrl(url, tab?.url || info.pageUrl);
    return;
  }

  if (info.menuItemId === MENU_BATCH_ID && tab?.id) {
    try {
      const resp = await chrome.tabs.sendMessage(tab.id, {
        type: "IDM_COLLECT_PAGE_LINKS",
      });
      if (resp && Array.isArray(resp.urls)) {
        for (const u of resp.urls) {
          await forwardSingleUrl(u, tab.url || info.pageUrl);
        }
      }
    } catch {
      // 忽略受保护的 chrome:// 页面异常
    }
  }
});

// 监听 onDeterminingFilename：此时浏览器已完成 302 重定向并解析出 Content-Disposition 真实文件名（兼容 GitHub Release / 网盘直链）
chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
  let suggested = false;
  const safeSuggest = () => {
    if (suggested) return;
    suggested = true;
    try {
      suggest();
    } catch {
      // 忽略下载已被取消后的回调异常
    }
  };

  const url = item.finalUrl || item.url;
  if (!url) {
    safeSuggest();
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
      safeSuggest();
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

    // 重要：必须先调用 suggest() 结束 Chrome 的文件名决策阶段，再执行 cancel/erase，
    // 否则在 cancel 之后调用 suggest() 会触发 "Unchecked runtime.lastError: Invalid downloadId"
    safeSuggest();

    if (accepted) {
      chrome.downloads.cancel(item.id, () => {
        void chrome.runtime.lastError;
        chrome.downloads.erase({ id: item.id }, () => {
          void chrome.runtime.lastError;
        });
      });
    }
  })();

  // 返回 true 告知 Chrome 异步调用 suggest()
  return true;
});
