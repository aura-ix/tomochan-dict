use bincode::{Encode, Decode};
use crate::schema::{JsonParseable, get_str, Frequency, FrequencyValue};

#[derive(Encode, Decode, Debug, Clone)]
pub struct KanjiMeta {
    pub character: String,
    pub mode: String,
    pub data: Frequency,
}

impl KanjiMeta {
    fn parse_frequency(value: &serde_json::Value) -> Result<Frequency, String> {
        if let Some(obj) = value.as_object() {
            return Ok(Frequency::Detailed {
                value: obj.get("value").and_then(|v| v.as_f64()).ok_or("Missing frequency value")? as f32,
                display_value: obj.get("displayValue").and_then(|v| v.as_str()).map(String::from),
            });
        }
        
        if let Some(num) = value.as_f64() {
            return Ok(Frequency::Simple(FrequencyValue::Number(num as f32)));
        }
        
        if let Some(s) = value.as_str() {
            return Ok(Frequency::Simple(FrequencyValue::String(s.to_string())));
        }
        
        Err("Invalid frequency format".to_string())
    }
}

impl JsonParseable for KanjiMeta {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String> {
        if arr.len() != 3 {
            return Err("Kanji meta array must have exactly 3 elements".to_string());
        }
        
        Ok(KanjiMeta {
            character: get_str(&arr[0], "character")?,
            mode: get_str(&arr[1], "mode")?,
            data: Self::parse_frequency(&arr[2])?,
        })
    }
}