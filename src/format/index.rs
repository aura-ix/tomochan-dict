use super::types::{TermId, KanjiId, TagId, TermMetaId, KanjiMetaId, LookupResult, TermLookupResult, DataType};
use super::store::UnifiedStore;
use crate::schema::{Term, Kanji, Tag, TermMeta, KanjiMeta};
use fst::{Map, MapBuilder, IntoStreamer, Streamer};
use std::collections::HashMap;

pub struct UnifiedFstIndex {
    fst_map: Map<Vec<u8>>,
}

impl UnifiedFstIndex {
    fn make_composite_key(data_type: DataType, key: &str, index: u64) -> Vec<u8> {
        let mut composite = Vec::new();
        composite.push(data_type.as_byte());
        composite.extend_from_slice(key.as_bytes());
        composite.push(0);
        composite.extend_from_slice(&index.to_le_bytes());
        composite
    }

    fn extract_key_from_composite(key: &[u8]) -> &str {
        if key.is_empty() {
            return "";
        }

        let key_bytes = &key[1..];
        if let Some(pos) = key_bytes.iter().position(|&b| b == 0) {
            std::str::from_utf8(&key_bytes[..pos]).unwrap_or("")
        } else {
            std::str::from_utf8(key_bytes).unwrap_or("")
        }
    }

    pub fn build(typed_mappings: Vec<(DataType, HashMap<String, Vec<u64>>)>) -> Result<Self, String> {
        let mut all_entries: Vec<(Vec<u8>, u64)> = Vec::new();
        
        for (data_type, key_to_ids) in typed_mappings {
            for (key, ids) in key_to_ids.iter() {
                for (index, &id) in ids.iter().enumerate() {
                    let composite_key = Self::make_composite_key(data_type, key, index as u64);
                    all_entries.push((composite_key, id));
                }
            }
        }

        all_entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut fst_builder = MapBuilder::memory();
        
        for (key, id) in all_entries {
            fst_builder.insert(&key, id)
                .map_err(|e| format!("Failed to insert key into FST: {}", e))?;
        }

        let fst_bytes = fst_builder.into_inner()
            .map_err(|e| format!("FST build failed: {}", e))?;
        let fst_map = Map::new(fst_bytes)
            .map_err(|e| format!("FST creation failed: {}", e))?;

        Ok(Self { fst_map })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.fst_map.as_fst().as_bytes()
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let fst_map = Map::new(bytes)
            .map_err(|e| format!("FST creation failed: {}", e))?;
        Ok(Self { fst_map })
    }

    pub fn lookup(&self, data_type: DataType, key: &str) -> Vec<u64> {
        let mut results = Vec::new();
        
        let mut prefix = Vec::new();
        prefix.push(data_type.as_byte());
        prefix.extend_from_slice(key.as_bytes());
        prefix.push(0);
        
        let mut stream = self.fst_map.range().ge(&prefix).into_stream();
        
        while let Some((composite_key, id_value)) = stream.next() {
            if composite_key.is_empty() || composite_key[0] != data_type.as_byte() {
                break;
            }
            
            let found_key = Self::extract_key_from_composite(composite_key);
            if found_key != key {
                break;
            }
            
            results.push(id_value);
        }
        
        results
    }

    pub fn lookup_prefix(&self, data_type: DataType, prefix: &str) -> Vec<(String, u64)> {
        let mut results = Vec::new();
        
        let mut search_prefix = Vec::new();
        search_prefix.push(data_type.as_byte());
        search_prefix.extend_from_slice(prefix.as_bytes());
        
        let mut stream = self.fst_map.range().ge(&search_prefix).into_stream();
        
        while let Some((composite_key, id_value)) = stream.next() {
            if composite_key.is_empty() || composite_key[0] != data_type.as_byte() {
                break;
            }
            
            let found_key = Self::extract_key_from_composite(composite_key);
            if !found_key.starts_with(prefix) {
                break;
            }
            
            results.push((found_key.to_string(), id_value));
        }
        
        results
    }

    pub fn keys(&self, data_type: DataType) -> Vec<String> {
        let mut keys = Vec::new();
        let type_byte = data_type.as_byte();
        
        let start_key = vec![type_byte];
        let mut stream = self.fst_map.range().ge(&start_key).into_stream();
        let mut last_key: Option<String> = None;
        
        while let Some((composite_key, _)) = stream.next() {
            if composite_key.is_empty() || composite_key[0] != type_byte {
                break;
            }
            
            let key = Self::extract_key_from_composite(composite_key).to_string();
            if last_key.as_ref() != Some(&key) {
                keys.push(key.clone());
                last_key = Some(key);
            }
        }
        
        keys
    }
}

