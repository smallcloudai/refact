use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use async_trait::async_trait;
use axum::http::StatusCode;
use indexmap::IndexMap;
use hashbrown::HashSet;
use crate::subchat::subchat;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters, ContextFile, PostprocessSettings};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::caps::resolve_chat_model;
use crate::custom_error::ScratchError;
use crate::files_correction::{canonicalize_normalized_path, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::get_file_text_from_memory_or_disk;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::tokens::count_text_tokens_with_fallback;

pub struct ToolLocateSearch;


const LS_SYSTEM_PROMPT: &str = r###"**Task**
Locate every file or symbol relevant to the problem described in the conversation.  
> **Important:** If the conversation already supplies certain files, treat them as fully reviewed and **focus on finding *additional* relevant files or symbols**. Do not stop until you have exhausted the project for new, useful artefacts.

**Available tools**
- `tree()`                     — view the project directory tree
- `search_symbol_definition()` — locate symbol definitions
- `search_symbol_usages()`     — locate call sites
- `search_pattern()`           — regex search
- `search_semantic()`          — broader semantic search

**Workflow**
1. **Plan** – Sketch a quick strategy: which tool you’ll start with and why.  
2. **Investigate iteratively**  
   - Run a tool.  
   - Interpret the output.  
   - Decide your next step.  
   - Repeat until no new relevant artefacts remain.  
3. **Explain** – Briefly justify each action as you take it.  
4. **Report** – End with a concise summary listing all newly discovered files/symbols and why they matter.
"###;


const LS_WRAP_UP: &str = r###"Inspect the task description and the files collected so far, then sort the relevant paths into the JSON structure below.

Guidelines
----------

0. **Sanity-check the task**  
   If, after reviewing the files, the task itself is impossible or incoherent, populate the `"rejection"` field with a **specific** reason and stop.

1. **Determine what must be found or changed**  
   • If the task is *find-only*, list the target files/symbols under **FOUND**.  
   • If the task requires *code changes*, put the one or two files that must change under **FOUND**.  
   • If the change belongs in an *entirely new* file, use **NEW_FILE** instead.  
   • If the task clearly needs changes but no files qualify, reject.

2. **Pick reference material for analogies**  
   If the task says “implement by analogy” or you see near-duplicate code, list up to **3** of the *best* reference files (not already in FOUND) under **SIMILAR**. Zero is fine.

3. **Flag additional impact**  
   *MORE_TOCHANGE* – Up to **3** small, simple files you are **reasonably sure** will also need edits.  
   *USAGE* – Up to **3** files that **call or depend on** the code you will change. Name the exact symbols being used.

4. **Be sparing**  
   Irrelevant files hurt more than missing ones. If uncertain, leave it out.

Output format
-------------

```json
{
  "rejection": "string explaining the concrete mismatch, if any"
}
```

or

```json
{
  "NEW_FILE": {                    // Omit if no new files are required
    "dir/new_module.py": ""
  },
  "FOUND": {                       // Must not be empty for change tasks
    "core/handler.py": "process_event,handle_error"
  },
  "SIMILAR": {
    "core/legacy_handler.py": "process_event"
  },
  "MORE_TOCHANGE": {
    "api/views.py": "EventView"
  },
  "USAGE": {
    "tests/test_handler.py": "process_event",
    "app/main.py": "handle_error"
  }
}

DO NOT CALL ANY TOOLS ANYMORE!
```"###;

static TOKENS_EXTRA_BUDGET_PERCENT: f32 = 0.06;


async fn _make_prompt(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    problem_statement: &String,
    important_paths: &Vec<PathBuf>,
    previous_messages: &Vec<ChatMessage>,
) -> Result<String, String> {
    let gcx = ccx.lock().await.global_context.clone();
    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err(|x| x.message)?;
    let model_rec = resolve_chat_model(caps, &subchat_params.subchat_model)?;
    let tokenizer = crate::tokens::cached_tokenizer(gcx.clone(), &model_rec.base).await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e)).map_err(|x| x.message)?;
    let tokens_extra_budget = (subchat_params.subchat_n_ctx as f32 * TOKENS_EXTRA_BUDGET_PERCENT) as usize;
    let mut tokens_budget: i64 = (subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens - subchat_params.subchat_tokens_for_rag - tokens_extra_budget) as i64;
    let final_message = problem_statement.to_string();
    tokens_budget -= count_text_tokens_with_fallback(tokenizer.clone(), &final_message) as i64;
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
        let left_tokens = tokens_budget - count_text_tokens_with_fallback(tokenizer.clone(), &message_row) as i64;
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
            tokenizer.clone(),
            subchat_params.subchat_tokens_for_rag + tokens_budget.max(0) as usize,
            false,
            &pp_settings,
        ).await {
            files_context.push_str(
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


#[async_trait]
impl Tool for ToolLocateSearch {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
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
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                1,
                false,
                ccx_lock.messages.clone(),
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
                ccx_lock.current_model.clone(),
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };


        let external_messages = {
            let ccx_lock = ccx.lock().await;
            ccx_lock.messages.clone()
        };
        let prompt = _make_prompt(
            ccx.clone(),
            &params,
            &LS_SYSTEM_PROMPT.to_string(),
            &important_paths,
            &external_messages
        ).await?;
        let (mut results, usage, tool_message, cd_instruction) = find_relevant_files_with_search(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            prompt,
        ).await?;

        tracing::info!("\n{}", tool_message);

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage),
            ..Default::default()
        }));

        if !cd_instruction.is_empty() {
            tracing::info!("\n{}", cd_instruction);
            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "cd_instruction".to_string(),
                content: ChatContent::SimpleText(cd_instruction),
                tool_calls: None,
                tool_call_id: "".to_string(),
                usage: None,
                ..Default::default()
            }));
        }

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}

