use crate::{config::Config, core::handle, process::AsyncHandler};

use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;

/// 互斥保护 change_clash_mode / patch_clash 的并发调用，防止配置修改竞态
pub static CLASH_PATCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn after_change_clash_mode() {
    // M2-03: 直接调用 close_all_connections，避免逐个关闭数千连接时阻塞
    AsyncHandler::spawn(move || async {
        let mihomo = handle::Handle::mihomo().await.clone();
        if let Err(err) = mihomo.close_all_connections().await {
            logging!(error, Type::Core, "Failed to close all connections: {err}");
        }
    });
}

/// Change Clash mode (rule/global/direct/script)
pub async fn change_clash_mode(mode: String) -> anyhow::Result<()> {
    let _guard = CLASH_PATCH_LOCK.lock().await;

    let mut mapping = Mapping::new();
    mapping.insert(Value::from("mode"), Value::from(mode.as_str()));
    let json_value = serde_json::json!({
        "mode": mode
    });
    logging!(debug, Type::Core, "change clash mode to {mode}");
    let mihomo = handle::Handle::mihomo().await.clone();
    match mihomo.patch_base_config(&json_value).await {
        Ok(_) => {
            let clash = Config::clash().await;
            let old_config = clash.latest_arc().0.clone();
            clash.edit_draft(|d| d.patch_config(&mapping));
            clash.apply();

            let clash_data = clash.data_arc();
            if let Err(save_err) = clash_data.save_config() {
                logging!(
                    error,
                    Type::Core,
                    "保存 Clash 配置到文件失败，回滚内存 Draft: {}",
                    save_err
                );
                // 与 patch_clash 对齐：落盘失败时回滚内存 Draft，保证 draft == 文件 == 旧值
                clash.edit_draft(|d| d.0 = old_config);
                clash.apply();
                return Err(save_err);
            }

            handle::Handle::refresh_clash();

            let is_auto_close_connection = Config::verge().await.data_arc().auto_close_connection.unwrap_or(false);
            if is_auto_close_connection {
                after_change_clash_mode();
            }

            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Core, "{err}");
            Err(anyhow::anyhow!("{err}"))
        }
    }
}
