use std::sync::Arc;
use std::sync::RwLock;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;

use crate::call_validation::{ChatMessage, ChatPost, ContextFile};
use crate::global_context::GlobalContext;
use crate::ast::structs::FileASTMarkup;
use crate::files_in_workspace::{Document, read_file_from_disk};

const RESERVE_FOR_QUESTION_AND_FOLLOWUP: usize = 1024;  // tokens


const DEBUG: bool = true;


#[derive(Debug)]
pub struct File {
    pub markup: FileASTMarkup,
    pub file_name: String,   // delete when we remove Url
}

#[derive(Debug)]
pub struct FileLine {
    pub fref: Arc<File>,
    pub line_n: usize,
    pub line_content: String,
    pub useful: f32,
    pub color: String,
    pub take: bool,
}

pub fn context_to_fim_debug_page(t0: &Instant, postprocessed_messages: &[ContextFile], was_looking_for: &HashMap<String, Vec<String>>) -> Value {
    let attached_files: Vec<_> = postprocessed_messages.iter().map(|x| {
        json!({
            "file_name": x.file_name,
            "file_content": x.file_content,
            "line1": x.line1,
            "line2": x.line2,
        })
    }).collect();

    let was_looking_for_vec: Vec<_> = was_looking_for.iter().flat_map(|(k, v)| {
        v.iter().map(move |i| {
            json!({
                "from": k,
                "symbol": i,
            })
        })
    }).collect();
    let elapsed = t0.elapsed().as_secs_f32();
    json!({
        "elapsed": elapsed,
        "was_looking_for": was_looking_for_vec,
        "attached_files": attached_files,
    })
}

