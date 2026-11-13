mod chain;
pub mod field;
mod merge;
mod script;
pub mod seq;
mod tun;

use self::{
    chain::{AsyncChainItemFrom as _, ChainItem, ChainType},
    field::{use_keys, use_lowercase, use_sort},
    merge::use_merge,
    script::use_script,
    seq::{SeqMap, use_seq},
    tun::use_tun_with_sniffer,
};
use crate::config::IVerge;
use crate::utils::dirs;
use crate::{config::Config, utils::tmpl};
use anyhow::{Context as _, Result};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

type ResultLog = Vec<(String, String)>;
#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    clash_core: Option<String>,
    enable_tun: bool,
    enable_tun_sniffer: Option<bool>,
    enable_builtin: bool,
    socks_enabled: bool,
    http_enabled: bool,
}

#[derive(Debug)]
struct ProfileItems {
    config: Mapping,
    merge_item: ChainItem,
    script_item: ChainItem,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: String,
}

impl Default for ProfileItems {
    fn default() -> Self {
        Self {
            config: Default::default(),
            profile_name: Default::default(),
            merge_item: ChainItem {
                uid: "".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            script_item: ChainItem {
                uid: "".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
            rules_item: ChainItem {
                uid: "".into(),
                data: ChainType::Rules(SeqMap::default()),
            },
            proxies_item: ChainItem {
                uid: "".into(),
                data: ChainType::Proxies(SeqMap::default()),
            },
            global_merge: ChainItem {
                uid: "Merge".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            global_script: ChainItem {
                uid: "Script".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
        }
    }
}

async fn get_config_values() -> ConfigValues {
    let clash = Config::clash().await;
    let clash_arc = clash.latest_arc();
    let clash_config = clash_arc.0.clone();
    drop(clash_arc);
    drop(clash);

    let verge = Config::verge().await;

    let verge_arc = verge.latest_arc();
    let IVerge {
        ref enable_tun_mode,
        ref enable_tun_sniffer,
        ref enable_builtin_enhanced,
        ref verge_socks_enabled,
        ref verge_http_enabled,
        ..
    } = *verge_arc;

    let (clash_core, enable_tun, enable_tun_sniffer, enable_builtin, socks_enabled, http_enabled) = (
        Some(verge_arc.get_valid_clash_core()),
        enable_tun_mode.unwrap_or(false),
        *enable_tun_sniffer,
        enable_builtin_enhanced.unwrap_or(true),
        verge_socks_enabled.unwrap_or(false),
        verge_http_enabled.unwrap_or(false),
    );

    drop(verge_arc);
    drop(verge);

    ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_tun_sniffer,
        enable_builtin,
        socks_enabled,
        http_enabled,
    }
}

#[allow(clippy::cognitive_complexity)]
async fn collect_profile_items() -> Result<ProfileItems> {
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    drop(profiles);

    let current_profile_uid = match profiles_arc.get_current().cloned() {
        Some(uid) => uid,
        None => {
            drop(profiles_arc);
            return Ok(ProfileItems::default());
        }
    };

    let current = profiles_arc
        .current_mapping()
        .with_context(|| format!("failed to read current profile \"{current_profile_uid}\""))?;

    let current_item = match profiles_arc.get_item(&current_profile_uid) {
        Ok(item) => item,
        Err(err) => {
            return Err(err).with_context(|| format!("failed to get current profile \"{current_profile_uid}\""));
        }
    };

    let merge_uid = current_item.current_merge().cloned().unwrap_or_else(|| "Merge".into());
    let script_uid = current_item
        .current_script()
        .cloned()
        .unwrap_or_else(|| "Script".into());
    let rules_uid = current_item.current_rules().cloned().unwrap_or_else(|| "Rules".into());
    let proxies_uid = current_item
        .current_proxies()
        .cloned()
        .unwrap_or_else(|| "Proxies".into());
    let name = current_item.name.clone().unwrap_or_default();

    let merge_item = {
        let item = profiles_arc.get_item(&merge_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let script_item = {
        let item = profiles_arc.get_item(&script_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    let rules_item = {
        let item = profiles_arc.get_item(&rules_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Rules(SeqMap::default()),
    });

    let proxies_item = {
        let item = profiles_arc.get_item(&proxies_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Proxies(SeqMap::default()),
    });

    let global_merge = {
        let item = profiles_arc.get_item("Merge").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Merge".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let global_script = {
        let item = profiles_arc.get_item("Script").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Script".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    drop(profiles_arc);

    Ok(ProfileItems {
        config: current,
        merge_item,
        script_item,
        rules_item,
        proxies_item,
        global_merge,
        global_script,
        profile_name: name,
    })
}

async fn process_global_items(
    mut config: Mapping,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    let mut result_map = HashMap::new();
    let mut exists_keys = use_keys(&config).collect::<Vec<_>>();

    if let ChainType::Merge(merge) = global_merge.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = global_script.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(global_script.uid, logs);
    }

    (config, exists_keys, result_map)
}

#[allow(clippy::too_many_arguments)]
async fn process_profile_items(
    mut config: Mapping,
    mut exists_keys: Vec<String>,
    mut result_map: HashMap<String, ResultLog>,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    merge_item: ChainItem,
    script_item: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    if let ChainType::Rules(rules) = rules_item.data {
        config = use_seq(rules, config.to_owned(), "rules");
    }

    if let ChainType::Proxies(proxies) = proxies_item.data {
        config = use_seq(proxies, config.to_owned(), "proxies");
    }

    if let ChainType::Merge(merge) = merge_item.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = script_item.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(script_item.uid, logs);
    }

    (config, exists_keys, result_map)
}

async fn merge_default_config(
    mut config: Mapping,
    clash_config: Mapping,
    socks_enabled: bool,
    http_enabled: bool,
) -> Mapping {
    for (key, value) in clash_config.into_iter() {
        if key.as_str() == Some("tun") {
            let mut tun = config.get_mut("tun").map_or_else(Mapping::new, |val| {
                val.as_mapping().cloned().unwrap_or_else(Mapping::new)
            });
            let patch_tun = value.as_mapping().cloned().unwrap_or_else(Mapping::new);
            for (key, value) in patch_tun.into_iter() {
                tun.insert(key, value);
            }
            // 注：旧默认值 stack=system → gvisor 的归一化统一在 IClashTemp::new() 完成
            //（clash.yaml 加载时改写），此处不再重复判断，避免同一语义两处维护。
            config.insert("tun".into(), tun.into());
        } else if matches!(key.as_str(), Some("dns") | Some("profile")) {
            // dns / profile 是"订阅优先"的复合节点：仅补缺，不覆盖订阅已声明的字段。
            // 与 tun 不同，二者归订阅所有（Mini 无对应覆写入口），模板只提供兜底默认值。
            let section_name = key.as_str().unwrap_or_default().to_string();
            let mut section = config.get_mut(section_name.as_str()).map_or_else(Mapping::new, |val| {
                val.as_mapping().cloned().unwrap_or_else(Mapping::new)
            });
            let defaults = value.as_mapping().cloned().unwrap_or_else(Mapping::new);
            for (default_key, default_value) in defaults.into_iter() {
                let absent = match default_key.as_str() {
                    Some(name) => !section.contains_key(Value::from(name)),
                    None => true,
                };
                if absent {
                    section.insert(default_key, default_value);
                }
            }
            config.insert(key, section.into());
        } else if key.as_str() == Some("tcp-concurrent") {
            // tcp-concurrent 是 Mini 为省资源自加的标量默认值（上游模板无此键），
            // 但订阅可能显式声明。与 dns/profile 同理：订阅优先，仅当订阅未声明时补
            // false，避免强制覆盖订阅的并发连接语义。
            if !config.contains_key(Value::from("tcp-concurrent")) {
                config.insert(key, value);
            }
        } else {
            if key.as_str() == Some("socks-port") && !socks_enabled {
                config.remove("socks-port");
                continue;
            }
            if key.as_str() == Some("port") && !http_enabled {
                config.remove("port");
                continue;
            }
            // Windows 下的 redir-port / tproxy-port 为其他平台专属键，直接跳过不写入
            if matches!(key.as_str(), Some("redir-port") | Some("tproxy-port")) {
                continue;
            }
            // config_version 是 Clash Mini 自己的**配置格式版本号**（随 clash.yaml 落盘），
            // 不是 mihomo 配置项，绝不能混进运行时配置。实测内核会忽略未知顶层键（-t 校验通过），
            // 但运行时配置理应只含内核认识的键，避免将来内核改为严格解析时校验失败。
            if key.as_str() == Some("config_version") {
                continue;
            }
            // 处理 external-controller 键的开关逻辑
            if key.as_str() == Some("external-controller") {
                let enable_external_controller = Config::verge()
                    .await
                    .latest_arc()
                    .enable_external_controller
                    .unwrap_or(false);

                if enable_external_controller {
                    config.insert(key, value);
                } else {
                    // 如果禁用了外部控制器，设置为空字符串
                    config.insert(key, "".into());
                }
            } else {
                config.insert(key, value);
            }
        }
    }

    config
}

async fn apply_builtin_scripts(mut config: Mapping, clash_core: Option<String>, enable_builtin: bool) -> Mapping {
    if enable_builtin {
        let items: Vec<_> = ChainItem::builtin()
            .into_iter()
            .filter(|(s, _)| s.is_support(clash_core.as_ref()))
            .map(|(_, c)| c)
            .collect();
        for item in items {
            logging!(debug, Type::Core, "run builtin script {}", item.uid);
            if let ChainType::Script(script) = item.data {
                match use_script(script, config.clone(), String::from("")).await {
                    Ok((res_config, _)) => {
                        config = res_config;
                    }
                    Err(err) => {
                        logging!(error, Type::Core, "builtin script error `{err}`");
                    }
                }
            }
        }
    }

    config
}

fn apply_mandatory_dns_settings(mut config: Mapping) -> Mapping {
    use serde_yaml_ng::Value;

    // 【修复】改为「仅补全缺失字段」，绝不整体替换 dns 节点。
    // 原实现 config.insert("dns", ...) 只保留 enable / enhanced-mode=redir-host /
    // cache-* / nameserver=[8.8.8.8,114.114.114.114]，会抹掉订阅
    // 提供的 default-nameserver、proxy-server-nameserver、fallback、fallback-filter、
    // nameserver-policy 等字段，并在流水线末端强制 redir-host 覆盖 use_tun 写入的
    // fake-ip。叠加 8.8.8.8 在中国大陆直连不可达，导致 TUN 下 redir-host 需要把域名
    // 实时解析成真实 IP 时出现间歇性解析失败（切换代理模式触发 reload 清 DNS 缓存后恢复）。
    let mut dns_config = match config.remove("dns") {
        Some(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };

    // 逐项补全：已存在的一律保留，不覆盖
    if !dns_config.contains_key(Value::from("enable")) {
        dns_config.insert("enable".into(), Value::Bool(true));
    }
    // 注：此处**不**补 dns.ipv6 —— 该字段由 `enhance::tun::use_tun` 作为唯一真源同步
    // （TUN 与非 TUN 两分支均无条件写入，值恒为用户「Allow IPv6」开关）。
    // 若在这里读顶层 ipv6 兜底，TUN 模式下读到的是 use_tun 刚强制过的 `true`（IPv6 防泄漏
    // 接管所需），会把用户关掉的 IPv6 静默写回开启 —— 这是靠「另一分支必然已写入」侥幸不出错的
    // 脆弱耦合，故直接移除。
    // 注：不设置 enhanced-mode / fake-ip-range —— 由 use_tun 在 TUN 场景写入 fake-ip，
    // 非 TUN 场景交由内核默认值（redir-host），避免覆盖用户显式配置。
    if !dns_config.contains_key(Value::from("nameserver")) {
        dns_config.insert(
            "nameserver".into(),
            Value::Sequence(vec![
                Value::String("https://doh.pub/dns-query".into()),
                Value::String("https://dns.alidns.com/dns-query".into()),
                Value::String("114.114.114.114".into()),
            ]),
        );
    }
    if !dns_config.contains_key(Value::from("default-nameserver")) {
        dns_config.insert(
            "default-nameserver".into(),
            Value::Sequence(vec![
                Value::String("223.6.6.6".into()),
                Value::String("223.5.5.5".into()),
                Value::String("114.114.114.114".into()),
            ]),
        );
    }
    if !dns_config.contains_key(Value::from("proxy-server-nameserver")) {
        dns_config.insert(
            "proxy-server-nameserver".into(),
            Value::Sequence(vec![
                Value::String("https://doh.pub/dns-query".into()),
                Value::String("https://dns.alidns.com/dns-query".into()),
                Value::String("tls://223.5.5.5".into()),
            ]),
        );
    }

    // 【冷启动保底】default-nameserver / proxy-server-nameserver 确保含明文 UDP 上游
    // 223.5.5.5（仅追加，不覆盖已有条目）。订阅的加密 DNS（DoH/DoT）服务器域名需先经
    // default-nameserver 解析，冷启动窗口内（TUN 接口绑定延迟、系统 DNS 被 dns-hijack
    // any:53 劫持回环、8.8.8.8 大陆直连不可达）该链可能全断 → 节点域名解析不出 →
    // 首轮测速全 timeout。明文 IP 上游无需任何前置解析，接口绑定后即可用，消除鸡生蛋。
    // 注：default-nameserver 补缺默认值已含明文上游，此处统一兜住"订阅已提供但缺明文"的情况。
    for key in ["default-nameserver", "proxy-server-nameserver"] {
        let fallback = Value::String("223.5.5.5".into());
        if let Some(val) = dns_config.get_mut(key) {
            match val {
                Value::Sequence(seq) if !seq.contains(&fallback) => {
                    seq.push(fallback);
                    logging!(
                        info,
                        Type::Core,
                        "DNS {key} 追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                    );
                }
                Value::String(s) if s != "223.5.5.5" => {
                    *val = Value::Sequence(vec![Value::String(s.clone()), fallback]);
                    logging!(
                        info,
                        Type::Core,
                        "DNS {key} 标量提升为列表并追加保底明文上游 223.5.5.5（冷启动防鸡生蛋）"
                    );
                }
                _ => {}
            }
        }
    }

    config.insert("dns".into(), Value::Mapping(dns_config));
    logging!(
        info,
        Type::Core,
        "补全缺失的 DNS 字段（非覆盖式，保留订阅/用户已有设置）"
    );
    config
}

/// 【订阅 DNS 卫生改写】境外公共 DNS 的明文 UDP 上游改写为 DoH 形态。
///
/// 机场订阅常在 `dns.nameserver` 里自带明文 UDP 上游（如 `8.8.8.8`）。在
/// `respect-rules: true` 下，内核自身发起的这类 DNS 查询会命中
/// `IP-CIDR,8.8.8.8/32,PROXY` 被塞进代理节点；当节点为 anytls 等 TCP 承载协议时
/// （UDP-over-TCP），每个 UDP 查询都要先向节点新建一条 TCP 会话——该会话建立
/// 慢于境内 DoH 上游，在内核 DNS 并发赛跑中必被取消，刷出大量
/// `failed to create session: dial tcp <node>: operation was canceled` 警告
/// （3.1.7 现场实证：7 条全部落在 anytls 被选中窗口内，切 ss 节点后零新增）。
///
/// 改写为 DoH（TCP 承载，走代理稳定）后该竞速取消彻底消失。范围界定：
/// - 仅处理 `nameserver`：`default-nameserver` 按协议定义必须为明文 IP
///   （用于解析 DoH 域名本身），不可改写；境内明文上游（223.5.5.5 /
///   114.114.114.114 等）直连可达且稳定赢得赛跑，无需处理。
/// - 已带 scheme（`tls://`、`https://`、`quic://` 等）的加密上游原样保留。
/// - 仅覆盖四家境外公共 DNS 的已知 IP（Google 8.8.8.8/8.8.4.4、
///   Cloudflare 1.1.1.1/1.0.0.1），与 `dns_fallback_proxy_rules` 的对偶口径一致。
struct ParsedDnsUpstream {
    host: std::string::String,
    is_plain: bool,
    force_direct: bool,
    is_special: bool,
    clean_upstream: std::string::String,
}

fn parse_dns_upstream(raw: &str) -> ParsedDnsUpstream {
    let trimmed = raw.trim();
    let (without_frag, frag_opt) = match trimmed.split_once('#') {
        Some((before, frag)) => (before.trim(), Some(frag.trim())),
        None => (trimmed, None),
    };
    let mut force_direct = false;
    let mut kept_frags: Vec<&str> = Vec::new();
    if let Some(frag) = frag_opt {
        for part in frag.split('&').map(str::trim).filter(|p| !p.is_empty()) {
            // C12：mihomo 原生并不识别 `#DIRECT` / `#PROXY` 政策后缀；
            // `#DIRECT` 在直连不可达语义下供清理器降级使用，且与 `#PROXY` 一并剥离，
            // 仅保留 `#h3=true` 等合法 mihomo 参数。
            if part.eq_ignore_ascii_case("direct") {
                force_direct = true;
            } else if !part.eq_ignore_ascii_case("proxy") {
                kept_frags.push(part);
            }
        }
    }
    let clean_upstream = if kept_frags.is_empty() {
        without_frag.to_string()
    } else {
        format!("{without_frag}#{}", kept_frags.join("&"))
    };

    let (scheme, rest) = match without_frag.split_once("://") {
        Some((s, r)) => (s.to_ascii_lowercase(), r),
        None => (std::string::String::new(), without_frag),
    };
    let is_special = scheme == "dhcp" || scheme == "rcode" || rest.eq_ignore_ascii_case("system");
    let is_plain = scheme.is_empty() || scheme == "udp" || scheme == "tcp";
    let host = extract_upstream_host(without_frag)
        .unwrap_or_default()
        .to_ascii_lowercase();

    ParsedDnsUpstream {
        host,
        is_plain,
        force_direct,
        is_special,
        clean_upstream,
    }
}

const fn is_shared_cgnat_v4(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0b1100_0000) == 64
}

const fn is_ula_or_link_local_v6(ip: std::net::Ipv6Addr) -> bool {
    let seg0 = ip.segments()[0];
    (seg0 & 0xfe00) == 0xfc00 || (seg0 & 0xffc0) == 0xfe80
}

fn is_local_or_domestic_dns_host(host: &str) -> bool {
    const CN_DNS_IPV4: &[&str] = &[
        "223.5.5.5",
        "223.6.6.6",
        "119.29.29.29",
        "119.28.28.28",
        "114.114.114.114",
        "114.114.115.115",
        "180.76.76.76",
        "180.184.1.1",
        "180.184.2.2",
        "1.2.4.8",
        "210.2.4.8",
    ];
    const CN_DNS_IPV6: &[&str] = &["2400:3200::1", "2400:3200:baba::1", "2402:4e00::", "2402:4e00:1::"];
    const CN_OR_LOCAL_DOMAINS: &[&str] = &[
        "doh.pub",
        "dot.pub",
        "dns.pub",
        "alidns.com",
        "dns.alidns.com",
        "doh.360.cn",
        "dot.360.cn",
        "360.cn",
        "dnspod.cn",
        "dns.dnspod.cn",
        "bytedns.com",
        "cn",
        "lan",
        "local",
        "internal",
        "home",
        "home.arpa",
        "localhost",
        "corp",
        "intranet",
    ];

    if host.is_empty() {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        return ip.is_private()
            || ip.is_loopback()
            || ip.is_link_local()
            || is_shared_cgnat_v4(ip)
            || CN_DNS_IPV4.contains(&host);
    }
    if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
        return ip.is_loopback() || ip.is_unspecified() || is_ula_or_link_local_v6(ip) || CN_DNS_IPV6.contains(&host);
    }
    if !host.contains('.') {
        return true;
    }
    CN_OR_LOCAL_DOMAINS.iter().any(|suffix| {
        host == *suffix
            || (host.len() > suffix.len()
                && host.ends_with(suffix)
                && host.as_bytes()[host.len() - suffix.len() - 1] == b'.')
    })
}

fn collect_dns_field_strings(dns: &Mapping, field: &str) -> Vec<std::string::String> {
    let Some(val) = dns.get(field) else {
        return Vec::new();
    };
    match val {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(seq) => seq.iter().filter_map(Value::as_str).map(ToString::to_string).collect(),
        _ => Vec::new(),
    }
}

fn derive_domestic_doh_upstream(dns: &Mapping) -> std::string::String {
    for raw in collect_dns_field_strings(dns, "nameserver") {
        let parsed = parse_dns_upstream(&raw);
        if !parsed.is_special
            && !parsed.is_plain
            && !parsed.clean_upstream.is_empty()
            && is_local_or_domestic_dns_host(&parsed.host)
        {
            // 降级保通模板必须去掉 fragment（如 `#h3=true`）：fragment 只用于「原样保留
            // 订阅条目」，而 domestic_fallback 是派生的通用境内 DoH，携带他家/他字段的
            // 参数会导致降级后仍解析失败（D-1）。
            return std::string::String::from(
                parsed
                    .clean_upstream
                    .split_once('#')
                    .map_or(parsed.clean_upstream.as_str(), |(base, _)| base),
            );
        }
    }
    "https://doh.pub/dns-query".to_string()
}

fn foreign_plain_doh_url(host: &str) -> Option<&'static str> {
    match host {
        "8.8.8.8" => Some("https://8.8.8.8/dns-query"),
        "8.8.4.4" => Some("https://8.8.4.4/dns-query"),
        "1.1.1.1" => Some("https://1.1.1.1/dns-query"),
        "1.0.0.1" => Some("https://1.0.0.1/dns-query"),
        "9.9.9.9" => Some("https://9.9.9.9/dns-query"),
        "149.112.112.112" => Some("https://149.112.112.112/dns-query"),
        "208.67.222.222" | "208.67.220.220" => Some("https://doh.opendns.com/dns-query"),
        _ => None,
    }
}

fn dual_secondary_v4(ip: &str) -> Option<&'static str> {
    match ip {
        "8.8.8.8" => Some("8.8.4.4"),
        "8.8.4.4" => Some("8.8.8.8"),
        "1.1.1.1" => Some("1.0.0.1"),
        "1.0.0.1" => Some("1.1.1.1"),
        "9.9.9.9" => Some("149.112.112.112"),
        "149.112.112.112" => Some("9.9.9.9"),
        "208.67.222.222" => Some("208.67.220.220"),
        "208.67.220.220" => Some("208.67.222.222"),
        _ => None,
    }
}

fn dual_secondary_v6(ip: &str) -> Option<&'static str> {
    match ip {
        "2001:4860:4860::8888" => Some("2001:4860:4860::8844"),
        "2001:4860:4860::8844" => Some("2001:4860:4860::8888"),
        "2606:4700:4700::1111" => Some("2606:4700:4700::1001"),
        "2606:4700:4700::1001" => Some("2606:4700:4700::1111"),
        "2620:fe::fe" => Some("2620:fe::9"),
        "2620:fe::9" => Some("2620:fe::fe"),
        "2620:119:35::35" => Some("2620:119:53::53"),
        "2620:119:53::53" => Some("2620:119:35::35"),
        _ => None,
    }
}

fn known_domain_landing_v4_ips(domain: &str) -> &'static [&'static str] {
    match domain {
        "dns.google" | "dns.google.com" | "8888.google" => &["8.8.8.8", "8.8.4.4"],
        "cloudflare-dns.com" | "dns.cloudflare.com" => &["1.1.1.1", "1.0.0.1"],
        "dns.quad9.net" => &["9.9.9.9", "149.112.112.112"],
        "doh.opendns.com" => &["208.67.222.222", "208.67.220.220"],
        _ => &[],
    }
}

/// 统一从一组 DNS 上游主机生成前置 `PROXY` 保通规则（A9：`dns.fallback` 与 `nameserver-policy` 共用真源）。
///
/// - 私网/环回/链路本地/境内 DNS IP 自动跳过；
/// - IP 字面量补全双活辅 IP（Google / Cloudflare / Quad9 / OpenDNS 双向对偶）；
/// - 域名型上游生成 `DOMAIN,<host>,PROXY`，并对已知域名补齐固定落地 IPv4 规则。
fn dns_upstream_proxy_rules_from_hosts<'a, I>(hosts: I) -> Vec<std::string::String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut rules = Vec::new();
    let mut seen = HashSet::new();

    for raw_host in hosts {
        let host = raw_host.trim().to_ascii_lowercase();
        if host.is_empty() || is_local_or_domestic_dns_host(&host) {
            continue;
        }
        let mut candidates: Vec<std::string::String> = Vec::new();

        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            if ip.is_private() || ip.is_loopback() || ip.is_link_local() || is_shared_cgnat_v4(ip) {
                continue;
            }
            candidates.push(format!("IP-CIDR,{host}/32,PROXY,no-resolve"));
            if let Some(peer) = dual_secondary_v4(&host) {
                candidates.push(format!("IP-CIDR,{peer}/32,PROXY,no-resolve"));
            }
        } else if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
            if ip.is_loopback() || ip.is_unspecified() || is_ula_or_link_local_v6(ip) {
                continue;
            }
            candidates.push(format!("IP-CIDR6,{host}/128,PROXY,no-resolve"));
            if let Some(peer) = dual_secondary_v6(&host) {
                candidates.push(format!("IP-CIDR6,{peer}/128,PROXY,no-resolve"));
            }
        } else {
            candidates.push(format!("DOMAIN,{host},PROXY"));
            for landing_ip in known_domain_landing_v4_ips(&host) {
                candidates.push(format!("IP-CIDR,{landing_ip}/32,PROXY,no-resolve"));
            }
        }

