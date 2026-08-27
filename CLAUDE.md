本文档为 Claude Code（及其他 AI 协作会话）提供 **project-cpe（UDX710 5G/LTE CPE 后台管理系统）** 的项目上下文，帮助理解项目结构、实现原理与开发约定。

无论用户输入的内容包含哪种语言（尤其是包含英文代码、报错信息或专业术语时），请始终强制使用简体中文进行回答和解释。只有当用户明确发出"用英语回答"或"翻译"的请求时，才可以使用其他语言。

## 环境操作限制

未经用户明确许可，禁止安装软件、工具链、编译器、rustup target、依赖等任何侵入性、影响本地开发环境的操作。本项目仅交叉编译到 `aarch64-unknown-linux-musl`（见"运行与部署"），在 Windows 上无法原生 `cargo build` 通过（`std::os::unix`、`libc::statvfs`/`uname` 等平台限制），这是已知的正常现象，不要尝试通过安装工具链、切换 target 等方式"修复"，语法层面正确即可，实际编译验证以 CI 或用户在真实设备/macOS 上手动操作为准。

## 需求处理流程（强制）

除非用户明确说明是"顺手""小改""直接改"这类不需要确认的琐碎调整，否则收到任何功能性需求时，**必须先完成理解与确认，再动手实现**：

1. **复述需求**：用自己的话简要复述理解到的需求，包含隐含的前提/边界（如涉及范围、是否要兼容旧数据、是否要动已有行为等）。如果需求本身有歧义或多种合理理解，明确指出分歧点，不要自行选一种理解就往下做。
2. **给出简要计划**：列出打算改动的文件/模块、大致思路，以及会不会涉及本文件"核心设计原则"或"关键设计决策"里的约束。计划要简短（几行到十几行即可），不是完整设计文档。
3. **等待用户确认后才动手实现**，包括写代码、跑测试、提交代码。用户可能会在这一步纠正理解偏差或调整方向，此时按新的理解重新给出复述和计划，而不是直接开始改。

该流程的意义是尽早发现"需求理解错了"或"方向错了"，避免做完一大圈工作后才发现要推倒重来；能省一步确认的简单任务不必生搬硬套。

[README.md](README.md) 面向使用者，讲功能特性、API 接口清单、频段/D-Bus 参考与性能调优；本文件面向开发，讲运行时原理、关键设计决策与操作性约束。两者冲突时以 README.md 描述的实际接口行为为准，并同步修订本文件。

仓库中另有 [.cursorrules](.cursorrules)（面向 Cursor 编辑器）与 [AGENTS.md](AGENTS.md)（极简速览），内容与本文件的"核心设计原则""架构说明"部分重合。三者冲突时以本文件为准；变更设计原则或架构时应同步更新 .cursorrules，避免不同工具下的 AI 得到不一致的指导。

## 项目概述

为市面上成品 5G CPE 设备（当前验证通过：华为 5G 通讯壳 P50/P60/Mate 系列，展锐 UDX710 平台）提供的进阶后台管理系统。设备本身运行紫光展锐原厂 Linux 固件（内核 4.14 aarch64），`ofonod`（电话协议栈）、`sprdrild`（展锐 RIL）、`connmand` 等是原厂组件；本项目**不实现底层能力，只作为应用层**，通过系统 D-Bus 调用 ofono，把蜂窝网络/短信/通话/设备管理能力封装成 HTTP API + Web 界面。

- **后端**（[backend/](backend/)）：Rust + Axum + zbus，单一二进制，常驻运行在设备的嵌入式 Linux（aarch64-musl）环境中，直接连接系统 D-Bus。
- **前端**（[frontend/](frontend/)）：React 19 + TypeScript + MUI，构建为静态文件，由后端同端口托管（SPA），不单独部署、不用 Docker。
- **核心工作流程**：前端调用 `/api/*` → 鉴权中间件（可选）→ handler 通过 `with_serial` 串行执行 D-Bus 调用/AT 指令操作 ofono → 统一 `ApiResponse<T>` 结构返回。

主要功能：

