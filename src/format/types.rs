use crate::schema::{Term, Kanji, Tag, TermMeta, KanjiMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u8)]
pub enum QueryKindKey {
    Term = 0x00,
    Kanji = 0x01,
    Tag = 0x02,
    TermMeta = 0x03,
    KanjiMeta = 0x04,
    File = 0x05,
}

impl QueryKindKey {
    pub fn as_byte(self) -> u8 {
        self as u8
    }
    
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(QueryKindKey::Term),
            0x01 => Some(QueryKindKey::Kanji),
            0x02 => Some(QueryKindKey::Tag),
            0x03 => Some(QueryKindKey::TermMeta),
            0x04 => Some(QueryKindKey::KanjiMeta),
            0x05 => Some(QueryKindKey::File),
            _ => None,
        }
    }
}

pub trait Queryable {
    const KIND: QueryKindKey;

    fn key(&self) -> String;
}

impl Queryable for Term {
    const KIND: QueryKindKey = QueryKindKey::Term;

    fn key(&self) -> String {
        self.term.clone()
    }
}

impl Queryable for Kanji {
    const KIND: QueryKindKey = QueryKindKey::Kanji;

    fn key(&self) -> String {
        self.character.clone()
    }
}

impl Queryable for Tag {
    const KIND: QueryKindKey = QueryKindKey::Tag;

    fn key(&self) -> String {
        self.name.clone()
    }
}

impl Queryable for TermMeta {
    const KIND: QueryKindKey = QueryKindKey::TermMeta;

    fn key(&self) -> String {
        self.term.clone()
    }
}

impl Queryable for KanjiMeta {
    const KIND: QueryKindKey = QueryKindKey::KanjiMeta;

    fn key(&self) -> String {
        self.character.clone()
    }
}