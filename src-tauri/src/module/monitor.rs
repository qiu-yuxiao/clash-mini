use crate::{
    config::{Config, PrfItem, PrfSelected, profiles_patch_item_safe},
    process::AsyncHandler,
};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::AbortHandle;
use tokio::time::{Duration, Instant, sleep};

/// 互斥锁：同一时间只允许一个 trigger_backend_auto_select 运行
static AUTO_SELECT_RUNNING: AtomicBool = AtomicBool::new(false);

/// 全局持有的正在运行的任务句柄，用于在 Profile 切换时进行主动中止
static ACTIVE_TASKS: Mutex<Vec<AbortHandle>> = Mutex::new(Vec::new());

/// 后台 monitor 任务的 JoinHandle，用于退出时主动中止
static MONITOR_TASK_HANDLE: Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = Mutex::new(None);

/// 中止所有活跃的测速任务（应用退出/重启时调用）
pub fn abort_all_active_tasks() {
    let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| {
        logging!(warn, Type::Lightweight, "ACTIVE_TASKS 锁被中毒线程污染，恢复继续");
        e.into_inner()
    });
    for handle in active.drain(..) {
        handle.abort();
    }
    drop(active);
    // 短暂等待，确保任务退出
    std::thread::sleep(std::time::Duration::from_millis(50));
}

/// 中止后台 monitor 常驻任务
pub fn abort_monitor() {
    let value = MONITOR_TASK_HANDLE
        .lock()
        .unwrap_or_else(|e| {
            logging!(
                warn,
                Type::Lightweight,
                "MONITOR_TASK_HANDLE 锁被中毒线程污染，恢复继续"
            );
            e.into_inner()
        })
        .take();
    if let Some(handle) = value {
        handle.abort();
    }
}

/// 用于通知后台监测线程：活动配置已被切换
pub static PROFILE_SWITCH_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 用于通知/唤醒后台监测线程（如退出轻量模式时）
pub static MONITOR_WAKEUP_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 节点延迟下限（毫秒）。小于此值的节点判定为假节点/广告节点。
/// 此为系统核心既定设计，严禁讨论或调整。
const NODE_DELAY_MIN_MS: u32 = 30;

/// 节点延迟上限（毫秒）。超过此值的节点判定为不可用（断流）。
/// 健康检测和自动选点测速均以此值作为超时阈值，确保判断标准统一。
const NODE_DELAY_MAX_MS: u32 = 2000;

/// 批量测速对 Clash 内核发起的探针超时上限。
/// 与死节点判定阈值 NODE_DELAY_MAX_MS(2000) 统一，确保「探针超时 == 判死阈值」：
/// 超过 2000ms 的节点一律按不可用（超时）处理，UI 显示 Error/Timeout，不再展示其真实延迟。
/// 节点「是否可用」的判定阈值同样以 NODE_DELAY_MAX_MS(2000) 为准。
const NODE_TEST_TIMEOUT_MS: u32 = NODE_DELAY_MAX_MS;

/// 自动选点执行结果
pub struct AutoSelectOutcome {
    /// 所有被测节点的延迟（含死节点/超时/错误），用于前端展示
    pub display: Vec<(String, u32)>,
    /// 是否存在可用（有效）节点并完成切换
    pub selected: bool,
}

/// 正常健康检测间隔（秒）
const NORMAL_CHECK_INTERVAL_SECS: u64 = 15;

/// 重试模式健康检测间隔（秒）
const RETRY_CHECK_INTERVAL_SECS: u64 = 3;

/// 离线时主循环间隔（秒）：与离线网络探测间隔对齐，确保 5 秒快速探测不被 15 秒主循环限制
const OFFLINE_CHECK_INTERVAL_SECS: u64 = 5;

/// 后台监测线程心跳日志周期（循环次数）
///
/// 【设计依据】
/// - 正常模式 15s/周期，20 个周期 ≈ 5 分钟
/// - 与 UI 线程心跳探针（5 秒）的"5 倍"原则保持一致：5s × 60 = 5min
/// - 远小于 v2.6.5 事件中的 4h22m 空白期，能快速区分"线程已死" vs "正常运行无事件"
/// - debug 级别，不影响生产日志体积（默认 Info 级别不显示）
///
/// 【设计盲区修复】
/// 此前 monitor.rs 主循环只在状态变化时打日志（节点变化/网络变化/失败告警），
/// 导致正常运行时连续几小时无日志输出，与"线程已死"无法区分。
/// 加周期性心跳后，排障时启用 RUST_LOG=debug 即可看到心跳，立刻判定线程状态。
const MONITOR_HEARTBEAT_CYCLE_COUNT: u64 = 20;

/// 从当前 Verge 配置中读取测速 URL，多处复用避免重复代码
async fn get_test_url() -> String {
    let verge = Config::verge().await.latest_arc();
    verge
        .default_latency_test
        .as_deref()
        .unwrap_or("http://cp.cloudflare.com/generate_204")
        .to_string()
}