pub async fn postprocess_rag_stage1(
    global_context: Arc<ARwLock<GlobalContext>>,
    origmsgs: Vec<ContextFile>,
    files_set: HashSet<String>,
    close_small_gaps: bool,
) -> (HashMap<String, Vec<Arc<FileLine>>>, Vec<Arc<FileLine>>) {
    // 2. Load files, with ast or not
    let mut files: HashMap<String, Arc<File>> = HashMap::new();
    let ast_module = global_context.read().await.ast_module.clone();
    for file_name in files_set {
        let doc = Document::new(&PathBuf::from(file_name.clone()), None);
        let mut f: Option<Arc<File>> = None;
        if let Some(astmod) = &ast_module {
            match astmod.read().await.file_markup(&doc).await {
                Ok(markup) => {
                    f = Some(Arc::new(File { markup, file_name: file_name.clone() }));
                },
                Err(err) => {
                    warn!("postprocess_rag_stage1 query file {:?} markup problem: {}", file_name, err);
                }
            }
        }
        if f.is_none() {
            f = Some(Arc::new(File {
                markup: FileASTMarkup {
                    file_path: doc.path.clone(),
                    file_content: read_file_from_disk(&doc.path).await.unwrap_or_default().to_string(),
                    symbols_sorted_by_path_len: Vec::new(),
                },
                file_name: file_name.clone(),
            }));
        }
        if f.is_some() {
            files.insert(file_name.clone(), f.unwrap());
        }
    }

    // 3. Generate line refs
    let mut lines_by_useful: Vec<Arc<FileLine>> = vec![];
    let mut lines_in_files: HashMap<String, Vec<Arc<FileLine>>> = HashMap::new();
    for fref in files.values() {
        for (line_n, line) in fref.markup.file_content.lines().enumerate() {
            let a = Arc::new(FileLine {
                fref: fref.clone(),
                line_n: line_n,
                line_content: line.to_string(),
                useful: 0.0,
                color: "".to_string(),
                take: false,
            });
            lines_by_useful.push(a.clone());
            let lines_in_files_mut = lines_in_files.entry(fref.file_name.clone()).or_insert(vec![]);
            lines_in_files_mut.push(a.clone());
        }
    }
    let colorize_if_more_useful = |linevec: &mut Vec<Arc<FileLine>>, line1: usize, line2: usize, color: &String, useful: f32|
    {
        if DEBUG {
            info!("    colorize_if_more_useful {}..{} <= color {:?} useful {}", line1, line2, color, useful);
        }
        for i in line1 .. line2 {
            if i >= linevec.len() {
                warn!("    {} has faulty range {}..{}", color, line1, line2);
                continue;
            }
            let lineref_mut: *mut FileLine = Arc::as_ptr(&linevec[i]) as *mut FileLine;
            let u = useful - (i as f32) * 0.001;
            unsafe {
                if (*lineref_mut).useful < u || (*lineref_mut).color.is_empty() {
                    (*lineref_mut).useful = u;
                    (*lineref_mut).color = color.clone();
                }
            }
        }
    };
    for linevec in lines_in_files.values_mut() {
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        info!("fref {:?} has {} bytes, {} symbols", fref.file_name, fref.markup.file_content.len(), fref.markup.symbols_sorted_by_path_len.len());
        for s in fref.markup.symbols_sorted_by_path_len.iter() {
            // info!("    {} {:?} {}-{}", s.symbol_path, s.symbol_type, s.full_range.start_point.row, s.full_range.end_point.row);
            let useful = 10.0;  // depends on symbol type?
            colorize_if_more_useful(linevec, s.full_range.start_point.row, s.full_range.end_point.row+1, &format!("{}", s.symbol_path), useful);
        }
        colorize_if_more_useful(linevec, 0, linevec.len(), &"".to_string(), 5.0);
    }

    // 4. Fill in usefulness from search results
    for omsg in origmsgs.iter() {
        let linevec: &mut Vec<Arc<FileLine>> = match lines_in_files.get_mut(&omsg.file_name) {
            Some(x) => x,
            None => {
                warn!("postprocess_rag_stage1: file not found {}", omsg.file_name);
                continue;
            }
        };
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        let mut maybe_symbol: Option<&SymbolInformation> = None;
        if !omsg.symbol.is_empty() {
            for x in fref.markup.symbols_sorted_by_path_len.iter() {
                if x.guid == omsg.symbol {
                    maybe_symbol = Some(x);
                    break;
                }
            }
            if maybe_symbol.is_none() {
                warn!("postprocess_rag_stage1: cannot find symbol {} in file {}", omsg.symbol, omsg.file_name);
            }
        }
        if let Some(s) = maybe_symbol {
            info!("    search result {} {:?} {:.2}", s.symbol_path, s.symbol_type, omsg.usefulness);
            colorize_if_more_useful(linevec, s.full_range.start_point.row, s.full_range.end_point.row+1, &format!("{}", s.symbol_path), omsg.usefulness);
        } else {
            // no symbol set in search result, go head with just line numbers, omsg.line1, omsg.line2 numbers starts from 1, not from 0
            if omsg.line1 == 0 || omsg.line2 == 0 || omsg.line1 > omsg.line2 || omsg.line1 > linevec.len() || omsg.line2 > linevec.len() {
                warn!("postprocess_rag_stage1: cannot use range {}:{}..{}", omsg.file_name, omsg.line1, omsg.line2);
                continue;
            }
            colorize_if_more_useful(linevec, omsg.line1-1, omsg.line2, &"nosymb".to_string(), omsg.usefulness);
        }
    }

    // 5. Downgrade sub-symbols and uninteresting regions
    let downgrade_lines_if_subsymbol = |linevec: &mut Vec<Arc<FileLine>>, line1_base0: usize, line2_base0: usize, subsymbol: &String, downgrade_coef: f32|
    {
        let mut changes_cnt = 0;
        for i in line1_base0 .. line2_base0 {
            assert!(i < linevec.len());
            let lineref_mut: *mut FileLine = Arc::as_ptr(&linevec[i]) as *mut FileLine;
            unsafe {
                if subsymbol.starts_with(&(*lineref_mut).color) { // && subsymbol != &(*lineref_mut).color {
                    if i == line2_base0-1 || i == line1_base0 {
                        if (*lineref_mut).line_content.trim().len() == 1 {
                            // HACK: closing brackets at the end, leave it alone without downgrade
                            continue;
                        }
                    }
                    (*lineref_mut).useful *= downgrade_coef;
                    (*lineref_mut).color = subsymbol.clone();
                    changes_cnt += 1;
                }
            }
        }
        if DEBUG {
            info!("        {}..{} ({} affected) <= subsymbol {:?} downgrade {}", changes_cnt, line1_base0, line2_base0, subsymbol, downgrade_coef);
        }
    };
    for linevec in lines_in_files.values_mut() {
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        info!("degrading body of symbols in {}", fref.file_name);
        for s in fref.markup.symbols_sorted_by_path_len.iter() {
            if DEBUG {
                info!("    {} {:?} {}-{}", s.symbol_path, s.symbol_type, s.full_range.start_point.row, s.full_range.end_point.row);
            }
            if s.definition_range.end_byte != 0 {
                // decl  void f() {
                // def      int x = 5;
                // def   }
                let (def0, def1) = (
                    s.definition_range.start_point.row.max(s.declaration_range.end_point.row + 1),   // definition must stay clear of declaration
                    s.definition_range.end_point.row + 1
                );
                if def1 > def0 {
                    downgrade_lines_if_subsymbol(linevec, def0, def1, &format!("{}::body", s.symbol_path), 0.8);
                    // NOTE: this will not downgrade function body of a function that is a search result, because it's not a subsymbol it's the symbol itself (equal path)
                }
            }
        }
    }

    // 6. A-la mathematical morphology, removes one-line holes
    if close_small_gaps {
        for linevec in lines_in_files.values_mut() {
            let mut useful_copy = linevec.iter().map(|x| x.useful).collect::<Vec<f32>>();
            for i in 1 .. linevec.len() - 1 {
                let l = linevec[i-1].useful;
                let m = linevec[i  ].useful;
                let r = linevec[i+1].useful;
                let both_l_and_r_support = l.min(r);
                useful_copy[i] = m.max(both_l_and_r_support);
            }
            for i in 0 .. linevec.len() {
                let lineref_mut: *mut FileLine = Arc::as_ptr(linevec.get(i).unwrap()) as *mut FileLine;
                unsafe {
                    (*lineref_mut).useful = useful_copy[i];
                }
            }
        }
    }

    (lines_in_files, lines_by_useful)
}

