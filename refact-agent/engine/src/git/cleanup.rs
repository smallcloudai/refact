use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use std::collections::HashSet;
use tokio::time::Duration;
use tokio::sync::RwLock as ARwLock;
use git2::{Repository, Oid, ObjectType};

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::custom_error::{trace_and_default, MapErrToString};
use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;

const MAX_INACTIVE_REPO_DURATION: Duration = Duration::from_secs(7 * 24 * 60); // 1 week
pub const RECENT_COMMITS_DURATION: Duration = Duration::from_secs(7 * 24 * 60); // 1 week
const CLEANUP_INTERVAL_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 1 day

pub async fn git_shadow_cleanup_background_task(gcx: Arc<ARwLock<GlobalContext>>) {
    loop {
        // wait 2 mins before cleanup; lower priority than other startup tasks
        tokio::time::sleep(tokio::time::Duration::from_secs(2 * 60 / 10)).await;

        let cache_dir = gcx.read().await.cache_dir.clone();
        let workspace_folders = get_project_dirs(gcx.clone()).await;
        let workspace_folder_hashes: Vec<_> = workspace_folders.into_iter()
            .map(|f| official_text_hashing_function(&f.to_string_lossy())).collect();

        let dirs_to_check: Vec<_> = [
            cache_dir.join("shadow_git"),
            cache_dir.join("shadow_git").join("nested")
        ].into_iter().filter(|dir| dir.exists()).collect();

        for dir in dirs_to_check {
            match cleanup_inactive_shadow_repositories(&dir, &workspace_folder_hashes).await {
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

        match cleanup_old_objects_from_repos(&cache_dir.join("shadow_git"), &workspace_folder_hashes).await {
            Ok(objects_cleaned) => {
                if objects_cleaned > 0 {
                    tracing::info!("Git object cleanup: removed {} old objects from active repositories", objects_cleaned);
                }
            }
            Err(e) => {
                tracing::error!("Git object cleanup failed: {}", e);
            }
        }

        tokio::time::sleep(CLEANUP_INTERVAL_DURATION).await;
    }
}

async fn cleanup_inactive_shadow_repositories(dir: &Path, workspace_folder_hashes: &[String]) -> Result<usize, String> {
    let mut inactive_repos = Vec::new();

    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| format!("Failed to read shadow_git directory: {}", e))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read directory entry: {}", e))? {

        let path = entry.path();
        let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !path.is_dir() || !path.join(".git").exists() || workspace_folder_hashes.contains(&dir_name) {
            continue;
        }

        if repo_is_inactive(&path).await.unwrap_or_else(trace_and_default) {
            inactive_repos.push(path);
        }
    }

    let mut repos_to_remove = Vec::new();
    for repo_path in inactive_repos {
        let dir_name = repo_path.file_name().unwrap_or_default().to_string_lossy();
        if !dir_name.ends_with("_to_remove") {
            let mut new_path = repo_path.clone();
            new_path.set_file_name(format!("{dir_name}_to_remove"));
            match tokio::fs::rename(&repo_path, &new_path).await {
                Ok(()) => repos_to_remove.push(new_path),
                Err(e) => {
                    tracing::warn!("Failed to rename repo {}: {}", repo_path.display(), e);
                    continue;
                }
            }
        } else {
            repos_to_remove.push(repo_path);
        }
    }

    let mut cleanup_count = 0;
    for repo in repos_to_remove {
        match tokio::fs::remove_dir_all(&repo).await {
            Ok(()) => {
                tracing::info!("Removed old shadow git repository: {}", repo.display());
                cleanup_count += 1;
            }
            Err(e) => tracing::warn!("Failed to remove shadow git repository {}: {}", repo.display(), e),
        }
    }

    Ok(cleanup_count)
}

async fn repo_is_inactive(
    repo_dir: &Path,
) -> Result<bool, String> {
    let metadata = tokio::fs::metadata(repo_dir).await
        .map_err_with_prefix(format!("Failed to get metadata for {}:", repo_dir.display()))?;

    let mtime = metadata.modified()
        .map_err_with_prefix(format!("Failed to get modified time for {}:", repo_dir.display()))?;

    let duration_since_mtime = SystemTime::now().duration_since(mtime)
        .map_err_with_prefix(format!("Failed to calculate age for {}:", repo_dir.display()))?;

    Ok(duration_since_mtime > MAX_INACTIVE_REPO_DURATION)
}

async fn cleanup_old_objects_from_repos(dir: &Path, workspace_folder_hashes: &[String]) -> Result<usize, String> {
    let mut total_objects_removed = 0;

    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| format!("Failed to read shadow_git directory: {}", e))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read directory entry: {}", e))? {

        let path = entry.path();
        if !path.is_dir() || !path.join(".git").exists() || repo_is_inactive(&path).await.unwrap_or_else(trace_and_default) {
            continue;
        }

        let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !workspace_folder_hashes.contains(&dir_name) {
            continue;
        }

        match cleanup_old_objects_from_single_repo(&path).await {
            Ok(removed_count) => {
                if removed_count > 0 {
                    tracing::info!("Cleaned {} old objects from repository: {}", removed_count, path.display());
                    total_objects_removed += removed_count;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to cleanup objects from repository {}: {}", path.display(), e);
            }
        }
    }

    Ok(total_objects_removed)
}

