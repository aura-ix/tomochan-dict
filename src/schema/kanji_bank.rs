use bincode::{Encode, Decode};
use std::collections::HashMap;
use crate::schema::{JsonParseable, get_str};

#[derive(Encode, Decode, Debug, Clone)]
pub struct Kanji {
    pub character: String,
    pub onyomi: String,
    pub kunyomi: String,
    pub tags: String,
    pub meanings: Vec<String>,
    pub stats: HashMap<String, String>,
}

impl Kanji {
    fn parse_meanings(value: &serde_json::Value) -> Result<Vec<String>, String> {
        Ok(value.as_array()
            .ok_or("Meanings must be an array")?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
    }
    
    fn parse_stats(value: &serde_json::Value) -> Result<HashMap<String, String>, String> {
        Ok(value.as_object()
            .ok_or("Stats must be an object")?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect())
    }
}

impl JsonParseable for Kanji {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String> {
        if arr.len() != 6 {
            return Err("Kanji array must have exactly 6 elements".to_string());
        }
        
        Ok(Kanji {
            character: get_str(&arr[0], "character")?,
            onyomi: get_str(&arr[1], "onyomi")?,
            kunyomi: get_str(&arr[2], "kunyomi")?,
            tags: get_str(&arr[3], "tags")?,
            meanings: Self::parse_meanings(&arr[4])?,
            stats: Self::parse_stats(&arr[5])?,
        })
    }
}