pub async fn postprocess_at_results2(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: Vec<ChatMessage>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
) -> Vec<ContextFile> {
    // 1. Decode all
    let mut origmsgs: Vec<ContextFile> = vec![];
    let mut files_set: HashSet<String> = HashSet::new();
    for msg in messages {
        match serde_json::from_str::<Vec<ContextFile>>(&msg.content) {
            Ok(decoded) => {
                origmsgs.extend(decoded.clone());
                for cf in decoded {
                    files_set.insert(cf.file_name.clone());
                }
            },
            Err(err) => {
                warn!("postprocess_at_results2 decoding results problem: {}", err);
                continue;
            }
        }
    }

    let close_small_gaps = true;
    let (mut lines_in_files, mut lines_by_useful) = postprocess_rag_stage1(global_context, origmsgs, files_set, close_small_gaps).await;

    // 7. Sort
    lines_by_useful.sort_by(|a, b| {
        b.useful.partial_cmp(&a.useful).unwrap_or(Ordering::Equal)
    });

    // 8. Convert line_content to tokens up to the limit
    let mut tokens_count: usize = 0;
    let mut lines_take_cnt: usize = 0;
    for lineref in lines_by_useful.iter_mut() {
        let ntokens = count_tokens(&tokenizer.read().unwrap(), &lineref.line_content);
        if tokens_count + ntokens > tokens_limit {
            break;
        }
        tokens_count += ntokens;
        unsafe {
            let lineref_mut: *mut FileLine = Arc::as_ptr(lineref) as *mut FileLine;
            (*lineref_mut).take = true;
            lines_take_cnt += 1;
        }
    }
    info!("{} lines in {} files  =>  tokens {} < {} tokens limit  =>  {} lines", lines_by_useful.len(), lines_in_files.len(), tokens_count, tokens_limit, lines_take_cnt);
    if DEBUG {
        for linevec in lines_in_files.values() {
            for lineref in linevec.iter() {
                info!("{} {}:{:04} {:>7.3} {}",
                if lineref.take { "take" } else { "dont" },
                crate::nicer_logs::last_n_chars(&lineref.fref.file_name, 30),
                lineref.line_n,
                lineref.useful,
                crate::nicer_logs::first_n_chars(&lineref.line_content, 20)
            );
            }
        }
    }

    // 9. Generate output
    let mut merged: Vec<ContextFile> = vec![];
    for linevec in lines_in_files.values_mut() {
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        let fname = fref.file_name.clone();
        let mut out = String::new();
        let mut first_line: usize = 0;
        let mut last_line: usize = 0;
        let mut prev_line: usize = 0;
        for (i, lineref) in linevec.iter_mut().enumerate() {
            last_line = i;
            if !lineref.take {
                continue;
            }
            if first_line == 0 { first_line = i; }
            if i > prev_line + 1 {
                out.push_str(format!("...{} lines\n", i - prev_line - 1).as_str());
            }
            out.push_str(&lineref.line_content);
            out.push_str("\n");
            prev_line = i;
        }
        if last_line > prev_line + 1 {
            out.push_str("...\n");
        }
        if DEBUG {
            info!("file {:?}\n{}", fname, out);
        }
        merged.push(ContextFile {
            file_name: fname,
            file_content: out,
            line1: first_line,
            line2: last_line,
            symbol: "".to_string(),
            usefulness: 0.0,
        });
    }
    merged
}

