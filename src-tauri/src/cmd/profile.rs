use super::CmdResult;
use super::StringifyErr as _;
use crate::core::validate::{ValidationNoticeTarget, handle_validation_notice};
use crate::feat::CURRENT_SWITCHING_PROFILE;
use crate::utils::window_manager::WindowManager;
use crate::{
    config::{
        Config, IProfiles, PrfItem, PrfOption,
        profiles::{
            profiles_append_item_with_filedata_safe, profiles_delete_item_safe, profiles_patch_item_safe,
            profiles_reorder_safe, profiles_save_file_safe,
        },
        profiles_append_item_safe,
    },
    core::{CoreManager, handle, timer::Timer, validate::ValidationOutcome},
    feat,
    utils::{dirs, help},
};
use clash_verge_draft::SharedDraft;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use smartstring::alias::String;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// 尝试获取配置切换锁，失败时记录指定的日志并返回指定的 busy 值。
/// 成功时自动 defer! 释放锁。
macro_rules! try_lock_profile_switching {
    ($log_msg:expr, $on_busy:expr) => {
        if CURRENT_SWITCHING_PROFILE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            logging!(info, Type::Cmd, $log_msg);
            return $on_busy;
        }
        defer! {
            CURRENT_SWITCHING_PROFILE.store(false, Ordering::Release);
        }
    };
}

#[tauri::command]
pub async fn get_profiles() -> CmdResult<SharedDraft<IProfiles>> {
    logging!(debug, Type::Cmd, "获取配置文件列表");
    let draft = Config::profiles().await;
    let data = draft.data_arc();
    Ok(data)
}

/// 增强配置文件
#[tauri::command]
pub async fn enhance_profiles() -> CmdResult<ValidationOutcome> {
    match feat::enhance_profiles().await {
        Ok(outcome) if outcome.is_valid() => {
            handle::Handle::refresh_clash();
            Ok(outcome)
        }
        Ok(outcome) => {
            logging!(
                warn,
                Type::Cmd,
                "Reactivate profiles command failed validation: {}",
                outcome
            );
            handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
            Ok(outcome)
        }
        Err(e) => {
            logging!(error, Type::Cmd, "{}", e);
            Err(e.to_string().into())
        }
    }
}

/// 导入配置文件
#[tauri::command]
pub async fn import_profile(url: std::string::String, option: Option<PrfOption>) -> CmdResult {
    try_lock_profile_switching!(
        "当前正在切换或更新配置，放弃导入请求",
        Err("当前正在切换或更新配置，请稍候再试".into())
    );

    logging!(info, Type::Cmd, "[导入订阅] 开始导入: {}", help::mask_url(&url));

    // 直接依赖 PrfItem::from_url 自身的超时/重试逻辑，不再使用 tokio::time::timeout 包裹
    let item = &mut match PrfItem::from_url(&url, None, None, option.as_ref()).await {
        Ok(it) => {
            logging!(info, Type::Cmd, "[导入订阅] 下载完成，开始保存配置");
            it
        }
        Err(e) => {
            logging!(error, Type::Cmd, "[导入订阅] 下载失败: {}", e);
            return Err(format!("导入订阅失败: {}", e).into());
        }
    };

    match profiles_append_item_safe(item).await {
        Ok(_) => match profiles_save_file_safe().await {
            Ok(_) => {
                logging!(info, Type::Cmd, "[导入订阅] 配置文件保存成功");
            }
            Err(e) => {
                logging!(error, Type::Cmd, "[导入订阅] 保存配置文件失败: {}", e);
            }
        },
        Err(e) => {
            logging!(error, Type::Cmd, "[导入订阅] 保存配置失败: {}", e);
            return Err(format!("导入订阅失败: {}", e).into());
        }
    }

    let final_current = Config::profiles().await.data_arc().current.clone();
    let is_current_changed = final_current.is_some() && final_current.as_ref() == item.uid.as_ref();

    if let Some(uid) = &item.uid {
        logging!(info, Type::Cmd, "[导入订阅] 发送配置变更通知: {}", uid);
        handle::Handle::notify_profile_changed(uid);
    }

    if is_current_changed {
        logging!(
            info,
            Type::Cmd,
            "[导入订阅] 自动激活首个导入的配置，开始刷新内核配置..."
        );
        crate::process::AsyncHandler::spawn(move || async move {
            if let Err(e) = CoreManager::global().update_config_forced().await {
                logging!(error, Type::Cmd, "[导入订阅] 自动刷新配置内核失败: {}", e);
            } else {
                handle::Handle::refresh_clash();
                logging!(info, Type::Cmd, "[导入订阅] 自动激活首个导入配置成功且内核重载完成");
            }
        });
    }

    logging!(info, Type::Cmd, "[导入订阅] 导入完成: {}", help::mask_url(&url));
    Ok(())
}

