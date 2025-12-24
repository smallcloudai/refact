use std::path::PathBuf;
use std::sync::Arc;
use chrono::{Local, Duration};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::file_filter::KNOWLEDGE_FOLDER_NAME;
use crate::files_correction::get_project_dirs;
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::global_context::{GlobalContext, try_load_caps_quickly_if_not_present};
use crate::knowledge_graph::kg_structs::KnowledgeFrontmatter;
use crate::knowledge_graph::kg_subchat::{enrich_knowledge_metadata, check_deprecation};
use crate::knowledge_graph::build_knowledge_graph;
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
    pub score: Option<f32>,  // VecDB similarity score (lower distance = higher relevance)
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
    top_n_memories: usize,
    top_n_trajectories: usize,
) -> Result<Vec<MemoRecord>, String> {
    let knowledge_dir = get_knowledge_dir(gcx.clone()).await?;

    let vecdb_arc = {
        let gcx_read = gcx.read().await;
        gcx_read.vec_db.clone()
    };

    let vecdb_guard = vecdb_arc.lock().await;
    if vecdb_guard.is_none() {
        drop(vecdb_guard);
        return memories_search_fallback(gcx, query, top_n_memories, &knowledge_dir).await;
    }

    let vecdb = vecdb_guard.as_ref().unwrap();
    let search_result = vecdb.vecdb_search(query.to_string(), (top_n_memories + top_n_trajectories) * 5, None).await
        .map_err(|e| format!("VecDB search failed: {}", e))?;
    drop(vecdb_guard);

    use std::collections::HashMap;

    struct KnowledgeMatch { best_score: f32 }
    struct TrajectoryMatch {
        best_score: f32,
        matched_ranges: Vec<(u64, u64)>,
    }

    let mut knowledge_matches: HashMap<PathBuf, KnowledgeMatch> = HashMap::new();
    let mut trajectory_matches: HashMap<PathBuf, TrajectoryMatch> = HashMap::new();

    for rec in search_result.results.iter() {
        let path_str = rec.file_path.to_string_lossy().to_string();
        let score = 1.0 - (rec.distance / 2.0).min(1.0);

        if path_str.contains(KNOWLEDGE_FOLDER_NAME) {
            knowledge_matches
                .entry(rec.file_path.clone())
                .and_modify(|m| { if score > m.best_score { m.best_score = score; } })
                .or_insert(KnowledgeMatch { best_score: score });
        } else if path_str.contains(".refact/trajectories/") && path_str.ends_with(".json") {
            trajectory_matches
                .entry(rec.file_path.clone())
                .and_modify(|m| {
                    if score > m.best_score { m.best_score = score; }
                    m.matched_ranges.push((rec.start_line, rec.end_line));
                })
                .or_insert(TrajectoryMatch {
                    best_score: score,
                    matched_ranges: vec![(rec.start_line, rec.end_line)],
                });
        }
    }

    let mut records = Vec::new();

    // Process knowledge files (whole content)
    let mut sorted_knowledge: Vec<_> = knowledge_matches.into_iter().collect();
    sorted_knowledge.sort_by(|a, b| b.1.best_score.partial_cmp(&a.1.best_score).unwrap_or(std::cmp::Ordering::Equal));

    for (file_path, file_match) in sorted_knowledge.into_iter().take(top_n_memories) {
        let text = match get_file_text_from_memory_or_disk(gcx.clone(), &file_path).await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let (frontmatter, content_start) = KnowledgeFrontmatter::parse(&text);
        if frontmatter.is_archived() {
            continue;
        }

        let content = text[content_start..].trim().to_string();
        let line_count = content.lines().count();
        let id = frontmatter.id.clone().unwrap_or_else(|| file_path.to_string_lossy().to_string());

        records.push(MemoRecord {
            memid: id,
            tags: frontmatter.tags,
            content,
            file_path: Some(file_path),
            line_range: Some((1, line_count as u64)),
            title: frontmatter.title,
            created: frontmatter.created,
            kind: frontmatter.kind,
            score: Some(file_match.best_score),
        });
    }

    // Process trajectories (matched parts only)
    let mut sorted_trajectories: Vec<_> = trajectory_matches.into_iter().collect();
    sorted_trajectories.sort_by(|a, b| b.1.best_score.partial_cmp(&a.1.best_score).unwrap_or(std::cmp::Ordering::Equal));

    for (file_path, traj_match) in sorted_trajectories.into_iter().take(top_n_trajectories) {
        let text = match get_file_text_from_memory_or_disk(gcx.clone(), &file_path).await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let traj_json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let traj_id = file_path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let traj_title = traj_json.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let messages = match traj_json.get("messages").and_then(|v| v.as_array()) {
            Some(m) => m,
            None => continue,
        };

        // Extract matched message content
        let mut matched_content = Vec::new();
        for (start, end) in &traj_match.matched_ranges {
            let start_idx = *start as usize;
            let end_idx = (*end as usize).min(messages.len().saturating_sub(1));

            for idx in start_idx..=end_idx {
                if let Some(msg) = messages.get(idx) {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let content = msg.get("content")
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.chars().take(500).collect::<String>()
                            } else {
                                v.to_string().chars().take(500).collect()
                            }
                        })
                        .unwrap_or_default();

                    if !content.is_empty() && role != "system" && role != "context_file" {
                        matched_content.push(format!("[msg {}] {}: {}", idx, role, content));
                    }
                }
            }
        }

        if matched_content.is_empty() {
            continue;
        }

        let content = format!(
            "Trajectory: {} ({})\n\n{}",
            traj_title,
            traj_id,
            matched_content.join("\n\n")
        );

        records.push(MemoRecord {
            memid: traj_id.clone(),
            tags: vec!["trajectory".to_string()],
            content,
            file_path: Some(file_path),
            line_range: None,
            title: Some(traj_title),
            created: None,
            kind: Some("trajectory".to_string()),
            score: Some(traj_match.best_score),
        });
    }

    tracing::info!("memories_search: found {} knowledge + {} trajectories",
        records.iter().filter(|r| r.kind.as_deref() != Some("trajectory")).count(),
        records.iter().filter(|r| r.kind.as_deref() == Some("trajectory")).count()
    );

    if !records.is_empty() {
        return Ok(records);
    }

    memories_search_fallback(gcx, query, top_n_memories, &knowledge_dir).await
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

        // Normalize keyword score to 0-1 range (assuming max ~10 word matches)
        let normalized_score = (score as f32 / 10.0).min(1.0);

        scored_results.push((score, MemoRecord {
            memid: id,
            tags: frontmatter.tags,
            content: content_preview,
            file_path: Some(path.to_path_buf()),
            line_range: None,
            title: frontmatter.title,
            created: frontmatter.created,
            kind: frontmatter.kind,
            score: Some(normalized_score),
        }));
    }

    scored_results.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(scored_results.into_iter().take(top_n).map(|(_, r)| r).collect())
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

