use super::verdict::NodeStatus;
use super::{
    DIAL_FAIL_REPROBE_COOLDOWN_SECS, DIAL_FAIL_REPROBE_THRESHOLD, DIAL_FAIL_WINDOW_SECS, FORCE_ONLINE_RECHECK,
    IDLE_HEALTH_FRESH_SECS, LAST_FLOWING_TRAFFIC_AT, MAX_KERNEL_LOG_INCREMENTAL_BYTES, PASSIVE_TRAFFIC_MAX_SKIP_SECS,
    STALL_REPROBE_COOLDOWN_SECS,
};
use std::collections::HashMap;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveTrafficState {
    /// 近一周期活跃节点连接有真实下行回包（`delta_download > 0`），物理证实远端存活
    Flowing,
    /// 近一周期活跃节点连接有上行发包但零下行回包（`delta_upload > 0 && delta_download == 0`），疑似黑洞停滞
    Stalled,
    /// 无活跃节点上下行流量变化（空闲、切换节点首轮建基线或连接查询不可用）
    Idle,
}

#[derive(Default)]
pub(super) struct ActiveTrafficTracker {
    active_node: Option<String>,
    conn_bytes: HashMap<String, (u64, u64)>,
    pub(super) last_stall_probe_at: Option<Instant>,
}

pub(super) static ACTIVE_TRAFFIC_TRACKER: parking_lot::Mutex<Option<ActiveTrafficTracker>> =
    parking_lot::Mutex::new(None);

/// 纯状态归因：对比前后两轮落在当前活跃出口节点上的连接上下行字节，判定流量走势并返回新基线。
pub(super) fn classify_active_traffic_delta(
    prev_node: Option<&str>,
    active_node: &str,
    prev_conns: &HashMap<String, (u64, u64)>,
    current_conns: &[(String, u64, u64)],
) -> (ActiveTrafficState, HashMap<String, (u64, u64)>) {
    let mut next_map = HashMap::with_capacity(current_conns.len());
    for (id, up, down) in current_conns {
        next_map.insert(id.clone(), (*up, *down));
    }

    if prev_node != Some(active_node) {
        return (ActiveTrafficState::Idle, next_map);
    }

    let mut delta_up: u64 = 0;
    let mut delta_down: u64 = 0;
    for (id, up, down) in current_conns {
        if let Some(&(prev_up, prev_down)) = prev_conns.get(id) {
            delta_up = delta_up.saturating_add(up.saturating_sub(prev_up));
            delta_down = delta_down.saturating_add(down.saturating_sub(prev_down));
        } else {
            delta_up = delta_up.saturating_add(*up);
            delta_down = delta_down.saturating_add(*down);
        }
    }

    let state = if delta_down > 0 {
        ActiveTrafficState::Flowing
    } else if delta_up > 0 {
        ActiveTrafficState::Stalled
    } else {
        ActiveTrafficState::Idle
    };
    (state, next_map)
}

fn update_active_traffic_state(
    tracker_slot: &mut Option<ActiveTrafficTracker>,
    active_node: &str,
    current_conns: &[(String, u64, u64)],
) -> (ActiveTrafficState, bool) {
    let tracker = tracker_slot.get_or_insert_with(ActiveTrafficTracker::default);
    let node_changed = tracker.active_node.as_deref() != Some(active_node);
    if node_changed {
        tracker.last_stall_probe_at = None;
    }
    let stall_cooldown_active = tracker
        .last_stall_probe_at
        .is_some_and(|t| t.elapsed() < Duration::from_secs(STALL_REPROBE_COOLDOWN_SECS));

    let (state, next_map) = classify_active_traffic_delta(
        tracker.active_node.as_deref(),
        active_node,
        &tracker.conn_bytes,
        current_conns,
    );
    tracker.active_node = Some(active_node.to_owned());
    tracker.conn_bytes = next_map;
    (state, stall_cooldown_active)
}

