use std::{collections::HashMap, fmt::Display};

use futures_util::{SinkExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpStream, sync::RwLock};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use ts_rs::TS;

use crate::ipc::WrapStream;

macro_rules! string_enum {
    (
        $(#[$enum_meta:meta])*
        pub enum $name:ident {
            $first_variant:ident => $first_value:literal,
            $($variant:ident => $value:literal,)*
        }
    ) => {
        $(#[$enum_meta])*
        pub enum $name {
            $first_variant,
            $($variant,)*
            Unknown(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    Self::$first_variant => $first_value,
                    $(Self::$variant => $value,)*
                    Self::Unknown(value) => value,
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Ok(match value.as_str() {
                    $first_value => Self::$first_variant,
                    $($value => Self::$variant,)*
                    _ => Self::Unknown(value),
                })
            }
        }

    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    LocalSocket,
}

impl Display for Protocol {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http => write!(f, "http"),
            Protocol::LocalSocket => {
                if cfg!(windows) {
                    write!(f, "named pipe")
                } else {
                    write!(f, "unix socket")
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all(serialize = "camelCase", deserialize = "kebab-case"))]
pub struct BaseConfig {
    pub port: u16,
    pub socks_port: u16,
    pub redir_port: u16,
    pub tproxy_port: u16,
    pub mixed_port: u16,
    pub tun: TunConfig,
    pub tuic_server: TuicServer,
    pub ss_config: String,
    pub vmess_config: String,
    pub authentication: Option<Vec<String>>,
    pub skip_auth_prefixes: Option<Vec<String>>,
    pub lan_allowed_ips: Option<Vec<String>>,
    pub lan_disallowed_ips: Option<Vec<String>>,
    pub allow_lan: bool,
    pub bind_address: String,
    pub inbound_tfo: bool,
    pub inbound_mptcp: bool,
    pub mode: ClashMode,
    pub unified_delay: bool,
    pub log_level: LogLevel,
    pub ipv6: bool,
    pub interface_name: String,
    pub routing_mark: isize,
    pub geox_url: GeoXUrl,
    pub geo_auto_update: bool,
    pub geo_update_interval: isize,
    pub geodata_mode: bool,
    pub geodata_loader: String,
    pub geosite_matcher: String,
    pub tcp_concurrent: bool,
    pub find_process_mode: FindProcessMode,
    pub sniffing: bool,
    pub global_ua: String,
    pub etag_support: bool,
    pub keep_alive_interval: isize,
    pub keep_alive_idle: isize,
    pub disable_keep_alive: bool,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub unknown_fields: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all(serialize = "camelCase", deserialize = "kebab-case"))]
pub struct TunConfig {
    pub enable: bool,
    pub device: String,
    pub stack: TunStack,
    pub dns_hijack: Vec<String>,
    pub auto_route: bool,
    pub auto_detect_interface: bool,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mtu: Option<u32>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub gso: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub gso_max_size: Option<u32>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet4_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet6_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub iproute2_table_index: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub iproute2_rule_index: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auto_redirect: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auto_redirect_input_mark: Option<u32>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auto_redirect_output_mark: Option<u32>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auto_redirect_iproute2_fallback_rule_index: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub loopback_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub strict_route: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub route_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub route_address_set: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub route_exclude_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub route_exclude_address_set: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_interface: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_interface: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_uid: Option<Vec<u32>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_uid_range: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_uid: Option<Vec<u32>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_uid_range: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_src_port: Option<Vec<u16>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_src_port_range: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_dst_port: Option<Vec<u16>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_dst_port_range: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_android_user: Option<Vec<isize>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_package: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_package: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_mac_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclude_mac_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoint_independent_nat: Option<bool>,

    #[ts(optional)]
    #[ts(type = "number")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub udp_timeout: Option<i64>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub disable_icmp_forwarding: Option<bool>,

    pub file_descriptor: isize,

    // The following `inet*` fields will be deprecated
    // refer: https://wiki.metacubex.one/config/inbound/tun/#_1
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet4_route_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet6_route_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet4_route_exclude_address: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inet6_route_exclude_address: Option<Vec<String>>,

    // darwin special config
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub recvmsgx: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sendmsgx: Option<bool>,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub unknown_fields: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all(serialize = "camelCase", deserialize = "kebab-case"))]
pub struct TuicServer {
    pub enable: bool,
    pub listen: String,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub users: Option<HashMap<String, String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub client_auth_type: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub client_auth_cert: Option<String>,

    pub certificate: String,
    pub private_key: String,
    pub ech_key: String,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub congestion_controller: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_idle_time: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication_timeout: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alpn: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_udp_relay_packet_size: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_datagram_frame_size: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cwnd: Option<isize>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bbr_profile: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mux_option: Option<MuxOption>,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct MuxOption {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub padding: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub brutal: Option<BrutalOption>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct BrutalOption {
    pub enabled: bool,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub up: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub down: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARNING,
    ERROR,
    SILENT,
}

impl Display for LogLevel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::DEBUG => write!(f, "debug"),
            LogLevel::INFO => write!(f, "info"),
            LogLevel::WARNING => write!(f, "warning"),
            LogLevel::ERROR => write!(f, "error"),
            LogLevel::SILENT => write!(f, "silent"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all(serialize = "camelCase", deserialize = "kebab-case"))]
pub struct GeoXUrl {
    pub geo_ip: String,
    pub mmdb: String,
    pub asn: String,
    pub geo_site: String,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum FindProcessMode {
    Strict,
    Always,
    Off,
}

/// mihomo version
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct MihomoVersion {
    pub meta: bool,
    pub version: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum CoreUpdaterChannel {
    #[serde(rename = "release")]
    ReleaseChannel,
    #[serde(rename = "alpha")]
    AlphaChannel,
    #[serde(rename = "auto")]
    Auto,
}

impl Display for CoreUpdaterChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreUpdaterChannel::ReleaseChannel => write!(f, "release"),
            CoreUpdaterChannel::AlphaChannel => write!(f, "alpha"),
            CoreUpdaterChannel::Auto => write!(f, "auto"),
        }
    }
}

/// clash mode enum
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ClashMode {
    Rule,
    Global,
    Direct,
}

impl Display for ClashMode {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClashMode::Rule => write!(f, "rule"),
            ClashMode::Global => write!(f, "global"),
            ClashMode::Direct => write!(f, "direct"),
        }
    }
}