- 设备/SIM/网络/信号/基站信息展示
- 移动数据、漫游、飞行模式、网络制式（4G/5G）、频段锁定、小区锁定
- 短信收发与会话管理、来电/去电/通话记录、呼叫转移、呼叫设置
- APN 管理、运营商扫描与注册、USB 网络模式切换（RNDIS/ECM/NCM）
- 系统资源监控（CPU/温度/内存/磁盘）、系统重启
- OTA 在线更新、开机自定义脚本（init.sh，可用于内核级网络调优等场景）
- 短信转发到 Webhook（飞书等）或推送服务（PushPlus/Server 酱/PushDeer/Bark/ntfy/钉钉群机器人）
- AT 指令控制台、Web 终端（ttyd，独立进程，经后端反代访问）
- 可选登录鉴权（默认关闭，Argon2 密码 + 内存 session）

当前处于**持续迭代**阶段（dev 分支领先 main 分支，主要增量是登录鉴权与 Web 终端反代两套安全相关子系统）。README.md 的 API 接口清单是已实现功能的权威列表，本文件不重复维护。

## 核心设计原则（最高优先级）

1. **【MUST】D-Bus/AT 操作全局串行化** —— 所有对 ofono 的 D-Bus 调用和 AT 指令（包括只读查询）必须通过 [serial.rs](backend/src/serial.rs) 的 `with_serial()` 包裹执行。ofono 不支持并发操作，并发调用会产生 `org.ofono.Error.InProgress` 错误。

2. **【MUST】飞行模式用 `Online` 属性，不用 `Powered`** —— 飞行模式必须通过 modem 的 `Online` 属性实现（关闭射频、保持上电）；`Powered=false` 会完全关闭调制解调器，不应用于飞行模式语义。涉及 [dbus.rs](backend/src/dbus.rs)。

3. **【MUST】射频模式（4G/5G）切换用 `org.ofono.RadioSettings` 的 `TechnologyPreference`** —— 不使用 `AT+SPLMODE` 指令，D-Bus 方式更可靠；切换后网络会重新注册，调用方需要等待。

4. **【MUST】ttyd 必须绑定回环、只能经后端反代访问** —— ttyd 用 `-H X-Remote-User` 做"鉴权"，但该请求头的值可被客户端任意伪造，唯一的安全边界是网络层面：ttyd 只监听 `127.0.0.1`，局域网请求物理上到不了它。任何改动都不能让 ttyd 监听非回环地址，否则局域网内任何人都能拿到设备 root shell。相关校验/自愈逻辑见 [terminal_proxy.rs](backend/src/terminal_proxy.rs) 的 `ensure_ttyd_proxy_runtime`。

5. **【MUST】数据连接切换清空 iptables 规则后必须补回端口保护规则** —— `/api/data` 切换时执行的 `iptables -F`（[iptables.rs](backend/src/iptables.rs)）会清掉包括第 4 条防护规则在内的所有规则；`flush_iptables()` 因此在清空后自动调用 `ensure_ttyd_port_protected()`（始终执行，安全不变量）与 `ensure_vendor_debug_ports_protected()`（仅当 `SecurityConfig::vendor_debug_port_protection` 为 `true` 时执行）补回。后者防护的是展锐原厂固件自带、无鉴权监听的工程调试端口（`VENDOR_DEBUG_PORTS`：adbd TCP 5555、remote_mgr 8002-8004/8006、engpc 10056/10057）——安全审查确认这些端口仅靠"网络不可达"作为唯一防线，局域网内可路由到就能直接拿到 root shell；该防护默认开启，可在 Web「系统配置」页（`/api/security/config`）关闭，关闭后 `remove_vendor_debug_ports_protection()` 会立即撤销已插入的规则，不必等待下一次 flush。ttyd 端口保护不提供关闭入口，任何改动都不能让它变成可选项（见第 4 条）。这些规则是安全不变量或按用户主动选择关闭的防护，不是可被"恢复干净网络状态"无差别带走的普通网络配置，新增任何清空/重置 iptables 的地方都要照此处理，且 `flush_iptables()` 现在需要 `&ConfigManager` 参数以读取该配置。

6. **【MUST】新增系统指标优先扩展 `/api/stats`，不轻易新建端点** —— 新字段加入 `SystemStatsResponse`（[models.rs](backend/src/models.rs)，已含网速/内存/磁盘/CPU/温度/运行时间/USB 模式），减少前端请求数与并发 D-Bus 冲突。仅当数据量特别大（如日志）、需要独立刷新频率、或属于完全独立功能域（如 `/api/location/cell-info`）时才新建端点。

