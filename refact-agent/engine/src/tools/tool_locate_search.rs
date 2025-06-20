use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use async_trait::async_trait;
use indexmap::IndexMap;
use hashbrown::HashSet;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, SubchatParameters, ContextFile, PostprocessSettings};
use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::files_correction::{canonicalize_normalized_path, get_project_dirs, preprocess_path_for_normalization}; 
use crate::files_in_workspace::get_file_text_from_memory_or_disk; 
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::tokens::count_text_tokens_with_fallback;


pub struct ToolLocateSearch {
    pub config_path: String,
}

static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;

async fn _make_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    problem_statement: &String,
    important_paths: &Vec<PathBuf>,
    previous_messages: &Vec<ChatMessage>,
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let tokens_extra_budget = (subchat_params.subchat_n_ctx as f32 * TOKENS_EXTRA_BUDGET_PERCENT) as usize;
    let mut tokens_budget: i64 = (subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens - subchat_params.subchat_tokens_for_rag - tokens_extra_budget) as i64;
    let final_message = problem_statement.to_string();
    tokens_budget -= count_text_tokens_with_fallback(None, &final_message) as i64;
    let mut context = "".to_string();
    let mut context_files = vec![];
    for p in important_paths.iter() {
        context_files.push(match get_file_text_from_memory_or_disk(gcx.clone(), &p).await {
            Ok(text) => {
                let total_lines = text.lines().count();
                ContextFile {
                    file_name: p.to_string_lossy().to_string(),
                    file_content: "".to_string(),
                    line1: 1,
                    line2: total_lines.max(1),
                    symbols: vec![],
                    gradient_type: 4,
                    usefulness: 100.0,
                }
            },
            Err(_) => {
                tracing::warn!("failed to read file '{:?}'. Skipping...", p);
                continue;
            }
        })
    }
    for message in previous_messages.iter().rev() {
        let message_row = match message.role.as_str() {
            "system" => {
                continue;
            }
            "user" => {
                format!("👤:\n{}\n\n", &message.content.content_text_only())
            }
            "assistant" => {
                format!("🤖:\n{}\n\n", &message.content.content_text_only())
            }
            "tool" => {
                format!("📎:\n{}\n\n", &message.content.content_text_only())
            }
            _ => {
                tracing::info!("skip adding message to the context: {}", crate::nicer_logs::first_n_chars(&message.content.content_text_only(), 40));
                continue;
            }
        };
        let left_tokens = tokens_budget - count_text_tokens_with_fallback(None, &message_row) as i64;
        if left_tokens < 0 {
            continue;
        } else {
            tokens_budget = left_tokens;
            context.insert_str(0, &message_row);
        }
    }
    if !context_files.is_empty() {
        let mut pp_settings = PostprocessSettings::new();
        pp_settings.max_files_n = context_files.len();
        let mut files_context = "".to_string();
        for context_file in postprocess_context_files(
            gcx.clone(),
            &mut context_files,
            None,
            subchat_params.subchat_tokens_for_rag + tokens_budget.max(0) as usize,
            false, &pp_settings,).await { files_context.push_str(
                &format!("📎 {}:{}-{}\n```\n{}```\n\n",
                         context_file.file_name,
                         context_file.line1,
                         context_file.line2,
                         context_file.file_content)
            );
        }
        Ok(format!("{final_message}\n\n# Conversation\n{context}\n\n# Already explored files\n{files_context}"))
    } else {
        Ok(format!("{final_message}\n\n# Conversation\n{context}"))
    }
}


pub fn check_for_inspected_files(inspected_files: &mut HashSet<String>, messages: &[ChatMessage]) {
    for context_file_msg in messages.iter().filter(|msg| msg.role == "context_file").cloned().collect::<Vec<ChatMessage>>() {
        if let Ok(context_files) = serde_json::from_str::<Vec<ContextFile>>(&context_file_msg.content.content_text_only()) {
            for context_file in context_files {
                inspected_files.insert(context_file.file_name.clone());
            }
        }
    }
}

#[async_trait]
impl Tool for ToolLocateSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "locate".to_string(),
            display_name: "Locate".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Get a list of files that are relevant to solve a particular task.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "what_to_find".to_string(),
                    param_type: "string".to_string(),
                    description: "A short narrative that includes (1) the problem you’re trying to solve, (2) which files or symbols have already been examined, and (3) exactly what additional files, code symbols, or text patterns the agent should locate next".to_string(),
                },
                ToolParam {
                    name: "important_paths".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of all filenames which are already explored.".to_string(),
                },
            ],
            parameters_required: vec!["what_to_find".to_string()],
        }
    }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let what_to_find = match args.get("what_to_find") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `what_to_find` is not a string: {:?}", v)),
            None => return Err("Missing argument `what_to_find`".to_string())
        };
        
        let params = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "locate").await?;
        let gcx = ccx.lock().await.global_context.clone();
        let important_paths = match args.get("important_paths") {
            Some(Value::String(s)) => {
                let mut paths = vec![];
                for s in s.split(",") {
                    let s_raw = s.trim().to_string();
                    let candidates_file = file_repair_candidates(gcx.clone(), &s_raw, 3, false).await;
                    paths.push(match return_one_candidate_or_a_good_error(gcx.clone(), &s_raw, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                        Ok(f) => canonicalize_normalized_path(PathBuf::from(preprocess_path_for_normalization(f.trim().to_string()))),
                        Err(_) => {
                            tracing::info!("cannot find a good file candidate for `{s_raw}`");
                            continue;
                        }
                    })
                }
                paths
            },
            Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
            None => vec![]
        };

        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                8,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
            ).await;
            Arc::new(AMutex::new(t))
        };


        let external_messages = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.messages.clone()
        };
        let prompt = _make_prompt(
            ccx.clone(),
            &params,
            &what_to_find,
            &important_paths,
            &external_messages
        ).await?;
        let (mut results, tool_message, cd_instruction) = find_relevant_files_with_search(
            ccx_subchat,
            tool_call_id,
            params,
            prompt,
        ).await?;

        tracing::info!("\n{}", tool_message);

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        if !cd_instruction.is_empty() {
            tracing::info!("\n{}", cd_instruction);
            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "cd_instruction".to_string(),
                content: ChatContent::SimpleText(cd_instruction),
                tool_calls: None,
                tool_call_id: "".to_string(),
                ..Default::default()
            }));
        }

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}

