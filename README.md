# UDX710 后台管理系统

面向市面成品 5G CPE 设备的后台管理系统，提供 Web 管理界面与扩展控制功能。

基于 Rust + Axum + zbus 的 5G/LTE 调制解调器后端服务，通过 ofono D-Bus 接口控制。

powered by Cursor Claude Opus 4.5 & Sonnet 4.5 & OpenAI GPT-5.1/5.2

欢迎 pr 和 issue，看到后会尽快处理。

## 免责声明

本项目仅供技术交流和学习使用，不得用于任何非法用途。任何使用本项目造成的任何后果，均与本项目无关，由使用者自行承担。

且目前测试通过的设备仅有：

- 华为 5G 通讯壳 P50 P60 Mate 系列

其余设备由于缺少设备，本人未做测试，你如果手里有多余的设备，可尝试*小心的*尝试使用，但不提供任何担保或保证。对设备造成任何的损坏，本人不承担任何责任。

或者愿意捐献设备来测试，可联系我，我将在第一时间进行测试并更新本项目。

## ⚖️ 开源协议声明

本项目采用 GNU General Public License v3.0 (GPLv3) 开源协议

鉴于目前大部分人对版权意识薄弱，特此声明

本项目采用 GPLv3 开源协议，您可以自由使用、研究、修改本软件，但必须保留所有版权声明和许可证声明，并且公开源代码，任何基于本项目的衍生作品也必须使用 GPLv3 协议。

### ✅ 您可以

- 自由使用、研究、修改本软件
- 分发本软件的副本
- 分发修改后的版本

### ⚠️ 但您必须

1. **保留所有版权声明和许可证声明** - 不得删除或修改原作者的版权信息
2. **公开源代码** - 如果您分发本软件或其修改版本，必须以 GPLv3 协议公开完整源代码
3. **使用相同协议** - 任何基于本项目的衍生作品也必须使用 GPLv3 协议
4. **标注修改** - 修改后的版本必须明确标注修改内容和修改日期
5. **提供许可证副本** - 分发时必须附带完整的 GPLv3 许可证文本

### ❌ 严禁以下行为

- **禁止闭源商业化**：不得将本项目或其衍生版本闭源后进行商业销售
- **禁止删除版权信息**：不得移除原作者的版权声明
- **禁止更改许可证**：不得将本项目改为其他许可证（如 MIT、Apache 等）
- **禁止专有软件化**：不得将本项目整合到专有/闭源软件中而不开源

## 🚀 快速开始

### 构建后端

```bash
# 交叉编译 (macOS -> Linux aarch64)
./scripts/build.sh

# 带 UPX 压缩
./scripts/build.sh --upx
```

### 构建前端

```bash
cd frontend && npm run build
```

### 部署

```bash
./scripts/deploy.sh
```

---

## 🔧 环境配置 (macOS)

```bash
# 1. 安装 Rust
brew install rust rustup
rustup default stable
rustup target add aarch64-unknown-linux-musl

# 2. 安装交叉编译工具链
brew tap messense/macos-cross-toolchains
brew install aarch64-unknown-linux-musl

# 3. 验证
rustup target list --installed
which aarch64-unknown-linux-musl-gcc
```

---

## 📡 ofono D-Bus 接口

### 核心接口

| 接口 | 说明 |
|------|------|
| `org.ofono.Manager` | 调制解调器管理 |
| `org.ofono.Modem` | Modem 属性和控制 |
| `org.ofono.NetworkRegistration` | 网络注册状态 |
| `org.ofono.SimManager` | SIM 卡管理 |
| `org.ofono.ConnectionManager` | 数据连接管理 |
| `org.ofono.VoiceCallManager` | 语音通话管理 |
| `org.ofono.MessageManager` | 短信管理 |

### 常用 D-Bus 命令

