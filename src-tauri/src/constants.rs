use std::time::Duration;

pub mod network {
    pub const DEFAULT_EXTERNAL_CONTROLLER: &str = "127.0.0.1:9098";

    pub mod ports {
        #[cfg(not(target_os = "windows"))]
        pub const DEFAULT_REDIR: u16 = 7895;
        #[cfg(target_os = "linux")]
        pub const DEFAULT_TPROXY: u16 = 7896;
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

    #[cfg(target_os = "windows")]
    pub const SERVICE_WAIT_MAX: Duration = Duration::from_millis(3000);
    #[cfg(target_os = "windows")]
    pub const SERVICE_WAIT_INTERVAL: Duration = Duration::from_millis(200);

    /// 代理节点判死与探针超时最大延迟限值
    pub const NODE_DELAY_MAX_MS: u32 = 2000;

    /// 内部控制、Local Socket 通信与 JS 合并脚本执行的最大熔断时间 (ms)
    pub const INTERNAL_CONTROL_TIMEOUT_MS: u64 = 3000;

    /// 内核配置文件语法验证的最长超时限值 (ms)，给大规则集、GeoIP/MRS文件解析预留充裕时间
    pub const VALIDATE_CONTROL_TIMEOUT_MS: u64 = 10000;
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

    pub const DNS_HIJACK: &[&str] = &["any:53"];

    /// fake-ip 模式下的默认免分配假 IP 过滤名单，防止 Windows NCSI 离线感叹号、局域网访问和 NTP 授时失效
    pub const DEFAULT_FAKE_IP_FILTER: &[&str] = &[
        "dns.msftncsi.com",
        "*.msftconnecttest.com",
        "*.msftncsi.com",
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

