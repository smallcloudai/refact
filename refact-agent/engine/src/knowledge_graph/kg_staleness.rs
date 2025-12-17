use std::collections::HashSet;
use std::path::PathBuf;
use chrono::{NaiveDate, Utc};

use super::kg_structs::KnowledgeGraph;

#[derive(Debug, Default)]
pub struct StalenessReport {
    pub orphan_file_refs: Vec<(PathBuf, Vec<String>)>,
    pub orphan_docs: Vec<PathBuf>,
    pub stale_by_age: Vec<(PathBuf, i64)>,
    pub past_review: Vec<PathBuf>,
    pub deprecated_ready_to_archive: Vec<PathBuf>,
    pub stale_trajectories: Vec<PathBuf>,
}

impl KnowledgeGraph {
    pub fn check_staleness(&self, max_age_days: i64, trajectory_max_age_days: i64) -> StalenessReport {
        let mut report = StalenessReport::default();
        let today = Utc::now().date_naive();

        for doc in self.docs.values() {
            let kind = doc.frontmatter.kind_or_default();

            if let Some(created) = &doc.frontmatter.created {
                if let Ok(created_date) = NaiveDate::parse_from_str(created, "%Y-%m-%d") {
                    let age_days = (today - created_date).num_days();

                    if kind == "trajectory" && age_days > trajectory_max_age_days {
                        report.stale_trajectories.push(doc.path.clone());
                        continue;
                    }

                    if age_days > max_age_days && doc.frontmatter.is_active() {
                        report.stale_by_age.push((doc.path.clone(), age_days));
                    }
                }
            }

            if let Some(review_after) = &doc.frontmatter.review_after {
                if let Ok(review_date) = NaiveDate::parse_from_str(review_after, "%Y-%m-%d") {
                    if today > review_date && doc.frontmatter.is_active() {
                        report.past_review.push(doc.path.clone());
                    }
                }
            }

            if doc.frontmatter.is_deprecated() {
                if let Some(deprecated_at) = &doc.frontmatter.deprecated_at {
                    if let Ok(deprecated_date) = NaiveDate::parse_from_str(deprecated_at, "%Y-%m-%d") {
                        let days_deprecated = (today - deprecated_date).num_days();
                        if days_deprecated > 60 {
                            report.deprecated_ready_to_archive.push(doc.path.clone());
                        }
                    }
                }
            }

            let missing_files: Vec<String> = doc.frontmatter.filenames.iter()
                .filter(|f| {
                    self.file_index.get(*f)
                        .and_then(|idx| self.graph.node_weight(*idx))
                        .map(|node| {
                            if let super::kg_structs::KgNode::FileRef { exists, .. } = node {
                                !exists
                            } else {
                                false
                            }
                        })
                        .unwrap_or(true)
                })
                .cloned()
                .collect();

            if !missing_files.is_empty() && doc.frontmatter.kind_or_default() == "code" {
                report.orphan_file_refs.push((doc.path.clone(), missing_files));
            }
        }

        let docs_with_links: HashSet<PathBuf> = self.docs.values()
            .flat_map(|d| d.frontmatter.links.iter())
            .filter_map(|link| self.docs.get(link))
            .map(|d| d.path.clone())
            .collect();

        for doc in self.docs.values() {
            if doc.frontmatter.tags.is_empty()
                && doc.frontmatter.filenames.is_empty()
                && doc.entities.is_empty()
                && !docs_with_links.contains(&doc.path)
                && doc.frontmatter.kind_or_default() != "trajectory"
            {
                report.orphan_docs.push(doc.path.clone());
            }
        }

        report
    }
}