/// 调整profile的顺序
#[tauri::command]
pub async fn reorder_profile(active_id: String, over_id: String) -> CmdResult {
    // M2-06: 防止切换期间修改 profiles 结构
    try_lock_profile_switching!(
        "当前正在切换配置，排序请求被拒绝",
        Err("当前正在切换或更新配置，请稍候再试".into())
    );
    match profiles_reorder_safe(&active_id, &over_id).await {
        Ok(_) => {
            logging!(info, Type::Cmd, "重新排序配置文件");
            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Cmd, "重新排序配置文件失败: {}", err);
            Err(format!("重新排序配置文件失败: {}", err).into())
        }
    }
}

/// 创建新的profile
/// 创建一个新的配置文件
#[tauri::command]
pub async fn create_profile(item: PrfItem, file_data: Option<String>) -> CmdResult {
    // M2-06: 防止切换期间修改 profiles 结构
    try_lock_profile_switching!(
        "当前正在切换配置，创建订阅请求被拒绝",
        Err("当前正在切换或更新配置，请稍候再试".into())
    );
    match profiles_append_item_with_filedata_safe(&item, file_data).await {
        Ok(_) => {
            profiles_save_file_safe().await.stringify_err()?;
            // 发送配置变更通知
            if let Some(uid) = &item.uid {
                logging!(info, Type::Cmd, "[创建订阅] 发送配置变更通知: {}", uid);
                handle::Handle::notify_profile_changed(uid);
            }
            Ok(())
        }
        Err(err) => match err.to_string().as_str() {
            "the file already exists" => Err("the file already exists".into()),
            _ => Err(format!("add profile error: {err}").into()),
        },
    }
}

#[tauri::command]
pub async fn update_profile(index: String, option: Option<PrfOption>) -> CmdResult {
    try_lock_profile_switching!(
        "当前正在切换或更新配置，放弃更新请求",
        Err("当前正在切换或更新配置，请稍候再试".into())
    );

    match feat::update_profile(&index, option.as_ref(), true, true, true).await {
        Ok(_) => Ok(()),
        Err(e) => {
            logging!(error, Type::Cmd, "{}", e);
            Err(e.to_string().into())
        }
    }
}

/// 删除配置文件
#[tauri::command]
pub async fn delete_profile(index: String) -> CmdResult {
    // M2-06: 防止切换期间删除当前活跃 Profile
    try_lock_profile_switching!(
        "当前正在切换配置，删除订阅请求被拒绝",
        Err("当前正在切换或更新配置，请稍候再试".into())
    );
    // 使用Send-safe helper函数
    let should_update = profiles_delete_item_safe(&index).await.stringify_err()?;
    profiles_save_file_safe().await.stringify_err()?;
    if should_update {
        match CoreManager::global().update_config_forced().await {
            Ok(outcome) if outcome.is_valid() => {
                handle::Handle::refresh_clash();
                // 发送配置变更通知
                let new_current = Config::profiles().await.data_arc().current.clone().unwrap_or_default();
                logging!(info, Type::Cmd, "[删除订阅] 发送配置变更通知: {}", new_current);
                handle::Handle::notify_profile_changed(&new_current);
            }
            Ok(outcome) => {
                logging!(warn, Type::Cmd, "删除订阅后更新配置失败: {}", outcome);
                handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
                return Err(outcome.to_string().into());
            }
            Err(e) => {
                logging!(error, Type::Cmd, "{}", e);
                return Err(e.to_string().into());
            }
        }
    }
    Timer::global().refresh().await.stringify_err()?;
    Ok(())
}

