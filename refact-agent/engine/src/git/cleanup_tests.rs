use super::*;
use std::{collections::HashSet, fs, time::SystemTime};
use tempfile::TempDir;
use git2::{Repository, Signature, Time};
use std::collections::HashMap;

async fn create_test_repository() -> (TempDir, HashMap<String, String>, HashMap<String, String>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let repo = Repository::init(repo_path).expect("Failed to init repo");
    let mut config = repo.config().expect("Failed to get config");
    config.set_str("user.name", "Test User").expect("Failed to set user name");
    config.set_str("user.email", "test@example.com").expect("Failed to set user email");

    let mut commit_hashes = HashMap::new();
    let mut blob_ids = HashMap::new();

    // Create signature for old commits (20 days ago)
    let old_time = Time::new(
        (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64) - (20 * 24 * 3600),
        0
    );
    let old_signature = Signature::new("Test User", "test@example.com", &old_time)
        .expect("Failed to create old signature");

    // Create first old commit with TWO files: one that will be kept, one that will be removed
    let old_kept_file_path = repo_path.join("old_kept_file.txt");
    fs::write(&old_kept_file_path, "This file should be kept").expect("Failed to write old kept file");

    let old_removed_file_path = repo_path.join("old_removed_file.txt");
    fs::write(&old_removed_file_path, "This file should be removed").expect("Failed to write old removed file");

    let mut index = repo.index().expect("Failed to get index");
    index.add_path(std::path::Path::new("old_kept_file.txt")).expect("Failed to add old kept file");
    index.add_path(std::path::Path::new("old_removed_file.txt")).expect("Failed to add old removed file");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    let old_commit_oid = repo.commit(
        Some("HEAD"),
        &old_signature,
        &old_signature,
        "Old commit - both files",
        &tree,
        &[]
    ).expect("Failed to create old commit");
    commit_hashes.insert("old_commit".to_string(), old_commit_oid.to_string());

    // Record the blob IDs for both files from the first commit
    let first_commit = repo.find_commit(old_commit_oid).expect("Failed to find first commit");
    let first_tree = first_commit.tree().expect("Failed to get first tree");

    let kept_entry = first_tree.get_name("old_kept_file.txt").expect("Failed to find old_kept_file.txt in first commit");
    let removed_entry = first_tree.get_name("old_removed_file.txt").expect("Failed to find old_removed_file.txt in first commit");

    blob_ids.insert("old_kept_blob".to_string(), kept_entry.id().to_string());
    blob_ids.insert("old_removed_blob".to_string(), removed_entry.id().to_string());

    // Create second old commit: delete old_removed_file.txt and add shared_file.txt
    let shared_file_path = repo_path.join("shared_file.txt");
    fs::write(&shared_file_path, "Shared content - version 1").expect("Failed to write shared file");

    let mut index = repo.index().expect("Failed to get index for second commit");
    // Remove the old_removed_file.txt from the index (this makes its blob unreferenced)
    index.remove_path(std::path::Path::new("old_removed_file.txt")).expect("Failed to remove old_removed_file.txt");
    // Add the shared file
    index.add_path(std::path::Path::new("shared_file.txt")).expect("Failed to add shared file");
    index.write().expect("Failed to write index for second commit");

    let tree_id = index.write_tree().expect("Failed to write tree for second commit");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree for second commit");
    let parent_commit = repo.find_commit(old_commit_oid).expect("Failed to find parent commit");

    let old_commit2_oid = repo.commit(
        Some("HEAD"),
        &old_signature,
        &old_signature,
        "Another old commit - removed old_removed_file.txt",
        &tree,
        &[&parent_commit]
    ).expect("Failed to create second old commit");
    commit_hashes.insert("old_commit2".to_string(), old_commit2_oid.to_string());

    // Create recent commit (2 days ago)
    let recent_time = Time::new(
        (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64) - (2 * 24 * 3600),
        0
    );
    let recent_signature = Signature::new("Test User", "test@example.com", &recent_time)
        .expect("Failed to create recent signature");

    // Modify shared file and add new file
    fs::write(&shared_file_path, "Shared content - version 2 (recent)").expect("Failed to update shared file");

    let recent_file_path = repo_path.join("recent_file.txt");
    fs::write(&recent_file_path, "This file should NOT be cleaned up").expect("Failed to write recent file");

    index.add_path(std::path::Path::new("shared_file.txt")).expect("Failed to add updated shared file");
    index.add_path(std::path::Path::new("recent_file.txt")).expect("Failed to add recent file");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let parent_commit = repo.find_commit(old_commit2_oid).expect("Failed to find parent commit");

    let recent_commit_oid = repo.commit(
        Some("HEAD"),
        &recent_signature,
        &recent_signature,
        "Recent commit - should be preserved",
        &tree,
        &[&parent_commit]
    ).expect("Failed to create recent commit");
    commit_hashes.insert("recent_commit".to_string(), recent_commit_oid.to_string());

    (temp_dir, commit_hashes, blob_ids)
}

