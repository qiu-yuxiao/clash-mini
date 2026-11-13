use super::CmdResult;
use crate::{cmd::StringifyErr as _, config::IVerge, feat};
use clash_verge_draft::SharedDraft;

fn parse_verge_patch_resets(payload: &serde_json::Value) -> feat::VergePatchResets {
    let reset_tun_sniffer = payload
        .as_object()
        .and_then(|obj| obj.get("enable_tun_sniffer"))
        .is_some_and(serde_json::Value::is_null);
    feat::VergePatchResets { reset_tun_sniffer }
}

/// 获取Verge配置
#[tauri::command]
pub async fn get_verge_config() -> CmdResult<SharedDraft<IVerge>> {
    feat::fetch_verge_config().await.stringify_err()
}

/// 修改Verge配置
#[tauri::command]
pub async fn patch_verge_config(payload: serde_json::Value) -> CmdResult {
    let resets = parse_verge_patch_resets(&payload);
    let patch: IVerge = serde_json::from_value(payload).stringify_err()?;
    feat::patch_verge_with_resets(&patch, false, resets)
        .await
        .stringify_err()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn explicit_null_tun_sniffer_triggers_reset_flag() {
        let null_payload = serde_json::json!({ "enable_tun_sniffer": null });
        assert_eq!(
            parse_verge_patch_resets(&null_payload),
            feat::VergePatchResets {
                reset_tun_sniffer: true
            }
        );

        let false_payload = serde_json::json!({ "enable_tun_sniffer": false });
        assert_eq!(
            parse_verge_patch_resets(&false_payload),
            feat::VergePatchResets::default()
        );

        let omitted_payload = serde_json::json!({ "language": "zh" });
        assert_eq!(
            parse_verge_patch_resets(&omitted_payload),
            feat::VergePatchResets::default()
        );
    }
}
