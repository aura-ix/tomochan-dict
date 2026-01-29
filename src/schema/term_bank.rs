use bincode::{Encode, Decode};
use crate::schema::{JsonParseable, get_str, get_str_or_default, get_f32, get_i32};

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

#[derive(Encode, Decode, Debug, Clone)]
pub enum StructuredContent {
    Text(String),
    Array(Vec<StructuredContent>),
    Element {
        tag: HtmlTag,
        content: Option<Box<StructuredContent>>,
        attrs: Attributes,
    },
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HtmlTag {
    Br = 0,
    Ruby = 1, Rt = 2, Rp = 3,
    Table = 4, Thead = 5, Tbody = 6, Tfoot = 7, Tr = 8, Td = 9, Th = 10,
    Span = 11, Div = 12, Ol = 13, Ul = 14, Li = 15,
    Details = 16, Summary = 17,
    Img = 18, A = 19,
}

#[derive(Encode, Decode, Debug, Clone, Default)]
pub struct Attributes {
    pub lang: Option<String>,
    pub title: Option<String>,
    pub href: Option<String>,
    pub col_span: Option<u16>,
    pub row_span: Option<u16>,
    pub path: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub alt: Option<String>,
    pub description: Option<String>,
    pub data: Option<Vec<(String, String)>>,
    pub open: Option<bool>,
    
    // Style fields
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_size: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub vertical_align: Option<VerticalAlign>,
    pub text_align: Option<TextAlign>,
    
    // Flags
    pub pixelated: bool,
    pub monochrome: bool,
    pub img_background: bool,
    pub collapsed: bool,
    pub collapsible: bool,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FontStyle {
    Normal = 0,
    Italic = 1,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FontWeight {
    Normal = 0,
    Bold = 1,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VerticalAlign {
    Baseline = 0, Sub = 1, Super = 2, TextTop = 3, TextBottom = 4, Middle = 5, Top = 6, Bottom = 7,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TextAlign {
    Start = 0, End = 1, Left = 2, Right = 3, Center = 4, Justify = 5, JustifyAll = 6, MatchParent = 7,
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
                Ok(Definition::StructuredContent(Self::parse_structured_content(content)?))
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
    
    fn parse_structured_content(value: &serde_json::Value) -> Result<StructuredContent, String> {
        if let Some(text) = value.as_str() {
            return Ok(StructuredContent::Text(text.to_string()));
        }
        
        if let Some(arr) = value.as_array() {
            return Ok(StructuredContent::Array(
                arr.iter().map(Self::parse_structured_content).collect::<Result<Vec<_>, _>>()?
            ));
        }
        
        let obj = value.as_object().ok_or("Invalid structured content")?;
        let tag = Self::parse_tag(obj.get("tag").and_then(|v| v.as_str()).ok_or("Missing tag")?)?;
        let content = obj.get("content")
            .map(|v| Self::parse_structured_content(v).map(Box::new))
            .transpose()?;
        let attrs = Self::parse_attributes(obj)?;
        
        Ok(StructuredContent::Element { tag, content, attrs })
    }
    
    fn parse_tag(tag: &str) -> Result<HtmlTag, String> {
        match tag {
            "br" => Ok(HtmlTag::Br),
            "ruby" => Ok(HtmlTag::Ruby),
            "rt" => Ok(HtmlTag::Rt),
            "rp" => Ok(HtmlTag::Rp),
            "table" => Ok(HtmlTag::Table),
            "thead" => Ok(HtmlTag::Thead),
            "tbody" => Ok(HtmlTag::Tbody),
            "tfoot" => Ok(HtmlTag::Tfoot),
            "tr" => Ok(HtmlTag::Tr),
            "td" => Ok(HtmlTag::Td),
            "th" => Ok(HtmlTag::Th),
            "span" => Ok(HtmlTag::Span),
            "div" => Ok(HtmlTag::Div),
            "ol" => Ok(HtmlTag::Ol),
            "ul" => Ok(HtmlTag::Ul),
            "li" => Ok(HtmlTag::Li),
            "details" => Ok(HtmlTag::Details),
            "summary" => Ok(HtmlTag::Summary),
            "img" => Ok(HtmlTag::Img),
            "a" => Ok(HtmlTag::A),
            _ => Err(format!("Unknown tag: {}", tag))
        }
    }
    
    fn parse_attributes(obj: &serde_json::Map<String, serde_json::Value>) -> Result<Attributes, String> {
        let mut attrs = Attributes::default();
        
        for (key, value) in obj.iter() {
            match key.as_str() {
                "tag" | "content" => continue,
                "lang" => attrs.lang = value.as_str().map(String::from),
                "title" => attrs.title = value.as_str().map(String::from),
                "href" => attrs.href = value.as_str().map(String::from),
                "colSpan" => attrs.col_span = value.as_u64().map(|n| n as u16),
                "rowSpan" => attrs.row_span = value.as_u64().map(|n| n as u16),
                "open" => attrs.open = value.as_bool(),
                "path" => attrs.path = value.as_str().map(String::from),
                "width" => attrs.width = value.as_f64().map(|n| n as f32),
                "height" => attrs.height = value.as_f64().map(|n| n as f32),
                "alt" => attrs.alt = value.as_str().map(String::from),
                "description" => attrs.description = value.as_str().map(String::from),
                "data" => {
                    if let Some(data_obj) = value.as_object() {
                        attrs.data = Some(
                            data_obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        );
                    }
                }
                _ => {}
            }
        }
        
        Ok(attrs)
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