fn get_all_objects(repo_path: &std::path::Path) -> Result<HashSet<String>, String> {
    let objects_dir = repo_path.join(".git").join("objects");
    let mut objects = HashSet::new();

    for entry in fs::read_dir(&objects_dir).map_err(|e| format!("Failed to read objects dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() && path.file_name().unwrap().to_str().unwrap().len() == 2 {
            let prefix = path.file_name().unwrap().to_str().unwrap();

            for obj_entry in fs::read_dir(&path).map_err(|e| format!("Failed to read object subdir: {}", e))? {
                let obj_entry = obj_entry.map_err(|e| format!("Failed to read object entry: {}", e))?;
                let obj_path = obj_entry.path();

                if obj_path.is_file() {
                    let suffix = obj_path.file_name().unwrap().to_str().unwrap();
                    let full_hash = format!("{}{}", prefix, suffix);
                    if full_hash.len() == 40 {
                        objects.insert(full_hash);
                    }
                }
            }
        }
    }

    Ok(objects)
}

fn get_objects_for_commit(repo: &Repository, commit_oid: &str) -> Result<HashSet<String>, String> {
    let mut objects = HashSet::new();
    let oid = git2::Oid::from_str(commit_oid).map_err(|e| format!("Invalid OID: {}", e))?;

    objects.insert(commit_oid.to_string());

    let commit = repo.find_commit(oid).map_err(|e| format!("Failed to find commit: {}", e))?;
    let tree_oid = commit.tree_id();

    cleanup::walk_tree_objects(repo, &tree_oid, &mut objects);

    Ok(objects)
}

fn verify_blob_exists_with_content(repo: &Repository, objects_after: &HashSet<String>, blob_id: &str, expected_content: &str, description: &str) {
    assert!(objects_after.contains(blob_id), "{} blob should exist in object store", description);
    let oid = git2::Oid::from_str(blob_id).expect("Invalid blob ID");
    let blob = repo.find_blob(oid).expect(&format!("Failed to find {} blob", description));
    let content = std::str::from_utf8(blob.content()).expect(&format!("Failed to read {} content", description));
    assert_eq!(content, expected_content, "{} has wrong content", description);
    println!("✓ {} blob preserved with correct content", description);
}

fn verify_blob_removed(objects_after: &HashSet<String>, blob_id: &str, description: &str) {
    assert!(!objects_after.contains(blob_id), "{} blob should have been cleaned up", description);
    println!("✓ {} blob was successfully cleaned up", description);
}

fn verify_file_in_head_with_content(repo: &Repository, objects_after: &HashSet<String>, filename: &str, expected_content: &str) {
    let head_commit = repo.head().unwrap().peel_to_commit().expect("Failed to get HEAD commit");
    let head_tree = head_commit.tree().expect("Failed to get HEAD tree");

    let entry = head_tree.get_name(filename).expect(&format!("{} should be in HEAD", filename));
    let blob = repo.find_blob(entry.id()).expect(&format!("Failed to find {} blob", filename));
    let content = std::str::from_utf8(blob.content()).expect(&format!("Failed to read {} content", filename));
    assert_eq!(content, expected_content, "{} has wrong content", filename);
    assert!(objects_after.contains(&entry.id().to_string()), "{} blob should exist in object store", filename);
    println!("✓ {} preserved with correct content", filename);
}

