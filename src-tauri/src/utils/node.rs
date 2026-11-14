use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// 节点 server 字段有效性：IP 字面量，或至少含一个点的域名。
///
/// 订阅常塞入 `server: '1'` 之类的占位假节点，名字伪装成 URL（如 "v0-网址:xxx"）
/// 绕过 is_dummy_node 的名字过滤。它一旦入选 PROXY 组成员（尤其是首位），内核重启
/// 后 Selector 落回该节点，TUN 下全系统流量打进死地址，触发秒级 connect error
/// 风暴打满核心 CPU，连带 API 全部超时——3.0.5 现场 80 秒全系统断网（BUG-301）。
/// 名字可以任意伪装，server 字段藏不住，按它拦截最可靠。
pub fn is_valid_node_server(server: &str) -> bool {
    let s = server.trim();
    if s.is_empty() {
        return false;
    }
    if s.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    s.contains('.')
}

/// 把节点名归一化，便于检测「广告短语不在最开头」的伪节点。
/// 1. 剥掉两端的空白与包裹符号（【】()（）[]「」），可多层；
/// 2. 剥掉通用前缀「节点-」「节点：」等（"节点-购买入口" 这类纯伪节点）。
///
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

/// 节点名过滤匹配（与前端 `src/utils/node.ts::matchNodeFilter` 逐条对齐）。
///
/// 语法规则（大小写不敏感）：
/// - 空串 / 仅空白 / 孤立符号（`-`、`!`、`|`）：返回 `true`（避免输入未完成时列表闪烁清空）；
/// - 空格分隔多子句（`AND` 关系）；
/// - 子句内支持 `|` 或全角 `｜` 分隔多关键词（`OR` 关系，允许符号两侧带空格，如 `日本 | 新加坡`）；
/// - 以 `-`、`!` 或 `！` 开头的子句表示排除（不含对应关键词才匹配，如 `-香港`、`日本|新加坡 -倍率`）。
pub fn match_node_filter(name: &str, filter_text: &str) -> bool {
    let trimmed = filter_text.trim();
    if trimmed.is_empty() {
        return true;
    }

    let name_lower = name.to_lowercase();
    let filter_lower = trimmed.to_lowercase();
    // 归一化管道符并消除管道符两侧空白，保证「日本 | 新加坡」与「日本|新加坡」语义一致
    let collapsed = filter_lower
        .replace('｜', "|")
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("|");

    for token in collapsed.split_whitespace() {
        if token.starts_with('-') || token.starts_with('!') || token.starts_with('！') {
            for part in token.split('|') {
                let term = part
                    .strip_prefix('-')
                    .or_else(|| part.strip_prefix('!'))
                    .or_else(|| part.strip_prefix('！'))
                    .unwrap_or(part)
                    .trim();
                if !term.is_empty() && name_lower.contains(term) {
                    return false;
                }
            }
        } else {
            let branches: Vec<&str> = token.split('|').map(str::trim).filter(|b| !b.is_empty()).collect();
            if !branches.is_empty() && !branches.iter().any(|b| name_lower.contains(b)) {
                return false;
            }
        }
    }

    true
}

/// 同一节点（同指纹 `server:port+config`）连续真 HTTPS 验真失败阈值：
/// 超过该次数（`fail_count > 5`，即连续 >= 6 轮在“物理网卡在线且同批至少 1 个参照节点验真通过”
/// 的前提下仍失败，且从未通过过真 HTTPS 验真）即永久判定为假节点。
pub const HTTPS_VERIFY_MAX_FAILS: u8 = 5;

const VERIFY_STORE_FILE: &str = "node_https_verify.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeVerifyRecord {
    pub verified_true: bool,
    pub fail_count: u8,
    pub last_verified_at: i64,
}

