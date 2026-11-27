mod remote;
#[cfg(test)]
mod tests;

use crate::{
    config::profiles,
    utils::{dirs, help, tmpl},
};
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

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
                result.user_agent = match b_ref.user_agent.as_deref().map(str::trim) {
                    Some("") => None,
                    Some(non_empty) => Some(non_empty.into()),
                    None => normalize_custom_user_agent(result.user_agent.as_deref()),
                };
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
            (Some(a_ref), None) => {
                let mut cloned = a_ref.clone();
                cloned.user_agent = normalize_custom_user_agent(cloned.user_agent.as_deref());
                Some(cloned)
            }
            (None, Some(b_ref)) => {
                let mut cloned = b_ref.clone();
                cloned.user_agent = normalize_custom_user_agent(cloned.user_agent.as_deref());
                Some(cloned)
            }
            (None, None) => None,
        }
    }
}

pub(super) fn normalize_custom_user_agent(ua: Option<&str>) -> Option<String> {
    ua.map(str::trim).filter(|s| !s.is_empty()).map(Into::into)
}

fn proxy_port_num(p: &serde_yaml_ng::Value) -> u64 {
    p.get("port")
        .and_then(|v| match v {
            serde_yaml_ng::Value::Number(n) => n.as_u64(),
            _ => None,
        })
        .unwrap_or(0)
}

fn merge_direct_import_mapping(existing: &mut Mapping, mut incoming: Mapping) {
    if let Some(serde_yaml_ng::Value::Sequence(new_seq)) = incoming.remove("proxies") {
        let existing_seq = match existing.get_mut("proxies") {
            Some(serde_yaml_ng::Value::Sequence(seq)) => seq,
            _ => {
                existing.insert(
                    serde_yaml_ng::Value::from("proxies"),
                    serde_yaml_ng::Value::Sequence(Vec::new()),
                );
                match existing.get_mut("proxies") {
                    Some(serde_yaml_ng::Value::Sequence(seq)) => seq,
                    _ => return,
                }
            }
        };
        for new_p in new_seq {
            let new_name = new_p.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let new_server = new_p.get("server").and_then(|v| v.as_str()).unwrap_or("");
            let new_port = proxy_port_num(&new_p);
            let existing_pos = existing_seq.iter().position(|p| {
                p.get("name").and_then(|v| v.as_str()).unwrap_or("") == new_name
                    && p.get("server").and_then(|v| v.as_str()).unwrap_or("") == new_server
                    && proxy_port_num(p) == new_port
            });
            if let Some(pos) = existing_pos {
                existing_seq[pos] = new_p;
            } else {
                existing_seq.push(new_p);
            }
        }
    }

    if let Some(serde_yaml_ng::Value::Mapping(new_providers)) = incoming.remove("proxy-providers")
        && !new_providers.is_empty()
    {
        match existing.get_mut("proxy-providers") {
            Some(serde_yaml_ng::Value::Mapping(existing_providers)) => {
                for (k, v) in new_providers {
                    existing_providers.insert(k, v);
                }
            }
            _ => {
                existing.insert(
                    serde_yaml_ng::Value::from("proxy-providers"),
                    serde_yaml_ng::Value::Mapping(new_providers),
                );
            }
        }
    }
}

pub(super) const MAX_YAML_SIZE: usize = 50 * 1024 * 1024; // 50MB YAML 解析上限

/// 合并后的增强项 ID 集合
pub(super) struct EnhanceItems {
    merge: Option<String>,
    script: Option<String>,
    rules: Option<String>,
    proxies: Option<String>,
    groups: Option<String>,
}

/// 确保 merge / script / rules / proxies 子项都存在，缺失则自动创建。
/// groups 只读返回不创建（阻止生成死 g*.yaml 文件）。
pub(super) async fn ensure_enhance_items(option: Option<&PrfOption>) -> Result<EnhanceItems> {
    let mut merge = option.and_then(|o| o.merge.clone());
    let script = option.and_then(|o| o.script.clone());
    let mut rules = option.and_then(|o| o.rules.clone());
    let mut proxies = option.and_then(|o| o.proxies.clone());
    let groups = option.and_then(|o| o.groups.clone());

    if merge.is_none() {
        let merge_item = &mut PrfItem::from_merge(None)?;
        profiles::profiles_append_item_safe(merge_item).await?;
        merge = merge_item.uid.clone();
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

        if url_trimmed.len() > MAX_YAML_SIZE {
            bail!("subscription content exceeds maximum allowed size (50MB)");
        }

        let (parsed, _) = crate::utils::resolve::universal_parser::parse_subscription_payload(url)
            .map_err(|_| anyhow::anyhow!("无法解析直接输入的节点配置，解析结果为空。请检查输入格式。"))?;

        final_mapping = parsed;

        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(mut existing_mapping) = serde_yaml_ng::from_str::<Mapping>(&content)
        {
            merge_direct_import_mapping(&mut existing_mapping, final_mapping);
            final_mapping = existing_mapping;
        }

        let serialized =
            serde_yaml_ng::to_string(&final_mapping).map_err(|e| anyhow::anyhow!("序列化节点配置失败: {}", e))?;

        let name_str = name.cloned().unwrap_or_else(|| "本地导入节点".into());
        let proxy_count = final_mapping
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .map_or(0, Vec::len);
        let provider_count = final_mapping
            .get("proxy-providers")
            .and_then(|v| v.as_mapping())
            .map_or(0, serde_yaml_ng::Mapping::len);
        let desc_str = if proxy_count > 0 {
            proxy_count.to_string()
        } else {
            provider_count.to_string()
        };

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
            std::fs::write(path, data.as_bytes()).context("failed to save the file")
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
