# CLAUDE.md

本文件为 Claude Code（及其他 AI 协作会话）提供项目上下文，便于快速理解项目结构、实现原理与开发约定。

# 语言要求
始终使用中文回复用户，无论用户使用什么语言提问。

## 文档规范

README.md、CLAUDE.md、其他 .md 文档及代码注释遵守：

- 只客观描述功能、现状、使用方法与约束，不强调「做了哪些修改/优化」、不解释「为什么这么改」（代码注释、commit message、对话回复同理）。
- 篇幅精简，不写长篇大论：一句话能说清楚的不写第二句，不铺垫背景、不重复自证；docstring/注释长度应与同一文件里其它同类函数/条目保持一致的密度。
- 使用中文编写，代码块、表格、列表等元素之间留空行，合理缩进，避免网页渲染时出现问题。
- 全角中文字符与半角英文字符之间应有一个半角空格；中文与阿拉伯数字之间加不加空格皆可，但全文风格须统一。

## 项目概要

**udx710** 是运行在 **紫光展锐（Unisoc）UDX710 5G 模组**（CPE / 蜂窝路由设备）上的**设备管理系统**，为插在设备中的 SIM 卡提供 Web 管理后台。

设备本身运行紫光展锐原厂 Linux 固件（内核 4.14 aarch64），其中 `ofonod`（电话协议栈）、`sprdrild`（展锐 RIL）、`connmand`、`modem_control` 等是**原厂组件**。本项目**不实现这些底层能力，而是作为应用层，通过系统 D-Bus 调用 ofono，把蜂窝网络/短信/通话/设备管理能力封装成 HTTP API + Web 界面**。

主要功能：
- 设备/SIM/网络/信号/基站信息展示
- 移动数据、漫游、飞行模式、网络制式（4G/5G）、频段锁定、小区锁定
- 短信收发与会话管理、来电/去电/通话记录、呼叫转移、呼叫设置
- APN 管理、运营商扫描与注册、USB 网络模式切换（RNDIS/ECM/NCM）
- 系统资源监控（CPU/温度/内存）、系统重启
- OTA 在线更新、开机自定义脚本（init.sh）
- 短信转发到 Webhook（飞书等）或推送服务（PushPlus/Server酱/PushDeer/Bark/ntfy）
- AT 指令控制台、Web 终端（ttyd，独立进程）

## 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust 2021 · axum 0.8 · tokio · zbus 5（D-Bus）· rusqlite（SQLite bundled）· reqwest（rustls）· clap |
| 前端 | React 19 · TypeScript · Vite 7 · MUI 7（@mui/material, x-charts, x-data-grid）· TanStack Query · React Router 7 |
| 目标平台 | `aarch64-unknown-linux-musl`（静态链接 musl，交叉编译） |
| 包管理 | 前端用 **pnpm**（`pnpm-lock.yaml`，勿用 npm/yarn） |

## 项目结构

