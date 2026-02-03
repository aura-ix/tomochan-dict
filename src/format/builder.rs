use super::store::UnifiedStoreBuilder;
use super::types::{Queryable, QueryKindKey};
use super::index::{UnifiedFstIndex};
use super::container::{ContainerFileInfo, ContainerHeader, write_container};
use crate::schema::{Term, Tag, Kanji, KanjiMeta, TermMeta, BINCODE_CONFIG};
use crate::schema::JsonParseable;
use std::fs;
use std::fs::File;
use std::path::Path;
use bincode::{Encode, Decode};
use std::io::{Read, Seek, SeekFrom};

// TODO: mmap fst
// TODO: fix error handling

#[derive(Encode, Decode)]
pub struct DictionaryPackage {
    pub fst: Vec<u8>,
    pub data: Vec<u8>,
}

impl DictionaryPackage {
    pub fn save(&self, path: &str, header: ContainerHeader) -> Result<(), String> {
        let encoded = bincode::encode_to_vec(self, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to encode package: {}", e))?;
        
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to open package file: {}", e))?;
        write_container(&mut file, header, &encoded)
            .map_err(|e| format!("Failed to write package file: {}", e))?;
            
        Ok(())
    }

    pub fn load_reader<R: Read>(reader: &mut R) -> Result<Self, String> {
        // TODO: validate entire length is read
        let package: Self = bincode::decode_from_std_read(reader, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to decode package: {}", e))?;

        Ok(package)
    }
    
    pub fn load_path(path: &str) -> Result<Self, String> {
        // TODO: validate entire length is read
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to read package file: {}", e))?;

        // TODO: check appropriate version
        let container = ContainerFileInfo::read_container(&mut file)
            .map_err(|e| format!("Container error: {}", e))?;
        
        file.seek(SeekFrom::Start(container.payload_offset))
            .map_err(|e| format!("Failed to seek to dictionary contents: {}", e))?;

        Self::load_reader(&mut file)
    }
}

fn import_files(
    base_dir: &str,
    current_dir: &str,
    store: &mut UnifiedStoreBuilder,
    mapping: &mut Vec<(QueryKindKey, String, u64)>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current_dir)
        .map_err(|e| format!("Failed to read directory {}: {}", current_dir, e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        
        if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid filename")?;
            
            const SKIP_PREFIXES: &[&str] = &["term_bank_", "term_meta_bank_", "kanji_bank_", "kanji_meta_bank_", "tag_bank_"];
            if file_name == "index.json" || SKIP_PREFIXES.iter().any(|prefix| file_name.starts_with(prefix)) {
                continue;
            }
            
            let file_data = fs::read(&path)
                .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
            
            let rel_path = path.strip_prefix(base_dir)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;
            let rel_path_str = rel_path.to_str()
                .ok_or("Invalid path string")?
                .to_string();

            mapping.push((QueryKindKey::File, rel_path_str, store.insert(&file_data)?));
            
        } else if path.is_dir() {
            let dir_str = path.to_str().ok_or("Invalid directory path")?;
            import_files(base_dir, dir_str, store, mapping)?;
        }
    }
    
    Ok(())
}

pub fn load_typed_banks<P, T>(dir: P, prefix: &str, type_name: &str, store: &mut UnifiedStoreBuilder,
    mapping: &mut Vec<(QueryKindKey, String, u64)>,) -> Result<(), String>
where
    P: AsRef<Path>,
    T: JsonParseable + Queryable + bincode::Encode,
{
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

            let elem = T::from_json_array(item_arr)?;
            mapping.push((T::KIND, elem.key(), store.insert(&elem)?));
        }
    }
    Ok(())
}

pub fn convert_yomitan_dictionary(dir: &str) -> Result<DictionaryPackage, String> {
    let mut mapping: Vec<(QueryKindKey, String, u64)> = Vec::new();
    let mut store = UnifiedStoreBuilder::new()?;

    // TODO: why does dict 08/09 have so many empty keys? need to look into fixing importer

    load_typed_banks::<&str, Term>(dir, "term_bank_", "Term", &mut store, &mut mapping)?;
    load_typed_banks::<&str, Kanji>(dir, "kanji_bank_", "Kanji", &mut store, &mut mapping)?;
    load_typed_banks::<&str, Tag>(dir, "tag_bank_", "Tag", &mut store, &mut mapping)?;
    load_typed_banks::<&str, TermMeta>(dir, "term_meta_bank_", "Term meta", &mut store, &mut mapping)?;
    load_typed_banks::<&str, KanjiMeta>(dir, "kanji_meta_bank_", "Kanji meta", &mut store, &mut mapping)?;
    import_files(dir, dir, &mut store, &mut mapping)?;

    Ok(DictionaryPackage {
        fst: UnifiedFstIndex::build(mapping)?.as_bytes().to_vec(),
        data: store.finalize()?,
    })
}