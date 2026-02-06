use super::store::StoreBuilder;
use super::types::{Queryable, QueryKindKey};
use super::index::DictionaryIndex;
use super::container::{ContainerHeader, write_container};
use super::dictionary::DictionaryHeader;
use crate::schema::{Term, Tag, Kanji, KanjiMeta, TermMeta, BINCODE_CONFIG};
use crate::schema::JsonParseable;
use std::fs;
use std::fs::File;
use std::path::Path;

// TODO: fix error handling throughout codebase

fn import_files(
    base_dir: &str,
    current_dir: &str,
    store: &mut StoreBuilder,
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

pub fn load_typed_banks<P, T>(dir: P, prefix: &str, type_name: &str, store: &mut StoreBuilder,
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

pub fn convert_yomitan_dictionary(src_dir: &str, dst: &str, header: ContainerHeader) -> Result<(), String> {
    let mut mapping: Vec<(QueryKindKey, String, u64)> = Vec::new();
    let mut store = StoreBuilder::new()?;

    // TODO: why does dict 08/09 have so many empty keys? need to look into fixing importer

    load_typed_banks::<&str, Term>(src_dir, "term_bank_", "Term", &mut store, &mut mapping)?;
    load_typed_banks::<&str, Kanji>(src_dir, "kanji_bank_", "Kanji", &mut store, &mut mapping)?;
    load_typed_banks::<&str, Tag>(src_dir, "tag_bank_", "Tag", &mut store, &mut mapping)?;
    load_typed_banks::<&str, TermMeta>(src_dir, "term_meta_bank_", "Term meta", &mut store, &mut mapping)?;
    load_typed_banks::<&str, KanjiMeta>(src_dir, "kanji_meta_bank_", "Kanji meta", &mut store, &mut mapping)?;
    import_files(src_dir, src_dir, &mut store, &mut mapping)?;

    let fst = DictionaryIndex::build(mapping)?.as_bytes().to_vec();
    let store = store.finalize()?;

    let mut encoded: Vec<u8> = bincode::encode_to_vec(
        DictionaryHeader {
            fst_len: fst.len() as u64,
            store_len: store.len() as u64,
        },
        BINCODE_CONFIG
    ).map_err(|e| format!("Failed to encode package: {}", e))?;

    encoded.extend(fst);
    encoded.extend(store);
    
    let mut file = File::create(dst)
        .map_err(|e| format!("Failed to open package file: {}", e))?;

    write_container(&mut file, header, &encoded)
        .map_err(|e| format!("Failed to write package file: {}", e))?;

    Ok(())
}