impl NodeVerifyRecord {
    /// 当节点从未通过真 HTTPS 验真且累计失败超过 `HTTPS_VERIFY_MAX_FAILS` 次时，确认为假节点
    pub const fn is_confirmed_fake(&self) -> bool {
        !self.verified_true && self.fail_count > HTTPS_VERIFY_MAX_FAILS
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
static RUNTIME_NODE_FINGERPRINTS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

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

fn load_verify_store_from_path(path: &std::path::Path) -> NodeVerifyStoreData {
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

fn write_canonical_yaml(out: &mut String, val: &serde_yaml_ng::Value) {
    match val {
        serde_yaml_ng::Value::Null => out.push_str("null"),
        serde_yaml_ng::Value::Bool(b) => {
            out.push_str(if *b { "true" } else { "false" });
        }
        serde_yaml_ng::Value::Number(n) => {
            let _ = write!(out, "n:{n}");
        }
        serde_yaml_ng::Value::String(s) => {
            let _ = write!(out, "s:{}:{}", s.len(), s);
        }
        serde_yaml_ng::Value::Sequence(seq) => {
            out.push('[');
            for (idx, item) in seq.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_yaml(out, item);
            }
            out.push(']');
        }
        serde_yaml_ng::Value::Mapping(map) => {
            let mut entries: Vec<(String, &serde_yaml_ng::Value)> = map
                .iter()
                .map(|(k, v)| {
                    let mut ks = String::new();
                    write_canonical_yaml(&mut ks, k);
                    (ks, v)
                })
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            out.push('{');
            for (idx, (k, v)) in entries.into_iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&k);
                out.push(':');
                write_canonical_yaml(out, v);
            }
            out.push('}');
        }
        serde_yaml_ng::Value::Tagged(tagged) => {
            let _ = write!(out, "t:{}:", tagged.tag);
            write_canonical_yaml(out, &tagged.value);
        }
    }
}

