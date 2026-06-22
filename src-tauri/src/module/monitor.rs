use crate::{config::Config, core::handle::Handle, process::AsyncHandler};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;
use tokio::time::{Duration, Instant, sleep};

/// 互斥锁：同一时间只允许一个 trigger_backend_auto_select 运行
static AUTO_SELECT_RUNNING: AtomicBool = AtomicBool::new(false);

/// 全局持有的正在运行的任务句柄，用于在 Profile 切换时进行主动中止
static ACTIVE_TASKS: Mutex<Vec<AbortHandle>> = Mutex::new(Vec::new());

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

/// 从本地 `proxy_head_state.json` 读取指定 Profile 的过滤规则
pub async fn get_active_filter_config(profile_uid: &str) -> FilterConfig {
    let path = match crate::utils::dirs::app_home_dir() {
        Ok(dir) => dir.join("proxy_head_state.json"),
        Err(_) => return FilterConfig::default(),
    };
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return FilterConfig::default(),
    };
    let json_val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return FilterConfig::default(),
    };

    let group_state = &json_val[profile_uid]["PROXY"];
    if group_state.is_null() {
        return FilterConfig::default();
    }

    FilterConfig {
        filter_text: group_state["filterText"].as_str().unwrap_or("").to_string(),
        use_regex: group_state["filterUseRegularExpression"].as_bool().unwrap_or(false),
        match_case: group_state["filterMatchCase"].as_bool().unwrap_or(false),
        match_whole_word: group_state["filterMatchWholeWord"].as_bool().unwrap_or(false),
    }
}

/// 从 `proxy_head_state.json` 读取前台保存的排序类型
async fn get_saved_sort_type(profile_uid: &str) -> Option<i32> {
    let path = crate::utils::dirs::app_home_dir().ok()?.join("proxy_head_state.json");
    let content = tokio::fs::read_to_string(path).await.ok()?;
    let json_val: serde_json::Value = serde_json::from_str(&content).ok()?;
    let sort_type = json_val[profile_uid]["PROXY"]["sortType"].as_i64()?;
    Some(sort_type as i32)
}

/// 过滤匹配算法：支持大小写敏感、正则匹配、全字匹配
fn match_filter(name: &str, config: &FilterConfig) -> bool {
    if config.filter_text.is_empty() {
        return true;
    }

    if config.use_regex {
        let pattern = if config.match_case {
            config.filter_text.clone()
        } else {
            format!("(?i){}", config.filter_text)
        };
        if let Ok(re) = regex::Regex::new(&pattern) {
            return re.is_match(name);
        }
    }

    let (n_str, f_str) = if config.match_case {
        (name.to_string(), config.filter_text.clone())
    } else {
        (name.to_lowercase(), config.filter_text.to_lowercase())
    };

    if config.match_whole_word {
        let pattern = format!(r"\b{}\b", regex::escape(&config.filter_text));
        let pattern = if config.match_case {
            pattern
        } else {
            format!("(?i){}", pattern)
        };
        if let Ok(re) = regex::Regex::new(&pattern) {
            return re.is_match(name);
        }
    }

    n_str.contains(&f_str)
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
        if delay_info.delay >= 50 {
            // 延迟满足 >= 50ms，即判定为健康
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

    let filter_config = get_active_filter_config(profile_uid).await;
    let sort_type = if sort_type == 0 {
        // 未传入 sort_type 时，从 head state 配置文件读取
        get_saved_sort_type(profile_uid).await.unwrap_or(1)
    } else {
        sort_type
    };

    let valid_nodes: Vec<String> = nodes
        .into_iter()
        .filter(|n| !is_dummy_node(n) && match_filter(n, &filter_config))
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

    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT_DELAY_TESTS));
    let mut tasks = Vec::new();
    let mut abort_handles = Vec::new();

    for node in valid_nodes {
        let mihomo = mihomo.clone();
        let test_url = test_url.clone();
        let node_name = node.clone();
        let sem = Arc::clone(&sem);

        let task = tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                return None;
            };
            if let Ok(delay_info) = mihomo.delay_proxy_by_name(&node_name, &test_url, 2000).await {
                if delay_info.delay >= 50 && delay_info.delay < 2000 {
                    return Some((node_name, delay_info.delay));
                }
            }
            None
        });
        abort_handles.push(task.abort_handle());
        tasks.push(task);
    }

    // 将这些中止句柄存入全局，以便需要时可以中止它们
    {
        let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = abort_handles;
    }

    let mut results = Vec::new();
    for task in tasks {
        if let Ok(Some(res)) = task.await {
            results.push(res);
        }
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

        loop {
            sleep(Duration::from_secs(1)).await;

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

            // 2. 定期检测与快速重试自愈
            let check_interval = if is_retry_mode { 3 } else { 15 };
            if last_check_time.elapsed().as_secs() >= check_interval {
                last_check_time = Instant::now();

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
                                            e,
                                            current_cooldown.as_secs()
                                        );
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