```bash
# 查看 Modem 属性
dbus-send --system --print-reply \
  --dest=org.ofono /ril_0 org.ofono.Modem.GetProperties

# 查看网络状态
dbus-send --system --print-reply \
  --dest=org.ofono /ril_0 org.ofono.NetworkRegistration.GetProperties

# 查看 SIM 卡信息
dbus-send --system --print-reply \
  --dest=org.ofono /ril_0 org.ofono.SimManager.GetProperties

# 设置飞行模式
dbus-send --system --print-reply \
  --dest=org.ofono /ril_0 org.ofono.Modem.SetProperty \
  string:"Online" variant:boolean:false

# 发送 AT 指令
dbus-send --system --print-reply \
  --dest=org.ofono /ril_0 org.ofono.Modem.SendAtcmd \
  string:"AT+CGSN"
```

### 监控 D-Bus

```bash
# 监听 ofono 发出的所有信号
dbus-monitor --system "sender='org.ofono'"

# 监听发给 ofono 的调用
dbus-monitor --system "destination='org.ofono'"

# 监听短信信号
dbus-monitor --system "interface='org.ofono.MessageManager'"
```

---

## 📶 频段锁定

仅供参考 真实性有待考证，请以实际设备为准

### LTE (4G) 频段

| 频段 | 位掩码 | 说明 |
|------|--------|------|
| B1 | 1 | FDD 2100MHz |
| B3 | 4 | FDD 1800MHz |
| B5 | 16 | FDD 850MHz |
| B8 | 128 | FDD 900MHz |
| B38 | 32 (TDD) | TDD 2600MHz |
| B40 | 128 (TDD) | TDD 2300MHz |
| B41 | 256 (TDD) | TDD 2500MHz |

### NR (5G) 频段

| 频段 | 位掩码 | 说明 |
|------|--------|------|
| N1 | 1 (FDD) | 2100MHz |
| N28 | 512 (FDD) | 700MHz |
| N41 | 16 (TDD) | 2500MHz |
| N77 | 128 (TDD) | 3700MHz |
| N78 | 256 (TDD) | 3500MHz |
| N79 | 512 (TDD) | 4500MHz |

### AT 指令

```bash
# 查询当前 LTE 频段
AT+SPLBAND=0

# 查询当前 NR 频段
AT+SPLBAND=3

# 锁定 LTE B1+B3
AT+SPLBAND=1,0,0,0,0,5,0

# 锁定 NR N78
AT+SPLBAND=2,0,0,256,0

# 解锁所有频段
AT+SPLBAND=1,0,0,0,0,0,0
AT+SPLBAND=2,0,0,0,0
```

---

## 📚 API 接口文档

### 基础信息

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/health` | GET | 健康检查 |
| `/api/device` | GET | 设备信息 (IMEI/ICCID/型号) |
| `/api/device/imeisv` | GET | 软件版本号 |
| `/api/sim` | GET | SIM 卡信息 |
| `/api/sim/slot` | GET | SIM 卡槽状态 |
| `/api/sim/slot/switch` | POST | 切换 SIM 卡槽 |

### 网络状态

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/network` | GET | 网络注册信息 |
| `/api/network/interfaces` | GET | 网络接口信息 |
| `/api/network/signal-strength` | GET | 信号强度 |
| `/api/network/nitz` | GET | 网络时间 |
| `/api/network/operators` | GET | 运营商列表 |
| `/api/network/operators/scan` | GET | 扫描运营商 (耗时) |
| `/api/network/register-manual` | POST | 手动注册运营商 |
| `/api/network/register-auto` | POST | 自动注册运营商 |
| `/api/cells` | GET | 基站信息 |
| `/api/location/cell-info` | GET | 基站定位参数 |
| `/api/qos` | GET | QoS 信息 |

### 模块控制

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/data` | GET/POST | 数据连接开关 |
| `/api/roaming` | GET/POST | 漫游开关 |
| `/api/airplane-mode` | GET/POST | 飞行模式开关 |
| `/api/radio-mode` | GET/POST | 射频模式 (4G/5G/自动) |
| `/api/band-lock` | GET/POST | 频段锁定 |
| `/api/cell-lock` | GET/POST | 小区锁定 |
| `/api/cell-lock/unlock-all` | POST | 解锁所有小区 |
| `/api/apn` | GET/POST | APN 配置 |
| `/api/usb-mode` | GET/POST | USB 模式切换 |
| `/api/usb-advance` | POST | 高级 USB 模式设置 |

### 通话功能

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/calls` | GET | 当前通话列表 |
| `/api/call/dial` | POST | 拨打电话 |
| `/api/call/hangup` | POST | 挂断指定电话 |
| `/api/call/hangup-all` | POST | 挂断所有电话 |
| `/api/call/answer` | POST | 接听来电 |
| `/api/call/volume` | GET/POST | 通话音量设置 |
| `/api/call/forwarding` | GET/POST | 呼叫转移设置 |
| `/api/call/settings` | GET/POST | 通话设置 |
| `/api/call/history` | GET | 通话记录列表 |
| `/api/call/history/{id}` | DELETE | 删除指定通话记录 |
| `/api/call/history/clear` | POST | 清空通话记录 |