```
├── backend/                  # Rust 后端（编译产物名: udx710）
│   ├── src/
│   │   ├── main.rs           # 入口：启动 axum、注册全部路由、拉起后台监听任务
│   │   ├── handlers.rs       # HTTP 处理器（最大文件，全部 REST 端点在此）
│   │   ├── dbus.rs           # 对 ofono 的 D-Bus 调用封装（网络/通话/短信/SIM/APN…）
│   │   ├── models.rs         # 请求/响应结构体定义
│   │   ├── config.rs         # config.json 配置管理 + loader.sh/init.sh 开机脚本管理
│   │   ├── db.rs             # SQLite：短信(sms_messages)、通话记录(call_history)
│   │   ├── sms_listener.rs   # 后台任务：监听 ofono D-Bus 信号，落库并触发转发
│   │   ├── sms_push.rs       # 短信推送（PushPlus/Server酱/PushDeer/Bark/ntfy）
│   │   ├── webhook.rs        # 短信/通话事件转发到自定义 Webhook
│   │   ├── ota.rs            # OTA 包解析、校验（MD5）、应用（本地上传式，无远程拉取）
│   │   ├── usb_switch.rs     # USB 网络模式切换（调用系统命令）
│   │   ├── iptables.rs       # 防火墙规则计数/清理
│   │   ├── serial.rs         # with_serial：串行化对 modem 的并发访问
│   │   ├── utils.rs          # 系统信息采集（CPU/温度/内存/网口）
│   │   ├── state.rs          # AppState（共享状态：DBus 连接、DB、配置、发送器）
│   │   └── pin.rs 无 —— 无鉴权层
│   ├── build.rs              # 编译期注入 VERSION、git branch/commit
│   ├── Cargo.toml            # 包名 udx710，release profile: strip+lto+panic=abort
│   └── .cargo/config.toml    # musl 交叉编译 linker 配置
│
├── frontend/                 # React 管理界面（构建产物部署为设备上的 www/）
│   ├── src/
│   │   ├── main.tsx, App.tsx
│   │   ├── api/index.ts      # API 封装，API_BASE = '/api'
│   │   ├── api/types.ts      # 前后端共享类型
│   │   ├── pages/            # 页面：Dashboard/ Network/ SMS/ Phone/ DeviceInfo/
│   │   │                     #       Configuration/ OtaUpdate/ InitScript/ ATConsole/ Terminal
│   │   ├── pages/Dashboard/  # 仪表盘（组件化：DeviceInfoCard/SimCardInfo/CellInfo/…）
│   │   ├── components/       # 布局（MainLayout/Sidebar/TopBar）、通用组件
│   │   ├── contexts/         # ThemeContext、RefreshContext
│   │   ├── hooks/            # useApi、useAdaptivePolling
│   │   └── utils/, theme.ts
│   ├── vite.config.ts        # 开发代理 /api -> http://192.168.66.1:80
│   └── package.json          # 名称 udx710，版本与后端保持一致
│
├── scripts/                  # 构建/部署/打包脚本
│   ├── build.sh              # 编译后端+前端，组装 userdata 目录
│   ├── deploy.sh             # 通过 ADB push 部署到设备 /home/root
│   ├── pack-ota.sh           # 打 OTA 包（meta.json + udx710 + www）
│   ├── pack-userdata.sh      # 打 UBIFS userdata 分区镜像（含 ttyd/busybox 等外部二进制）
│   ├── monitor.sh, setup-env.sh
├── bruno-api/                # Bruno API 测试集合（每个端点一个 .bru，是最全的接口清单）
├── .github/workflows/build-ota.yml   # CI：交叉编译 + UPX + 打 OTA 包（仅 workflow_dispatch）
├── dbus.sh                   # ofono D-Bus / AT 指令速查备忘
├── VERSION                   # 全局版本号（后端 Cargo.toml、前端 package.json 需一致）
└── README.md, AGENTS.md, band.md
```

## 运行时架构与原理

### 进程与启动
- 设备开机由 `/home/root/loader.sh` 拉起（内容由 `config.rs` 管理），依次启动：
  1. `/home/root/ttyd/start.sh`（ttyd Web 终端，**独立于本项目的第三方二进制**，监听 7681）
  2. `/home/root/udx710 -p 80`（**本项目后端**，默认监听 80）
  3. `sh /home/root/init.sh`（用户自定义开机脚本，可在 Web「初始化脚本」页面编辑）
- 后端 `main()` 启动时：连接 system D-Bus → 打开/迁移 SQLite → 加载 config.json → `ensure_loader_hooks_init()` 维护 loader.sh → 启动多个 `tokio::spawn` 后台任务：
  - **短信监听**：订阅 ofono `MessageManager.IncomingMessage` 信号，落库并转发 webhook/push
  - **通话监听**：订阅 `VoiceCallManager`/`VoiceCall` 信号，记录通话历史并转发
  - **数据连接自动初始化**（延迟 2s）
  - **数据连接看门狗**（每 15s 检查，断线自动恢复）

