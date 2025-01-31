use std::sync::Arc;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatMessage, ChatMeta, ChatMode};
use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::integrations::go_to_configuration_message;
use crate::tools::tool_patch_aux::tickets_parsing::get_tickets_from_messages;
use crate::agentic::generate_follow_up_message::generate_follow_up_message;
use crate::git::commit_info::{get_commit_information_from_current_changes, generate_commit_messages};
// use crate::http::routers::v1::git::GitCommitPost;

#[derive(Deserialize, Clone, Debug)]
pub struct LinksPost {
    messages: Vec<ChatMessage>,
    model_name: String,
    meta: ChatMeta,
}

#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum LinkAction {
    #[default]
    PatchAll,
    FollowUp,
    Commit,
    Goto,
    SummarizeProject,
    PostChat,
    RegenerateWithIncreasedContextSize,
}

#[derive(Default, Serialize, Debug)]
pub struct Link {
    link_action: LinkAction,
    link_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_goto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_summary_path: Option<String>,
    link_tooltip: String,
    #[serde(default, skip_serializing_if = "is_default_json_value")]
    link_payload: serde_json::Value,
}

fn is_default_json_value(value: &serde_json::Value) -> bool {
    value == &serde_json::Value::Null
}

fn last_message_assistant_without_tools(messages: &Vec<ChatMessage>) -> bool {
    if let Some(m) = messages.last() {
        m.role == "assistant" && m.tool_calls.as_ref().map(|x| x.is_empty()).unwrap_or(true)
    } else {
        false
    }
}

fn last_message_stripped_assistant(messages: &Vec<ChatMessage>) -> bool {
    if let Some(m) = messages.last() {
        m.role == "assistant" && m.finish_reason == Some("length".to_string())
    } else {
        false
    }
}

