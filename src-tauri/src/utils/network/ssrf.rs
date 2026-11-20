use anyhow::{Result, bail};
use tauri::Url;

/// 校验单个 IP 地址是否属于禁止访问的回环 / 私网 / 链路本地 / 唯一本地 / IPv4 映射私网地址（BUG-327）
pub(crate) fn check_disallowed_ssrf_ip(ip: std::net::IpAddr) -> Result<()> {
    let check_v4 = |ipv4: std::net::Ipv4Addr| -> Result<()> {
        if ipv4.is_loopback() || ipv4.is_private() || ipv4.is_unspecified() || ipv4.is_link_local() {
            bail!("cannot fetch subscription from private/loopback IP");
        }
        Ok(())
    };

    match ip {
        std::net::IpAddr::V4(ipv4) => check_v4(ipv4),
        std::net::IpAddr::V6(ipv6) => {
            if let Some(mapped_v4) = ipv6.to_ipv4_mapped() {
                check_v4(mapped_v4)?;
            }
            if ipv6.is_loopback() || ipv6.is_unspecified() {
                bail!("cannot fetch subscription from loopback IP");
            }
            let segments = ipv6.segments();
            // 链路本地地址 (fe80::/10) 与唯一本地地址 ULA (fc00::/7)
            if (segments[0] & 0xffc0) == 0xfe80 || (segments[0] & 0xfe00) == 0xfc00 {
                bail!("cannot fetch subscription from local/link-local IP");
            }
            Ok(())
        }
    }
}

/// SSRF 防护：禁止访问内网/回环地址（入口静态校验 + 本地预解析拦截，配合 `NetworkManager::get_with_ssrf_guard`
/// 的连接期 `SsrfSafeDnsResolver` 与 30x 重定向逐跳复验形成多层防线，BUG-327）
pub(crate) fn validate_url_no_ssrf(url: &Url) -> Result<()> {
    if let Some(host) = url.host() {
        // 1. 静态字符串与直接 IP 校验
        match host {
            url::Host::Domain(d) => {
                let lower = d.trim_end_matches('.').to_ascii_lowercase();
                if lower.is_empty()
                    || !lower.contains('.')
                    || lower == "localhost"
                    || lower.ends_with(".localhost")
                    || lower.ends_with(".local")
                    || lower.ends_with(".internal")
                    || lower.ends_with(".lan")
                    || lower.ends_with(".home.arpa")
                    || lower == "127.0.0.1"
                    || lower == "::1"
                    || lower == "0.0.0.0"
                    || lower.starts_with("169.254.")
                {
                    bail!("cannot fetch subscription from localhost/internal address");
                }
            }
            url::Host::Ipv4(ip) => {
                check_disallowed_ssrf_ip(std::net::IpAddr::V4(ip))?;
            }
            url::Host::Ipv6(ip) => {
                check_disallowed_ssrf_ip(std::net::IpAddr::V6(ip))?;
            }
        }

        // 2. 动态 DNS 预解析校验，拦截解析到回环/私网/ULA/IPv4-mapped 私网的域名。
        // 【多层防线与已知边界说明（BUG-327）】：
        // - 本地 `ToSocketAddrs` 在国内 ISP DNS 下对被污染/NXDOMAIN 的正规机场订阅域名会返回 Err，
        //   此处不直接拒绝，以便 `perform_profile_update` 后续回退至 `127.0.0.1:10801` 本地代理通道拉取；
        // - 直连请求（`ProxyType::None`）由 `NetworkManager` 挂载连接期 `SsrfSafeDnsResolver` 消除 DNS Rebinding TOCTOU；
        // - 直连与代理请求均挂载 `redirect::Policy::custom` 对每一跳 30x 重定向目标再次执行 `validate_url_no_ssrf`；
        // - 已知架构边界：当请求经 HTTP 代理隧道（`ProxyType::Localhost` / `ProxyType::System`）交由远端节点解析域名且未发生 HTTP 重定向时，
        //   客户端套接字层不可见远端 DNS 解析出的最终 IP，该场景依赖代理内核及出站节点侧策略隔离。
        use std::net::ToSocketAddrs as _;
        let host_str = match host {
            url::Host::Domain(d) => d.to_string(),
            url::Host::Ipv4(ip) => ip.to_string(),
            url::Host::Ipv6(ip) => ip.to_string(),
        };
        let port = url.port().unwrap_or(80);
        if let Ok(addrs) = (host_str.as_str(), port).to_socket_addrs() {
            for addr in addrs {
                check_disallowed_ssrf_ip(addr.ip())?;
            }
        }
    }
    Ok(())
}

/// 连接期异步 DNS 校验器（BUG-327）：在直连建连阶段实时解析并校验全部候选 IP，
/// 消除预检 `ToSocketAddrs` 与真实建连之间的 DNS Rebinding TOCTOU 窗口。
pub(super) struct SsrfSafeDnsResolver;

impl reqwest::dns::Resolve for SsrfSafeDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let resolved: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            for addr in &resolved {
                check_disallowed_ssrf_ip(addr.ip())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string()))?;
            }
            let iter: reqwest::dns::Addrs = Box::new(resolved.into_iter());
            Ok(iter)
        })
    }
}
