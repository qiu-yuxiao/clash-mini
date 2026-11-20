use super::filter::{has_region_hint, is_dummy_node, is_valid_node_server};
use parking_lot::RwLock;
use sha2::{Digest as _, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

static RUNTIME_NODE_FINGERPRINTS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);
static RUNTIME_TAINTED_BANNER_FPS: RwLock<Option<HashSet<String>>> = RwLock::new(None);

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

/// 克隆体同指纹名称优选等级：`(!colon && region) > region > !colon > fallback`，
/// 确保横幅寄宿于真地区节点（如 `香港1`）时永远保留真地区名称，令地域排除过滤生效。
fn clone_name_rank(name: &str) -> u8 {
    let region = has_region_hint(name);
    let no_colon = is_preferred_clone_name(name);
    match (region, no_colon) {
        (true, true) => 3,
        (true, false) => 2,
        (false, true) => 1,
        (false, false) => 0,
    }
}

/// 折叠除 `name` 外连接配置完全一致的订阅克隆体（例如信息展示假名字借壳真节点配置）。
/// 若多个条目指纹相同，优先保留「带地区暗示且不含冒号」的名称，同优先级保留订阅靠前条目；
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
            if clone_name_rank(&name) > clone_name_rank(&names[existing_idx]) {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProxyEntry {
    pub name: String,
    pub fingerprint: Option<String>,
    pub is_dummy: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyTopologyOutcome {
    pub names: Vec<String>,
    pub clone_removed: usize,
    pub tainted_banner_removed: usize,
    pub tainted_fps: HashSet<String>,
}

#[derive(Debug, Clone, Copy)]
struct FpGroupStats {
    total: usize,
    has_dummy: bool,
    has_regional_non_dummy: bool,
    all_colon: bool,
}

impl Default for FpGroupStats {
    fn default() -> Self {
        Self {
            total: 0,
            has_dummy: false,
            has_regional_non_dummy: false,
            all_colon: true,
        }
    }
}

/// 先全量分析原始 `proxies` 同指纹分组，再执行伪节点连坐剔除与真节点去重归真：
/// 1. 同一连接指纹被 `>= 2` 条节点共用、不含任何带地区暗示的非伪节点，且被任一伪节点或全员冒号结构污染时，
///    判定该端点为订阅说明专用端点，整组连坐全灭并记入 `tainted_fps`；
/// 2. 若同指纹组内存在带地区暗示的真节点（如横幅挂靠 `香港1`），吞掉所有横幅并保留最高优先级地区名，
///    确保用户 `-香港` 等地域排除规则可靠命中；
/// 3. 孤立 `is_dummy` 条目单独过滤，不额外污染无指纹或孤立真实节点。
pub fn resolve_proxy_clone_topology(entries: Vec<RawProxyEntry>) -> ProxyTopologyOutcome {
    let mut group_stats: HashMap<&str, FpGroupStats> = HashMap::new();
    for entry in &entries {
        let Some(fp) = entry.fingerprint.as_deref() else {
            continue;
        };
        let stats = group_stats.entry(fp).or_default();
        stats.total += 1;
        if entry.is_dummy {
            stats.has_dummy = true;
        } else if has_region_hint(&entry.name) {
            stats.has_regional_non_dummy = true;
        }
        if is_preferred_clone_name(&entry.name) {
            stats.all_colon = false;
        }
    }

    let tainted_fps: HashSet<String> = group_stats
        .into_iter()
        .filter(|(_, s)| s.total >= 2 && !s.has_regional_non_dummy && (s.has_dummy || s.all_colon))
        .map(|(fp, _)| fp.to_owned())
        .collect();

    let mut tainted_banner_removed = 0usize;
    let mut clean_entries: Vec<(String, Option<String>)> = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.fingerprint.as_deref().is_some_and(|fp| tainted_fps.contains(fp)) {
            if !entry.is_dummy {
                tainted_banner_removed += 1;
            }
            continue;
        }
        if entry.is_dummy {
            continue;
        }
        clean_entries.push((entry.name, entry.fingerprint));
    }

    let (names, clone_removed) = deduplicate_cloned_proxies(clean_entries);
    ProxyTopologyOutcome {
        names,
        clone_removed,
        tainted_banner_removed,
        tainted_fps,
    }
}

fn extract_runtime_topology_from_sequence(seq: &[serde_yaml_ng::Value]) -> (HashMap<String, String>, HashSet<String>) {
    let mut map = HashMap::new();
    let mut raw_entries = Vec::with_capacity(seq.len());
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        let Some(name) = m.get("name").and_then(serde_yaml_ng::Value::as_str) else {
            continue;
        };
        if let Some(server) = m.get("server").and_then(serde_yaml_ng::Value::as_str) {
            if !is_valid_node_server(server) {
                continue;
            }
        }
        let fp = proxy_endpoint_fingerprint(m);
        if let Some(ref fp_str) = fp {
            map.insert(name.to_owned(), fp_str.clone());
        }
        raw_entries.push(RawProxyEntry {
            name: name.to_owned(),
            fingerprint: fp,
            is_dummy: is_dummy_node(name),
        });
    }
    let tainted = resolve_proxy_clone_topology(raw_entries).tainted_fps;
    (map, tainted)
}