7. **【MUST】新增/修改/删除 API 时同步维护 [bruno-api/](bruno-api/)** —— 每个 `main.rs` 路由须有对应 `.bru` 文件（GET 用 `get_*.bru`，写操作用 `set_*.bru`/`post_*.bru`），并更新 [bruno-api/README.md](bruno-api/README.md)。当前 `/api/auth/*`、`/api/terminal/proxy*` 是例外（依赖 session cookie，脚本化测试成本高），其余端点不应再有缺口。

## 通用开发约束

- **【MUST】** 保持代码简洁易懂，避免过度设计和不必要的抽象；不擅自添加用户未要求的功能。
- **【MUST】** 用户提出需求时，先阅读 [README.md](README.md) 和相关模块代码，理解当前项目目标、架构与实现方式。
- **【MUST】** 功能/架构发生变化后同步更新 README.md 与本文件，避免文档与代码脱节。
- **【MUST】** 新增功能时创建新模块或函数，不要把逻辑全部塞进已有的大文件或大函数（`handlers.rs`/`dbus.rs`/`utils.rs` 已经很大，新增独立功能域时优先考虑拆分新模块，如 `auth.rs`/`terminal_proxy.rs` 的先例）。
- **【MUST】** 项目特性按用户需求分阶段实现，不需要一次性把 README/设计文档里列出的所有特性都做完。
- **【MUST】** 处理需求前先查"架构说明"定位到相关模块，用 grep/关键字搜索确认涉及范围，只读取真正相关的文件；不要在不确定范围时就通读整个项目或大量无关文件。"架构说明"和"关键设计决策"没有覆盖到、确实无法判断影响范围时，再扩大搜索或直接询问用户。
- **【SHOULD】** 查询外部资源时优先参考官方文档（ofono、zbus、axum、argon2 等官方文档），专业论坛为辅，中文互联网资源仅供参考。

## 代码风格

- **【MUST】** 不在代码注释、日志输出、控制台输出中使用 emoji 表情及特殊符号（终端显示可能异常）；README.md、脚本的用户提示输出与文档中可以使用。
- **【MUST】** 敏感信息（session token、Webhook secret、短信推送凭证、密码哈希等）严禁写入日志、异常信息，或提交到仓库；这些内容只存在于设备运行时生成的 `config.json`/内存中。
- **【MUST】** 涉及网络请求 / D-Bus 调用 / 文件读写等操作，须捕获异常并记录日志，失败要可见（不静默吞掉），且不应无谓中断整体流程（单点失败不影响其它任务，即"故障隔离"，如 iptables 兜底规则写入失败只记警告）。
- **【MUST】** 日志使用 `tracing`（如 `info!`/`warn!`/`error!`），不用 `println!`；日志初始化统一在 `main.rs` 用 `tracing_subscriber`，级别由 `RUST_LOG` 环境变量控制（默认 info）。
- **【MUST】** Rust 后端 API 响应统一使用 `ApiResponse<T> { status, message, data }` 结构（`status` 取值 `"ok"`/`"error"`）；错误处理统一用 `anyhow::Result` 或 `Result<T, String>`。
- **【MUST】** 阻塞/CPU 密集操作（Argon2 哈希校验、`std::process::Command` 同步调用等）必须放进 `tokio::task::spawn_blocking`，不能独占 tokio 工作线程——这台设备核心数很少，卡住一个线程足以拖住其它并发请求（含登录接口）。
- **【SHOULD】** 前端新文件统一用 TypeScript；React 组件优先使用函数组件 + Hooks；路由用 `react-router-dom`；数据请求优先复用已有的 `swr`/`@tanstack/react-query` 模式，而非直接 `fetch`。
- **【SHOULD】** 新增第三方依赖前先评估必要性，保持依赖精简，同步更新 `Cargo.toml`/`package.json`。
- 前端换行统一 LF；仓库 `.editorconfig`/git 会对 `.tsx/.ts/.sh` 做 CRLF↔LF 处理，提交时出现的换行告警属正常现象。

## 文档规范

README.md、CLAUDE.md 及代码注释遵守：