### 短信功能

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/sms/send` | POST | 发送短信 |
| `/api/sms/list` | GET | 短信列表 |
| `/api/sms/conversation` | GET | 短信会话列表 |
| `/api/sms/stats` | GET | 短信统计 |
| `/api/sms/clear` | POST | 清空短信 |

### IMS/VoLTE

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/ims/status` | GET | IMS 状态 |
| `/api/voicemail/status` | GET | 语音信箱状态 |

### 系统信息

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/stats` | GET | 系统统计（网速/内存/运行时间） |
| `/api/stats/cpu` | GET | CPU 信息 |
| `/api/connectivity` | GET | 网络连通性检查 |
| `/api/system/reboot` | POST | 重启系统 |
| `/api/at` | POST | 执行 AT 指令 |

### Webhook 配置

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/webhook/config` | GET/POST | Webhook 配置管理 |
| `/api/webhook/test` | POST | 测试 Webhook |

### OTA 更新

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/ota/status` | GET | OTA 更新状态 |
| `/api/ota/upload` | POST | 上传 OTA 包 (最大 50MB) |
| `/api/ota/apply` | POST | 应用 OTA 更新 |
| `/api/ota/cancel` | POST | 取消 OTA 更新 |

---

## 🛠 开发指南

### D-Bus 操作序列化

所有 D-Bus/AT 操作必须通过 `with_serial` 串行执行：

```rust
use crate::serial::with_serial;

pub async fn send_at_command(conn: &Connection, cmd: &str) -> zbus::Result<String> {
    with_serial(async {
        let proxy = Proxy::new(conn, "org.ofono", "/ril_0", "org.ofono.Modem").await?;
        proxy.call("SendAtcmd", &(cmd)).await
    }).await
}
```

### API 响应格式

```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub status: String,   // "ok" 或 "error"
    pub message: String,
    pub data: Option<T>,
}
```

---

## 📈 性能调优（UDX710 平台）

> 感谢 [@qingwei0326](https://github.com/qingwei0326) 提供本节内容（[#22](https://github.com/1orz/project-cpe/pull/22)）。
> 这是**内核级网络调优**，不涉及本项目代码修改。

### 问题背景

UDX710 双核 Cortex-A55 上，Linux 默认将 5G 数据接收中断 `sipa`（IRQ 22）和 USB 出口中断 `xhci`（IRQ 97）**都绑定在 CPU0**。两条高频中断串行化在同一个核心上，导致：

- 下行吞吐量被单核瓶颈限制（实测峰值仅 200–400 Mbps）
- 系统 CPU 整体空闲率 87%，但 load average 高达 2.4–3.4（典型单核 I/O 排队表现）

可通过 `cat /proc/interrupts` 确认：如果 IRQ 22 和 IRQ 97 的计数全部集中在 CPU0 列，CPU1 列为 0，则存在此瓶颈。

### 优化方案

四步操作：**IRQ 亲和性分离** + **RPS 软中断分散** + **XPS 发包 CPU 分散** + **收包队列上限扩容**。

```bash
# 1) 将 xhci USB 中断（IRQ 97）迁移到 CPU1
#    注意：IRQ 编号以实际设备 /proc/interrupts 为准
echo 2 > /proc/irq/97/smp_affinity

# 2) 启用 RPS，将收包软中断分散到两个核心；启用 XPS，让发包 CPU 选择同样双核分散
for d in sipa_eth0 usb0; do
  for q in /sys/class/net/$d/queues/rx-*/rps_cpus; do
    echo 3 > "$q"
  done
  for q in /sys/class/net/$d/queues/tx-*/xps_cpus; do
    echo 3 > "$q"
  done
