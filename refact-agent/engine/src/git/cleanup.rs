use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::Duration;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::trace_and_default;
use crate::files_correction::{canonical_path, get_project_dirs};
use crate::global_context::GlobalContext;

const MAX_INACTIVE_DURATION: Duration = Duration::from_secs(5 * 24 * 3600); // 5 days

pub async fn git_shadow_cleanup_background_task(gcx: Arc<ARwLock<GlobalContext>>) {
    let cache_dir = gcx.read().await.cache_dir.clone();
    let workspace_folders = get_project_dirs(gcx.clone()).await;

    let dirs_to_check: Vec<_> = [
        cache_dir.join("shadow_git"),
        cache_dir.join("shadow_git").join("nested")
    ].into_iter().filter(|dir| dir.exists()).collect();

    for dir in dirs_to_check {
        match cleanup_inactive_shadow_repositories(&dir, &workspace_folders).await {
            Ok(cleanup_count) => {
                if cleanup_count > 0 {
                    tracing::info!("Git shadow cleanup: removed {} old repositories", cleanup_count);
                }
            }
            Err(e) => {
                tracing::error!("Git shadow cleanup failed: {}", e);
            }
        }
    }
}

async fn cleanup_inactive_shadow_repositories(dir: &Path, workspace_folders: &[PathBuf]) -> Result<usize, String> {
    let mut cleanup_count = 0;

    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| format!("Failed to read shadow_git directory: {}", e))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read directory entry: {}", e))? {

        let path = entry.path();
        if !path.is_dir() || !path.join(".git").exists() || workspace_folders.contains(&canonical_path(path.to_string_lossy())) {
            continue;
        }

        if repo_is_inactive(&path).await {
            match tokio::fs::remove_dir_all(&path).await {
                Ok(()) => {
                    tracing::info!("Removed old shadow git repository: {}", path.display());
                    cleanup_count += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to remove shadow git repository {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(cleanup_count)
}

async fn repo_is_inactive(
    repo_dir: &Path,
) -> bool {
    let now = SystemTime::now();

    let head_is_old_or_missing = match get_file_age(&repo_dir.join(".git").join("HEAD"), &now).await {
        Ok(Some(age)) => age > MAX_INACTIVE_DURATION,
        Ok(None) => true,
        Err(e) => trace_and_default(e),
    };

    let index_is_old_or_missing = match get_file_age(&repo_dir.join(".git").join("index"), &now).await {
        Ok(Some(age)) => age > MAX_INACTIVE_DURATION,
        Ok(None) => true,
        Err(e) => trace_and_default(e),
    };

    head_is_old_or_missing && index_is_old_or_missing
}

async fn get_file_age(file_path: &Path, now: &SystemTime) -> Result<Option<Duration>, String> {
    let metadata = match tokio::fs::metadata(file_path).await {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Failed to get metadata for {}: {}", file_path.display(), e)),
    };

    let modified = metadata.modified()
        .map_err(|e| format!("Failed to get modified time for {}: {}", file_path.display(), e))?;

    let duration = now.duration_since(modified)
        .map_err(|e| format!("Failed to calculate age for {}: {}", file_path.display(), e))?;

    Ok(Some(duration))
}
