use crate::config::Config;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use reqwest::{
    Client, Proxy, StatusCode,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use smartstring::alias::String;
use std::{sync::Arc, time::Duration};
use sysproxy::Sysproxy;
use tauri::Url;

#[derive(Debug)]
pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl HttpResponse {
    pub const fn new(status: StatusCode, headers: HeaderMap, body: String) -> Self {
        Self { status, headers, body }
    }

    pub const fn status(&self) -> StatusCode {
        self.status
    }

    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn text_with_charset(&self) -> Result<&str> {
        Ok(&self.body)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProxyType {
    None,
    Localhost,
    System,
}

#[derive(Debug, Clone, Copy)]
enum TlsRootMode {
    PlatformVerifier,
    StaticWebpkiRoots,
}

/// 连接期异步 DNS 校验器（BUG-327）：在直连建连阶段实时解析并校验全部候选 IP，
/// 消除预检 `ToSocketAddrs` 与真实建连之间的 DNS Rebinding TOCTOU 窗口。
struct SsrfSafeDnsResolver;

impl reqwest::dns::Resolve for SsrfSafeDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let resolved: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            for addr in &resolved {
                crate::config::check_disallowed_ssrf_ip(addr.ip())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string()))?;
            }
            let iter: reqwest::dns::Addrs = Box::new(resolved.into_iter());
            Ok(iter)
        })
    }
}

