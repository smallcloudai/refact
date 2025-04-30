use std::path::PathBuf;
use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncFilesExtractTarPost {
    pub tar_path: String,
    pub extract_to: String,
}

pub async fn handle_v1_sync_files_extract_tar(
    Extension(_gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SyncFilesExtractTarPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;

    let (tar_path, extract_to) = (PathBuf::from(&post.tar_path), PathBuf::from(&post.extract_to));

    // Create extraction directory if it doesn't exist
    tokio::fs::create_dir_all(&extract_to).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create extraction directory: {}", e)))?;
    
    // Retry logic for extraction
    let max_retries = 3;
    let mut last_error = None;
    
    for retry in 0..max_retries {
        // Check if the tar file exists and is readable
        match tokio::fs::metadata(&tar_path).await {
            Ok(_) => {},
            Err(e) => {
                tracing::warn!("Failed to access tar file (attempt {}/{}): {}", retry + 1, max_retries, e);
                if retry == max_retries - 1 {
                    return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("Can't access tar file: {}", e)));
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }
        };
        
        // Get file size for logging
        let file_size = match tokio::fs::metadata(&tar_path).await {
            Ok(m) => m.len(),
            Err(e) => {
                tracing::warn!("Failed to get tar file metadata: {}", e);
                // Continue with extraction even if we can't get metadata
                0
            }
        };
        
        tracing::info!("Attempting to extract tar file (size: {} bytes, attempt {}/{})", 
                      file_size, retry + 1, max_retries);
        
        // Try to extract using system tar command
        let tar_command = format!(
            "cd {} && tar -xf {}",
            shell_words::quote(&extract_to.to_string_lossy()),
            shell_words::quote(&tar_path.to_string_lossy())
        );
        
        tracing::info!("Executing tar extraction command: {}", tar_command);
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&tar_command)
            .output()
            .await;
            
        match output {
            Ok(output) => {
                if output.status.success() {
                    tracing::info!("Successfully extracted tar file on attempt {}/{}", retry + 1, max_retries);
                    break;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    last_error = Some(format!("Tar extraction command failed: {}", stderr));
                    tracing::warn!("Failed to extract tar file (attempt {}/{}): {}", 
                                  retry + 1, max_retries, last_error.as_ref().unwrap());
                    
                    if retry == max_retries - 1 {
                        return Err(ScratchError::new(
                            StatusCode::INTERNAL_SERVER_ERROR, 
                            format!("Can't unpack tar file: {}", last_error.unwrap())
                        ));
                    }
                    
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
            },
            Err(e) => {
                last_error = Some(e.to_string());
                tracing::warn!("Failed to execute tar extraction command (attempt {}/{}): {}", 
                              retry + 1, max_retries, last_error.as_ref().unwrap());
                
                if retry == max_retries - 1 {
                    return Err(ScratchError::new(
                        StatusCode::INTERNAL_SERVER_ERROR, 
                        format!("Can't execute tar extraction command: {}", last_error.unwrap())
                    ));
                }
                
                // Wait before retrying
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        }
    }

    tokio::fs::remove_file(&tar_path).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Can't remove tar file: {}", e)))?;

    Ok(Response::builder().status(StatusCode::OK).body(Body::from(
        serde_json::to_string(&serde_json::json!({ "success": true })).unwrap()
    )).unwrap())
}