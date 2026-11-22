/**
 * 插件 Popup 面板交互脚本：展示桌面端在线状态并持久化拦截规则
 */

import { DEFAULT_CONFIG } from "./filter.js";
import { checkDesktopHealth } from "./bridge.js";

const $ = (id) => document.getElementById(id);

async function refreshHealth(endpoint) {
  const badge = $("status");
  const ok = await checkDesktopHealth(endpoint);
  badge.textContent = ok ? "● 桌面端在线" : "○ 桌面端离线";
  badge.className = `badge ${ok ? "online" : "offline"}`;
}

async function init() {
  const stored = await chrome.storage.local.get("config");
  const cfg = { ...DEFAULT_CONFIG, ...(stored.config || {}) };

  $("enabled").checked = cfg.enabled;
  $("endpoint").value = cfg.desktopEndpoint;
  $("token").value = cfg.token || "";
  $("minSizeMb").value = (cfg.minFileSizeBytes / (1024 * 1024)).toString();
  $("extensions").value = (cfg.extensions || []).join(", ");
  $("excludedDomains").value = (cfg.excludedDomains || []).join(", ");

  await refreshHealth(cfg.desktopEndpoint);

  $("saveBtn").addEventListener("click", async () => {
    const minMb = Math.max(0, Number.parseFloat($("minSizeMb").value) || 0);
    const nextConfig = {
      enabled: $("enabled").checked,
      desktopEndpoint: $("endpoint").value.trim() || DEFAULT_CONFIG.desktopEndpoint,
      token: $("token").value.trim(),
      minFileSizeBytes: Math.round(minMb * 1024 * 1024),
      extensions: $("extensions")
        .value.split(",")
        .map((s) => s.trim().toLowerCase().replace(/^\./, ""))
        .filter(Boolean),
      excludedDomains: $("excludedDomains")
        .value.split(",")
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean),
    };

    await chrome.storage.local.set({ config: nextConfig });
    $("msg").textContent = "设置已保存";
    await refreshHealth(nextConfig.desktopEndpoint);
  });
}

init();