/// 将测速 URL 解析为 `lookup_host` 可用的 `"host:port"` 字符串。
/// 【关键修正】对 IPv6 地址必须使用 `[addr]:port` 格式，裸拼接会导致解析失败。
fn resolve_probe_target(test_url: &str) -> String {
    match url::Url::parse(test_url) {
        Ok(parsed_url) => {
            let port = parsed_url.port_or_known_default().unwrap_or(80);
            match parsed_url.host() {
                Some(url::Host::Ipv6(addr)) => format!("[{}]:{}", addr, port),
                Some(url::Host::Ipv4(addr)) => format!("{}:{}", addr, port),
                Some(url::Host::Domain(domain)) => format!("{}:{}", domain, port),
                None => "cp.cloudflare.com:80".to_string(),
            }
        }
        Err(_) => "cp.cloudflare.com:80".to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub filter_text: String,
}

// 识别广告/假节点的实现已迁移到 crate::utils::node::is_dummy_node
// 以便 enhance/mod.rs 和 lifecycle.rs 共享同一份判定逻辑，避免遗漏
pub use crate::utils::node::is_dummy_node;

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
    };
    let sort_type = group_state["sortType"].as_i64().map(|v| v as i32);
    (filter_config, sort_type)
}

/// 过滤匹配算法：支持首尾去空、大小写不敏感的普通子串模糊匹配
/// `filter_lower` 应由调用方预先执行 trim + to_lowercase，避免在 filter 闭包中重复计算
fn match_filter(name: &str, filter_lower: &str) -> bool {
    if filter_lower.is_empty() {
        return true;
    }
    name.to_lowercase().contains(filter_lower)
}

/// 获取当前活动 Profile 的 UID
pub(crate) async fn get_current_profile_uid() -> Option<String> {
    let profiles = Config::profiles().await;
    profiles.data_arc().current.as_ref().map(|s| s.to_string())
}

/// 等待 Clash 内核 API 及代理组节点列表填充完毕
/// 如果期间检测到 Profile 切换通知，立即返回 false，让调用方重新用新 Profile 操作
pub(crate) async fn wait_for_clash_ready() -> bool {
    let start_time = Instant::now();

    // 阶段 1：等待内核 API 接口响应
    // 【注意】此处 clone mihomo 后跨 await 使用是安全的：
    // - MihomoManager 是全局单例，clone 的是 Arc，实例不会被替换
    // - 若后续改为可热替换，则需每次 await 后重新获取
    while start_time.elapsed().as_secs() < 30 {
        if crate::core::handle::Handle::global().is_exiting() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 中断：应用正在退出");
            return false;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if mihomo.get_base_config().await.is_ok() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 1 完成：内核 API 已就绪");
            break;
        }
        tokio::select! {
            _ = sleep(Duration::from_millis(200)) => {}
            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 1 中断：检测到 Profile 切换");
                return false;
            }
        }
    }

    if start_time.elapsed().as_secs() >= 30 {
        logging!(warn, Type::Lightweight, "[后台监测] 阶段 1 失败：等待内核 API 响应超时");
        return false;
    }

    // 阶段 2：等待代理节点列表填充
    let start_time_2 = Instant::now();
    while start_time_2.elapsed().as_secs() < 20 {
        if crate::core::handle::Handle::global().is_exiting() {
            logging!(info, Type::Lightweight, "[后台监测] 阶段 2 中断：应用正在退出");
            return false;
        }
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        if let Ok(group_info) = mihomo.get_group_by_name("PROXY").await {
            if let Some(all) = group_info.all {
                if !all.is_empty() {
                    logging!(info, Type::Lightweight, "[后台监测] 阶段 2 完成：代理节点列表已填充");
                    return true;
                }
            }
        }
        tokio::select! {
            _ = sleep(Duration::from_millis(500)) => {}
            _ = PROFILE_SWITCH_NOTIFY.notified() => {
                logging!(info, Type::Lightweight, "[后台监测] 阶段 2 中断：检测到 Profile 切换");
                return false;
            }
        }
    }

    logging!(
        warn,
        Type::Lightweight,
        "[后台监测] 阶段 2 失败：等待内核加载节点列表超时"
    );
    false
}

