/// 识别广告/假节点
///
/// 订阅源常会在节点列表里塞入伪节点（"网址"、"官网"、"剩余流量"、"过期时间"、
/// "套餐到期"、"续费"、"购买"、"群"、"公告"、URL 形式等），
/// 这些伪节点不能进入 PROXY 组，否则：
/// - 内核 reload_config 时可能把 `now` 重置到列表首个伪节点
/// - 后台监测识别为异常触发自愈 → 切回真节点 → 下次 reload 又被重置 → 死循环
///
/// 此函数由 monitor.rs、enhance/mod.rs、lifecycle.rs 共享，确保从配置生成、
/// 自愈判定到节点恢复的全链路都剔除伪节点。
///
/// 【修正】中文关键词从 `contains` 改为「归一化后 starts_with」，避免误杀真实节点
///（如 "HK-流量优化"、"CN2-购买入口" 等）被错误过滤导致用户可用节点丢失；
/// 同时修复此前 `starts_with` 引入的假阴性——伪节点用括号/书名号包裹或带
/// "节点-" 通用前缀（如 "【剩余流量】"、"（购买入口）"、"节点-购买入口"）时，
/// 广告短语不在最开头会漏进 PROXY。归一化先剥包裹符号与通用前缀再判定。
pub fn is_dummy_node(name: &str) -> bool {
    let lower = name.to_lowercase();
    // 高置信度：包含 URL scheme 或明确的英文广告词
    if lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("expire")
        || lower.contains("traffic")
        || lower.contains("website")
        || lower.contains("subscribe")
    {
        return true;
    }
    // 归一化：剥掉包裹括号/书名号、通用前缀，让广告短语回到开头再判定
    let normalized = normalize_dummy_name(&lower);

    // 中文广告短语：要求完全匹配或以该短语开头。
    // 广告节点通常以这些词开头（如"剩余流量：100GB"），而真实节点很少如此。
    let ad_phrases = [
        "流量",
        "过期时间",
        "网址",
        "官网",
        "剩余",
        "套餐到期",
        "续费",
        "公告",
        "购买",
        "群",
    ];
    for phrase in &ad_phrases {
        if normalized == *phrase || normalized.starts_with(phrase) {
            return true;
        }
    }
    false
}

/// 把节点名归一化，便于检测「广告短语不在最开头」的伪节点。
/// 1. 剥掉两端的空白与包裹符号（【】()（）[]「」），可多层；
/// 2. 剥掉通用前缀「节点-」「节点：」等（"节点-购买入口" 这类纯伪节点）。
/// 不剥区域/协议前缀（如 "CN2-"、"HK-"），以免误杀真实节点。
fn normalize_dummy_name(name: &str) -> String {
    let mut s = name.trim().to_string();
    let wrappers = [('【', '】'), ('(', ')'), ('（', '）'), ('[', ']'), ('「', '」')];
    loop {
        let before = s.len();
        for &(open, close) in &wrappers {
            if s.starts_with(open) && s.ends_with(close) && s.len() >= open.len_utf8() + close.len_utf8() {
                let start = open.len_utf8();
                let end = s.len() - close.len_utf8();
                s = s[start..end].to_string();
                s = s.trim().to_string();
                break;
            }
        }
        if s.len() == before {
            break;
        }
    }
    if let Some(rest) = s
        .strip_prefix("节点-")
        .or_else(|| s.strip_prefix("节点:"))
        .or_else(|| s.strip_prefix("节点："))
        .or_else(|| s.strip_prefix("节点 "))
    {
        s = rest.trim().to_string();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::is_dummy_node;

    #[test]
    fn test_dummy_node_detection() {
        // 应被识别为广告/假节点的典型命名
        assert!(is_dummy_node("剩余流量：100GB"));
        assert!(is_dummy_node("过期时间：2024-12-31"));
        assert!(is_dummy_node("官网 https://example.com"));
        assert!(is_dummy_node("套餐到期"));
        assert!(is_dummy_node("续费链接"));
        assert!(is_dummy_node("公告栏"));
        assert!(is_dummy_node("购买地址"));
        assert!(is_dummy_node("群：123456"));
        assert!(is_dummy_node("expire soon"));
        assert!(is_dummy_node("traffic left"));
        assert!(is_dummy_node("website"));
        assert!(is_dummy_node("http://example.com"));
        assert!(is_dummy_node("https://example.com"));
        assert!(is_dummy_node("subscribe now"));
    }

    #[test]
    fn test_dummy_node_false_positives() {
        // 真实节点不应被误杀
        assert!(!is_dummy_node("HK-流量优化-01"));
        assert!(!is_dummy_node("日本群组-A"));
        assert!(!is_dummy_node("CN2-购买入口"));
        assert!(!is_dummy_node("BGP-公告频道"));
        assert!(!is_dummy_node("新加坡-官网加速"));
        assert!(!is_dummy_node("美国-网址专线"));
        assert!(!is_dummy_node("韩国-剩余节点-01"));
        assert!(!is_dummy_node("台湾-套餐到期提醒")); // 虽然包含关键词，但不是以关键词开头
        assert!(!is_dummy_node("香港-续费专线"));
        assert!(!is_dummy_node("日本-过期时间测试"));
        assert!(!is_dummy_node("上海-群节点-01"));
    }

    #[test]
    fn test_dummy_node_leak_side() {
        // 广告短语被包裹/加前缀时也应被识别，修复此前 starts_with 导致的漏判
        assert!(is_dummy_node("【剩余流量】"));
        assert!(is_dummy_node("【剩余流量：100GB】"));
        assert!(is_dummy_node("(购买入口)"));
        assert!(is_dummy_node("（购买入口）"));
        assert!(is_dummy_node("[官网]"));
        assert!(is_dummy_node("「公告频道」"));
        assert!(is_dummy_node("节点-购买入口"));
        assert!(is_dummy_node("节点：剩余流量"));
        // 多层包裹
        assert!(is_dummy_node("【(剩余流量)】"));
    }
}
