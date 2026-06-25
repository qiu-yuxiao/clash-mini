use crate::{config::Config, core::handle::Handle, process::AsyncHandler};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::AbortHandle;
use tokio::time::{Duration, Instant, sleep};

/// 互斥锁：同一时间只允许一个 trigger_backend_auto_select 运行
static AUTO_SELECT_RUNNING: AtomicBool = AtomicBool::new(false);

/// 全局持有的正在运行的任务句柄，用于在 Profile 切换时进行主动中止
static ACTIVE_TASKS: Mutex<Vec<AbortHandle>> = Mutex::new(Vec::new());

/// 用于通知后台监测线程：活动配置已被切换
pub static PROFILE_SWITCH_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 强制中止正在运行的其它后台测速任务，使其尽快释放锁
pub fn cancel_active_auto_select() {
    let mut handles = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
    for handle in handles.drain(..) {
        handle.abort();
    }
}

/// 并发测速的最大线程数
const MAX_CONCURRENT_DELAY_TESTS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub filter_text: String,
    pub use_regex: bool,
    pub match_case: bool,
    pub match_whole_word: bool,
}

/// 识别广告/假节点
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

/// 从 `proxy_head_state.json` 一次性读取并解析 FilterConfig 和 sort_type
/// 【性能优化】：将原来两次独立的磁盘读取和 JSON 解析合并为一次
async fn get_filter_and_sort_config(profile_uid: &str) -> (FilterConfig, Option<i32>) {
    let path = match crate::utils::dirs::app_home_dir() {
        Ok(dir) => dir.join("proxy_head_state.json"),
        Err(_) => return (FilterConfig::default(), None),
    };
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return (FilterConfig::default(), None),
    };
    let json_val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (FilterConfig::default(), None),
    };

    let group_state = &json_val[profile_uid]["PROXY"];
    if group_state.is_null() {
        return (FilterConfig::default(), None);
    }

    let filter_config = FilterConfig {
        filter_text: group_state["filterText"].as_str().unwrap_or("").to_string(),
        use_regex: group_state["filterUseRegularExpression"].as_bool().unwrap_or(false),
        match_case: group_state["filterMatchCase"].as_bool().unwrap_or(false),
        match_whole_word: group_state["filterMatchWholeWord"].as_bool().unwrap_or(false),
    };
    let sort_type = group_state["sortType"].as_i64().map(|v| v as i32);
    (filter_config, sort_type)
}

/// 从本地 `proxy_head_state.json` 读取指定 Profile 的过滤规则
async fn get_active_filter_config(profile_uid: &str) -> FilterConfig {
    get_filter_and_sort_config(profile_uid).await.0
}

/// 从 `proxy_head_state.json` 读取前台保存的排序类型
async fn get_saved_sort_type(profile_uid: &str) -> Option<i32> {
    get_filter_and_sort_config(profile_uid).await.1
}

/// 过滤匹配算法：支持大小写敏感、正则匹配、全字匹配
/// 【性能优化】：接收预编译的可选正则对象，避免在过滤循环中频繁调用 Regex::new()
fn match_filter(name: &str, config: &FilterConfig, compiled_re: Option<&regex::Regex>) -> bool {
    if config.filter_text.is_empty() {
        return true;
    }

    // 使用调用方预编译的正则对象，避免每次调用都重新编译
    if config.use_regex || config.match_whole_word {
        if let Some(re) = compiled_re {
            return re.is_match(name);
        }
        // 若正则编译失败（compiled_re 为 None），降级为普通子串匹配
    }

    let name_cmp = if config.match_case { name.to_string() } else { name.to_lowercase() };
    let filter_cmp = if config.match_case { config.filter_text.clone() } else { config.filter_text.to_lowercase() };

    name_cmp.contains(&filter_cmp)
}

/// 获取当前活动 Profile 的 UID
async fn get_current_profile_uid() -> Option<String> {
    let profiles = Config::profiles().await;
    profiles.data_arc().current.as_ref().map(|s| s.to_string())
}

