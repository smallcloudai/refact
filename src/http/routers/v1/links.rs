use std::sync::Arc;
use std::fs;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tracing::error;

use crate::agentic::generate_commit_message::generate_commit_message_by_diff;
use crate::call_validation::{ChatMessage, ChatMeta, ChatMode};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::tools::tool_patch_aux::tickets_parsing::get_tickets_from_messages;
use crate::agentic::generate_follow_up_message::generate_follow_up_message;

#[derive(Deserialize, Clone, Debug)]
pub struct LinksPost {
    messages: Vec<ChatMessage>,
    model_name: String,
    meta: ChatMeta,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum LinkAction {
    PatchAll,
    FollowUp,
    Commit,
    Goto,
    SummarizeProject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Link {
    // XXX rename:
    // link_action
    // link_text
    // link_goto
    // link_tooltip
    action: LinkAction,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // projects: Option<Vec<ProjectCommit>>,
    current_config_file: Option<String>,   // XXX rename
    link_tooltip: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectCommit {
    path: String,
    commit_message: String,
}

pub async fn handle_v1_links(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LinksPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let mut links = Vec::new();
    tracing::info!("for links, post.meta.chat_mode == {:?}", post.meta.chat_mode);
    let (integrations_map, integration_yaml_errors) = crate::integrations::running_integrations::load_integrations(gcx.clone(), "".to_string(), gcx.read().await.cmdline.experimental).await;

    if post.messages.is_empty() {
        let (already_exists, summary_path_option) = crate::scratchpads::chat_utils_prompts::dig_for_project_summarization_file(gcx.clone()).await;
        if !already_exists {
            // doesn't exist
            links.push(Link {
                action: LinkAction::SummarizeProject,
                text: "Initial project summarization".to_string(),
                goto: None,
                current_config_file: summary_path_option,
                link_tooltip: format!("Project summary is a starting point for Refact Agent."),
            });
        } else {
            // exists
            if let Some(summary_path) = summary_path_option {
                match fs::read_to_string(&summary_path) {
                    Ok(content) => {
                        match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                            Ok(yaml) => {
                                if let Some(recommended_integrations) = yaml.get("recommended_integrations").and_then(|rt| rt.as_sequence()) {
                                    for igname_value in recommended_integrations {
                                        if let Some(igname) = igname_value.as_str() {
                                            if !integrations_map.contains_key(igname) {
                                                tracing::info!("tool {} not present => link", igname);
                                                links.push(Link {
                                                    action: LinkAction::Goto,
                                                    text: format!("Configure {igname}"),
                                                    goto: Some(format!("SETTINGS:{igname}")),
                                                    current_config_file: None,
                                                    link_tooltip: format!(""),
                                                });
                                            } else {
                                                tracing::info!("tool {} present => happy", igname);
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                tracing::error!("Failed to parse project summary YAML file: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to read project summary file: {}", e);
                    }
                }
            }
        }
    }

    if post.meta.chat_mode == ChatMode::CONFIGURE {
        links.push(Link {
            action: LinkAction::Goto,
            text: "Return".to_string(),
            goto: Some("SETTINGS:DEFAULT".to_string()),
            current_config_file: None,
            link_tooltip: format!(""),
        });
        
        if !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
            links.push(Link {
                action: LinkAction::PatchAll,
                text: "Save and return".to_string(),
                goto: Some("SETTINGS:DEFAULT".to_string()),
                current_config_file: None,
                link_tooltip: format!(""),
            });
        }
    }

    // if post.meta.chat_mode == ChatMode::AGENT {
    //     let (project_commits, files_changed) = generate_commit_messages_with_current_changes(gcx.clone()).await;
    //     if !project_commits.is_empty() {
    //         links.push(Link {
    //             action: LinkAction::Commit,
    //             text: format!("Commit {files_changed} files"),
    //             goto: None,
    //             // projects: Some(project_commits),
    //             current_config_file: None,
    //             link_tooltip: format!(""),
    //         });
    //     }
    // }

    if post.meta.chat_mode == ChatMode::AGENT {
        for failed_integr_name in failed_integration_names_after_last_user_message(&post.messages) {
            links.push(Link {
                action: LinkAction::Goto,
                text: format!("Configure {failed_integr_name}"),
                goto: Some(format!("SETTINGS:{failed_integr_name}")),
                current_config_file: None,
                link_tooltip: format!(""),
            })
        }
    }

    for e in integration_yaml_errors {
        links.push(Link {
            action: LinkAction::Goto,
            text: format!("Syntax error in {}", crate::nicer_logs::last_n_chars(&e.integr_config_path, 20)),
            goto: Some(format!("SETTINGS:{}", e.integr_config_path)),
            current_config_file: None,
            link_tooltip: format!("Error at line {}: {}", e.error_line, e.error_msg),
        });
    }

    // hmm maybe (post.meta.chat_mode == ChatMode::EXPLORE || post.meta.chat_mode == ChatMode::AGENT)
    if post.meta.chat_mode != ChatMode::NO_TOOLS && links.is_empty() && post.messages.len() > 2 {
        let follow_up_messages: Vec<String> = generate_follow_up_message(post.messages.clone(), gcx.clone(), &post.model_name, &post.meta.chat_id).await
            .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error generating follow-up message: {}", e)))?;
        for follow_up_message in follow_up_messages {
            tracing::info!("follow-up {:?}", follow_up_message);
            links.push(Link {
                action: LinkAction::FollowUp,
                text: follow_up_message,
                goto: None,
                current_config_file: None,
                link_tooltip: format!(""),
            });
        }
    }

    tracing::info!("generated links2: {:?}", links);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&serde_json::json!({"links": links})).unwrap()))
        .unwrap())
}

async fn generate_commit_messages_with_current_changes(gcx: Arc<ARwLock<GlobalContext>>) -> (Vec<ProjectCommit>, usize) {
    let mut project_commits = Vec::new();
    let mut total_file_changes = 0;

    for project_path in crate::files_correction::get_project_dirs(gcx.clone()).await {
        let repository = match git2::Repository::open(&project_path) {
            Ok(repo) => repo,
            Err(e) => { error!("{}", e); continue; }
        };

        let (added, modified, deleted) = match crate::git::count_file_changes(&repository, true) {
            Ok((0, 0, 0)) => { continue; }
            Ok(changes) => changes,
            Err(e) => { error!("{}", e); continue; }
        };

        let diff = match crate::git::git_diff_from_all_changes(&repository) {
            Ok(d) if d.is_empty() => { continue; }
            Ok(d) => d,
            Err(e) => { error!("{}", e); continue; }
        };

        let commit_msg = match generate_commit_message_by_diff(gcx.clone(), &diff, &None).await {
            Ok(msg) => msg,
            Err(e) => { error!("{}", e); continue; }
        };

        project_commits.push(ProjectCommit {
            path: project_path.to_string_lossy().to_string(),
            commit_message: commit_msg,
        });
        total_file_changes += added + modified + deleted;
    }

    (project_commits, total_file_changes)
}

fn failed_integration_names_after_last_user_message(messages: &Vec<ChatMessage>) -> Vec<String> {
    let last_user_msg_index = messages.iter().rposition(|m| m.role == "user").unwrap_or(0);
    let tool_calls = messages[last_user_msg_index..].iter().filter(|m| m.role == "assistant")
        .filter_map(|m| m.tool_calls.as_ref()).flatten().collect::<Vec<_>>();

    let mut result = Vec::new();
    for tool_call in tool_calls {
        if let Some(answer_text) = messages.iter()
            .find(|m| m.role == "tool" && m.tool_call_id == tool_call.id)
            .map(|m| m.content.content_text_only()) {
            if answer_text.contains(&go_to_configuration_message(&tool_call.function.name)) {
                result.push(tool_call.function.name.clone());
            }
        }
    }
    result.sort();
    result.dedup();
    result
}
