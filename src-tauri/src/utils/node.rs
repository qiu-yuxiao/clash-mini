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
/// 【修正】中文关键词从 `contains` 改为 `starts_with`，避免误杀真实节点
///（如 "HK-流量优化"、"日本群组-01" 等）被错误过滤导致用户可用节点丢失。
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
    // 中文广告短语：要求完全匹配或以该短语开头。
    // 广告节点通常以这些词开头（如"剩余流量：100GB"），而真实节点很少如此。
    let ad_phrases = [
        "流量", "过期时间", "网址", "官网", "剩余", "套餐到期", "续费", "公告", "购买", "群",
    ];
    for phrase in &ad_phrases {
        if lower == *phrase || lower.starts_with(phrase) {
            return true;
        }
    }
    false
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
}
