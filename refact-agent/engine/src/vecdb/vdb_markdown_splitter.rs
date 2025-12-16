use std::path::PathBuf;
use std::sync::Arc;
use regex::Regex;
use tokio::sync::RwLock as ARwLock;

use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;
use crate::vecdb::vdb_structs::SplitResult;
use crate::ast::chunk_utils::official_text_hashing_function;

#[derive(Debug, Clone, Default)]
pub struct MarkdownFrontmatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl MarkdownFrontmatter {
    pub fn parse(content: &str) -> (Self, usize) {
        let mut frontmatter = Self::default();
        let mut end_offset = 0;

        if content.starts_with("---") {
            if let Some(end_idx) = content[3..].find("\n---") {
                let yaml_content = &content[3..3 + end_idx];
                end_offset = 3 + end_idx + 4;
                if content.len() > end_offset && content.as_bytes()[end_offset] == b'\n' {
                    end_offset += 1;
                }

                for line in yaml_content.lines() {
                    let line = line.trim();
                    if let Some(pos) = line.find(':') {
                        let key = line[..pos].trim();
                        let value = line[pos + 1..].trim().trim_matches('"').trim_matches('\'');
                        match key {
                            "title" => frontmatter.title = Some(value.to_string()),
                            "created" => frontmatter.created = Some(value.to_string()),
                            "updated" => frontmatter.updated = Some(value.to_string()),
                            "tags" => {
                                if value.starts_with('[') && value.ends_with(']') {
                                    frontmatter.tags = value[1..value.len() - 1]
                                        .split(',')
                                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else if !value.is_empty() {
                                    frontmatter.tags = vec![value.to_string()];
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        (frontmatter, end_offset)
    }
}

#[derive(Debug, Clone)]
struct MarkdownSection {
    heading_path: Vec<String>,
    content: String,
    start_line: usize,
    end_line: usize,
}

pub struct MarkdownFileSplitter {
    max_tokens: usize,
    overlap_lines: usize,
}

impl MarkdownFileSplitter {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens, overlap_lines: 3 }
    }

    pub async fn split(
        &self,
        doc: &Document,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Result<Vec<SplitResult>, String> {
        let text = doc.clone().get_text_or_read_from_disk(gcx).await.map_err(|e| e.to_string())?;
        let path = doc.doc_path.clone();
        let (frontmatter, content_start) = MarkdownFrontmatter::parse(&text);
        let frontmatter_lines = if content_start > 0 { text[..content_start].lines().count() } else { 0 };
        let content = &text[content_start..];
        let sections = self.extract_sections(content, frontmatter_lines);

        let mut results = Vec::new();

        if frontmatter.title.is_some() || !frontmatter.tags.is_empty() {
            let frontmatter_text = self.format_frontmatter_chunk(&frontmatter);
            if !frontmatter_text.is_empty() {
                results.push(SplitResult {
                    file_path: path.clone(),
                    window_text: frontmatter_text.clone(),
                    window_text_hash: official_text_hashing_function(&frontmatter_text),
                    start_line: 0,
                    end_line: frontmatter_lines.saturating_sub(1) as u64,
                    symbol_path: "frontmatter".to_string(),
                });
            }
        }

        for section in sections {
            results.extend(self.chunk_section(&section, &path));
        }
        Ok(results)
    }

    fn format_frontmatter_chunk(&self, fm: &MarkdownFrontmatter) -> String {
        let mut parts = Vec::new();
        if let Some(title) = &fm.title {
            parts.push(format!("Title: {}", title));
        }
        if !fm.tags.is_empty() {
            parts.push(format!("Tags: {}", fm.tags.join(", ")));
        }
        parts.join("\n")
    }

    fn extract_sections(&self, content: &str, line_offset: usize) -> Vec<MarkdownSection> {
        let heading_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        let code_fence_re = Regex::new(r"^```").unwrap();
        let mut sections = Vec::new();
        let mut current_heading_path: Vec<String> = Vec::new();
        let mut current_content = String::new();
        let mut current_start_line = line_offset;
        let mut in_code_block = false;
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let absolute_line = line_offset + idx;

            if code_fence_re.is_match(line) {
                in_code_block = !in_code_block;
                current_content.push_str(line);
                current_content.push('\n');
                continue;
            }

            if in_code_block {
                current_content.push_str(line);
                current_content.push('\n');
                continue;
            }

            if let Some(caps) = heading_re.captures(line) {
                if !current_content.trim().is_empty() {
                    sections.push(MarkdownSection {
                        heading_path: current_heading_path.clone(),
                        content: current_content.trim().to_string(),
                        start_line: current_start_line,
                        end_line: absolute_line.saturating_sub(1),
                    });
                }

                let level = caps.get(1).unwrap().as_str().len();
                let heading_text = caps.get(2).unwrap().as_str().to_string();
                while current_heading_path.len() >= level {
                    current_heading_path.pop();
                }
                current_heading_path.push(format!("{} {}", "#".repeat(level), heading_text));
                current_content = format!("{}\n", line);
                current_start_line = absolute_line;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        if !current_content.trim().is_empty() {
            sections.push(MarkdownSection {
                heading_path: current_heading_path,
                content: current_content.trim().to_string(),
                start_line: current_start_line,
                end_line: line_offset + lines.len().saturating_sub(1),
            });
        }
        sections
    }

    fn chunk_section(&self, section: &MarkdownSection, file_path: &PathBuf) -> Vec<SplitResult> {
        let estimated_tokens = section.content.len() / 4;

        if estimated_tokens <= self.max_tokens {
            return vec![SplitResult {
                file_path: file_path.clone(),
                window_text: section.content.clone(),
                window_text_hash: official_text_hashing_function(&section.content),
                start_line: section.start_line as u64,
                end_line: section.end_line as u64,
                symbol_path: section.heading_path.join(" > "),
            }];
        }

        self.split_large_content(&section.content, section.start_line)
            .into_iter()
            .map(|(chunk_text, start, end)| SplitResult {
                file_path: file_path.clone(),
                window_text: chunk_text.clone(),
                window_text_hash: official_text_hashing_function(&chunk_text),
                start_line: start as u64,
                end_line: end as u64,
                symbol_path: section.heading_path.join(" > "),
            })
            .collect()
    }

    fn split_large_content(&self, content: &str, start_line: usize) -> Vec<(String, usize, usize)> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let chars_per_chunk = self.max_tokens * 4;
        let mut current_chunk = String::new();
        let mut chunk_start = start_line;
        let mut current_line = start_line;

        for (idx, line) in lines.iter().enumerate() {
            if current_chunk.len() + line.len() + 1 > chars_per_chunk && !current_chunk.is_empty() {
                chunks.push((current_chunk.trim().to_string(), chunk_start, current_line.saturating_sub(1)));
                let overlap_start = idx.saturating_sub(self.overlap_lines);
                current_chunk = lines[overlap_start..idx].join("\n");
                if !current_chunk.is_empty() {
                    current_chunk.push('\n');
                }
                chunk_start = start_line + overlap_start;
            }
            current_chunk.push_str(line);
            current_chunk.push('\n');
            current_line = start_line + idx;
        }

        if !current_chunk.trim().is_empty() {
            chunks.push((current_chunk.trim().to_string(), chunk_start, start_line + lines.len().saturating_sub(1)));
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_parsing() {
        let content = r#"---
title: "Test Document"
created: 2024-12-17
tags: ["rust", "testing"]
---

# Hello World
"#;
        let (fm, offset) = MarkdownFrontmatter::parse(content);
        assert_eq!(fm.title, Some("Test Document".to_string()));
        assert_eq!(fm.tags, vec!["rust", "testing"]);
        assert_eq!(fm.created, Some("2024-12-17".to_string()));
        assert!(offset > 0);
    }

    #[test]
    fn test_frontmatter_no_frontmatter() {
        let content = "# Just a heading\n\nSome content";
        let (fm, offset) = MarkdownFrontmatter::parse(content);
        assert!(fm.title.is_none());
        assert!(fm.tags.is_empty());
        assert_eq!(offset, 0);
    }
}
