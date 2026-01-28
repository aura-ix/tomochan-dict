use bincode::{Encode, Decode};
use crate::schema::{JsonParseable, get_str, get_f32};

#[derive(Encode, Decode, Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub order: f32,
    pub notes: String,
    pub score: f32,
}

impl JsonParseable for Tag {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String> {
        if arr.len() != 5 {
            return Err("Tag array must have exactly 5 elements".to_string());
        }
        
        Ok(Tag {
            name: get_str(&arr[0], "tag name")?,
            category: get_str(&arr[1], "category")?,
            order: get_f32(&arr[2], "order")?,
            notes: get_str(&arr[3], "notes")?,
            score: get_f32(&arr[4], "score")?,
        })
    }
}