### 请求路径
- 前端静态资源由后端托管：`main.rs` 的 `spa_fallback` 从二进制同级 `www/` 目录读取，找不到则回退 `index.html`（SPA 路由）。
- API 全部挂在 `/api/*`，前端 `API_BASE='/api'`；开发时 Vite 代理到 `192.168.66.1:80`。
- 所有涉及 modem 的 D-Bus 调用经 `serial.rs` 的 `with_serial()` 串行化，避免并发访问 modem 冲突。

### 数据与配置持久化
- `get_persistent_root_dir()`：设备上优先用 `/data`（存在则用），否则回退可执行文件同级目录。
- 配置：`{持久化目录}/config.json`（Webhook、短信推送、刷新策略等）
- 数据库：`{持久化目录}/data.db`（SQLite）
  - `sms_messages(id, direction, phone_number, content, timestamp, status, pdu, created_at)`
  - `call_history(id, direction, phone_number, duration, start_time, end_time, answered, created_at)`

### OTA 机制（重要）
- **纯本地上传式，后端无任何远程下载/回连逻辑**。流程：前端上传 `.tar.gz`（`meta.json`+`udx710`+`www`）→ `POST /api/ota/upload` 暂存并校验 MD5 → `POST /api/ota/apply` 覆盖二进制与 www 目录，可选重启。
- OTA 包**只更新应用层（udx710 + www）**，不含也不触碰 ttyd/busybox 等底层二进制（那些随 userdata 镜像烧录）。

## API 约定
- 统一响应包裹：`{ "status": "ok"|"error", "message": string, "data": T }`（见 `models.rs` 的 `ApiResponse`）。
- 端点全清单以 `bruno-api/*.bru` 为准（约 90 个），命名规则清晰：`get_*` 读、`set_*`/`post_*` 写。
- 关键端点分类：设备/SIM/网络信息、数据/漫游/飞行/制式/频段/小区、通话与短信、APN/运营商、USB 模式、系统统计/重启、OTA、init-script、webhook/sms-push 配置。

## 开发与构建

```bash
# 前端开发（代理到真机 192.168.66.1）
cd frontend && pnpm install && pnpm dev

# 前端构建 / 检查
pnpm build          # = pnpm lint && vite build
pnpm type-check

# 后端交叉编译（需 musl 工具链）
cd backend && cargo build --release --target aarch64-unknown-linux-musl

# 一键构建 + 部署到真机（需 adb 连接设备）
./scripts/build.sh
./scripts/deploy.sh            # adb push 到 /home/root，可 --backend-only/--frontend-only

# 打 OTA 包
./scripts/pack-ota.sh
```

- **版本一致性**：改版本号需同步 `VERSION`、`backend/Cargo.toml`、`frontend/package.json`，CI 会校验三者一致否则构建失败。
- **后端启动参数**：`-p/--port`（默认 3000，设备上以 80 启动）、`-H/--host`（默认 0.0.0.0）、支持 `PORT`/`HOST` 环境变量；日志用 `RUST_LOG` 控制（默认 info）。

## 开发约定与注意事项
- 后端**无鉴权层**，设计上假设运行在设备本地可信网络（USB 直连 `192.168.66.1`）。新增端点时沿用现有 `AppState` 注入与 `ApiResponse` 包裹风格即可。
- 前端换行统一 LF；仓库 `.editorconfig`/git 会对 `.tsx/.ts/.sh` 做 CRLF↔LF 处理，注意提交时的换行告警属正常。
- 涉及 modem 的新 D-Bus 调用务必走 `with_serial()`。
- 敏感信息（IMEI/ICCID/号码等）展示由前端各卡片的 `showInfo` 状态控制，属 UI 层显示开关。
- 新增前端页面：在 `pages/` 下建组件并在 `Sidebar` + 路由（`App.tsx`）注册。
- 修改 `bruno-api/` 保持与后端端点同步，它是接口的事实文档。