- 只客观描述功能、现状、使用方法与约束，不强调"做了哪些修改/优化"、不解释"为什么这么改"（代码注释、commit message、对话回复同理）；确需长期保留原因的，写进"关键设计决策"这类明确标注为决策记录的章节。
- 篇幅精简，不写长篇大论：一句话能说清楚的不写第二句，不铺垫背景、不重复自证；docstring/注释长度应与同一文件里其它同类函数/条目保持一致的密度。
- "关键设计决策"只记录当前结论与必要的取舍/风险点，不归因到"业务方拍板/确认"、不写审批日期或人名，不用"这是一个例外"之类强调语气自证——客观陈述当前行为即可。
- 使用中文编写，代码块、表格、列表等不同元素之间需要有空行隔开，合理缩进，避免网页渲染时出现问题。
- 全角中文字符与半角英文字符之间，应有一个半角空格；全角中文字符与半角阿拉伯数字之间，有没有半角空格都可，但必须保证风格统一，不能两种风格混杂。

## 架构说明

数据流：HTTP 请求 → [main.rs](backend/src/main.rs) 路由（先过鉴权中间件）→ [handlers.rs](backend/src/handlers.rs) → [dbus.rs](backend/src/dbus.rs) / [usb_switch.rs](backend/src/usb_switch.rs) / [utils.rs](backend/src/utils.rs) 等 → ofono D-Bus / AT 指令 / `/proc`、`/sys` → [models.rs](backend/src/models.rs) 定义的结构序列化为 `ApiResponse<T>` 返回。

### 后端（backend/src/）

- **[main.rs](backend/src/main.rs)** —— 路由注册、CLI 参数（`--port`/`-H --host`，可用 `PORT`/`HOST` 环境变量）、启动时初始化顺序（D-Bus 连接 → 数据库 → ConfigManager → CSPRNG 预热 → ttyd 校准 → SMS/通话监听线程 → 自动连接数据网络 → 数据连接 watchdog）、鉴权中间件挂载、`spa_fallback` 前端静态资源托管、优雅关闭。新增路由、改启动流程看这里。
- **[handlers.rs](backend/src/handlers.rs)**（体量最大）—— 所有 HTTP handler，按功能分类（设备/SIM/网络/小区/通话/短信/USB/系统/Webhook/OTA/鉴权 等）。新增一个 API 时看这里加 handler。
- **[dbus.rs](backend/src/dbus.rs)** —— ofono D-Bus 代理封装（`Modem`/`NetworkRegistration`/`SimManager`/`ConnectionManager`/`RadioSettings` 等接口）、AT 指令发送、`init_data_connection`、`data_connection_watchdog`。改"发哪个 D-Bus 接口/AT 指令""数据连接自动重连逻辑"看这里。
- **[models.rs](backend/src/models.rs)** —— 所有 API 请求/响应数据结构（含 `ApiResponse<T>`、`SystemStatsResponse`、`AuthConfig` 等）。
- **[utils.rs](backend/src/utils.rs)** —— 系统信息读取（`/proc`、`/sys`，含 CPU/温度/内存/磁盘/网口）、AT 响应文本解析、格式化等工具函数。
- **[usb_switch.rs](backend/src/usb_switch.rs)** —— USB 模式（普通/高级）切换实现。
- **[config.rs](backend/src/config.rs)** —— `ConfigManager`：`config.json` 加载/持久化（Webhook/短信推送/刷新策略/鉴权配置）；`get_persistent_root_dir()`/`get_default_config_path()` 决定配置与数据库存放位置；`ensure_loader_hooks_init()` 维护 `loader.sh`；`ensure_ttyd_proxy_mode()` 校准 `ttyd/start.sh` 的启动参数（`TTYD_START_SCRIPT_PATH` 常量）；init.sh 读写。
- **[db.rs](backend/src/db.rs)** —— SQLite（`rusqlite`，bundled）封装，短信、通话记录持久化（`data.db`）。
- **[sms_listener.rs](backend/src/sms_listener.rs)** —— D-Bus 信号监听（`start_sms_listener`/`start_call_listener`），短信/通话事件写入数据库并触发 webhook/短信推送。
- **[sms_push.rs](backend/src/sms_push.rs)** —— 第三方短信转发推送发送器。
- **[webhook.rs](backend/src/webhook.rs)** —— Webhook 配置与发送（HMAC 签名基于 `secret`）。
- **[iptables.rs](backend/src/iptables.rs)** —— 规则计数/清空（`flush_iptables`，接收 `&ConfigManager`，附带自动补回 ttyd 端口保护规则与按需补回/撤销原厂调试端口保护规则）、`ensure_ttyd_port_protected()`。
- **[auth.rs](backend/src/auth.rs)** —— 密码哈希（Argon2）、session 生成/校验/续期、登录限流、CSPRNG 预热、全局鉴权中间件 `auth_middleware`。改鉴权相关逻辑看这里，详见"运行时架构与原理 → 鉴权与终端代理安全模型"。
- **[terminal_proxy.rs](backend/src/terminal_proxy.rs)** —— ttyd 反向代理（HTTP + WebSocket 隧道）、ttyd 运行状态探测与自愈重启（`ensure_ttyd_proxy_runtime`/`restart_ttyd_verified`）。
- **[serial.rs](backend/src/serial.rs)** —— 全局 `with_serial()` 互斥锁（见"核心设计原则"第 1 条）。
- **[state.rs](backend/src/state.rs)** —— Axum 共享状态 `AppState`（D-Bus 连接、数据库、ConfigManager、Webhook/短信推送发送器、`FrontendRuntime`、`SessionStore`）。
- **[build.rs](backend/build.rs)** —— 编译期注入版本号（读 [VERSION](VERSION)）、Git 分支/commit（`APP_VERSION`/`GIT_BRANCH`/`GIT_COMMIT`）。