done

# 3) 扩大 RPS 流表、收包队列上限与单次软中断处理配额
echo 32768 > /proc/sys/net/core/rps_sock_flow_entries
echo 2000 > /proc/sys/net/core/netdev_max_backlog
echo 600 > /proc/sys/net/core/netdev_budget
```

### 持久化：写入项目 init.sh（推荐）

上述命令重启后失效。本项目自带 **初始化脚本** 功能，可在 Web 管理界面直接编辑并持久保存：

1. 打开 Web 管理后台 → 侧边栏「**初始化脚本**」页面
2. 将以下内容追加到 init.sh 编辑区：

```bash
# === UDX710 网络性能调优 ===
# logger 默认发 notice 级别，设备 syslogd 会过滤掉，须加 -p user.warning 才会落盘
tune_net() {
  xhci_irq=$(awk '/xhci/ {print $1}' /proc/interrupts | tr -d ':')
  [ -n "$xhci_irq" ] && [ -w "/proc/irq/$xhci_irq/smp_affinity" ] && [ "$(cat "/proc/irq/$xhci_irq/smp_affinity" 2>/dev/null)" != "2" ] && { echo 2 > "/proc/irq/$xhci_irq/smp_affinity"; logger -p user.warning -t udx710-tune "xhci irq $xhci_irq affinity -> CPU1"; }

  # RPS 收包、XPS 发包分散到双核
  for d in sipa_eth0 usb0; do
    for q in /sys/class/net/$d/queues/rx-*; do
      [ -w "$q/rps_cpus" ] && [ "$(cat "$q/rps_cpus" 2>/dev/null)" != "3" ] && { echo 3 > "$q/rps_cpus"; logger -p user.warning -t udx710-tune "$d $(basename "$q") rps_cpus -> 3"; }
      [ -w "$q/rps_flow_cnt" ] && [ "$(cat "$q/rps_flow_cnt" 2>/dev/null)" != "4096" ] && { echo 4096 > "$q/rps_flow_cnt"; logger -p user.warning -t udx710-tune "$d $(basename "$q") rps_flow_cnt -> 4096"; }
    done
    for q in /sys/class/net/$d/queues/tx-*; do
      [ -w "$q/xps_cpus" ] && [ "$(cat "$q/xps_cpus" 2>/dev/null)" != "3" ] && { echo 3 > "$q/xps_cpus"; logger -p user.warning -t udx710-tune "$d $(basename "$q") xps_cpus -> 3"; }
    done
  done

  for kv in "/proc/sys/net/core/rps_sock_flow_entries:32768" "/proc/sys/net/core/netdev_max_backlog:2000" "/proc/sys/net/core/netdev_budget:600"; do
    f="${kv%%:*}"; want="${kv##*:}"
    [ -w "$f" ] && [ "$(cat "$f" 2>/dev/null)" != "$want" ] && { echo "$want" > "$f"; logger -p user.warning -t udx710-tune "$f -> $want"; }
  done
}

# 每 60 秒巡检一次，应对 usb0 重新枚举导致 sysfs 节点被内核重建、
# 调优悄悄失效的情况；实测单次耗时约 0.1s，可忽略不计
(
  sleep 3
  while true; do
    tune_net
    sleep 60
  done
) &
```

3. 点击「**保存**」即可，下次开机自动生效。

> **原理说明**：init.sh 由设备开机脚本 `loader.sh` 在后端服务启动后调用，通过 Web 界面编辑保存到 `/home/root/init.sh`（或 `/data/init.sh`），无需 SSH 登录设备手动操作。

### 效果验证

调优后可通过以下方式验证：

```bash
# 检查 IRQ 97 是否分散到 CPU1（CPU1 列应开始递增）
cat /proc/interrupts | grep -E "22:|97:"

# 检查软中断分布（NET_RX/NET_TX 行应双核均衡）
cat /proc/softirqs | grep -E "NET_RX|NET_TX"

# 观察负载变化
uptime