async fn trunc_pinned_message_link(
    gcx: Arc<ARwLock<GlobalContext>>, 
    messages: &Vec<ChatMessage>,
    chat_mode: &ChatMode,
) -> Option<String> {
    if let Some(m) = messages.last() {
        let tickets = crate::tools::tool_patch_aux::tickets_parsing::parse_tickets(
            gcx.clone(), &m.content.content_text_only(), messages.len() - 1,
        ).await;
        
        let truncated_ids = tickets
            .iter()
            .filter(|t| t.is_truncated)
            .map(|x| x.id.to_string())
            .join(", ");
        
        match chat_mode {
            ChatMode::AGENT => {
                if !truncated_ids.is_empty() {
                    Some(format!("Regenerate truncated {truncated_ids} 📍-tickets and continue to generate others (if needed). Then use patch() to apply them"))
                } else {
                    None
                }
            }
            _ => {
                if !truncated_ids.is_empty() {
                    Some(format!("Regenerate truncated {truncated_ids} 📍-tickets and continue to generate others (if needed)"))
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}

async fn apply_patch_promptly_link(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ChatMessage>,
) -> Option<String> {
    if let Some(m) = messages.last() {
        let tickets = crate::tools::tool_patch_aux::tickets_parsing::parse_tickets(
            gcx.clone(), &m.content.content_text_only(), messages.len() - 1,
        ).await;

        let has_truncated_tickets = tickets.iter().filter(|t| t.is_truncated).count() > 0;
        let ids_to_apply = tickets
            .iter()
            .filter(|t| !t.is_truncated)
            .map(|x| x.id.to_string())
            .join(", ");
        if !has_truncated_tickets && !ids_to_apply.is_empty() {
            Some(format!("Use patch() to apply tickets: {ids_to_apply}"))
        } else {
            None
        }
    } else {
        None
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
    let (_integrations_map, integration_yaml_errors) = crate::integrations::running_integrations::load_integrations(gcx.clone(), experimental).await;

    // if post.meta.chat_mode == ChatMode::CONFIGURE {
    //     if !get_tickets_from_messages(gcx.clone(), &post.messages).await.is_empty() {
    //         links.push(Link {
    //             link_action: LinkAction::PatchAll,
    //             link_text: "Save and return".to_string(),
    //             link_goto: Some("SETTINGS:DEFAULT".to_string()),
    //             link_summary_path: None,
    //             link_tooltip: format!(""),
    //             link_payload: None,
    //         });
    //     }
    // }

    if post.meta.chat_mode == ChatMode::PROJECT_SUMMARY {
        if !get_tickets_from_messages(gcx.clone(), &post.messages, None).await.is_empty() {
            links.push(Link {
                link_action: LinkAction::PatchAll,
                link_text: "Save and return".to_string(),
                link_goto: Some("NEWCHAT".to_string()),
                link_summary_path: None,
                link_tooltip: format!(""),
                ..Default::default()
            });
        } else if last_message_assistant_without_tools(&post.messages) {
            links.push(Link {
                link_action: LinkAction::FollowUp,
                link_text: "Looks alright! Please, save the generated summary!".to_string(),
                link_goto: None,
                link_summary_path: None,
                link_tooltip: format!(""),
                ..Default::default()
            });
        }
    }

    // GIT uncommitted
    if post.meta.chat_mode == ChatMode::AGENT && post.messages.is_empty() {
        let commits = get_commit_information_from_current_changes(gcx.clone()).await;

        let mut s = Vec::new();
        for commit in &commits {
            s.push(format!(
                "In project {}:\n{}{}",
                commit.get_project_name(),
                commit.file_changes.iter().take(3).map(|f| format!("{} {}", f.status.initial(), f.relative_path.to_string_lossy())).collect::<Vec<_>>().join("\n"),
                if commit.file_changes.len() > 3 { format!("\n...{} files more\n", commit.file_changes.len() - 3) } else { format!("\n") },
            ));
        }
        if !s.is_empty() {
            if s.len() > 5 {
                let omitted_projects = s.len() - 4;
                s.truncate(4);
                s.push(format!("...{} projects more", omitted_projects));
            }
            uncommited_changes_warning = format!("You have uncommitted changes:\n```\n{}\n```\nIt's fine, but you might have a problem rolling back agent's changes.", s.join("\n"));
        }

        if false {
            for commit_with_msg in generate_commit_messages(gcx.clone(), commits).await {
                let tooltip_message = format!(
                    "git commit -m \"{}{}\"\n{}",
                    commit_with_msg.commit_message.lines().next().unwrap_or(""),
                    if commit_with_msg.commit_message.lines().count() > 1 { "..." } else { "" },
                    commit_with_msg.file_changes.iter().map(|f| format!("{} {}", f.status.initial(), f.relative_path.to_string_lossy())).collect::<Vec<_>>().join("\n"),
                );
                links.push(Link {
                    link_action: LinkAction::Commit,
                    link_text: format!("Commit {} files in `{}`", commit_with_msg.file_changes.len(), commit_with_msg.get_project_name()),
                    link_goto: Some("LINKS_AGAIN".to_string()),
                    link_summary_path: None,
                    link_tooltip: tooltip_message,
                    link_payload: serde_json::json!({ "commits": [commit_with_msg] }),
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
                ..Default::default()
            })
        }
    
        // YAML problems
        for e in integration_yaml_errors {
            links.push(Link {
                link_action: LinkAction::Goto,
                link_text: format!("Syntax error in {}", crate::nicer_logs::last_n_chars(&e.integr_config_path, 20)),
                link_goto: Some(format!("SETTINGS:{}", e.integr_config_path)),
                link_summary_path: None,
                link_tooltip: format!("Error at line {}: {}", e.error_line, e.error_msg),
                ..Default::default()
            });
        }
    }
    
    // RegenerateWithIncreasedContextSize
    if last_message_stripped_assistant(&post.messages) {
        links.push(Link {
            link_action: LinkAction::RegenerateWithIncreasedContextSize,
            link_text: format!("Increase tokens limit and regenerate last message"),
            link_goto: None,
            link_summary_path: None,
            link_tooltip: format!(""),
            ..Default::default()
        });
        if let Some(msg) = trunc_pinned_message_link(gcx.clone(), &post.messages, &post.meta.chat_mode).await {
            links.push(Link {
                link_action: LinkAction::FollowUp,
                link_text: msg,
                link_goto: None,
                link_summary_path: None,
                link_tooltip: format!(""),
                ..Default::default()
            });
        } else {
            links.push(Link {
                link_action: LinkAction::FollowUp,
                link_text: "Complete the previous message from where it was left off".to_string(),
                link_goto: None,
                link_summary_path: None,
                link_tooltip: format!(""),
                ..Default::default()
            });
        }
    }
    
    // Link to apply 📍 tickets promptly
    if post.meta.chat_mode == ChatMode::AGENT {
        if let Some(msg) = apply_patch_promptly_link(gcx.clone(), &post.messages).await {
            links.push(Link {
                link_action: LinkAction::FollowUp,
                link_text: msg,
                link_goto: None,
                link_summary_path: None,
                link_tooltip: format!(""),
                ..Default::default()
            });
        }
    }

    // Tool recommendations
    /* temporary remove project summary and recomended integrations 
    if (post.meta.chat_mode == ChatMode::AGENT) {
        if post.messages.is_empty() {
            let (summary_exists, summary_path_option) = crate::scratchpads::chat_utils_prompts::dig_for_project_summarization_file(gcx.clone()).await;
            if !summary_exists {
                // doesn't exist
                links.push(Link {
                    link_action: LinkAction::SummarizeProject,
                    link_text: "Initial project summarization".to_string(),
                    link_goto: None,
                    link_summary_path: summary_path_option,
                    link_tooltip: format!("Project summary is a starting point for Refact Agent."),
                    ..Default::default()
                });
            } else {
                // exists
                if let Some(summary_path) = summary_path_option {
                    match fs::read_to_string(&summary_path) {
                        Ok(content) => {
                            match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                                Ok(yaml) => {
                                    if let Some(recommended_integrations) = yaml.get("recommended_integrations").and_then(|rt| rt.as_sequence()) {
                                        let mut any_recommended = false;
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
                                                        ..Default::default()
                                                    });
                                                    any_recommended = true;
                                                } else {
                                                    tracing::info!("tool {} present => happy", igname);
                                                }
                                            }
                                        }
                                        if any_recommended {
                                            links.push(Link {
                                                link_action: LinkAction::PostChat,
                                                link_text: format!("Stop recommending integrations"),
                                                link_goto: None,
                                                link_summary_path: None,
                                                link_tooltip: format!(""),
                                                link_payload: serde_json::json!({
                                                    "chat_meta": crate::call_validation::ChatMeta {
                                                        chat_id: "".to_string(),
                                                        chat_remote: false,
                                                        chat_mode: crate::call_validation::ChatMode::CONFIGURE,
                                                        current_config_file: summary_path.clone(),
                                                    },
                                                    "messages": [
                                                        crate::call_validation::ChatMessage {
                                                            role: "user".to_string(),
                                                            content: crate::call_validation::ChatContent::SimpleText(format!("Make recommended_integrations an empty list, follow the system prompt.")),
                                                            ..Default::default()
                                                        },
                                                    ]
                                                }),
                                            });
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
    }
    */

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
                    ..Default::default()
                });
            }
        }
    }

    tracing::info!("generated links2\n{}", serde_json::to_string_pretty(&links).unwrap());

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