async fn find_relevant_files_with_search(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: SubchatParameters,
    tool_call_id: String,
    user_query: String,
) -> Result<(Vec<ContextEnum>, ChatUsage, String, String), String> {
    ccx.lock().await.pp_skeleton = true;
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let total_files_in_project = gcx.read().await.documents_state.workspace_files.lock().unwrap().len();

    let mut usage = ChatUsage { ..Default::default() };
    // let mut real_files = IndexMap::new();
    let mut inspected_files = HashSet::new();
    let mut results: Vec<ContextEnum> = vec![];

    if total_files_in_project == 0 {
        let tool_message = format!("Inspected 0 files, project has 0 files");
        return Ok((results, usage, tool_message, "".to_string()))
    }

    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut msgs = vec![];
    msgs.push(ChatMessage::new("system".to_string(), LS_SYSTEM_PROMPT.to_string()));
    msgs.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let result = subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        msgs,
        vec![
            "tree".to_string(),
            "search_symbol_definition".to_string(), "search_symbol_usages".to_string(),
            "search_pattern".to_string(), "search_semantic".to_string(),
        ],
        16,
        subchat_params.subchat_max_new_tokens,
        LS_WRAP_UP,
        1,
        None,
        subchat_params.subchat_reasoning_effort,
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-search")),
        Some(false),  
    ).await?[0].clone();

    crate::tools::tool_relevant_files::check_for_inspected_files(&mut inspected_files, &result);

    let last_message = result.last().unwrap();
    crate::tools::tool_relevant_files::update_usage_from_message(&mut usage, &last_message);
    assert!(last_message.role == "assistant");

    let assistant_output1 = crate::json_utils::extract_json_object::<IndexMap<String, serde_json::Value>>(last_message.content.content_text_only().as_str()).map_err(|e| {
        tracing::warn!("\n{}\nUnable to parse JSON: {:?}", last_message.content.content_text_only(), e);
        format!("Unable to parse JSON: {:?}", e)
    })?;
    let rejection = assistant_output1.get("rejection");
    if let Some(_rejection_message) = rejection {
        let cd_instruction = format!("💿 locate() looked inside of {} files, workspace has {} files.", inspected_files.len(), total_files_in_project).replace("\n", " ");
        return Ok((results, usage, serde_json::to_string_pretty(&assistant_output1).unwrap(), cd_instruction));
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

    Ok((results, usage, serde_json::to_string_pretty(&assistant_output2).unwrap(), cd_instruction))
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