/// 为单个代理节点映射生成规范化连接指纹（SHA-256 hex，去掉 `name` 后按字段键排序哈希）。
/// 仅当 `server`（非空字符串）与 `port`（正整数或非空字符串）同时存在时返回 `Some(fp)`。
pub fn proxy_endpoint_fingerprint(mapping: &serde_yaml_ng::Mapping) -> Option<String> {
    let server_ok = mapping
        .get("server")
        .and_then(serde_yaml_ng::Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let port_ok = mapping
        .get("port")
        .is_some_and(|v| v.as_u64().is_some_and(|p| p > 0) || v.as_str().is_some_and(|s| !s.trim().is_empty()));
    if !server_ok || !port_ok {
        return None;
    }

    let mut entries: Vec<(String, &serde_yaml_ng::Value)> = mapping
        .iter()
        .filter(|(k, _)| k.as_str() != Some("name"))
        .map(|(k, v)| {
            let mut ks = String::new();
            write_canonical_yaml(&mut ks, k);
            (ks, v)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut canonical = String::new();
    for (k, v) in entries {
        canonical.push_str(&k);
        canonical.push('=');
        write_canonical_yaml(&mut canonical, v);
        canonical.push(';');
    }

    let digest = Sha256::digest(canonical.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Some(hex)
}

fn is_preferred_clone_name(name: &str) -> bool {
    !name.contains(':') && !name.contains('：')
}

/// 折叠除 `name` 外连接配置完全一致的订阅克隆体（例如信息展示假名字借壳真节点配置）。
/// 若多个条目指纹相同，优先保留名称不含冒号 `:` / `：` 的条目，同优先级保留订阅靠前条目；
/// 没有指纹（`None`）的条目原样保留。返回 `(去重后名称列表, 被折叠条数)`。
pub fn deduplicate_cloned_proxies(entries: Vec<(String, Option<String>)>) -> (Vec<String>, usize) {
    let mut names: Vec<String> = Vec::with_capacity(entries.len());
    let mut fp_to_idx: HashMap<String, usize> = HashMap::new();
    let mut removed = 0usize;

    for (name, fp_opt) in entries {
        let Some(fp) = fp_opt else {
            names.push(name);
            continue;
        };
        if let Some(&existing_idx) = fp_to_idx.get(&fp) {
            removed += 1;
            if is_preferred_clone_name(&name) && !is_preferred_clone_name(&names[existing_idx]) {
                names[existing_idx] = name;
            }
        } else {
            let idx = names.len();
            fp_to_idx.insert(fp, idx);
            names.push(name);
        }
    }

    (names, removed)
}

fn extract_fingerprints_from_sequence(seq: &[serde_yaml_ng::Value]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        let Some(name) = m.get("name").and_then(serde_yaml_ng::Value::as_str) else {
            continue;
        };
        if let Some(fp) = proxy_endpoint_fingerprint(m) {
            map.insert(name.to_owned(), fp);
        }
    }
    map
}

/// 注册当前运行态 `proxies:` 的 `node_name -> fingerprint` 映射
pub fn set_runtime_node_fingerprints(map: HashMap<String, String>) {
    let mut guard = RUNTIME_NODE_FINGERPRINTS.write();
    *guard = Some(map);
}

/// 若运行态指纹映射尚未加载，尝试从 `clash-mini.yaml` 的 `proxies:` 冷启动回填
pub fn ensure_runtime_node_fingerprints_loaded() {
    if RUNTIME_NODE_FINGERPRINTS.read().is_some() {
        return;
    }
    let home = if cfg!(test) {
        None
    } else {
        crate::utils::dirs::app_home_dir().ok()
    };
    let map = home
        .and_then(|dir| std::fs::read_to_string(dir.join(crate::constants::files::RUNTIME_CONFIG)).ok())
        .and_then(|content| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content).ok())
        .and_then(|val| {
            val.get("proxies")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .map(|seq| extract_fingerprints_from_sequence(seq))
        })
        .unwrap_or_default();
    let mut guard = RUNTIME_NODE_FINGERPRINTS.write();
    if guard.is_none() {
        *guard = Some(map);
    }
}

/// 查询指定指纹是否已被确认永久拉黑
pub fn is_confirmed_fake_fp(fp: &str) -> bool {
    ensure_verify_store_loaded();
    VERIFY_STORE
        .read()
        .as_ref()
        .is_some_and(|store| store.records.get(fp).is_some_and(NodeVerifyRecord::is_confirmed_fake))
}

/// 查询指定指纹是否已通过真 HTTPS 验真（单向粘滞真值）
pub fn is_verified_true_fp(fp: &str) -> bool {
    ensure_verify_store_loaded();
    VERIFY_STORE
        .read()
        .as_ref()
        .is_some_and(|store| store.records.get(fp).is_some_and(|r| r.verified_true))
}

fn fingerprint_for_node(name: &str) -> Option<String> {
    ensure_runtime_node_fingerprints_loaded();
    RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .and_then(|map| map.get(name).cloned())
}

/// 查询节点名是否已被确认假节点（通过当前指纹表匹配；无指纹时返回 `false`）
pub fn is_confirmed_fake_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_confirmed_fake_fp(&fp))
}

/// 查询节点名是否已通过真 HTTPS 验真
pub fn is_verified_true_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_verified_true_fp(&fp))
}

/// 查询节点名是否曾在有参照节点通过的前提下累计过真 HTTPS 验真失败
pub fn has_verify_failure_node(name: &str) -> bool {
    let Some(fp) = fingerprint_for_node(name) else {
        return false;
    };
    ensure_verify_store_loaded();
    VERIFY_STORE.read().as_ref().is_some_and(|store| {
        store
            .records
            .get(&fp)
            .is_some_and(|r| !r.verified_true && r.fail_count > 0)
    })
}

