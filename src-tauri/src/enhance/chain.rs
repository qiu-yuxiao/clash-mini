use super::SeqMap;
use crate::{
    config::PrfItem,
    utils::{dirs, help},
};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

#[derive(Debug, Clone)]
pub struct ChainItem {
    pub uid: String,
    pub data: ChainType,
}

#[derive(Debug, Clone)]
pub enum ChainType {
    Merge(Mapping),
    Rules(SeqMap),
    Proxies(SeqMap),
}

// Helper trait to allow async conversion
pub trait AsyncChainItemFrom {
    async fn from_async(item: &PrfItem) -> Option<ChainItem>;
}

impl AsyncChainItemFrom for Option<ChainItem> {
    async fn from_async(item: &PrfItem) -> Self {
        let itype = item.itype.as_ref()?.as_str();
        let file = item.file.clone()?;
        let uid = item.uid.clone().unwrap_or_else(|| "".into());
        let path = dirs::app_profiles_dir().ok()?.join(file.as_str());

        if !path.exists() {
            return None;
        }

        match itype {
            "merge" => Some(ChainItem {
                uid,
                data: ChainType::Merge(help::read_mapping(&path).ok()?),
            }),
            "rules" => {
                let seq_map = help::read_seq_map(&path).ok()?;
                Some(ChainItem {
                    uid,
                    data: ChainType::Rules(seq_map),
                })
            }
            "proxies" => {
                let seq_map = help::read_seq_map(&path).ok()?;
                Some(ChainItem {
                    uid,
                    data: ChainType::Proxies(seq_map),
                })
            }
            // 旧版 profiles.yaml 若遗留 `type: script` 增强项，按无操作安全忽略，确保向后兼容
            _ => None,
        }
    }
}
