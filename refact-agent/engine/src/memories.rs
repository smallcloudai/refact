use std::path::PathBuf;
use std::sync::Arc;
use chrono::Local;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio::fs;
use tracing::info;
use walkdir::WalkDir;

use crate::file_filter::KNOWLEDGE_FOLDER_NAME;
use crate::files_correction::get_project_dirs;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::vecdb::vdb_markdown_splitter::MarkdownFrontmatter;
use crate::vecdb::vdb_structs::VecdbSearch;

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MemoRecord {
    pub memid: String,
    pub tags: Vec<String>,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub line_range: Option<(u64, u64)>,
    pub title: Option<String>,
    pub created: Option<String>,
}

fn generate_slug(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("knowledge");
    first_line
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
        .chars()
        .take(50)
        .collect()
}

fn generate_filename(content: &str) -> String {
    let timestamp = Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let slug = generate_slug(content);
    if slug.is_empty() {
        format!("{}_knowledge.md", timestamp)
    } else {
        format!("{}_{}.md", timestamp, slug)
    }
}

fn create_markdown_content(tags: &[String], filenames: &[String], content: &str) -> String {
    let now = Local::now().format("%Y-%m-%d").to_string();
    let title = content.lines().next().unwrap_or("Knowledge Entry");
    let tags_str = tags.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
    let filenames_str = if filenames.is_empty() {
        String::new()
    } else {
        format!("\nfilenames: [{}]", filenames.iter().map(|f| format!("\"{}\"", f)).collect::<Vec<_>>().join(", "))
    };

    format!(
        r#"---
title: "{}"
created: {}
tags: [{}]{}
---

{}
"#,
        title.trim_start_matches('#').trim(),
        now,
        tags_str,
        filenames_str,
        content
    )
}

async fn get_knowledge_dir(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let project_dirs = get_project_dirs(gcx).await;
    let workspace_root = project_dirs.first().ok_or("No workspace folder found")?;
    Ok(workspace_root.join(KNOWLEDGE_FOLDER_NAME))
}

pub async fn memories_add(
    gcx: Arc<ARwLock<GlobalContext>>,
    tags: &[String],
    filenames: &[String],
    content: &str,
) -> Result<PathBuf, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;
    fs::create_dir_all(&knowledge_dir).await.map_err(|e| format!("Failed to create knowledge dir: {}", e))?;

    let filename = generate_filename(content);
    let file_path = knowledge_dir.join(&filename);

    if file_path.exists() {
        return Err(format!("File already exists: {}", file_path.display()));
    }

    let md_content = create_markdown_content(tags, filenames, content);
    fs::write(&file_path, &md_content).await.map_err(|e| format!("Failed to write knowledge file: {}", e))?;

    info!("Created knowledge entry: {}", file_path.display());

    if let Some(vecdb) = gcx.read().await.vec_db.lock().await.as_ref() {
        vecdb.vectorizer_enqueue_files(&vec![file_path.to_string_lossy().to_string()], true).await;
    }

    Ok(file_path)
}

pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &str,
    top_n: usize,
) -> Result<Vec<MemoRecord>, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;
    let knowledge_prefix = knowledge_dir.to_string_lossy().to_string();

    let has_vecdb = gcx.read().await.vec_db.lock().await.is_some();
    if has_vecdb {
        let vecdb_lock = gcx.read().await.vec_db.clone();
        let vecdb_guard = vecdb_lock.lock().await;
        let vecdb = vecdb_guard.as_ref().unwrap();
        let search_result = vecdb.vecdb_search(query.to_string(), top_n * 3, None).await
            .map_err(|e| format!("VecDB search failed: {}", e))?;

        let mut records = Vec::new();
        for rec in search_result.results {
            let path_str = rec.file_path.to_string_lossy().to_string();
            if !path_str.contains(KNOWLEDGE_FOLDER_NAME) {
                continue;
            }

            let text = match get_file_text_from_memory_or_disk(gcx.clone(), &rec.file_path).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let (frontmatter, _) = MarkdownFrontmatter::parse(&text);
            let lines: Vec<&str> = text.lines().collect();
            let start = (rec.start_line as usize).min(lines.len().saturating_sub(1));
            let end = (rec.end_line as usize).min(lines.len().saturating_sub(1));
            let snippet = lines[start..=end].join("\n");

            records.push(MemoRecord {
                memid: format!("{}:{}-{}", path_str, rec.start_line, rec.end_line),
                tags: frontmatter.tags,
                content: snippet,
                file_path: Some(rec.file_path.clone()),
                line_range: Some((rec.start_line, rec.end_line)),
                title: frontmatter.title,
                created: frontmatter.created,
            });

            if records.len() >= top_n {
                break;
            }
        }

        if !records.is_empty() {
            return Ok(records);
        }
    }

    memories_search_fallback(gcx, query, top_n, &knowledge_prefix).await
}

async fn memories_search_fallback(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &str,
    top_n: usize,
    knowledge_dir: &str,
) -> Result<Vec<MemoRecord>, String> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let mut scored_results: Vec<(usize, MemoRecord)> = Vec::new();

    let knowledge_path = PathBuf::from(knowledge_dir);
    if !knowledge_path.exists() {
        return Ok(vec![]);
    }

    for entry in WalkDir::new(&knowledge_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "mdx" {
            continue;
        }

        let text = match get_file_text_from_memory_or_disk(gcx.clone(), &path.to_path_buf()).await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let text_lower = text.to_lowercase();
        let score: usize = query_words.iter().filter(|w| text_lower.contains(*w)).count();
        if score == 0 {
            continue;
        }

        let (frontmatter, _) = MarkdownFrontmatter::parse(&text);
        scored_results.push((score, MemoRecord {
            memid: path.to_string_lossy().to_string(),
            tags: frontmatter.tags,
            content: text.chars().take(500).collect(),
            file_path: Some(path.to_path_buf()),
            line_range: None,
            title: frontmatter.title,
            created: frontmatter.created,
        }));
    }

    scored_results.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(scored_results.into_iter().take(top_n).map(|(_, r)| r).collect())
}

pub async fn save_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    compressed_trajectory: &str,
) -> Result<PathBuf, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;
    let trajectories_dir = knowledge_dir.join("trajectories");
    fs::create_dir_all(&trajectories_dir).await.map_err(|e| format!("Failed to create trajectories dir: {}", e))?;

    let filename = generate_filename(compressed_trajectory);
    let file_path = trajectories_dir.join(&filename);

    let tags = vec!["trajectory".to_string()];
    let md_content = create_markdown_content(&tags, &[], compressed_trajectory);
    fs::write(&file_path, &md_content).await.map_err(|e| format!("Failed to write trajectory file: {}", e))?;

    info!("Saved trajectory: {}", file_path.display());

    if let Some(vecdb) = gcx.read().await.vec_db.lock().await.as_ref() {
        vecdb.vectorizer_enqueue_files(&vec![file_path.to_string_lossy().to_string()], true).await;
    }

    Ok(file_path)
}
