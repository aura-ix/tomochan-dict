use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use crate::schema::BINCODE_CONFIG;
use bincode::{Encode, Decode};
use super::container::{ContainerFormat, Role};

const DEPTH_LIMIT: u32 = 10;

// TODO: validate using rules from yomitan dictionary, seems like we can just match with condition names

#[derive(Debug, Clone, Encode, Decode)]
pub struct Deinflector {
    pub transform_meta: Vec<TransformMeta>,
    pub suffix_map: HashMap<Vec<u8>, Vec<Production>>,
    pub suffix_lengths: Vec<usize>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct TransformMeta {
    pub name: String,
    pub desc: Option<String>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Production {
    pub transform_idx: usize,
    pub accept_tags: u64,
    pub produce_tags: u64,
    pub produce_suffix: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DeinflectionResult {
    pub term: String,
    pub rule_chain: Vec<usize>,
}

impl Deinflector {
    pub fn deinflect_recursive(
        &self,
        term: &[u8],
        rule_chain: Vec<usize>,
        tags: u64,
        results: &mut Vec<DeinflectionResult>,
        depth: u32,
    ) {
        if depth > DEPTH_LIMIT {
            return
        }

        for &length in &self.suffix_lengths {
            if length > term.len() {
                // lengths are in sorted order, so just break
                break;
            }
            let suffix_idx = term.len() - length;
            if let Some(result) = self.suffix_map.get(&term[suffix_idx..]) {
                for result in result {
                    if tags & result.accept_tags == 0 {
                        continue
                    }

                    let mut new_term = Vec::new();
                    new_term.extend_from_slice(&term[..suffix_idx]);
                    new_term.extend_from_slice(&result.produce_suffix);

                    let mut rule_chain = rule_chain.clone();
                    rule_chain.push(result.transform_idx);

                    let Ok(term_str) = str::from_utf8(&new_term) else {
                        // TODO: emit a warning somehow, this is a fault of the deinflection data
                        continue
                    };

                    results.push(DeinflectionResult {
                        term: term_str.to_string(),
                        rule_chain: rule_chain.clone(),
                    });

                    if self.transform_meta[result.transform_idx].is_final {
                        continue
                    }
                    
                    self.deinflect_recursive(
                        &new_term,
                        rule_chain,
                        result.produce_tags,
                        results,
                        depth + 1,
                    );
                }
            }
        }
    }

    pub fn deinflect(&self, term: &str) -> Vec<DeinflectionResult> {
        let mut results = Vec::new();
        
        results.push(DeinflectionResult {
            term: term.to_string(),
            rule_chain: vec![],
        });
        self.deinflect_recursive(term.as_bytes(), vec![], u64::MAX, &mut results, 0);
        results
    }
}

impl ContainerFormat for Deinflector {
    fn role() -> Role {
        Role::Deinflector
    }

    fn min_role_version() -> u64 {
        0
    }

    fn role_version() -> u64 {
        0
    }

    fn load(path: &str, payload_offset: u64, verify: bool) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to read package file: {}", e))?;

        file.seek(SeekFrom::Start(payload_offset))
            .map_err(|e| format!("Failed to seek to dictionary contents: {}", e))?;

        let deinflector: Self = bincode::decode_from_std_read(&mut file, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to decode header: {}", e))?;

        let file_meta = file.metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;

        let pos = file.stream_position()
            .map_err(|e| format!("Failed to get stream position: {}", e))?;

        if pos != file_meta.len() {
            return Err(format!("expected file length {}, found {}", file_meta.len(), pos))
        }

        // TODO: verify rules, utf8, meta indexes

        Ok(deinflector)
    }
}