/// 执行配置更新并处理结果
async fn restore_previous_profile(prev_profile: &String) -> CmdResult<()> {
    logging!(info, Type::Cmd, "尝试恢复到之前的配置: {}", prev_profile);
    let restore_profiles = IProfiles {
        current: Some(prev_profile.to_owned()),
        items: None,
    };
    Config::profiles()
        .await
        .edit_draft(|d| d.patch_config(&restore_profiles));
    Config::profiles().await.apply();
    if let Err(e) = profiles_save_file_safe().await {
        logging!(warn, Type::Cmd, "Warning: 异步保存恢复配置文件失败: {e}");
    }
    if let Err(e) = CoreManager::global().update_config_forced().await {
        logging!(error, Type::Cmd, "Failed to reload Clash config after restore: {e}");
    }
    logging!(info, Type::Cmd, "成功恢复到之前的配置");
    Ok(())
}

async fn handle_success(current_value: Option<&String>) -> CmdResult<ValidationOutcome> {
    Config::profiles().await.apply();
    handle::Handle::refresh_clash();

    if let Err(e) = profiles_save_file_safe().await {
        logging!(warn, Type::Cmd, "Warning: 异步保存配置文件失败: {e}");
    }

    if let Some(current) = current_value
        && WindowManager::get_main_window().is_some()
    {
        logging!(info, Type::Cmd, "向前端发送配置变更事件: {}", current);
        handle::Handle::notify_profile_changed(current);
    }

    Ok(ValidationOutcome::Valid)
}

async fn discard_and_restore(current_profile: Option<&String>) -> CmdResult<()> {
    Config::profiles().await.discard();
    if let Some(prev_profile) = current_profile {
        restore_previous_profile(prev_profile).await?;
        logging!(
            info,
            Type::Cmd,
            "配置更新失败已回滚，向前端发送配置重置变更事件: {}",
            prev_profile
        );
        handle::Handle::notify_profile_changed(prev_profile);
    }
    Ok(())
}

async fn handle_validation_failure(
    outcome: ValidationOutcome,
    current_profile: Option<&String>,
) -> CmdResult<ValidationOutcome> {
    logging!(warn, Type::Cmd, "配置验证失败: {}", outcome);
    discard_and_restore(current_profile).await?;
    handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
    Ok(outcome)
}

async fn handle_update_error<E: std::fmt::Display>(
    e: E,
    current_profile: Option<&String>,
) -> CmdResult<ValidationOutcome> {
    logging!(warn, Type::Cmd, "更新过程发生错误: {}", e,);
    discard_and_restore(current_profile).await?;
    let message: String = e.to_string().into();
    handle::Handle::notice_message("config_validate::boot_error", message.clone());
    Ok(ValidationOutcome::invalid_from_message(message))
}

async fn handle_timeout(current_profile: Option<&String>) -> CmdResult<ValidationOutcome> {
    let timeout_msg: String = "配置更新超时(30秒)，可能是配置验证或核心通信阻塞".into();
    logging!(error, Type::Cmd, "{}", timeout_msg);
    discard_and_restore(current_profile).await?;
    handle::Handle::notice_message("config_validate::timeout", timeout_msg.clone());
    Ok(ValidationOutcome::invalid_from_message(timeout_msg))
}