/// 恢复当前活动 Profile 配置文件中所保存的上次选定的 PROXY 组节点
// 【核心架构约定】Mini 只有 PROXY 一个有效代理组（所有上游代理组已合并），
// 此函数只处理 PROXY 组，不遍历多组（上游 Clash Verge Rev 的多组遍历逻辑已移除）。
pub(crate) async fn restore_profile_selected_nodes(profile_uid: &str) -> anyhow::Result<()> {
    let profiles = Config::profiles().await;
    let latest = profiles.latest_arc();
    let item = latest.get_item(profile_uid)?;
    if let Some(selected) = &item.selected {
        // Mini 单组架构：只恢复 PROXY 组
        if let Some(entry) = selected.iter().find(|s| s.name.as_deref() == Some("PROXY")) {
            if let (Some(_group), Some(node)) = (&entry.name, &entry.now) {
                if !node.is_empty() {
                    // 【治本修复②】恢复前校验节点是否在当前 filterText 子集内，
                    // 越界（被污染的持久选择）或伪节点不还原，交给定语 auto-select 在子集内重选，
                    // 避免"轻量唤醒后前端显示节点 ≠ 内核实际选路"的污染复现。
                    let filter_lower = get_filter_and_sort_config(profile_uid)
                        .await
                        .0
                        .filter_text
                        .trim()
                        .to_lowercase();
                    if is_dummy_node(node) || (!filter_lower.is_empty() && !match_filter(node, &filter_lower)) {
                        logging!(
                            warn,
                            Type::Lightweight,
                            "[后台监测] 跳过恢复越界/伪节点 {}（不在过滤范围「{}」内，疑似被污染的持久选择），交由自动选点重选",
                            node,
                            filter_lower
                        );
                        return Ok(());
                    }
                    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
                    logging!(info, Type::Lightweight, "[后台监测] 恢复 PROXY 组选择节点: {}", node);
                    let _ = mihomo.select_node_for_group("PROXY", node).await;
                }
            }
        }
    }
    Ok(())
}

/// 单节点健康状态（由测量组维护的 history/alive 判定，TUN 下可靠）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStatus {
    /// 测量组尚未测过该节点（数据缺失），不动作、等待测量
    Unknown,
    /// 节点不可用（测速超时/失败/假节点）
    Dead,
    /// 节点健康，附带真实延迟
    Alive(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 故障转移裁决
pub enum FailoverVerdict {
    /// 当前活跃节点健康或数据缺失，无需动作
    NoAction,
    /// 活跃节点已死且子集内存在健康候选，应触发选点
    ShouldFailover,
    /// 子集内全部节点均不可用
    AllDead,
}

/// 依据测量组（PROXY__METRICS，url-test）维护的 history/alive 判定节点状态。
/// 这是唯一权威健康来源：由 Mihomo 内核周期性直接拨测，TUN 下可靠，
/// 彻底取代旧设计「后端用 delay_proxy_by_name 自测活跃节点」的不可靠路径。
fn node_status(proxy: &tauri_plugin_mihomo::models::Proxy) -> NodeStatus {
    // 测量组尚未测过该节点 → 数据缺失，不误判
    // 取最近一次 history 延迟；缺失则同样视为 Unknown
    let last = match proxy.history.last() {
        Some(h) => h.delay as u32,
        None => return NodeStatus::Unknown,
    };
    // alive=false（测速失败）或延迟不在 [MIN, MAX) 区间 → 死
    //（低于下限 = 机场伪造假节点/广告节点，达到/超过上限 = 测速超时）
    if !proxy.alive || !(NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(&last) {
        NodeStatus::Dead
    } else {
        NodeStatus::Alive(last)
    }
}

/// 评估当前活跃节点是否需要故障转移。
/// 返回 Err 表示 Mihomo API 不可达（交由调用方累计 api_error_count）。
///
/// 每一次评估前先对测量组 PROXY__METRICS 发起一次即时 url-test（内核内部拨测，TUN 下可靠），
/// 确保活跃节点的 `history.last()` 与 `alive` 标志反映最新真实状态，
/// 而非依赖测量组 300 秒自然周期的陈旧缓存。
async fn evaluate_failover(profile_uid: &str) -> anyhow::Result<FailoverVerdict> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();

    let group_info = mihomo
        .get_group_by_name("PROXY")
        .await
        .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;

    let active_node = match &group_info.now {
        Some(n) if !n.is_empty() && n != "DIRECT" && n != "REJECT" => n.clone(),
        _ => return Ok(FailoverVerdict::NoAction), // 直连/拒绝节点或无选中，不动作
    };

    // 假/广告节点 → 视为需要重选（无需等 url-test）
    if is_dummy_node(&active_node) {
        return Ok(FailoverVerdict::ShouldFailover);
    }

    // 【v2.6.6 修复】在读取历史健康数据之前，先对测量组触发一次即时 url-test，
    // 确保 proxy.alive 与 proxy.history.last() 反映的是当前真实状态，
    // 而非 300 秒前测量组自然周期的陈旧缓存。
    // delay_group → Mihomo 内核直接拨测每个成员（TUN 下可靠），与旧 delay_proxy_by_name
    // 自指回环机制不同；单次调用拿到全组新鲜延迟后再判定活跃节点死活。
    let test_url = get_test_url().await;
    let _ = mihomo
        .delay_group("PROXY__METRICS", &test_url, NODE_DELAY_MAX_MS)
        .await
        .unwrap_or_default();

    let proxies = mihomo
        .get_proxies()
        .await
        .map_err(|e| anyhow::anyhow!("获取代理列表失败: {e}"))?;

    // 活跃节点本身健康或数据缺失 → 不动作
    match proxies.proxies.get(&active_node).map(node_status) {
        Some(NodeStatus::Alive(_)) | Some(NodeStatus::Unknown) | None => {
            return Ok(FailoverVerdict::NoAction);
        }
        Some(NodeStatus::Dead) => {}
    }

    // 活跃节点已死：在子集（filterText）内寻找健康候选
    let (filter_config, _) = get_filter_and_sort_config(profile_uid).await;
    let filter_lower = filter_config.filter_text.trim().to_lowercase();
    let members = group_info.all.clone().unwrap_or_default();

    let mut found_healthy = false;
    for name in &members {
        if name == &active_node || is_dummy_node(name) {
            continue;
        }
        if !filter_lower.is_empty() && !match_filter(name, &filter_lower) {
            continue;
        }
        if let Some(NodeStatus::Alive(_)) = proxies.proxies.get(name).map(node_status) {
            found_healthy = true;
            break;
        }
    }

    if found_healthy {
        Ok(FailoverVerdict::ShouldFailover)
    } else {
        Ok(FailoverVerdict::AllDead)
    }
}

/// 自动并发测速并优选切换到符合过滤条件的最快节点
/// - `sort_type`: 0=从配置文件读取, 1=按延迟排序, 2=按名称排序（仅影响展示顺序）
/// - `select`: true=测速完成后将 PROXY 切换至最快节点；false=仅测速填充展示，不切换
/// - `skip_wait_ready`: true=跳过 wait_for_clash_ready（调用方已确保就绪）
pub async fn trigger_backend_auto_select(
    profile_uid: &str,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
    skip_wait_ready: bool,
) -> anyhow::Result<AutoSelectOutcome> {
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

    // 确保内核就绪后再进行选点操作（调用方已确保就绪时跳过）
    if !skip_wait_ready && !wait_for_clash_ready().await {
        logging!(warn, Type::Lightweight, "[后台监测] 自动选点失败：内核尚未就绪");
        return Ok(AutoSelectOutcome {
            display: vec![],
            selected: false,
        });
    }

    // 直接调用内部函数
    // 如果发生 panic，_guard 会在栈展开时自动释放锁
    let result = trigger_backend_auto_select_inner(profile_uid, node_names, sort_type, select).await;

    // 锁已通过 Drop Guard 释放，在此之后执行副作用操作
    // （refresh_clash 和 notify_delay_results 不影响选点逻辑，放在锁外可减少锁持有时间）
    // 🛡️ 防线二：轻量模式下直接跳过副作用分发，避免无用的跨进程通信和撞车风险
    // 【治本】不再调用 refresh_clash()：整份配置重载 + auto_close_connection 是真实断流推手；
    // 选点本身已是干净热切换（select_node_for_group），无需重载。仅把真实延迟结果回写前端。
    if !crate::module::lightweight::is_in_lightweight_mode() {
        if let Ok(outcome) = &result {
            if !outcome.display.is_empty() {
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] 准备发送 {} 条延迟结果到前端...",
                    outcome.display.len()
                );
                let t0 = std::time::Instant::now();
                crate::core::handle::Handle::notify_delay_results("PROXY".into(), outcome.display.clone());
                logging!(
                    info,
                    Type::Lightweight,
                    "[后台监测] notify_delay_results 完成，耗时 {:?}",
                    t0.elapsed()
                );
            }
        }
    }

    result
}