/// 返回当前运行配置中所有已确认假节点的名称列表（供前端隐藏显示）
pub fn confirmed_fake_node_names() -> Vec<String> {
    ensure_runtime_node_fingerprints_loaded();
    let pairs: Vec<(String, String)> = {
        let guard = RUNTIME_NODE_FINGERPRINTS.read();
        guard
            .as_ref()
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    if pairs.is_empty() {
        return Vec::new();
    }
    ensure_verify_store_loaded();
    let mut names = Vec::new();
    if let Some(store) = VERIFY_STORE.read().as_ref() {
        for (name, fp) in pairs {
            if store.records.get(&fp).is_some_and(NodeVerifyRecord::is_confirmed_fake) {
                names.push(name);
            }
        }
    }
    names
}

/// 判断给定候选节点中是否仍存在“未验真真值且尚未确认拉黑”的待观察节点
pub fn has_pending_unverified_nodes(candidates: &[String]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    ensure_runtime_node_fingerprints_loaded();
    let fps: Vec<String> = RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .map(|map| candidates.iter().filter_map(|name| map.get(name).cloned()).collect())
        .unwrap_or_default();
    if fps.is_empty() {
        return false;
    }
    ensure_verify_store_loaded();
    VERIFY_STORE.read().as_ref().is_none_or(|store| {
        fps.iter().any(|fp| {
            store
                .records
                .get(fp)
                .is_none_or(|rec| !rec.verified_true && !rec.is_confirmed_fake())
        })
    })
}

/// 纯状态转移函数：将一批真 HTTPS 验真结果应用到指纹记录表。
///
/// 不变量：
/// 1. `verified_true` 为单向粘滞标记：一旦为 `true` 永不降级、不再累加 `fail_count`。
/// 2. 只有当 `passed_fps` 非空（至少 1 个参照节点通过真 HTTPS 验真）时，才对同批未通过的未验真指纹累加 `fail_count`。
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
        if rec.verified_true || rec.is_confirmed_fake() {
            continue;
        }
        rec.fail_count = rec.fail_count.saturating_add(1);
        rec.last_verified_at = now_ts;
        outcome.dirty = true;
        if rec.is_confirmed_fake() {
            outcome.newly_confirmed_fake += 1;
        }
    }

    outcome
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
    let (passed_fps, failed_fps): (Vec<String>, Vec<String>) = RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .map(|map| {
            (
                passed_names.iter().filter_map(|n| map.get(n).cloned()).collect(),
                failed_names.iter().filter_map(|n| map.get(n).cloned()).collect(),
            )
        })
        .unwrap_or_default();
    if passed_fps.is_empty() && failed_fps.is_empty() {
        return VerifyBatchOutcome::default();
    }

    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    let (outcome, snapshot) = apply_and_snapshot_store(&mut VERIFY_STORE.write(), &passed_fps, &failed_fps, now_ts);

    if let Some(store_snapshot) = snapshot {
        if let Some(path) = verify_store_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(err) = crate::utils::help::save_json(&path, &store_snapshot) {
                clash_verge_logging::logging!(
                    warn,
                    clash_verge_logging::Type::Config,
                    "保存节点 HTTPS 验真状态失败: {err}"
                );
            }
        }
    }

    outcome
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        HTTPS_VERIFY_MAX_FAILS, NodeVerifyRecord, NodeVerifyStoreData, apply_verify_batch_to_records,
        deduplicate_cloned_proxies, is_dummy_node, is_valid_node_server, load_verify_store_from_path,
        match_node_filter, proxy_endpoint_fingerprint,
    };
    use std::collections::HashMap;

    #[test]
    fn test_valid_node_server() {
        // 有效：IP 字面量与正常域名
        assert!(is_valid_node_server("1.2.3.4"));
        assert!(is_valid_node_server("::1"));
        assert!(is_valid_node_server("a03.jiffdl.net"));
        assert!(is_valid_node_server(" jp.example.com "));
        // 无效：占位假节点（BUG-301 现场 server='1'）与空值
        assert!(!is_valid_node_server("1"));
        assert!(!is_valid_node_server(""));
        assert!(!is_valid_node_server("   "));
    }

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

    #[test]
    fn test_match_node_filter() {
        // 空值、纯空白及孤立符号全部放行（避免输入未完成时列表闪烁清空）
        for empty in ["", "   ", "-", "!", "！", "|", "｜", "  -  "] {
            assert!(match_node_filter("香港1 (005024)", empty), "empty-like '{empty}'");
        }
        assert!(match_node_filter("日本 01 [IPLC]", "日本 -"));

        // 单关键词大小写不敏感匹配
        assert!(match_node_filter("HK-BGP-01", "hk"));
        assert!(match_node_filter("日本 01 [IPLC]", "日本"));
        assert!(!match_node_filter("新加坡 01", "日本"));

        // 多关键词 OR 匹配（半角 | / 全角 ｜ / 两侧空格 / 尾部悬空 |）
        let or_queries = ["日本|新加坡|台湾", "日本 | 新加坡 ｜ 台湾", "日本|"];
        for q in or_queries {
            assert!(match_node_filter("日本 01", q), "query '{q}'");
            if !q.ends_with('|') {
                assert!(match_node_filter("新加坡-专线", q), "query '{q}'");
                assert!(match_node_filter("台湾-原生", q), "query '{q}'");
            }
            assert!(!match_node_filter("香港1 (005024)", q), "query '{q}'");
        }

        // 纯排除语法（- / ! / ！）与多排除
        assert!(!match_node_filter("香港1 (005024)", "-香港"));
        assert!(match_node_filter("日本 01", "-香港"));
        assert!(!match_node_filter("HK-01", "!hk"));
        assert!(!match_node_filter("高倍率 3x", "！倍率"));
        assert!(!match_node_filter("香港-专线", "-香港 -英国"));
        assert!(!match_node_filter("英国-01", "-香港|-英国"));
        assert!(match_node_filter("新加坡-01", "-香港|-英国"));

        // 组合语法：OR + AND + 排除
        assert!(match_node_filter("日本 专线 01", "日本 | 新加坡 -倍率 -香港"));
        assert!(match_node_filter("新加坡 BGP 02", "日本 | 新加坡 -倍率 -香港"));
        assert!(!match_node_filter("日本 2x高倍率", "日本 | 新加坡 -倍率 -香港"));
        assert!(!match_node_filter("香港-日本中转", "日本 | 新加坡 -倍率 -香港"));

        assert!(match_node_filter("HK BGP IPLC-01", "hk bgp -倍率"));
        assert!(!match_node_filter("HK BGP 3倍率", "hk bgp -倍率"));

        // 含内部连字符与空格的真实节点名
        assert!(match_node_filter("英国-3倍率", "英国-3倍率"));
        assert!(match_node_filter("香港1 (005024)", "香港1 (005024)"));
    }

    #[test]
    fn test_proxy_endpoint_fingerprint_ignores_name_and_key_order() {
        let a: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r"
name: 距离下次重置剩余：25 天
type: anytls
server: 1.2.3.4
port: 443
password: secret
sni: example.com
",
        )
        .unwrap();
        let b: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r"