        for rule in candidates {
            if seen.insert(rule.clone()) {
                rules.push(rule);
            }
        }
    }

    rules
}

fn find_dns_proxy_rule_insert_pos(rules: &[Value]) -> usize {
    if let Some(gfw_idx) = rules.iter().position(|r| r.as_str() == Some("RULE-SET,gfwlist,PROXY")) {
        let mut pos = gfw_idx + 1;
        while let Some(next_str) = rules.get(pos).and_then(Value::as_str) {
            if next_str.ends_with(",PROXY") || next_str.ends_with(",PROXY,no-resolve") {
                pos += 1;
            } else {
                break;
            }
        }
        return pos;
    }

    rules
        .iter()
        .position(|r| {
            r.as_str().is_some_and(|s| {
                s.starts_with("IP-CIDR,127.0.0.0/8,")
                    || s.starts_with("GEOIP,")
                    || s.starts_with("IP-CIDR6,::/0,")
                    || s.starts_with("MATCH,")
            })
        })
        .unwrap_or(rules.len())
}

/// 机场订阅 DNS 统一卫生治理（BUG-308 / BUG-364 / A9 / C10-C12 / §3.7）：
/// - 统一覆盖 `dns.nameserver`、`dns.fallback` 与 `dns.nameserver-policy`（绝不触碰
///   必须保持明文的 `default-nameserver`，也不删除任何 `nameserver-policy` 策略键）；
/// - 仅以 `is_local_or_domestic_dns_host` 判定是否境内/内网可达（不再因条目出现在
///   `fallback` 而误判，也不为境内 DNS 生成 `PROXY` 规则）；
/// - 剥离 Mihomo 不识别的 `#DIRECT` / `#PROXY` 伪后缀，保留 `#h3=true` 等原生参数；
/// - `respect-rules` 未开启（或携带 `#DIRECT`）时，将不可达海外上游就地改写为由 `nameserver`
///   推导的可达境内 DoH（兜底 `https://doh.pub/dns-query`）并去重；
/// - `respect-rules: true` 时，将境外明文 UDP 公共 DNS 升级为 DoH（消除 TCP 承载 UDP
///   竞速取消），并与 `dns_fallback_proxy_rules` 共用真源向 `rules` 补齐 `PROXY` 规则。
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod nameserver_policy_tests {
    use super::sanitize_dns_upstreams;
    use serde_yaml_ng::{Mapping, Value};

    #[test]
    fn rewrites_unreachable_foreign_upstreams_and_preserves_keys_when_respect_rules_disabled() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  nameserver:
    - https://dns.alidns.com/dns-query
    - https://dns.google/dns-query
    - 8.8.8.8
  fallback:
    - https://1.1.1.1/dns-query
    - 8.8.4.4
  default-nameserver:
    - 223.5.5.5
    - 8.8.8.8
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.github.com": 8.8.8.8
    "+.unknown-foreign.example": https://dns.mullvad.net/dns-query
    "+.mixed.example":
      - https://dns.cloudflare.com/dns-query
      - https://dns.alidns.com/dns-query
      - https://dns.alidns.com/dns-query
    "+.corp.local": 10.0.0.1
    "geosite:cn": https://dns.alidns.com/dns-query
    "*.lan": system
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let dns = cleaned.get("dns").unwrap();

        // C10：respect-rules: false 下，nameserver 与 fallback 中的海外加密/明文上游亦就地降级为境内 DoH 并去重
        let nameserver: Vec<&str> = dns
            .get("nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(nameserver, vec!["https://dns.alidns.com/dns-query"]);

        let fallback: Vec<&str> = dns
            .get("fallback")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(fallback, vec!["https://dns.alidns.com/dns-query"]);

        // default-nameserver 保持明文不动
        let defaults: Vec<&str> = dns
            .get("default-nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(defaults, vec!["223.5.5.5", "8.8.8.8"]);

        let policy = dns.get("nameserver-policy").and_then(Value::as_mapping).unwrap();

        // BUG-364：绝不因海外上游不可达而删除策略键，统一就地改写为配置推导的可达境内 DoH
        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(
            policy.get("+.github.com").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(
            policy.get("+.unknown-foreign.example").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(policy.get("+.corp.local").and_then(Value::as_str), Some("10.0.0.1"));
        assert_eq!(
            policy.get("geosite:cn").and_then(Value::as_str),
            Some("https://dns.alidns.com/dns-query")
        );
        assert_eq!(policy.get("*.lan").and_then(Value::as_str), Some("system"));

        let mixed = policy.get("+.mixed.example").and_then(Value::as_sequence).unwrap();
        assert_eq!(mixed.len(), 1);
        assert_eq!(mixed[0].as_str(), Some("https://dns.alidns.com/dns-query"));
    }

    #[test]
    fn preserves_nameserver_policy_with_doh_pub_fallback_when_no_domestic_nameserver() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  nameserver:
    - https://1.1.1.1/dns-query
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.youtube.com": https://dns.google/dns-query
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let dns = cleaned.get("dns").unwrap();
        let nameserver: Vec<&str> = dns
            .get("nameserver")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(nameserver, vec!["https://doh.pub/dns-query"]);

        let policy = dns.get("nameserver-policy").and_then(Value::as_mapping).unwrap();
        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        assert_eq!(
            policy.get("+.youtube.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
    }

    #[test]
    fn respect_rules_true_rewrites_plain_udp_keeps_doh_and_injects_dual_proxy_rules() {
        let config: Mapping = serde_yaml_ng::from_str(
            r#"
dns:
  enable: true
  respect-rules: true
  nameserver:
    - https://doh.pub/dns-query
  fallback:
    - https://doh.pub/dns-query
    - 223.5.5.5
  nameserver-policy:
    "+.google.com": https://dns.cloudflare.com/dns-query
    "+.github.com": 8.8.8.8
    "+.blocked-direct.com": https://dns.google/dns-query#DIRECT
    "+.proxied-h3.com": https://8.8.8.8/dns-query#PROXY&h3=true
    "+.cn-in-fallback.com": https://doh.pub/dns-query
    "+.local.lan": 192.168.1.1
rules:
  - RULE-SET,gfwlist,PROXY
  - IP-CIDR,8.8.8.8/32,PROXY,no-resolve
  - IP-CIDR,127.0.0.0/8,DIRECT,no-resolve
  - GEOIP,CN,DIRECT
  - MATCH,DIRECT
"#,
        )
        .unwrap();

        let cleaned = sanitize_dns_upstreams(config);
        let policy = cleaned
            .get("dns")
            .unwrap()
            .get("nameserver-policy")
            .and_then(Value::as_mapping)
            .unwrap();

        assert_eq!(
            policy.get("+.google.com").and_then(Value::as_str),
            Some("https://dns.cloudflare.com/dns-query")
        );
        assert_eq!(
            policy.get("+.github.com").and_then(Value::as_str),
            Some("https://8.8.8.8/dns-query")
        );
        // 废弃的 `#DIRECT` 海外条目不再删除键，改写为境内可达 DoH 且不写入 `#DIRECT`
        assert_eq!(
            policy.get("+.blocked-direct.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        // C12：剥离 `#PROXY` 伪后缀的同时保留合法 Mihomo `#h3=true` 参数
        assert_eq!(
            policy.get("+.proxied-h3.com").and_then(Value::as_str),
            Some("https://8.8.8.8/dns-query#h3=true")
        );
        // C11：境内 DoH 即使出现在 fallback 中也不得被误判为境外
        assert_eq!(
            policy.get("+.cn-in-fallback.com").and_then(Value::as_str),
            Some("https://doh.pub/dns-query")
        );
        assert_eq!(policy.get("+.local.lan").and_then(Value::as_str), Some("192.168.1.1"));

        let rules: Vec<&str> = cleaned
            .get("rules")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(
            rules,
            vec![
                "RULE-SET,gfwlist,PROXY",
                "IP-CIDR,8.8.8.8/32,PROXY,no-resolve",
                "DOMAIN,dns.cloudflare.com,PROXY",
                "IP-CIDR,1.1.1.1/32,PROXY,no-resolve",
                "IP-CIDR,1.0.0.1/32,PROXY,no-resolve",
                "IP-CIDR,8.8.4.4/32,PROXY,no-resolve",
                "IP-CIDR,127.0.0.0/8,DIRECT,no-resolve",
                "GEOIP,CN,DIRECT",
                "MATCH,DIRECT",
            ]
        );
    }
}

#[derive(Default)]
struct SanitizeDnsCounters {
    rewritten_foreign: usize,
    rewritten_plain: usize,
}

fn push_unique_string(target: &mut Vec<std::string::String>, value: std::string::String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn sanitize_upstream_items<'a, I>(
    raw_items: I,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) -> Vec<std::string::String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut cleaned: Vec<std::string::String> = Vec::new();
    for raw in raw_items {
        let parsed = parse_dns_upstream(raw);
        if parsed.clean_upstream.is_empty() {
            continue;
        }
        if parsed.is_special || is_local_or_domestic_dns_host(&parsed.host) {
            push_unique_string(&mut cleaned, parsed.clean_upstream);
            continue;
        }

        if parsed.force_direct || !respect_rules {
            counters.rewritten_foreign += 1;
            push_unique_string(&mut cleaned, domestic_fallback.to_string());
            continue;
        }

        let target_upstream = if parsed.is_plain {
            if let Some(doh) = foreign_plain_doh_url(&parsed.host) {
                counters.rewritten_plain += 1;
                doh.to_string()
            } else {
                parsed.clean_upstream
            }
        } else {
            parsed.clean_upstream
        };

        let target_host = extract_upstream_host(&target_upstream)
            .unwrap_or(parsed.host)
            .to_ascii_lowercase();
        if !target_host.is_empty() {
            push_unique_string(proxy_hosts, target_host);
        }
        push_unique_string(&mut cleaned, target_upstream);
    }
    cleaned
}

fn sanitize_dns_list_field(
    dns: &mut Mapping,
    field: &str,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) {
    let Some(val) = dns.get_mut(field) else {
        return;
    };
    let (was_scalar, raw_items): (bool, Vec<std::string::String>) = match val {
        Value::String(s) => (true, vec![s.clone()]),
        Value::Sequence(seq) => (
            false,
            seq.iter().filter_map(Value::as_str).map(ToString::to_string).collect(),
        ),
        _ => return,
    };
    if raw_items.is_empty() {
        return;
    }

    let cleaned = sanitize_upstream_items(
        raw_items.iter().map(std::string::String::as_str),
        respect_rules,
        domestic_fallback,
        proxy_hosts,
        counters,
    );

    if cleaned.is_empty() {
        *val = if was_scalar {
            Value::String(domestic_fallback.to_string())
        } else {
            Value::Sequence(vec![Value::String(domestic_fallback.to_string())])
        };
    } else if was_scalar && cleaned.len() == 1 {
        if let Some(single) = cleaned.into_iter().next() {
            *val = Value::String(single);
        }
    } else {
        *val = Value::Sequence(cleaned.into_iter().map(Value::String).collect());
    }
}

fn sanitize_nameserver_policy_map(
    dns: &mut Mapping,
    respect_rules: bool,
    domestic_fallback: &str,
    proxy_hosts: &mut Vec<std::string::String>,
    counters: &mut SanitizeDnsCounters,
) {
    let Some(Value::Mapping(policy)) = dns.get_mut("nameserver-policy") else {
        return;
    };
    let mut next_policy = Mapping::with_capacity(policy.len());

    for (key, val) in policy.iter() {
        let (was_scalar, raw_items): (bool, Vec<&str>) = match val {
            Value::String(s) => (true, vec![s.as_str()]),
            Value::Sequence(seq) => (false, seq.iter().filter_map(Value::as_str).collect()),
            other => {
                next_policy.insert(key.clone(), other.clone());
                continue;
            }
        };

        let cleaned = sanitize_upstream_items(raw_items, respect_rules, domestic_fallback, proxy_hosts, counters);

        if cleaned.is_empty() {
            next_policy.insert(key.clone(), Value::String(domestic_fallback.to_string()));
        } else if was_scalar && cleaned.len() == 1 {
            if let Some(single) = cleaned.into_iter().next() {
                next_policy.insert(key.clone(), Value::String(single));
            }
        } else {
            next_policy.insert(
                key.clone(),
                Value::Sequence(cleaned.into_iter().map(Value::String).collect()),
            );
        }
    }

    *policy = next_policy;
}

fn inject_dns_proxy_rules(config: &mut Mapping, proxy_hosts: &[std::string::String]) {
    if proxy_hosts.is_empty() {
        return;
    }
    let generated = dns_upstream_proxy_rules_from_hosts(proxy_hosts.iter().map(std::string::String::as_str));
    let existing_rules = config.get("rules").and_then(Value::as_sequence);
    let to_insert: Vec<Value> = generated
        .into_iter()
        .filter(|rule| {
            !existing_rules
                .map(|seq| seq.iter().any(|r| r.as_str() == Some(rule.as_str())))
                .unwrap_or(false)
        })
        .map(Value::String)
        .collect();
    if !to_insert.is_empty()
        && let Some(Value::Sequence(rules)) = config.get_mut("rules")
    {
        let insert_pos = find_dns_proxy_rule_insert_pos(rules);
        rules.splice(insert_pos..insert_pos, to_insert);
    }
}

fn sanitize_dns_upstreams(mut config: Mapping) -> Mapping {
    let mut proxy_hosts: Vec<std::string::String> = Vec::new();
    let mut counters = SanitizeDnsCounters::default();

    if let Some(Value::Mapping(dns)) = config.get_mut("dns") {
        let respect_rules = dns.get("respect-rules").and_then(Value::as_bool).unwrap_or(false);
        let domestic_fallback = derive_domestic_doh_upstream(dns);

        for field in ["nameserver", "fallback"] {
            sanitize_dns_list_field(
                dns,
                field,
                respect_rules,
                &domestic_fallback,
                &mut proxy_hosts,
                &mut counters,
            );
        }
        sanitize_nameserver_policy_map(dns, respect_rules, &domestic_fallback, &mut proxy_hosts, &mut counters);
    }

    inject_dns_proxy_rules(&mut config, &proxy_hosts);

    if counters.rewritten_foreign > 0 || counters.rewritten_plain > 0 {
        logging!(
            info,
            Type::Config,
            "DNS 上游完成统一卫生处理：海外不可达改写境内 DoH {} 条，明文 UDP 升级 DoH {} 条",
            counters.rewritten_foreign,
            counters.rewritten_plain
        );
    }
    config
}

/// 从 DNS 上游字符串中取出主机名部分。
///
/// 支持 mihomo 的全部常见写法：`1.1.1.1`、`8.8.8.8:53`、
/// `https://1.1.1.1/dns-query`、`tls://8.8.8.8`、
/// `https://[2606:4700:4700::1111]/dns-query#h3=true`。
fn extract_upstream_host(raw: &str) -> Option<std::string::String> {
    let s = raw.trim();
    // 去掉 `#h3=true` 之类的策略参数
    let s = s.split('#').next()?;
    // 去掉 scheme
    let s = match s.find("://") {
        Some(idx) => s.get(idx + 3..)?,
        None => s,
    };
    // 去掉 path 与 query
    let host_port = s.split(['/', '?']).next()?;

    let host = if let Some(rest) = host_port.strip_prefix('[') {
        // `[IPv6]:port`
        rest.split(']').next()?
    } else if host_port.matches(':').count() == 1 {
        // `host:port`；单冒号一定不是 IPv6 字面量
        host_port.split(':').next()?
    } else {
        // 裸 IPv6 或纯主机名
        host_port
    };

    let host = host.trim();
    (!host.is_empty()).then(|| std::string::String::from(host))
}

// 原由 `dns.fallback` 独立生成强制走代理规则（已重构并由下方注释说明单一路径替代）：
// `dns_fallback_proxy_rules` 已删除（B-1/F-2）：PROXY 规则统一由
// `sanitize_dns_upstreams` → `inject_dns_proxy_rules` 单一路径生成，
// 覆盖 fallback + nameserver + nameserver-policy 三域，不再有第二注入点。

async fn enforce_mini_agreements(mut config: Mapping) -> Mapping {
    // 1. Extract all raw proxies from `proxies` sequence
    //    过滤广告/假节点（"网址"、"官网"、"剩余流量" 等），避免它们进入 PROXY 组后
    //    被内核 reload_config 选中触发自愈死循环
    let mut proxy_names = Vec::new();
    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for p in proxies {
            let Some(m) = p.as_mapping() else {
                // 纯字符串形式（"- 节点名"）没有 server 字段，沿用名字过滤
                if let Some(name) = p.as_str() {
                    if !crate::utils::node::is_dummy_node(name) {
                        proxy_names.push(Value::from(name.to_owned()));
                    }
                }
                continue;
            };
            let Some(name) = m.get("name").and_then(Value::as_str) else {
                continue;
            };
            if crate::utils::node::is_dummy_node(name) {
                continue;
            }
            // 【BUG-301】占位假节点的名字可伪装成 URL 绕过名字过滤（现场：
            // "v0-网址:xxx" + server='1'），但 server 字段藏不住。按 server 有效性
            // 拦截入组；节点定义保留在 proxies: 中（与 dummy 过滤同策略），只是
            // 不允许它成为 PROXY 组成员——否则内核重启后 Selector 落回首位死节点，
            // TUN 下全系统流量打进死地址触发 connect error 风暴。
            if let Some(server) = m.get("server").and_then(Value::as_str) {
                if !crate::utils::node::is_valid_node_server(server) {
                    logging!(
                        warn,
                        Type::Config,
                        "跳过 server 字段无效的节点: {name} (server={server:?})"
                    );
                    continue;
                }
            }
            proxy_names.push(Value::from(name.to_owned()));
        }
    }

    // 2. Extract all proxy-provider names from `proxy-providers` mapping
    let mut provider_names = Vec::new();
    if let Some(Value::Mapping(providers)) = config.get("proxy-providers") {
        for (k, _) in providers {
            if let Some(name) = k.as_str() {
                provider_names.push(Value::from(name.to_owned()));
            }
        }
    }

    // 3. Construct the control group "PROXY" with type "select"
    //    —— 用户/app 控制，选择稳定、手动选点持久（不被内核自动翻回）
    let proxy_names_value = Value::from(proxy_names.clone());
    let provider_names_value = Value::from(provider_names.clone());

    let mut single_group = Mapping::new();
    single_group.insert(Value::from("name"), Value::from("PROXY"));
    single_group.insert(Value::from("type"), Value::from("select"));
    if !proxy_names.is_empty() {
        single_group.insert(Value::from("proxies"), proxy_names_value.clone());
    }
    if !provider_names.is_empty() {
        single_group.insert(Value::from("use"), provider_names_value.clone());
    }
    // If both are empty, fallback to DIRECT
    if single_group.get("proxies").is_none() && single_group.get("use").is_none() {
        single_group.insert(Value::from("proxies"), Value::from(vec![Value::from("DIRECT")]));
    }

    // 3b. Construct the measurement group "PROXY__METRICS" with type "url-test", same membership.
    //     Mihomo 内核按 interval 周期性直接拨测每个成员，维护其 history/alive（TUN 下可靠）。
    //     这是节点延迟的唯一权威来源；app 不再用 delay_proxy_by_name 自测活跃节点
    //     （旧设计在 TUN 下对活跃节点自测会回环返回 timeout，是断流+全 timeout 的根因）。
    let test_url = Config::verge()
        .await
        .latest_arc()
        .default_latency_test
        .as_deref()
        .unwrap_or("https://cp.cloudflare.com/generate_204")
        .to_string();

    let mut metrics_group = Mapping::new();
    metrics_group.insert(Value::from("name"), Value::from("PROXY__METRICS"));
    metrics_group.insert(Value::from("type"), Value::from("url-test"));
    if !proxy_names.is_empty() {
        metrics_group.insert(Value::from("proxies"), proxy_names_value.clone());
    }
    if !provider_names.is_empty() {
        metrics_group.insert(Value::from("use"), provider_names_value.clone());
    }
    if metrics_group.get("proxies").is_none() && metrics_group.get("use").is_none() {
        metrics_group.insert(Value::from("proxies"), Value::from(vec![Value::from("DIRECT")]));
    }
    metrics_group.insert(Value::from("url"), Value::from(test_url));
    metrics_group.insert(Value::from("interval"), Value::from(300));

    // Replace the entire proxy-groups sequence with [PROXY (control), PROXY__METRICS (measurement)]
    config.insert(
        Value::from("proxy-groups"),
        Value::from(vec![Value::from(single_group), Value::from(metrics_group)]),
    );

    // 4. Inject rule-providers for GFWList
    //
    // 【随包预置 + 定时更新（BUG-309）】GFWList 规则集是境外域名走代理的唯一判定源。
    // 构建/发版阶段（scripts/prebuild.mjs）主动拉取最新 Loyalsoldier/clash-rules gfw.txt
    // 随包打入 resources/gfwlist.yaml，启动早期由 init_resources 释放至
    // <app_home_dir>/ruleset/gfwlist.yaml。因此全新安装第 0 毫秒本地缓存即已就绪，
    // 无需首次启动时联网等待拉取（彻底消除拉取完成前 GitHub/YouTube 落 MATCH,DIRECT
    // 直连被墙的冷启动空窗）；运行期仍由内核按 interval: 86400（24h）经 PROXY 通道
    // 后台定时拉取更新。
    let mut gfwlist_provider = Mapping::new();
    gfwlist_provider.insert(Value::from("type"), Value::from("http"));
    gfwlist_provider.insert(Value::from("behavior"), Value::from("domain"));
    gfwlist_provider.insert(
        Value::from("url"),
        Value::from("https://cdn.jsdelivr.net/gh/Loyalsoldier/clash-rules@release/gfw.txt"),
    );
    gfwlist_provider.insert(Value::from("path"), Value::from("./ruleset/gfwlist.yaml"));
    gfwlist_provider.insert(Value::from("interval"), Value::from(86400));
    gfwlist_provider.insert(Value::from("proxy"), Value::from("PROXY"));

    let raw_rule_providers = match config.remove("rule-providers") {
        Some(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    };

    // 5. Construct the rules list
    let mut final_rules = Vec::new();

    // Add manual rules from prepend-rules
    if let Some(Value::Sequence(prepend)) = config.remove("prepend-rules") {
        for rule in prepend {
            final_rules.push(rule);
        }
    }

    // 注：境外域名走代理由 GFWList 规则集判断（规则集从 GEOIP/GEOSITE 更换而来，
    // 省去 geosite.dat 的常驻内存开销；GEOIP 数据与 gfwlist.yaml 均随包携带，
    // 见 utils::init::init_resources）。
    // MATCH 兜底策略由 rule_fallback 控制；IPv6 境内外分流依赖 Country.mmdb
    // （随包携带、离线可用），见下方 IPv6 兜底规则。

    // Add GFWList rule:
    final_rules.push(Value::from("RULE-SET,gfwlist,PROXY"));

    // DNS fallback 上游走代理的 PROXY 规则已统一由 `inject_dns_proxy_rules` 在
    // `sanitize_dns_upstreams` 末端生成（B-1/F-2 单点注入），此处不再重复注入。

    // Add local LAN bypass rules (DIRECT) to avoid DNS loopback & LAN routing deadlock
    // 【IPv4 私网 / 环回 / 链路本地】防止「代理所有流量」（MATCH,PROXY）下本地流量被误转远端代理
    final_rules.push(Value::from("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));
    // IPv4 链路本地（APIPA，169.254.0.0/16）：与下方 IPv6 `fe80::/10` 对称，此前缺失
    final_rules.push(Value::from("IP-CIDR,169.254.0.0/16,DIRECT,no-resolve"));

    // 【IPv6 环回 / 未指定 / ULA / 链路本地 / 组播】
    // ULA（fc00::/7）覆盖 TUN 自身地址段（fdfe:dcba:9876::1/126），必须直连。
    final_rules.push(Value::from("IP-CIDR6,::1/128,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::/128,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,fc00::/7,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,fe80::/10,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,ff00::/8,DIRECT,no-resolve"));

    // 【IPv4-mapped IPv6（::ffff:a.b.c.d）的私网/环回等价段】
    // 注意：**不可**整体把 `::ffff:0:0/96` 设为 DIRECT —— 那会把国外目标也直连、绕过代理；
    // 只精确匹配与上方 IPv4 私网规则一一对应的映射段（96 位映射前缀 + IPv4 前缀长度）。
    final_rules.push(Value::from("IP-CIDR6,::ffff:127.0.0.0/104,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:10.0.0.0/104,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:172.16.0.0/108,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:192.168.0.0/112,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:100.64.0.0/106,DIRECT,no-resolve"));
    final_rules.push(Value::from("IP-CIDR6,::ffff:169.254.0.0/112,DIRECT,no-resolve"));

    // Add manual rules from append-rules
    if let Some(Value::Sequence(append)) = config.remove("append-rules") {
        for rule in append {
            final_rules.push(rule);
        }
    }

    // 【国内 GEOIP 兜底】
    // 1) 【必须用 GEOIP 而非 GEOIP6】mihomo（Clash Meta）不支持 `GEOIP6` 规则类型
    //    （老 Clash Premium 语法），写入即 `unsupported rule type: GEOIP6`，会导致配置校验失败、
    //    内核拒绝启动并全部流量回落直连。mihomo 的 `GEOIP` 自动覆盖 IPv4/IPv6（查 mmdb 对应段）。
    // 2) 【刻意不加 no-resolve】保持域名目标先由内核 DNS 解析后再比对 CN IP 段直连。
    // 3) 【不再注入 IP-CIDR6,::/0,PROXY】原位于 GEOIP,CN,DIRECT 与 MATCH,DIRECT 之间的
    //    `IP-CIDR6,::/0,PROXY` 会将境内 CDN/下载/通讯 IPv6 流量及非 GFWList 域名 AAAA 会话
    //    全部送入付费代理节点偷跑流量，故默认 GFWList 模式保持对称直连兜底 `MATCH,DIRECT`。
    // Read verge config to get the rule fallback type
    let verge = Config::verge().await.latest_arc();
    let rule_fallback = verge.rule_fallback.as_deref().unwrap_or("direct");

    // Add MATCH final rule according to the fallback type
    // 注意：默认 GFWList 模式（rule_fallback != "proxy"）仅注入 GEOIP,CN,DIRECT + MATCH,DIRECT 对称兜底；
    // 已移除原先位于 MATCH,DIRECT 之前的 IP-CIDR6,::/0,PROXY，避免境内未命中 GeoIP 的 CDN/下载/通讯 IPv6 流量
    // 及非 GFWList 域名 AAAA 会话偷跑进机场付费节点计费。
    // 仅当用户选择「代理全部流量」（rule_fallback == "proxy"）时，直接落入 MATCH,PROXY。
    if rule_fallback == "proxy" {
        final_rules.push(Value::from("MATCH,PROXY"));
    } else {
        final_rules.push(Value::from("GEOIP,CN,DIRECT"));
        final_rules.push(Value::from("MATCH,DIRECT"));
    }

    // 仅保留被最终 rules（含用户 prepend-rules / append-rules）实际引用的 rule-providers。
    // 机场订阅常自带 China-Site / China-IP / Local-IP 等未被 Clash Mini 规则链使用的
    // rule-providers（3.1.9 现场实证：订阅自带 3 个 .mrs 规则集共 ~700KB，虽未被 rules
    // 引用却仍触发内核首次启动联网拉取并常驻内存；若机场规则源被墙更会拖慢甚至卡死首次校验）。
    // 按引用关系裁剪后，默认仅保留已随包预置的 gfwlist，实现首次启动路由规则 100% 离线零拉取。
    let mut referenced_providers: HashSet<std::string::String> = HashSet::new();
    referenced_providers.insert("gfwlist".to_string());
    for rule in &final_rules {
        if let Some(s) = rule.as_str() {
            let mut parts = s.split(',');
            if let (Some(kind), Some(name)) = (parts.next(), parts.next())
                && kind.trim().eq_ignore_ascii_case("RULE-SET")
            {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    referenced_providers.insert(trimmed.to_string());
                }
            }
        }
    }

    let mut rule_providers = Mapping::new();
    for (k, v) in raw_rule_providers {
        if let Some(name) = k.as_str()
            && referenced_providers.contains(name)
        {
            rule_providers.insert(k, v);
        }
    }
    rule_providers.insert(Value::from("gfwlist"), Value::from(gfwlist_provider));
    config.insert(Value::from("rule-providers"), Value::from(rule_providers));

    config.insert(Value::from("rules"), Value::from(final_rules));

    config
}

#[allow(clippy::collapsible_if, clippy::needless_borrows_for_generic_args)]
fn get_merged_proxies(profiles: &crate::config::profiles::IProfiles) -> Vec<Value> {
    use chrono::{Local, TimeZone as _};
    let mut all_proxies = Vec::new();
    let mut raw_proxies = Vec::new();

    if let Some(items) = profiles.get_items() {
        for item in items {
            // Only process remote and local types
            if let Some(itype) = &item.itype {
                if itype == "remote" || itype == "local" {
                    // Read file
                    if let Some(file) = &item.file {
                        if let Ok(file_path) = dirs::app_profiles_dir().map(|d| d.join(file.as_str())) {
                            if let Ok(mut mapping) = crate::utils::help::read_mapping(&file_path) {
                                if let Some(Value::Sequence(proxies)) = mapping.remove("proxies") {
                                    // Generate suffix from item.updated timestamp
                                    let ts = item.updated.unwrap_or(0);
                                    let suffix = if let Some(dt) = Local.timestamp_opt(ts, 0).single() {
                                        dt.format("%H%M%S").to_string()
                                    } else {
                                        "000000".to_string()
                                    };

                                    for proxy in proxies {
                                        let original_name = if let Some(map) = proxy.as_mapping() {
                                            map.get(&Value::from("name"))
                                                .and_then(Value::as_str)
                                                .unwrap_or("Proxy")
                                                .to_string()
                                        } else {
                                            "Proxy".to_string()
                                        };
                                        raw_proxies.push((proxy, original_name, suffix.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Identify duplicate (name, suffix) keys
    let mut name_suffix_counts = std::collections::HashMap::new();
    for (_, name, suffix) in &raw_proxies {
        let key = (name.clone(), suffix.clone());
        *name_suffix_counts.entry(key).or_insert(0) += 1;
    }

    // Format names and collect proxies
    let mut name_suffix_counters = std::collections::HashMap::new();
    for (mut proxy, name, suffix) in raw_proxies {
        let key = (name.clone(), suffix.clone());
        let count = name_suffix_counts.get(&key).copied().unwrap_or(0);

        let new_name = if count > 1 {
            let idx = name_suffix_counters.entry(key).or_insert(0);
            *idx += 1;
            format!("{} #{} ({})", name, idx, suffix)
        } else {
            format!("{} ({})", name, suffix)
        };

        if let Some(map) = proxy.as_mapping_mut() {
            map.insert(Value::from("name"), Value::from(new_name));
        }
        all_proxies.push(proxy);
    }

    all_proxies
}

/// Enhance mode
/// 返回最终订阅、该订阅包含的键、和script执行的结果
pub async fn enhance() -> Result<(Mapping, HashSet<String>, HashMap<String, ResultLog>)> {
    // gather config values
    let cfg_vals = get_config_values().await;
    let ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_tun_sniffer,
        enable_builtin,
        socks_enabled,
        http_enabled,
    } = cfg_vals;

    // collect profile items
    let profile = collect_profile_items().await?;
    let mut config = profile.config;

    // Always merge proxies from all remote and local subscription files
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    drop(profiles);
    let merged = get_merged_proxies(&profiles_arc);
    config.insert(Value::from("proxies"), Value::from(merged));

    let merge_item = profile.merge_item;
    let script_item = profile.script_item;
    let rules_item = profile.rules_item;
    let proxies_item = profile.proxies_item;
    let global_merge = profile.global_merge;
    let global_script = profile.global_script;
    let profile_name = profile.profile_name;

    // process globals
    let (config, exists_keys, result_map) =
        process_global_items(config, global_merge, global_script, &profile_name).await;

    // process profile-specific items
    let (config, exists_keys, result_map) = process_profile_items(
        config,
        exists_keys,
        result_map,
        rules_item,
        proxies_item,
        merge_item,
        script_item,
        &profile_name,
    )
    .await;

    // merge default clash config
    let config = merge_default_config(config, clash_config, socks_enabled, http_enabled).await;

    // builtin scripts
    let mut config = apply_builtin_scripts(config, clash_core, enable_builtin).await;

    config = enforce_mini_agreements(config).await;

    config = use_tun_with_sniffer(config, enable_tun, enable_tun_sniffer);
    config = use_sort(config);

    // dns settings
    config = apply_mandatory_dns_settings(config);
    config = sanitize_dns_upstreams(config);

    let mut exists_keys_set = HashSet::new();
    exists_keys_set.extend(exists_keys);

    Ok((config, exists_keys_set, result_map))
}

#[allow(clippy::expect_used, clippy::unwrap_used)]
#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_enforce_mini_agreements_logic() {
        use super::enforce_mini_agreements;
        use serde_yaml_ng::{Mapping, Value};

        // `enforce_mini_agreements` 经 `Config::verge()` → `Config::global()` →
        // `IClashTemp::new()` → `dirs::clash_path()` → `app_home_dir()` 触达全局
        // `APP_HANDLE`。单测环境没有 Tauri AppHandle，若走非 portable 分支会
        // panic（App handle not initialized）。将 PORTABLE_FLAG 置为 true 使
        // `app_home_dir()` 走 portable 分支（仅依赖 current_exe，不碰 app_handle），
        // 从而让本测试自包含、可单独运行——与本仓库其它单测（save_profile / prfitem）一致。
        let _ = crate::utils::dirs::PORTABLE_FLAG.get_or_init(|| true);

        let config_str = r#"
proxies:
  - name: "node-A"
    type: ss
  - name: "node-B"
    type: vmess
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxy-groups:
  - name: "Group-A"
    type: select
    proxies:
      - "node-A"
  - name: "Group-B"
    type: select
    proxies:
      - "node-B"
rule-providers:
  China-Site:
    type: http
    behavior: domain
    format: mrs
    url: https://example.com/China-Site.mrs
    path: ./ruleset/China-Site.mrs
  Custom-RuleSet:
    type: http
    behavior: domain
    url: https://example.com/custom.yaml
    path: ./ruleset/custom.yaml
prepend-rules:
  - PROCESS-NAME,custom-process,DIRECT
append-rules:
  - DOMAIN,custom-domain,REJECT
"#;

        let mut config: Mapping = serde_yaml_ng::from_str(config_str).unwrap();
        config = enforce_mini_agreements(config).await;

        // 1. Verify PROXY (control) + PROXY__METRICS (measurement) groups
        let groups = config.get("proxy-groups").and_then(Value::as_sequence).unwrap();
        assert_eq!(groups.len(), 2);

        let proxy_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY"))
            .and_then(Value::as_mapping)
            .unwrap();
        assert_eq!(proxy_group.get("type").unwrap().as_str(), Some("select"));

        let metrics_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("PROXY__METRICS"))
            .and_then(Value::as_mapping)
            .unwrap();
        assert_eq!(metrics_group.get("type").unwrap().as_str(), Some("url-test"));
        assert!(metrics_group.get("url").is_some());
        assert!(metrics_group.get("interval").is_some());

        let group_proxies = proxy_group.get("proxies").and_then(Value::as_sequence).unwrap();
        assert_eq!(group_proxies.len(), 2);
        assert_eq!(group_proxies[0].as_str(), Some("node-A"));
        assert_eq!(group_proxies[1].as_str(), Some("node-B"));

        let group_uses = proxy_group.get("use").and_then(Value::as_sequence).unwrap();
        assert_eq!(group_uses.len(), 1);
        assert_eq!(group_uses[0].as_str(), Some("providerA"));

        // 2. Verify gfwlist provider & rules (pre-bundled + 24h periodic update via PROXY),
        //    and verify unreferenced airport rule-providers (China-Site, Custom-RuleSet) are stripped
        let providers = config.get("rule-providers").and_then(Value::as_mapping).unwrap();
        assert_eq!(providers.len(), 1);
        assert!(providers.get("China-Site").is_none());
        assert!(providers.get("Custom-RuleSet").is_none());
        let gfwlist = providers.get("gfwlist").and_then(Value::as_mapping).unwrap();
        assert_eq!(gfwlist.get("type").unwrap().as_str(), Some("http"));
        assert_eq!(gfwlist.get("behavior").unwrap().as_str(), Some("domain"));
        assert_eq!(
            gfwlist.get("url").unwrap().as_str(),
            Some("https://cdn.jsdelivr.net/gh/Loyalsoldier/clash-rules@release/gfw.txt")
        );
        assert_eq!(gfwlist.get("path").unwrap().as_str(), Some("./ruleset/gfwlist.yaml"));
        assert_eq!(gfwlist.get("interval").unwrap().as_u64(), Some(86400));
        assert_eq!(gfwlist.get("proxy").unwrap().as_str(), Some("PROXY"));

        let rules = config.get("rules").and_then(Value::as_sequence).unwrap();
        // prepend(1) + GFWList(1) + IP-CIDR(6) + IP-CIDR6(5) + IPv4-mapped(6)
        // + append(1) + GEOIP,CN(1) + MATCH(1) = 22
        // MATCH=DIRECT because default rule_fallback is "direct" (GFWList mode); IP-CIDR6,::/0,PROXY 已移除
        assert_eq!(rules.len(), 22);
        assert_eq!(rules[0].as_str(), Some("PROCESS-NAME,custom-process,DIRECT"));
        assert_eq!(rules[1].as_str(), Some("RULE-SET,gfwlist,PROXY"));
        assert_eq!(rules[2].as_str(), Some("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
        assert_eq!(rules[3].as_str(), Some("IP-CIDR,172.16.0.0/12,DIRECT,no-resolve"));
        assert_eq!(rules[4].as_str(), Some("IP-CIDR,192.168.0.0/16,DIRECT,no-resolve"));
        assert_eq!(rules[5].as_str(), Some("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve"));
        assert_eq!(rules[6].as_str(), Some("IP-CIDR,100.64.0.0/10,DIRECT,no-resolve"));
        assert_eq!(rules[7].as_str(), Some("IP-CIDR,169.254.0.0/16,DIRECT,no-resolve"));
        assert_eq!(rules[8].as_str(), Some("IP-CIDR6,::1/128,DIRECT,no-resolve"));
        assert_eq!(rules[9].as_str(), Some("IP-CIDR6,::/128,DIRECT,no-resolve"));
        assert_eq!(rules[10].as_str(), Some("IP-CIDR6,fc00::/7,DIRECT,no-resolve"));
        assert_eq!(rules[11].as_str(), Some("IP-CIDR6,fe80::/10,DIRECT,no-resolve"));
        assert_eq!(rules[12].as_str(), Some("IP-CIDR6,ff00::/8,DIRECT,no-resolve"));
        assert_eq!(
            rules[13].as_str(),
            Some("IP-CIDR6,::ffff:127.0.0.0/104,DIRECT,no-resolve")
        );
        assert_eq!(
            rules[14].as_str(),
            Some("IP-CIDR6,::ffff:10.0.0.0/104,DIRECT,no-resolve")
        );
        assert_eq!(
            rules[15].as_str(),
            Some("IP-CIDR6,::ffff:172.16.0.0/108,DIRECT,no-resolve")
        );
        assert_eq!(
            rules[16].as_str(),
            Some("IP-CIDR6,::ffff:192.168.0.0/112,DIRECT,no-resolve")
        );
        assert_eq!(
            rules[17].as_str(),
            Some("IP-CIDR6,::ffff:100.64.0.0/106,DIRECT,no-resolve")
        );
        assert_eq!(
            rules[18].as_str(),
            Some("IP-CIDR6,::ffff:169.254.0.0/112,DIRECT,no-resolve")
        );
        assert_eq!(rules[19].as_str(), Some("DOMAIN,custom-domain,REJECT"));
        // IPv6 兜底（在 MATCH 之前，确保境外/投毒 IPv6 不直连超时）
        // 刻意不带 no-resolve：否则域名目标会跳过这两条规则、仍漏到 MATCH,DIRECT
        // GEOIP 而非 GEOIP6：mihomo 不支持 GEOIP6 规则类型（配置校验直接失败）
        assert_eq!(rules[20].as_str(), Some("GEOIP,CN,DIRECT"));
        assert_eq!(rules[21].as_str(), Some("MATCH,DIRECT"));

        // Verify prepend-rules and append-rules keys are removed
        assert!(config.get("prepend-rules").is_none());
        assert!(config.get("append-rules").is_none());
    }

    /// 回归：`dns.respect-rules: true` + `rule_fallback = direct` 时，fallback 的
    /// 境外 DoH 会被 `MATCH,DIRECT` 兜走并直连失败，导致防投毒链路整体失效
    /// （v3.0.0 现场：`dial DIRECT (match Match/) mihomo --> 8.8.8.8:443 error`）。
    /// 本用例锁定「按 fallback 里的 IP 字面量生成走代理规则」这一约定。
    #[test]
    fn dns_fallback_upstreams_are_forced_through_proxy() {
        use super::{collect_dns_field_strings, dns_upstream_proxy_rules_from_hosts, extract_upstream_host};
        use serde_yaml_ng::Mapping;

        // 标量与列表混用 + 各种 scheme / 端口 / h3 参数 / 去重
        let dns: Mapping = serde_yaml_ng::from_str(
            r"
fallback:
  - https://1.1.1.1/dns-query
  - https://8.8.8.8/dns-query#h3=true
  - tls://8.8.4.4:853
  - https://dns.google/dns-query
  - https://[2606:4700:4700::1111]/dns-query
  - 1.1.1.1
  - https://192.168.1.1/dns-query
  - 127.0.0.1
  - 10.0.0.1
  - https://[::1]/dns-query
  - https://[fe80::1]/dns-query
",
        )
        .unwrap();

        let hosts: Vec<std::string::String> = collect_dns_field_strings(&dns, "fallback")
            .iter()
            .filter_map(|raw| extract_upstream_host(raw))
            .collect();
        let rules: Vec<std::string::String> = dns_upstream_proxy_rules_from_hosts(hosts.iter().map(|s| s.as_str()));

        assert_eq!(
            rules,
            vec![
                std::string::String::from("IP-CIDR,1.1.1.1/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,1.0.0.1/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
                std::string::String::from("DOMAIN,dns.google,PROXY"),
                std::string::String::from("IP-CIDR6,2606:4700:4700::1111/128,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR6,2606:4700:4700::1001/128,PROXY,no-resolve"),
            ],
            "域名型上游与双活对偶辅IP（含IPv4/IPv6）生成走代理规则，按出现顺序去重"
        );

        // Quad9 与 OpenDNS 对偶辅 IP 自动补全（A9）
        let quad9_opendns: Mapping =
            serde_yaml_ng::from_str("fallback: ['https://9.9.9.9/dns-query', '208.67.222.222']").unwrap();
        let quad9_hosts: Vec<std::string::String> = collect_dns_field_strings(&quad9_opendns, "fallback")
            .iter()
            .filter_map(|raw| extract_upstream_host(raw))
            .collect();
        let quad9_rules: Vec<std::string::String> =
            dns_upstream_proxy_rules_from_hosts(quad9_hosts.iter().map(|s| s.as_str()));
        assert_eq!(
            quad9_rules,
            vec![
                std::string::String::from("IP-CIDR,9.9.9.9/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,149.112.112.112/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,208.67.222.222/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,208.67.220.220/32,PROXY,no-resolve"),
            ]
        );

        // 单独配置 8.8.8.8 时，应自动补齐对偶辅 IP 8.8.4.4
        let single_google: Mapping = serde_yaml_ng::from_str("fallback: ['https://8.8.8.8/dns-query']").unwrap();
        let single_hosts: Vec<std::string::String> = collect_dns_field_strings(&single_google, "fallback")
            .iter()
            .filter_map(|raw| extract_upstream_host(raw))
            .collect();
        let single_rules: Vec<std::string::String> =
            dns_upstream_proxy_rules_from_hosts(single_hosts.iter().map(|s| s.as_str()));
        assert_eq!(
            single_rules,
            vec![
                std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
            ]
        );

        // 单独配置 dns.google 时，应同时生成 DOMAIN 与 8.8.8.8/8.8.4.4 落地规则
        let domain_google: Mapping = serde_yaml_ng::from_str("fallback: ['https://dns.google/dns-query']").unwrap();
        let domain_hosts: Vec<std::string::String> = collect_dns_field_strings(&domain_google, "fallback")
            .iter()
            .filter_map(|raw| extract_upstream_host(raw))
            .collect();
        let domain_rules: Vec<std::string::String> =
            dns_upstream_proxy_rules_from_hosts(domain_hosts.iter().map(|s| s.as_str()));
        assert_eq!(
            domain_rules,
            vec![
                std::string::String::from("DOMAIN,dns.google,PROXY"),
                std::string::String::from("IP-CIDR,8.8.8.8/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,8.8.4.4/32,PROXY,no-resolve"),
            ]
        );

        // 没有上游时不得凭空造规则
        assert!(dns_upstream_proxy_rules_from_hosts(std::iter::empty::<&str>()).is_empty());
    }

    /// 回归（BUG-308 / C10）：订阅自带 `dns.nameserver` / `dns.fallback` 含境外明文 UDP 上游（如 8.8.8.8）时，
    /// `respect-rules: true` 下该查询穿代理，TCP 承载节点（anytls）上 UDP-over-TCP 会话
    /// 建立慢于境内 DoH、在 DNS 并发赛跑中必被取消（3.1.7 现场实证 7 条
    /// `failed to create session` 全落在 anytls 窗口内）；`respect-rules: false` 下直连必超时的
    /// 境外加密 DoH/DoT 与明文上游统一就地降级为境内 DoH。锁定「明文境外公共 DNS → DoH
    /// 改写并去重、境内明文与加密上游保留、default-nameserver 不动」的卫生改写约定。
    #[test]
    fn foreign_plain_dns_upstreams_in_nameserver_are_sanitized_to_doh() {
        use super::sanitize_dns_upstreams;
        use serde_yaml_ng::Mapping;

        let config: Mapping = serde_yaml_ng::from_str(
            r"
dns:
  respect-rules: true
  nameserver:
    - https://doh.pub/dns-query
    - https://dns.alidns.com/dns-query
    - 8.8.8.8
    - 8.8.8.8:53
    - 1.1.1.1
    - 223.5.5.5
    - 114.114.114.114
    - tls://8.8.4.4:853
    - https://8.8.4.4/dns-query
  default-nameserver:
    - 223.6.6.6
    - 8.8.8.8
rules:
  - RULE-SET,gfwlist,PROXY
  - MATCH,DIRECT
",
        )
        .unwrap();

        let sanitized = sanitize_dns_upstreams(config);
        let dns = sanitized.get("dns").unwrap();
        let nameservers: Vec<std::string::String> = dns
            .get("nameserver")
            .unwrap()
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        assert_eq!(
            nameservers,
            vec![
                std::string::String::from("https://doh.pub/dns-query"),
                std::string::String::from("https://dns.alidns.com/dns-query"),
                // 境外明文 UDP → DoH 改写并去重（裸 IP 与 :53 端口合并）
                std::string::String::from("https://8.8.8.8/dns-query"),
                std::string::String::from("https://1.1.1.1/dns-query"),
                // 境内明文上游直连可达，保留
                std::string::String::from("223.5.5.5"),
                std::string::String::from("114.114.114.114"),
                // 已加密上游原样保留
                std::string::String::from("tls://8.8.4.4:853"),
                std::string::String::from("https://8.8.4.4/dns-query"),
            ],
            "仅境外公共 DNS 明文形式被改写为 DoH"
        );

        // respect-rules: true 时 nameserver 中的境外上游亦自动补齐 PROXY 保通规则
        let rules: Vec<&str> = sanitized
            .get("rules")
            .unwrap()
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(rules.contains(&"IP-CIDR,8.8.8.8/32,PROXY,no-resolve"));
        assert!(rules.contains(&"IP-CIDR,8.8.4.4/32,PROXY,no-resolve"));
        assert!(rules.contains(&"IP-CIDR,1.1.1.1/32,PROXY,no-resolve"));
        assert!(!rules.contains(&"IP-CIDR,223.5.5.5/32,PROXY,no-resolve"));
        assert!(!rules.contains(&"IP-CIDR,114.114.114.114/32,PROXY,no-resolve"));

        // default-nameserver 按协议定义必须为明文 IP，不得改写
        let defaults: Vec<std::string::String> = dns
            .get("default-nameserver")
            .unwrap()
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            defaults,
            vec![
                std::string::String::from("223.6.6.6"),
                std::string::String::from("8.8.8.8"),
            ],
            "default-nameserver 保持明文不动"
        );

        // 无 dns 段 / 无 nameserver 时安全跳过
        let no_dns: Mapping = serde_yaml_ng::from_str("mixed-port: 7890").unwrap();
        let untouched = sanitize_dns_upstreams(no_dns);
        assert!(untouched.get("dns").is_none());
        let no_nameserver: Mapping = serde_yaml_ng::from_str("dns:\n  enable: true").unwrap();
        let untouched = sanitize_dns_upstreams(no_nameserver);
        assert!(untouched.get("dns").unwrap().get("nameserver").is_none());
    }

    #[test]
    fn dns_upstream_proxy_rules_skip_domestic_public_dns() {
        use super::{collect_dns_field_strings, dns_upstream_proxy_rules_from_hosts, extract_upstream_host};
        use serde_yaml_ng::Mapping;

        // C11：境内公共 DNS IP / DoH 域名即使误配在 fallback 中也不得生成 PROXY 规则
        let dns: Mapping = serde_yaml_ng::from_str(
            r"
fallback:
  - 223.5.5.5
  - 114.114.114.114
  - https://doh.pub/dns-query
  - https://dns.alidns.com/dns-query
  - https://1.1.1.1/dns-query
",
        )
        .unwrap();

        let hosts: Vec<std::string::String> = collect_dns_field_strings(&dns, "fallback")
            .iter()
            .filter_map(|raw| extract_upstream_host(raw))
            .collect();
        let rules: Vec<std::string::String> = dns_upstream_proxy_rules_from_hosts(hosts.iter().map(|s| s.as_str()));
        assert_eq!(
            rules,
            vec![
                std::string::String::from("IP-CIDR,1.1.1.1/32,PROXY,no-resolve"),
                std::string::String::from("IP-CIDR,1.0.0.1/32,PROXY,no-resolve"),
            ]
        );
    }

    #[test]
    fn extract_upstream_host_handles_all_mihomo_forms() {
        use super::extract_upstream_host;

        assert_eq!(extract_upstream_host("1.1.1.1").as_deref(), Some("1.1.1.1"));
        assert_eq!(extract_upstream_host("8.8.8.8:53").as_deref(), Some("8.8.8.8"));
        assert_eq!(
            extract_upstream_host("https://1.1.1.1/dns-query").as_deref(),
            Some("1.1.1.1")
        );
        assert_eq!(extract_upstream_host("tls://8.8.4.4:853").as_deref(), Some("8.8.4.4"));
        assert_eq!(
            extract_upstream_host("https://dns.google/dns-query#h3=true").as_deref(),
            Some("dns.google")
        );
        assert_eq!(
            extract_upstream_host("https://[2606:4700:4700::1111]/dns-query").as_deref(),
            Some("2606:4700:4700::1111")
        );
        // 裸 IPv6 不带方括号
        assert_eq!(
            extract_upstream_host("2606:4700:4700::1001").as_deref(),
            Some("2606:4700:4700::1001")
        );
        assert_eq!(extract_upstream_host("   ").as_deref(), None);
    }
}
