use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs};
use crate::global_context::GlobalContext;

/// Resolves a scope string into a list of files to search.
/// 
/// # Arguments
/// 
/// * `gcx` - Global context
/// * `scope` - Scope string, can be "workspace", a directory path (ending with / or \), or a file path
/// 
/// # Returns
/// 
/// * `Ok(Vec<String>)` - List of file paths to search
/// * `Err(String)` - Error message if scope resolution fails
/// 
/// # Examples
/// 
/// ```
/// let files = resolve_scope(gcx.clone(), "workspace").await?;
/// let files = resolve_scope(gcx.clone(), "src/").await?;
/// let files = resolve_scope(gcx.clone(), "src/main.rs").await?;
/// ```
pub async fn resolve_scope(
    gcx: Arc<ARwLock<GlobalContext>>,
    scope: &str,
) -> Result<Vec<String>, String> {
    let scope_string = scope.to_string();
    // Case 1: Workspace scope
    if scope == "workspace" {
        let workspace_files = gcx.read().await.documents_state.workspace_files.lock().unwrap().clone();
        return Ok(workspace_files.into_iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>());
    }
    
    // Check if scope is a directory (ends with / or \)
    let scope_is_dir = scope.ends_with('/') || scope.ends_with('\\');
    
    // Case 2: Directory scope
    if scope_is_dir {
        let dir_path = return_one_candidate_or_a_good_error(
            gcx.clone(),
            &scope_string,
            &correct_to_nearest_dir_path(gcx.clone(), &scope_string, false, 10).await,
            &get_project_dirs(gcx.clone()).await,
            true,
        ).await?;
        
        let workspace_files = gcx.read().await.documents_state.workspace_files.lock().unwrap().clone();
        return Ok(workspace_files.into_iter()
            .filter(|f| f.starts_with(&dir_path))
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>());
    }
    
    // Case 3: File scope (with fallback to directory if file not found)
    match return_one_candidate_or_a_good_error(
        gcx.clone(),
        &scope_string,
        &file_repair_candidates(gcx.clone(), &scope_string, 10, false).await,
        &get_project_dirs(gcx.clone()).await,
        false,
    ).await {
        // File found
        Ok(file_path) => Ok(vec![file_path]),
        
        // File not found, try as directory
        Err(file_err) => {
            match return_one_candidate_or_a_good_error(
                gcx.clone(),
                &scope_string,
                &correct_to_nearest_dir_path(gcx.clone(), &scope_string, false, 10).await,
                &get_project_dirs(gcx.clone()).await,
                true,
            ).await {
                // Directory found
                Ok(dir_path) => {
                    let workspace_files = gcx.read().await.documents_state.workspace_files.lock().unwrap().clone();
                    Ok(workspace_files.into_iter()
                        .filter(|f| f.starts_with(&dir_path))
                        .map(|f| f.to_string_lossy().to_string())
                        .collect::<Vec<_>>())
                },
                // Neither file nor directory found
                Err(_) => Err(file_err),
            }
        },
    }
}

/// Creates a SQL-like filter string for the given scope.
/// This is specifically for the search tool which uses SQL-like filters.
/// 
/// # Arguments
/// 
/// * `gcx` - Global context
/// * `scope` - Scope string
/// 
/// # Returns
/// 
/// * `Ok(Option<String>)` - SQL-like filter string, or None for workspace scope
/// * `Err(String)` - Error message if scope resolution fails
pub async fn create_scope_filter(
    gcx: Arc<ARwLock<GlobalContext>>,
    scope: &str,
) -> Result<Option<String>, String> {
    let scope_string = scope.to_string();
    if scope == "workspace" {
        return Ok(None);
    }
    
    let scope_is_dir = scope.ends_with('/') || scope.ends_with('\\');
    
    if scope_is_dir {
        let dir_path = return_one_candidate_or_a_good_error(
            gcx.clone(),
            &scope_string,
            &correct_to_nearest_dir_path(gcx.clone(), &scope_string, false, 10).await,
            &get_project_dirs(gcx.clone()).await,
            true,
        ).await?;
        
        return Ok(Some(format!("(scope LIKE '{}%')", dir_path)));
    }
    
    match return_one_candidate_or_a_good_error(
        gcx.clone(),
        &scope_string,
        &file_repair_candidates(gcx.clone(), &scope_string, 10, false).await,
        &get_project_dirs(gcx.clone()).await,
        false,
    ).await {
        Ok(file_path) => Ok(Some(format!("(scope = \"{}\")", file_path))),
        Err(file_err) => {
            match return_one_candidate_or_a_good_error(
                gcx.clone(),
                &scope_string,
                &correct_to_nearest_dir_path(gcx.clone(), &scope_string, false, 10).await,
                &get_project_dirs(gcx.clone()).await,
                true,
            ).await {
                Ok(dir_path) => Ok(Some(format!("(scope LIKE '{}%')", dir_path))),
                Err(_) => Err(file_err),
            }
        },
    }
}

/// Validates that the scope is not empty and returns an appropriate error message if it is.
/// 
/// # Arguments
/// 
/// * `files` - List of files resolved from the scope
/// * `scope` - Original scope string for error reporting
/// 
/// # Returns
/// 
/// * `Ok(Vec<String>)` - The same list of files if not empty
/// * `Err(String)` - Error message if the list is empty
pub fn validate_scope_files(
    files: Vec<String>,
    scope: &str,
) -> Result<Vec<String>, String> {
    if files.is_empty() {
        Err(format!("No files found in scope: {}", scope))
    } else {
        Ok(files)
    }
}