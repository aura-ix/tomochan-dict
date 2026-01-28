use bincode::{Encode, Decode};
use std::collections::HashMap;
use crate::schema::get_optional_str;

#[derive(Encode, Decode, Debug, Clone)]
pub struct DictionaryIndex {
    pub title: String,
    pub revision: String,
    pub minimum_yomitan_version: Option<String>,
    pub sequenced: bool,
    pub format: u8,
    pub author: Option<String>,
    pub is_updatable: bool,
    pub index_url: Option<String>,
    pub download_url: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
    pub frequency_mode: Option<FrequencyMode>,
    pub tag_meta: Option<HashMap<String, TagMetaInfo>>,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrequencyMode {
    OccurrenceBased = 0,
    RankBased = 1,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct TagMetaInfo {
    pub category: Option<String>,
    pub order: Option<f32>,
    pub notes: Option<String>,
    pub score: Option<f32>,
}

impl DictionaryIndex {
    pub fn from_json(obj: &serde_json::Map<String, serde_json::Value>) -> Result<Self, String> {
        let title = obj.get("title")
            .and_then(|v| v.as_str())
            .ok_or("Missing title")?
            .to_string();
        
        let revision = obj.get("revision")
            .and_then(|v| v.as_str())
            .ok_or("Missing revision")?
            .to_string();
        
        let format = obj.get("format")
            .or_else(|| obj.get("version"))
            .and_then(|v| v.as_u64())
            .ok_or("Missing format/version")? as u8;
        
        if format < 1 || format > 3 {
            return Err(format!("Invalid format value: {}", format));
        }
        
        Ok(DictionaryIndex {
            title,
            revision,
            minimum_yomitan_version: get_optional_str(obj, "minimumYomitanVersion"),
            sequenced: obj.get("sequenced").and_then(|v| v.as_bool()).unwrap_or(false),
            format,
            author: get_optional_str(obj, "author"),
            is_updatable: obj.get("isUpdatable").and_then(|v| v.as_bool()).unwrap_or(false),
            index_url: get_optional_str(obj, "indexUrl"),
            download_url: get_optional_str(obj, "downloadUrl"),
            url: get_optional_str(obj, "url"),
            description: get_optional_str(obj, "description"),
            attribution: get_optional_str(obj, "attribution"),
            source_language: get_optional_str(obj, "sourceLanguage"),
            target_language: get_optional_str(obj, "targetLanguage"),
            frequency_mode: obj.get("frequencyMode")
                .and_then(|v| v.as_str())
                .and_then(Self::parse_frequency_mode),
            tag_meta: obj.get("tagMeta")
                .and_then(|v| v.as_object())
                .map(|meta_obj| Self::parse_tag_meta(meta_obj))
                .transpose()?,
        })
    }
    
    fn parse_frequency_mode(s: &str) -> Option<FrequencyMode> {
        match s {
            "occurrence-based" => Some(FrequencyMode::OccurrenceBased),
            "rank-based" => Some(FrequencyMode::RankBased),
            _ => None,
        }
    }
    
    fn parse_tag_meta(obj: &serde_json::Map<String, serde_json::Value>) -> Result<HashMap<String, TagMetaInfo>, String> {
        Ok(obj.iter()
            .filter_map(|(tag_name, value)| {
                value.as_object().map(|tag_obj| {
                    (tag_name.clone(), TagMetaInfo {
                        category: get_optional_str(tag_obj, "category"),
                        order: tag_obj.get("order").and_then(|v| v.as_f64()).map(|n| n as f32),
                        notes: get_optional_str(tag_obj, "notes"),
                        score: tag_obj.get("score").and_then(|v| v.as_f64()).map(|n| n as f32),
                    })
                })
            })
            .collect())
    }
}