### 前端（frontend/src/）

- **[api/index.ts](frontend/src/api/index.ts)** + **[api/types.ts](frontend/src/api/types.ts)** —— API 客户端方法与对应 TypeScript 类型；401 响应会派发 `udx710:unauthorized` 事件。后端新增接口后必须同步在这里加方法和类型。
- **[pages/](frontend/src/pages/)** —— 各功能页面：`Dashboard/`、`Network.tsx`、`Phone.tsx`、`SMS.tsx`、`OtaUpdate.tsx`、`Configuration.tsx`（含鉴权开关/改密码 UI）、`ATConsole.tsx`、`Terminal.tsx`、`InitScript.tsx`、`DeviceInfo.tsx`、`Login.tsx`。
- **[contexts/AuthContext.tsx](frontend/src/contexts/AuthContext.tsx)** —— 登录状态（`enabled`/`loggedIn`/`statusKnown`）与 `RequireAuth` 路由守卫、`udx710:unauthorized` 事件处理。**[contexts/RefreshContext.tsx](frontend/src/contexts/RefreshContext.tsx)** —— 统一自动刷新控制；**[contexts/ThemeContext.tsx](frontend/src/contexts/ThemeContext.tsx)** —— 主题（配合 [theme.ts](frontend/src/theme.ts)）。
- **[components/](frontend/src/components/)** —— 布局（`Layout/MainLayout`、`Layout/Sidebar`、`Layout/TopBar`）与通用组件：`ConfirmDialog`（二次确认）、`ErrorBoundary`（渲染期异常兜底）、`ErrorSnackbar`（错误提示）。
- **[utils/lazyWithReload.ts](frontend/src/utils/lazyWithReload.ts)** —— 包装 `React.lazy`，chunk 取不到时整页 reload（应对 OTA 覆盖 www 后旧 chunk 文件名失效）。

### 其它

- **[scripts/](scripts/)** —— `build.sh`（交叉编译，支持 `--upx`）、`deploy.sh`（ADB 部署，支持 `--backend-only`/`--frontend-only`/`--no-restart`/`--target=`）、`pack-ota.sh`/`pack-userdata.sh`（打包）、`monitor.sh`、`setup-env.sh`（macOS 交叉编译环境配置）。
- **[bruno-api/](bruno-api/)** —— Bruno API 测试集合（83 个 `.bru` 文件），须与后端路由保持同步（见"核心设计原则"第 7 条）。
- **[.github/workflows/build-ota.yml](.github/workflows/build-ota.yml)** —— 手动触发的 OTA 包构建 CI（交叉编译后端 + 构建前端 + 打包 + 校验三处版本号一致）；不作为 PR 门禁，不跑测试/lint。

## 运行时架构与原理

### 进程与启动

设备开机由 `/home/root/loader.sh` 拉起（内容由 `config.rs` 管理），依次启动：