#[tokio::test]
async fn test_cleanup_old_objects_comprehensive() {
    let (temp_dir, commit_hashes, blob_ids) = create_test_repository().await;
    let repo_path = temp_dir.path();
    let repo = Repository::open(repo_path).expect("Failed to open repository");

    println!("Test repository created at: {:?}", repo_path);
    println!("Commits: {:?}", commit_hashes);

    let objects_before = get_all_objects(repo_path).expect("Failed to get objects before cleanup");
    println!("Objects before cleanup: {}", objects_before.len());

    let old_objects1 = get_objects_for_commit(&repo, &commit_hashes["old_commit"]).unwrap();
    let old_objects2 = get_objects_for_commit(&repo, &commit_hashes["old_commit2"]).unwrap();
    let recent_objects = get_objects_for_commit(&repo, &commit_hashes["recent_commit"]).unwrap();

    println!("Old commit 1 objects: {}", old_objects1.len());
    println!("Old commit 2 objects: {}", old_objects2.len());
    println!("Recent commit objects: {}", recent_objects.len());

    let all_old_objects: HashSet<String> = old_objects1.union(&old_objects2).cloned().collect();
    let should_be_removed: HashSet<String> = all_old_objects.difference(&recent_objects).cloned().collect();
    let should_be_kept: HashSet<String> = recent_objects.clone();

    println!("Should remove: {} objects", should_be_removed.len());
    println!("Should keep: {} objects", should_be_kept.len());
    println!("Objects to remove: {:?}", should_be_removed);
    println!("Objects to keep: {:?}", should_be_kept);

    let removed_count = cleanup::cleanup_old_objects_from_single_repo(repo_path).await.expect("Cleanup failed");
    println!("Cleanup completed: {} objects removed", removed_count);

    let objects_after = get_all_objects(repo_path).expect("Failed to get objects after cleanup");
    println!("Objects after cleanup: {}", objects_after.len());

    verify_blob_removed(&objects_after, &blob_ids["old_removed_blob"], "old_removed_file.txt");
    verify_blob_exists_with_content(&repo, &objects_after, &blob_ids["old_kept_blob"], "This file should be kept", "old_kept_file.txt");

    verify_file_in_head_with_content(&repo, &objects_after, "old_kept_file.txt", "This file should be kept");
    verify_file_in_head_with_content(&repo, &objects_after, "shared_file.txt", "Shared content - version 2 (recent)");
    verify_file_in_head_with_content(&repo, &objects_after, "recent_file.txt", "This file should NOT be cleaned up");

    let missing_objects: HashSet<String> = should_be_kept.difference(&objects_after).cloned().collect();
    assert!(missing_objects.is_empty(), "Expected objects missing after cleanup: {:?}", missing_objects);
    println!("✓ All expected objects preserved");

    let removed_objects: HashSet<String> = objects_before.difference(&objects_after).cloned().collect();
    println!("Actually removed {} objects", removed_objects.len());
    if removed_objects.is_empty() {
        println!("⚠ No objects were actually removed (this might be OK if all objects are shared)");
    } else {
        println!("✓ Some old objects were removed: {:?}", removed_objects);
    }

    // Test that we can make a new commit after cleanup
    let new_file_path = repo_path.join("new_file_after_cleanup.txt");
    fs::write(&new_file_path, "New file after cleanup").expect("Failed to write new file");

    let mut index = repo.index().expect("Failed to get index after cleanup");
    index.add_path(std::path::Path::new("new_file_after_cleanup.txt")).expect("Failed to add new file to index");
    index.write().expect("Failed to write index after cleanup");

    let tree_id = index.write_tree().expect("Failed to write tree after cleanup");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree after cleanup");

    let signature = Signature::now("Test User", "test@example.com").expect("Failed to create signature");
    let head_commit = repo.head().unwrap().peel_to_commit().expect("Failed to get HEAD commit");

    let new_commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "New commit after cleanup",
        &tree,
        &[&head_commit]
    ).expect("Failed to create new commit after cleanup");

    println!("✓ Successfully created new commit after cleanup: {}", new_commit_oid);

    // Verify the new file works correctly
    let objects_final = get_all_objects(repo_path).expect("Failed to get final objects");
    verify_file_in_head_with_content(&repo, &objects_final, "new_file_after_cleanup.txt", "New file after cleanup");

    // Verify repository integrity
    let head_commit = repo.head().unwrap().peel_to_commit().expect("Failed to get HEAD after new commit");
    println!("✓ Repository HEAD accessible: {}", head_commit.message().unwrap_or("No message"));

    println!("✓ Test completed successfully!");
}
