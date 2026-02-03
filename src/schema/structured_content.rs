use bincode::{Encode, Decode};

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

    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_size: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub vertical_align: Option<VerticalAlign>,
    pub text_align: Option<TextAlign>,
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

impl StructuredContent {
    pub fn parse(value: &serde_json::Value) -> Result<StructuredContent, String> {
        if let Some(text) = value.as_str() {
            return Ok(StructuredContent::Text(text.into()));
        }
        
        if let Some(arr) = value.as_array() {
            return Ok(StructuredContent::Array(
                arr.iter().map(Self::parse).collect::<Result<Vec<_>, _>>()?
            ));
        }
        
        let obj = value.as_object().ok_or("Invalid structured content")?;
        let tag = Self::parse_tag(obj.get("tag").and_then(|v| v.as_str()).ok_or("Missing tag")?)?;
        let content = obj.get("content")
            .map(|v| Self::parse(v).map(Box::new))
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
                "lang" => attrs.lang = value.as_str().map(|s| s.into()),
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
                                .filter_map(|(k, v)| v.as_str().map(|s| (String::from(k.as_str()), String::from(s))))
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