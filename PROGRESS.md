# 项目开发进度追踪

> 本文档实时记录 `idm-aria2-rs` 各阶段任务的开发、测试与验收状态。

## 阶段零：项目骨架搭建
- [x] 梳理完整功能清单 ([FEATURES.md](./FEATURES.md))
- [x] 初始化 Git 仓库与 Rust Workspace (`aria2-rpc` / `core` / `desktop`)
- [x] 初始化浏览器插件骨架 (`extension/` Manifest V3 + TypeScript)
- [x] 编写骨架阶段基础单元测试并通过验收
- [x] 提交首个骨架 commit 至本地 Git 仓库

## 阶段一：核心架构与 MVP (P0)
- [x] `aria2-rpc`：aria2c 进程托管与完整 JSON-RPC 客户端（添加/暂停/恢复/删除/状态查询）
- [x] `core`：任务管理服务、自动文件分类、SQLite 持久化与本地 HTTP RPC 接收服务
- [ ] `desktop`：基于 `GPUI + gpui-kit` 的主界面（工具栏、分类树、任务列表、新建下载弹窗、设置页）
- [ ] `extension`：浏览器下载拦截、右键菜单、过滤规则、Popup 状态面板

## 阶段二：完善 IDM 核心体验 (P1)
- [ ] 剪贴板监听、批量/通配符下载、队列与计划任务
- [ ] 全局限速/单任务限速、代理设置、任务详情与速度曲线
- [ ] 系统托盘、原生通知、开机自启

## 阶段三：进阶能力 (P2)
- [ ] BT / 磁力链接支持
- [ ] 桌面悬浮窗
- [ ] 网页媒体资源嗅探