async fn find_relevant_files_with_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tool_call_id: &str,
    subchat_params: SubchatParameters,
    user_query: String,
) -> Result<(Vec<ContextEnum>, String, String), String> {
    ccx.lock().await.pp_skeleton = true;
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut inspected_files = HashSet::new();
    let mut results: Vec<ContextEnum> = vec![];

    if total_files_in_project == 0 {
        let tool_message = format!("Inspected 0 files, project has 0 files");
        return Ok((results, tool_message, "".to_string()))
    }
    let result = crate::cloud::subchat::subchat(
        ccx.clone(),
        "id:locate:1.0",
        tool_call_id,
        vec![
            ChatMessage::new("user".to_string(), user_query.to_string())
        ],
        subchat_params.subchat_temperature,
        Some(subchat_params.subchat_max_new_tokens),
        subchat_params.subchat_reasoning_effort.clone(),
    ).await?;
    
    check_for_inspected_files(&mut inspected_files, &result);

    let last_message = if let Some(message) = result.iter().rev().find(|msg| msg.role == "assistant").cloned() {
        message
    } else {
        return Err("No assistant messages found in the subchat threads".to_string());
    };
    let assistant_output1 = crate::json_utils::extract_json_object::<IndexMap<String, serde_json::Value>>(last_message.content.content_text_only().as_str()).map_err(|e| {
        tracing::warn!("\n{}\nUnable to parse JSON: {:?}", last_message.content.content_text_only(), e);
        format!("Unable to parse JSON: {:?}", e)
    })?;
    let rejection = assistant_output1.get("rejection");
    if let Some(_rejection_message) = rejection {
        let cd_instruction = format!("💿 locate() looked inside of {} files, workspace has {} files.", inspected_files.len(), total_files_in_project).replace("\n", " ");
        return Ok((results, serde_json::to_string_pretty(&assistant_output1).unwrap(), cd_instruction));
    }

    let assistant_output2 = crate::json_utils::extract_json_object::<IndexMap<String, IndexMap<String, String>>>(last_message.content.content_text_only().as_str()).map_err(|e| {
        tracing::warn!("\n{}\nUnable to parse JSON: {:?}", last_message.content.content_text_only(), e);
        format!("Unable to parse JSON: {:?}", e)
    })?;

    let processed_results = process_assistant_output(&assistant_output2).await?;
    results.extend(processed_results);

    let cd_instruction = format!(r###"💿 locate() looked inside of {} files, workspace has {} files. Files relevant to the task were attached above.
Don't call cat() for the same files, you already have them. Follow your task and the system prompt.
"###, inspected_files.len(), total_files_in_project).replace("\n", " ");

    Ok((results, serde_json::to_string_pretty(&assistant_output2).unwrap(), cd_instruction))
}


async fn process_assistant_output(
    assistant_output: &IndexMap<String, IndexMap<String, String>>,
) -> Result<Vec<ContextEnum>, String> {
    let mut results: Vec<ContextEnum> = vec![];

    for (category, files) in assistant_output.iter() {
        for (file_path, symbols) in files {
            match category.as_str() {
                "FOUND" => {
                    let file_usefulness = match category.as_str() {
                        "FOUND" => 100.0,
                        _ => panic!("unexpected category: {:?}", category),
                    };
                    results.push(ContextEnum::ContextFile(ContextFile {
                        file_name: file_path.clone(),
                        file_content: "".to_string(),
                        line1: 0,
                        line2: 0,
                        symbols: vec![],
                        gradient_type: 4,
                        usefulness: file_usefulness,
                    }));
                },
                "MORE_TOCHANGE" | "SIMILAR" | "USAGE" => {
                    let symbols_vec: Vec<String> = symbols.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let symbol_usefulness = match category.as_str() {
                        "MORE_TOCHANGE" => 75.0,
                        "SIMILAR" => 75.0,
                        "USAGE" => 75.0,
                        _ => panic!("unexpected category: {:?}", category),
                    };
                    for symbol in symbols_vec {
                        results.push(ContextEnum::ContextFile(ContextFile {
                            file_name: file_path.clone(),
                            file_content: "".to_string(),
                            line1: 0,
                            line2: 0,
                            symbols: vec![symbol.clone()],
                            gradient_type: 4,
                            usefulness: symbol_usefulness,
                        }));
                    }
                },
                _ => {
                    tracing::warn!("unexpected category: {:?}", category);
                },
            }
        }
    }

    Ok(results)
}
