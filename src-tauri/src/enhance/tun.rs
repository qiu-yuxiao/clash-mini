use serde_yaml_ng::{Mapping, Value};

#[cfg(target_os = "macos")]
use clash_verge_logging::{Type as LogType, logging};
#[cfg(target_os = "macos")]
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use tauri::async_runtime::JoinHandle;

#[cfg(target_os = "macos")]
use crate::process::AsyncHandler;

// M2-13: 追踪 macOS DNS 切换任务，快速切换 TUN 时 abort 上一个避免交叉执行
#[cfg(target_os = "macos")]
static DNS_TASK_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// M2-13: 中止上一个 DNS 切换任务（macOS 专用）
#[cfg(target_os = "macos")]
fn abort_prev_dns_task() {
    if let Some(handle) = dns_lock().take() {
        handle.abort();
    }
}

/// 获取 DNS 任务锁；中毒时记日志并恢复，防止 macOS 系统 DNS 残留不一致状态
#[cfg(target_os = "macos")]
fn dns_lock() -> std::sync::MutexGuard<'static, Option<JoinHandle<()>>> {
    DNS_TASK_HANDLE.lock().unwrap_or_else(|e| {
        logging!(warn, LogType::System, "DNS 任务锁被中毒线程污染，恢复后继续执行");
        e.into_inner()
    })
}

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

// if key not exists then append value
#[allow(unused_macros)]
macro_rules! append {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        if !$map.contains_key(&ret_key) {
            $map.insert(ret_key, Value::from($val));
        }
    };
}

pub fn use_tun(mut config: Mapping, enable: bool) -> Mapping {
    let tun_key = Value::from("tun");
    let tun_val = config.get(&tun_key);
    let mut tun_val = tun_val.map_or_else(Mapping::new, |val| {
        val.as_mapping().cloned().unwrap_or_else(Mapping::new)
    });

    if enable {
        // 【上游语义】TUN 开启时接管 DNS：仅当 enhanced-mode 为 fake-ip（或未设置）时，
        // 补齐 enable / ipv6 / enhanced-mode / fake-ip-range，只补缺失键，
        // 绝不整体覆盖，保留订阅或用户已提供的 nameserver 等字段。
        // fake-ip 模式下客户端立即拿到假 IP，内核按「域名→规则→转发」处理，
        // 不依赖 redir-host 那样把域名实时解析成真实 IP，TUN 下更健壮。
        let dns_key = Value::from("dns");
        let dns_val = config.get(&dns_key);
        let mut dns_val = dns_val.map_or_else(Mapping::new, |val| {
            val.as_mapping().cloned().unwrap_or_else(Mapping::new)
        });
        let ipv6_val = config
            .get(Value::from("ipv6"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let current_mode = dns_val
            .get(Value::from("enhanced-mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip");

        if current_mode == "fake-ip" || !dns_val.contains_key(Value::from("enhanced-mode")) {
            revise!(dns_val, "enable", true);
            revise!(dns_val, "ipv6", ipv6_val);
            if !dns_val.contains_key(Value::from("enhanced-mode")) {
                revise!(dns_val, "enhanced-mode", "fake-ip");
            }
            if !dns_val.contains_key(Value::from("fake-ip-range")) {
                revise!(dns_val, "fake-ip-range", "198.18.0.1/16");
            }
            if ipv6_val && !dns_val.contains_key(Value::from("fake-ip-range6")) {
                revise!(dns_val, "fake-ip-range6", "2001:2::0/64");
            }
            // 仅 macOS 下接管系统 DNS
            #[cfg(target_os = "macos")]
            {
                abort_prev_dns_task();
                let handle = AsyncHandler::spawn(move || async move {
                    crate::utils::resolve::dns::restore_public_dns().await;
                    crate::utils::resolve::dns::set_public_dns("114.114.114.114".to_string()).await;
                });
                *dns_lock() = Some(handle);
            }
        }
        revise!(config, "dns", dns_val);
    } else {
        // TUN未启用时，仅恢复系统DNS，不修改配置文件中的DNS设置
        #[cfg(target_os = "macos")]
        {
            abort_prev_dns_task();
            let handle = AsyncHandler::spawn(move || async move {
                crate::utils::resolve::dns::restore_public_dns().await;
            });
            *dns_lock() = Some(handle);
        }
    }

    // 更新TUN配置
    revise!(tun_val, "enable", enable);
    revise!(config, "tun", tun_val);

    config
}