fn extract_entities(content: &str) -> Vec<String> {
    let backtick_re = Regex::new(r"`([a-zA-Z_][a-zA-Z0-9_:]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)`").unwrap();
    backtick_re.captures_iter(content)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .filter(|e| e.len() >= 3 && e.len() <= 100)
        .collect()
}

fn extract_file_paths(content: &str) -> Vec<String> {
    let path_re = Regex::new(r"(?:^|[\s`])((?:[a-zA-Z0-9_-]+/)+[a-zA-Z0-9_-]+\.[a-zA-Z0-9]+)").unwrap();
    path_re.captures_iter(content)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .collect()
}

pub struct EnrichmentParams {
    pub base_tags: Vec<String>,
    pub base_filenames: Vec<String>,
    pub base_kind: String,
    pub base_title: Option<String>,
}

pub async fn memories_add_enriched(
    ccx: Arc<AMutex<AtCommandsContext>>,
    content: &str,
    params: EnrichmentParams,
) -> Result<PathBuf, String> {
    let gcx = ccx.lock().await.global_context.clone();

    let entities = extract_entities(content);
    let detected_paths = extract_file_paths(content);

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await
        .map_err(|e| format!("Failed to load caps: {}", e.message))?;
    let light_model = if caps.defaults.chat_light_model.is_empty() {
        caps.defaults.chat_default_model.clone()
    } else {
        caps.defaults.chat_light_model.clone()
    };

    let kg = build_knowledge_graph(gcx.clone()).await;

    let candidate_files: Vec<String> = {
        let mut files = params.base_filenames.clone();
        files.extend(detected_paths);
        files.into_iter().take(30).collect()
    };

    let candidate_docs: Vec<(String, String)> = kg.active_docs()
        .take(20)
        .map(|d| {
            let id = d.frontmatter.id.clone().unwrap_or_else(|| d.path.to_string_lossy().to_string());
            let title = d.frontmatter.title.clone().unwrap_or_else(|| "Untitled".to_string());
            (id, title)
        })
        .collect();

    let enrichment = enrich_knowledge_metadata(
        ccx.clone(),
        &light_model,
        content,
        &entities,
        &candidate_files,
        &candidate_docs,
    ).await;

    let (final_title, final_tags, final_filenames, final_kind, final_links, review_days) = match enrichment {
        Ok(e) => {
            let mut tags = params.base_tags.clone();
            tags.extend(e.tags);
            tags.sort();
            tags.dedup();

            let mut files = params.base_filenames.clone();
            files.extend(e.filenames);
            files.sort();
            files.dedup();

            let kind = e.kind.unwrap_or_else(|| params.base_kind.clone());

            (
                e.title.or(params.base_title.clone()).or_else(|| content.lines().next().map(|l| l.trim_start_matches('#').trim().to_string())),
                if tags.is_empty() { vec![params.base_kind.clone()] } else { tags },
                files,
                kind,
                e.links,
                e.review_after_days.unwrap_or(90),
            )
        }
        Err(e) => {
            warn!("Enrichment failed, using defaults: {}", e);
            let tags = if params.base_tags.is_empty() { vec![params.base_kind.clone()] } else { params.base_tags };
            (
                params.base_title.or_else(|| content.lines().next().map(|l| l.trim_start_matches('#').trim().to_string())),
                tags,
                params.base_filenames,
                params.base_kind,
                vec![],
                90,
            )
        }
    };

    let now = Local::now();
    let frontmatter = KnowledgeFrontmatter {
        id: Some(Uuid::new_v4().to_string()),
        title: final_title.clone(),
        tags: final_tags.clone(),
        created: Some(now.format("%Y-%m-%d").to_string()),
        updated: Some(now.format("%Y-%m-%d").to_string()),
        filenames: final_filenames.clone(),
        links: final_links,
        kind: Some(final_kind),
        status: Some("active".to_string()),
        superseded_by: None,
        deprecated_at: None,
        review_after: Some((now + Duration::days(review_days)).format("%Y-%m-%d").to_string()),
    };

    let file_path = memories_add(gcx.clone(), &frontmatter, content).await?;
    let new_doc_id = frontmatter.id.clone().unwrap();

    let deprecation_candidates = kg.get_deprecation_candidates(
        &final_tags,
        &final_filenames,
        &entities,
        Some(&new_doc_id),
    );

    if !deprecation_candidates.is_empty() {
        let snippet: String = content.chars().take(500).collect();

        match check_deprecation(
            ccx.clone(),
            &light_model,
            final_title.as_deref().unwrap_or("Untitled"),
            &final_tags,
            &final_filenames,
            &snippet,
            &deprecation_candidates,
        ).await {
            Ok(result) => {
                for decision in result.deprecate {
                    if decision.confidence >= 0.75 {
                        if let Some(doc) = kg.get_doc_by_id(&decision.target_id) {
                            if let Err(e) = deprecate_document(
                                gcx.clone(),
                                &doc.path,
                                Some(&new_doc_id),
                                &decision.reason,
                            ).await {
                                warn!("Failed to deprecate {}: {}", decision.target_id, e);
                            } else {
                                info!("Deprecated {} (confidence: {:.2}): {}", decision.target_id, decision.confidence, decision.reason);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Deprecation check failed: {}", e);
            }
        }
    }

    Ok(file_path)
}
