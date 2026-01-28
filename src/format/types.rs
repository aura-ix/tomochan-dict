use bincode::{Encode, Decode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataType {
    Term = 0x00,
    Kanji = 0x01,
    Tag = 0x02,
    TermMeta = 0x03,
    KanjiMeta = 0x04,
    ExtraFile = 0x05,
}

impl DataType {
    pub fn as_byte(self) -> u8 {
        self as u8
    }
    
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(DataType::Term),
            0x01 => Some(DataType::Kanji),
            0x02 => Some(DataType::Tag),
            0x03 => Some(DataType::TermMeta),
            0x04 => Some(DataType::KanjiMeta),
            0x05 => Some(DataType::ExtraFile),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash)]
pub struct TermId(pub u64);

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash)]
pub struct KanjiId(pub u64);

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash)]
pub struct TagId(pub u64);

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash)]
pub struct TermMetaId(pub u64);

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash)]
pub struct KanjiMetaId(pub u64);

#[derive(Debug, Clone)]
pub struct LookupResult<T> {
    pub id: T,
}

#[derive(Debug, Clone)]
pub struct TermLookupResult {
    pub term_id: TermId,
    pub score: f32,
}

impl TermLookupResult {
    pub fn new(term_id: TermId, score: f32) -> Self {
        Self { term_id, score }
    }
}

/// import format
#[derive(Debug, Clone)]
pub struct DictEntry(
    pub String,                     // term
    pub String,                     // reading
    pub Option<String>,             // definition_tags
    pub String,                     // rules
    pub i32,                        // score
    pub Vec<serde_json::Value>,     // definitions
    pub i32,                        // sequence
    pub String,                     // term_tags
);