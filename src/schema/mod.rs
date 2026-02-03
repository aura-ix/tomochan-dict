mod dictionary_index;
mod kanji_bank;
mod kanji_meta_bank;
mod tag_bank;
mod term_bank;
mod term_meta_bank;
mod structured_content;
mod json_helpers;

pub use kanji_bank::Kanji;
pub use kanji_meta_bank::KanjiMeta;
pub use tag_bank::Tag;
pub use term_bank::Term;
pub use term_meta_bank::{TermMeta, Frequency, FrequencyValue};

pub(crate) use json_helpers::*;

pub(crate) trait JsonParseable: Sized {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String>;
}

pub(crate) const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();