1. `/home/root/ttyd/start.sh`（ttyd Web 终端，**独立于本项目的第三方二进制**，监听 7681；启动参数与安全绑定校准见下方"鉴权与终端代理安全模型"）
2. `/home/root/udx710 -p 80`（**本项目后端**，默认监听 80）
3. `sh /home/root/init.sh`（用户自定义开机脚本，可在 Web"初始化脚本"页面编辑，社区已有内核级网络调优等用法，见 README）

后端 `main()` 启动时：连接 system D-Bus → 打开/迁移 SQLite → 加载 config.json → `ensure_loader_hooks_init()` 维护 loader.sh → 预热 CSPRNG（`auth::spawn_rng_warmup`，设备无硬件熵源，内核 CRNG 需约 100 秒累积中断熵才就绪）→ 校准并校验 ttyd → 启动多个 `tokio::spawn` 后台任务：

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
- 配置：`{持久化目录}/config.json`（Webhook、短信推送、刷新策略、鉴权配置等）
- 数据库：`{持久化目录}/data.db`（SQLite）
  - `sms_messages(id, direction, phone_number, content, timestamp, status, pdu, created_at)`
  - `call_history(id, direction, phone_number, duration, start_time, end_time, answered, created_at)`

### OTA 机制（重要）

- **纯本地上传式，后端无任何远程下载/回连逻辑**。流程：前端上传 `.tar.gz`（`meta.json`+`udx710`+`www`）→ `POST /api/ota/upload` 暂存并校验 MD5 → `POST /api/ota/apply` 覆盖二进制与 www 目录，可选重启。
- OTA 包**只更新应用层（udx710 + www）**，不含也不触碰 ttyd/busybox 等底层二进制（那些随 userdata 镜像烧录）。

### 鉴权与终端代理安全模型（重要）

- **默认关闭**：`AuthConfig::default().enabled == false`，装机即用；在 Web「系统配置」页开启并设置用户名/密码后才生效。
- **中间件只拦截 `/api/*`**（`auth.rs::auth_middleware`），`/api/auth/login`、`/api/auth/status`、`/api/health` 及全部 SPA 静态资源永远放行——否则鉴权异常时会连登录页都加载不出来。
- **Session**：仅存内存（`SessionStore`，token -> 过期时间），**设备重启即全部失效**，不落库。Cookie 名 `udx710_session`，`HttpOnly; SameSite=Lax`，24 小时 TTL，剩余不足 12 小时时随请求自动滑动续期并重新下发 `Set-Cookie`；`/api/auth/logout`、`/api/auth/config` 两个端点不参与续期（它们本身会主动废弃会话）。
- **密码**：Argon2id 哈希；新哈希使用调轻的参数（8 MiB / 1 轮，而非 RFC 9106 默认的 19 MiB / 2 轮），因为设备要与 `ofonod`/`sprdrild` 抢 CPU，默认参数单次校验可能耗时数十秒——校验时用的是哈希串内嵌的历史参数，旧密码需重设一次才会换成轻量参数。密码规则：8~128 位可见 ASCII（不含空格）。
- **登录接口防护**：全局并发闸（同时最多 2 个登录请求，超出直接拒绝不排队，防止 Argon2 内存占用被打爆）；按来源 IP 计数，10 分钟内失败 10 次即锁定 10 分钟；密码比较用常数时间比较，响应固定最小延迟 300ms，两者共同抹平时序侧信道。
- **ttyd Web 终端信任模型**：ttyd 自身不做鉴权，仅要求请求带有 `X-Remote-User` 头（值可被客户端任意设置），因此**必须**绑定在 `127.0.0.1`（启动参数 `-i lo -b /api/terminal/proxy -H X-Remote-User`）、只能经后端反代访问；后端启动时校准 `start.sh` 参数并探测 ttyd 是否确实「代理模式 + 绑定回环」，不满足则重启（最多 3 次，每次等待 15 秒），同时用 iptables 插入 `-i !lo --dport 7681 -j DROP` 兜底（`flush_iptables()` 清空规则后会自动补回，见 [iptables.rs](backend/src/iptables.rs)）。WebSocket 隧道建立后不再经过 HTTP 鉴权中间件，靠后台每 30 秒轮询 session 有效性主动断开，退出登录/改密码不会立刻掐断已打开的终端。
- **调试端口保护开关**：`/api/security/config`（GET/POST）控制是否防护 `VENDOR_DEBUG_PORTS`（见第 5 条），默认开启；关闭/开启立即生效（分别调用 `remove_vendor_debug_ports_protection()`/`ensure_vendor_debug_ports_protected()`），无需重启设备。仅影响原厂调试端口，不提供关闭 ttyd 保护的入口。前端入口在「系统配置」页「面板登录鉴权」区块下方的「调试端口保护」区块。
- **前端**：`AuthContext`/`RequireAuth`（[AuthContext.tsx](frontend/src/contexts/AuthContext.tsx)）在登录状态未知前只渲染占位，避免未登录时业务树整棵挂载、并发打出十几个必然 401 的请求；收到 `udx710:unauthorized` 事件后用整页 `location.replace('/login')` 而非 SPA 路由跳转（会话失效没有内存状态需要保留，且能拿到 OTA 后最新的 chunk 清单）。
- `/api/auth/*`、`/api/terminal/proxy*` 尚未纳入 `bruno-api/`（依赖 session cookie，脚本化测试成本高于其余无状态端点）。