async fn trigger_backend_auto_select_inner(
    profile_uid: &str,
    node_names: Option<Vec<String>>,
    sort_type: i32,
    select: bool,
) -> anyhow::Result<AutoSelectOutcome> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();

    // 解析展示排序方式（sort_type==0 时从 head state 读取）
    let (filter_config, saved_sort_type) = get_filter_and_sort_config(profile_uid).await;
    let sort_type = if sort_type == 0 {
        // 未传入 sort_type 时，从 head state 配置文件读取
        saved_sort_type.unwrap_or(1)
    } else {
        sort_type
    };

    // 确定待测节点子集：
    // - 传入 node_names（非空）：使用调用方指定的子集（如前端当前可见/所选节点），
    //   仅剔除 dummy，不再套用后端 filter_text（子集已由前端按当前筛选条件给出）
    // - 未传入：后端自取 PROXY 全量节点，套用 dummy + 保存的 filter_text 过滤（F1/自动选点场景）
    let valid_nodes: Vec<String> = match node_names {
        Some(names) if !names.is_empty() => names.into_iter().filter(|n| !is_dummy_node(n)).collect(),
        _ => {
            let group_info = mihomo
                .get_group_by_name("PROXY")
                .await
                .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;
            let nodes = match group_info.all {
                Some(n) => n,
                None => {
                    return Ok(AutoSelectOutcome {
                        display: vec![],
                        selected: false,
                    });
                }
            };
            let filter_lower = filter_config.filter_text.trim().to_lowercase();
            nodes
                .into_iter()
                .filter(|n| !is_dummy_node(n) && match_filter(n, &filter_lower))
                .collect()
        }
    };

    if valid_nodes.is_empty() {
        logging!(
            warn,
            Type::Lightweight,
            "[后台监测] 自动选点失败: 没有可用的非广告且符合过滤条件的节点"
        );
        return Ok(AutoSelectOutcome {
            display: vec![],
            selected: false,
        });
    }

    logging!(
        info,
        Type::Lightweight,
        "[后台监测] 开始自动优选最快节点，过滤词: \"{}\"，待测节点数: {}",
        filter_config.filter_text,
        valid_nodes.len()
    );

    let test_url = get_test_url().await;

    // 测量层根因修复：不再用 delay_proxy_by_name 对节点逐个自测
    // （TUN 下对活跃节点自测会回环返回 timeout，是断流+全 timeout 的根因）。
    // 改为对「测量组 PROXY__METRICS」发起 group delay：Mihomo 内核直接拨测每个成员，
    // 单调用、无回环，拿到全量子集的新鲜延迟后挑最快。测量组由 enhance 模块随 PROXY 组一同注入。
    let mut delays: std::collections::HashMap<String, u32> = match mihomo
        .delay_group("PROXY__METRICS", &test_url, NODE_TEST_TIMEOUT_MS)
        .await
    {
        Ok(d) => d,
        Err(e) => {
            logging!(warn, Type::Lightweight, "[后台监测] 读取测量组延迟失败: {e}");
            std::collections::HashMap::new()
        }
    };

    // 兜底：若 group delay 未返回任何数据（测量组尚未就绪），退化为读取各节点已维护的 history
    if delays.is_empty() {
        if let Ok(proxies) = mihomo.get_proxies().await {
            for name in &valid_nodes {
                if let Some(p) = proxies.proxies.get(name) {
                    if let Some(h) = p.history.last() {
                        delays.insert(name.clone(), h.delay as u32);
                    }
                }
            }
        }
    }

    // 分类收集：display 用于展示（全部有效节点），candidates 用于选点（仅有效延迟节点）
    let mut display: Vec<(String, u32)> = Vec::new();
    let mut candidates: Vec<(String, u32)> = Vec::new();
    for name in &valid_nodes {
        let delay = match delays.get(name) {
            Some(&d) => d,
            // 未测得（超时/失败/测量组未就绪）→ 0（timeout 语义）
            // 与 delay_proxy_by_name 失败时的返回一致，前端显示 "Timeout" 而非 "Error"
            None => 0,
        };
        display.push((name.clone(), delay));
        if (NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(&delay) {
            candidates.push((name.clone(), delay));
        }
    }

    // 展示结果按 sort_type 排序（仅影响返回顺序，不影响选点）
    match sort_type {
        2 => display.sort_by(|a, b| a.0.cmp(&b.0)), // 按名称排序
        _ => display.sort_by_key(|r| r.1),          // 按延迟升序（默认）
    }

    let mut selected = false;

    if select {
        // 选点固定按延迟升序取全局最快节点，不受 sort_type 影响
        // （修复：原逻辑依赖 results.first()，在 sort_type=2 时会误选首个字母序节点）
        if let Some((fastest_node, delay)) = candidates.iter().min_by_key(|r| r.1) {
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
                return Ok(AutoSelectOutcome {
                    display,
                    selected: false,
                });
            }

            match mihomo.select_node_for_group("PROXY", fastest_node).await {
                Ok(_) => {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 成功将 PROXY 策略组切换为: {}",
                        fastest_node
                    );
                    selected = true;
                    // 【关键修复】选点成功后同步回写 profile.selected，
                    // 否则唤醒/进入轻量模式时 restore_profile_selected_nodes
                    // 和 activateSelected 会用过时的 selected 值把节点切回旧节点。
                    // 单组架构下 selected 数组只含 PROXY 一项。
                    // 【治本修复①】写回前校验节点是否在当前 filterText 子集内；
                    // 越界（被污染的持久选择）不写回，避免污染 selected 后再次复现前后端不一致。
                    let filter_lower = get_filter_and_sort_config(profile_uid)
                        .await
                        .0
                        .filter_text
                        .trim()
                        .to_lowercase();
                    if filter_lower.is_empty() || match_filter(fastest_node, &filter_lower) {
                        let new_selected = vec![PrfSelected {
                            name: Some("PROXY".into()),
                            now: Some(fastest_node.clone().into()),
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
                            filter_lower
                        );
                    }
                    // 【二次bug修复】通知前端刷新代理缓存。
                    // 49212979 删除了 refresh_clash() 以避免 TUN 断流，但连带删掉了
                    // 前端刷新链路，导致活跃节点栏不更新、显示节点与实际节点不一致。
                    // 这里只调 refresh_proxies()（emit verge://refresh-proxy-config 事件），
                    // 不调 refresh_clash()（避免整份配置重载 + auto_close_connection 断流）。
                    // 轻量模式下界面不可见，无需刷新缓存，跳过多余的跨进程通信。
                    if !crate::module::lightweight::is_in_lightweight_mode() {
                        crate::core::handle::Handle::refresh_proxies();
                    }
                }
                Err(e) => {
                    logging!(warn, Type::Lightweight, "[后台监测] 切换节点失败: {e}");
                }
            }
        } else {
            logging!(warn, Type::Lightweight, "[后台监测] 自动选点失败: 所有测速节点均不可达");
        }
    }

    Ok(AutoSelectOutcome { display, selected })
}

