use super::fingerprint::{
    ensure_runtime_node_fingerprints_loaded, fingerprint_for_node, is_tainted_banner_fp, map_runtime_node_names_to_fps,
    runtime_fps_for_nodes, snapshot_runtime_node_pairs, snapshot_runtime_tainted_banner_fps,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// 活跃节点触发即时验真退避迁移所需的最小连续失败次数（在物理在线且有参照节点通过的前提下）：
/// 避免仅 1 轮偶发 TLS 抖动就误判为需迁移，连续 >= 2 轮失败再允许切换到已验真节点。
pub const HTTPS_VERIFY_MIGRATE_MIN_FAILS: u8 = 2;

/// 同一节点（同指纹 `server:port+config`）连续真 HTTPS 验真失败阈值：
/// 超过该次数（`fail_count > 5`，即连续 >= 6 轮在“物理网卡在线且同批至少 1 个参照节点验真通过”
/// 的前提下仍失败，且从未通过过真 HTTPS 验真）即进入 24 小时软隔离。
pub const HTTPS_VERIFY_MAX_FAILS: u8 = 5;

/// 真 HTTPS 验真软隔离过期时长（秒，24 小时）：
/// 避免机场临时维护或验证靶站波动导致可用节点被永久不可逆清除；期满自动解除隔离并在后续周期重新观察。
pub const HTTPS_VERIFY_BAN_TTL_SECS: i64 = 86_400;

const VERIFY_STORE_FILE: &str = "node_https_verify.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeVerifyRecord {
    pub verified_true: bool,
    pub fail_count: u8,
    pub last_verified_at: i64,
}

impl NodeVerifyRecord {
    /// 判断在指定 Unix 时间戳 `now_ts` 下节点是否仍处于「软隔离假节点」窗口内。
    /// 仅当从未通过真 HTTPS 验真、累计失败超过 `HTTPS_VERIFY_MAX_FAILS` 次且距上次失败未满 `HTTPS_VERIFY_BAN_TTL_SECS`（24h）时为 `true`。
    pub const fn is_confirmed_fake_at(&self, now_ts: i64) -> bool {
        if self.verified_true || self.fail_count <= HTTPS_VERIFY_MAX_FAILS {
            return false;
        }
        if self.last_verified_at <= 0 {
            return true;
        }
        now_ts.saturating_sub(self.last_verified_at) < HTTPS_VERIFY_BAN_TTL_SECS
    }

    /// 按当前系统时钟判断节点是否处于软隔离假节点状态
    pub fn is_confirmed_fake(&self) -> bool {
        self.is_confirmed_fake_at(current_unix_ts())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeVerifyStoreData {
    pub records: HashMap<String, NodeVerifyRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifyBatchOutcome {
    pub newly_verified: usize,
    pub newly_confirmed_fake: usize,
    pub dirty: bool,
}

static VERIFY_STORE: RwLock<Option<NodeVerifyStoreData>> = RwLock::new(None);

fn current_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn verify_store_path() -> Option<std::path::PathBuf> {
    if cfg!(test) {
        return None;
    }
    crate::utils::dirs::app_home_dir()
        .ok()
        .map(|dir| dir.join(VERIFY_STORE_FILE))
}

pub(super) fn load_verify_store_from_path(path: &std::path::Path) -> NodeVerifyStoreData {
    std::fs::read_to_string(path).map_or_else(
        |_| NodeVerifyStoreData::default(),
        |content| serde_json::from_str::<NodeVerifyStoreData>(&content).unwrap_or_default(),
    )
}

fn ensure_verify_store_loaded() {
    if VERIFY_STORE.read().is_some() {
        return;
    }
    let loaded = verify_store_path().map_or_else(NodeVerifyStoreData::default, |p| load_verify_store_from_path(&p));
    let mut guard = VERIFY_STORE.write();
    if guard.is_none() {
        *guard = Some(loaded);
    }
}

/// 仅查询指纹是否处于持久化真 HTTPS 验真 24h 软隔离窗口内（不含内存级拓扑连坐集合，供配置重构独立重算）
pub fn is_https_quarantined_fp(fp: &str) -> bool {
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    VERIFY_STORE.read().as_ref().is_some_and(|store| {
        store
            .records
            .get(fp)
            .is_some_and(|rec| rec.is_confirmed_fake_at(now_ts))
    })
}

/// 查询指定指纹是否应按假节点屏蔽（含真 HTTPS 验真 24h 软隔离，以及同配置横幅连坐集合）
pub fn is_confirmed_fake_fp(fp: &str) -> bool {
    is_https_quarantined_fp(fp) || is_tainted_banner_fp(fp)
}

/// 查询指定指纹是否已通过真 HTTPS 验真（单向粘滞真值）
pub fn is_verified_true_fp(fp: &str) -> bool {
    ensure_verify_store_loaded();
    VERIFY_STORE
        .read()
        .as_ref()
        .is_some_and(|store| store.records.get(fp).is_some_and(|r| r.verified_true))
}

/// 查询节点名是否已被确认假节点（处于软隔离期内；无指纹时返回 `false`）
pub fn is_confirmed_fake_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_confirmed_fake_fp(&fp))
}

/// 查询节点名是否已通过真 HTTPS 验真
pub fn is_verified_true_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_verified_true_fp(&fp))
}

