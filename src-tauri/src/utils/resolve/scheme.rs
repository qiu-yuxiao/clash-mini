use anyhow::Result;
use percent_encoding::percent_decode_str;
use smartstring::alias::String;
use tauri::Url;

use crate::{
    config::{Config, PrfItem, profiles},
    core::{CoreManager, handle},
    utils::help,
};
use clash_verge_logging::{Type, logging, logging_error};

pub(super) async fn resolve_scheme(param: &str) -> Result<()> {
    let param_str = if param.starts_with("[") && param.len() > 4 {
        param
            .get(2..param.len() - 2)
            .ok_or_else(|| anyhow::anyhow!("Invalid string slice boundaries"))?
    } else {
        param
    };
    let masked_deep_link = help::mask_url(param_str);

    logging!(debug, Type::Config, "received deep link: {masked_deep_link}");

    let link_parsed = Url::parse(param_str)
        .map_err(|e| anyhow::anyhow!("failed to parse deep link: {e:?}, param: {masked_deep_link}"))?;

    let Some((url, name)) = extract_subscription_info(&link_parsed) else {
        logging!(
            warn,
            Type::Config,
            "missing url parameter in deep link: {masked_deep_link}"
        );
        return Ok(());
    };

    import_subscription(&url, name.as_ref()).await;
    Ok(())
}

fn extract_subscription_info(link_parsed: &Url) -> Option<(std::string::String, Option<String>)> {
    if !matches!(link_parsed.scheme(), "clash" | "clash-verge" | "clash-mini") {
        return None;
    }

    let name = link_parsed
        .query_pairs()
        .find(|(key, _)| key == "name")
        .map(|(_, value)| value.into_owned().into());
    let url = extract_subscription_url(link_parsed)?;
    Some((url, name))
}

fn extract_subscription_url(link_parsed: &Url) -> Option<std::string::String> {
    let query = link_parsed.query()?;
    let prefix = "url=";
    let pos = query
        .find("&url=")
        .map(|i| i + 1)
        .or_else(|| query.starts_with(prefix).then_some(0))?;
    let after_prefix = query[pos + prefix.len()..].trim();
    if after_prefix.is_empty() {
        return None;
    }

    // 标准百分号编码订阅链接或不含内嵌 `?` 的直白链接，直接使用标准 query_pairs 提取，
    // 避免把后续 `&name=...` 等扩展参数误拼入订阅 URL（BUG-323）。
    let lower = after_prefix.to_ascii_lowercase();
    let is_unencoded_nested_query = (lower.starts_with("http://") || lower.starts_with("https://"))
        && after_prefix.split('&').next().is_some_and(|first| first.contains('?'));

    if !is_unencoded_nested_query && let Some((_, val)) = link_parsed.query_pairs().find(|(k, _)| k == "url") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(decode_subscription_url(trimmed));
        }
    }

    // 兼容未对内嵌 URL 进行百分号编码的非标链接（如 url=https://a.com/sub?token=1&flag=clash&name=MySub），
    // 剥离已知的外层 deep-link 尾部参数（&name= / &silent=）。
    let mut raw_url = after_prefix;
    for marker in ["&name=", "&silent="] {
        if let Some(idx) = raw_url.find(marker) {
            raw_url = &raw_url[..idx];
        }
    }
    Some(decode_subscription_url(raw_url.trim()))
}

fn decode_subscription_url(raw_url: &str) -> std::string::String {
    // Avoid double-decoding nested subscription URLs; decode only when needed.
    if Url::parse(raw_url).is_ok() {
        return raw_url.to_string();
    }

    let mut candidate = raw_url.to_string();
    for _ in 0..2 {
        let next = percent_decode_str(&candidate).decode_utf8_lossy().to_string();
        if next == candidate {
            break;
        }
        candidate = next;
        if Url::parse(&candidate).is_ok() {
            break;
        }
    }
    candidate
}

async fn import_subscription(url: &str, name: Option<&String>) {
    let had_current_profile = {
        let profiles = Config::profiles().await;
        profiles.latest_arc().current.is_some()
    };

    let Some(mut item) = fetch_profile_item(url, name).await else {
        return;
    };

    let uid = item.uid.clone().unwrap_or_default();
    if let Err(e) = profiles::profiles_append_item_safe(&mut item).await {
        logging!(error, Type::Config, "failed to import subscription url: {:?}", e);
        Config::profiles().await.discard();
        handle::Handle::notice_message("import_sub_url::error", e.to_string());
        return;
    }

    Config::profiles().await.apply();
    logging_error!(Type::Config, Config::profiles().await.data_arc().save_file());
    handle::Handle::notice_message(
        "import_sub_url::ok",
        "", // 空 msg 传入，我们不希望导致 后端-前端-后端 死循环，这里只做提醒。
    );

    post_import_updates(&uid, had_current_profile).await;
}

async fn fetch_profile_item(url: &str, name: Option<&String>) -> Option<PrfItem> {
    match PrfItem::from_url(url, name, None, None).await {
        Ok(item) => Some(item),
        Err(e) => {
            logging!(error, Type::Config, "failed to parse profile from url: {:?}", e);
            handle::Handle::notice_message("import_sub_url::error", e.to_string());
            None
        }
    }
}

async fn post_import_updates(uid: &String, had_current_profile: bool) {
    handle::Handle::refresh_verge();
    handle::Handle::notify_profile_changed(uid);

    let should_update_core = if uid.is_empty() || had_current_profile {
        false
    } else {
        let profiles = Config::profiles().await;
        profiles.latest_arc().is_current_profile_index(uid)
    };

    if should_update_core {
        refresh_core_config().await;
    }
}

async fn refresh_core_config() {
    logging!(
        info,
        Type::Config,
        "Deep link import set current profile; refreshing core config"
    );
    match CoreManager::global().update_config_forced().await {
        Ok(outcome) if outcome.is_valid() => handle::Handle::refresh_clash(),
        Ok(outcome) => {
            let message = outcome.to_string();
            logging!(warn, Type::Config, "Apply config failed: {}", message);
            handle::Handle::notice_message("config_validate::error", message);
        }
        Err(err) => {
            logging!(error, Type::Config, "Apply config error: {}", err);
            handle::Handle::notice_message("update_failed", format!("{err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subscription_info_strips_outer_name_param() {
        // BUG-323: 标准 deep link 携带 &name= 时，不得将 &name=... 误拼入订阅 URL
        let parsed = Url::parse(
            "clash://install-config?url=https%3A%2F%2Fexample.com%2Fsub%3Ftoken%3Dabc%26flag%3Dclash&name=MyAirport",
        )
        .unwrap();
        let (url, name) = extract_subscription_info(&parsed).unwrap();
        assert_eq!(url, "https://example.com/sub?token=abc&flag=clash");
        assert_eq!(name.as_deref(), Some("MyAirport"));

        // 未编码直白订阅链接 + &name=
        let parsed_plain = Url::parse(
            "clash://install-config?url=https://example.com/api/v1/client/subscribe?token=xyz&flag=meta&name=PlainSub",
        )
        .unwrap();
        let (url2, name2) = extract_subscription_info(&parsed_plain).unwrap();
        assert_eq!(url2, "https://example.com/api/v1/client/subscribe?token=xyz&flag=meta");
        assert_eq!(name2.as_deref(), Some("PlainSub"));
    }
}
