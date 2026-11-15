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
    is_structural_banner_name(name, &lower)
}

const REGION_SUBSTRINGS_CJK: &[&str] = &[
    "香港",
    "台湾",
    "台灣",
    "澳门",
    "澳門",
    "日本",
    "新加坡",
    "狮城",
    "美国",
    "美國",
    "韩国",
    "韓國",
    "首尔",
    "东京",
    "大阪",
    "硅谷",
    "洛杉矶",
    "西雅图",
    "圣何塞",
    "纽约",
    "英国",
    "英國",
    "伦敦",
    "德国",
    "德國",
    "法兰克福",
    "法国",
    "法國",
    "巴黎",
    "荷兰",
    "荷蘭",
    "阿姆斯特丹",
    "俄罗斯",
    "俄羅斯",
    "莫斯科",
    "加拿大",
    "多伦多",
    "温哥华",
    "澳洲",
    "澳大利亚",
    "悉尼",
    "墨尔本",
    "新西兰",
    "印度",
    "孟买",
    "印尼",
    "雅加达",
    "马来西亚",
    "吉隆坡",
    "泰国",
    "曼谷",
    "越南",
    "菲律宾",
    "土耳其",
    "伊斯坦布尔",
    "阿根廷",
    "巴西",
    "圣保罗",
    "智利",
    "墨西哥",
    "南非",
    "埃及",
    "阿联酋",
    "迪拜",
    "以色列",
    "乌克兰",
    "波兰",
    "瑞典",
    "瑞士",
    "苏黎世",
    "意大利",
    "西班牙",
    "爱尔兰",
    "芬兰",
    "挪威",
    "丹麦",
    "奥地利",
    "比利时",
    "捷克",
    "匈牙利",
    "罗马尼亚",
    "卢森堡",
    "冰岛",
    "葡萄牙",
    "希腊",
    "中国",
    "上海",
    "北京",
    "广州",
    "深圳",
    "欧洲",
    "北美",
    "南美",
    "亚太",
    "hong kong",
];

const REGION_ASCII_TOKENS: &[&str] = &[
    "hk",
    "hkg",
    "hongkong",
    "tw",
    "tpe",
    "taiwan",
    "mo",
    "mac",
    "jp",
    "jpn",
    "japan",
    "tyo",
    "osa",
    "nrt",
    "kix",
    "tokyo",
    "osaka",
    "sg",
    "sgp",
    "sin",
    "singapore",
    "usa",
    "america",
    "lax",
    "sjc",
    "sea",
    "ord",
    "ewr",
    "nyc",
    "iad",
    "dfw",
    "kr",
    "kor",
    "korea",
    "icn",
    "sel",
    "seoul",
    "uk",
    "gbr",
    "britain",
    "england",
    "lon",
    "london",
    "de",
    "deu",
    "germany",
    "fra",
    "frankfurt",
    "fr",
    "france",
    "par",
    "paris",
    "nl",
    "nld",
    "netherlands",
    "ams",
    "amsterdam",
    "ru",
    "rus",
    "russia",
    "mow",
    "moscow",
    "ca",
    "can",
    "canada",
    "yyz",
    "yvr",
    "toronto",
    "au",
    "aus",
    "australia",
    "syd",
    "sydney",
    "mel",
    "ind",
    "india",
    "bom",
    "idn",
    "indonesia",
    "cgk",
    "mys",
    "malaysia",
    "kul",
    "th",
    "tha",
    "thailand",
    "bkk",
    "vn",
    "vnm",
    "vietnam",
    "ph",
    "phl",
    "philippines",
    "mnl",
    "tr",
    "tur",
    "turkey",
    "ist",
    "arg",
    "argentina",
    "bra",
    "brazil",
    "gru",
    "uae",
    "dxb",
    "dubai",
    "bgp",
    "iplc",
    "iepl",
];

const BANNER_DOMAIN_TLDS: &[&str] = &[
    "com", "net", "org", "xyz", "top", "vip", "cc", "io", "me", "club", "link", "site", "shop", "info", "icu", "cyou",
    "online", "store", "tech", "app", "dev", "cloud", "pw", "co", "tv", "im", "la", "pro", "ltd", "work", "fun",
    "live", "ink", "buzz", "click", "cn", "hk", "tw", "jp", "sg", "uk", "us", "ru", "kr", "de",
];

const fn is_code_context_byte(b: Option<u8>) -> bool {
    matches!(b, Some(b'-' | b'_' | b'[' | b']' | b'0'..=b'9'))
}