sni: example.com
password: secret
port: 443
server: 1.2.3.4
type: anytls
name: 香港1
",
        )
        .unwrap();
        let c: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r"
name: 香港2
type: anytls
server: 1.2.3.5
port: 443
password: secret
",
        )
        .unwrap();
        let missing_port: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r"
name: 无端口
type: anytls
server: 1.2.3.4
",
        )
        .unwrap();

        let fp_a = proxy_endpoint_fingerprint(&a).unwrap();
        let fp_b = proxy_endpoint_fingerprint(&b).unwrap();
        let fp_c = proxy_endpoint_fingerprint(&c).unwrap();
        assert_eq!(fp_a, fp_b);
        assert_ne!(fp_a, fp_c);
        assert!(proxy_endpoint_fingerprint(&missing_port).is_none());
    }

    #[test]
    fn test_deduplicate_cloned_proxies_prefers_non_colon_name() {
        let entries = vec![
            ("距离下次重置剩余：25 天".to_owned(), Some("fp1".to_owned())),
            ("日本1".to_owned(), Some("fp2".to_owned())),
            ("香港1".to_owned(), Some("fp1".to_owned())),
            ("无指纹直连".to_owned(), None),
            ("新加坡：备用".to_owned(), Some("fp3".to_owned())),
            ("新加坡：主".to_owned(), Some("fp3".to_owned())),
        ];
        let (names, removed) = deduplicate_cloned_proxies(entries);
        assert_eq!(removed, 2);
        assert_eq!(names, vec!["香港1", "日本1", "无指纹直连", "新加坡：备用"]);
    }

    #[test]
    fn test_verify_state_machine_sticky_pass_threshold_and_offline_guard() {
        let mut records: HashMap<String, NodeVerifyRecord> = HashMap::new();
        let good = "fp-good".to_owned();
        let dead = "fp-dead".to_owned();

        // 0 个参照节点通过时，禁止累加任何节点的 fail_count（防止断网/梯子挂了误杀真节点）
        let o0 = apply_verify_batch_to_records(&mut records, &[], &[good.clone(), dead.clone()], 100);
        assert!(!o0.dirty);
        assert!(records.is_empty());

        // good 验真通过 1 次 -> 单向粘滞为真；dead 同批失败 -> fail_count = 1
        let o1 = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&good),
            std::slice::from_ref(&dead),
            101,
        );
        assert_eq!(o1.newly_verified, 1);
        assert_eq!(o1.newly_confirmed_fake, 0);
        assert!(records[&good].verified_true);
        assert_eq!(records[&dead].fail_count, 1);
        assert!(!records[&dead].is_confirmed_fake());

        // good 随后即使作为参照失败也不会降级，fail_count 保持 0
        for round in 2..=HTTPS_VERIFY_MAX_FAILS {
            let o = apply_verify_batch_to_records(
                &mut records,
                &["fp-ref".to_owned()],
                &[good.clone(), dead.clone()],
                100 + i64::from(round),
            );
            assert_eq!(o.newly_confirmed_fake, 0);
            assert!(records[&good].verified_true);
            assert_eq!(records[&good].fail_count, 0);
            assert_eq!(records[&dead].fail_count, round);
            assert!(!records[&dead].is_confirmed_fake());
        }

        // 第 6 次失败（超过 HTTPS_VERIFY_MAX_FAILS = 5）-> 确认假节点
        let o6 = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&good),
            std::slice::from_ref(&dead),
            200,
        );
        assert_eq!(o6.newly_confirmed_fake, 1);
        assert!(records[&dead].is_confirmed_fake());

        // 已确认假节点再次失败不重复计数
        let o7 = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&good),
            std::slice::from_ref(&dead),
            201,
        );
        assert_eq!(o7.newly_confirmed_fake, 0);
    }

    #[test]
    fn test_verify_store_json_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("clash_mini_verify_store_test_{}.json", std::process::id()));
        let mut store = NodeVerifyStoreData::default();
        store.records.insert(
            "abc".to_owned(),
            NodeVerifyRecord {
                verified_true: true,
                fail_count: 0,
                last_verified_at: 123_456,
            },
        );
        let serialized = serde_json::to_string_pretty(&store).unwrap();
        std::fs::write(&tmp, serialized).unwrap();

        let loaded = load_verify_store_from_path(&tmp);
        assert_eq!(loaded.records.get("abc"), store.records.get("abc"));
        let _ = std::fs::remove_file(&tmp);

        let missing = load_verify_store_from_path(&tmp);
        assert!(missing.records.is_empty());
    }
}
