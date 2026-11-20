/// 识别广告/假节点（与前端 `src/utils/node.ts::isDummyNode` 及通用解析链路同源对齐）：
/// URL/英文广告词匹配子串；中文广告短语先剥除包裹符号与通用前缀后再做前缀匹配，防止误杀正常节点。
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
        if let Some(after_ws) = rhs.get(after_num..).map(str::trim_start) {
            let tail_bytes = after_ws.as_bytes();
            for unit in ["gib", "tib", "gb", "tb", "mb", "pb"] {
                if after_ws.starts_with(unit) && tail_bytes.get(unit.len()).is_none_or(|b| !b.is_ascii_lowercase()) {
                    return true;
                }
            }
            if after_ws.starts_with("天") || after_ws.starts_with("小时") {
                return true;
            }
            for unit in ["days", "day"] {
                if after_ws.starts_with(unit) && tail_bytes.get(unit.len()).is_none_or(|b| !b.is_ascii_lowercase()) {
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

fn strip_exclusion_prefix(token: &str) -> &str {
    token
        .strip_prefix('-')
        .or_else(|| token.strip_prefix('!'))
        .or_else(|| token.strip_prefix('！'))
        .unwrap_or(token)
        .trim()
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
                let term = strip_exclusion_prefix(part);
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

/// 判断过滤文本是否包含至少一个非空的实际匹配词（与前端 `src/utils/node.ts::hasEffectiveFilterTerm` 1:1 对齐）。
pub fn has_effective_filter_term(filter_text: &str) -> bool {
    let normalized = filter_text.trim().replace('｜', "|");
    if normalized.is_empty() {
        return false;
    }
    for token in normalized.split_whitespace() {
        for part in token.split('|') {
            if !strip_exclusion_prefix(part.trim()).is_empty() {
                return true;
            }
        }
    }
    false
}

/// 判断过滤文本是否处于不完整的操作符半输入态（与前端 `src/utils/node.ts::isIncompleteFilterInput` 1:1 对齐）。
pub fn is_incomplete_filter_input(filter_text: &str) -> bool {
    let trimmed = filter_text.trim();
    if !has_effective_filter_term(trimmed) {
        return true;
    }
    if trimmed.ends_with('|') || trimmed.ends_with('｜') {
        return true;
    }
    let normalized = trimmed.replace('｜', "|");
    let last_token = normalized.split_whitespace().next_back().unwrap_or("");
    let last_part = last_token.split('|').next_back().unwrap_or("").trim();
    matches!(last_part, "-" | "!" | "！")
}

/// 对齐有效过滤词：空或半输入态一律退化为空串（视为不设过滤条件）。
pub fn effective_filter_text(filter_text: &str) -> &str {
    if is_incomplete_filter_input(filter_text) {
        ""
    } else {
        filter_text.trim()
    }
}

/// 按优先级从 `PROXY` 全量成员中挑选兜底节点：
/// 1. 匹配 `filter_text` 且已通过真 HTTPS 验真的有效节点；
/// 2. 匹配 `filter_text` 的有效节点；
/// 3. 仅当 `filter_text.trim().is_empty()`（未设有效筛选）时才回退至首个全局有效节点；
///    存在显式关键词筛选且子集全灭时返回 `None`，严禁静默切出筛选范围。
pub fn pick_subset_fallback_node<'a, FValid, FVerified>(
    all: &'a [String],
    filter_text: &str,
    is_valid: FValid,
    is_verified: FVerified,
) -> Option<&'a str>
where
    FValid: Fn(&str) -> bool,
    FVerified: Fn(&str) -> bool,
{
    let trimmed = filter_text.trim();
    all.iter()
        .find(|n| is_valid(n) && is_verified(n) && match_node_filter(n, trimmed))
        .or_else(|| all.iter().find(|n| is_valid(n) && match_node_filter(n, trimmed)))
        .or_else(|| {
            if trimmed.is_empty() {
                all.iter().find(|n| is_valid(n))
            } else {
                None
            }
        })
        .map(String::as_str)
}

/// 将符合用户过滤范围的持久选中节点（或合规子集首节点）提前至 `PROXY.proxies[0]`，
/// 使 `store-selected: false` 下 Mihomo 启动/重载第 0 毫秒即落位在合规节点而非未过滤的订阅首节点。
pub fn promote_preferred_proxy_first<FValid, FVerified>(
    names: &mut Vec<String>,
    preferred: Option<&str>,
    filter_text: &str,
    is_valid: FValid,
    is_verified: FVerified,
) -> Option<String>
where
    FValid: Fn(&str) -> bool,
    FVerified: Fn(&str) -> bool,
{
    let effective = effective_filter_text(filter_text);
    let target = preferred
        .map(str::trim)
        .filter(|n| {
            !n.is_empty()
                && is_valid(n)
                && match_node_filter(n, effective)
                && names.iter().any(|existing| existing == n)
        })
        .map(str::to_owned)
        .or_else(|| pick_subset_fallback_node(names, effective, &is_valid, &is_verified).map(str::to_owned))?;

    if let Some(pos) = names.iter().position(|n| n == &target) {
        if pos > 0 {
            let item = names.remove(pos);
            names.insert(0, item);
        }
        return Some(target);
    }
    None
}