pub(super) async fn sample_active_node_traffic(
    mihomo: &tauri_plugin_mihomo::Mihomo,
    active_node: &str,
) -> (ActiveTrafficState, bool) {
    let Ok(conns) = mihomo.get_connections().await else {
        return (ActiveTrafficState::Idle, false);
    };

    let current_conns: Vec<(String, u64, u64)> = conns
        .connections
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.chains.iter().any(|ch| ch == active_node))
        .map(|c| (c.id, c.upload, c.download))
        .collect();

    let (state, stall_cooldown_active) =
        update_active_traffic_state(&mut ACTIVE_TRAFFIC_TRACKER.lock(), active_node, &current_conns);
    if state == ActiveTrafficState::Flowing {
        *LAST_FLOWING_TRAFFIC_AT.lock() = Some(Instant::now());
    }
    (state, stall_cooldown_active)
}

/// 后台守护周期最近一轮完整健康观测快照，供 10 分钟 `info` 稳态摘要复用（零额外 IPC）。
#[derive(Debug, Clone)]
pub(super) struct ActiveHealthObservation {
    pub(super) active_node: String,
    pub(super) status: NodeStatus,
    pub(super) traffic_state: ActiveTrafficState,
    pub(super) history_age_secs: Option<u64>,
    pub(super) probed_this_round: bool,
}

pub(super) static LAST_HEALTH_OBSERVATION: parking_lot::Mutex<Option<ActiveHealthObservation>> =
    parking_lot::Mutex::new(None);

/// 内核日志告警事件分类（BUG-415 / BUG-416）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KernelLogEvent {
    ProxyDialFailure,
    TunInterfaceLost,
    TunInterfaceChanged,
}

/// 驱动跳过健康缓存立即复核的信号原因（BUG-415 / BUG-416）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KernelReprobeReason {
    DialFailureBurst(usize),
    TunInterfaceChanged,
}

/// 代理节点建连/握手/会话阶段失败及网卡变更新增追踪器（BUG-415 / BUG-416，零常驻 WebSocket）。
#[derive(Default)]
pub(super) struct ProxyDialFailureTracker {
    active_node: Option<String>,
    active_since_utc: Option<chrono::DateTime<chrono::Utc>>,
    failure_instants: Vec<Instant>,
    interface_changed_pending: bool,
    last_probe_at: Option<Instant>,
    cursor_path: Option<PathBuf>,
    cursor_offset: u64,
    pub(super) reanchor_to_end: bool,
}

pub(super) static PROXY_DIAL_FAILURE_TRACKER: parking_lot::Mutex<Option<ProxyDialFailureTracker>> =
    parking_lot::Mutex::new(None);

/// 精准识别指向 `PROXY` 策略组的代理建连/握手/会话失败日志行（排除 `DIRECT`、`PROXY__METRICS` 与纯 DNS/取消噪声）。
pub(super) fn is_proxy_dial_failure_log_line(line: &str) -> bool {
    if !line.contains("dial PROXY ") {
        return false;
    }
    let Some((_, err_tail)) = line.rsplit_once(" error: ") else {
        return false;
    };
    let err_lower = err_tail.to_ascii_lowercase();
    if err_lower.contains("operation was canceled")
        || err_lower.contains("context canceled")
        || err_lower.contains("dns resolve failed")
        || err_lower.contains("couldn't find ip")
        || err_lower.contains("no such host")
    {
        return false;
    }
    err_lower.contains("i/o timeout")
        || err_lower.contains("context deadline exceeded")
        || err_lower.contains("failed to create session")
        || err_lower.contains("handshake")
        || err_lower.contains("connection refused")
        || err_lower.contains("actively refused")
        || err_lower.contains("connection reset")
        || err_lower.contains("forcibly closed")
        || err_lower.contains("broken pipe")
        || err_lower.contains("eof")
        || err_lower.contains("network is unreachable")
        || err_lower.contains("no route to host")
}

