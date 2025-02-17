use crate::ast::ast_indexer_thread::AstIndexService;
use crate::ast::ast_structs::{AstDB, AstDefinition};
use crate::call_validation::{ContextFile, CursorPosition, PostprocessSettings};
use crate::global_context::GlobalContext;
use crate::postprocessing::pp_context_files::postprocess_context_files;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use std::vec;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::info;

const DEBUG: bool = false;

const TAKE_USAGES_AROUND_CURSOR: usize = 20;

async fn _render_context_files(
    gcx: Arc<ARwLock<GlobalContext>>,
    context_format: &String,
    postprocessed_messages: &Vec<ContextFile>,
    cursor_filepath: &PathBuf,
) -> String {
    if postprocessed_messages.is_empty() {
        return "".to_string();
    }
    let (repo_name, cursor_filepath_stripped) =
        if let Some(project_dir) = crate::files_correction::get_project_dirs(gcx).await.get(0) {
            let repo_name = project_dir
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or("default_repo".to_string());
            let cursor_filepath_stripped = cursor_filepath
                .strip_prefix(project_dir)
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or(cursor_filepath.to_string_lossy().to_string());
            (repo_name, cursor_filepath_stripped)
        } else {
            (
                "default_repo".to_string(),
                cursor_filepath.to_string_lossy().to_string(),
            )
        };
    let mut context_files_prompt = String::new();
    match context_format.as_str() {
        "starcoder" => {
            context_files_prompt.push_str(&format!("<repo_name>{}\n", repo_name));
            for m in postprocessed_messages {
                context_files_prompt
                    .push_str(&format!("<file_sep>{}\n{}", m.file_name, m.file_content));
            }
            format!("{context_files_prompt}<file_sep>{cursor_filepath_stripped}\n")
        }
        "qwen2.5" => {
            context_files_prompt.push_str(&format!("<|repo_name|>{}\n", repo_name));
            for m in postprocessed_messages {
                context_files_prompt
                    .push_str(&format!("<|file_sep|>{}\n{}", m.file_name, m.file_content));
            }
            format!("{context_files_prompt}<|file_sep|>{cursor_filepath_stripped}\n")
        }
        "chat" => {
            for m in postprocessed_messages {
                context_files_prompt
                    .push_str(&format!("Filename: {}\nUseful content:\n```\n{}\n```\n\n", m.file_name, m.file_content));
            }
            context_files_prompt
        }
        _ => {
            tracing::warn!("context_format \"{}\" not recognized", context_format);
            "".to_string()
        }
    }
}

async fn _cursor_position_to_context_file(
    ast_index: Arc<AMutex<AstDB>>,
    cpath: String,
    cursor_line: i32,
    context_used: &mut Value,
) -> Vec<ContextFile> {
    if cursor_line < 0 || cursor_line > 65535 {
        tracing::error!("cursor line {} out of range", cursor_line);
        return vec![];
    }
    let cursor_line = (cursor_line + 1) as usize; // count from 1
    let usages: Vec<(usize, String)> =
        crate::ast::ast_db::doc_usages(ast_index.clone(), &cpath).await;
    // uline in usage counts from 1

    let mut distances: Vec<(i32, String, usize)> = usages
        .into_iter()
        .map(|(line, usage)| {
            let distance = (line as i32 - cursor_line as i32).abs();
            (distance, usage, line)
        })
        .collect();
    distances.sort_by_key(|&(distance, _, _)| distance);
    let nearest_usages: Vec<(usize, String)> = distances
        .into_iter()
        .take(TAKE_USAGES_AROUND_CURSOR)
        .map(|(_, usage, line)| (line, usage))
        .collect();

    if DEBUG {
        info!("nearest_usages\n{:#?}", nearest_usages);
    }

    let unique_paths: HashSet<_> = nearest_usages
        .into_iter()
        .map(|(_line, double_colon_path)| double_colon_path)
        .collect();
    let mut output = vec![];
    let mut bucket_declarations = vec![];
    for double_colon_path in unique_paths {
        if DEBUG {
            info!("adding {} to context", double_colon_path);
        }
        let defs: Vec<Arc<AstDefinition>> =
            crate::ast::ast_db::definitions(ast_index.clone(), double_colon_path.as_str()).await;
        if defs.len() != 1 {
            tracing::warn!(
                "hmm, number of definitions for {} is {} which is not one",
                double_colon_path,
                defs.len()
            );
        }
        for def in defs {
            output.push(ContextFile {
                file_name: def.cpath.clone(),
                file_content: "".to_string(),
                line1: def.full_line1(),
                line2: def.full_line2(),
                symbols: vec![def.path_drop0()],
                gradient_type: -1,
                usefulness: 100.,
            });
            let usage_dict = json!({
                "file_path": def.cpath.clone(),
                "line1": def.full_line1(),
                "line2": def.full_line2(),
                "name": def.path_drop0(),
            });
            bucket_declarations.push(usage_dict);
        }
    }
    context_used["bucket_declarations"] = json!(bucket_declarations);

    info!("FIM context\n{:#?}", output);
    output
}

