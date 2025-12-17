use std::collections::{HashMap, HashSet};

use super::kg_structs::{KnowledgeDoc, KnowledgeGraph};

struct RelatedDoc {
    id: String,
    score: f64,
}

impl KnowledgeGraph {
    fn find_related(&self, doc_id: &str, max_results: usize) -> Vec<RelatedDoc> {
        let Some(doc) = self.docs.get(doc_id) else {
            return vec![];
        };

        let mut scores: HashMap<String, f64> = HashMap::new();

        for tag in &doc.frontmatter.tags {
            for related_id in self.docs_with_tag(tag) {
                if related_id != doc_id {
                    *scores.entry(related_id).or_insert(0.0) += 1.0;
                }
            }
        }

        for filename in &doc.frontmatter.filenames {
            for related_id in self.docs_referencing_file(filename) {
                if related_id != doc_id {
                    *scores.entry(related_id).or_insert(0.0) += 2.0;
                }
            }
        }

        for entity in &doc.entities {
            for related_id in self.docs_mentioning_entity(entity) {
                if related_id != doc_id {
                    *scores.entry(related_id).or_insert(0.0) += 1.5;
                }
            }
        }

        let mut results: Vec<RelatedDoc> = scores.into_iter()
            .filter(|(id, _)| self.docs.get(id).map(|d| d.frontmatter.is_active()).unwrap_or(false))
            .map(|(id, score)| RelatedDoc { id, score })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_results);
        results
    }

    pub fn expand_search_results(&self, initial_doc_ids: &[String], max_expansion: usize) -> Vec<String> {
        let mut all_ids: HashSet<String> = initial_doc_ids.iter().cloned().collect();
        let mut expanded: Vec<String> = vec![];

        for doc_id in initial_doc_ids {
            let related = self.find_related(doc_id, max_expansion);
            for rel in related {
                if !all_ids.contains(&rel.id) {
                    all_ids.insert(rel.id.clone());
                    expanded.push(rel.id);
                }
                if expanded.len() >= max_expansion {
                    break;
                }
            }
            if expanded.len() >= max_expansion {
                break;
            }
        }

        expanded
    }

    fn find_similar_docs(&self, tags: &[String], filenames: &[String], entities: &[String]) -> Vec<(String, f64)> {
        let mut scores: HashMap<String, f64> = HashMap::new();

        for tag in tags {
            for id in self.docs_with_tag(tag) {
                *scores.entry(id).or_insert(0.0) += 1.0;
            }
        }

        for filename in filenames {
            for id in self.docs_referencing_file(filename) {
                *scores.entry(id).or_insert(0.0) += 2.0;
            }
        }

        for entity in entities {
            for id in self.docs_mentioning_entity(entity) {
                *scores.entry(id).or_insert(0.0) += 1.5;
            }
        }

        let mut results: Vec<_> = scores.into_iter()
            .filter(|(id, _)| self.docs.get(id).map(|d| d.frontmatter.is_active()).unwrap_or(false))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn get_deprecation_candidates(&self, new_doc_tags: &[String], new_doc_filenames: &[String], new_doc_entities: &[String], exclude_id: Option<&str>) -> Vec<&KnowledgeDoc> {
        let similar = self.find_similar_docs(new_doc_tags, new_doc_filenames, new_doc_entities);

        similar.into_iter()
            .filter(|(id, score)| {
                *score >= 2.0 && exclude_id.map(|e| e != id).unwrap_or(true)
            })
            .filter_map(|(id, _)| {
                let doc = self.docs.get(&id)?;
                if doc.frontmatter.is_deprecated() || doc.frontmatter.is_archived() {
                    return None;
                }
                if doc.frontmatter.kind_or_default() == "trajectory" {
                    return None;
                }
                Some(doc)
            })
            .take(10)
            .collect()
    }
}
