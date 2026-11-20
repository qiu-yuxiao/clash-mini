use super::probe::get_filter_and_sort_config;
use crate::{
    config::{PrfItem, PrfSelected, profiles_patch_item_safe},
    utils::node::match_node_filter,
};
use clash_verge_logging::{Type, logging};

/// 判断某条活跃连接在前台用户显式切换节点至 `target_node` 后是否应被断开（BUG-396）：
/// - 仅匹配经过 `group_name`（`"PROXY"`）代理组的连接，绝不触碰 `DIRECT` / `REJECT` 直连或拦截连接；
/// - 若连接已位于 `target_node` 上（如 F4/F1 重测后最快节点未变），返回 `false` 保持零中断；
/// - 若连接仍挂在任意非 `target_node` 的历史代理节点上（含多跳前残留），返回 `true` 立即清理。
pub fn should_close_stale_proxy_connection(chains: &[String], group_name: &str, target_node: &str) -> bool {
    let target = target_node.trim();
    let group = group_name.trim();
    !target.is_empty()
        && !group.is_empty()
        && !target.eq_ignore_ascii_case("DIRECT")
        && !target.eq_ignore_ascii_case("REJECT")
        && !target.eq_ignore_ascii_case("GLOBAL")
        && chains.iter().any(|c| c == group)
        && !chains.iter().any(|c| c == target)
}

/// 判断某条活跃代理连接的真实出口节点（`chains[0]`）是否已被当前有效过滤词排除（BUG-405）：
/// - 过滤词为空或处于半输入态时返回 `false`，不触碰任何连接；
/// - 仅匹配经过 `group`（`"PROXY"`）且末端为真实代理节点的连接，绝不误伤 `DIRECT` / `REJECT`。
pub(crate) fn should_close_excluded_proxy_connection(chains: &[String], group: &str, filter_text: &str) -> bool {
    if crate::utils::node::is_incomplete_filter_input(filter_text) || !chains.iter().any(|c| c == group) {
        return false;
    }
    let Some(exit_node) = chains.first().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return false;
    };
    if exit_node.eq_ignore_ascii_case("DIRECT")
        || exit_node.eq_ignore_ascii_case("REJECT")
        || exit_node.eq_ignore_ascii_case("REJECT-DROP")
        || exit_node.eq_ignore_ascii_case("PASS")
        || exit_node.eq_ignore_ascii_case("COMPATIBLE")
        || exit_node.eq_ignore_ascii_case("PROXY")
        || exit_node.eq_ignore_ascii_case("GLOBAL")
    {
        return false;
    }
    !match_node_filter(exit_node, filter_text.trim())
}

async fn close_matching_proxy_connections<F>(mut predicate: F) -> usize
where
    F: FnMut(&[String]) -> bool,
{
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let Ok(snapshot) = mihomo.get_connections().await else {
        return 0;
    };
    let ids: Vec<String> = snapshot
        .connections
        .unwrap_or_default()
        .into_iter()
        .filter(|conn| predicate(&conn.chains))
        .map(|conn| conn.id)
        .collect();
    let count = ids.len();
    for id in ids {
        let _ = mihomo.close_connection(&id).await;
    }
    count
}

/// 前台用户显式触发选点切换后，精准清理仍挂在非 `target_node` 旧出口上的 `PROXY` 存量长连接（BUG-396）。
/// 注意：后台自愈与常规冷启动保持零断流热切换；过滤越界清理由 `close_excluded_proxy_connections` 负责。
pub async fn close_stale_proxy_connections(target_node: &str) {
    let count =
        close_matching_proxy_connections(|chains| should_close_stale_proxy_connection(chains, "PROXY", target_node))
            .await;
    if count > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[前台切点] 已精准清理 {} 条非当前目标节点（{}）的存量代理连接",
            count,
            target_node
        );
    }
}

/// 清理仍绑定在已被过滤词排除的代理节点上的存量连接（BUG-405），绝不触碰合规代理或 `DIRECT`/`REJECT`。
pub async fn close_excluded_proxy_connections(filter_text: &str) {
    let trimmed = filter_text.trim();
    if crate::utils::node::is_incomplete_filter_input(trimmed) {
        return;
    }
    let count =
        close_matching_proxy_connections(|chains| should_close_excluded_proxy_connection(chains, "PROXY", trimmed))
            .await;
    if count > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[过滤边界] 已清理 {} 条滞留在已排除节点上的代理连接（过滤词: \"{}\"）",
            count,
            trimmed
        );
    }
}

/// 判断某条活跃连接在后台自愈切离确认故障/黑洞的 `dead_node` 后是否应被断开（BUG-412）：
/// - 仅精准匹配同时经过 `group_name`（`"PROXY"`）与该故障 `dead_node` 的连接，绝不触碰 `DIRECT` / `REJECT` 与其它节点连接。
pub(crate) fn should_close_dead_node_proxy_connection(chains: &[String], group_name: &str, dead_node: &str) -> bool {
    let dead = dead_node.trim();
    let group = group_name.trim();
    !dead.is_empty()
        && !group.is_empty()
        && !dead.eq_ignore_ascii_case("DIRECT")
        && !dead.eq_ignore_ascii_case("REJECT")
        && !dead.eq_ignore_ascii_case("GLOBAL")
        && chains.iter().any(|c| c == group)
        && chains.iter().any(|c| c == dead)
}

/// 后台自愈确认原节点故障/间歇黑洞并切离后，精准断开滞留在原 `dead_node` 上的 `PROXY` 存量长连接（BUG-412），
/// 促使 HTTP/2 / gRPC 等长连接立即重建到新节点，绝不触碰 `DIRECT` / `REJECT`。
pub(super) async fn close_dead_node_proxy_connections(dead_node: &str) {
    let count =
        close_matching_proxy_connections(|chains| should_close_dead_node_proxy_connection(chains, "PROXY", dead_node))
            .await;
    if count > 0 {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 自愈切离故障节点 {}，已清理 {} 条滞留死链代理连接",
            dead_node,
            count
        );
    }
}

pub(crate) async fn persist_selected_node_if_matching(profile_uid: &str, fastest_node: &str) {
    let filter_text = get_filter_and_sort_config(profile_uid)
        .await
        .0
        .filter_text
        .trim()
        .to_string();
    if match_node_filter(fastest_node, &filter_text) {
        let new_selected = vec![PrfSelected {
            name: Some("PROXY".into()),
            now: Some(fastest_node.into()),
        }];
        let patch_item = PrfItem {
            selected: Some(new_selected),
            ..Default::default()
        };
        let uid_smart: smartstring::alias::String = profile_uid.into();
        if let Err(e) = profiles_patch_item_safe(&uid_smart, &patch_item).await {
            logging!(warn, Type::Lightweight, "[后台监测] 回写 profile.selected 失败: {e}");
        }
    } else {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 最优节点 {} 不在当前过滤范围「{}」内，跳过回写 profile.selected 以免污染",
            fastest_node,
            filter_text
        );
    }
}