/// 纯判定函数（便于单测锁定）：判断单条验真记录是否满足活跃节点即时验真退避迁移门槛
pub(super) const fn record_has_verify_failure(
    verified_true: bool,
    fail_count: u8,
    last_verified_at: i64,
    now_ts: i64,
) -> bool {
    !verified_true
        && fail_count >= HTTPS_VERIFY_MIGRATE_MIN_FAILS
        && (last_verified_at <= 0 || now_ts.saturating_sub(last_verified_at) < HTTPS_VERIFY_BAN_TTL_SECS)
}

/// 查询节点名是否在有效观察期（未过 24h TTL）内曾在有参照节点通过的前提下累计过达到迁移阈值的真 HTTPS 验真失败
pub fn has_verify_failure_node(name: &str) -> bool {
    let Some(fp) = fingerprint_for_node(name) else {
        return false;
    };
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    VERIFY_STORE.read().as_ref().is_some_and(|store| {
        store
            .records
            .get(&fp)
            .is_some_and(|r| record_has_verify_failure(r.verified_true, r.fail_count, r.last_verified_at, now_ts))
    })
}

/// 返回当前运行配置中所有仍在软隔离期内或被拓扑连坐认定的假节点名称列表（供前端隐藏显示）
pub fn confirmed_fake_node_names() -> Vec<String> {
    ensure_runtime_node_fingerprints_loaded();
    let pairs = snapshot_runtime_node_pairs();
    if pairs.is_empty() {
        return Vec::new();
    }
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    let tainted_fps = snapshot_runtime_tainted_banner_fps();
    let https_fake_fps: HashSet<String> = VERIFY_STORE
        .read()
        .as_ref()
        .map(|store| {
            pairs
                .iter()
                .filter(|(_, fp)| {
                    !tainted_fps.contains(fp)
                        && store
                            .records
                            .get(fp)
                            .is_some_and(|rec| rec.is_confirmed_fake_at(now_ts))
                })
                .map(|(_, fp)| fp.clone())
                .collect()
        })
        .unwrap_or_default();
    pairs
        .into_iter()
        .filter(|(_, fp)| tainted_fps.contains(fp) || https_fake_fps.contains(fp))
        .map(|(name, _)| name)
        .collect()
}

/// 判断给定候选节点中是否仍存在“未验真真值且当前未处于软隔离期内”的待观察节点
pub fn has_pending_unverified_nodes(candidates: &[String]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    ensure_runtime_node_fingerprints_loaded();
    let fps = runtime_fps_for_nodes(candidates);
    if fps.is_empty() {
        return false;
    }
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    let tainted_fps = snapshot_runtime_tainted_banner_fps();
    let store_guard = VERIFY_STORE.read();
    store_guard.as_ref().is_none_or(|store| {
        fps.iter().any(|fp| {
            !tainted_fps.contains(fp)
                && store
                    .records
                    .get(fp)
                    .is_none_or(|rec| !rec.verified_true && !rec.is_confirmed_fake_at(now_ts))
        })
    })
}

/// 纯状态转移函数：将一批真 HTTPS 验真结果应用到指纹记录表。
///
/// 不变量：
/// 1. `verified_true` 为单向粘滞标记：一旦为 `true` 永不降级、不再累加 `fail_count`。
/// 2. 只有当 `passed_fps` 非空（至少 1 个参照节点通过真 HTTPS 验真）时，才对同批未通过的未验真指纹累加 `fail_count`。
/// 3. 软隔离期满 24h 自动解除；期满后再次失败时先清零旧计数再重新计入新观察周期。
pub fn apply_verify_batch_to_records(
    records: &mut HashMap<String, NodeVerifyRecord>,
    passed_fps: &[String],
    failed_fps: &[String],
    now_ts: i64,
) -> VerifyBatchOutcome {
    let mut outcome = VerifyBatchOutcome::default();
    let mut passed_set = HashSet::with_capacity(passed_fps.len());

    for fp in passed_fps {
        if !passed_set.insert(fp.as_str()) {
            continue;
        }
        let rec = records.entry(fp.clone()).or_default();
        if !rec.verified_true {
            rec.verified_true = true;
            outcome.newly_verified += 1;
            outcome.dirty = true;
        }
        if rec.fail_count != 0 {
            rec.fail_count = 0;
            outcome.dirty = true;
        }
        if rec.last_verified_at != now_ts {
            rec.last_verified_at = now_ts;
            outcome.dirty = true;
        }
    }

    if passed_set.is_empty() {
        return outcome;
    }

    let mut seen_failed = HashSet::with_capacity(failed_fps.len());
    for fp in failed_fps {
        if passed_set.contains(fp.as_str()) || !seen_failed.insert(fp.as_str()) {
            continue;
        }
        let rec = records.entry(fp.clone()).or_default();
        if rec.verified_true || rec.is_confirmed_fake_at(now_ts) {
            continue;
        }
        // 24h 软隔离已到期：清零旧累计失败次数，重新开启一轮 6 次宽限观察
        if rec.fail_count > HTTPS_VERIFY_MAX_FAILS {
            rec.fail_count = 0;
        }
        rec.fail_count = rec.fail_count.saturating_add(1);
        rec.last_verified_at = now_ts;
        outcome.dirty = true;
        if rec.is_confirmed_fake_at(now_ts) {
            outcome.newly_confirmed_fake += 1;
        }
    }

    outcome
}