/// tun stack enum
#[derive(Debug, TS, PartialEq, Eq)]
#[ts(export)]
pub enum TunStack {
    #[ts(rename = "Mixed")]
    Mixed,
    #[ts(rename = "gVisor")]
    Gvisor,
    #[ts(rename = "System")]
    System,
}

impl Serialize for TunStack {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let s = match self {
            TunStack::Mixed => "Mixed",
            TunStack::Gvisor => "gVisor",
            TunStack::System => "System",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for TunStack {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "Mixed" => Ok(Self::Mixed),
            "gVisor" => Ok(Self::Gvisor),
            "System" => Ok(Self::System),
            _ => Err(serde::de::Error::custom("invalid tun stack")),
        }
    }
}

impl Display for TunStack {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunStack::Mixed => write!(f, "Mixed"),
            TunStack::Gvisor => write!(f, "gVisor"),
            TunStack::System => write!(f, "System"),
        }
    }
}

/// group proxies
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Groups {
    pub proxies: Vec<Proxy>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Proxy {
    // group type need
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub all: Option<Vec<String>>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expected_status: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fixed: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hidden: Option<bool>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub icon: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub now: Option<String>,

    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub test_url: Option<String>,

    // single proxy type need
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,

    // basic fields
    pub alive: bool,
    pub history: Vec<DelayHistory>,
    pub extra: HashMap<String, Extra>,
    pub name: String,
    pub udp: bool,
    pub uot: bool,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub xudp: bool,
    pub tfo: bool,
    pub mptcp: bool,
    pub smux: bool,
    pub interface: String,

    #[serde(rename(serialize = "dialerProxy", deserialize = "dialer-proxy"))]
    pub dialer_proxy: String,

    #[serde(rename(serialize = "routingMark", deserialize = "routing-mark"))]
    pub routing_mark: i32,

    #[serde(rename(serialize = "providerName", deserialize = "provider-name"))]
    pub provider_name: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub unknown_fields: HashMap<String, Value>,
}

string_enum! {
    #[derive(Debug, TS, PartialEq, Eq)]
    #[ts(export)]
    #[ts(type = "string")]
    pub enum ProxyType {
        Direct => "Direct",
        Reject => "Reject",
        RejectDrop => "RejectDrop",
        Compatible => "Compatible",
        Pass => "Pass",
        PassRule => "PassRule",
        Dns => "Dns",
        Shadowsocks => "Shadowsocks",
        ShadowsocksR => "ShadowsocksR",
        Snell => "Snell",
        Socks5 => "Socks5",
        Http => "Http",
        Vmess => "Vmess",
        Vless => "Vless",
        Trojan => "Trojan",
        Hysteria => "Hysteria",
        Hysteria2 => "Hysteria2",
        WireGuard => "WireGuard",
        Tuic => "Tuic",
        Ssh => "Ssh",
        Mieru => "Mieru",
        Masque => "Masque",
        AnyTLS => "AnyTLS",
        Relay => "Relay",
        Sudoku => "Sudoku",
        TrustTunnel => "TrustTunnel",
        OpenVPN => "OpenVPN",
        Tailscale => "Tailscale",
        GostRelay => "GostRelay",
        Selector => "Selector",
        Fallback => "Fallback",
        URLTest => "URLTest",
        LoadBalance => "LoadBalance",
    }
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Extra {
    pub alive: bool,
    pub history: Vec<DelayHistory>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct DelayHistory {
    pub time: String,
    pub delay: u16,
}

/// proxies
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Proxies {
    pub proxies: HashMap<String, Proxy>,
}

/// proxy delay result
///
/// displays a message if it times out, otherwise it only displays the delay
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct ProxyDelay {
    pub delay: u32,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct ProxyProviders {
    pub providers: HashMap<String, ProxyProvider>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum ProviderType {
    Proxy,
    Rule,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum VehicleType {
    File,
    HTTP,
    Compatible,
    Inline,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProxyProvider {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    pub vehicle_type: VehicleType,
    pub proxies: Vec<Proxy>,
    pub test_url: String,
    pub expected_status: String,
    pub updated_at: Option<String>,
    pub subscription_info: Option<SubScriptionInfo>,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
#[serde(rename_all = "PascalCase")]
pub struct SubScriptionInfo {
    #[ts(type = "number")]
    pub upload: i64,
    #[ts(type = "number")]
    pub download: i64,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub expire: i64,
}

/// rules
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub struct Rule {
    #[serde(rename = "type")]
    pub rule_type: RuleType,
    pub index: i32,
    pub payload: String,
    pub proxy: String,
    pub size: i32,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

string_enum! {
    #[derive(Debug, TS, PartialEq, Eq)]
    #[ts(export)]
    #[ts(type = "string")]
    pub enum RuleType {
        Domain => "Domain",
        DomainSuffix => "DomainSuffix",
        DomainKeyword => "DomainKeyword",
        DomainRegex => "DomainRegex",
        DomainWildcard => "DomainWildcard",
        GeoSite => "GeoSite",
        GeoIP => "GeoIP",
        SrcGeoIP => "SrcGeoIP",
        IPASN => "IPASN",
        SrcIPASN => "SrcIPASN",
        IPCIDR => "IPCIDR",
        SrcIPCIDR => "SrcIPCIDR",
        IPSuffix => "IPSuffix",
        SrcIPSuffix => "SrcIPSuffix",
        SrcPort => "SrcPort",
        DstPort => "DstPort",
        InPort => "InPort",
        DSCP => "DSCP",
        InUser => "InUser",
        InName => "InName",
        InType => "InType",
        ProcessName => "ProcessName",
        ProcessPath => "ProcessPath",
        ProcessNameRegex => "ProcessNameRegex",
        ProcessPathRegex => "ProcessPathRegex",
        ProcessNameWildcard => "ProcessNameWildcard",
        ProcessPathWildcard => "ProcessPathWildcard",
        Match => "Match",
        RuleSet => "RuleSet",
        Network => "Network",
        Uid => "Uid",
        SubRules => "SubRules",
        AND => "AND",
        OR => "OR",
        NOT => "NOT",
    }
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct RuleProviders {
    pub providers: HashMap<String, RuleProvider>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum RuleBehavior {
    Domain,
    #[serde(rename = "IPCIDR")]
    IpCidr,
    Classical,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum RuleFormat {
    #[serde(rename = "YamlRule")]
    Yaml,
    #[serde(rename = "TextRule")]
    Text,
    #[serde(rename = "MrsRule")]
    Mrs,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RuleProvider {
    pub behavior: RuleBehavior,
    pub format: RuleFormat,
    pub name: String,
    pub rule_count: u32,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    pub updated_at: String,
    pub vehicle_type: VehicleType,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// connections
#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Connections {
    #[ts(type = "number")]
    pub download_total: u64,
    #[ts(type = "number")]
    pub upload_total: u64,
    pub connections: Option<Vec<Connection>>,
    pub memory: u64,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub metadata: ConnectionMetaData,
    #[ts(type = "number")]
    pub upload: u64,
    #[ts(type = "number")]
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
    #[serde(default)]
    pub provider_chains: Option<Vec<String>>,
    pub rule: String,
    pub rule_payload: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

string_enum! {
    #[derive(Debug, TS, PartialEq, Eq)]
    #[ts(export)]
    #[ts(type = "string")]
    pub enum Network {
        TCP => "tcp",
        UDP => "udp",
        ALLNet => "all",
    }
}

string_enum! {
    #[derive(Debug, TS, PartialEq, Eq)]
    #[ts(export)]
    #[ts(type = "string")]
    pub enum ConnectionType {
        HTTP => "HTTP",
        HTTPS => "HTTPS",
        SOCKS4 => "Socks4",
        SOCKS5 => "Socks5",
        SHADOWSOCKS => "ShadowSocks",
        SNELL => "Snell",
        VMESS => "Vmess",
        VLESS => "Vless",
        REDIR => "Redir",
        TPROXY => "TProxy",
        TROJAN => "Trojan",
        TUNNEL => "Tunnel",
        TUN => "Tun",
        TUIC => "Tuic",
        HYSTERIA2 => "Hysteria2",
        ANYTLS => "AnyTLS",
        MIERU => "Mieru",
        SUDOKU => "Sudoku",
        TRUSTTUNNEL => "TrustTunnel",
        INNER => "Inner",
    }
}

string_enum! {
    #[derive(Debug, TS, PartialEq, Eq)]
    #[ts(export)]
    #[ts(type = "string")]
    pub enum DNSMode {
        Normal => "normal",
        FakeIP => "fake-ip",
        Mapping => "redir-host",
        Hosts => "hosts",
    }
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetaData {
    pub network: Network,

    #[serde(rename = "type")]
    pub connection_type: ConnectionType,

    #[serde(rename = "sourceIP")]
    pub source_ip: String,

    #[serde(rename = "destinationIP")]
    pub destination_ip: String,

    #[serde(rename = "sourceGeoIP")]
    pub source_geo_ip: Option<Vec<String>>,

    #[serde(rename = "destinationGeoIP")]
    pub destination_geo_ip: Option<Vec<String>>,

    #[serde(rename = "sourceIPASN")]
    pub source_ip_asn: String,

    #[serde(rename = "destinationIPASN")]
    pub destination_ip_asn: String,

    pub source_port: String,
    pub destination_port: String,

    #[serde(rename = "inboundIP")]
    pub inbound_ip: String,

    pub inbound_port: String,
    pub inbound_name: String,
    pub inbound_user: String,
    pub host: String,
    pub dns_mode: DNSMode,
    pub uid: u32,
    pub process: String,
    pub process_path: String,
    pub special_proxy: String,
    pub special_rules: String,
    pub remote_destination: String,
    pub dscp: u8,
    pub sniff_host: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Traffic {
    #[ts(type = "number")]
    pub up: u64,
    #[ts(type = "number")]
    pub down: u64,
    #[serde(rename = "upTotal", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub up_total: Option<u64>,
    #[serde(rename = "downTotal", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub down_total: Option<u64>,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Memory {
    pub inuse: u64,
    pub oslimit: u64,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(default)]
#[ts(export)]
pub struct Log {
    #[serde(rename = "type")]
    pub log_type: String,
    pub payload: String,

    #[ts(skip)]
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

// ------------- use in rust, no need export to typescript -----------------
#[derive(Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ErrorResponse {
    pub message: String,
}

pub type ConnectionId = u32;
pub enum WebSocketWriter {
    TcpStreamWriter(SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>),
    SocketStreamWriter(SplitSink<WebSocketStream<WrapStream>, Message>),
}

impl WebSocketWriter {
    pub async fn send(&mut self, message: Message) -> crate::Result<()> {
        match self {
            WebSocketWriter::TcpStreamWriter(write) => {
                write.send(message).await?;
            }
            WebSocketWriter::SocketStreamWriter(write) => {
                write.send(message).await?;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ConnectionManager(pub RwLock<HashMap<ConnectionId, WebSocketWriter>>);