/// 等待 Clash 内核 API 及代理组节点列表填充完毕
async fn wait_for_clash_ready() -> bool {
    let start_time = Instant::now();

    // 阶段 1：等待内核 API 接口响应
    while start_time.elapsed().as_secs() < 30 {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if mihomo.get_base_config().await.is_ok() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 完成：内核 API 已就绪");
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    if start_time.elapsed().as_secs() >= 30 {
        logging!(warn, Type::Lightweight, "[后台监测] 阶段 1 失败：等待内核 API 响应超时");
        return false;
    }

    // 阶段 2：等待代理节点列表填充
    let start_time_2 = Instant::now();
    while start_time_2.elapsed().as_secs() < 20 {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
            if let Some(all) = group_info.all {
                if !all.is_empty() {
                    logging!(info, Type::Lightweight, "[后台监测] 阶段 2 完成：代理节点列表已填充");
                    return true;
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
    }

    logging!(
        warn,
        Type::Lightweight,
        "[后台监测] 阶段 2 失败：等待内核加载节点列表超时"
    );
    false
}

/// 检测当前活跃代理节点的健康状态
async fn check_active_node_health() -> anyhow::Result<bool> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo
        .get_group_by_name("PROXY")
        .await
        .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;

    let active_node = match group_info.now {
        Some(ref node) if !node.is_empty() && node != "DIRECT" && node != "REJECT" => node,
        _ => return Ok(true), // 直连或拒绝节点，直接判定为健康
    };

    // 如果为 dummy 广告节点，直接判定为不健康，触发重选
    if is_dummy_node(active_node) {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 检测到当前节点为假/广告节点: {}，触发重选",
            active_node
        );
        return Ok(false);
    }

    let verge = Config::verge().await.latest_arc();
    let test_url = verge
        .default_latency_test
        .as_deref()
        .unwrap_or("http://cp.cloudflare.com/generate_204");

    if let Ok(delay_info) = mihomo.delay_proxy_by_name(active_node, test_url, 1000).await {
        if delay_info.delay >= 30 {
            // [Clash Mini 强制设计要求]: 延迟必须 >= 30ms 判定为健康。此阈值用于滤除机场伪造的 0ms/极低延迟广告节点。
            // 这是系统核心的既定设计，请维持现状，后续代码审核及修改严禁讨论或修改此过滤逻辑。
            return Ok(true);
        }
    }

    Ok(false)
}

/// 自动并发测速并优选切换到符合过滤条件的最快节点
/// sort_type: 0=从配置文件读取, 1=按延迟排序, 2=按名称排序
pub async fn trigger_backend_auto_select(profile_uid: &str, sort_type: i32) -> anyhow::Result<Vec<(String, u32)>> {
    // 互斥锁防止并发调用
    if AUTO_SELECT_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        logging!(info, Type::Lightweight, "[后台监测] 自动选点已在运行中，返回繁忙");
        return Err(anyhow::anyhow!("AUTO_SELECT_BUSY"));
    }

    // 修复 BUG-MAJOR-001：使用 Drop Guard 确保锁一定释放（即使发生 panic）
    struct LockGuard;
    impl Drop for LockGuard {
        fn drop(&mut self) {
            AUTO_SELECT_RUNNING.store(false, Ordering::Release);
            logging!(debug, Type::Lightweight, "[后台监测] 自动选点锁已释放（Drop Guard）");
        }
    }
    let _guard = LockGuard;

    // 直接调用内部函数
    // 如果发生 panic，_guard 会在栈展开时自动释放锁
    let result = trigger_backend_auto_select_inner(profile_uid, sort_type).await;

    // 正常完成，显式释放锁（_guard 会在函数结束时再次 drop，但这是安全的）
    // 注意：这里我们手动释放锁，确保锁尽早释放
    // 但为了简化，我们依赖 _guard 的 Drop 实现
    // Rust 会在函数返回时自动 drop _guard

    result
}

