/**
 * IDM-Aria2-RS 网页内容注入脚本 (Content Script)
 * 功能：
 * 1. 仿 IDM 视频悬浮下载条：自动检测页面中的 <video> / <audio> 直链媒体并在右上角显示「⬇ 使用 IDM 下载该视频」浮动面板
 * 2. 仿 IDM 划词/整页批量提取下载链接：支持右键提取选中区域或整页中所有匹配后缀的下载直链
 */

const DOWNLOADABLE_EXT_RE =
  /\.(zip|rar|7z|tar|gz|xz|exe|msi|dmg|apk|iso|mp4|mkv|avi|mov|webm|mp3|flac|wav|pdf|docx?|xlsx?|pptx?)(\?.*)?$/i;

/**
 * 提取当前选中区域（若有）或整页中的所有可下载链接
 */
function collectDownloadableLinks() {
  const selection = window.getSelection();
  const urls = new Set();

  let anchors = [];
  if (selection && !selection.isCollapsed && selection.rangeCount > 0) {
    const container = document.createElement("div");
    for (let i = 0; i < selection.rangeCount; i++) {
      container.appendChild(selection.getRangeAt(i).cloneContents());
    }
    anchors = Array.from(container.querySelectorAll("a[href]"));
  } else {
    anchors = Array.from(document.querySelectorAll("a[href]"));
  }

  for (const a of anchors) {
    try {
      const resolved = new URL(a.getAttribute("href"), window.location.href).href;
      if (
        (resolved.startsWith("http://") || resolved.startsWith("https://")) &&
        DOWNLOADABLE_EXT_RE.test(resolved)
      ) {
        urls.add(resolved);
      }
    } catch {
      // 忽略非法链接
    }
  }

  return Array.from(urls).slice(0, 50);
}

/**
 * 解析媒体元素的真实播放源（排除无法直接由外部下载的 blob: 流）
 */
function resolveMediaSrc(mediaEl) {
  const candidates = [mediaEl.currentSrc, mediaEl.src];
  for (const s of mediaEl.querySelectorAll("source[src]")) {
    candidates.push(s.src);
  }
  for (const raw of candidates) {
    if (
      raw &&
      (raw.startsWith("http://") || raw.startsWith("https://")) &&
      !raw.startsWith("blob:")
    ) {
      return raw;
    }
  }
  return null;
}

/**
 * 为检测到直链的 <video> 元素挂载 IDM 风格右上角浮动下载条
 */
function attachVideoGrabber() {
  const videos = document.querySelectorAll("video");
  for (const video of videos) {
    if (video.dataset.idmBound === "true") continue;
    const src = resolveMediaSrc(video);
    if (!src) continue;

    video.dataset.idmBound = "true";

    const badge = document.createElement("button");
    badge.type = "button";
    badge.textContent = "⬇ 使用 IDM-Aria2-RS 下载该视频";
    Object.assign(badge.style, {
      position: "fixed",
      zIndex: "2147483647",
      display: "none",
      padding: "5px 10px",
      fontSize: "12px",
      fontFamily: '"Segoe UI", sans-serif',
      color: "#ffffff",
      background: "linear-gradient(180deg, #2563eb, #1d4ed8)",
      border: "1px solid #93c5fd",
      borderRadius: "4px",
      boxShadow: "0 2px 8px rgba(0,0,0,0.3)",
      cursor: "pointer",
    });

    const updatePosition = () => {
      const rect = video.getBoundingClientRect();
      if (rect.width < 120 || rect.height < 80) {
        badge.style.display = "none";
        return;
      }
      badge.style.top = `${Math.max(6, rect.top + 8)}px`;
      badge.style.left = `${Math.max(6, rect.right - 210)}px`;
    };

    video.addEventListener("mouseenter", () => {
      const current = resolveMediaSrc(video);
      if (!current) return;
      updatePosition();
      badge.style.display = "block";
    });

    badge.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const current = resolveMediaSrc(video) || src;
      chrome.runtime.sendMessage(
        {
          type: "IDM_DOWNLOAD_URLS",
          urls: [{ url: current, filename: null }],
          referer: window.location.href,
        },
        () => {
          badge.textContent = "✓ 已发送至 IDM-Aria2-RS";
          setTimeout(() => {
            badge.textContent = "⬇ 使用 IDM-Aria2-RS 下载该视频";
            badge.style.display = "none";
          }, 1800);
        }
      );
    });

    document.body.appendChild(badge);
  }
}

if (typeof window !== "undefined" && typeof document !== "undefined") {
  attachVideoGrabber();
  const observer = new MutationObserver(() => attachVideoGrabber());
  if (document.documentElement) {
    observer.observe(document.documentElement, { childList: true, subtree: true });
  }

  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg && msg.type === "IDM_COLLECT_PAGE_LINKS") {
      sendResponse({ urls: collectDownloadableLinks() });
      return true;
    }
    return false;
  });
}