async fn perform_config_update(
    current_value: Option<&String>,
    current_profile: Option<&String>,
) -> CmdResult<ValidationOutcome> {
    let update_result =
        tokio::time::timeout(Duration::from_secs(30), CoreManager::global().update_config_forced()).await;

    match update_result {
        Ok(Ok(outcome)) if outcome.is_valid() => handle_success(current_value).await,
        Ok(Ok(outcome)) => handle_validation_failure(outcome, current_profile).await,
        Ok(Err(e)) => handle_update_error(e, current_profile).await,
        Err(_) => handle_timeout(current_profile).await,
    }
}

/// 修改profiles的配置
#[tauri::command]
pub async fn patch_profiles_config(profiles: IProfiles) -> CmdResult<ValidationOutcome> {
    try_lock_profile_switching!("当前正在切换配置，放弃请求", Ok(ValidationOutcome::Busy));

    let target_profile = profiles.current.as_ref();

    logging!(info, Type::Cmd, "开始修改配置文件，目标profile: {:?}", target_profile);

    // 保存当前配置，以便在验证失败时恢复
    let previous_profile = Config::profiles().await.data_arc().current.clone();
    logging!(info, Type::Cmd, "当前配置: {:?}", previous_profile);

    Config::profiles().await.edit_draft(|d| d.patch_config(&profiles));

    perform_config_update(target_profile, previous_profile.as_ref()).await
}

/// 修改某个profile item的
#[tauri::command]
pub async fn patch_profile(index: String, profile: PrfItem) -> CmdResult {
    // 保存修改前检查是否有更新 update_interval
    let profiles = Config::profiles().await;
    let should_refresh_timer = if let Ok(old_profile) = profiles.latest_arc().get_item(&index)
        && let Some(new_option) = profile.option.as_ref()
    {
        let old_interval = old_profile.option.as_ref().and_then(|o| o.update_interval);
        let new_interval = new_option.update_interval;
        let old_allow_auto_update = old_profile.option.as_ref().and_then(|o| o.allow_auto_update);
        let new_allow_auto_update = new_option.allow_auto_update;
        (old_interval != new_interval) || (old_allow_auto_update != new_allow_auto_update)
    } else {
        false
    };

    profiles_patch_item_safe(&index, &profile).await.stringify_err()?;

    // 如果更新间隔或允许自动更新变更，异步刷新定时器
    if should_refresh_timer {
        crate::process::AsyncHandler::spawn(move || async move {
            logging!(info, Type::Timer, "Timer update settings changed, refreshing timer...");
            if let Err(e) = crate::core::Timer::global().refresh().await {
                logging!(error, Type::Timer, "Failed to refresh timer: {}", e);
            } else {
                // 刷新成功后发送自定义事件，不触发配置重载
                crate::core::handle::Handle::notify_timer_updated(&index);
            }
        });
    }

    Ok(())
}

/// 查看配置文件
#[tauri::command]
pub async fn view_profile(index: String) -> CmdResult {
    let profiles = Config::profiles().await;
    let profiles_ref = profiles.latest_arc();
    let file = profiles_ref
        .get_item(&index)
        .stringify_err()?
        .file
        .as_ref()
        .ok_or("the file field is null")?;

    let path = dirs::app_profiles_dir().stringify_err()?.join(file.as_str());
    if !path.exists() {
        return CmdResult::Err(format!("file not found \"{}\"", path.display()).into());
    }

    help::open_file(path).stringify_err()
}

/// 读取配置文件内容
#[tauri::command]
pub async fn read_profile_file(index: String) -> CmdResult<String> {
    let item = {
        let profiles = Config::profiles().await;
        let profiles_ref = profiles.latest_arc();
        PrfItem {
            file: profiles_ref.get_item(&index).stringify_err()?.file.to_owned(),
            ..Default::default()
        }
    };
    let data = item.read_file().stringify_err()?;
    Ok(data)
}

/// 获取下一次更新时间
#[tauri::command]
pub async fn get_next_update_time(uid: String) -> CmdResult<Option<i64>> {
    let timer = Timer::global();
    let next_time = timer.get_next_update_time(&uid).await;
    Ok(next_time)
}