/// 将内核日志行分类为代理建连失败或 TUN 默认网卡丢失/切换事件（BUG-416）。
pub(super) fn classify_kernel_log_event(line: &str) -> Option<KernelLogEvent> {
    if line.contains("[TUN] default interface lost by monitor") {
        return Some(KernelLogEvent::TunInterfaceLost);
    }
    if line.contains("[TUN] default interface changed by monitor") {
        return Some(KernelLogEvent::TunInterfaceChanged);
    }
    if is_proxy_dial_failure_log_line(line) {
        return Some(KernelLogEvent::ProxyDialFailure);
    }
    None
}

/// 从 Mihomo 结构化日志行提取 `time="..."` RFC3339 时间戳并转为 UTC。
fn parse_mihomo_log_timestamp(line: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let (_, after_time) = line.split_once("time=\"")?;
    let (ts_str, _) = after_time.split_once('"')?;
    chrono::DateTime::parse_from_rfc3339(ts_str)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// 按字节偏移增量读取 `service_latest.log` 中的完整新增行：
/// - 首次挂载或刚切换节点（`reanchor_to_end == true`）时直接锚定文件末尾，不回放旧日志；
/// - 发生日志轮转截断（`file_len < *cursor_offset`）时自动从 `0` 读取新文件；
/// - 末尾若存在尚未写完的半行（无 `\n`），保留游标等待下次补全后一并读取。
pub(super) fn read_incremental_log_lines(
    path: &Path,
    cursor_path: &mut Option<PathBuf>,
    cursor_offset: &mut u64,
    reanchor_to_end: &mut bool,
) -> Vec<String> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Vec::new();
    };
    let file_len = meta.len();

    if *reanchor_to_end || cursor_path.as_deref() != Some(path) {
        *cursor_path = Some(path.to_path_buf());
        *cursor_offset = file_len;
        *reanchor_to_end = false;
        return Vec::new();
    }

    if file_len < *cursor_offset {
        *cursor_offset = 0;
    }
    if file_len == *cursor_offset {
        return Vec::new();
    }

    let start = (*cursor_offset).max(file_len.saturating_sub(MAX_KERNEL_LOG_INCREMENTAL_BYTES));
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }

    let mut buf = Vec::with_capacity((file_len - start).min(MAX_KERNEL_LOG_INCREMENTAL_BYTES) as usize);
    if file
        .take(MAX_KERNEL_LOG_INCREMENTAL_BYTES)
        .read_to_end(&mut buf)
        .is_err()
        || buf.is_empty()
    {
        return Vec::new();
    }

    let Some(last_nl) = buf.iter().rposition(|&b| b == b'\n') else {
        return Vec::new();
    };
    let consumed = last_nl + 1;
    *cursor_offset = start + consumed as u64;
    String::from_utf8_lossy(&buf[..consumed])
        .lines()
        .map(str::to_owned)
        .collect()
}

impl ProxyDialFailureTracker {
    pub(super) fn on_node_checked(&mut self, active_node: &str, now_utc: chrono::DateTime<chrono::Utc>) {
        if self.active_node.as_deref() != Some(active_node) {
            self.active_node = Some(active_node.to_owned());
            self.active_since_utc = Some(now_utc);
            self.failure_instants.clear();
            self.interface_changed_pending = false;
            self.last_probe_at = None;
            self.reanchor_to_end = true;
        }
    }

    pub(super) fn reset_for_manual_switch(&mut self, now_utc: chrono::DateTime<chrono::Utc>) {
        self.active_since_utc = Some(now_utc);
        self.failure_instants.clear();
        self.interface_changed_pending = false;
        self.last_probe_at = None;
        self.reanchor_to_end = true;
    }

    fn prune(&mut self, now_inst: Instant) {
        self.failure_instants
            .retain(|t| now_inst.duration_since(*t).as_secs() < DIAL_FAIL_WINDOW_SECS);
    }

