use super::CoreManager;
use crate::{
    config::{Config, ConfigType, runtime::IRuntime},
    constants::timing,
    core::{
        handle,
        validate::{CoreConfigValidator, ValidationOutcome, ValidationSkipReason},
    },
    utils::{dirs, help},
};
use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use std::{collections::HashSet, path::PathBuf, time::Instant};
use tauri_plugin_mihomo::Error as MihomoError;

impl CoreManager {
    pub async fn use_default_config(&self, error_key: &str, error_msg: &str) -> Result<()> {
        use crate::constants::files::RUNTIME_CONFIG;

        let runtime_path = dirs::app_home_dir()?.join(RUNTIME_CONFIG);
        let clash_config = &Config::clash().await.latest_arc().0;

        Config::runtime().await.edit_draft(|d| {
            *d = IRuntime {
                config: Some(clash_config.to_owned()),
                exists_keys: HashSet::new(),
                chain_logs: Default::default(),
            }
        });

        help::save_yaml(&runtime_path, &clash_config, Some("# Clash Verge Runtime"))?;
        handle::Handle::notice_message(error_key, error_msg);
        Ok(())
    }

    pub async fn update_config_forced(&self) -> Result<ValidationOutcome> {
        self.update_config_with_force(true).await
    }

    pub async fn update_config_with_force(&self, force: bool) -> Result<ValidationOutcome> {
        if handle::Handle::global().is_exiting() {
            return Ok(ValidationOutcome::Skipped {
                reason: ValidationSkipReason::Exiting,
            });
        }

        if !self.try_start_config_update() {
            logging!(info, Type::Core, "配置更新正在进行中，跳过本次请求");
            return Ok(ValidationOutcome::Busy);
        }
        scopeguard::defer! {
            self.finish_config_update();
        }

        if !force && !self.should_update_config() {
            logging!(debug, Type::Core, "Skipping config update due to debounce");
            return Ok(ValidationOutcome::Skipped {
                reason: ValidationSkipReason::Debounced,
            });
        }

        if force {
            self.set_last_update(Instant::now());
        }

        self.perform_config_update().await
    }

    pub async fn update_config_checked(&self) -> Result<()> {
        let outcome = self.update_config_forced().await?;
        if outcome.is_valid() {
            Ok(())
        } else {
            Err(anyhow!("{outcome}"))
        }
    }

    fn should_update_config(&self) -> bool {
        let now = Instant::now();
        let last = self.get_last_update();

        if let Some(last_time) = last
            && now.duration_since(*last_time) < timing::CONFIG_UPDATE_DEBOUNCE
        {
            return false;
        }

        self.set_last_update(now);
        true
    }

    async fn perform_config_update(&self) -> Result<ValidationOutcome> {
        if let Err(err) = Config::generate().await {
            let message: String = err.to_string().into();
            Config::runtime().await.discard();
            return Ok(ValidationOutcome::invalid_from_message(message));
        }

        self.apply_generate_config().await
    }

    pub async fn apply_generate_config(&self) -> Result<ValidationOutcome> {
        match CoreConfigValidator::global().validate_config_outcome().await {
            Ok(outcome) if outcome.is_valid() => {
                let run_path = Config::generate_file(ConfigType::Run).await?;
                self.apply_config(run_path).await?;
                Ok(ValidationOutcome::Valid)
            }
            Ok(outcome) => {
                Config::runtime().await.discard();
                Ok(outcome)
            }
            Err(e) => {
                Config::runtime().await.discard();
                Err(e)
            }
        }
    }

    async fn apply_config(&self, path: PathBuf) -> Result<()> {
        let path = dirs::path_to_str(&path)?;

        // 【关键修复】与 restart_core 路径对称：reload_config(force=true) 会重置 Selector 组
        // 的 now 到列表首个节点，必须在调用前快照、调用后恢复，否则会导致前端显示节点
        // 与 mihomo 实际选路节点不一致（"前端显示日本aw2但实际走列表首个真实节点"），
        // 引发应用层断流（YouTube 不通）但 UI 仍显示健康节点的诡异现象。
        // 历史背景：8b483a7b 提交引入 restore_proxy_group_now 但只覆盖 restart_core 路径；
        // v2.6.4 伪节点过滤修复使首个节点变为真实节点后，该漏洞显化。
        let saved_proxy_now = self.snapshot_proxy_group_now().await;

        match self.reload_config(path).await {
            Ok(_) => {
                Config::runtime().await.apply();
                logging!(info, Type::Core, "Configuration applied");

                // 恢复 PROXY 组的节点选择（与 restart_core 路径对称）
                if let Some(node) = saved_proxy_now {
                    self.restore_proxy_group_now(&node).await;
                }

                // 【强咬合防线】通知前端获取重载后的最新节点和内核客观配置
                crate::core::handle::Handle::refresh_clash();
                Ok(())
            }
            Err(err) => {
                logging!(
                    warn,
                    Type::Core,
                    "Failed to apply configuration by mihomo api, restart core to apply it, error msg: {err}"
                );
                match self.restart_core().await {
                    Ok(_) => {
                        Config::runtime().await.apply();
                        logging!(info, Type::Core, "Configuration applied after restart");
                        // restart_core 内部已做 snapshot + restore，无需重复恢复
                        Ok(())
                    }
                    Err(err) => {
                        logging!(error, Type::Core, "Failed to restart core: {}", err);
                        Config::runtime().await.discard();
                        Err(anyhow!("Failed to apply config: {}", err))
                    }
                }
            }
        }
    }

    async fn reload_config(&self, path: &str) -> Result<(), MihomoError> {
        let mihomo = handle::Handle::mihomo().await.clone();
        mihomo.reload_config(true, path).await
    }
}
