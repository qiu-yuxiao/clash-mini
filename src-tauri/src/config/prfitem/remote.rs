use super::{
    EnhanceItems, MAX_YAML_SIZE, PrfExtra, PrfItem, PrfOption, ensure_enhance_items, normalize_custom_user_agent,
};
use crate::utils::{
    help,
    network::{NetworkManager, ProxyType, validate_url_no_ssrf},
};
use anyhow::{Result, bail};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use std::time::Duration;
use tauri::Url;
use url::form_urlencoded;

impl PrfItem {
    /// ## Remote type
    /// create a new item from url
    #[allow(clippy::cognitive_complexity)]
    pub async fn from_url(
        url: &str,
        name: Option<&String>,
        desc: Option<&String>,
        option: Option<&PrfOption>,
    ) -> Result<Self> {
        // 直连节点输入（非 HTTP(S) URL）→ 委托给 from_url_direct
        let url_lower = url.trim().to_ascii_lowercase();
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            return Self::from_url_direct(url, name, option).await;
        }

        let with_proxy = option.is_some_and(|o| o.with_proxy.unwrap_or(false));
        let self_proxy = option.is_some_and(|o| o.self_proxy.unwrap_or(false));
        let accept_invalid_certs = option.is_some_and(|o| o.danger_accept_invalid_certs.unwrap_or(false));
        let allow_auto_update = option.map(|o| o.allow_auto_update.unwrap_or(true));
        let user_agent = normalize_custom_user_agent(option.and_then(|o| o.user_agent.as_deref()));
        let update_interval = option.and_then(|o| o.update_interval);
        let timeout = option.and_then(|o| o.timeout_seconds).unwrap_or(20);
        let EnhanceItems {
            merge,
            script,
            rules,
            proxies,
            groups,
        } = ensure_enhance_items(option).await?;

        // 选择代理类型
        let proxy_type = if self_proxy {
            ProxyType::Localhost
        } else if with_proxy {
            ProxyType::System
        } else {
            ProxyType::None
        };

        let url = fix_dirty_url(url)?;

        // SSRF 防护：禁止访问内网地址
        validate_url_no_ssrf(&url)?;

        // 记录危险选项日志
        if accept_invalid_certs {
            logging!(
                warn,
                Type::Config,
                "⚠️ 订阅使用了危险选项 `danger_accept_invalid_certs=true`，TLS 证书验证被跳过！"
            );
        }