/// 判断节点名是否带有地区或专线暗示（必须与前端 `src/utils/node.ts::hasRegionHint` 逐条对齐）。
/// 用于区分真实代理节点与不含地区名的订阅说明横幅，防止结构形态识别及拓扑消歧误杀真实节点。
pub fn has_region_hint(name: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }
    // 1. 国旗 Emoji（Regional Indicator Symbols）
    if name.chars().any(|c| ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)) {
        return true;
    }
    // 2. 中文地区/城市名与带空格地区词
    let lower = name.to_lowercase();
    if REGION_SUBSTRINGS_CJK.iter().any(|kw| lower.contains(kw)) {
        return true;
    }
    // 3. 独立边界的 ASCII 地区/专线字母段（邻接 `.` 时跳过，避免把官网后缀 `.hk`/`.us` 误作地区）
    let bytes = name.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        let end = i;
        let left = start.checked_sub(1).and_then(|idx| bytes.get(idx).copied());
        let right = bytes.get(end).copied();
        if left == Some(b'.') || right == Some(b'.') {
            continue;
        }
        let Some(raw) = name.get(start..end) else {
            continue;
        };
        let token = raw.to_ascii_lowercase();
        if REGION_ASCII_TOKENS.contains(&token.as_str()) {
            return true;
        }
        if token == "cn" && (raw == "CN" || right == Some(b'2')) {
            return true;
        }
        if token == "us" && (raw == "US" || is_code_context_byte(left) || is_code_context_byte(right)) {
            return true;
        }
    }
    false
}

/// 在带冒号/中文语境的横幅中识别内嵌裸域名（如 `永久官网:666.boostqz.com`）。
/// 纯 ASCII 文本且不含冒号时直接放行，避免误杀以原始主机域名命名的真实节点（如 `vps1.myserver.com`）。
fn has_embedded_banner_domain(name: &str, lower: &str) -> bool {
    if !(name.contains(':') || name.contains('：') || !name.is_ascii()) {
        return false;
    }
    let bytes = lower.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b != b'.' || idx < 2 || idx + 2 >= bytes.len() {
            continue;
        }
        // `.` 左侧必须为至少 2 个 `[a-z0-9-]` 且紧邻 `.` 的字符不是 `-`
        let prev = bytes[idx - 1];
        if !(prev.is_ascii_lowercase() || prev.is_ascii_digit()) {
            continue;
        }
        let mut l = idx;
        while l > 0 {
            let c = bytes[l - 1];
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' {
                l -= 1;
            } else {
                break;
            }
        }
        if idx - l < 2 {
            continue;
        }
        // `.` 右侧必须为纯 `[a-z]+` 常见 TLD，且后面不紧跟 `[a-z0-9.]`
        let mut r = idx + 1;
        while r < bytes.len() && bytes[r].is_ascii_lowercase() {
            r += 1;
        }
        if r == idx + 1 {
            continue;
        }
        if bytes
            .get(r)
            .is_some_and(|&next| next.is_ascii_lowercase() || next.is_ascii_digit() || next == b'.')
        {
            continue;
        }
        if let Some(tld) = lower.get(idx + 1..r) {
            if BANNER_DOMAIN_TLDS.contains(&tld) {
                return true;
            }
        }
    }
    false
}

/// 识别首个冒号右侧带有日期（`20xx-xx` / `20xx年xx`）、流量配额（`GB/TB/MB/PB`）或倒计时（`天/小时/days`）的键值横幅。
fn has_colon_quota_or_date(lower: &str) -> bool {
    let Some((pos, ch)) = lower.match_indices([':', '：']).next() else {
        return false;
    };
    let Some(rhs) = lower.get(pos + ch.len()..) else {
        return false;
    };
    let bytes = rhs.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let num_start = i;
        // 避免匹配长数字中间片段
        if num_start > 0 && (bytes[num_start - 1].is_ascii_digit() || bytes[num_start - 1] == b'.') {
            i += 1;
            continue;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let int_end = i;

        // 1. 年月日期形态：`20\d{2}[-/.年]\d`
        if int_end - num_start == 4 && rhs.get(num_start..num_start + 2) == Some("20") {
            if let Some(tail) = rhs.get(int_end..) {
                let mut tail_chars = tail.chars();
                if matches!(tail_chars.next(), Some('-' | '/' | '.' | '年'))
                    && tail_chars.next().is_some_and(|c| c.is_ascii_digit())
                {
                    return true;
                }
            }
        }

        // 2. 配额 / 倒计时形态：支持小数（如 `167.26 GB`）与可选空白
        let mut after_num = int_end;
        if bytes.get(after_num) == Some(&b'.') && bytes.get(after_num + 1).is_some_and(u8::is_ascii_digit) {
            after_num += 1;
            while after_num < bytes.len() && bytes[after_num].is_ascii_digit() {
                after_num += 1;
            }
        }
        let mut k = after_num;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if let Some(after_ws) = rhs.get(k..) {
            for unit in ["gib", "tib", "gb", "tb", "mb", "pb"] {
                if after_ws.starts_with(unit) && bytes.get(k + unit.len()).is_none_or(|b| !b.is_ascii_lowercase()) {
                    return true;
                }
            }
            if after_ws.starts_with("天") || after_ws.starts_with("小时") {
                return true;
            }
            for unit in ["days", "day"] {
                if after_ws.starts_with(unit) && bytes.get(k + unit.len()).is_none_or(|b| !b.is_ascii_lowercase()) {
                    return true;
                }
            }
        }
    }
    false
}