    pub(super) fn ingest_log_line(&mut self, line: &str, now_inst: Instant, now_utc: chrono::DateTime<chrono::Utc>) {
        let Some(event) = classify_kernel_log_event(line) else {
            return;
        };
        let event_inst = if let Some(ts) = parse_mihomo_log_timestamp(line) {
            if !matches!(event, KernelLogEvent::TunInterfaceLost)
                && self.active_since_utc.is_some_and(|since| ts < since)
            {
                return;
            }
            let age_secs = now_utc.signed_duration_since(ts).num_seconds();
            if age_secs >= DIAL_FAIL_WINDOW_SECS as i64 {
                return;
            }
            now_inst
                .checked_sub(Duration::from_secs(age_secs.max(0) as u64))
                .unwrap_or(now_inst)
        } else {
            now_inst
        };

        match event {
            KernelLogEvent::TunInterfaceLost => {
                *LAST_FLOWING_TRAFFIC_AT.lock() = None;
                FORCE_ONLINE_RECHECK.store(true, Ordering::Release);
            }
            KernelLogEvent::TunInterfaceChanged => {
                *LAST_FLOWING_TRAFFIC_AT.lock() = None;
                FORCE_ONLINE_RECHECK.store(true, Ordering::Release);
                self.interface_changed_pending = true;
            }
            KernelLogEvent::ProxyDialFailure => {
                self.prune(now_inst);
                if self.failure_instants.len() >= 64 {
                    self.failure_instants.remove(0);
                }
                self.failure_instants.push(event_inst);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn evaluate_burst(&mut self, now_inst: Instant) -> (usize, bool) {
        self.prune(now_inst);
        let count = self.failure_instants.len();
        let cooldown_active = self
            .last_probe_at
            .is_some_and(|t| now_inst.duration_since(t) < Duration::from_secs(DIAL_FAIL_REPROBE_COOLDOWN_SECS));
        (count, count >= DIAL_FAIL_REPROBE_THRESHOLD && !cooldown_active)
    }

    pub(super) fn evaluate_reprobe_signal(&mut self, now_inst: Instant) -> Option<KernelReprobeReason> {
        self.prune(now_inst);
        let cooldown_active = self
            .last_probe_at
            .is_some_and(|t| now_inst.duration_since(t) < Duration::from_secs(DIAL_FAIL_REPROBE_COOLDOWN_SECS));
        if cooldown_active {
            return None;
        }
        if self.failure_instants.len() >= DIAL_FAIL_REPROBE_THRESHOLD {
            Some(KernelReprobeReason::DialFailureBurst(self.failure_instants.len()))
        } else if self.interface_changed_pending {
            Some(KernelReprobeReason::TunInterfaceChanged)
        } else {
            None
        }
    }

    pub(super) fn consume_burst_for_probe(&mut self, now_inst: Instant) {
        self.failure_instants.clear();
        self.interface_changed_pending = false;
        self.last_probe_at = Some(now_inst);
    }
}

/// 供 Sidecar 日志消费循环直喂内核输出行（无 `"dial PROXY "` 与 `"[TUN] default interface "` 时零锁跳过，BUG-415 / BUG-416）。
pub fn record_kernel_log_line(line: &str) {
    if !line.contains("dial PROXY ") && !line.contains("[TUN] default interface ") {
        return;
    }
    let now_inst = Instant::now();
    let now_utc = chrono::Utc::now();
    PROXY_DIAL_FAILURE_TRACKER
        .lock()
        .get_or_insert_with(ProxyDialFailureTracker::default)
        .ingest_log_line(line, now_inst, now_utc);
}

async fn sync_service_kernel_logs_inner(expected_active_node: Option<&str>) {
    let is_service_mode = matches!(
        *crate::core::manager::CoreManager::global().get_running_mode(),
        crate::core::manager::RunningMode::Service
    );
    if !is_service_mode {
        return;
    }
    let Ok(log_dir) = crate::utils::dirs::service_log_dir() else {
        return;
    };

    let (cursor_path, cursor_offset, reanchor_to_end) = {
        let mut guard = PROXY_DIAL_FAILURE_TRACKER.lock();
        let tracker = guard.get_or_insert_with(ProxyDialFailureTracker::default);
        let snap = (
            tracker.cursor_path.clone(),
            tracker.cursor_offset,
            tracker.reanchor_to_end,
        );
        drop(guard);
        snap
    };

    let log_path = log_dir.join("service_latest.log");
    let (lines, next_path, next_offset, next_reanchor) = tokio::task::spawn_blocking(move || {
        let mut path_slot = cursor_path;
        let mut offset_slot = cursor_offset;
        let mut reanchor_slot = reanchor_to_end;
        let lines = read_incremental_log_lines(&log_path, &mut path_slot, &mut offset_slot, &mut reanchor_slot);
        (lines, path_slot, offset_slot, reanchor_slot)
    })
    .await
    .unwrap_or_default();

    let now_inst = Instant::now();
    let now_utc = chrono::Utc::now();
    let mut guard = PROXY_DIAL_FAILURE_TRACKER.lock();
    let tracker = guard.get_or_insert_with(ProxyDialFailureTracker::default);
    if expected_active_node.is_none_or(|node| tracker.active_node.as_deref() == Some(node)) {
        tracker.cursor_path = next_path;
        tracker.cursor_offset = next_offset;
        tracker.reanchor_to_end = next_reanchor;
        for line in lines {
            tracker.ingest_log_line(&line, now_inst, now_utc);
        }
    }
    drop(guard);
}

/// 在线态主循环探针前同步 Service 模式增量日志，及时捕捉 `[TUN] default interface lost / changed`（BUG-416）。
pub(super) async fn sync_service_kernel_logs() {
    sync_service_kernel_logs_inner(None).await;
}

pub(super) async fn poll_active_node_dial_failures(active_node: &str) -> Option<KernelReprobeReason> {
    {
        let mut guard = PROXY_DIAL_FAILURE_TRACKER.lock();
        let tracker = guard.get_or_insert_with(ProxyDialFailureTracker::default);
        tracker.on_node_checked(active_node, chrono::Utc::now());
        drop(guard);
    }

    sync_service_kernel_logs_inner(Some(active_node)).await;

    PROXY_DIAL_FAILURE_TRACKER
        .lock()
        .get_or_insert_with(ProxyDialFailureTracker::default)
        .evaluate_reprobe_signal(Instant::now())
}

/// 判定本次体检是否必须发起一轮 `PROXY__METRICS` 物理实拨：
/// 非 `Alive` 或时间戳缺失强制实拨；`Alive` 结合活跃出口流量态与代理建连失败突发分流免测（Flowing 30m / Idle 15m，Stalled 与建连失败突发冷却外破缓存）。
pub(super) fn should_probe_active_node(
    status: NodeStatus,
    age: Option<Duration>,
    traffic: ActiveTrafficState,
    stall_cooldown_active: bool,
    dial_fail_burst_ready: bool,
) -> bool {
    let NodeStatus::Alive(_) = status else {
        return true;
    };
    let Some(age) = age else {
        return true;
    };
    if dial_fail_burst_ready {
        return true;
    }

    match traffic {
        ActiveTrafficState::Flowing => age >= Duration::from_secs(PASSIVE_TRAFFIC_MAX_SKIP_SECS),
        ActiveTrafficState::Stalled => !stall_cooldown_active || age >= Duration::from_secs(IDLE_HEALTH_FRESH_SECS),
        ActiveTrafficState::Idle => age >= Duration::from_secs(IDLE_HEALTH_FRESH_SECS),
    }
}

pub(super) fn notify_observer_manual_switch(now_utc: chrono::DateTime<chrono::Utc>) {
    if let Some(tracker) = PROXY_DIAL_FAILURE_TRACKER.lock().as_mut() {
        tracker.reset_for_manual_switch(now_utc);
    }
}

pub(super) fn reset_observer_state() {
    *ACTIVE_TRAFFIC_TRACKER.lock() = None;
    *PROXY_DIAL_FAILURE_TRACKER.lock() = None;
    *LAST_HEALTH_OBSERVATION.lock() = None;
}
