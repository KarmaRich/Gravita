# Gravita (原 idm-aria2-rs) 项目开发进度追踪

> 本文档实时记录 **Gravita (引力下载器)** 各阶段任务的开发、测试与验收状态。

## 阶段零：项目骨架搭建
- [x] 梳理完整功能清单 ([FEATURES.md](./FEATURES.md))
- [x] 初始化 Git 仓库与 Rust Workspace (`aria2-rpc` / `core` / `desktop`)
- [x] 初始化浏览器插件骨架 (`extension/` Manifest V3)
- [x] 编写骨架阶段基础单元测试并通过验收
- [x] 提交首个骨架 commit 至本地 Git 仓库

## 阶段一：核心架构与 MVP (P0)
- [x] `aria2-rpc`：`aria2c` 进程静默托管与完整 JSON-RPC 客户端（添加/暂停/恢复/删除/状态查询/保存会话/优雅停机）
- [x] `core`：任务管理服务、自动文件分类子目录映射、`tasks.json` + `aria2.session` 双轨持久化与完整 TCP 拼包本地 HTTP 接收服务
- [x] `desktop`：基于 `GPUI + gpui-kit` 的主界面（工具栏、可折叠分类与状态树、可滚动任务列表、新建下载弹窗、5 大分类设置页）
- [x] `extension`：浏览器下载拦截（兼容 302 重定向与 `Content-Disposition`）、Cookie/Referer/UA 透传、右键菜单、过滤规则、Popup 状态面板

## 阶段二：完善 IDM 核心体验 (P1)
- [x] 剪贴板下载链接自动识别与回填
- [x] 远程 URL 预检探测（自动发送 `Range: bytes=0-0` 探测真实文件名、准确文件大小与多线程续传支持）
- [x] 批量下载与多重通配符笛卡尔积展开（支持 `[01-10]` 前导零数字范围、`[a-d]` 字母范围及换行/逗号/分号分隔）
- [x] 全局限速 / 单任务限速、HTTP/SOCKS 代理设置、带 `*` 引擎启动级参数修改后自动热重启内置 `aria2c`
- [x] 底部任务详情检查器、32 槽动态分片位图 (`bitfield`) 可视化、双击直接打开文件、`explorer /select` 定位文件、过期链接刷新无缝续传、彻底删除本地文件
- [x] 纯原生 Win32 系统托盘、零 PowerShell 原生通知 (`Shell_NotifyIconW`)、零 `reg.exe` 原生注册表开机自启 (`advapi32`)

## 阶段三：进阶能力与品牌化 (P2)
- [x] BT / 磁力链接 (`magnet:?`)、DHT、PEX、做种分享率与 Tracker 配置支持
- [x] 网页直链 `<video>` 视频右上角悬浮下载条 (`content.js`) 与划词/整页批量提取可下载链接
- [x] **Gravita** 品牌升级与多尺寸高清图标设计（`build.rs` + `embed-resource` 编译期嵌入 `.exe` 图标、高清抗锯齿托盘图标、Chrome 扩展多尺寸图标）
- [ ] 多队列计划任务调度器（定时开始/停止、完成后自动关机/休眠）
- [ ] 桌面迷你网速悬浮窗与 HLS (`.m3u8`) 分片流解密合并