/// 非穷举结构形态识别：不含任何地区暗示时，按内嵌横幅域名与冒号配额/日期格式捕获未知广告文案。
fn is_structural_banner_name(name: &str, lower: &str) -> bool {
    if has_region_hint(name) {
        return false;
    }
    has_embedded_banner_domain(name, lower) || has_colon_quota_or_date(lower)
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
static RUNTIME_NODE_FINGERPRINTS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);
static RUNTIME_TAINTED_BANNER_FPS: RwLock<Option<HashSet<String>>> = RwLock::new(None);

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

fn is_tainted_banner_fp(fp: &str) -> bool {
    ensure_runtime_node_fingerprints_loaded();
    RUNTIME_TAINTED_BANNER_FPS
        .read()
        .as_ref()
        .is_some_and(|set| set.contains(fp))
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

fn fingerprint_for_node(name: &str) -> Option<String> {
    ensure_runtime_node_fingerprints_loaded();
    RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .and_then(|map| map.get(name).cloned())
}

/// 查询节点名是否已被确认假节点（处于软隔离期内；无指纹时返回 `false`）
pub fn is_confirmed_fake_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_confirmed_fake_fp(&fp))
}

/// 查询节点名是否已通过真 HTTPS 验真
pub fn is_verified_true_node(name: &str) -> bool {
    fingerprint_for_node(name).is_some_and(|fp| is_verified_true_fp(&fp))
}

/// 查询节点名是否在有效观察期（未过 24h TTL）内曾在有参照节点通过的前提下累计过真 HTTPS 验真失败
pub fn has_verify_failure_node(name: &str) -> bool {
    let Some(fp) = fingerprint_for_node(name) else {
        return false;
    };
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    VERIFY_STORE.read().as_ref().is_some_and(|store| {
        store.records.get(&fp).is_some_and(|r| {
            !r.verified_true
                && r.fail_count > 0
                && (r.last_verified_at <= 0 || now_ts.saturating_sub(r.last_verified_at) < HTTPS_VERIFY_BAN_TTL_SECS)
        })
    })
}

/// 返回当前运行配置中所有仍在软隔离期内或被拓扑连坐认定的假节点名称列表（供前端隐藏显示）
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
    let now_ts = current_unix_ts();
    let tainted_fps = RUNTIME_TAINTED_BANNER_FPS.read().clone().unwrap_or_default();
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
    let fps: Vec<String> = RUNTIME_NODE_FINGERPRINTS
        .read()
        .as_ref()
        .map(|map| candidates.iter().filter_map(|name| map.get(name).cloned()).collect())
        .unwrap_or_default();
    if fps.is_empty() {
        return false;
    }
    ensure_verify_store_loaded();
    let now_ts = current_unix_ts();
    let tainted_fps = RUNTIME_TAINTED_BANNER_FPS.read().clone().unwrap_or_default();
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