# 确认后台巡检持续在跑，且近期确有生效（重新枚举后应看到新的 -> 记录）；
# 若长期无输出属正常（值未漂移，巡检判断无需重新写入，不代表脚本未运行）。
# 部分固件 logread 读不到 syslogd 内存缓冲区（"can't find syslogd buffer"
# 报错），此时改查 syslogd 实际落盘位置（以 `ps | grep syslogd` 里 -O 参数
# 为准，常见路径 /mnt/data/yocto.log）
logread | grep udx710-tune | tail -20
grep udx710-tune /mnt/data/yocto.log | tail -20
```

参考实测数据（中国联通 5G NR n78，100 MHz 单载波，SINR ~10 dB）：

| 指标 | 调优前 | 调优后 |
|------|--------|--------|
| 下行峰值 | 200–400 Mbps | 300–500 Mbps |
| 1 分钟负载 | 3.4 | 2.2 |
| NET_RX 分布 | CPU0 82% / CPU1 18% | CPU0 32% / CPU1 68% |

> ⚠️ 实际效果因信号环境、基站负载、运营商等因素而异，以上数据仅供参考。

---

## ⚠️ 已知问题（UDX710 平台）

### CDC-NCM 模式下 usb0 网络间歇性发送冻结

USB 模式设为 NCM 时，`usb0` 接口开机运行一段时间（通常一分钟左右）后会出现网络不可达：ARP 无响应、ping 不通，但 ADB（走独立的 USB gadget 功能，不依赖 usb0）仍可正常连接。

现场诊断确认的现象与结论：

- `usb0` 收包（RX）正常，持续有流量进来；发包（TX）完全停滞，`/proc/net/dev` 的 TX 计数器冻结不再增长，且不产生任何 err/drop 计数——是发送队列被卡住，不是物理链路、iptables 或路由问题。
- 执行 `ip link set usb0 down && ip link set usb0 up` 可立即恢复连通，但一段时间后会再次卡死，说明是可逆的软件/驱动状态问题，不是硬件损坏。
- 设备内核日志（`dmesg`）中有一个与故障时机吻合的周期性循环（约一分钟一次）：`sipa_rm`（展锐 IPA 硬件资源管理器）反复出现 `SIPA_RM_RES_PROD_IPA` / `SIPA_RM_RES_CONS_WWAN_UL` / `SIPA_RM_RES_CONS_WWAN_DL` 状态切换和 `SIPA LEAVE FLOWCTRL`。
- 已排除本项目代码导致：停止 `udx710` 后端进程（含其 `data_connection_watchdog`）后，上述周期仍照常发生，确认是展锐基带/RIL 固件自身的行为，不是应用层触发的。
- 结合 UDX710 内核源码（[strongtz/linux-sprd](https://github.com/strongtz/linux-sprd) 的 `drivers/staging/sprd/sipa/sipa_usb_cons.c`）分析，`usb0` 作为 IPA 资源管理器的 `SIPA_RM_RES_CONS_USB` 消费者注册，其资源释放事件（`SIPA_RM_EVT_RELEASED`）回调是空实现；硬件加速通道与 CPU 发送路径之间的交接缺陷是比较可能的方向，但未在闭源驱动二进制层面进一步定位。
- 临时关闭 SFP 硬件转发加速（`/proc/net/sfp/enable`）只能推迟故障出现的时间（约从一分钟延长到一分半），不能根治，说明单靠关闭 SFP 不足以避开问题。

目前没有可在应用层（本项目代码）根治的方案，这是设备固件/闭源内核驱动层面的问题。恩山无线论坛「4G 5G CPE」板块有用户反馈过同类「切换 USB 模式后 SIPA 通道不通、ADB/网络失联」的现象，也有反馈 RNDIS 模式相较 NCM/ECM 更稳定，可作为规避思路，但未经本项目验证。

---

## 📦 依赖

- **zbus 5.x** - D-Bus 客户端
- **tokio 1.48** - 异步运行时
- **axum 0.8** - Web 框架
- **rusqlite 0.32** - SQLite (bundled)
- **tower-http 0.6** - HTTP 中间件

---

## license 许可证

GNU General Public License v3.0
