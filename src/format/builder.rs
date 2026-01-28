use super::store::UnifiedStoreBuilder;
use super::types::{Queryable, QueryKindKey};
use super::index::{UnifiedFstIndex, UnifiedIndex};
use crate::schema::{Dictionary, BINCODE_CONFIG};
use std::collections::HashMap;
use std::fs;
use bincode::{Encode, Decode};

#[derive(Encode, Decode)]
pub struct DictionaryPackage {
    fst: Vec<u8>,
    data: Vec<u8>,
}

impl DictionaryPackage {
    pub fn save(&self, path: &str) -> Result<(), String> {
        let encoded = bincode::encode_to_vec(self, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to encode package: {}", e))?;
        
        std::fs::write(path, encoded)
            .map_err(|e| format!("Failed to write package file: {}", e))?;
        
        Ok(())
    }
    
    pub fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read package file: {}", e))?;
        
        let (package, _) = bincode::decode_from_slice(&data, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to decode package: {}", e))?;
        
        Ok(package)
    }
}

pub struct DictionaryIndexBuilder {
    store_builder: Option<UnifiedStoreBuilder>,
    
    term_keys: HashMap<String, Vec<u64>>,
    kanji_keys: HashMap<String, Vec<u64>>,
    tag_keys: HashMap<String, Vec<u64>>,
    term_meta_keys: HashMap<String, Vec<u64>>,
    kanji_meta_keys: HashMap<String, Vec<u64>>,
    file_keys: HashMap<String, Vec<u64>>,
    
    source_dir: Option<String>,
}

impl DictionaryIndexBuilder {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            store_builder: Some(UnifiedStoreBuilder::new()?),
            term_keys: HashMap::new(),
            kanji_keys: HashMap::new(),
            tag_keys: HashMap::new(),
            term_meta_keys: HashMap::new(),
            kanji_meta_keys: HashMap::new(),
            file_keys: HashMap::new(),
            source_dir: None,
        })
    }
    
    pub fn set_source_directory(&mut self, dir: String) {
        self.source_dir = Some(dir);
    }
    
    fn import_items<T>(
        items: Vec<T>,
        builder: &mut Option<UnifiedStoreBuilder>,
        keys: &mut HashMap<String, Vec<u64>>,
    ) -> Result<(), String>
    where
        T: Queryable + bincode::Encode,
    {
        let builder = builder.as_mut()
            .ok_or("builder already finalized")?;

        for item in items {
            let id = builder.insert(&item)?;
            keys.entry(item.key()).or_default().push(id);
        }

        Ok(())
    }

    pub fn import_dictionary(&mut self, dict: Dictionary) -> Result<(), String> {
        Self::import_items(dict.terms, &mut self.store_builder, &mut self.term_keys)?;
        Self::import_items(dict.kanji, &mut self.store_builder, &mut self.kanji_keys)?;
        Self::import_items(dict.tags, &mut self.store_builder, &mut self.tag_keys)?;
        Self::import_items(dict.term_meta, &mut self.store_builder, &mut self.term_meta_keys)?;
        Self::import_items(dict.kanji_meta, &mut self.store_builder, &mut self.kanji_meta_keys)?;
        Ok(())
    }
    
    fn collect_files(&mut self) -> Result<(), String> {
        if let Some(source_dir) = &self.source_dir {
            Self::collect_files_recursive(source_dir, source_dir, &mut self.store_builder, &mut self.file_keys)?;
        }
        Ok(())
    }
    
    fn collect_files_recursive(
        base_dir: &str,
        current_dir: &str,
        builder: &mut Option<UnifiedStoreBuilder>,
        keys: &mut HashMap<String, Vec<u64>>,
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
                
                let builder_ref = builder.as_mut().ok_or("builder already finalized")?;
                let offset = builder_ref.insert(&file_data)?;
                keys.entry(rel_path_str).or_default().push(offset);
                
            } else if path.is_dir() {
                let dir_str = path.to_str().ok_or("Invalid directory path")?;
                Self::collect_files_recursive(base_dir, dir_str, builder, keys)?;
            }
        }
        
        Ok(())
    }

    pub fn finalize_to_single_file(mut self, output_path: &str) -> Result<(), String> {
        println!("Collecting files...");
        self.collect_files()?;
        if !self.file_keys.is_empty() {
            println!("  Found {} file(s)", self.file_keys.len());
        }
        
        let store_builder = self.store_builder.take()
            .ok_or("builder already finalized")?;
        let data = store_builder.finalize()?;
        
        println!("Building unified FST index...");
        let typed_mappings = vec![
            (QueryKindKey::Term, self.term_keys),
            (QueryKindKey::Kanji, self.kanji_keys),
            (QueryKindKey::Tag, self.tag_keys),
            (QueryKindKey::TermMeta, self.term_meta_keys),
            (QueryKindKey::KanjiMeta, self.kanji_meta_keys),
            (QueryKindKey::File, self.file_keys),
        ];
        
        let fst = UnifiedFstIndex::build(typed_mappings)?;
        
        let package = DictionaryPackage {
            fst: fst.as_bytes().to_vec(),
            data,
        };
        
        fn fmt_size(bytes: usize) -> String {
            if bytes < 1024 {
                format!("{} B", bytes)
            } else if bytes < 1024 * 1024 {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            } else {
                format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
            }
        }
        
        println!("\n=== Package Size Breakdown ===");
        println!("FST Index:      {:>12}", fmt_size(package.fst.len()));
        println!("Data (zstd):    {:>12}", fmt_size(package.data.len()));
        println!("Total:          {:>12}", fmt_size(package.fst.len() + package.data.len()));
        println!("==============================");

        println!("\nSaving to {}...", output_path);
        package.save(output_path)?;
        
        println!("Conversion complete!");
        Ok(())
    }
}

pub fn open_package(package_file: &str) -> Result<UnifiedIndex, String> {
    let package = DictionaryPackage::load(package_file)?;
    
    let fst = UnifiedFstIndex::from_bytes(package.fst)?;
    let store = super::store::UnifiedStore::new(package.data)?;
    let unified_index = UnifiedIndex::new(fst, store);
    
    Ok(unified_index)
}