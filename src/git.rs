use git2::{Branch, BranchType, IndexAddOption, Oid, Repository, Signature, Status};

/// Similar to git checkout -b <branch_name>
pub fn create_or_checkout_to_branch<'repo>(repository: &'repo Repository, branch_name: &str) -> Result<Branch<'repo>, String> {
    let branch = match repository.find_branch(branch_name, BranchType::Local) {
        Ok(branch) => branch,
        Err(_) => {
            let head_commit = repository.head()
                .and_then(|h| h.peel_to_commit())
                .map_err(|e| format!("Failed to get HEAD commit: {}", e))?;
            repository.branch(branch_name, &head_commit, false)
                .map_err(|e| format!("Failed to create branch: {}", e))?
        }
    };

    // Checkout to the branch
    let object = repository.revparse_single(&("refs/heads/".to_owned() + branch_name))
        .map_err(|e| format!("Failed to revparse single: {}", e))?;
    repository.checkout_tree(&object, None)
        .map_err(|e| format!("Failed to checkout tree: {}", e))?;
    repository.set_head(&format!("refs/heads/{}", branch_name))
      .map_err(|e| format!("Failed to set head: {}", e))?;

    Ok(branch)
}

/// Similar to git add .
pub fn stage_all_changes(repository: &Repository) -> Result<(), String> {
    let mut index = repository.index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|e| format!("Failed to add files to index: {}", e))?;
    index.write()
        .map_err(|e| format!("Failed to write index: {}", e))?;
    Ok(()) 
}

/// Returns:
/// 
/// A tuple containing the number of new files, modified files, and deleted files.
pub fn count_file_changes(repository: &Repository) -> Result<(usize, usize, usize), String> {
    let (mut new_files, mut modified_files, mut deleted_files) = (0, 0, 0);

    let statuses = repository.statuses(None)
        .map_err(|e| format!("Failed to get statuses: {}", e))?;
    for entry in statuses.iter() {
        let status = entry.status();
        if status.contains(Status::INDEX_NEW) { new_files += 1; }
        if status.contains(Status::INDEX_MODIFIED) { modified_files += 1;}
        if status.contains(Status::INDEX_DELETED) { deleted_files += 1; }
    }

    Ok((new_files, modified_files, deleted_files))
}

pub fn commit(repository: &Repository, branch: &Branch, message: &str, author_name: &str, author_email: &str) -> Result<Oid, String> {
    
    let mut index = repository.index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    let tree_id = index.write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repository.find_tree(tree_id)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    let signature = Signature::now(author_name, author_email)
        .map_err(|e| format!("Failed to create signature: {}", e))?;

    let branch_ref_name = branch.get().name()
        .ok_or_else(|| "Invalid branch name".to_string())?;

    let parent_commit = if let Some(target) = branch.get().target() {
        repository.find_commit(target)
            .map_err(|e| format!("Failed to find branch commit: {}", e))?
    } else {
        return Err("No parent commits found (initial commit is not supported)".to_string());
    };

    repository.commit(
        Some(branch_ref_name), &signature, &signature, message, &tree, &[&parent_commit]
    ).map_err(|e| format!("Failed to create commit: {}", e))
}

