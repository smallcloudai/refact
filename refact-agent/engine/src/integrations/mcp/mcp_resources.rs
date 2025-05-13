use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use rmcp::model::{ResourceContents, ReadResourceRequestParam};

use crate::scratchpads::multimodality::{MultimodalElement, MULTIMODALITY_IMAGE_EXTENSIONS};
use crate::integrations::sessions::IntegrationSession;
use super::session_mcp::SessionMCP;

pub async fn read_resource(
    session_arc: Arc<AMutex<Box<dyn IntegrationSession>>>,
    uri: String
) -> Result<Vec<ResourceContents>, String> {
    let (mcp_client_arc, resource) = {
        let mut session_locked = session_arc.lock().await;
        let session_mcp = session_locked.as_any_mut().downcast_mut::<SessionMCP>()
            .ok_or_else(|| "Failed to downcast session".to_string())?;

        (
            session_mcp.mcp_client.as_ref()
                .ok_or_else(|| "No MCP client in session".to_string())?
                .clone(),
            session_mcp.mcp_resources.as_ref()
                .and_then(|resources| resources.iter().find(|r| r.uri == uri).cloned()),
        )
    };

    let mut resource_content = {
        let mcp_client_locked = mcp_client_arc.lock().await;
        let client = mcp_client_locked.as_ref()
            .ok_or_else(|| "MCP client not running".to_string())?;

        client.read_resource(ReadResourceRequestParam { uri })
            .await
            .map_err(|e| format!("Failed to read resource: {}", e))?
    };

    // Some servers, set the mime_type in the list of resources, but not in the response of read/resource
    if resource_content.contents.len() == 1 {
        if let Some(resource_mime) = resource.and_then(|r| r.mime_type.clone()) {
            match &mut resource_content.contents[0] {
                ResourceContents::BlobResourceContents { mime_type, .. }
                | ResourceContents::TextResourceContents { mime_type, .. } if mime_type.is_none() => {
                    *mime_type = Some(resource_mime);
                }
                _ => {}
            }
        }
    }

    Ok(resource_content.contents)
}

pub fn convert_resource_contents_to_multimodal_elements(resource_contents: Vec<ResourceContents>) -> Vec<MultimodalElement> {
    let mut elements = Vec::new();
    for resource_content in resource_contents {
        match resource_content {
            ResourceContents::TextResourceContents { text, .. } => {
                elements.push(MultimodalElement {
                    m_type: "text".to_string(),
                    m_content: text,
                });
            },
            ResourceContents::BlobResourceContents { blob, mime_type, .. } => {
                match mime_type {
                    Some(mime) if mime.starts_with("image/") => {
                        elements.push(MultimodalElement {
                            m_type: mime.clone(),
                            m_content: blob,
                        });
                    },
                    Some(mime) if MULTIMODALITY_IMAGE_EXTENSIONS.contains(&mime.as_str()) => {
                        elements.push(MultimodalElement {
                            m_type: format!("image/{}", mime),
                            m_content: blob,
                        });
                    },
                    Some(mime) => {
                        elements.push(MultimodalElement {
                            m_type: "text".to_string(),
                            m_content: format!("Resource has unsupported MIME type: {}. Content cannot be displayed.", mime),
                        });
                    },
                    None => {
                        tracing::info!("blob:: {:?}", crate::nicer_logs::first_n_chars(&blob, 300));
                        elements.push(MultimodalElement {
                            m_type: "text".to_string(),
                            m_content: "Resource has no MIME type specified. Content cannot be displayed.".to_string(),
                        });
                    },
                }
            },
        }
    }
    elements
}
