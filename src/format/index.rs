use super::types::{QueryKindKey, Queryable};
use super::store::UnifiedStore;
use fst::{Map, MapBuilder, IntoStreamer, Streamer};
use std::collections::HashMap;

pub struct UnifiedFstIndex {
    fst_map: Map<Vec<u8>>,
}

impl UnifiedFstIndex {
    fn make_composite_key(data_type: QueryKindKey, key: &str, index: u64) -> Vec<u8> {
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

    pub fn build(typed_mappings: Vec<(QueryKindKey, HashMap<String, Vec<u64>>)>) -> Result<Self, String> {
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

    pub fn lookup(&self, data_type: QueryKindKey, key: &str) -> Vec<u64> {
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

    pub fn keys(&self, data_type: QueryKindKey) -> Vec<String> {
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

    pub fn query<T: bincode::Decode<()> + Queryable>(&mut self, key: &str) -> Vec<T> {
        self.fst.lookup(T::KIND, key)
            .into_iter()
            .filter_map(|id| self.store.get::<T>(id).ok())
            .collect() // TODO: handle errors
    }

    pub fn query_file(&mut self, key: &str) -> Option<Vec<u8>> {
        self.fst.lookup(QueryKindKey::File, key)
            .into_iter()
            .filter_map(|id| self.store.get::<Vec<u8>>(id).ok())
            .next() // TODO: handle errors
    }
    // TODO:replace with a generic method
    pub fn term_keys(&self) -> Vec<String> {
        self.fst.keys(QueryKindKey::Term)
    }
}
