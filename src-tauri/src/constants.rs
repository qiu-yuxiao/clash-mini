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
    pub const DNS_CONFIG: &str = "dns_config.yaml";
    pub const WINDOW_STATE: &str = "window_state.json";
}

pub mod tun {
    pub const DEFAULT_STACK: &str = "system";

    pub const DNS_HIJACK: &[&str] = &["any:53"];
}
