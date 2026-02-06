use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::schema::BINCODE_CONFIG;
use super::container::{write_container, ContainerMeta};
use std::fs::File;
use std::fs;

use super::deinflector::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformSet {
    pub dict_tags: Vec<String>,
    // for a subtags x, y of A, x and y will match an A constraint, but x will not match y, and vice versa, and A will not match x or y constraints
    pub subtags: HashMap<String, HashSet<String>>,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub suffix: String,
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub accept: State,
    pub produce: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub name: String,
    pub desc: Option<String>,
    pub is_final: bool,
    pub rules: Vec<Rule>
}

pub fn convert_deinflector(src_path: &str, dst_path: &str, meta: ContainerMeta) -> Result<(), String> {
    let json = fs::read_to_string(src_path)
        .map_err(|e| format!("Failed to read source file: {}", e))?;
    let s: TransformSet = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse source file: {}", e))?;

    let mut next_leaf_id: u64 = 1;
    let mut key_to_id: HashMap<String, u64> = HashMap::new();

    let mut transform_meta = Vec::new();
    for transform in &s.transforms {
        transform_meta.push(TransformMeta {
            name: transform.name.clone(),
            desc: transform.desc.clone(),
            is_final: transform.is_final,
        });
    }

    // TODO: handle subtags by making a concrete tag for all named tags, but in rule inputs,
    // when accepting a parent tag, you need to accept all of it's children as well. essentially
    // we need some kind of mapping from tag name to accept tag. for rule outputs, just output the named tags

    for transform in &s.transforms {
        for rule in &transform.rules {
            for tag in rule.accept.tags.iter().chain(rule.produce.tags.iter()) {
                if !key_to_id.contains_key(tag) {
                    if next_leaf_id == 0 {
                        return Err("too many tags".to_string());
                    }
                    key_to_id.insert(tag.clone(), next_leaf_id);
                    next_leaf_id <<= 1;
                }
            }
        }
    }

    let mut suffix_map: HashMap<Vec<u8>, Vec<Production>> = HashMap::new();
    let mut suffix_lengths = HashSet::new();

    for (transform_idx, transform) in s.transforms.iter().enumerate()  {
        for rule in &transform.rules {
            let produce_tags = rule.produce.tags.iter()
                .map(|st| key_to_id[st])
                .fold(0, |a, b| a | b);

            let accept_tags = if rule.accept.tags.is_empty() {
                u64::MAX
            } else {
                let mut acc = 0;
                for tag in &rule.accept.tags {
                    acc |= key_to_id[tag];
                }

                acc
            };

            // TODO: check for duplicate entries with hashset?
            suffix_map.entry(rule.accept.suffix.as_bytes().to_vec()).or_default().push(
                Production {
                    transform_idx,
                    accept_tags,
                    produce_tags,
                    produce_suffix: rule.produce.suffix.as_bytes().to_vec(),
                }
            );
            suffix_lengths.insert(rule.accept.suffix.len());
        }
    }

    let mut suffix_lengths: Vec<usize> = suffix_lengths.into_iter().collect();
    suffix_lengths.sort();

    let mut encoded: Vec<u8> = bincode::encode_to_vec(
        Deinflector {
            transform_meta,
            suffix_map,
            suffix_lengths,
        },
        BINCODE_CONFIG
    ).map_err(|e| format!("Failed to encode deinflector: {}", e))?;
    
    let mut file = File::create(dst_path)
        .map_err(|e| format!("Failed to open package file: {}", e))?;

    write_container::<Deinflector, _>(&mut file, meta, &encoded)
        .map_err(|e| format!("Failed to write container: {}", e))
}