## 关键设计决策

### 1. 配置与数据持久化路径

`get_persistent_root_dir()`（[config.rs](backend/src/config.rs)）优先使用 `/data`（真实设备上的持久化分区）；本地开发环境该目录不存在时回退到可执行文件所在目录。`config.json`、`data.db` 均存放于此路径下，不随仓库分发，也不提供示例配置文件——所有配置（含 Webhook/短信推送的 URL、secret，以及鉴权的用户名/密码哈希）通过前端 Web UI 在设备运行时写入。旧版本遗留在可执行文件同级目录的 `data.db` 会在启动时自动迁移到新路径。

### 2. 前端随后端二进制分发，不单独部署、不用 Docker

后端通过 `spa_fallback`（[main.rs](backend/src/main.rs)）从可执行文件同级的 `www/` 目录读取前端构建产物；非 `/api/` 路径找不到对应文件时回退到 `index.html`（SPA 客户端路由）。构建产物直接通过 ADB（[deploy.sh](scripts/deploy.sh)）推送到设备，以单一进程常驻运行。

### 3. OTA 升级包结构与版本号一致性

`/api/ota/upload` 接受最大 50MB 的 `tar.gz` 包，固定包含 `meta.json` + `udx710`（后端二进制）+ `www/`（前端产物）。[VERSION](VERSION)、[backend/Cargo.toml](backend/Cargo.toml)、[frontend/package.json](frontend/package.json) 三处版本号必须一致，由 CI 在打包前校验，不一致则构建失败。

### 4. 仅支持交叉编译到 aarch64-unknown-linux-musl

后端只面向设备实际架构（静态链接 musl + `crt-static`，见 [backend/.cargo/config.toml](backend/.cargo/config.toml)）构建发布产物；本地开发在 macOS 上交叉编译，CI 在 `ubuntu-24.04-arm` 上构建。

### 5. Session 只存内存，不做持久化

鉴权 session 存在进程内 `HashMap`，设备重启（包括 OTA 应用后的重启）会让所有人被登出，需要重新输入密码。以此换来实现简单、不用额外处理"落库 session 的失效/清理"，代价是重启频率越高，登录频率也越高。

### 6. 登录接口的防御纵深优先于极致性能

Argon2 参数调轻是为了让设备在与 ofonod/sprdrild 抢 CPU 的前提下仍能在合理时间内完成校验，但登录路径仍额外叠加了并发闸、按 IP 失败锁定、常数时间比较与固定最小延迟——这些开销是刻意保留的，不应因为"看起来变慢了"而移除。

## 配置与凭证约束

- **【MUST】** 所有运行时凭证（session token、Webhook secret、短信推送凭证、密码哈希等）只存在于设备上运行时生成的 `config.json`/内存中（见"关键设计决策"第 1、5 条），**绝不提交到仓库**，也不写入日志或异常信息。
- **【MUST】** `.gitignore` 已排除构建产物（`backend/target/`、`frontend/dist/`、`frontend/node_modules/`）与 `release/`；新增会在本地生成敏感/大体量文件的路径时同步补充排除规则。
- **【SHOULD】** `/api/ota/upload`、`/api/at` 等接口的风险边界依赖全局鉴权开关；若鉴权未开启，这些接口对局域网内任何能访问设备的人开放，新增此类高风险接口前评估是否需要独立于全局鉴权的额外限制。

