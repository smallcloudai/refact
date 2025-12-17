use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use tokio::fs;
use tracing::{info, warn};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::global_context::GlobalContext;
use crate::memories::archive_document;
use super::kg_builder::build_knowledge_graph;

const CLEANUP_INTERVAL_SECS: u64 = 7 * 24 * 60 * 60;
const TRAJECTORY_MAX_AGE_DAYS: i64 = 90;
const STALE_DOC_AGE_DAYS: i64 = 180;

#[derive(Debug, Serialize, Deserialize, Default)]
struct CleanupState {
    last_run: i64,
}

async fn load_cleanup_state(gcx: Arc<ARwLock<GlobalContext>>) -> CleanupState {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let state_file = cache_dir.join("knowledge_cleanup_state.json");

    match fs::read_to_string(&state_file).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => CleanupState::default(),
    }
}

async fn save_cleanup_state(gcx: Arc<ARwLock<GlobalContext>>, state: &CleanupState) {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let state_file = cache_dir.join("knowledge_cleanup_state.json");

    if let Ok(content) = serde_json::to_string(state) {
        let _ = fs::write(&state_file, content).await;
    }
}

pub async fn knowledge_cleanup_background_task(gcx: Arc<ARwLock<GlobalContext>>) {
    loop {
        let state = load_cleanup_state(gcx.clone()).await;
        let now = Utc::now().timestamp();

        if now - state.last_run >= CLEANUP_INTERVAL_SECS as i64 {
            info!("knowledge_cleanup: running weekly cleanup");

            match run_cleanup(gcx.clone()).await {
                Ok(report) => {
                    info!("knowledge_cleanup: completed - archived {} trajectories, {} deprecated docs, {} orphan warnings",
                        report.archived_trajectories,
                        report.archived_deprecated,
                        report.orphan_warnings,
                    );
                }
                Err(e) => {
                    warn!("knowledge_cleanup: failed - {}", e);
                }
            }

            let new_state = CleanupState { last_run: now };
            save_cleanup_state(gcx.clone(), &new_state).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(24 * 60 * 60)).await;
    }
}

#[derive(Debug, Default)]
struct CleanupReport {
    archived_trajectories: usize,
    archived_deprecated: usize,
    orphan_warnings: usize,
}

async fn run_cleanup(gcx: Arc<ARwLock<GlobalContext>>) -> Result<CleanupReport, String> {
    let kg = build_knowledge_graph(gcx.clone()).await;
    let staleness = kg.check_staleness(STALE_DOC_AGE_DAYS, TRAJECTORY_MAX_AGE_DAYS);
    let mut report = CleanupReport::default();

    for path in staleness.stale_trajectories {
        match archive_document(gcx.clone(), &path).await {
            Ok(_) => report.archived_trajectories += 1,
            Err(e) => warn!("Failed to archive trajectory {}: {}", path.display(), e),
        }
    }

    for path in staleness.deprecated_ready_to_archive {
        match archive_document(gcx.clone(), &path).await {
            Ok(_) => report.archived_deprecated += 1,
            Err(e) => warn!("Failed to archive deprecated doc {}: {}", path.display(), e),
        }
    }

    report.orphan_warnings = staleness.orphan_file_refs.len();
    for (path, missing_files) in &staleness.orphan_file_refs {
        info!("knowledge_cleanup: {} references missing files: {:?}", path.display(), missing_files);
    }

    for path in &staleness.past_review {
        info!("knowledge_cleanup: {} is past review date", path.display());
    }

    Ok(report)
}
