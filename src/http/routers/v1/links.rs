use std::sync::Arc;
use std::fs;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize, Serializer};
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, ChatMeta, ChatMode};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::tools::tool_patch_aux::tickets_parsing::get_tickets_from_messages;
use crate::agentic::generate_follow_up_message::generate_follow_up_message;
use crate::git::{get_commit_information_from_current_changes, generate_commit_messages};
use crate::http::routers::v1::git::GitCommitPost;

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

#[derive(Serialize, Debug)]
pub struct Link {
    link_action: LinkAction,
    link_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_goto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_summary_path: Option<String>,
    link_tooltip: String,
    link_payload: Option<LinkPayload>,
}

#[derive(Debug)]
pub enum LinkPayload {
    CommitPayload(GitCommitPost),
}
impl Serialize for LinkPayload {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LinkPayload::CommitPayload(post) => post.serialize(serializer),
        }
    }
}


pub async fn handle_v1_links(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<LinksPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    let mut links: Vec<Link> = Vec::new();
    let mut uncommited_changes_warning = String::new();

    tracing::info!("for links, post.meta.chat_mode == {:?}", post.meta.chat_mode);
    let experimental = gcx.read().await.cmdline.experimental;
    let (integrations_map, integration_yaml_errors) = crate::integrations::running_integrations::load_integrations(gcx.clone(), "".to_string(), experimental).await;

    if post.meta.chat_mode == ChatMode::CONFIGURE {
        if !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
            links.push(Link {
                link_action: LinkAction::PatchAll,
                link_text: "Save and return".to_string(),
                link_goto: Some("SETTINGS:DEFAULT".to_string()),
                link_summary_path: None,
                link_tooltip: format!(""),
                link_payload: None,
            });
        }
    }

    if post.meta.chat_mode == ChatMode::PROJECT_SUMMARY {
        if !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
            links.push(Link {
                link_action: LinkAction::PatchAll,
                link_text: "Save and return".to_string(),
                link_goto: Some("NEWCHAT".to_string()),
                link_summary_path: None,
                link_tooltip: format!(""),
                link_payload: None,
            });
        }
    }

    // GIT uncommitted
    if post.meta.chat_mode == ChatMode::AGENT {
        let commits = get_commit_information_from_current_changes(gcx.clone()).await;

        let mut project_changes = Vec::new();
        for commit in &commits {
            project_changes.push(format!(
                "In project {}:\n{}{}",
                commit.get_project_name(),
                commit.file_changes.iter().take(3).map(|f| format!("{} {}", f.status.initial(), f.path)).collect::<Vec<_>>().join("\n"),
                if commit.file_changes.len() > 3 { "\n...\n" } else { "\n" },
            ));
        }
        if !project_changes.is_empty() && post.messages.is_empty() {
            if project_changes.len() > 4 {
                project_changes.truncate(4);
                project_changes.push("...".to_string());
            }
            uncommited_changes_warning = format!("You have uncommitted changes:\n```\n{}\n```\nIt's fine, but you might have a problem rolling back agent's changes.", project_changes.join("\n"));
        }

        if false {
            for commit_with_msg in generate_commit_messages(gcx.clone(), commits).await {
                let tooltip_message = format!(
                    "git commit -m \"{}{}\"\n{}",
                    commit_with_msg.commit_message.lines().next().unwrap_or(""),
                    if commit_with_msg.commit_message.lines().count() > 1 { "..." } else { "" },
                    commit_with_msg.file_changes.iter().map(|f| format!("{} {}", f.status.initial(), f.path)).collect::<Vec<_>>().join("\n"),
                );
                links.push(Link {
                    link_action: LinkAction::Commit,
                    link_text: format!("Commit {} files in `{}`", commit_with_msg.file_changes.len(), commit_with_msg.get_project_name()),
                    link_goto: Some("LINKS_AGAIN".to_string()),
                    link_summary_path: None,
                    link_tooltip: tooltip_message,
                    link_payload: Some(LinkPayload::CommitPayload(GitCommitPost { commits: vec![commit_with_msg] })),
                });
            }
        }
    }

    // Failures in integrations
    if post.meta.chat_mode == ChatMode::AGENT {
        for failed_integr_name in failed_integration_names_after_last_user_message(&post.messages) {
            links.push(Link {
                link_action: LinkAction::Goto,
                link_text: format!("Configure {failed_integr_name}"),
                link_goto: Some(format!("SETTINGS:{failed_integr_name}")),
                link_summary_path: None,
                link_tooltip: format!(""),
                link_payload: None,
            })
        }
    }

    // YAML problems
    for e in integration_yaml_errors {
        links.push(Link {
            link_action: LinkAction::Goto,
            link_text: format!("Syntax error in {}", crate::nicer_logs::last_n_chars(&e.integr_config_path, 20)),
            link_goto: Some(format!("SETTINGS:{}", e.integr_config_path)),
            link_summary_path: None,
            link_tooltip: format!("Error at line {}: {}", e.error_line, e.error_msg),
            link_payload: None,
        });
    }

    // Tool recommendations
    if post.messages.is_empty() {
        let (already_exists, summary_path_option) = crate::scratchpads::chat_utils_prompts::dig_for_project_summarization_file(gcx.clone()).await;
        if !already_exists {
            // doesn't exist
            links.push(Link {
                link_action: LinkAction::SummarizeProject,
                link_text: "Initial project summarization".to_string(),
                link_goto: None,
                link_summary_path: summary_path_option,
                link_tooltip: format!("Project summary is a starting point for Refact Agent."),
                link_payload: None,
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
                                            if igname == "isolation" || igname == "docker" {
                                                continue;
                                            }
                                            if !integrations_map.contains_key(igname) {
                                                tracing::info!("tool {} not present => link", igname);
                                                links.push(Link {
                                                    link_action: LinkAction::Goto,
                                                    link_text: format!("Configure {igname}"),
                                                    link_goto: Some(format!("SETTINGS:{igname}")),
                                                    link_summary_path: None,
                                                    link_tooltip: format!(""),
                                                    link_payload: None,
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

    // Follow-up
    if false {
        if post.meta.chat_mode != ChatMode::NO_TOOLS && links.is_empty() && post.messages.len() > 2 {
            let follow_up_messages: Vec<String> = generate_follow_up_message(post.messages.clone(), gcx.clone(), &post.model_name, &post.meta.chat_id).await
                .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error generating follow-up message: {}", e)))?;
            for follow_up_message in follow_up_messages {
                tracing::info!("follow-up {:?}", follow_up_message);
                links.push(Link {
                    link_action: LinkAction::FollowUp,
                    link_text: follow_up_message,
                    link_goto: None,
                    link_summary_path: None,
                    link_tooltip: format!(""),
                    link_payload: None,
                });
            }
        }
    }

    tracing::info!("generated links2: {:?}", links);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&serde_json::json!({
            "links": links,
            "uncommited_changes_warning": uncommited_changes_warning,
        })).unwrap())).unwrap())
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