## 运行与部署

本地开发不需要 Docker。后端需要能连接系统 D-Bus 且有 ofono 服务运行的环境（通常直接在目标设备或已装 ofono 的 Linux 上跑）：

```bash
# 后端（默认监听 0.0.0.0:3000，可用 --port/-p、--host/-H 或 PORT/HOST 环境变量覆盖）
cd backend && cargo run

# 前端开发服务器（代理到真机 192.168.66.1）
cd frontend && pnpm install && pnpm dev
```

交叉编译、打包与部署到设备：

```bash
# 交叉编译后端 (macOS -> aarch64-unknown-linux-musl)，可选 UPX 压缩
./scripts/build.sh
./scripts/build.sh --upx

# 构建前端
cd frontend && pnpm run build

# 通过 ADB 部署到设备（默认目标路径 /home/root，会 killall 现有进程后重启）
./scripts/deploy.sh
./scripts/deploy.sh --backend-only
./scripts/deploy.sh --frontend-only
./scripts/deploy.sh --no-restart
./scripts/deploy.sh --target=/data/app

# 打 OTA 包
./scripts/pack-ota.sh
```

- **版本一致性**：改版本号需同步 `VERSION`、`backend/Cargo.toml`、`frontend/package.json`，CI 会校验三者一致否则构建失败。
- **后端启动参数**：`-p/--port`（默认 3000，设备上以 80 启动）、`-H/--host`（默认 0.0.0.0）、支持 `PORT`/`HOST` 环境变量；日志用 `RUST_LOG` 控制（默认 info）。

### 测试与检查

```bash
# 后端：仅 config.rs、iptables.rs 有少量单元测试；无集成测试框架，CI 也未强制运行
cd backend && cargo test
cargo clippy   # 未在 CI 中强制，建议改动前后自行运行

# 前端：build/build:full 会自动先跑 lint
cd frontend
pnpm lint         # eslint --max-warnings 0
pnpm type-check   # tsc -b --noEmit
pnpm build        # lint + vite build
```

后端接口层面没有自动化 E2E 测试，靠 [bruno-api/](bruno-api/) 的 Bruno 测试集合手动/半自动验证；`/api/auth/*`、`/api/terminal/proxy*` 及涉及 ttyd/iptables 的改动，无法在 Windows 本地模拟，必须在真实设备（或至少有 ofono/ttyd/iptables 的 Linux 环境）上验证。

## 实现状态

- **已实现**：README 接口清单覆盖的全部设备管理功能；可选登录鉴权（Argon2 + 内存 session + 登录限流）；Web 终端经后端反代访问（ttyd 绑定回环 + 启动时自愈校验）。
- **未覆盖**：`/api/auth/*`、`/api/terminal/proxy*` 无 bruno-api 测试覆盖；后端除 `config.rs`/`iptables.rs` 的少量单元测试外无更广泛的自动化测试（集成测试、前端测试均未配置）。

## 开发执行检查清单

### 需求确认阶段

- [ ] 已按"需求处理流程"复述需求、给出简要计划，并等到用户确认（琐碎调整除外）

### 编码阶段

- [ ] 是否遵循本文件"核心设计原则"中的约束（D-Bus 串行化、Online/Powered、RadioSettings、ttyd 绑定回环等）
- [ ] D-Bus/AT 调用是否都经过 `with_serial()`
- [ ] 阻塞/CPU 密集操作是否放进了 `spawn_blocking`
- [ ] 网络请求 / D-Bus 调用 / 文件读写是否捕获异常，不中断整体流程
- [ ] 敏感信息（session token、secret、密码哈希）未出现在日志、异常信息或提交内容中
- [ ] 新增/修改/删除路由时，[bruno-api/](bruno-api/) 下对应 `.bru` 文件与 README.md 是否同步更新

### 验证阶段

- [ ] 本地/设备环境运行核心流程无报错，行为符合预期
- [ ] 涉及数据连接/射频模式切换/鉴权/ttyd/iptables 的改动，已在真实设备上验证（无法用 Windows 本地环境模拟）
- [ ] `cargo test` 与 `pnpm lint`/`pnpm type-check` 通过

### 提交阶段

- [ ] 更新 README.md（如功能/接口变更）与本文件（如设计原则/架构/约束变更）
- [ ] 提交信息清晰说明改动内容
