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
        // TUN 启用时，DNS 由 apply_mandatory_dns_settings 统一配置为 redir-host，
        // 此处不再强制 fake-ip（原 fake-ip 分支会被流水线末端的 redir-host 覆盖，属死代码）。
        // 仅 macOS 下接管系统 DNS。
        #[cfg(target_os = "macos")]
        {
            abort_prev_dns_task();
            let handle = AsyncHandler::spawn(move || async move {
                crate::utils::resolve::dns::restore_public_dns().await;
                crate::utils::resolve::dns::set_public_dns("114.114.114.114".to_string()).await;
            });
            *dns_lock() = Some(handle);
        }
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