pub async fn retrieve_ast_based_extra_context(
    gcx: Arc<ARwLock<GlobalContext>>,
    ast_service: Option<Arc<AMutex<AstIndexService>>>,
    t: &HasTokenizerAndEot,
    cpath: &PathBuf,
    pos: &CursorPosition,
    subblock_to_ignore_range: (i32, i32),
    pp_settings: PostprocessSettings,
    rag_tokens_n: usize,
    context_used: &mut Value,
) -> String {
    info!(" -- ast-based rag search starts --");
    let mut pp_settings = pp_settings;
    if pp_settings.max_files_n == 0 {
        pp_settings.max_files_n = 5;
    }

    let rag_t0 = Instant::now();
    let mut ast_context_file_vec: Vec<ContextFile> = if let Some(ast) = &ast_service {
        let ast_index = ast.lock().await.ast_index.clone();
        _cursor_position_to_context_file(
            ast_index.clone(),
            cpath.to_string_lossy().to_string(),
            pos.line,
            context_used,
        )
        .await
    } else {
        vec![]
    };

    let to_buckets_ms = rag_t0.elapsed().as_millis() as i32;
    if subblock_to_ignore_range.0 != i32::MAX && subblock_to_ignore_range.1 != i32::MIN {
        // disable (usefulness==-1) the FIM region around the cursor from getting into the results
        let fim_ban = ContextFile {
            file_name: cpath.to_string_lossy().to_string(),
            file_content: "".to_string(),
            line1: (subblock_to_ignore_range.0 + 1) as usize,
            line2: (subblock_to_ignore_range.1 + 1) as usize,
            symbols: vec![],
            gradient_type: -1,
            usefulness: -1.0,
        };
        ast_context_file_vec.push(fim_ban);
    }

    info!(" -- post processing starts --");
    let post_t0 = Instant::now();
    let postprocessed_messages = postprocess_context_files(
        gcx.clone(),
        &mut ast_context_file_vec,
        t.tokenizer.clone(),
        rag_tokens_n,
        false,
        &pp_settings,
    )
    .await;
    let rag_ms = rag_t0.elapsed().as_millis() as i32;
    let post_ms = post_t0.elapsed().as_millis() as i32;
    info!(
        " -- /post buckets {}ms, post {}ms -- ",
        to_buckets_ms, post_ms
    );

    // Done, only reporting is left
    // context_to_fim_debug_page(&postprocessed_messages);
    context_used["attached_files"] = Value::Array(
        postprocessed_messages
            .iter()
            .map(|x| {
                json!({
                    "file_name": x.file_name,
                    "file_content": x.file_content,
                    "line1": x.line1,
                    "line2": x.line2,
                })
            })
            .collect(),
    );
    context_used["rag_ms"] = Value::from(rag_ms);
    _render_context_files(
        gcx.clone(),
        &t.context_format,
        &postprocessed_messages,
        &cpath,
    )
    .await
}

//     // context["cursor_symbols"] = Value::Array(search_traces.cursor_symbols.iter()
//     // context["bucket_declarations"] = Value::Array(search_traces.bucket_declarations.iter()
//     // context["bucket_usage_of_same_stuff"] = Value::Array(search_traces.bucket_usage_of_same_stuff.iter()
//     // context["bucket_high_overlap"] = Value::Array(search_traces.bucket_high_overlap.iter()
//     // context["bucket_imports"] = Value::Array(search_traces.bucket_imports.iter()