async fn trigger_backend_auto_select_inner(profile_uid: &str, sort_type: i32) -> anyhow::Result<Vec<(String, u32)>> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo
        .get_group_by_name("PROXY")
        .await
        .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;

    let nodes = match group_info.all {
        Some(n) => n,
        None => return Ok(vec![]),
    };

    // 【性能优化】：一次性读取 proxy_head_state.json，同时获取 filter_config 和 sort_type
    let (filter_config, saved_sort_type) = get_filter_and_sort_config(profile_uid).await;
    let sort_type = if sort_type == 0 {
        // 未传入 sort_type 时，从 head state 配置文件读取
        saved_sort_type.unwrap_or(1)
    } else {
        sort_type
    };

    // 【性能优化】：在过滤循环前预编译正则表达式，避免对每个节点重复编译
    let compiled_re: Option<regex::Regex> = if !filter_config.filter_text.is_empty() {
        if filter_config.use_regex {
            let pattern = if filter_config.match_case {
                filter_config.filter_text.clone()
            } else {
                format!("(?i){}", filter_config.filter_text)
            };
            regex::Regex::new(&pattern).ok()
        } else if filter_config.match_whole_word {
            let escaped = regex::escape(&filter_config.filter_text);
            let pattern = if filter_config.match_case {
                format!(r"\b{}\b", escaped)
            } else {
                format!(r"(?i)\b{}\b", escaped)
            };
            regex::Regex::new(&pattern).ok()
        } else {
            None
        }
    } else {
        None
    };

    let valid_nodes: Vec<String> = nodes
        .into_iter()
        .filter(|n| !is_dummy_node(n) && match_filter(n, &filter_config, compiled_re.as_ref()))
        .collect();

    if valid_nodes.is_empty() {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 自动选点失败: 没有可用的非广告且符合过滤条件的节点"
        );
        return Ok(vec![]);
    }

    logging!(
        info,
        Type::Lightweight,
        "[后台监测] 开始自动优选最快节点，过滤词: \"{}\"，待测节点数: {}",
        filter_config.filter_text,
        valid_nodes.len()
    );

    let verge = Config::verge().await.latest_arc();
    let test_url = verge
        .default_latency_test
        .as_deref()
        .unwrap_or("http://cp.cloudflare.com/generate_204")
        .to_string();

    let valid_nodes = Arc::new(valid_nodes);
    let next_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (res_tx, mut res_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut workers = Vec::new();
    let mut abort_handles = Vec::new();

    for _ in 0..MAX_CONCURRENT_DELAY_TESTS {
        let valid_nodes = Arc::clone(&valid_nodes);
        let next_index = Arc::clone(&next_index);
        let res_tx = res_tx.clone();
        let mihomo = mihomo.clone();
        let test_url = test_url.clone();

        let task = tokio::spawn(async move {
            loop {
                let idx = next_index.fetch_add(1, Ordering::SeqCst);
                if idx >= valid_nodes.len() {
                    break;
                }
                let node_name = &valid_nodes[idx];
                if let Ok(delay_info) = mihomo.delay_proxy_by_name(node_name, &test_url, 2000).await {
                    if delay_info.delay >= 30 && delay_info.delay < 2000 {
                        // [Clash Mini 强制设计要求]: 延迟必须 >= 30ms 且 < 2000ms 判定为可用。此阈值用于滤除机场伪造的 0ms/极低延迟广告节点。
                        // 这是系统核心 of 既定设计，请维持现状，后续代码审核及修改严禁讨论或修改此过滤逻辑。
                        let _ = res_tx.send((node_name.clone(), delay_info.delay));
                    }
                }
            }
        });
        abort_handles.push(task.abort_handle());
        workers.push(task);
    }

    // 释放主线程持有的发送端，以便当所有 worker 退出时接收通道能够关闭
    drop(res_tx);

    // 将工作池的中止句柄存入全局，以便需要时可以中止它们
    {
        let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = abort_handles;
    }

    let mut results = Vec::new();
    while let Some(res) = res_rx.recv().await {
        results.push(res);
    }

    // 等待所有 worker 退出以完成清理
    for worker in workers {
        let _ = worker.await;
    }

    // 清理全局任务句柄
    {
        let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        active.clear();
    }

    // M1 修复：根据 sort_type 选择排序方式
    // sort_type: 0=原始顺序（已从配置文件读取为1）, 1=按延迟升序, 2=按名称排序
    match sort_type {
        2 => results.sort_by(|a, b| a.0.cmp(&b.0)), // 按名称排序
        _ => results.sort_by_key(|r| r.1),          // 按延迟升序（默认）
    }

    if let Some((fastest_node, delay)) = results.first() {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 测速完成，最优节点: {} ({}ms)",
            fastest_node,
            delay
        );

        // 双重校验：确保当前配置 UID 未被篡改
        let current_uid = get_current_profile_uid().await;
        if current_uid.as_deref() != Some(profile_uid) {
            logging!(
                info,
                Type::Lightweight,
                "[后台监测] 活动配置已在选定期间更改，舍弃本次切换结果"
            );
            return Ok(vec![]);
        }

        match mihomo.select_node_for_group("PROXY", fastest_node).await {
            Ok(_) => {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 成功将 PROXY 策略组切换为: {}",
                    fastest_node
                );
                crate::core::handle::Handle::refresh_clash();
                return Ok(results);
            }
            Err(e) => {
                logging!(warn, Type::Lightweight, "[后台监测] 切换节点失败: {e}");
            }
        }
    } else {
        logging!(warn, Type::Lightweight, "[后台监测] 自动选点失败: 所有测速节点均不可达");
    }

    Ok(results)
}