pub struct NetworkManager;

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkManager {
    pub const fn new() -> Self {
        Self
    }

    fn build_client(
        &self,
        proxy_url: Option<std::string::String>,
        default_headers: HeaderMap,
        accept_invalid_certs: bool,
        timeout_secs: Option<u64>,
        tls_root_mode: TlsRootMode,
        ssrf_guard: bool,
    ) -> Result<Client> {
        let redirect_policy = if ssrf_guard {
            reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.error("too many redirects");
                }
                if let Err(err) = crate::config::validate_url_no_ssrf(attempt.url()) {
                    return attempt.error(err.to_string());
                }
                attempt.follow()
            })
        } else {
            reqwest::redirect::Policy::limited(10)
        };

        let mut builder = Client::builder()
            .tls_backend_rustls()
            .redirect(redirect_policy)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(Some(Duration::from_secs(15)));

        if matches!(tls_root_mode, TlsRootMode::StaticWebpkiRoots) {
            builder = builder.tls_backend_preconfigured(Self::static_webpki_tls_config()?);
        }

        // 设置代理（直连且启用 SSRF 守卫时挂载连接期 SsrfSafeDnsResolver，消除 DNS 重绑定窗口）
        if let Some(proxy_str) = proxy_url {
            let proxy = Proxy::all(proxy_str)?;
            builder = builder.proxy(proxy);
        } else {
            builder = builder.no_proxy();
            if ssrf_guard {
                builder = builder.dns_resolver(Arc::new(SsrfSafeDnsResolver));
            }
        }

        builder = builder.default_headers(default_headers);

        // SSL/TLS
        if accept_invalid_certs {
            builder = builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        // 超时设置
        if let Some(secs) = timeout_secs {
            builder = builder
                .timeout(Duration::from_secs(secs))
                .connect_timeout(Duration::from_secs(secs.min(10)));
        }

        Ok(builder.build()?)
    }

    fn static_webpki_tls_config() -> Result<rustls::ClientConfig> {
        static CACHE: std::sync::OnceLock<rustls::ClientConfig> = std::sync::OnceLock::new();
        if let Some(config) = CACHE.get() {
            return Ok(config.clone());
        }
        let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config =
            rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()?
                .with_root_certificates(root_store)
                .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let _ = CACHE.set(config.clone());
        Ok(config)
    }

    fn should_retry_with_static_webpki_roots(err: &anyhow::Error) -> bool {
        err.chain().any(|e| {
            let msg = e.to_string().to_ascii_lowercase();
            [
                "certificate",
                "cert",
                "tls",
                "ssl",
                "rustls",
                "webpki",
                "revocation",
                "ocsp",
                "crl",
                "issuer",
                "unknownissuer",
            ]
            .iter()
            .any(|kw| msg.contains(kw))
        })
    }

    pub async fn create_request(
        &self,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
    ) -> Result<Client> {
        self.create_request_with_tls_mode(
            proxy_type,
            timeout_secs,
            user_agent,
            accept_invalid_certs,
            TlsRootMode::PlatformVerifier,
            false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn get_with_tls_mode(
        &self,
        url: &str,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
        tls_root_mode: TlsRootMode,
        ssrf_guard: bool,
    ) -> Result<HttpResponse> {
        let mut parsed = Url::parse(url)?;
        let mut extra_headers = HeaderMap::new();

        if !parsed.username().is_empty()
            && let Some(pass) = parsed.password()
        {
            let username = percent_encoding::percent_decode_str(parsed.username())
                .decode_utf8_lossy()
                .into_owned();
            let password = percent_encoding::percent_decode_str(pass)
                .decode_utf8_lossy()
                .into_owned();
            let auth_str = format!("{}:{}", username, password);
            let encoded = general_purpose::STANDARD.encode(auth_str);
            extra_headers.insert("Authorization", HeaderValue::from_str(&format!("Basic {}", encoded))?);
        }

        parsed.set_username("").ok();
        parsed.set_password(None).ok();

        // 创建请求
        let client = self
            .create_request_with_tls_mode(
                proxy_type,
                timeout_secs,
                user_agent,
                accept_invalid_certs,
                tls_root_mode,
                ssrf_guard,
            )
            .await?;

        let mut request_builder = client.get(parsed);

        for (key, value) in extra_headers.iter() {
            request_builder = request_builder.header(key, value);
        }

        let response = match request_builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(anyhow::Error::new(e).context("Request failed"));
            }
        };

        let status = response.status();
        let headers = response.headers().to_owned();

        // Limit download size to 10MB to avoid OOM crashes
        const MAX_DOWNLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB
        let mut body_bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| anyhow::anyhow!("Stream read error: {}", e))?
        {
            if body_bytes.len() + chunk.len() > MAX_DOWNLOAD_SIZE {
                anyhow::bail!("Subscription file size exceeds the 10MB limit");
            }
            body_bytes.extend_from_slice(&chunk);
        }
        let body = std::string::String::from_utf8(body_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode response as UTF-8: {}", e))?
            .into();

        Ok(HttpResponse::new(status, headers, body))
    }

    async fn create_request_with_tls_mode(
        &self,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
        tls_root_mode: TlsRootMode,
        ssrf_guard: bool,
    ) -> Result<Client> {
        let proxy_url: Option<std::string::String> = match proxy_type {
            ProxyType::None => None,
            ProxyType::Localhost => {
                let port = {
                    let verge_port = Config::verge().await.data_arc().verge_mixed_port;
                    match verge_port {
                        Some(port) => port,
                        None => Config::clash().await.data_arc().get_mixed_port(),
                    }
                };
                Some(format!("http://127.0.0.1:{port}"))
            }
            ProxyType::System => {
                if let Ok(p @ Sysproxy { enable: true, .. }) = Sysproxy::get_system_proxy() {
                    Some(format!("http://{}:{}", p.host, p.port))
                } else {
                    None
                }
            }
        };

        let mut headers = HeaderMap::new();

        // 设置 User-Agent：单订阅非空自定义值优先，否则恒定使用与 UI 版本号解耦的订阅兼容标识（BUG-362）
        let effective_ua = resolve_effective_user_agent(user_agent.as_deref());
        headers.insert(USER_AGENT, HeaderValue::from_str(effective_ua)?);

        self.build_client(
            proxy_url,
            headers,
            accept_invalid_certs,
            timeout_secs,
            tls_root_mode,
            ssrf_guard,
        )
    }

    pub async fn get_with_interrupt(
        &self,
        url: &str,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
    ) -> Result<HttpResponse> {
        self.get_internal(url, proxy_type, timeout_secs, user_agent, accept_invalid_certs, false)
            .await
    }

    pub async fn get_with_ssrf_guard(
        &self,
        url: &str,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
    ) -> Result<HttpResponse> {
        self.get_internal(url, proxy_type, timeout_secs, user_agent, accept_invalid_certs, true)
            .await
    }

    async fn get_internal(
        &self,
        url: &str,
        proxy_type: ProxyType,
        timeout_secs: Option<u64>,
        user_agent: Option<String>,
        accept_invalid_certs: bool,
        ssrf_guard: bool,
    ) -> Result<HttpResponse> {
        let platform_result = self
            .get_with_tls_mode(
                url,
                proxy_type,
                timeout_secs,
                user_agent.clone(),
                accept_invalid_certs,
                TlsRootMode::PlatformVerifier,
                ssrf_guard,
            )
            .await;

        match platform_result {
            Ok(response) => Ok(response),
            Err(err) if !accept_invalid_certs && Self::should_retry_with_static_webpki_roots(&err) => self
                .get_with_tls_mode(
                    url,
                    proxy_type,
                    timeout_secs,
                    user_agent,
                    accept_invalid_certs,
                    TlsRootMode::StaticWebpkiRoots,
                    ssrf_guard,
                )
                .await
                .map_err(|fallback_err| {
                    anyhow::anyhow!(
                        "platform TLS verifier failed: {err}; static webpki roots fallback failed: {fallback_err}"
                    )
                }),
            Err(err) => Err(err),
        }
    }
}

