use std::path::PathBuf;
use std::sync::Arc;
use chrono::{Local, Duration};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio::fs;
use tracing::info;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::file_filter::KNOWLEDGE_FOLDER_NAME;
use crate::files_correction::get_project_dirs;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::GlobalContext;
use crate::knowledge_graph::kg_structs::KnowledgeFrontmatter;
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
    pub kind: Option<String>,
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
    let short_uuid = &Uuid::new_v4().to_string()[..8];
    if slug.is_empty() {
        format!("{}_{}_knowledge.md", timestamp, short_uuid)
    } else {
        format!("{}_{}_{}.md", timestamp, short_uuid, slug)
    }
}

pub fn create_frontmatter(
    title: Option<&str>,
    tags: &[String],
    filenames: &[String],
    links: &[String],
    kind: &str,
) -> KnowledgeFrontmatter {
    let now = Local::now();
    let created = now.format("%Y-%m-%d").to_string();
    let review_days = match kind {
        "trajectory" => 90,
        "preference" => 365,
        _ => 90,
    };
    let review_after = (now + Duration::days(review_days)).format("%Y-%m-%d").to_string();

    KnowledgeFrontmatter {
        id: Some(Uuid::new_v4().to_string()),
        title: title.map(|t| t.to_string()),
        tags: tags.to_vec(),
        created: Some(created.clone()),
        updated: Some(created),
        filenames: filenames.to_vec(),
        links: links.to_vec(),
        kind: Some(kind.to_string()),
        status: Some("active".to_string()),
        superseded_by: None,
        deprecated_at: None,
        review_after: Some(review_after),
    }
}

async fn get_knowledge_dir(gcx: Arc<ARwLock<GlobalContext>>) -> Result<PathBuf, String> {
    let project_dirs = get_project_dirs(gcx).await;
    let workspace_root = project_dirs.first().ok_or("No workspace folder found")?;
    Ok(workspace_root.join(KNOWLEDGE_FOLDER_NAME))
}

pub async fn memories_add(
    gcx: Arc<ARwLock<GlobalContext>>,
    frontmatter: &KnowledgeFrontmatter,
    content: &str,
) -> Result<PathBuf, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;
    fs::create_dir_all(&knowledge_dir).await.map_err(|e| format!("Failed to create knowledge dir: {}", e))?;

    let filename = generate_filename(content);
    let file_path = knowledge_dir.join(&filename);

    if file_path.exists() {
        return Err(format!("File already exists: {}", file_path.display()));
    }

    let md_content = format!("{}\n\n{}", frontmatter.to_yaml(), content);
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

            let (frontmatter, _content_start) = KnowledgeFrontmatter::parse(&text);

            if frontmatter.is_archived() {
                continue;
            }

            let lines: Vec<&str> = text.lines().collect();
            let start = (rec.start_line as usize).min(lines.len().saturating_sub(1));
            let end = (rec.end_line as usize).min(lines.len().saturating_sub(1));
            let snippet = lines[start..=end].join("\n");

            let id = frontmatter.id.clone().unwrap_or_else(|| path_str.clone());

            records.push(MemoRecord {
                memid: format!("{}:{}-{}", id, rec.start_line, rec.end_line),
                tags: frontmatter.tags,
                content: snippet,
                file_path: Some(rec.file_path.clone()),
                line_range: Some((rec.start_line, rec.end_line)),
                title: frontmatter.title,
                created: frontmatter.created,
                kind: frontmatter.kind,
            });

            if records.len() >= top_n {
                break;
            }
        }

        if !records.is_empty() {
            return Ok(records);
        }
    }

    memories_search_fallback(gcx, query, top_n, &knowledge_dir).await
}

async fn memories_search_fallback(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &str,
    top_n: usize,
    knowledge_dir: &PathBuf,
) -> Result<Vec<MemoRecord>, String> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let mut scored_results: Vec<(usize, MemoRecord)> = Vec::new();

    if !knowledge_dir.exists() {
        return Ok(vec![]);
    }

    for entry in WalkDir::new(knowledge_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.to_string_lossy().contains("/archive/") {
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

        let (frontmatter, content_start) = KnowledgeFrontmatter::parse(&text);
        if frontmatter.is_archived() {
            continue;
        }

        let id = frontmatter.id.clone().unwrap_or_else(|| path.to_string_lossy().to_string());
        let content_preview: String = text[content_start..].chars().take(500).collect();

        scored_results.push((score, MemoRecord {
            memid: id,
            tags: frontmatter.tags,
            content: content_preview,
            file_path: Some(path.to_path_buf()),
            line_range: None,
            title: frontmatter.title,
            created: frontmatter.created,
            kind: frontmatter.kind,
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

    let frontmatter = create_frontmatter(
        compressed_trajectory.lines().next(),
        &["trajectory".to_string()],
        &[],
        &[],
        "trajectory",
    );

    let md_content = format!("{}\n\n{}", frontmatter.to_yaml(), compressed_trajectory);
    fs::write(&file_path, &md_content).await.map_err(|e| format!("Failed to write trajectory file: {}", e))?;

    info!("Saved trajectory: {}", file_path.display());

    if let Some(vecdb) = gcx.read().await.vec_db.lock().await.as_ref() {
        vecdb.vectorizer_enqueue_files(&vec![file_path.to_string_lossy().to_string()], true).await;
    }

    Ok(file_path)
}

pub async fn deprecate_document(
    gcx: Arc<ARwLock<GlobalContext>>,
    doc_path: &PathBuf,
    superseded_by: Option<&str>,
    reason: &str,
) -> Result<(), String> {
    let text = get_file_text_from_memory_or_disk(gcx.clone(), doc_path).await
        .map_err(|e| format!("Failed to read document: {}", e))?;

    let (mut frontmatter, content_start) = KnowledgeFrontmatter::parse(&text);
    let content = &text[content_start..];

    frontmatter.status = Some("deprecated".to_string());
    frontmatter.deprecated_at = Some(Local::now().format("%Y-%m-%d").to_string());
    if let Some(new_id) = superseded_by {
        frontmatter.superseded_by = Some(new_id.to_string());
    }

    let deprecated_banner = format!("\n\n> ⚠️ **DEPRECATED**: {}\n", reason);
    let new_content = format!("{}\n{}{}", frontmatter.to_yaml(), deprecated_banner, content);

    fs::write(doc_path, new_content).await.map_err(|e| format!("Failed to write: {}", e))?;

    info!("Deprecated document: {}", doc_path.display());

    if let Some(vecdb) = gcx.read().await.vec_db.lock().await.as_ref() {
        vecdb.vectorizer_enqueue_files(&vec![doc_path.to_string_lossy().to_string()], true).await;
    }

    Ok(())
}

pub async fn archive_document(gcx: Arc<ARwLock<GlobalContext>>, doc_path: &PathBuf) -> Result<PathBuf, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;
    let archive_dir = knowledge_dir.join("archive");
    fs::create_dir_all(&archive_dir).await.map_err(|e| format!("Failed to create archive dir: {}", e))?;

    let filename = doc_path.file_name().ok_or("Invalid filename")?;
    let archive_path = archive_dir.join(filename);

    fs::rename(doc_path, &archive_path).await.map_err(|e| format!("Failed to move to archive: {}", e))?;

    info!("Archived document: {} -> {}", doc_path.display(), archive_path.display());

    Ok(archive_path)
}
