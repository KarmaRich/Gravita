<div align="center">
  <img src="./resources/gravita.png" width="128" height="128" alt="Gravita Logo" />
  <h1>Gravita (引力下载器)</h1>
  <p><strong>基于 Rust + GPUI + Aria2 构建的高性能多线程桌面下载器与浏览器接管套件</strong></p>
</div>

---

## 一、 项目简介

**Gravita** 是一款使用 **Rust** 与 **GPUI (`gpui-kit`)** 构建的原生桌面下载管理器。它深度复刻了经典 IDM (Internet Download Manager) 的高效表格工作流，同时内置并托管全能下载引擎 **`aria2c`**，支持 **HTTP / HTTPS 多线程分片加速、断点续传、过期换链续传、BitTorrent / 磁力链 (`magnet:?`)** 以及 **Chrome / Edge 浏览器下载自动接管与网页视频悬浮嗅探**。

---

## 二、 核心特性

### 1. 桌面客户端 (`crates/desktop` & `crates/core`)
- **经典三栏布局**：顶栏大图标操作区 + 左侧可折叠分类/状态导航树（覆盖 6 大文件分类与 5 种生命周期状态）+ 右侧可滚动高速任务列表 + 底部实时状态栏。
- **底部任务检查器与 32 槽动态分片进度图**：
  - 选中任意任务即可实时查看 `aria2` 十六进制分片位图 (`bitfield`) 渲染而成的 **32 格蓝色分片区块进度条**、实时并发连接数与保存路径。
  - 提供一键 **「▶ 打开文件」**（或双击已完成任务行直接打开）、**「📁 定位文件」**（调用 `explorer /select` 高亮定位）、**「🔗 复制链接」**、**「↻ 刷新链接/重下」**（直链或 Cookie 过期时替换旧 GID 并无缝接续原文件进度）、**「🗑 彻底删除(含本地文件)」**。
- **远程文件元数据预检 (`probe.rs`)**：
  - 新建任务、剪贴板自动识别或浏览器推送下载时，自动发送带 `Range: bytes=0-0`（透传 Cookie / Referer / 代理）的预检请求。
  - 自动跟随 302 重定向、解析 `Content-Disposition`（含 RFC 5987 `filename*=UTF-8''`）获取服务器真实文件名、探测文件准确大小、判断多线程断点续传支持能力，并自动映射到对应的分类子目录（如 `Downloads/Video`、`Downloads/Compressed`）。
- **批量下载与多重通配符展开 (`batch.rs`)**：
  - 支持多条链接（换行、逗号 `,`、分号 `;` 分隔）批量导入。
  - 支持 IDM 经典数字范围（自动保留前导零宽度，如 `[01-20]`）、字母范围（如 `[a-d]`）以及多重笛卡尔积组合通配符（如 `https://example.com/s[1-2]/ep_[01-05]_[a-b].mp4`）实时预览与展开。
- **双轨持久化与引擎热重启 (`engine.rs`)**：
  - **历史记录永久保存**：`~/.idm-aria2-rs/tasks.json` 与 `aria2.session` 双轨合并，软件重启后已完成、已暂停与出错任务及其原始 URL / 请求头完整保留。
  - **5 大分类设置与引擎热重启**：涵盖常规保存、连接限速、HTTP/代理、BT/磁力、引擎会话共 30+ 项参数；普通参数通过 `aria2.changeGlobalOption` 毫秒级热更新，带 `*` 的启动级参数（磁盘缓存、预分配、证书校验、RPC Secret、DHT 等）修改保存后**自动在后台热重启内置 `aria2c` 子进程立即生效**。
- **纯原生 Win32 系统集成 (`system.rs` & `tray.rs`)**：
  - **零外部 Shell 进程调用**：通过 FFI 直接调用 `shell32.dll` (`Shell_NotifyIconW`) 弹出原生完成/失败气泡通知，调用 `advapi32.dll` (`RegSetValueExW`) 管理 Windows 开机自启，绝不拉起隐藏 `powershell.exe` 或 `reg.exe`，杜绝杀毒软件/HIPS 误报拦截。
  - **系统托盘驻留与优雅退出**：点击主窗口关闭按钮默认隐藏至系统托盘；通过托盘「退出」时自动触发 `aria2.saveSession` -> `aria2.shutdown` -> 保存 `tasks.json` -> 回收子进程，绝无孤儿进程残留。

