use bincode::{Encode, Decode};
use crate::schema::{JsonParseable, get_str, get_str_or_default, get_f32, get_i32};
use crate::schema::structured_content::StructuredContent;

#[derive(Encode, Decode, Debug, Clone)]
pub struct Term {
    pub term: String,
    pub reading: String,
    pub definitions: Vec<Definition>,
    pub score: f32, // TODO: utilize score
    pub sequence: i32, // TODO: utilize sequence
    pub definition_tags: String,
    pub rules: String,
    pub term_tags: String,
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum Definition {
    Text(String),
    StructuredContent(StructuredContent),
    Image {
        path: String,
        width: Option<u16>,
        height: Option<u16>,
        title: Option<String>,
        alt: Option<String>,
        description: Option<String>,
        pixelated: bool,
        monochrome: bool,
        background: bool,
    },
    Deinflection {
        uninflected: String,
        rules: Vec<String>,
    },
}

impl Term {
    fn parse_definitions(value: &serde_json::Value) -> Result<Vec<Definition>, String> {
        value.as_array()
            .ok_or("Definitions must be an array")?
            .iter()
            .map(Self::parse_definition)
            .collect()
    }
    
    pub fn parse_definition(value: &serde_json::Value) -> Result<Definition, String> {
        if let Some(text) = value.as_str() {
            return Ok(Definition::Text(text.to_string()));
        }
        
        if let Some(arr) = value.as_array() {
            if arr.len() == 2 {
                if let (Some(uninflected), Some(rules_arr)) = (arr[0].as_str(), arr[1].as_array()) {
                    let rules = rules_arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    return Ok(Definition::Deinflection { 
                        uninflected: uninflected.to_string(), 
                        rules 
                    });
                }
            }
        }
        
        let obj = value.as_object().ok_or("Invalid definition format")?;
        let def_type = obj.get("type").and_then(|v| v.as_str()).ok_or("Missing type")?;
        
        match def_type {
            "text" => {
                let text = obj.get("text").and_then(|v| v.as_str()).ok_or("Missing text")?;
                Ok(Definition::Text(text.to_string()))
            }
            "structured-content" => {
                let content = obj.get("content").ok_or("Missing content")?;
                Ok(Definition::StructuredContent(StructuredContent::parse(content)?))
            }
            "image" => {
                Ok(Definition::Image {
                    path: obj.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?.to_string(),
                    width: obj.get("width").and_then(|v| v.as_u64()).map(|v| v as u16),
                    height: obj.get("height").and_then(|v| v.as_u64()).map(|v| v as u16),
                    title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
                    alt: obj.get("alt").and_then(|v| v.as_str()).map(String::from),
                    description: obj.get("description").and_then(|v| v.as_str()).map(String::from),
                    pixelated: obj.get("pixelated").and_then(|v| v.as_bool()).unwrap_or(false),
                    monochrome: obj.get("appearance").and_then(|v| v.as_str()) == Some("monochrome"),
                    background: obj.get("background").and_then(|v| v.as_bool()).unwrap_or(true),
                })
            }
            _ => Err(format!("Unknown definition type: {}", def_type))
        }
    }
}

impl JsonParseable for Term {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String> {
        if arr.len() != 8 {
            return Err("Term array must have exactly 8 elements".to_string());
        }
        
        Ok(Term {
            term: get_str(&arr[0], "term")?,
            reading: get_str(&arr[1], "reading")?,
            definition_tags: get_str_or_default(&arr[2]),
            rules: get_str(&arr[3], "rules")?,
            score: get_f32(&arr[4], "score")?,
            definitions: Self::parse_definitions(&arr[5])?,
            sequence: get_i32(&arr[6], "sequence")?,
            term_tags: get_str(&arr[7], "term tags")?,
        })
    }
}