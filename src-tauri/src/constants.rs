use std::time::Duration;

pub mod network {
    pub const DEFAULT_EXTERNAL_CONTROLLER: &str = "127.0.0.1:9098";

    /// 远程机场订阅与默认 HTTP 请求的兼容 User-Agent 标识（BUG-362）。
    ///
    /// 【选型与核实依据】：
    /// - 对应上游官方仓库 `clash-verge-rev/clash-verge-rev` 的正式发行代际标签 `v2.5.10`
    ///   （2026-11 发布，内置同类 `Mihomo v1.19.x` 内核）。主流订阅转换/机场面板
    ///   （Subconverter / Xboard / V2Board / SSPanel 等）通过匹配 `clash-verge/v2.x`
    ///   来下发兼容 Clash.Meta/Mihomo 的 YAML 配置并开放同代协议族。
    /// - 权衡说明：相比动态生成客户端版本号或轮换随机 UA，固定单段兼容 UA 的代价是默认
    ///   请求头指纹可聚类；但能彻底规避 `clash-mini/3.x` 被面板误判为未知/不兼容分支或
    ///   超前下发未支持协议字段导致内核解析失败。需要差异化 UA 时可通过单订阅的
    ///   `option.user_agent` 覆盖。
    ///
    /// 【维护铁律（详见 `clash_mini_agreements.md` §7.6 / §8.2）】：
    /// 1. 必须与客户端自身 UI 发行版本号（`CARGO_PKG_VERSION`）彻底解耦；
    /// 2. 固定采用与当前内置 `mini-mihomo`（`Mihomo v1.19.x`）同协议代际的上游官方正式版纯净单段标识；
    /// 3. 日常 UI 修复与发版升号一律不改动此常量，仅在升级替换内置内核大版本且引入新协议代际时同步审视更新。
    pub const SUBSCRIPTION_COMPAT_USER_AGENT: &str = "clash-verge/v2.5.10";

    pub mod ports {
        pub const DEFAULT_MIXED: u16 = 10801;
        pub const DEFAULT_SOCKS: u16 = 10802;
        pub const DEFAULT_HTTP: u16 = 10803;

        #[cfg(not(feature = "verge-dev"))]
        pub const SINGLETON_SERVER: u16 = 33335;
        #[cfg(feature = "verge-dev")]
        pub const SINGLETON_SERVER: u16 = 33336;
    }
}

pub mod timing {
    use super::Duration;

    pub const CONFIG_UPDATE_DEBOUNCE: Duration = Duration::from_millis(300);
    pub const STARTUP_ERROR_DELAY: Duration = Duration::from_secs(2);

    /// 服务就绪等待上限。全新安装场景下「安装器收尾 → SCM 拉起服务 → 服务初始化建 IPC 管道」
    /// 全链路实测可超过 10 秒（Windows 服务启动经 SCM 调度 + 杀软扫描新注册的服务二进制），
    /// 旧的 3 秒窗口会导致首次安装后误判失败（BUG-298）。就绪时 connect 立即返回，多等无副作用。
    pub const SERVICE_WAIT_MAX: Duration = Duration::from_millis(30000);
    pub const SERVICE_WAIT_INTERVAL: Duration = Duration::from_millis(200);

    /// 代理节点判死与探针超时最大延迟限值。
    /// 与 src-tauri/src/module/monitor.rs 的 NODE_DELAY_MAX_MS 同语义，必须保持一致
    /// （BUG-304：2000 → 5000，覆盖拥塞期「慢而可用」节点）。
    pub const NODE_DELAY_MAX_MS: u32 = 5000;

    /// 内部控制、Local Socket 通信与 JS 合并脚本执行的最大熔断时间 (ms)
    pub const INTERNAL_CONTROL_TIMEOUT_MS: u64 = 3000;

    /// 内核配置文件语法验证的最长超时限值 (ms)，给大规则集、GeoIP/MRS文件解析预留充裕时间
    pub const VALIDATE_CONTROL_TIMEOUT_MS: u64 = 10000;

    /// 启动脚本执行的最长超时时限 (ms)。超时后强行终止脚本进程，防止脚本挂起导致初始化永久等待
    pub const STARTUP_SCRIPT_TIMEOUT_MS: u64 = 30000;
}

pub mod files {
    pub const RUNTIME_CONFIG: &str = "clash-mini.yaml";
    pub const CHECK_CONFIG: &str = "clash-mini-check.yaml";
    pub const WINDOW_STATE: &str = "window_state.json";
}

pub mod tun {
    // 对齐上游默认值，改用用户态 gvisor 栈以保证跨平台兼容性。
    // Mini 无 TUN 设置 UI，若默认 system 栈在个别环境异常，用户将无处回退。
    pub const DEFAULT_STACK: &str = "gvisor";

    /// DNS 劫持规则。mihomo 语义：不书写协议时默认按 `udp://` 处理，
    /// 故必须显式补上 `tcp://any:53` —— 否则走 TCP 的 DNS 查询（大响应、部分解析器）
    /// 不会被劫持进内核 DNS 模块，而是按规则转发，形成 DNS 泄漏面。
    pub const DNS_HIJACK: &[&str] = &["any:53", "tcp://any:53"];

    /// TUN 网卡的 IPv6 地址（ULA 段，mihomo 标准默认值）。
    /// 【作用】让 TUN 具备 IPv6 收包能力，从而把 IPv6 默认路由一并导入 TUN，
    /// 防止 IPv6 流量绕过代理从物理网卡直连泄漏（实测：不设该项时 IPv6 默认路由
    /// 仍指向物理网关，所有 IPv6 流量 100% 绕过代理）。
    /// 【前提】mihomo 官方文档明确该字段「需要同时将顶层配置中的 ipv6 设置为 true 才会生效」，
    /// 故 use_tun 会在 TUN 启用时强制顶层 ipv6=true（是否「使用」IPv6 另由 dns.ipv6 控制）。
    pub const TUN_INET6_ADDRESS: &str = "fdfe:dcba:9876::1/126";

    /// fake-ip 模式下的默认免分配假 IP 过滤名单，防止 Windows NCSI 离线感叹号、局域网访问和 NTP 授时失效
    pub const DEFAULT_FAKE_IP_FILTER: &[&str] = &[
        "dns.msftncsi.com",
        "+.msftconnecttest.com",
        "+.msftncsi.com",
        "*.lan",
        "*.local",
        "localhost.ptlogin2.qq.com",
        "time.*.com",
        "time.*.gov",
        "time.*.edu.cn",
        "time.*.apple.com",
        "time1.*.com",
        "time2.*.com",
        "time3.*.com",
        "time4.*.com",
        "time5.*.com",
        "time6.*.com",
        "time7.*.com",
        "ntp.*.com",
        "*.time.edu.cn",
        "*.ntp.org.cn",
        "+.pool.ntp.org",
    ];
}
