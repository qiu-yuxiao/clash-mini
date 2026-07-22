use crate::{
    config::profiles,
    utils::{
        dirs, help,
        network::{NetworkManager, ProxyType},
        tmpl,
    },
};
use anyhow::{Context as _, Result, bail};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use std::time::Duration;
// TODO, use other re-export
use reqwest_dav::re_exports::url::form_urlencoded;
use tauri::Url;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PrfItem {
    pub uid: Option<String>,

    /// profile item type
    /// enum value: remote | local | script | merge
    #[serde(rename = "type")]
    pub itype: Option<String>,

    /// profile name
    pub name: Option<String>,

    /// profile file
    pub file: Option<String>,

    /// profile description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,

    /// source url
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// selected information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<Vec<PrfSelected>>,

    /// subscription user info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<PrfExtra>,

    /// updated time
    pub updated: Option<i64>,

    /// some options of the item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<PrfOption>,

    /// profile web page url
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,

    /// the file data
    #[serde(skip)]
    pub file_data: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct PrfSelected {
    pub name: Option<String>,
    pub now: Option<String>,
}

#[derive(Default, Debug, Clone, Copy, Deserialize, Serialize)]
pub struct PrfExtra {
    pub upload: u64,
    pub download: u64,
    pub total: u64,
    pub expire: u64,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PrfOption {
    /// for `remote` profile's http request
    /// see issue #13
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// for `remote` profile
    /// use system proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_proxy: Option<bool>,

    /// for `remote` profile
    /// use self proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_proxy: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_interval: Option<u64>,

    /// for `remote` profile
    /// HTTP request timeout in seconds
    /// default is 60 seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// for `remote` profile
    /// disable certificate validation
    /// default is `false`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger_accept_invalid_certs: Option<bool>,

    #[serde(default = "default_allow_auto_update")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_auto_update: Option<bool>,

    pub merge: Option<String>,

    pub script: Option<String>,

    pub rules: Option<String>,

    pub proxies: Option<String>,

    pub groups: Option<String>,
}

impl PrfOption {
    pub fn merge(one: Option<&Self>, other: Option<&Self>) -> Option<Self> {
        match (one, other) {
            (Some(a_ref), Some(b_ref)) => {
                let mut result = a_ref.clone();
                result.user_agent = b_ref.user_agent.clone().or(result.user_agent);
                result.with_proxy = b_ref.with_proxy.or(result.with_proxy);
                result.self_proxy = b_ref.self_proxy.or(result.self_proxy);
                result.danger_accept_invalid_certs =
                    b_ref.danger_accept_invalid_certs.or(result.danger_accept_invalid_certs);
                result.allow_auto_update = b_ref.allow_auto_update.or(result.allow_auto_update);
                result.update_interval = b_ref.update_interval.or(result.update_interval);
                result.merge = b_ref.merge.clone().or(result.merge);
                result.script = b_ref.script.clone().or(result.script);
                result.rules = b_ref.rules.clone().or(result.rules);
                result.proxies = b_ref.proxies.clone().or(result.proxies);
                result.groups = b_ref.groups.clone().or(result.groups);
                result.timeout_seconds = b_ref.timeout_seconds.or(result.timeout_seconds);
                Some(result)
            }
            (Some(a_ref), None) => Some(a_ref.clone()),
            (None, Some(b_ref)) => Some(b_ref.clone()),
            (None, None) => None,
        }
    }
}

const MAX_YAML_SIZE: usize = 50 * 1024 * 1024; // 50MB YAML 解析上限

/// 合并后的增强项 ID 集合
struct EnhanceItems {
    merge: Option<String>,
    script: Option<String>,
    rules: Option<String>,
    proxies: Option<String>,
    groups: Option<String>,
}

/// 确保 merge / script / rules / proxies 子项都存在，缺失则自动创建。
/// groups 只读返回不创建（阻止生成死 g*.yaml 文件）。
async fn ensure_enhance_items(option: Option<&PrfOption>) -> Result<EnhanceItems> {
    let mut merge = option.and_then(|o| o.merge.clone());
    let mut script = option.and_then(|o| o.script.clone());
    let mut rules = option.and_then(|o| o.rules.clone());
    let mut proxies = option.and_then(|o| o.proxies.clone());
    let groups = option.and_then(|o| o.groups.clone());

    if merge.is_none() {
        let merge_item = &mut PrfItem::from_merge(None)?;
        profiles::profiles_append_item_safe(merge_item).await?;
        merge = merge_item.uid.clone();
    }
    if script.is_none() {
        let script_item = &mut PrfItem::from_script(None)?;
        profiles::profiles_append_item_safe(script_item).await?;
        script = script_item.uid.clone();
    }
    if rules.is_none() {
        let rules_item = &mut PrfItem::from_rules()?;
        profiles::profiles_append_item_safe(rules_item).await?;
        rules = rules_item.uid.clone();
    }
    if proxies.is_none() {
        let proxies_item = &mut PrfItem::from_proxies()?;
        profiles::profiles_append_item_safe(proxies_item).await?;
        proxies = proxies_item.uid.clone();
    }

    Ok(EnhanceItems {
        merge,
        script,
        rules,
        proxies,
        groups,
    })
}

impl PrfItem {
    /// From partial item
    /// must contain `itype`
    pub async fn from(item: &Self, file_data: Option<String>) -> Result<Self> {
        if item.itype.is_none() {
            bail!("type should not be null");
        }

        let itype = item
            .itype
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("type should not be null"))?;
        match itype.as_str() {
            "remote" => {
                let url = item
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("url should not be null"))?;
                let name = item.name.as_ref();
                let desc = item.desc.as_ref();
                let option = item.option.as_ref();
                Self::from_url(url, name, desc, option).await
            }
            "local" => {
                let name = item.name.clone().unwrap_or_else(|| "Local File".into());
                let desc = item.desc.clone().unwrap_or_else(|| "".into());
                let option = item.option.as_ref();
                Self::from_local(name, desc, file_data, option).await
            }
            typ => bail!("invalid profile item type \"{typ}\""),
        }
    }

    /// ## Local type
    /// create a new item from name/desc
    pub async fn from_local(
        name: String,
        desc: String,
        file_data: Option<String>,
        option: Option<&PrfOption>,
    ) -> Result<Self> {
        let uid = help::get_uid("L").into();
        let file = format!("{uid}.yaml").into();
        let opt_ref = option.as_ref();
        let update_interval = opt_ref.and_then(|o| o.update_interval);
        let EnhanceItems {
            merge,
            script,
            rules,
            proxies,
            groups,
        } = ensure_enhance_items(option).await?;
        // groups is skipped to prevent creating dead g*.yaml files
        Ok(Self {
            uid: Some(uid),
            itype: Some("local".into()),
            name: Some(name),
            desc: Some(desc),
            file: Some(file),
            url: None,
            selected: None,
            extra: None,
            option: Some(PrfOption {
                update_interval,
                merge,
                script,
                rules,
                proxies,
                groups,
                ..PrfOption::default()
            }),
            home: None,
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(file_data.unwrap_or_else(|| tmpl::ITEM_LOCAL.into())),
        })
    }

    /// ## Direct node input
    /// 直连节点输入（非 HTTP URL）——支持 clear/clean 清空、URI-list 解析、与已有节点合并
    async fn from_url_direct(url: &str, name: Option<&String>, option: Option<&PrfOption>) -> Result<Self> {
        let url_trimmed = url.trim();
        let uid_str = "L_Direct_Imports".to_string();
        let file_name = "L_Direct_Imports.yaml".to_string();
        let path = dirs::app_profiles_dir()?.join(&file_name);

        let mut final_mapping = Mapping::new();
        final_mapping.insert(
            serde_yaml_ng::Value::from("proxies"),
            serde_yaml_ng::Value::from(Vec::<serde_yaml_ng::Value>::new()),
        );

        let EnhanceItems {
            merge,
            script,
            rules,
            proxies,
            groups,
        } = ensure_enhance_items(option).await?;

        if url_trimmed.eq_ignore_ascii_case("clear") || url_trimmed.eq_ignore_ascii_case("clean") {
            let serialized =
                serde_yaml_ng::to_string(&final_mapping).map_err(|e| anyhow::anyhow!("序列化节点配置失败: {}", e))?;
            std::fs::write(&path, serialized.as_bytes())
                .with_context(|| format!("failed to write to file \"{file_name}\""))?;

            let name_str = name.cloned().unwrap_or_else(|| "本地导入节点".into());
            let desc_str = "0".to_string();

            return Ok(Self {
                uid: Some(uid_str.into()),
                itype: Some("local".into()),
                name: Some(name_str),
                desc: Some(desc_str.into()),
                file: Some(file_name.into()),
                url: None,
                selected: None,
                extra: None,
                option: Some(PrfOption {
                    update_interval: None,
                    merge,
                    script,
                    rules,
                    proxies,
                    groups,
                    ..PrfOption::default()
                }),
                home: None,
                updated: Some(chrono::Local::now().timestamp()),
                file_data: Some(serialized.into()),
            });
        }

        let parsed = match crate::utils::resolve::universal_parser::parse_uri_list(url) {
            Some(p) => p,
            None => bail!("无法解析直接输入的节点配置，解析结果为空。请检查输入格式。"),
        };

        final_mapping = parsed;

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut existing_mapping) = serde_yaml_ng::from_str::<Mapping>(&content) {
                    if let Some(existing_proxies_val) = existing_mapping.get_mut("proxies") {
                        if let Some(existing_seq) = existing_proxies_val.as_sequence_mut() {
                            if let Some(new_proxies_val) = final_mapping.get("proxies") {
                                if let Some(new_seq) = new_proxies_val.as_sequence() {
                                    for new_p in new_seq {
                                        let new_name = new_p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        let new_server = new_p.get("server").and_then(|v| v.as_str()).unwrap_or("");
                                        let new_port = new_p
                                            .get("port")
                                            .and_then(|v| match v {
                                                serde_yaml_ng::Value::Number(n) => n.as_u64(),
                                                _ => None,
                                            })
                                            .unwrap_or(0);

                                        let existing_pos = existing_seq.iter().position(|p| {
                                            let p_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                            let p_server = p.get("server").and_then(|v| v.as_str()).unwrap_or("");
                                            let p_port = p
                                                .get("port")
                                                .and_then(|v| match v {
                                                    serde_yaml_ng::Value::Number(n) => n.as_u64(),
                                                    _ => None,
                                                })
                                                .unwrap_or(0);
                                            p_name == new_name && p_server == new_server && p_port == new_port
                                        });

                                        if let Some(pos) = existing_pos {
                                            existing_seq[pos] = new_p.clone();
                                        } else {
                                            existing_seq.push(new_p.clone());
                                        }
                                    }
                                    final_mapping = existing_mapping;
                                }
                            }
                        }
                    }
                }
            }
        }

        let serialized =
            serde_yaml_ng::to_string(&final_mapping).map_err(|e| anyhow::anyhow!("序列化节点配置失败: {}", e))?;

        let name_str = name.cloned().unwrap_or_else(|| "本地导入节点".into());
        let count = final_mapping
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .map(|s| s.len())
            .unwrap_or(0);
        let desc_str = count.to_string();

        Ok(Self {
            uid: Some(uid_str.into()),
            itype: Some("local".into()),
            name: Some(name_str),
            desc: Some(desc_str.into()),
            file: Some(file_name.into()),
            url: None,
            selected: None,
            extra: None,
            option: Some(PrfOption {
                update_interval: None,
                merge,
                script,
                rules,
                proxies,
                groups,
                ..PrfOption::default()
            }),
            home: None,
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(serialized.into()),
        })
    }

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
        let user_agent = option.and_then(|o| o.user_agent.clone());
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

        // 使用网络管理器发送请求
        let resp = match NetworkManager::new()
            .get_with_interrupt(
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

    /// ## Merge type (enhance)
    /// create the enhanced item by using `merge` rule
    pub fn from_merge(uid: Option<String>) -> Result<Self> {
        let (id, template) = if let Some(uid) = uid {
            (uid, tmpl::ITEM_MERGE.into())
        } else {
            (help::get_uid("m").into(), tmpl::ITEM_MERGE_EMPTY.into())
        };
        let file = format!("{id}.yaml").into();

        Ok(Self {
            uid: Some(id),
            itype: Some("merge".into()),
            file: Some(file),
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(template),
            ..Default::default()
        })
    }

    /// ## Script type (enhance)
    /// create the enhanced item by using javascript quick.js
    pub fn from_script(uid: Option<String>) -> Result<Self> {
        let id = if let Some(uid) = uid {
            uid
        } else {
            help::get_uid("s").into()
        };
        let file = format!("{id}.js").into(); // js ext
        Ok(Self {
            uid: Some(id),
            itype: Some("script".into()),
            file: Some(file),
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(tmpl::ITEM_SCRIPT.into()),
            ..Default::default()
        })
    }

    /// ## Rules type (enhance)
    pub fn from_rules() -> Result<Self> {
        let uid = help::get_uid("r").into();
        let file = format!("{uid}.yaml").into(); // yaml ext

        Ok(Self {
            uid: Some(uid),
            itype: Some("rules".into()),
            file: Some(file),
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(tmpl::ITEM_RULES.into()),
            ..Default::default()
        })
    }

    /// ## Proxies type (enhance)
    pub fn from_proxies() -> Result<Self> {
        let uid = help::get_uid("p").into();
        let file = format!("{uid}.yaml").into(); // yaml ext

        Ok(Self {
            uid: Some(uid),
            itype: Some("proxies".into()),
            file: Some(file),
            updated: Some(chrono::Local::now().timestamp()),
            file_data: Some(tmpl::ITEM_PROXIES.into()),
            ..Default::default()
        })
    }

    /// get the file data
    pub fn read_file(&self) -> Result<String> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("could not find the file"))?;
        let path = dirs::app_profiles_dir()?.join(file.as_str());
        let content = std::fs::read_to_string(path).context("failed to read the file")?;
        Ok(content.into())
    }

    /// save the file data
    pub fn save_file(&self, data: String) -> Result<()> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("could not find the file"))?;
        let path = dirs::app_profiles_dir()?.join(file.as_str());

        let should_write = match std::fs::read_to_string(&path) {
            Ok(existing_content) => {
                if existing_content == data {
                    false
                } else {
                    existing_content.replace("\r\n", "\n") != data.replace("\r\n", "\n")
                }
            }
            Err(_) => true,
        };

        if should_write {
            std::fs::write(path, data.as_bytes())
                .context("failed to save the file")
        } else {
            Ok(())
        }
    }
}

