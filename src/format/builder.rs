use super::store::UnifiedStoreBuilder;
use super::types::{Queryable, QueryKindKey};
use super::index::{UnifiedFstIndex};
use super::container::{ContainerHeader, write_container};
use crate::schema::{Dictionary, BINCODE_CONFIG};
use std::fs;
use std::fs::File;
use bincode::{Encode, Decode};
use std::io::Read;

// TODO: mmap fst
// TODO: fix error handling
// TODO: avoid compressing files that are already compressed

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
        // TODO: update to use container
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read package file: {}", e))?;
        
        let (package, _) = bincode::decode_from_slice(&data, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to decode package: {}", e))?;
        
        Ok(package)
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

fn import_items<T>(
    items: &[T],
    store: &mut UnifiedStoreBuilder,
    mapping: &mut Vec<(QueryKindKey, String, u64)>,
) -> Result<(), String>
where
    T: Queryable + bincode::Encode,
{
    for item in items {
        mapping.push((T::KIND, item.key(), store.insert(&item)?));
    }

    Ok(())
}

pub fn convert_yomitan_dictionary(dir: &str) -> Result<DictionaryPackage, String> {
    let mut mapping: Vec<(QueryKindKey, String, u64)> = Vec::new();
    let mut store = UnifiedStoreBuilder::new()?;
    // TODO: dictionary::from_directory implementation should probably live here and not in schema
    let dictionary = Dictionary::from_directory(dir)?;

    import_items(&dictionary.terms, &mut store, &mut mapping)?;
    import_items(&dictionary.kanji, &mut store, &mut mapping)?;
    import_items(&dictionary.tags, &mut store, &mut mapping)?;
    import_items(&dictionary.term_meta, &mut store, &mut mapping)?;
    import_items(&dictionary.kanji_meta, &mut store, &mut mapping)?;
    import_files(dir, dir, &mut store, &mut mapping)?;

    Ok(DictionaryPackage {
        fst: UnifiedFstIndex::build(mapping)?.as_bytes().to_vec(),
        data: store.finalize()?,
    })
}