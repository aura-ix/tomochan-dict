use super::types::QueryKindKey;
use fst::{Map, MapBuilder, IntoStreamer, Streamer};
use fst::map::OpBuilder;
use std::fs::File;
use memmap2::Mmap;

// TODO: use the fst::verify method. we probably need our format trait to have verify/read/write
// TODO: validate all keys

enum BackingStore {
    Memory(Vec<u8>),
    Mmap((Mmap, usize, usize)),
}

impl AsRef<[u8]> for BackingStore {
    fn as_ref(&self) -> &[u8] {
        match self {
            BackingStore::Memory(v) => v.as_ref(),
            BackingStore::Mmap(m) => &m.0[m.1..m.2],
        }
    }
}

pub struct DictionaryIndex {
    fst_map: Map<BackingStore>,
}

impl DictionaryIndex {
    fn make_composite_key(data_type: QueryKindKey, key: &str, index: u32) -> Vec<u8> {
        let mut composite = Vec::new();
        composite.push(data_type.as_byte());
        composite.extend_from_slice(key.as_bytes());
        composite.push(0);
        composite.extend_from_slice(&index.to_be_bytes());
        composite
    }

    fn destructure_key(key: &[u8]) -> Result<(QueryKindKey, &str, u32), String> {
        // TODO: we need some way to validate loaded FSTs/deinflection rulesets etc at import time
        if key.len() < 5 {
            return Err("malformed key".to_string());
        }

        let kind = QueryKindKey::from_byte(key[0])?;
        let key_str = std::str::from_utf8(&key[1..key.len()-5])
            .map_err(|e| format!("invalid utf8 key: {}", e))?;
        let idx = u32::from_be_bytes(key[key.len()-4..].try_into().unwrap());

        Ok((kind, key_str, idx))
    }

    pub fn build(mut mappings: Vec<(QueryKindKey, String, u64)>) -> Result<Self, String> {
        mappings.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));

        let mut fst_builder = MapBuilder::memory();
        let mut prev: Option<(QueryKindKey, &str)> = None;
        let mut reps: u32 = 0;
        for (kind, key, offset) in &mappings {
            if prev != Some((*kind, key)) {
                reps = 0;
            }
            fst_builder.insert(
                &Self::make_composite_key(*kind, key, reps),
                *offset,
            ).map_err(|e| format!("Failed to insert key into FST: {} {:#?} {:#?} {} {:#?}", e, prev, (kind, key), reps, &Self::make_composite_key(*kind, key, reps)))?;
            prev = Some((*kind, key));
            reps += 1;
        }

        let fst_bytes = fst_builder.into_inner()
            .map_err(|e| format!("FST build failed: {}", e))?;
        let fst_map = Map::new(BackingStore::Memory(fst_bytes))
            .map_err(|e| format!("FST creation failed: {}", e))?;

        Ok(Self { fst_map })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.fst_map.as_fst().as_bytes()
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let fst_map = Map::new(BackingStore::Memory(bytes))
            .map_err(|e| format!("FST creation failed: {}", e))?;
        Ok(Self { fst_map })
    }

    pub fn load_mmap(path: &str, offset: u64, len: u64) -> Result<Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("Failed to open FST file: {}", e))?;

        let offset: usize = offset.try_into()
            .map_err(|_| "dictionary file too large for 32 bit platform")?;

        let len: usize = offset.try_into()
            .map_err(|_| "dictionary file too large for 32 bit platform")?;
        
        let mmap = unsafe { 
            Mmap::map(&file)
                .map_err(|e| format!("Failed to mmap file: {}", e))?
        };
        
        let fst_map = Map::new(BackingStore::Mmap((mmap, offset, offset+len)))
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
            
            // TODO: remove unwrap
            let (_, found_key, _) = Self::destructure_key(composite_key).unwrap();
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
            
            // TODO: remove unwrap
            let (_, key, _) = Self::destructure_key(composite_key).unwrap();
            if last_key.as_deref() != Some(key) {
                keys.push(key.to_string());
                last_key = Some(key.to_string());
            }
        }
        
        keys
    }

    pub fn unique_terms_in_collection(indexes: &[DictionaryIndex]) -> Result<usize, String> {
        let mut op = OpBuilder::new();
        let prefix = QueryKindKey::Term.as_byte();
        for index in indexes {
            op.push(index.fst_map.range().ge(&[prefix]).lt(&[prefix + 1]));
        }

        let mut stream = op.union();
        let mut count = 0;
        while let Some((composite_key, _)) = stream.next() {
            let (_, _, rep) = Self::destructure_key(composite_key)?;
            if rep == 0 {
                count += 1;
            }
        }

        Ok(count)
    }
}