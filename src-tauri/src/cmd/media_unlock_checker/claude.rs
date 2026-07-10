use reqwest::Client;

use super::UnlockItem;

const BLOCKED_CODES: [&str; 10] = ["AF", "BY", "CN", "CU", "HK", "IR", "KP", "MO", "RU", "SY"];

pub(super) async fn check_claude(client: &Client) -> UnlockItem {
    let url = "https://claude.ai/cdn-cgi/trace";
    let failed = || UnlockItem::checked("Claude", "Failed", None);

    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return failed(),
    };

    let body = match response.text().await {
        Ok(b) => b,
        Err(_) => return failed(),
    };

    let country_code = body
        .lines()
        .find_map(|line| line.strip_prefix("loc=").map(|rest| rest.trim().to_uppercase()));

    match country_code {
        Some(code) => {
            let status = if BLOCKED_CODES.contains(&code.as_str()) {
                "No"
            } else {
                "Yes"
            };
            UnlockItem::checked_region("Claude", status, &code)
        }
        None => failed(),
    }
}