pub struct UnifiedIndex {
    fst: UnifiedFstIndex,
    store: UnifiedStore,
}

impl UnifiedIndex {
    pub fn new(fst: UnifiedFstIndex, store: UnifiedStore) -> Self {
        Self { fst, store }
    }
    
    fn lookup_ids(&self, data_type: DataType, key: &str) -> Vec<u64> {
        self.fst.lookup(data_type, key)
    }
    
    fn lookup_full<T, ID, F>(&mut self, data_type: DataType, key: &str, id_wrapper: F, getter: fn(&mut UnifiedStore, ID) -> Result<T, String>) -> Vec<T>
    where
        F: Fn(u64) -> ID,
    {
        self.lookup_ids(data_type, key)
            .into_iter()
            .filter_map(|id| getter(&mut self.store, id_wrapper(id)).ok())
            .collect()
    }
    
    pub fn lookup_terms(&self, term: &str) -> Vec<TermLookupResult> {
        self.lookup_ids(DataType::Term, term)
            .into_iter()
            .map(|id| TermLookupResult::new(TermId(id), 0.0))
            .collect()
    }

    pub fn lookup_terms_prefix(&self, prefix: &str) -> Vec<TermLookupResult> {
        self.fst.lookup_prefix(DataType::Term, prefix)
            .into_iter()
            .map(|(_, id)| TermLookupResult::new(TermId(id), 0.0))
            .collect()
    }

    pub fn get_term(&mut self, id: TermId) -> Result<Term, String> {
        self.store.get_term(id)
    }

    pub fn lookup_terms_full(&mut self, term: &str) -> Vec<Term> {
        self.lookup_full(DataType::Term, term, TermId, UnifiedStore::get_term)
    }

    pub fn lookup_terms_prefix_full(&mut self, prefix: &str) -> Vec<Term> {
        self.lookup_terms_prefix(prefix)
            .into_iter()
            .filter_map(|r| self.get_term(r.term_id).ok())
            .collect()
    }

    pub fn term_keys(&self) -> Vec<String> {
        self.fst.keys(DataType::Term)
    }

    pub fn lookup_kanji(&self, character: &str) -> Vec<LookupResult<KanjiId>> {
        self.lookup_ids(DataType::Kanji, character)
            .into_iter()
            .map(|id| LookupResult { id: KanjiId(id) })
            .collect()
    }

    pub fn get_kanji(&mut self, id: KanjiId) -> Result<Kanji, String> {
        self.store.get_kanji(id)
    }

    pub fn lookup_kanji_full(&mut self, character: &str) -> Vec<Kanji> {
        self.lookup_full(DataType::Kanji, character, KanjiId, UnifiedStore::get_kanji)
    }

    pub fn lookup_tag(&mut self, name: &str) -> Option<Tag> {
        self.lookup_ids(DataType::Tag, name)
            .first()
            .and_then(|&id| self.store.get_tag(TagId(id)).ok())
    }

    pub fn lookup_term_meta(&self, term: &str) -> Vec<LookupResult<TermMetaId>> {
        self.lookup_ids(DataType::TermMeta, term)
            .into_iter()
            .map(|id| LookupResult { id: TermMetaId(id) })
            .collect()
    }

    pub fn get_term_meta(&mut self, id: TermMetaId) -> Result<TermMeta, String> {
        self.store.get_term_meta(id)
    }

    pub fn lookup_term_meta_full(&mut self, term: &str) -> Vec<TermMeta> {
        self.lookup_full(DataType::TermMeta, term, TermMetaId, UnifiedStore::get_term_meta)
    }

    pub fn lookup_kanji_meta(&self, character: &str) -> Vec<LookupResult<KanjiMetaId>> {
        self.lookup_ids(DataType::KanjiMeta, character)
            .into_iter()
            .map(|id| LookupResult { id: KanjiMetaId(id) })
            .collect()
    }

    pub fn get_kanji_meta(&mut self, id: KanjiMetaId) -> Result<KanjiMeta, String> {
        self.store.get_kanji_meta(id)
    }

    pub fn lookup_kanji_meta_full(&mut self, character: &str) -> Vec<KanjiMeta> {
        self.lookup_full(DataType::KanjiMeta, character, KanjiMetaId, UnifiedStore::get_kanji_meta)
    }
    
    pub fn lookup_extra_file(&self, path: &str) -> Option<u64> {
        self.lookup_ids(DataType::ExtraFile, path).first().copied()
    }
    
    pub fn get_extra_file(&mut self, offset: u64) -> Result<Vec<u8>, String> {
        self.store.get_extra_file(offset)
    }
    
    pub fn list_extra_files(&self) -> Vec<String> {
        self.fst.keys(DataType::ExtraFile)
    }
}
