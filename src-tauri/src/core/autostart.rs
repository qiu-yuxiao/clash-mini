use crate::{config::Config, core::handle::Handle};
use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use std::os::windows::process::CommandExt as _;
use tauri::Manager as _;
use tauri_plugin_autostart::AutoLaunchManager;

/// 历史计划任务名（2025-12-26 起使用的 schtasks 方案）。
/// 卸载脚本从不清理它们（installer.nsi 全文无 schtasks），老用户升级后任务会残留并
/// 继续随登录拉起 —— 表现为「自启开关关着，开机却照样启动」。
const LEGACY_TASK_NAMES: [&str; 2] = ["Clash Mini", "Clash Mini (Admin)"];
/// 迁移完成标记文件（放在 app_home 下，与配置数据同生命周期）。
const MIGRATION_MARKER: &str = ".autostart_task_migrated";
/// 两档迁移标记：全部清净写 `done`；仅存在普通权限删不掉的 Admin 任务时写 `pending_admin` 留待提权时补删（BUG-414）。
const MARKER_DONE: &str = "done";
const MARKER_PENDING_ADMIN: &str = "pending_admin";
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Update the application auto-launch configuration.
///
/// 实现：注册表 `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`
/// （由已注册的 `tauri-plugin-autostart` 提供；plugin 构建时注入 `--silent` 参数）。
///
/// WARNING: DO NOT query `Config::verge().await.latest_arc().enable_auto_launch` to determine the target state during a configuration patch.
/// Doing so causes a timing race condition (the draft config hasn't been applied yet, so it reads the old state, causing toggling ON to disable, and toggling OFF to enable).
/// ALWAYS pass the target state (`enable_auto_launch`) from the patch.
pub async fn update_launch(enable_auto_launch: Option<bool>) -> Result<()> {
    let is_enable = match enable_auto_launch {
        Some(val) => val,
        None => Config::verge().await.latest_arc().enable_auto_launch.unwrap_or(false),
    };
    logging!(info, Type::System, "Setting auto-launch enabled state to: {is_enable}");

    // 用 try_state 而非 autolaunch()：setup_autostart 失败时只记日志、应用继续运行，
    // 此时插件 state 未注册，autolaunch() 会直接 panic。这里取不到就返回错误，由调用方降级。
    let handle = Handle::app_handle();
    let Some(manager) = handle.try_state::<AutoLaunchManager>() else {
        return Err(anyhow!(
            "autostart 插件未就绪（setup_autostart 未成功），跳过自启状态写入"
        ));
    };

    if is_enable {
        // enable 幂等（set_value 覆盖写），可无条件执行
        manager.enable()?;
    } else {
        // disable 底层是 delete_value，值不存在会直接报错。
        // 启动对账每次都会走到这里，必须先判断再删，否则每次开机都刷一条错误日志。
        match manager.is_enabled() {
            Ok(false) | Err(_) => {
                // 已经是关的（或读不到就当关的）：无需动作
            }
            Ok(true) => manager.disable()?,
        }
    }

    Ok(())
}

pub fn get_launch_status() -> Result<bool> {
    let handle = Handle::app_handle();
    let Some(manager) = handle.try_state::<AutoLaunchManager>() else {
        return Err(anyhow!(
            "autostart 插件未就绪（setup_autostart 未成功），无法读取自启状态"
        ));
    };

    let status = manager.is_enabled()?;
    logging!(info, Type::System, "Auto-launch status (registry Run): {status}");
    Ok(status)
}

/// 启动时把「真实的自启状态」对齐到「配置」。
///
/// 【为什么必须有】旧实现只在用户拨动开关时才写自启状态，启动流程里没有任何对账步骤，
/// 于是残留的计划任务（或残留的注册表值）会一直生效，而开关因为只绑配置值而显示「关」。
/// 这里在每次启动后无条件以配置为准重写一次真实状态，从根上消除「配置与事实不一致」。
pub async fn reconcile_on_startup() -> Result<()> {
    cleanup_legacy_scheduled_tasks();

    // 传 None：以当前配置值作为目标状态，纯对齐，不改变用户选择
    update_launch(None).await
}

fn should_skip_cleanup_from_marker(marker_content: Option<&str>, is_admin: bool) -> bool {
    match marker_content.map(str::trim) {
        Some(MARKER_DONE) => true,
        Some(MARKER_PENDING_ADMIN) => !is_admin,
        _ => false,
    }
}

