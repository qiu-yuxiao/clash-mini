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

/// 迁移：清掉历史 schtasks 计划任务（只做一次，成功与否都打标记）。
///
/// 必须在**切到注册表方案之后**执行，否则老任务会与新注册表值同时生效、重复拉起。
/// 不做提权（不弹 UAC）：用户级任务普通权限即可删除；Admin 级任务删不掉只记日志，
/// 绝不为了清理而打断启动流程或弹出 UAC。
fn cleanup_legacy_scheduled_tasks() {
    let Some(home) = crate::utils::dirs::app_home_dir().ok() else {
        return;
    };
    let marker = home.join(MIGRATION_MARKER);
    if marker.exists() {
        return;
    }

    for name in LEGACY_TASK_NAMES {
        let output = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                logging!(info, Type::Setup, "已清理历史自启计划任务（迁移至注册表方案）: {name}");
            }
            Ok(_) => {
                // 任务不存在（最常见）或权限不足：均非致命错误，忽略
                logging!(debug, Type::Setup, "历史自启计划任务不存在或无法删除，跳过: {name}");
            }
            Err(err) => {
                logging!(
                    warn,
                    Type::Setup,
                    "清理历史自启计划任务时无法执行 schtasks: {name} - {err}"
                );
            }
        }
    }

    // 不论结果都落标记：避免每次启动都拉起 schtasks 子进程
    if let Err(err) = std::fs::write(&marker, b"1") {
        logging!(warn, Type::Setup, "写入自启迁移标记失败（下次启动会重试清理）: {err}");
    }
}
