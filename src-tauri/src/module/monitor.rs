use crate::{config::Config, process::AsyncHandler};
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

/// 中止所有活跃的测速任务（应用退出/重启时调用）
pub fn abort_all_active_tasks() {
    let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
    for handle in active.drain(..) {
        handle.abort();
    }
}

/// 用于通知后台监测线程：活动配置已被切换
pub static PROFILE_SWITCH_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 用于通知/唤醒后台监测线程（如退出轻量模式时）
pub static MONITOR_WAKEUP_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// 并发测速的最大线程数
const MAX_CONCURRENT_DELAY_TESTS: usize = 32;

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

/// 从当前 Verge 配置中读取测速 URL，多处复用避免重复代码
async fn get_test_url() -> String {
    let verge = Config::verge().await.latest_arc();
    verge
        .default_latency_test
        .as_deref()
        .unwrap_or("http://cp.cloudflare.com/generate_204")
        .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub filter_text: String,
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

/// 恢复当前活动 Profile 配置文件中所保存的上次选定的各策略组节点
pub(crate) async fn restore_profile_selected_nodes(profile_uid: &str) -> anyhow::Result<()> {
    let profiles = Config::profiles().await;
    let latest = profiles.latest_arc();
    let item = latest.get_item(profile_uid)?;
    if let Some(selected) = &item.selected {
        let mihomo = crate::core::handle::Handle::mihomo().await.clone();
        for select in selected {
            if let (Some(group), Some(node)) = (&select.name, &select.now) {
                if !node.is_empty() && !group.is_empty() {
                    logging!(
                        info,
                        Type::Lightweight,
                        "[后台监测] 恢复策略组选择节点: {} -> {}",
                        group,
                        node
                    );
                    let _ = mihomo.select_node_for_group(group, node).await;
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHealthStatus {
    Healthy,
    Unhealthy,
    Dead,
}

/// 检测当前活跃代理节点的健康状态
async fn check_active_node_health() -> anyhow::Result<NodeHealthStatus> {
    let mihomo = crate::core::handle::Handle::mihomo().await.clone();
    let group_info = mihomo
        .get_group_by_name("PROXY")
        .await
        .map_err(|e| anyhow::anyhow!("获取 PROXY 组信息失败: {e}"))?;

    let active_node = match group_info.now {
        Some(ref node) if !node.is_empty() && node != "DIRECT" && node != "REJECT" => node,
        _ => return Ok(NodeHealthStatus::Healthy), // 直连或拒绝节点，直接判定为健康
    };

    // 如果为 dummy 广告节点，判定为亚健康，触发重选
    if is_dummy_node(active_node) {
        logging!(
            info,
            Type::Lightweight,
            "[后台监测] 检测到当前节点为假/广告节点: {}",
            active_node
        );
        return Ok(NodeHealthStatus::Unhealthy);
    }

    let test_url = get_test_url().await;

    match mihomo
        .delay_proxy_by_name(active_node, &test_url, NODE_DELAY_MAX_MS)
        .await
    {
        Ok(delay_info) => {
            if delay_info.delay >= NODE_DELAY_MIN_MS && delay_info.delay < NODE_DELAY_MAX_MS {
                Ok(NodeHealthStatus::Healthy)
            } else {
                // 小于 30ms 的节点一律为机场伪造的假节点/广告节点，判定为亚健康并触发重选。
                // 此为系统核心既定设计，严禁后续代码审核或修改中再对此阈值进行任何讨论或调整。
                Ok(NodeHealthStatus::Unhealthy)
            }
        }
        Err(_) => {
            // 测速直接报错，说明节点彻底断线
            Ok(NodeHealthStatus::Dead)
        }
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
    if let Ok(outcome) = &result {
        if outcome.selected {
            crate::core::handle::Handle::refresh_clash();
        }
        if !outcome.display.is_empty() {
            crate::core::handle::Handle::notify_delay_results("PROXY".into(), outcome.display.clone());
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

    struct WorkerPoolGuard {
        abort_handles: Vec<AbortHandle>,
        armed: bool,
    }

    impl Drop for WorkerPoolGuard {
        fn drop(&mut self) {
            if self.armed {
                for handle in &self.abort_handles {
                    handle.abort();
                }
            }
        }
    }

    impl WorkerPoolGuard {
        const fn new() -> Self {
            Self {
                abort_handles: Vec::new(),
                armed: true,
            }
        }

        fn push(&mut self, handle: AbortHandle) {
            self.abort_handles.push(handle);
        }

        fn disarm(mut self) -> Vec<AbortHandle> {
            self.armed = false;
            std::mem::take(&mut self.abort_handles)
        }
    }

    let valid_nodes = Arc::new(valid_nodes);
    let next_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (res_tx, mut res_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut workers = Vec::new();
    let mut pool_guard = WorkerPoolGuard::new();

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
                // 探针超时=判死阈值=2000ms：超过即按不可用处理，简化前后端一致性。
                match mihomo
                    .delay_proxy_by_name(node_name, &test_url, NODE_TEST_TIMEOUT_MS)
                    .await
                {
                    Ok(delay_info) => {
                        // 全部上报用于展示（含低于下限的假节点、达到上限的死节点）
                        let _ = res_tx.send((node_name.clone(), delay_info.delay));
                    }
                    Err(_) => {
                        // 测速报错（彻底断线/内核异常），上报为 Error
                        let _ = res_tx.send((node_name.clone(), 1_000_000));
                    }
                }
            }
        });
        pool_guard.push(task.abort_handle());
        workers.push(task);
    }

    // 释放主线程持有的发送端，以便当所有 worker 退出时接收通道能够关闭
    drop(res_tx);

    // 所有 worker spawn 成功，解除 guard 的自动 abort，将句柄存入全局
    let abort_handles = pool_guard.disarm();
    {
        let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = abort_handles;
    }

    // 分类收集：display 用于展示（全部节点），candidates 用于选点（仅有效节点）
    let mut display: Vec<(String, u32)> = Vec::new();
    let mut candidates: Vec<(String, u32)> = Vec::new();
    while let Some((name, delay)) = res_rx.recv().await {
        display.push((name.clone(), delay));
        if (NODE_DELAY_MIN_MS..NODE_DELAY_MAX_MS).contains(&delay) {
            candidates.push((name, delay));
        }
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

/// 启动全局后台节点监测常驻线程
pub fn start_background_monitor() {
    AsyncHandler::spawn(move || async move {
        logging!(info, Type::Lightweight, "[后台监测] 自动监测及故障自愈守护线程启动成功");
        let mut last_check_time = Instant::now();
        let mut consecutive_fails = 0;
        let mut is_retry_mode = false;
        let mut last_active_node: Option<String> = None;
        let mut was_online = true;
        let mut is_first_run = true;
        let mut last_online_check_time: Option<Instant> = None;
        let mut last_auto_select_time: Option<Instant> = None;
        let mut auto_select_fail_count = 0u32;

        loop {
            if crate::core::handle::Handle::global().is_exiting() {
                logging!(info, Type::Lightweight, "[后台监测] 检测到应用退出，监测线程终止");
                break;
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
                // 定期健康检测的间隔：重试模式下为 3 秒，正常模式下为 15 秒
                let check_interval = if is_retry_mode {
                    RETRY_CHECK_INTERVAL_SECS
                } else {
                    NORMAL_CHECK_INTERVAL_SECS
                };

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
                        let mut active = ACTIVE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
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

                let host_port = match url::Url::parse(&test_url) {
                    Ok(parsed_url) => {
                        let host = parsed_url.host_str().unwrap_or("cp.cloudflare.com");
                        let port = parsed_url
                            .port()
                            .unwrap_or_else(|| if parsed_url.scheme() == "https" { 443 } else { 80 });
                        format!("{}:{}", host, port)
                    }
                    Err(_) => "cp.cloudflare.com:80".to_string(),
                };

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

                // 如果当前活跃节点不可用，立刻触发一次自愈选点
                if check_active_node_health().await.ok() != Some(NodeHealthStatus::Healthy) {
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

            // 2. 定期检测与快速重试自愈
            let check_interval = if is_retry_mode {
                RETRY_CHECK_INTERVAL_SECS
            } else {
                NORMAL_CHECK_INTERVAL_SECS
            };
            if last_check_time.elapsed().as_secs() >= check_interval {
                last_check_time = Instant::now();

                // 如果当前处于离线状态，跳过本次检测
                if !is_online {
                    continue;
                }

                // 检测活动节点是否发生变化（包括自动选点和手动切换）
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
                }

                match check_active_node_health().await {
                    Ok(status) => {
                        match status {
                            NodeHealthStatus::Healthy => {
                                consecutive_fails = 0;
                                is_retry_mode = false;
                            }
                            NodeHealthStatus::Unhealthy | NodeHealthStatus::Dead => {
                                consecutive_fails += 1;
                                is_retry_mode = true;
                                logging!(
                                    info,
                                    Type::Lightweight,
                                    "[后台监测] 活跃节点检测异常 ({:?})，连续失败次数: {}",
                                    status,
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