pub(crate) fn resolve_effective_user_agent(custom_ua: Option<&str>) -> &str {
    match custom_ua.map(str::trim) {
        Some(trimmed) if !trimmed.is_empty() => trimmed,
        _ => crate::constants::network::SUBSCRIPTION_COMPAT_USER_AGENT,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    #[tokio::test]
    async fn test_get_with_ssrf_guard_blocks_loopback_at_connect_time() {
        // 即使绕过外层静态校验直接调用 get_with_ssrf_guard，连接期 SsrfSafeDnsResolver 也必须拒绝私网/回环地址
        let res = NetworkManager::new()
            .get_with_ssrf_guard("http://localhost:9/sub", ProxyType::None, Some(2), None, false)
            .await;
        assert!(res.is_err(), "SsrfSafeDnsResolver must block localhost at connect time");
        if let Err(err) = res {
            let chain = format!("{err:#}");
            assert!(
                chain.contains("private/loopback") || chain.contains("loopback") || chain.contains("Request failed"),
                "unexpected error chain: {chain}"
            );
        }
    }

    #[test]
    fn test_subscription_compat_user_agent_decoupled_and_pure_single_token() {
        let compat_ua = crate::constants::network::SUBSCRIPTION_COMPAT_USER_AGENT;
        assert_eq!(compat_ua, "clash-verge/v2.5.10");
        // 严禁包含空格复合后缀，确保兼容粗糙 PHP 面板 explode('/', $ua) 与行尾正则
        assert!(!compat_ua.contains(' '));
        assert!(compat_ua.starts_with("clash-verge/v2."));
        // 严禁与客户端自身 UI 版本号（如 3.2.5）耦合
        assert_ne!(compat_ua, format!("clash-verge/v{}", env!("CARGO_PKG_VERSION")));

        assert_eq!(resolve_effective_user_agent(None), compat_ua);
        assert_eq!(resolve_effective_user_agent(Some("")), compat_ua);
        assert_eq!(resolve_effective_user_agent(Some("   \t  ")), compat_ua);
        assert_eq!(
            resolve_effective_user_agent(Some("  CustomSubUA/1.2  ")),
            "CustomSubUA/1.2"
        );
    }

    #[tokio::test]
    async fn test_network_manager_sends_compat_ua_by_default_and_honors_override() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut captured = Vec::with_capacity(2);
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                captured.push(std::string::String::from_utf8_lossy(&buf[..n]).into_owned());
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                    .await
                    .unwrap();
            }
            captured
        });

        let url = format!("http://{addr}/sub");
        let nm = NetworkManager::new();
        let resp_default = nm
            .get_with_interrupt(&url, ProxyType::None, Some(5), None, false)
            .await
            .unwrap();
        assert_eq!(resp_default.text_with_charset().unwrap(), "ok");

        let resp_custom = nm
            .get_with_interrupt(&url, ProxyType::None, Some(5), Some("MyAirportUA/2.0".into()), false)
            .await
            .unwrap();
        assert_eq!(resp_custom.text_with_charset().unwrap(), "ok");

        let captured = server_task.await.unwrap();
        assert!(
            captured[0]
                .to_ascii_lowercase()
                .contains("user-agent: clash-verge/v2.5.10\r\n"),
            "unexpected default request headers: {}",
            captured[0]
        );
        assert!(
            captured[1].contains("MyAirportUA/2.0")
                && captured[1]
                    .to_ascii_lowercase()
                    .contains("user-agent: myairportua/2.0\r\n"),
            "unexpected custom request headers: {}",
            captured[1]
        );
    }
}