impl PrfItem {
    /// 获取current指向的订阅的merge
    pub fn current_merge(&self) -> Option<&String> {
        self.option.as_ref().and_then(|o| o.merge.as_ref())
    }

    /// 获取current指向的订阅的script
    pub fn current_script(&self) -> Option<&String> {
        self.option.as_ref().and_then(|o| o.script.as_ref())
    }

    /// 获取current指向的订阅的rules
    pub fn current_rules(&self) -> Option<&String> {
        self.option.as_ref().and_then(|o| o.rules.as_ref())
    }

    /// 获取current指向的订阅的proxies
    pub fn current_proxies(&self) -> Option<&String> {
        self.option.as_ref().and_then(|o| o.proxies.as_ref())
    }

    /// 获取current指向的订阅的groups
    pub fn current_groups(&self) -> Option<&String> {
        self.option.as_ref().and_then(|o| o.groups.as_ref())
    }
}

// 向前兼容，默认为订阅启用自动更新
#[allow(clippy::unnecessary_wraps)]
const fn default_allow_auto_update() -> Option<bool> {
    Some(true)
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

/// SSRF 防护：禁止访问内网/回环地址
fn validate_url_no_ssrf(url: &Url) -> Result<()> {
    if let Some(host) = url.host() {
        // 1. 静态字符串与直接 IP 校验
        match host {
            url::Host::Domain(d) => {
                let lower = d.to_ascii_lowercase();
                if lower == "localhost"
                    || lower == "127.0.0.1"
                    || lower == "::1"
                    || lower == "0.0.0.0"
                    || lower.starts_with("169.254.")
                {
                    bail!("cannot fetch subscription from localhost/loopback address");
                }
            }
            url::Host::Ipv4(ip) => {
                if ip.is_loopback() || ip.is_private() || ip.is_unspecified() || ip.is_link_local() {
                    bail!("cannot fetch subscription from private/loopback IP");
                }
            }
            url::Host::Ipv6(ip) => {
                if ip.is_loopback() || ip.is_unspecified() {
                    bail!("cannot fetch subscription from loopback IP");
                }
                let segments = ip.segments();
                // 链路本地地址 (fe80::/10) 唯一本地地址 (fc00::/7)
                if (segments[0] & 0xffc0) == 0xfe80 || (segments[0] & 0xfe00) == 0xfc00 {
                    bail!("cannot fetch subscription from local/link-local IP");
                }
            }
        }

        // 2. 动态 DNS 解析校验，防止 DNS 重绑定绕过
        use std::net::ToSocketAddrs as _;
        let host_str = match host {
            url::Host::Domain(d) => d.to_string(),
            url::Host::Ipv4(ip) => ip.to_string(),
            url::Host::Ipv6(ip) => ip.to_string(),
        };
        let port = url.port().unwrap_or(80);
        if let Ok(addrs) = (host_str.as_str(), port).to_socket_addrs() {
            for addr in addrs {
                let ip = addr.ip();
                match ip {
                    std::net::IpAddr::V4(ipv4) => {
                        if ipv4.is_loopback() || ipv4.is_private() || ipv4.is_unspecified() || ipv4.is_link_local() {
                            bail!("cannot fetch subscription from private/loopback IP");
                        }
                    }
                    std::net::IpAddr::V6(ipv6) => {
                        if ipv6.is_loopback() || ipv6.is_unspecified() {
                            bail!("cannot fetch subscription from loopback IP");
                        }
                        let segments = ipv6.segments();
                        if (segments[0] & 0xffc0) == 0xfe80 {
                            bail!("cannot fetch subscription from link-local IP");
                        }
                        if (segments[0] & 0xfe00) == 0xfc00 {
                            bail!("cannot fetch subscription from local range IP");
                        }
                    }
                }
            }
        }
    }
    Ok(())
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prf_item_save_file_read_before_write() {
        // Force portable flag to true so we don't need a Tauri app handle
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);

        // Ensure profiles directory exists
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_prf_item_save_r_b_w.yaml";
        let file_path = profiles_dir.join(file_name);

        // Clean up any existing file
        let _ = tokio::fs::remove_file(&file_path).await;

        let item = PrfItem {
            file: Some(file_name.into()),
            ..Default::default()
        };

        let initial_data: String = "key: value\r\nlist:\r\n  - item1\r\n".into();

        // 1. Initial save (should write)
        item.save_file(initial_data.clone()).expect("initial save failed");
        assert!(file_path.exists());

        let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_first = metadata_first.modified().expect("modified time failed");

        // Sleep to ensure modification time can be distinguished if a write happens
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 2. Save identical content but with different line endings (should skip writing)
        let identical_data = "key: value\nlist:\n  - item1\n".into();
        item.save_file(identical_data).expect("second save failed");

        let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_second = metadata_second.modified().expect("modified time failed");
        assert_eq!(
            mtime_first, mtime_second,
            "mtime changed, meaning file was written unnecessarily"
        );

        // Sleep
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 3. Save actually different content (should write)
        let different_data = "key: different_value\nlist:\n  - item1\n".into();
        item.save_file(different_data).expect("third save failed");

        let metadata_third = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_third = metadata_third.modified().expect("modified time failed");
        assert_ne!(
            mtime_second, mtime_third,
            "mtime did not change, meaning file was not written when it should have been"
        );

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[tokio::test]
    async fn test_prf_item_save_file_empty_strings() {
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_prf_item_save_empty.yaml";
        let file_path = profiles_dir.join(file_name);
        let _ = tokio::fs::remove_file(&file_path).await;

        let item = PrfItem {
            file: Some(file_name.into()),
            ..Default::default()
        };

        // Save empty string
        item.save_file("".into()).expect("save empty string failed");
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.expect("read failed");
        assert_eq!(content, "");

        let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_first = metadata_first.modified().expect("modified time failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Save empty string again (should be skipped)
        item.save_file("".into())
            .expect("save empty string second time failed");
        let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_second = metadata_second.modified().expect("modified time failed");
        assert_eq!(
            mtime_first, mtime_second,
            "mtime changed for redundant empty string write"
        );

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[tokio::test]
    async fn test_prf_item_save_file_missing_file() {
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_prf_item_save_missing.yaml";
        let file_path = profiles_dir.join(file_name);
        let _ = tokio::fs::remove_file(&file_path).await;

        let item = PrfItem {
            file: Some(file_name.into()),
            ..Default::default()
        };

        // File is missing, saving should succeed and create the file
        item.save_file("some content".into())
            .expect("saving to missing file failed");
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.expect("read failed");
        assert_eq!(content, "some content");

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[tokio::test]
    async fn test_prf_item_save_file_line_endings() {
        let _ = dirs::PORTABLE_FLAG.get_or_init(|| true);
        let profiles_dir = dirs::app_profiles_dir().expect("failed to get profiles dir");
        tokio::fs::create_dir_all(&profiles_dir)
            .await
            .expect("failed to create profiles dir");

        let file_name = "test_prf_item_save_endings.yaml";
        let file_path = profiles_dir.join(file_name);
        let _ = tokio::fs::remove_file(&file_path).await;

        let item = PrfItem {
            file: Some(file_name.into()),
            ..Default::default()
        };

        // 1. Save LF content
        item.save_file("line1\nline2\n".into()).expect("save LF failed");
        let metadata_first = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_first = metadata_first.modified().expect("mtime failed");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 2. Save CRLF equivalent content -> should skip write
        item.save_file("line1\r\nline2\r\n".into())
            .expect("save CRLF failed");
        let metadata_second = tokio::fs::metadata(&file_path).await.expect("metadata failed");
        let mtime_second = metadata_second.modified().expect("mtime failed");
        assert_eq!(mtime_first, mtime_second, "mtime changed for normalized CRLF content");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 3. Save mixed content -> should skip write if normalized equivalent
        item.save_file("line1\r\nline2\n".into())
            .expect("save mixed failed");
        let metadata_third = tokio::fs::metadata(&file_path).await.expect("mtime failed");
        let mtime_third = metadata_third.modified().expect("mtime failed");
        assert_eq!(mtime_first, mtime_third, "mtime changed for normalized mixed content");

        // Clean up
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    #[test]
    fn test_prf_item_smartstring_type_safety() {
        // Verify we can manipulate PrfItem using smartstrings as expected
        let uid_smart: smartstring::alias::String = "test_uid".into();
        let name_smart: smartstring::alias::String = "test_name".into();
        let file_smart: smartstring::alias::String = "test_file.yaml".into();

        let item = PrfItem {
            uid: Some(uid_smart.clone()),
            name: Some(name_smart.clone()),
            file: Some(file_smart.clone()),
            ..Default::default()
        };

        assert_eq!(item.uid.unwrap(), uid_smart);
        assert_eq!(item.name.unwrap(), name_smart);
        assert_eq!(item.file.unwrap(), file_smart);
    }
}
