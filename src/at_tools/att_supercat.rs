use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::{Mutex as AMutex};
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ContextEnum, ContextFile};


const MAX_TOKENS_PER_FILE: usize = 6_000;
const MAX_TOKENS: usize = 24_000;


pub struct AttSuperCat;


#[async_trait]
impl Tool for AttSuperCat {
    async fn tool_execute(&mut self, ccx: Arc<AMutex<AtCommandsContext>>, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        todo!();
        // let paths = match args.get("paths") {
        //     Some(Value::String(s)) => {
        //         let paths = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
        //         paths
        //     },
        //     Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
        //     None => return Err("Missing argument `paths` for att_super_cat".to_string())
        // };
        // let symbols_str = match args.get("symbols") {
        //     Some(Value::String(s)) => {
        //         let symbols = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
        //         symbols
        //     },
        //     Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
        //     None => vec![],
        // };
        // 
        // let (global_context, tokenizer) = {
        //     let ccx_lock = ccx.lock().await;
        //     (ccx_lock.global_context.clone(), ccx_lock.tokenizer.clone().ok_or("Tokenizer not found. Try again later".to_string())?)
        // };
        // 
        // let mut corrected_paths = vec![];
        // for p in paths {
        //     let candidate = match file_repair_candidates(&p, global_context.clone(), 1, true).await.get(0) {
        //         Some(x) => x.clone(),
        //         None => continue,
        //     };
        //     corrected_paths.push(candidate);
        // }
        // // drop duplicates
        // let corrected_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
        // 
        // let mut context_files_in = vec![];
        // 
        // if !symbols_str.is_empty() {
        //     let ast_arc = global_context.read().await.ast_module.clone().unwrap();
        //     let ast_lock = ast_arc.read().await;
        //     for p in corrected_paths.iter() {
        //         
        //         let mut doc = Document::new(&PathBuf::from(p));
        //         doc.update_text_from_disk().await?;
        //         let doc_syms = ast_lock.get_file_symbols(RequestSymbolType::All, &doc).await?
        //             .symbols.iter().map(|s|(s.name.clone(), s.guid.clone())).collect::<Vec<_>>();
        //         
        //         let sym_set = doc_syms.iter()
        //             .filter(|(s_name, _)| symbols_str.contains(s_name))
        //             .map(|(_, s_uuid)|s_uuid.clone())
        //             .collect::<Vec<_>>();
        //         
        //         let text = doc.text.map(|t|t.to_string()).unwrap_or("".to_string());
        //         let cf = ContextFile {
        //             file_name: p.clone(),
        //             file_content: text.clone(),
        //             line1: 0,
        //             line2: text.lines().count(),
        //             symbol: sym_set,
        //             gradient_type: -1,
        //             usefulness: 100.0,
        //             is_body_important: false,
        //         };
        //         context_files_in.push(cf);
        //     }
        // }
        // 
        // let filenames_present = context_files_in.iter().map(|x|x.file_name.clone()).collect::<Vec<_>>();
        // for p in corrected_paths.iter().filter(|x|!filenames_present.contains(x)) {
        //     let text = read_file_from_disk(&PathBuf::from(p)).await?.to_string();
        //     let cf = ContextFile {
        //         file_name: p.clone(),
        //         file_content: text.clone(),
        //         line1: 0,
        //         line2: text.lines().count(),
        //         symbol: vec![],
        //         gradient_type: -1,
        //         usefulness: 100.0,
        //         is_body_important: false,
        //     };
        //     context_files_in.push(cf);
        // }
        // 
        // let cf_out = supercat(
        //     global_context.clone(),
        //     tokenizer,
        //     context_files_in,
        //     MAX_TOKENS_PER_FILE,
        //     MAX_TOKENS,
        // ).await?;
        // 
        // let mut res = vec![];
        // let m = ContextEnum::ChatMessage(ChatMessage {
        //     role: "supercat".to_string(),
        //     content: format_context_files_to_message_content(cf_out),
        //     tool_call_id: tool_call_id.clone(),
        //     ..Default::default()
        // });
        // res.push(m);
        // 
        // Ok(res)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

pub fn format_context_files_to_message_content(
    context_files: Vec<ContextFile>,
) -> String {
    let mut content: String = String::new();
    for x in context_files.iter() {
        content.push_str(format!("{}:\n\n{}\n\n", x.file_name.as_str(), x.file_content.as_str()).as_str());
    }
    content
}

// pub async fn supercat(
//     gcx: Arc<ARwLock<GlobalContext>>,
//     tokenizer: Arc<RwLock<Tokenizer>>,
//     files: Vec<ContextFile>,
//     max_tokens_p_file: usize,
//     max_tokens: usize,
// ) -> Result<Vec<ContextFile>, String> {
//     let mut tok_per_f = (max_tokens / files.len()).min(max_tokens_p_file);
//     let mut total_tokens_used = 0;
// 
//     let mut files = files.clone();
//     for f in files.iter_mut() {
//         if !f.file_content.is_empty() {
//             continue;
//         }
//         let file_text = read_file_from_disk(&PathBuf::from(&f.file_name)).await?.to_string();
//         f.file_content = file_text;
//     }
//     // sort (ascending) by file size
//     files.sort_by(|a, b| a.file_content.len().cmp(&b.file_content.len()));
// 
//     let mut results = vec![];
//     let files_cnt = files.len();
//     for (idx, f) in files.into_iter().enumerate() {
//         info!("supercat: {}/{}; tok_per_f: {}", idx + 1, files_cnt, tok_per_f);
//         info!("supercat: {}; approx {} tokens", f.file_name, (f.file_content.len().max(1) as f32 / 2.6) as usize);
//         let (res, tok_used) = postprocess_at_results2(
//             gcx.clone(),
//             &vec![f],
//             tokenizer.clone(),
//             tok_per_f,
//             true,
//             1
//         ).await;
//         total_tokens_used += tok_used;
//         info!("supercat: file used {} tokens; total={}", tok_used, total_tokens_used);
// 
//         if idx != files_cnt - 1 {
//             // distributing non-used rest of tokens among the others
//             tok_per_f += tok_per_f.saturating_sub(tok_used) / (files_cnt - idx - 1);
//             tok_per_f = tok_per_f.min(max_tokens_p_file);
//         }
//         results.extend(res);
//     }
// 
//     Ok(results)
// }