pub async fn cleanup_old_objects_from_single_repo(repo_path: &Path) -> Result<usize, String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("Failed to open repository {}: {}", repo_path.display(), e))?;

    let now = SystemTime::now();
    let cutoff_time = now.checked_sub(RECENT_COMMITS_DURATION)
        .ok_or("Failed to calculate cutoff time")?;

    let (recent_objects, old_objects) = collect_objects_from_commits(&repo, cutoff_time)?;

    let objects_to_remove: HashSet<_> = old_objects.difference(&recent_objects).collect();

    if objects_to_remove.is_empty() {
        return Ok(0);
    }

    remove_unreferenced_objects(repo_path, &objects_to_remove).await
}

fn collect_objects_from_commits(repo: &Repository, cutoff_time: SystemTime) -> Result<(HashSet<String>, HashSet<String>), String> {
    let mut recent_objects = HashSet::new();
    let mut old_objects = HashSet::new();

    let head_oid = repo.head().ok()
        .and_then(|head| head.target())
        .and_then(|target| repo.find_commit(target).ok())
        .map(|commit| commit.id());

    let mut revwalk = repo.revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    let mut any_branch_pushed = false;
    if let Ok(refs) = repo.references() {
        for reference in refs {
            if let Ok(reference) = reference {
                if reference.is_branch() {
                    if let Some(target) = reference.target() {
                        if let Ok(_) = revwalk.push(target) {
                            any_branch_pushed = true;
                        }
                    }
                }
            }
        }
    }
    if !any_branch_pushed {
        if let Err(e) = revwalk.push_head() {
            tracing::warn!("Failed to push HEAD and branches to revwalk: {}", e);
        }
    }

    revwalk.set_sorting(git2::Sort::TIME)
        .map_err(|e| format!("Failed to set revwalk sorting: {}", e))?;

    for oid_result in revwalk {
        let oid = match oid_result {
            Ok(oid) => oid,
            Err(e) => { tracing::warn!("{e}"); continue; }
        };

        let commit = match repo.find_commit(oid) {
            Ok(commit) => commit,
            Err(e) => { tracing::warn!("{e}"); continue; }
        };

        let commit_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(commit.time().seconds() as u64);
        let is_recent = commit_time >= cutoff_time || Some(oid) == head_oid;

        let tree_oid = commit.tree_id();
        let objects_set = if is_recent { &mut recent_objects } else { &mut old_objects };

        objects_set.insert(oid.to_string());

        walk_tree_objects(repo, &tree_oid, objects_set);
    }

    Ok((recent_objects, old_objects))
}


pub fn walk_tree_objects(repo: &Repository, tree_oid: &Oid, objects: &mut HashSet<String>) {
    let tree = match repo.find_tree(*tree_oid) {
        Ok(t) => t,
        Err(_) => return,
    };

    for entry in tree.iter() {
        let entry_oid = entry.id();
        let entry_oid_str = entry_oid.to_string();

        if objects.contains(&entry_oid_str) {
            continue;
        }

        objects.insert(entry_oid_str);

        // If this entry is a tree (subdirectory), recursively walk it
        if entry.kind() == Some(ObjectType::Tree) {
            walk_tree_objects(repo, &entry_oid, objects);
        }
    }
}

async fn remove_unreferenced_objects(repo_path: &Path, objects_to_remove: &HashSet<&String>) -> Result<usize, String> {
    let objects_dir = repo_path.join(".git").join("objects");
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("Failed to open repository {}: {}", repo_path.display(), e))?;
    let mut removed_count = 0;

    for object_id in objects_to_remove {
        if object_id.len() < 2 {
            continue; // Invalid object ID
        }

        let oid = match Oid::from_str(object_id) {
            Ok(oid) => oid,
            Err(_) => continue,
        };

        let obj_type = match repo.find_object(oid, None) {
            Ok(obj) => obj.kind(),
            Err(_) => None,
        };
        if obj_type != Some(ObjectType::Blob) && obj_type != Some(ObjectType::Tree) {
            continue;
        }

        let (dir_name, file_name) = object_id.split_at(2);
        let object_path = objects_dir.join(dir_name).join(file_name);

        if object_path.exists() {
            match tokio::fs::remove_file(&object_path).await {
                Ok(()) => removed_count += 1,
                Err(e) => tracing::warn!("Failed to remove blob object file {}: {}", object_path.display(), e),
            }
        }
    }

    Ok(removed_count)
}
