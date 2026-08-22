/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-12-07 07:33:11
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:46:06
 * @FilePath: /udx710-backend/backend/src/iptables.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
//! iptables 操作模块
//!
//! 提供 iptables 规则检查和清空功能

use std::process::Command;
use tokio::task;
use tracing::warn;

/// iptables 规则统计信息
#[derive(Debug, Default)]
pub struct IptablesRuleCount {
    pub ipv4_rules: usize,
    pub ipv6_rules: usize,
}

impl IptablesRuleCount {
    /// 是否有任何规则
    pub fn has_rules(&self) -> bool {
        self.ipv4_rules > 0 || self.ipv6_rules > 0
    }
    
    /// 总规则数
    pub fn total(&self) -> usize {
        self.ipv4_rules + self.ipv6_rules
    }
}

/// 获取 iptables 规则数量
///
/// 统计 iptables 和 ip6tables 中 filter 表的规则数量（排除默认策略行与本程序自己
/// 维护的规则，见 `is_managed_rule`）
///
/// # Returns
/// * `Ok(IptablesRuleCount)` - 规则统计
/// * `Err(String)` - 操作失败的错误信息
pub async fn get_iptables_rule_count() -> Result<IptablesRuleCount, String> {
    task::spawn_blocking(|| {
        let mut count = IptablesRuleCount::default();

        // 获取 iptables 规则数量
        // iptables -L -n 输出中，每条规则是一行，但需要排除链名行和策略行
        // 使用 iptables -S 更简单，每条规则一行，-P 开头的是策略，-A 开头的是规则
        if let Ok(output) = Command::new("iptables").args(["-S"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // 统计 -A 开头的行（实际规则），排除 -P（策略）和 -N（链定义）
                count.ipv4_rules = stdout.lines()
                    .filter(|line| line.starts_with("-A ") && !is_managed_rule(line))
                    .count();
            }
        }

        // 获取 ip6tables 规则数量
        if let Ok(output) = Command::new("ip6tables").args(["-S"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                count.ipv6_rules = stdout.lines()
                    .filter(|line| line.starts_with("-A ") && !is_managed_rule(line))
                    .count();
            }
        }

        Ok(count)
    })
    .await
    .map_err(|e| format!("Task execution failed: {}", e))?
}

/// 判断一条 `iptables -S` 输出行是否是本程序维护的 ttyd 端口保护规则
///
/// 计数必须排除这条规则：看门狗「有规则就清空」，而清空后它会被立即补回，算进去
/// 就会每个看门狗周期 flush 一次，并在补回前留下一个无保护窗口。
///
/// 只认端口与动作，不匹配 `! -i lo`：`-S` 会补上 `-m tcp` 等匹配器，接口取反的
/// 写法也随 iptables 版本而异（`! -i lo` / `-i ! lo`），匹配上去反而容易漏判。
/// 用户若自己写了同样丢弃该端口的规则，一并排除也符合本意。
fn is_managed_rule(rule: &str) -> bool {
    rule.contains(&format!("--dport {}", TTYD_PORT)) && rule.contains("-j DROP")
}

/// 清空所有 iptables 规则
///
/// 执行等同于 `iptables -F` 的操作，清空 filter 表的所有链
///
/// # Returns
/// * `Ok(())` - 成功清空规则
/// * `Err(String)` - 操作失败的错误信息
///
/// # 说明
/// 此函数会清空以下链的规则：
/// - INPUT 链
/// - FORWARD 链
/// - OUTPUT 链
///
/// 清空后会立即补回 ttyd 端口保护规则（见 `ensure_ttyd_port_protected`），
/// 该规则是安全不变量而非网络配置，不应被「恢复干净网络状态」的操作带走。
pub async fn flush_iptables() -> Result<(), String> {
    let result = task::spawn_blocking(|| {
        // 清空 filter 表的所有规则
        let outputv4 = Command::new("iptables")
            .arg("-F")
            .output()
            .map_err(|e| format!("Failed to execute ip6tables: {}", e))?;
        if !outputv4.status.success()  {
            let stderr = String::from_utf8_lossy(&outputv4.stderr);
            return Err(format!("iptables -F failed: {}", stderr));
        }
        let outputv6 = Command::new("ip6tables")
            .arg("-F")
            .output()
            .map_err(|e| format!("Failed to execute ip6tables: {}", e))?;
        if !outputv6.status.success()  {
            let stderr = String::from_utf8_lossy(&outputv6.stderr);
            return Err(format!("ip6tables -F failed: {}", stderr));
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("Task execution failed: {}", e))?;

    ensure_ttyd_port_protected().await;

    result
}

/// ttyd 监听端口
const TTYD_PORT: u16 = crate::terminal_proxy::TTYD_PORT;

/// 确保存在「丢弃非回环接口发往 ttyd 端口的流量」的 INPUT 规则
///
/// 与写入 ttyd 启动脚本的 `-i lo` 互为兜底：外部 start.sh 格式无法识别时参数注入
/// 会静默跳过，这条规则仍能挡住来自局域网的直连。规则已存在时不重复插入；失败只记
/// 日志不影响主流程（设备可能没有 iptables 或缺少相应内核模块）。
pub async fn ensure_ttyd_port_protected() {
    let result = task::spawn_blocking(|| {
        let port = TTYD_PORT.to_string();
        let args = [
            "INPUT",
            "!",
            "-i",
            "lo",
            "-p",
            "tcp",
            "--dport",
            port.as_str(),
            "-j",
            "DROP",
        ];

        for binary in ["iptables", "ip6tables"] {
            // -C 查询规则是否已存在；不支持 -C 的实现返回非零，此时直接插入
            let exists = Command::new(binary)
                .arg("-C")
                .args(args)
                .output()
                .is_ok_and(|output| output.status.success());

            if exists {
                continue;
            }

            match Command::new(binary).arg("-I").args(args).output() {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("{} -I INPUT failed: {}", binary, stderr.trim()));
                }
                Err(e) => return Err(format!("Failed to execute {}: {}", binary, e)),
            }
        }

        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!(error = %e, port = TTYD_PORT, "Failed to protect ttyd port"),
        Err(e) => warn!(error = %e, "ttyd port protection task panicked"),
    }
}

/// 清空所有 iptables 规则（包括 nat 和 mangle 表）
///
/// 执行更完整的清空操作，清空 filter、nat、mangle 表的所有规则
///
/// # Returns
/// * `Ok(())` - 成功清空规则
/// * `Err(String)` - 操作失败的错误信息
#[allow(dead_code)]
pub async fn flush_all_iptables() -> Result<(), String> {
    task::spawn_blocking(|| {
        let tables = ["filter", "nat", "mangle"];
        
        for table in &tables {
            let output = Command::new("iptables")
                .arg("-t")
                .arg(table)
                .arg("-F")
                .output()
                .map_err(|e| format!("Failed to execute iptables for table {}: {}", table, e))?;

            if !output.status.success() {
                // 如果表不存在或不支持，继续处理下一个表（某些表可能不存在）
                // 静默处理，不输出警告
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("Task execution failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 root 权限，默认忽略
    async fn test_flush_iptables() {
        let result = flush_iptables().await;
        assert!(result.is_ok());
    }
}