        // 使用带连接期/重定向逐跳 SSRF 守卫的网络管理器发送请求（BUG-327）
        let resp = match NetworkManager::new()
            .get_with_ssrf_guard(
                url.as_str(),
                proxy_type,
                Some(timeout),
                user_agent.clone(),
                accept_invalid_certs,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                bail!("failed to fetch remote profile: {}", e);
            }
        };

        let status_code = resp.status();
        if !status_code.is_success() {
            bail!("failed to fetch remote profile with status {status_code}")
        }

        let header = resp.headers();

        // parse the Subscription UserInfo
        let extra;
        'extra: {
            for (k, v) in header.iter() {
                let key_lower = k.as_str().to_ascii_lowercase();
                // Accept standard custom-metadata prefixes (x-amz-meta-, x-obs-meta-, x-cos-meta-, etc.).
                if key_lower
                    .strip_suffix("subscription-userinfo")
                    .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('-'))
                {
                    let sub_info = v.to_str().unwrap_or("");
                    extra = Some(PrfExtra {
                        upload: help::parse_str(sub_info, "upload").unwrap_or(0),
                        download: help::parse_str(sub_info, "download").unwrap_or(0),
                        total: help::parse_str(sub_info, "total").unwrap_or(0),
                        expire: help::parse_str(sub_info, "expire").unwrap_or(0),
                    });
                    break 'extra;
                }
            }
            extra = None;
        }

        // parse the Content-Disposition
        let filename = match header.get("Content-Disposition") {
            Some(value) => {
                let filename = format!("{value:?}");
                let filename = filename.trim_matches('"');
                match help::parse_str::<String>(filename, "filename*") {
                    Some(filename) => {
                        let iter = percent_encoding::percent_decode(filename.as_bytes());
                        let filename = iter.decode_utf8().unwrap_or_default();
                        filename.split("''").last().map(|s| s.into())
                    }
                    None => match help::parse_str::<String>(filename, "filename") {
                        Some(filename) => {
                            let filename = filename.trim_matches('"');
                            Some(filename.into())
                        }
                        None => None,
                    },
                }
            }
            None => {
                Some(crate::utils::help::get_last_part_and_decode(url.as_str()).unwrap_or_else(|| "Remote File".into()))
            }
        };
        let update_interval = match update_interval {
            Some(val) => Some(val),
            None => match header.get("profile-update-interval") {
                Some(value) => match value.to_str().unwrap_or("").parse::<u64>() {
                    Ok(val) => Some(val * 60), // hour -> min
                    Err(_) => None,
                },
                None => None,
            },
        };

        let home = match header.get("profile-web-page-url") {
            Some(value) => {
                let str_value = value.to_str().unwrap_or("");
                Some(str_value.into())
            }
            None => None,
        };

        let uid = help::get_uid("R").into();
        let file = format!("{uid}.yaml").into();
        let name = name
            .map(|s| s.to_owned())
            .unwrap_or_else(|| filename.map(|s| s.into()).unwrap_or_else(|| "Remote File".into()));
        let data = resp.text_with_charset()?;

        // process the charset "UTF-8 with BOM"
        let data = data.trim_start_matches('\u{feff}');

        // YAML 大小限制防止 DoS
        if data.len() > MAX_YAML_SIZE {
            bail!("subscription content exceeds maximum allowed size (50MB)");
        }

        let (_yaml, serialized_data) = parse_subscription_content(data)?;

        // groups is skipped to prevent creating dead g*.yaml files

        Ok(Self {
            uid: Some(uid),
            itype: Some("remote".into()),
            name: Some(name),
            desc: desc.cloned(),
            file: Some(file),
            url: Some(url.as_str().into()),
            selected: None,
            extra,
            option: Some(PrfOption {
                user_agent,
                update_interval,
                merge,
                script,
                rules,
                proxies,
                groups,
                allow_auto_update,
                ..PrfOption::default()
            }),
            home,
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(serialized_data),
        })
    }
}

/// 解析订阅响应内容：YAML → Base64 解码 → URI-list 三级回退
fn parse_subscription_content(data: &str) -> Result<(Mapping, String)> {
    let decoded_opt = crate::utils::resolve::universal_parser::decode_base64_robust(data);

    match serde_yaml_ng::from_str::<Mapping>(data) {
        Ok(y) if y.contains_key("proxies") || y.contains_key("proxy-providers") => Ok((y, data.to_string().into())),
        _ => {
            let decoded_str_opt = decoded_opt
                .as_ref()
                .and_then(|bytes| std::string::String::from_utf8(bytes.clone()).ok());

            let parsed_yaml_from_b64 = decoded_str_opt.as_ref().and_then(|decoded_str| {
                serde_yaml_ng::from_str::<Mapping>(decoded_str)
                    .ok()
                    .filter(|y| y.contains_key("proxies") || y.contains_key("proxy-providers"))
                    .map(|y| (y, decoded_str.to_string().into()))
            });

            if let Some(res) = parsed_yaml_from_b64 {
                Ok(res)
            } else {
                let parse_content = decoded_str_opt.as_deref().unwrap_or(data);
                if let Some(parsed) = crate::utils::resolve::universal_parser::parse_uri_list(parse_content) {
                    let serialized = serde_yaml_ng::to_string(&parsed).unwrap_or_default().into();
                    Ok((parsed, serialized))
                } else {
                    Err(anyhow::anyhow!(
                        "订阅链接内容格式错误，既不是合法的 YAML 配置文件，也无法解析为节点链接列表"
                    ))
                }
            }
        }
    }
}

/// Fix URLs where query parameters are incorrectly appended to the path segment
///
/// Incorrect Example: https://example.com/path&param1=value1
fn fix_dirty_url(input: &str) -> Result<Url> {
    let mut url = match Url::parse(input) {
        Ok(u) => u,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to parse subscription URL: {:?}, input: {}",
                e,
                help::mask_url(input)
            ));
        }
    };

    if url.query().is_none() && url.path().contains('&') {
        let path = url.path().to_string();

        if let Some((clean_path, dirty_params)) = path.split_once('&') {
            url.set_path(clean_path);

            url.query_pairs_mut()
                .extend_pairs(form_urlencoded::parse(dirty_params.as_bytes()));
        }
    }

    Ok(url)
}
