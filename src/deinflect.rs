use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEPTH_LIMIT: u32 = 10;

// TODO: move the input to the schema crate, convert deconjugator to FST
// TODO: convert this to a more efficient binary format, prove graph is acyclic
// TODO: a lot of performance in both speed and memory usage left on the table here
// TODO: validate using rules from yomitan dictionary, seems like we can just match with condition names

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformSet {
    pub dict_tags: Vec<String>,
    pub subtags: HashMap<String, Vec<String>>,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub suffix: String,
    pub tags: Vec<String>,
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
    pub rule_chain: Vec<String>,
    pub tags: Vec<String>,
}

impl TransformSet {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn deinflect(&self, term: &str) -> Vec<DeinflectionResult> {
        let mut results = Vec::new();
        
        results.push(DeinflectionResult {
            term: term.to_string(),
            rule_chain: vec![],
            tags: vec![],
        });

        self.deinflect_recursive(term, vec![], vec![], &mut results, 0);
        
        results
    }

    fn deinflect_recursive(
        &self,
        term: &str,
        rule_chain: Vec<String>,
        tags: Vec<String>,
        results: &mut Vec<DeinflectionResult>,
        depth: u32,
    ) {
        if depth > DEPTH_LIMIT {
            return
        }

        for transform in &self.transforms {
            for rule in &transform.rules {
                let accept = &rule.accept;
                let produce = &rule.produce;

                if term.ends_with(&accept.suffix) && (tags.is_empty() || accept.tags.iter().any(|x| tags.iter().any(|y| x == y))) {
                    let deinflected = format!("{}{}", term.strip_suffix(&accept.suffix).unwrap(), produce.suffix);

                    let mut new_chain = rule_chain.clone();
                    new_chain.push(transform.name.clone());

                    results.push(DeinflectionResult {
                        term: deinflected.clone(),
                        rule_chain: new_chain.clone(),
                        tags: produce.tags.clone(),
                    }); 

                    if transform.is_final {
                        continue
                    }
                    self.deinflect_recursive(
                        &deinflected,
                        new_chain.clone(),
                        produce.tags.clone(),
                        results,
                        depth + 1,
                    )
                }
            }
        }
    }
}