fn clear_unverified_failures_in_records(records: &mut HashMap<String, NodeVerifyRecord>) -> usize {
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
        persist_verify_store_snapshot(&store_snapshot);
    }

    outcome
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        HTTPS_VERIFY_BAN_TTL_SECS, HTTPS_VERIFY_MAX_FAILS, NodeVerifyRecord, NodeVerifyStoreData, RawProxyEntry,
        apply_verify_batch_to_records, clear_unverified_failures_in_records, deduplicate_cloned_proxies,
        has_region_hint, is_dummy_node, is_valid_node_server, load_verify_store_from_path, match_node_filter,
        proxy_endpoint_fingerprint, resolve_proxy_clone_topology,
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
        assert!(!records[&dead].is_confirmed_fake_at(101));

        // good 随后即使作为参照失败也不会降级，fail_count 保持 0
        for round in 2..=HTTPS_VERIFY_MAX_FAILS {
            let ts = 100 + i64::from(round);
            let o =
                apply_verify_batch_to_records(&mut records, &["fp-ref".to_owned()], &[good.clone(), dead.clone()], ts);
            assert_eq!(o.newly_confirmed_fake, 0);
            assert!(records[&good].verified_true);
            assert_eq!(records[&good].fail_count, 0);
            assert_eq!(records[&dead].fail_count, round);
            assert!(!records[&dead].is_confirmed_fake_at(ts));
        }

        // 第 6 次失败（超过 HTTPS_VERIFY_MAX_FAILS = 5）-> 进入 24h 软隔离假节点
        let o6 = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&good),
            std::slice::from_ref(&dead),
            200,
        );
        assert_eq!(o6.newly_confirmed_fake, 1);
        assert!(records[&dead].is_confirmed_fake_at(200));

        // 24h 软隔离期内再次失败不重复计数
        let o7 = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&good),
            std::slice::from_ref(&dead),
            201,
        );
        assert_eq!(o7.newly_confirmed_fake, 0);
    }

    #[test]
    fn test_verify_state_machine_quarantine_ttl_and_manual_clear() {
        let mut records: HashMap<String, NodeVerifyRecord> = HashMap::new();
        let ref_fp = "fp-ref".to_owned();
        let node_a = "fp-temp-down".to_owned();
        let node_b = "fp-recovered".to_owned();

        records.insert(
            ref_fp.clone(),
            NodeVerifyRecord {
                verified_true: true,
                fail_count: 0,
                last_verified_at: 200,
            },
        );
        records.insert(
            node_a.clone(),
            NodeVerifyRecord {
                verified_true: false,
                fail_count: HTTPS_VERIFY_MAX_FAILS + 1,
                last_verified_at: 200,
            },
        );
        records.insert(
            node_b.clone(),
            NodeVerifyRecord {
                verified_true: false,
                fail_count: HTTPS_VERIFY_MAX_FAILS + 1,
                last_verified_at: 200,
            },
        );

        // 1. 满 24h 前一秒仍处于软隔离；满 24h 自动解除
        assert!(records[&node_a].is_confirmed_fake_at(200 + HTTPS_VERIFY_BAN_TTL_SECS - 1));
        assert!(!records[&node_a].is_confirmed_fake_at(200 + HTTPS_VERIFY_BAN_TTL_SECS));

        // 2. 期满后重新参加一轮验真：node_b 通过升级粘滞真值，node_a 再次失败则清零重计为 fail_count = 1
        let ttl_ts = 200 + HTTPS_VERIFY_BAN_TTL_SECS;
        let o_recheck = apply_verify_batch_to_records(
            &mut records,
            std::slice::from_ref(&node_b),
            std::slice::from_ref(&node_a),
            ttl_ts,
        );
        assert_eq!(o_recheck.newly_verified, 1);
        assert_eq!(o_recheck.newly_confirmed_fake, 0);
        assert!(records[&node_b].verified_true);
        assert_eq!(records[&node_b].fail_count, 0);
        assert_eq!(records[&node_a].fail_count, 1);
        assert!(!records[&node_a].is_confirmed_fake_at(ttl_ts));

        // 3. 手动更新订阅清零未验真失败记录，但完整保留 verified_true 白名单
        let cleared = clear_unverified_failures_in_records(&mut records);
        assert_eq!(cleared, 1);
        assert!(!records.contains_key(&node_a));
        assert!(records[&ref_fp].verified_true);
        assert!(records[&node_b].verified_true);
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

    #[test]
    fn test_has_region_hint_and_structural_banner_detection() {
        // 1. 真实订阅实战漏网文案：含冒号倒计时/配额/日期或内嵌横幅域名（无地区暗示）
        assert!(is_dummy_node("距离下次重置剩余：24 天 (045017)"));
        assert!(is_dummy_node("永久官网:666.boostqz.com (045017)"));
        assert!(is_dummy_node("有效期至：2027-02-09"));
        assert!(is_dummy_node("更新日期：2026年11月"));
        assert!(is_dummy_node("Remaining: 30 days"));
        assert!(is_dummy_node("Quota: 167.26 GB"));
        assert!(is_dummy_node("Contact us: boostqz.com"));
        // .hk 后缀相邻点号不算地区暗示，应识别为官网域名横幅
        assert!(!has_region_hint("永久官网:abc.hk"));
        assert!(is_dummy_node("永久官网:abc.hk"));

        // 2. 地区暗示校验：中英文、国旗、边界 IATA/国家码与 CN2/US 消歧
        assert!(has_region_hint("🇭🇰 HK-01"));
        assert!(has_region_hint("香港1 (045017)"));
        assert!(has_region_hint("us-01 [VLESS]"));
        assert!(has_region_hint("CN2-GIA-01"));
        assert!(!has_region_hint("About us: boostqz.com"));

        // 3. 防误杀真实节点名（不含横幅结构或带真实地区暗示）
        assert!(!is_dummy_node("vps1.myserver.com"));
        assert!(!is_dummy_node("新加坡：备用"));
        assert!(!is_dummy_node("专线：10Gbps"));
        assert!(!is_dummy_node("【剩余流量】80GB"));
        assert!(!is_dummy_node("最新官网发布页"));
        assert!(!is_dummy_node("香港1 (045017)"));
    }

    #[test]
    fn test_resolve_proxy_clone_topology_taint_and_parasite_cases() {
        // 场景 A：机场把多条横幅绑在同一真实可用香港端点上，且混入全新未知广告文案（不含任何地区真节点）
        // 预期：整组指纹连坐 0 保留全灭，绝不留任何一个横幅进入 PROXY
        let entries_banner_only = vec![
            RawProxyEntry {
                name: "剩余流量：167.26 GB (045017)".to_owned(),
                fingerprint: Some("fp-banner".to_owned()),
                is_dummy: is_dummy_node("剩余流量：167.26 GB (045017)"),
            },
            RawProxyEntry {
                name: "距离下次重置剩余：24 天 (045017)".to_owned(),
                fingerprint: Some("fp-banner".to_owned()),
                is_dummy: is_dummy_node("距离下次重置剩余：24 天 (045017)"),
            },
            RawProxyEntry {
                name: "全网独创全新福利宣传语".to_owned(), // 完全未知文案，is_dummy = false
                fingerprint: Some("fp-banner".to_owned()),
                is_dummy: is_dummy_node("全网独创全新福利宣传语"),
            },
            RawProxyEntry {
                name: "香港1 (045017)".to_owned(),
                fingerprint: Some("fp-hk1".to_owned()),
                is_dummy: is_dummy_node("香港1 (045017)"),
            },
        ];
        let out_a = resolve_proxy_clone_topology(entries_banner_only);
        assert_eq!(out_a.names, vec!["香港1 (045017)"]);
        assert_eq!(out_a.tainted_banner_removed, 1);
        assert!(out_a.tainted_fps.contains("fp-banner"));

        // 场景 B：机场把未知文案横幅直接挂在真实「香港1」节点同一指纹上，且排在「香港1」前面
        // 预期：有真实地区节点同行不整组全灭，但优选等级强制折叠归真为「香港1 (045017)」，令 -香港 可靠排除
        let entries_parasite = vec![
            RawProxyEntry {
                name: "距离下次重置剩余：24 天 (045017)".to_owned(),
                fingerprint: Some("fp-hk1".to_owned()),
                is_dummy: true,
            },
            RawProxyEntry {
                name: "全网独创客服宣传语".to_owned(),
                fingerprint: Some("fp-hk1".to_owned()),
                is_dummy: false,
            },
            RawProxyEntry {
                name: "香港1 (045017)".to_owned(),
                fingerprint: Some("fp-hk1".to_owned()),
                is_dummy: false,
            },
        ];
        let out_b = resolve_proxy_clone_topology(entries_parasite);
        assert_eq!(out_b.names, vec!["香港1 (045017)"]);
        assert_eq!(out_b.clone_removed, 1);
        assert!(out_b.tainted_fps.is_empty());

        // 场景 C：真实双带冒号地区节点共享同一指纹（如「新加坡：备用」「新加坡：主」）不被误伤
        let entries_dual_colon_region = vec![
            RawProxyEntry {
                name: "新加坡：备用".to_owned(),
                fingerprint: Some("fp-sg".to_owned()),
                is_dummy: false,
            },
            RawProxyEntry {
                name: "新加坡：主".to_owned(),
                fingerprint: Some("fp-sg".to_owned()),
                is_dummy: false,
            },
        ];
        let out_c = resolve_proxy_clone_topology(entries_dual_colon_region);
        assert_eq!(out_c.names, vec!["新加坡：备用"]);
        assert!(out_c.tainted_fps.is_empty());
    }
}
