use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use bincode::{Encode, Decode};

const DEPTH_LIMIT: u32 = 10;

// TODO: validate using rules from yomitan dictionary, seems like we can just match with condition names

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

#[derive(Debug, Clone)]
pub struct DeinflectionResult {
    pub term: String,
    pub rule_chain: Vec<usize>,
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

#[derive(Debug, Clone, Encode, Decode)]
pub struct Deinflector {
    transform_meta: Vec<TransformMeta>,
    suffix_map: HashMap<Vec<u8>, Vec<Production>>,
    suffix_lengths: Vec<usize>,
}

impl Deinflector {
    pub fn make(s: TransformSet) -> Result<Self, String> {
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

        Ok(Self {
            transform_meta,
            suffix_map,
            suffix_lengths,
        })
    }

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

impl TransformSet {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}