async fn get_active_node_name() -> Option<String> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo.get_group_by_name("PROXY").await.ok()?;
    group_info.now.filter(|node| !node.is_empty())
}

/// 计算下一次健康检测的等待间隔（秒）：重试模式 3 秒、离线 5 秒、正常 15 秒，与唤醒周期对齐。
const fn check_interval_secs(is_retry_mode: bool, was_online: bool) -> u64 {
    if is_retry_mode {
        RETRY_CHECK_INTERVAL_SECS
    } else if !was_online {
        OFFLINE_CHECK_INTERVAL_SECS
    } else {
        NORMAL_CHECK_INTERVAL_SECS
    }
}

/// 启动全局后台节点监测常驻线程
pub fn start_background_monitor() {
    let handle = AsyncHandler::spawn(move || async move {
        logging!(info, Type::Lightweight, "[后台监测] 自动监测及故障自愈守护线程启动成功");
        let thread_start_time = Instant::now();
        let mut last_check_time = Instant::now();
        let mut consecutive_fails = 0;
        let mut is_retry_mode = false;
        let mut last_active_node: Option<String> = None;
        let mut was_online = true;
        let mut is_first_run = true;
        let mut last_online_check_time: Option<Instant> = None;
        let mut last_auto_select_time: Option<Instant> = None;
        let mut auto_select_fail_count = 0u32;
        // 连续 API 异常计数：mihomo API 不可达时（get_group_by_name 超时/连接失败）累计，
        // 达到阈值后触发自愈，避免把内核卡死误判为"内核重载中"而放弃自愈。
        let mut api_error_count = 0u32;
        // 【v2.6.5 隐患 C 修复】周期性心跳计数器
        // 用于在正常运行无事件时仍能定期输出 debug 心跳日志，与"线程已死"区分
        let mut cycle_count: u64 = 0;

        loop {
            if crate::core::handle::Handle::global().is_exiting() {
                logging!(info, Type::Lightweight, "[后台监测] 检测到应用退出，监测线程终止");
                break;
            }

            // 【v2.6.5 隐患 C 修复】周期性心跳日志
            // 每 MONITOR_HEARTBEAT_CYCLE_COUNT 个循环周期打一条 debug 心跳，
            // 让"正常运行无事件"也能留下日志痕迹，与"线程已死"可区分。
            // debug 级别，生产默认不可见；排障时用 RUST_LOG=debug 启用。
            cycle_count += 1;
            if cycle_count.is_multiple_of(MONITOR_HEARTBEAT_CYCLE_COUNT) {
                let uptime = thread_start_time.elapsed().as_secs();
                logging!(
                    debug,
                    Type::Lightweight,
                    "[后台监测] 心跳: active={:?}, fails={}, online={}, retry_mode={}, api_errors={}, uptime={}s, cycles={}",
                    last_active_node,
                    consecutive_fails,
                    was_online,
                    is_retry_mode,
                    api_error_count,
                    uptime,
                    cycle_count,
                );
            }

            if is_first_run {
                is_first_run = false;
                // 首次启动时等待内核就绪，确保后续 API 调用不会失败
                if !wait_for_clash_ready().await {
                    logging!(warn, Type::Lightweight, "[后台监测] 内核未就绪，跳过首次检测周期");
                    last_check_time = Instant::now();
                    continue;
                }
                // 窗口不可见（轻量/纯托盘启动）时，由后端在启动时执行一次初始化自动选点；
                // 窗口存在则交给前端，此处不动作。进入轻量模式（lightweight.rs B4）也会选点，
                // 二者由 trigger_backend_auto_select 内部的 AUTO_SELECT_RUNNING 互斥，不会重复执行。
                let window_state = crate::utils::window_manager::WindowManager::get_main_window_state();
                if matches!(window_state, crate::utils::window_manager::WindowState::NotExist) {
                    if let Some(uid) = get_current_profile_uid().await {
                        let _ = restore_profile_selected_nodes(&uid).await;
                        // 委托后端执行初始化自动选点；结果经事件回写前端 UI
                        let _ = trigger_backend_auto_select(&uid, None, 0, true, false).await;
                    }
                }
            } else {
                // 定期健康检测的间隔：重试模式 3 秒，离线 5 秒（对齐快速探测），正常 15 秒
                let check_interval = check_interval_secs(is_retry_mode, was_online);

                // 【注意】tokio::select! 中两个 Notify 的 notified() 分支：
                // - 若两个 Notify 同时有许可，select! 只会选中一个，另一个的许可会保留到下一次循环
                // - 由于两个分支最终都会将 last_check_time 提前（效果等价），丢失一次也不会出问题
                // - Notify::notify_one() 会存储许可，因此不存在"通知完全丢失"的风险
                tokio::select! {
                                _ = sleep(Duration::from_secs(check_interval)) => {}
                                _ = MONITOR_WAKEUP_NOTIFY.notified() => {
                                    logging!(debug, Type::Lightweight, "[后台监测] 收到唤醒信号，立即唤醒监测");
                                    // 唤醒即放行一次体检（探活+必要时自愈），不再等下一自然周期
                                    last_check_time = Instant::now() - Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS + 1);
                                }
                                _ = PROFILE_SWITCH_NOTIFY.notified() => {
                                    logging!(debug, Type::Lightweight, "[后台监测] 收到配置切换通知信号，中止旧测速任务并立即唤醒");
                                    // 中止旧Profile的测速任务，释放AUTO_SELECT_RUNNING锁
                                    let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| {
                    logging!(warn, Type::Lightweight, "ACTIVE_TASKS 锁被中毒线程污染，恢复继续");
                    e.into_inner()
                });
                                    for handle in active.drain(..) {
                                        handle.abort();
                                    }
                                    drop(active);
                                    last_check_time = Instant::now() - Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS + 1);
                                }
                            }
            }

            let current_profile = match get_current_profile_uid().await {
                Some(uid) => uid,
                None => {
                    continue;
                }
            };

            // 检测物理网络连通性状态（有节流门控，避免每次循环都发起 DNS 查询）
            let probe_interval = if was_online {
                Duration::from_secs(60) // 在线时：60 秒探测一次
            } else {
                Duration::from_secs(5) // 离线时：5 秒探测一次，快速发现网络恢复
            };
            let need_probe = last_online_check_time
                .map(|t| t.elapsed() >= probe_interval)
                .unwrap_or(true); // 首次循环一定探测

            let is_online = if need_probe {
                last_online_check_time = Some(Instant::now());

                // 【性能优化与动态探测】：直接从当前配置的测速网址中解析域名与端口作为探测目标，
                // 彻底消除硬编码的第三方网站，测速用什么网络检测就测什么，天然兼顾海内外。
                let test_url = get_test_url().await;
                let host_port = resolve_probe_target(&test_url);

                tokio::time::timeout(Duration::from_secs(2), tokio::net::lookup_host(host_port))
                    .await
                    .map(|res| res.is_ok())
                    .unwrap_or(false)
            } else {
                // 尚未到探测间隔，复用上次结果
                was_online
            };

            if !was_online && is_online {
                logging!(info, Type::Lightweight, "[后台监测] 检测到网络连接已恢复，重置失败计数");
                consecutive_fails = 0;
                is_retry_mode = false;

                // 如果当前活跃节点已死（测量组判定），立刻触发一次自愈选点
                if evaluate_failover(&current_profile)
                    .await
                    .map(|v| v != FailoverVerdict::NoAction)
                    .unwrap_or(false)
                {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 当前活跃节点不可用，立即触发网络恢复自愈选点"
                    );
                    // 委托后端执行网络恢复自愈选点；结果经事件回写前端 UI
                    let _ = trigger_backend_auto_select(&current_profile, None, 0, true, false).await;
                }
            } else if was_online && !is_online {
                logging!(
                    warn,
                    Type::Lightweight,
                    "[后台监测] 检测到网络已断开，暂停健康检测与自愈"
                );
            }
            was_online = is_online;

            // 2. 定期检测与快速重试自愈：重试 3 秒，离线 5 秒，正常 15 秒，对齐唤醒周期
            let check_interval = check_interval_secs(is_retry_mode, was_online);
            if last_check_time.elapsed().as_secs() >= check_interval {
                last_check_time = Instant::now();

                // 如果当前处于离线状态，跳过本次检测
                if !is_online {
                    continue;
                }

                // 检测活动节点是否发生变化（包括自动选点和手动切换）
                let active_node_name = match get_active_node_name().await {
                    Some(name) => name,
                    None => {
                        // API 不可达（内核崩了/未就绪）——静默跳过本次检测，不把空字符串
                        // 当作"节点消失"写入日志，也不重置 last_active_node。
                        continue;
                    }
                };
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
                }

                match evaluate_failover(&current_profile).await {
                    Ok(verdict) => {
                        // API 调用成功，重置 API 异常计数
                        api_error_count = 0;
                        match verdict {
                            FailoverVerdict::NoAction => {
                                consecutive_fails = 0;
                                is_retry_mode = false;
                            }
                            FailoverVerdict::ShouldFailover | FailoverVerdict::AllDead => {
                                consecutive_fails += 1;
                                is_retry_mode = true;
                                logging!(
                                    info,
                                    Type::Lightweight,
                                    "[后台监测] 活跃节点检测异常 ({:?})，连续失败次数: {}",
                                    verdict,
                                    consecutive_fails
                                );

                                if consecutive_fails >= 2 {
                                    consecutive_fails = 0;
                                    is_retry_mode = false;

                                    // 60 秒冷却保护：auto_select 失败后至少等 60 秒再重试
                                    let now = Instant::now();
                                    if let Some(last_time) = last_auto_select_time {
                                        let elapsed = now.duration_since(last_time).as_secs();
                                        if elapsed < 60 {
                                            logging!(
                                                debug,
                                                Type::Lightweight,
                                                "[后台监测] 自愈选点在 60 秒冷却中（已过 {} 秒），跳过本次",
                                                elapsed
                                            );
                                            continue;
                                        }
                                    }
                                    let prev_auto_select_time = last_auto_select_time;
                                    last_auto_select_time = Some(now);

                                    logging!(
                                        info,
                                        Type::Lightweight,
                                        "[后台监测] 连续 2 次检测失败，启动后台自愈选点"
                                    );

                                    match trigger_backend_auto_select(&current_profile, None, 0, true, false).await {
                                        Ok(outcome) => {
                                            if outcome.selected {
                                                auto_select_fail_count = 0; // 选点成功，重置失败计数
                                            } else {
                                                auto_select_fail_count += 1;
                                                logging!(
                                                    warn,
                                                    Type::Lightweight,
                                                    "[后台监测] 自愈选点未选出可用节点（所有节点不可达），连续失败次数: {}",
                                                    auto_select_fail_count
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
                                                // BUSY 不算失败，回退冷却时间戳，允许下次重试
                                                last_auto_select_time = prev_auto_select_time;
                                            } else {
                                                auto_select_fail_count += 1;
                                                logging!(
                                                    warn,
                                                    Type::Lightweight,
                                                    "[后台监测] 自愈选点失败 ({}), 连续失败次数: {}",
                                                    err_str,
                                                    auto_select_fail_count
                                                );
                                            }
                                            last_check_time = Instant::now();
                                        }
                                    }

                                    // 连续 5 次 auto_select 失败 → Windows 系统警报
                                    // 注意：show_error_dialog 内部使用 MessageBoxW 同步阻塞调用，
                                    // 这会阻塞当前 tokio worker 线程直到用户点击"确定"。
                                    // 这是有意为之的设计：5次失败说明网络环境已严重恶化，
                                    // 继续健康检测和自愈已无意义，弹窗期间停下来等用户处理是正确行为。
                                    if auto_select_fail_count >= 5 {
                                        auto_select_fail_count = 0;
                                        logging!(
                                            warn,
                                            Type::Lightweight,
                                            "[后台监测] 连续 5 次自愈选点失败，弹出 Windows 提示框"
                                        );
                                        crate::show_error_dialog(
                                            "Clash Mini - 网络警报",
                                            "后台自动优选节点连续 5 次失败，当前所有代理节点均已失效，无法正常连接网络。\n\n请检查您的网络连接或节点订阅状态。",
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        api_error_count += 1;
                        // 日志从 debug 升级为 warn：API 异常可能是内核卡死而非重载，
                        // 需要在 normal 日志级别可见，便于排查。
                        logging!(
                            warn,
                            Type::Lightweight,
                            "[后台监测] 节点健康检查异常 (API不可达): {e}，连续异常次数: {}",
                            api_error_count
                        );
                        // 连续 3 次 API 异常（正常模式约 45 秒，重试模式约 9 秒），
                        // 说明不是内核重载的短暂现象，而是 mihomo API 持续不可达。
                        // 触发自愈选点尝试恢复（若自愈也因 API 不可达而失败，
                        // auto_select_fail_count 会累计，5 次后弹 Windows 警报）。
                        if api_error_count >= 3 {
                            api_error_count = 0;
                            logging!(warn, Type::Lightweight, "[后台监测] 连续 3 次 API 异常，触发自愈选点");
                            let _ = trigger_backend_auto_select(&current_profile, None, 0, true, false).await;
                        }
                    }
                }
            }
        }
    });

    *MONITOR_TASK_HANDLE.lock().unwrap_or_else(|e| {
        logging!(
            warn,
            Type::Lightweight,
            "MONITOR_TASK_HANDLE 锁被中毒线程污染，恢复继续"
        );
        e.into_inner()
    }) = Some(handle);
}

#[cfg(test)]
mod tests {
    use super::resolve_probe_target;

    #[test]
    fn test_resolve_probe_target_ipv4() {
        assert_eq!(resolve_probe_target("http://1.2.3.4/generate_204"), "1.2.3.4:80");
        assert_eq!(resolve_probe_target("https://1.2.3.4:8443/test"), "1.2.3.4:8443");
    }

    #[test]
    fn test_resolve_probe_target_ipv6() {
        // IPv6 必须被包装为 [addr]:port，否则 lookup_host 解析失败
        assert_eq!(
            resolve_probe_target("http://[2001:db8::1]/generate_204"),
            "[2001:db8::1]:80"
        );
        assert_eq!(resolve_probe_target("https://[::1]:8443/test"), "[::1]:8443");
    }

    #[test]
    fn test_resolve_probe_target_domain() {
        assert_eq!(
            resolve_probe_target("http://cp.cloudflare.com/generate_204"),
            "cp.cloudflare.com:80"
        );
        assert_eq!(
            resolve_probe_target("https://example.com:8443/test"),
            "example.com:8443"
        );
    }

    #[test]
    fn test_resolve_probe_target_malformed() {
        // 非法 URL fallback 到默认目标
        assert_eq!(resolve_probe_target("not-a-valid-url"), "cp.cloudflare.com:80");
    }
}