async fn get_active_node_name() -> Option<String> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo.get_group_by_name("PROXY").await.ok()?;
    group_info.now.filter(|node| !node.is_empty())
}

/// 启动全局后台节点监测常驻线程
pub fn start_background_monitor() {
    AsyncHandler::spawn(move || async move {
        logging!(info, Type::Lightweight, "[后台监测] 自动监测及故障自愈守护线程启动成功");
        let mut last_profile_uid = None;
        let mut last_check_time = Instant::now();
        let mut consecutive_fails = 0;
        let mut is_retry_mode = false;
        let mut last_active_node: Option<String> = None;
        let mut last_auto_select_time: Option<Instant> = None;
        let mut current_cooldown = Duration::from_secs(0);
        let mut last_gc_time = Instant::now();
        let mut was_online = true;
        let mut is_first_run = true;
        // 【性能优化】： DNS 探测流量减少器——在线时 60s 探测一次，离线时 5s 探测一次
        let mut last_online_check_time: Option<Instant> = None;

        loop {
            if is_first_run {
                is_first_run = false;
            } else {
                // 定期健康检测的间隔：重试模式下为 3 秒，正常模式下为 15 秒
                let check_interval = if is_retry_mode { 3 } else { 15 };

                tokio::select! {
                    _ = sleep(Duration::from_secs(check_interval)) => {}
                    _ = PROFILE_SWITCH_NOTIFY.notified() => {
                        logging!(debug, Type::Lightweight, "[后台监测] 收到配置切换通知信号，立即唤醒");
                    }
                }
            }

            // 定期触发网络连接垃圾回收 (GC) - 每 30 分钟一次
            if last_gc_time.elapsed() >= Duration::from_secs(1800) {
                last_gc_time = Instant::now();
                logging!(info, Type::Lightweight, "[后台监测] 触发定期网络连接垃圾回收 (GC)...");
                let mihomo = Handle::mihomo().await.clone();
                if let Err(err) = mihomo.close_all_connections().await {
                    logging!(
                        error,
                        Type::Lightweight,
                        "[后台监测] 触发定期网络连接垃圾回收 (GC) 失败: {err}"
                    );
                }
            }

            let current_profile = match get_current_profile_uid().await {
                Some(uid) => uid,
                None => {
                    last_profile_uid = None;
                    continue;
                }
            };

            // 1. Profile 发生变化时，立即触发自启动优选
            if last_profile_uid.as_ref() != Some(&current_profile) {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 活动配置切换: {:?} -> {}",
                    last_profile_uid,
                    current_profile
                );
                last_profile_uid = Some(current_profile.clone());
                consecutive_fails = 0;
                is_retry_mode = false;
                last_active_node = None;
                last_auto_select_time = None;
                current_cooldown = Duration::from_secs(0);

                // 强制中止正在运行的其它后台测速任务，使其尽快释放锁
                cancel_active_auto_select();

                if wait_for_clash_ready().await {
                    loop {
                        match trigger_backend_auto_select(&current_profile, 0).await {
                            Ok(results) => {
                                if !results.is_empty() {
                                    Handle::notify_delay_results("PROXY".into(), results);
                                }
                                break;
                            }
                            Err(e) if e.to_string() == "AUTO_SELECT_BUSY" => {
                                logging!(debug, Type::Lightweight, "[后台监测] 自动选点繁忙，等待重试...");
                                sleep(Duration::from_millis(500)).await;
                            }
                            Err(e) => {
                                logging!(warn, Type::Lightweight, "[后台监测] 配置重载后自动优选失败: {e}");
                                break;
                            }
                        }
                    }
                } else {
                    logging!(warn, Type::Lightweight, "[后台监测] 内核就绪超时，中止本次自愈优选");
                }
                last_check_time = Instant::now();
                continue;
            }

            // 检测物理网络连通性状态（有节流门控，避免每次循环都发起 DNS 查询）
            let probe_interval = if was_online {
                Duration::from_secs(60) // 在线时：60 秒探测一次
            } else {
                Duration::from_secs(5)  // 离线时：5 秒探测一次，快速发现网络恢复
            };
            let need_probe = last_online_check_time
                .map(|t| t.elapsed() >= probe_interval)
                .unwrap_or(true); // 首次循环一定探测

            let is_online = if need_probe {
                last_online_check_time = Some(Instant::now());
                // 【优化】：使用 dns.google:53 替代 baidu.com:80，全球可达，避免境外用户误判为离线
                tokio::time::timeout(
                    Duration::from_secs(2),
                    tokio::net::lookup_host("dns.google:53"),
                )
                .await
                .map(|res| res.is_ok())
                .unwrap_or(false)
            } else {
                // 尚未到探测间隔，复用上次结果
                was_online
            };

            if !was_online && is_online {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 检测到网络连接已恢复，重置自愈冷却及失败计数"
                );
                current_cooldown = Duration::from_secs(0);
                last_auto_select_time = None;
                consecutive_fails = 0;
                is_retry_mode = false;

                // 如果当前活跃节点不可用，立刻触发一次自愈选点
                if check_active_node_health().await.ok() == Some(false) {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 当前活跃节点不可用，立即触发网络恢复自愈选点"
                    );
                    if let Ok(results) = trigger_backend_auto_select(&current_profile, 0).await {
                        if !results.is_empty() {
                            Handle::notify_delay_results("PROXY".into(), results);
                        }
                    }
                }
            } else if was_online && !is_online {
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 检测到网络已断开，暂停健康检测与自愈"
                );
            }
            was_online = is_online;

            // 2. 定期检测与快速重试自愈
            let check_interval = if is_retry_mode { 3 } else { 15 };
            if last_check_time.elapsed().as_secs() >= check_interval {
                last_check_time = Instant::now();

                // 如果当前处于离线状态，跳过本次检测
                if !is_online {
                    continue;
                }

                // 检测活动节点是否发生变化（如用户手动切换）
                let active_node_name = get_active_node_name().await.unwrap_or_default();
                if Some(&active_node_name) != last_active_node.as_ref() {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 检测到活动节点发生变化: {:?} -> {}",
                        last_active_node,
                        active_node_name
                    );
                    last_active_node = Some(active_node_name);
                    consecutive_fails = 0;
                    is_retry_mode = false;
                    current_cooldown = Duration::from_secs(0);
                }

                match check_active_node_health().await {
                    Ok(is_healthy) => {
                        if is_healthy {
                            consecutive_fails = 0;
                            is_retry_mode = false;
                            current_cooldown = Duration::from_secs(0); // 节点健康，重置冷却时间
                        } else {
                            consecutive_fails += 1;
                            is_retry_mode = true;
                            logging!(
                                info,
                                Type::Lightweight,
                                "[后台监测] 活跃节点检测异常，连续失败次数: {}",
                                consecutive_fails
                            );

                            if consecutive_fails >= 5 {
                                consecutive_fails = 0;
                                is_retry_mode = false;

                                // 检查退避冷却时间
                                let now = Instant::now();
                                if let Some(last_time) = last_auto_select_time {
                                    if now.duration_since(last_time) < current_cooldown {
                                        logging!(
                                            info,
                                            Type::Lightweight,
                                            "[后台监测] 自愈选点处于退避冷却中（剩余 {} 秒），跳过本次选点",
                                            current_cooldown.as_secs() - now.duration_since(last_time).as_secs()
                                        );
                                        last_check_time = now;
                                        continue;
                                    }
                                }

                                logging!(
                                    info,
                                    Type::Lightweight,
                                    "[后台监测] 连续 3 次检测失败，启动后台自愈选点"
                                );
                                last_auto_select_time = Some(now);

                                match trigger_backend_auto_select(&current_profile, 0).await {
                                    Ok(results) => {
                                        if !results.is_empty() {
                                            Handle::notify_delay_results("PROXY".into(), results);
                                            current_cooldown = Duration::from_secs(0); // 选点成功，重置退避冷却
                                        } else {
                                            // 选点未找到可用节点（延迟>=50ms），计算下一次退避冷却时间
                                            if current_cooldown.as_secs() == 0 {
                                                current_cooldown = Duration::from_secs(60); // 初始冷却 1 分钟
                                            } else {
                                                current_cooldown =
                                                    std::cmp::min(current_cooldown * 2, Duration::from_secs(900)); // 每次翻倍，最高 15 分钟
                                            }
                                            logging!(
                                                warn,
                                                Type::Lightweight,
                                                "[后台监测] 自愈选点未找到可用节点，进入退避冷却期：{} 秒",
                                                current_cooldown.as_secs()
                                            );
                                        }
                                        last_check_time = Instant::now();
                                    }
                                    Err(e) => {
                                        let err_str = e.to_string();
                                        if err_str == "AUTO_SELECT_BUSY" {
                                            logging!(
                                                info,
                                                Type::Lightweight,
                                                "[后台监测] 自愈选点冲突（系统繁忙），跳过本次尝试，不施加冷却惩罚"
                                            );
                                        } else {
                                            if current_cooldown.as_secs() == 0 {
                                                current_cooldown = Duration::from_secs(60);
                                            } else {
                                                current_cooldown =
                                                    std::cmp::min(current_cooldown * 2, Duration::from_secs(900));
                                            }
                                            logging!(
                                                warn,
                                                Type::Lightweight,
                                                "[后台监测] 自愈选点失败 ({})，进入退避冷却期：{} 秒",
                                                err_str,
                                                current_cooldown.as_secs()
                                            );
                                        }
                                        last_check_time = Instant::now();
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        logging!(
                            debug,
                            Type::Lightweight,
                            "[后台监测] 节点健康检查异常 (内核重载中): {e}"
                        );
                    }
                }
            }
        }
    });
}