### 2. 浏览器扩展 (`extension/` Manifest V3)
- **智能下载接管**：监听 `chrome.downloads.onDeterminingFilename`（兼容 GitHub Release / 网盘 302 重定向直链），自动读取当前站点 `chrome.cookies` 并连同 `Referer`、`User-Agent` 一并推送至桌面端 `BridgeServer`（内置完整 TCP 拼包状态机），唤起桌面端前置弹窗确认。
- **网页视频右上角悬浮下载条 (`content.js`)**：自动侦测页面中的 `<video>` 直链媒体，鼠标悬停时在视频右上角浮现 **「⬇ 使用 Gravita 下载该视频」** 快捷按钮。
- **右键菜单与划词/整页批量提取**：支持单个链接/图片/音视频右键下载，以及框选页面局部区域或整页右键 **「使用 Gravita 下载当前选中/页面全部可下载链接」**。
- **自定义过滤规则 (`popup.html`)**：支持按最小文件大小阈值（MB）、扩展名白名单、排除域名黑名单过滤拦截。

---

## 三、 项目目录结构

```text
Gravita/
├── crates/
│   ├── aria2-rpc/          # Aria2 JSON-RPC 2.0 异步客户端与子进程静默托管库
│   ├── core/               # 核心领域模型、远程 URL 预检、多重通配符展开、TCP 桥接服务与持久化
│   └── desktop/            # 基于 GPUI + gpui-kit 的桌面客户端 UI、引擎调度器与原生 Win32 集成
├── extension/              # Chrome / Edge 浏览器扩展 (Manifest V3)
│   ├── icons/              # 16x16 / 48x48 / 128x128 扩展图标
│   ├── src/                # background.js / content.js / filter.js / bridge.js / popup.js
│   ├── manifest.json
│   └── popup.html
├── resources/
│   ├── bin/aria2c.exe      # 内置免配置 aria2c 二进制引擎
│   ├── gravita.ico         # 编译期嵌入 Windows .exe 的多尺寸高清图标
│   ├── gravita.png         # 256x256 透明主图标
│   └── tray_32x32.rgba     # 内嵌至二进制的 32x32 抗锯齿托盘图标位图
├── ARIA2_OPTIONS.md        # Aria2 核心参数与 GUI 映射设计文档
├── FEATURES.md             # 功能规划与完成度清单
└── PROGRESS.md             # 阶段开发与验收进度追踪
```

---

## 四、 快速开始

### 1. 编译与运行桌面端

确保已安装 **Rust (Edition 2024 / 1.85+)** 工具链：

```powershell
# 运行桌面客户端（自动嵌入 Gravita 图标并静默拉起 resources/bin/aria2c.exe）
cargo run -p idm-desktop --release
```

### 2. 安装浏览器扩展

1. 打开 Chrome 或 Edge 浏览器，访问 `chrome://extensions/`（或 `edge://extensions/`）。
2. 开启右上角的 **「开发者模式」**。
3. 点击左上角 **「加载已解压的扩展程序」**，选择本项目下的 `extension` 目录即可。

### 3. 一键编译优化与打包分发 (Release Portable)

项目已配置 `[profile.release]` 生产级编译优化（`opt-level = 3`、`lto = "thin"`、`codegen-units = 1`、`strip = true`、`panic = "abort"` 以及 Windows 纯 GUI 子系统无黑框启动）。运行内置打包脚本即可一键生成开箱即用的 Windows 便携版发行包：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package.ps1
```

打包完成后将在 `dist/` 目录下生成：
- `dist/Gravita-v0.1.0-win64/Gravita.exe`：内嵌高清图标、无控制台黑框的独立桌面主程序。
- `dist/Gravita-v0.1.0-win64/resources/bin/aria2c.exe`：随附的免配置下载引擎。
- `dist/Gravita-v0.1.0-win64/extension/`（及 `Gravita-Browser-Extension.zip`）：剔除测试文件后的生产版浏览器扩展。
- `dist/Gravita-v0.1.0-win64.zip`：可直接发给用户解压即用的完整便携包。

### 4. 运行自动化测试

```powershell
# 运行 Rust 全工作区单元测试与集成测试（共 27 项测试）
cargo test --workspace

# 运行浏览器扩展过滤规则与通信桥接测试（共 6 项测试）
node --test extension/src/filter.test.js
```

---

## 五、 配置文件与数据存储位置

应用运行期间的所有持久化文件均存放在用户主目录的 `.idm-aria2-rs` 文件夹下（Windows 下为 `C:\Users\<用户名>\.idm-aria2-rs\`）：
- `settings.json`：桌面端全部配置项持久化文件。
- `tasks.json`：历史下载任务（含已完成/已暂停/出错任务、原始 URL 与请求头）持久化仓库。
- `aria2.session`：Aria2 引擎未完成任务与 BT/磁力分片进度会话文件。
