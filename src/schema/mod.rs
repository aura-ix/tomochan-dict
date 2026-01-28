use bincode::{Encode, Decode};
use std::{fs, io::Read, path::Path};

mod dictionary_index;
mod kanji_bank;
mod kanji_meta_bank;
mod tag_bank;
mod term_bank;
mod term_meta_bank;

mod indexable;
mod json_helpers;

pub use dictionary_index::DictionaryIndex;
pub use kanji_bank::Kanji;
pub use kanji_meta_bank::KanjiMeta;
pub use tag_bank::Tag;
pub use term_bank::Term;
pub use term_meta_bank::{TermMeta, Frequency, FrequencyValue};
pub use indexable::Indexable;

pub(crate) use json_helpers::*;

pub(crate) trait JsonParseable: Sized {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String>;
}

pub(crate) const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Encode, Decode, Debug, Clone)]
pub struct Dictionary {
    pub index: DictionaryIndex,
    pub terms: Vec<Term>,
    pub term_meta: Vec<TermMeta>,
    pub kanji: Vec<Kanji>,
    pub kanji_meta: Vec<KanjiMeta>,
    pub tags: Vec<Tag>,
}

impl Dictionary {
    pub fn from_directory<P: AsRef<Path>>(dir: P) -> Result<Self, String> {
        let dir = dir.as_ref();
        if !dir.is_dir() { return Err(format!("Path is not a directory: {:?}", dir)); }

        Ok(Self {
            index: Self::load_index(dir)?,
            terms: Self::load_typed_banks(dir, "term_bank_", "Term")?,
            term_meta: Self::load_typed_banks(dir, "term_meta_bank_", "Term meta")?,
            kanji: Self::load_typed_banks(dir, "kanji_bank_", "Kanji")?,
            kanji_meta: Self::load_typed_banks(dir, "kanji_meta_bank_", "Kanji meta")?,
            tags: Self::load_typed_banks(dir, "tag_bank_", "Tag")?,
        })
    }

    fn load_index<P: AsRef<Path>>(dir: P) -> Result<DictionaryIndex, String> {
        let path = dir.as_ref().join("index.json");
        let json: serde_json::Value = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read index.json: {}", e))?
            .parse()
            .map_err(|e| format!("Failed to parse index.json: {}", e))?;
        let obj = json.as_object().ok_or("index.json must be an object")?;
        DictionaryIndex::from_json(obj)
    }

    fn load_typed_banks<P, T>(dir: P, prefix: &str, type_name: &str) -> Result<Vec<T>, String>
    where
        P: AsRef<Path>,
        T: JsonParseable,
    {
        let mut all = Vec::new();
        for i in 1.. {
            let file = dir.as_ref().join(format!("{}{}.json", prefix, i));
            if !file.exists() { break; }
            let content = fs::read_to_string(&file)
                .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;
            let arr = serde_json::from_str::<Vec<serde_json::Value>>(&content)
                .map_err(|e| format!("Failed to parse {}: {}", file.display(), e))?;
            for item in arr.iter() {
                let item_arr = item.as_array()
                    .ok_or(format!("{} entry must be an array", type_name))?;
                all.push(T::from_json_array(item_arr)?);
            }
        }
        Ok(all)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let encoded = bincode::encode_to_vec(self, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to encode dictionary: {}", e))?;
        fs::write(path, encoded).map_err(|e| format!("Failed to write file: {}", e))
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut buffer = Vec::new();
        fs::File::open(&path).and_then(|mut f| f.read_to_end(&mut buffer))
            .map_err(|e| format!("Failed to read file: {}", e))?;
        bincode::decode_from_slice(&buffer, BINCODE_CONFIG)
            .map(|(dict, _)| dict)
            .map_err(|e| format!("Failed to decode dictionary: {}", e))
    }
}
