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
pub fn is_dummy_node(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("流量")
        || lower.contains("过期时间")
        || lower.contains("网址")
        || lower.contains("官网")
        || lower.contains("剩余")
        || lower.contains("expire")
        || lower.contains("traffic")
        || lower.contains("website")
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("套餐到期")
        || lower.contains("续费")
        || lower.contains("公告")
        || lower.contains("购买")
        || lower.contains("subscribe")
        || lower.contains("群")
}
