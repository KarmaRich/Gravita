# aria2 (v1.37.0) 完整配置项梳理与桌面端选项映射

> 本文档完整梳理 `aria2c` 的核心配置项，明确每个参数的**生效方式**（热更新 / 需重启 / 单任务）与**桌面端选项分类**，避免配置遗漏或将不支持热更新的参数错误下发给 `changeGlobalOption`。

### 生效方式图例
- **热更新**：可通过 `aria2.changeGlobalOption` 在运行中实时生效
- **需重启**：仅在启动 `aria2c` 命令行/配置文件时生效（修改后需重启内置 `aria2c`）
- **单任务**：可在 `aria2.addUri` 或 `aria2.changeOption` 中针对单个任务覆盖

---

## 1. 常规与文件保存 (General & Files)

| aria2 参数 | 类型 / 可选值 | 默认值 | 生效方式 | 中文说明 |
| :--- | :--- | :--- | :--- | :--- |
| `dir` | 路径 | 当前目录 | 热更新 / 单任务 | 默认下载保存目录 |
| `continue` | `true` / `false` | `false`（建议 `true`） | 热更新 / 单任务 | 启用断点续传 |
| `allow-overwrite` | `true` / `false` | `false` | 热更新 / 单任务 | 是否允许覆盖已存在同名文件 |
| `auto-file-renaming` | `true` / `false` | `true` | 热更新 / 单任务 | 同名文件自动重命名（如 `file.1.zip`） |
| `file-allocation` | `none` / `prealloc` / `trunc` / `falloc` | `prealloc`（SSD 建议 `none`） | 需重启 / 单任务 | 磁盘空间预分配方式 |
| `disk-cache` | 大小（如 `16M`、`64M`） | `16M` | 需重启 | 内存磁盘缓存大小，减少磁盘随机写 |
| `always-resume` | `true` / `false` | `true` | 热更新 / 单任务 | 总是尝试断点续传 |
| `max-resume-failure-tries` | 整数 | `0` | 热更新 / 单任务 | 断点续传失败次数上限（0 为不限） |
| `remote-time` | `true` / `false` | `false` | 热更新 / 单任务 | 尝试将本地文件修改时间设为服务器时间 |
| `check-integrity` | `true` / `false` | `false` | 热更新 / 单任务 | 下载后校验文件哈希（支持 BT/Metalink） |
| `remove-control-file` | `true` / `false` | `false` | 热更新 / 单任务 | 开始前删除 `.aria2` 控制文件 |

---

## 2. 连接、分片与并发 (Connection & Split)

| aria2 参数 | 类型 / 可选值 | 默认值 | 生效方式 | 中文说明 |
| :--- | :--- | :--- | :--- | :--- |
| `max-concurrent-downloads` | 整数 (`1~N`) | `5` | 热更新 | 最大同时下载任务数 |
| `split` | 整数 (`1~64`) | `5`（推荐 `16`） | 热更新 / 单任务 | 单任务分片数 |
| `max-connection-per-server` | 整数 (`1~16`) | `1`（推荐 `16`） | 热更新 / 单任务 | 单个服务器最大并发连接数（aria2 上限 16） |
| `min-split-size` | 大小 (`1M~1024M`) | `20M`（推荐 `1M`） | 热更新 / 单任务 | 触发分片的最小文件大小 |
| `lowest-speed-limit` | 速率（如 `0`、`50K`） | `0` | 热更新 / 单任务 | 低于此速度自动断开重连（0 为禁用） |
| `max-overall-download-limit` | 速率（如 `0`、`5M`） | `0` | 热更新 | 全局最大下载速度（0 为不限速） |
| `max-download-limit` | 速率（如 `0`、`2M`） | `0` | 热更新 / 单任务 | 单任务最大下载速度 |
| `max-overall-upload-limit` | 速率（如 `0`、`1M`） | `0` | 热更新 | 全局最大上传速度（BT 场景） |
| `max-upload-limit` | 速率（如 `0`、`500K`） | `0` | 热更新 / 单任务 | 单任务最大上传速度 |
| `optimize-concurrent-downloads` | `true` / `false` | `false` | 热更新 | 根据带宽自动动态调整并发数 |
| `stream-piece-selector` | `default` / `inorder` / `random` / `geom` | `default` | 热更新 / 单任务 | 分片选择算法（`geom` 对大文件更友好） |

---

## 3. HTTP / HTTPS 与代理 (HTTP & Proxy)