pub(super) fn clear_unverified_failures_in_records(records: &mut HashMap<String, NodeVerifyRecord>) -> usize {
    let mut cleared = 0usize;
    records.retain(|_, rec| {
        if !rec.verified_true && rec.fail_count > 0 {
            cleared += 1;
            false
        } else {
            true
        }
    });
    cleared
}

fn persist_verify_store_snapshot(store_snapshot: &NodeVerifyStoreData) {
    if let Some(path) = verify_store_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(err) = crate::utils::help::save_json(&path, store_snapshot) {
            clash_verge_logging::logging!(
                warn,
                clash_verge_logging::Type::Config,
                "保存节点 HTTPS 验真状态失败: {err}"
            );
        }
    }
}

fn clear_and_snapshot_store(store_slot: &mut Option<NodeVerifyStoreData>) -> (usize, Option<NodeVerifyStoreData>) {
    let store = store_slot.get_or_insert_with(NodeVerifyStoreData::default);
    let cleared = clear_unverified_failures_in_records(&mut store.records);
    let snapshot = (cleared > 0).then(|| store.clone());
    (cleared, snapshot)
}

/// 清空所有未验真节点的历史 HTTPS 验真失败记录与软隔离标记（保留已通过 `verified_true` 的记录），
/// 供用户手动更新订阅时立即恢复被隔离节点进入观测池。
pub fn clear_unverified_https_failures() -> usize {
    ensure_verify_store_loaded();
    let (cleared, snapshot) = clear_and_snapshot_store(&mut VERIFY_STORE.write());
    if let Some(store_snapshot) = snapshot {
        persist_verify_store_snapshot(&store_snapshot);
    }
    cleared
}

/// 测试辅助：向内存 `VERIFY_STORE` 写入或移除单条指纹记录（仅在 `#[cfg(test)]` 编译）
#[cfg(test)]
pub fn set_verify_record_for_test(fp: &str, rec: Option<NodeVerifyRecord>) {
    ensure_verify_store_loaded();
    if let Some(r) = rec {
        VERIFY_STORE
            .write()
            .get_or_insert_with(NodeVerifyStoreData::default)
            .records
            .insert(fp.to_owned(), r);
    } else {
        VERIFY_STORE
            .write()
            .get_or_insert_with(NodeVerifyStoreData::default)
            .records
            .remove(fp);
    }
}

fn apply_and_snapshot_store(
    store_slot: &mut Option<NodeVerifyStoreData>,
    passed_fps: &[String],
    failed_fps: &[String],
    now_ts: i64,
) -> (VerifyBatchOutcome, Option<NodeVerifyStoreData>) {
    let store = store_slot.get_or_insert_with(NodeVerifyStoreData::default);
    let outcome = apply_verify_batch_to_records(&mut store.records, passed_fps, failed_fps, now_ts);
    let snapshot = outcome.dirty.then(|| store.clone());
    (outcome, snapshot)
}

/// 将一轮真 HTTPS 验真的节点名称结果写入持久化状态机（按当前运行时指纹归集）
pub fn record_https_verify_results(passed_names: &[String], failed_names: &[String]) -> VerifyBatchOutcome {
    ensure_runtime_node_fingerprints_loaded();
    let (passed_fps, failed_fps) = map_runtime_node_names_to_fps(passed_names, failed_names);
    if passed_fps.is_empty() && failed_fps.is_empty() {
        return VerifyBatchOutcome::default();
    }

    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    let (outcome, snapshot) = apply_and_snapshot_store(&mut VERIFY_STORE.write(), &passed_fps, &failed_fps, now_ts);

    if let Some(store_snapshot) = snapshot {
        persist_verify_store_snapshot(&store_snapshot);
    }

    outcome
}