/// 注册当前运行态 `proxies:` 的 `node_name -> fingerprint` 映射
pub fn set_runtime_node_fingerprints(map: HashMap<String, String>) {
    *RUNTIME_NODE_FINGERPRINTS.write() = Some(map);
}

/// 注册当前运行态被订阅拓扑连坐认定的纯横幅展示端点的连接指纹集合（仅内存态）
pub fn set_runtime_tainted_banner_fps(fps: HashSet<String>) {
    *RUNTIME_TAINTED_BANNER_FPS.write() = Some(fps);
}

fn set_runtime_topology_if_none(map: HashMap<String, String>, tainted: HashSet<String>) {
    let mut fp_guard = RUNTIME_NODE_FINGERPRINTS.write();
    if fp_guard.is_none() {
        *fp_guard = Some(map);
    }
    drop(fp_guard);
    let mut taint_guard = RUNTIME_TAINTED_BANNER_FPS.write();
    if taint_guard.is_none() {
        *taint_guard = Some(tainted);
    }
}

/// 若运行态指纹或横幅连坐集合尚未加载，尝试从 `clash-mini.yaml` 的 `proxies:` 冷启动回填
pub fn ensure_runtime_node_fingerprints_loaded() {
    if RUNTIME_NODE_FINGERPRINTS.read().is_some() && RUNTIME_TAINTED_BANNER_FPS.read().is_some() {
        return;
    }
    let home = if cfg!(test) {
        None
    } else {
        crate::utils::dirs::app_home_dir().ok()
    };
    let (map, tainted) = home
        .and_then(|dir| std::fs::read_to_string(dir.join(crate::constants::files::RUNTIME_CONFIG)).ok())
        .and_then(|content| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content).ok())
        .and_then(|val| {
            val.get("proxies")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .map(|seq| extract_runtime_topology_from_sequence(seq))
        })
        .unwrap_or_default();
    set_runtime_topology_if_none(map, tainted);
}

pub(super) fn is_tainted_banner_fp(fp: &str) -> bool {
    ensure_runtime_node_fingerprints_loaded();
    RUNTIME_TAINTED_BANNER_FPS
        .read()
        .as_ref()
        .is_some_and(|set| set.contains(fp))
}

pub(super) fn fingerprint_for_node(name: &str) -> Option<String> {
    ensure_runtime_node_fingerprints_loaded();
    RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .and_then(|map| map.get(name).cloned())
}

pub(super) fn snapshot_runtime_node_pairs() -> Vec<(String, String)> {
    let guard = RUNTIME_NODE_FINGERPRINTS.read();
    guard
        .as_ref()
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
        .unwrap_or_default()
}

pub(super) fn snapshot_runtime_tainted_banner_fps() -> HashSet<String> {
    RUNTIME_TAINTED_BANNER_FPS.read().clone().unwrap_or_default()
}

pub(super) fn runtime_fps_for_nodes(candidates: &[String]) -> Vec<String> {
    RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .map(|map| candidates.iter().filter_map(|name| map.get(name).cloned()).collect())
        .unwrap_or_default()
}

pub(super) fn map_runtime_node_names_to_fps(
    passed_names: &[String],
    failed_names: &[String],
) -> (Vec<String>, Vec<String>) {
    RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .map(|map| {
            (
                passed_names.iter().filter_map(|n| map.get(n).cloned()).collect(),
                failed_names.iter().filter_map(|n| map.get(n).cloned()).collect(),
            )
        })
        .unwrap_or_default()
}