/// 检查 `%SystemRoot%\System32\Tasks\<name>` 是否明确不存在。
/// 仅当 `Tasks` 父目录本身可读存在且子条目返回 `NotFound` 时才判定为不存在；
/// 遇到权限拒绝或无法判断时保守返回 `false`，交由 `schtasks` 处理（BUG-414）。
fn is_task_file_definitely_absent(tasks_dir: Option<&std::path::Path>, task_name: &str) -> bool {
    let Some(dir) = tasks_dir else {
        return false;
    };
    if !dir.is_dir() {
        return false;
    }
    matches!(
        std::fs::symlink_metadata(dir.join(task_name)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound
    )
}

/// 迁移：清掉历史 schtasks 计划任务。
///
/// 必须在**切到注册表方案之后**执行，否则老任务会与新注册表值同时生效、重复拉起。
/// 不做提权（不弹 UAC）：用户级任务普通权限即可删除；若存在普通权限删不掉的 Admin 级任务，
/// 写入 `pending_admin` 标记避免普通启动反复拉起 `schtasks`，待管理员启动或安装器清理后再落 `done`。
fn cleanup_legacy_scheduled_tasks() {
    let Ok(home) = crate::utils::dirs::app_home_dir() else {
        return;
    };
    let is_admin = crate::utils::sysinfo::is_current_app_handle_admin(Handle::app_handle());
    let marker = home.join(MIGRATION_MARKER);
    let existing_marker = std::fs::read_to_string(&marker).ok();
    if should_skip_cleanup_from_marker(existing_marker.as_deref(), is_admin) {
        return;
    }

    let tasks_dir = std::env::var_os("SystemRoot")
        .filter(|v| !v.is_empty())
        .map(|root| std::path::PathBuf::from(root).join("System32").join("Tasks"));
    let tasks_dir_ref = tasks_dir.as_deref();

    let mut all_clean = true;
    for name in LEGACY_TASK_NAMES {
        if is_task_file_definitely_absent(tasks_dir_ref, name) {
            continue;
        }

        let output = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                logging!(info, Type::Setup, "已清理历史自启计划任务（迁移至注册表方案）: {name}");
            }
            Ok(_) if is_task_file_definitely_absent(tasks_dir_ref, name) => {
                logging!(debug, Type::Setup, "历史自启计划任务已不存在，跳过: {name}");
            }
            Ok(out) => {
                all_clean = false;
                logging!(
                    warn,
                    Type::Setup,
                    "历史自启计划任务暂无法删除（可能需管理员权限，留待提权启动或安装器清理）: {name} (exit={:?})",
                    out.status.code()
                );
            }
            Err(err) => {
                all_clean = false;
                logging!(
                    warn,
                    Type::Setup,
                    "清理历史自启计划任务时无法执行 schtasks: {name} - {err}"
                );
            }
        }
    }

    let next_marker = if all_clean { MARKER_DONE } else { MARKER_PENDING_ADMIN };
    if let Err(err) = std::fs::write(&marker, next_marker.as_bytes()) {
        logging!(warn, Type::Setup, "写入自启迁移标记失败（下次启动会重试清理）: {err}");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_migration_marker_respects_pending_admin_and_task_presence() {
        assert!(!should_skip_cleanup_from_marker(None, false));
        assert!(!should_skip_cleanup_from_marker(Some("1"), false));
        assert!(should_skip_cleanup_from_marker(Some(MARKER_DONE), false));
        assert!(should_skip_cleanup_from_marker(Some(MARKER_DONE), true));
        assert!(should_skip_cleanup_from_marker(Some(MARKER_PENDING_ADMIN), false));
        assert!(!should_skip_cleanup_from_marker(Some(MARKER_PENDING_ADMIN), true));

        let temp_dir = std::env::temp_dir().join(format!("clash_mini_task_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        assert!(is_task_file_definitely_absent(Some(&temp_dir), "MissingTask"));
        let existing_task = temp_dir.join("ExistingTask");
        std::fs::write(&existing_task, b"<Task/>").unwrap();
        assert!(!is_task_file_definitely_absent(Some(&temp_dir), "ExistingTask"));
        assert!(!is_task_file_definitely_absent(None, "MissingTask"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