pub fn count_tokens(
    tokenizer: &Tokenizer,
    text: &str,
) -> usize {
    match tokenizer.encode(text, false) {
        Ok(tokens) => tokens.len(),
        Err(_) => 0,
    }
}

pub async fn run_at_commands(
    global_context: Arc<ARwLock<GlobalContext>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    maxgen: usize,
    n_ctx: usize,
    post: &mut ChatPost,
    top_n: usize,
    stream_back_to_user: &mut HasVecdbResults,
) -> usize {
    // TODO: don't operate on `post`, return a copy of the messages
    let context = AtCommandsContext::new(global_context.clone()).await;

    let mut user_msg_starts = post.messages.len();
    let mut user_messages_with_at: usize = 0;
    while user_msg_starts > 0 {
        let message = post.messages.get(user_msg_starts - 1).unwrap().clone();
        let role = message.role.clone();
        let content = message.content.clone();
        info!("user_msg_starts {} {}", user_msg_starts - 1, role);
        if role == "user" {
            user_msg_starts -= 1;
            if content.contains("@") {
                user_messages_with_at += 1;
            }
        } else {
            break;
        }
    }
    user_messages_with_at = user_messages_with_at.max(1);
    let reserve_for_context = n_ctx - maxgen - RESERVE_FOR_QUESTION_AND_FOLLOWUP;
    info!("reserve_for_context {} tokens", reserve_for_context);

    // Token limit works like this:
    // - if there's only 1 user message at the bottom, it receives ntokens_minus_maxgen tokens for context
    // - if there are N user messages, they receive ntokens_minus_maxgen/N tokens each (and there's no taking from one to give to the other)
    // This is useful to give prefix and suffix of the same file precisely the position necessary for FIM-like operation of a chat model

    let mut rebuilt_messages: Vec<ChatMessage> = post.messages.iter().take(user_msg_starts).map(|m| m.clone()).collect();
    for msg_idx in user_msg_starts..post.messages.len() {
        let mut user_posted = post.messages[msg_idx].content.clone();
        let user_posted_ntokens = count_tokens(&tokenizer.read().unwrap(), &user_posted);
        let mut context_limit = reserve_for_context / user_messages_with_at;
        if context_limit <= user_posted_ntokens {
            context_limit = 0;
        } else {
            context_limit -= user_posted_ntokens;
        }
        info!("msg {} user_posted {:?} that's {} tokens", msg_idx, user_posted, user_posted_ntokens);
        info!("that leaves {} tokens for context of this message", context_limit);

        let valid_commands = crate::at_commands::utils::find_valid_at_commands_in_query(&mut user_posted, &context).await;
        let mut messages_for_postprocessing = vec![];
        for cmd in valid_commands {
            match cmd.command.lock().await.execute(&user_posted, &cmd.args, top_n, &context).await {
                Ok(msg) => {
                    messages_for_postprocessing.push(msg);
                },
                Err(e) => {
                    tracing::warn!("can't execute command that indicated it can execute: {}", e);
                }
            }
        }
        let t0 = std::time::Instant::now();
        let processed = postprocess_at_results2(
            global_context.clone(),
            messages_for_postprocessing,
            tokenizer.clone(),
            context_limit
        ).await;
        info!("postprocess_at_results2 {:.3}s", t0.elapsed().as_secs_f32());
        if processed.len() > 0 {
            let message = ChatMessage {
                role: "context_file".to_string(),
                content: serde_json::to_string(&processed).unwrap(),
            };
            rebuilt_messages.push(message.clone());
            stream_back_to_user.push_in_json(json!(message));
        }
        if user_posted.trim().len() > 0 {
            let msg = ChatMessage {
                role: "user".to_string(),
                content: user_posted,  // stream back to the user, without commands
            };
            rebuilt_messages.push(msg.clone());
            stream_back_to_user.push_in_json(json!(msg));
        }
    }
    post.messages = rebuilt_messages;
    user_msg_starts
}


pub struct HasVecdbResults {
    pub was_sent: bool,
    pub in_json: Vec<Value>,
}

impl HasVecdbResults {
    pub fn new() -> Self {
        HasVecdbResults {
            was_sent: false,
            in_json: vec![],
        }
    }
}

impl HasVecdbResults {
    pub fn push_in_json(&mut self, value: Value) {
        self.in_json.push(value);
    }

    pub fn response_streaming(&mut self) -> Result<Vec<Value>, String> {
        if self.was_sent == true || self.in_json.is_empty() {
            return Ok(vec![]);
        }
        self.was_sent = true;
        Ok(self.in_json.clone())
    }
}