| aria2 参数 | 类型 / 可选值 | 默认值 | 生效方式 | 中文说明 |
| :--- | :--- | :--- | :--- | :--- |
| `all-proxy` | URL（如 `http://127.0.0.1:7890`） | 空 | 热更新 / 单任务 | 全部协议统一代理地址（留空直连） |
| `all-proxy-user` / `all-proxy-passwd` | 字符串 | 空 | 热更新 / 单任务 | 代理服务器认证用户名与密码 |
| `no-proxy` | 逗号分隔域名/IP | 空 | 热更新 / 单任务 | 不走代理的域名白名单（如 `localhost,127.0.0.1`） |
| `user-agent` | 字符串 | `aria2/1.37.0` | 热更新 / 单任务 | HTTP 请求 User-Agent |
| `referer` | URL 或 `*` | 空 | 单任务 | HTTP Referer（`*` 表示使用下载链接本身） |
| `header` | 字符串数组 | 空 | 单任务 | 自定义 HTTP 请求头（如 `Cookie: ...`、`Authorization: ...`） |
| `http-accept-gzip` | `true` / `false` | `false` | 热更新 / 单任务 | 允许 HTTP Gzip 压缩传输 |
| `check-certificate` | `true` / `false` | `true` | 需重启 | 校验 HTTPS 服务器 TLS 证书 |
| `connect-timeout` | 秒 | `60` | 热更新 / 单任务 | 建立 TCP 连接超时时间 |
| `timeout` | 秒 | `60` | 热更新 / 单任务 | 数据读写超时时间 |
| `max-tries` | 整数 | `5` | 热更新 / 单任务 | 最大重试次数（`0` 为无限重试） |
| `retry-wait` | 秒 | `0`（推荐 `3`） | 热更新 / 单任务 | 重试前等待间隔秒数 |
| `reuse-uri` | `true` / `false` | `true` | 热更新 / 单任务 | 复用已使用的镜像 URI |
| `enable-http-keep-alive` | `true` / `false` | `true` | 热更新 / 单任务 | 启用 HTTP Keep-Alive 长连接 |
| `enable-http-pipelining` | `true` / `false` | `false` | 热更新 / 单任务 | 启用 HTTP Pipelining |

---

## 4. BitTorrent / 磁力链接 (BT & DHT)

| aria2 参数 | 类型 / 可选值 | 默认值 | 生效方式 | 中文说明 |
| :--- | :--- | :--- | :--- | :--- |
| `listen-port` | 端口或范围（如 `6881-6999`） | `6881-6999` | 需重启 | BT TCP 监听端口 |
| `enable-dht` | `true` / `false` | `true` | 需重启 | 启用 IPv4 DHT 网络 |
| `dht-listen-port` | 端口或范围 | `6881-6999` | 需重启 | DHT UDP 监听端口 |
| `enable-peer-exchange` | `true` / `false` | `true` | 需重启 | 启用 PEX 节点交换 |
| `bt-enable-lpd` | `true` / `false` | `false` | 需重启 | 启用本地节点发现 (LPD) |
| `bt-max-peers` | 整数 | `55` | 热更新 / 单任务 | 单个 BT 任务最大连接节点数（`0` 不限） |
| `seed-ratio` | 浮点数（如 `1.0`、`0.0`） | `1.0` | 热更新 / 单任务 | 分享率达到后停止做种（`0.0` 不做种） |
| `seed-time` | 分钟（如 `0`、`60`） | 空 | 热更新 / 单任务 | 做种时间（分钟，`0` 表示下载完即停） |
| `follow-torrent` | `true` / `false` / `mem` | `true` | 热更新 / 单任务 | 下载 `.torrent` 后自动开始 BT 任务 |
| `bt-tracker` | 逗号分隔 Tracker URL | 空 | 热更新 / 单任务 | 附加 BT Tracker 服务器列表 |
| `bt-save-metadata` | `true` / `false` | `false` | 热更新 / 单任务 | 保存磁力链接获取到的 `.torrent` 元数据 |

---

## 5. RPC、会话持久化与日志 (RPC & Session)

| aria2 参数 | 类型 / 可选值 | 默认值 | 生效方式 | 中文说明 |
| :--- | :--- | :--- | :--- | :--- |
| `enable-rpc` | `true` / `false` | `true`（内置强制开启） | 需重启 | 启用 JSON-RPC 服务 |
| `rpc-listen-port` | 端口 | `16801`（内置专属） | 需重启 | RPC 监听端口 |
| `rpc-secret` | 字符串 | 空 | 需重启 | RPC 鉴权 Token |
| `rpc-allow-origin-all` | `true` / `false` | `false` | 需重启 | 允许跨域 RPC 访问 |
| `save-session` | 文件路径 | `~/.gravita/aria2.session` | 需重启 | 退出时保存未完成任务列表（重启自动恢复） |
| `input-file` | 文件路径 | 同 `save-session` | 需重启 | 启动时从会话文件恢复未完成任务 |
| `save-session-interval` | 秒 | `60` | 需重启 | 定时保存会话间隔 |
| `disable-ipv6` | `true` / `false` | `false` | 需重启 | 禁用 IPv6（部分网络环境下可减少超时） |
| `log-level` | `debug` / `info` / `notice` / `warn` / `error` | `warn` | 需重启 | 日志级别 |

---

## 6. 桌面端「选项」界面分类建议

基于以上完整梳理，桌面端「选项设置」弹窗建议分为 **5 个清晰标签页**，并明确标注需重启引擎的项（保存时若修改了「需重启」项，自动重启内置 `aria2c`）：

1. **常规与保存**：`dir`、`continue`、`allow-overwrite`、`auto-file-renaming`、`file-allocation`、`disk-cache`、`remote-time`
2. **连接与限速**：`max-concurrent-downloads`、`split`、`max-connection-per-server`、`min-split-size`、`max-overall-download-limit`、`max-overall-upload-limit`、`lowest-speed-limit`
3. **HTTP 与代理**：`all-proxy`、`no-proxy`、`user-agent`、`connect-timeout`、`timeout`、`max-tries`、`retry-wait`、`check-certificate`、`http-accept-gzip`
4. **BT 与磁力**：`enable-dht`、`enable-peer-exchange`、`bt-max-peers`、`seed-ratio`、`seed-time`、`bt-save-metadata`、`bt-tracker`
5. **引擎与会话**：`rpc-listen-port`、`rpc-secret`、自动保存会话 (`save-session` / `save-session-interval`)、`disable-ipv6`
