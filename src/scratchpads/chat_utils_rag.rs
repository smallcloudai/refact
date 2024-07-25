use std::sync::Arc;
use std::sync::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{info, warn};
use serde_json::{json, Value};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use std::hash::{Hash, Hasher};
use uuid::Uuid;
use crate::ast::structs::SymbolsSearchResultStruct;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::structs::SymbolType;

use crate::call_validation::{ChatMessage, ContextFile};
use crate::global_context::GlobalContext;
use crate::ast::structs::FileASTMarkup;
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};


const RESERVE_FOR_QUESTION_AND_FOLLOWUP: usize = 1024;  // tokens
const DEBUG: i32 = 0;  // 0 nothing, 1 summary "N lines in K files => X tokens", 2 everything


#[derive(Debug)]
pub struct File {
    pub markup: FileASTMarkup,
    pub cpath: PathBuf,
    pub cpath_symmetry_breaker: f32,
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

pub fn max_tokens_for_rag_chat(n_ctx: usize, maxgen: usize) -> usize {
    ((n_ctx/2) as i32 - maxgen as i32 - RESERVE_FOR_QUESTION_AND_FOLLOWUP as i32).max(0) as usize
}

pub fn context_to_fim_debug_page(
    postprocessed_messages: &[ContextFile],
    search_traces: &crate::ast::structs::AstCursorSearchResult,
) -> Value {
    let mut context = json!({});
    fn shorter_symbol(x: &SymbolsSearchResultStruct) -> Value {
        let mut t: Value = json!({});
        t["name"] = Value::String(x.symbol_declaration.name.clone());
        t["file_path"] = Value::String(x.symbol_declaration.file_path.display().to_string());
        t["line1"] = json!(x.symbol_declaration.full_range.start_point.row + 1);
        t["line2"] = json!(x.symbol_declaration.full_range.end_point.row + 1);
        t
    }
    context["cursor_symbols"] = Value::Array(search_traces.cursor_symbols.iter()
        .map(|x| shorter_symbol(x)).collect());
    context["bucket_declarations"] = Value::Array(search_traces.bucket_declarations.iter()
        .map(|x| shorter_symbol(x)).collect());
    context["bucket_usage_of_same_stuff"] = Value::Array(search_traces.bucket_usage_of_same_stuff.iter()
        .map(|x| shorter_symbol(x)).collect());
    context["bucket_high_overlap"] = Value::Array(search_traces.bucket_high_overlap.iter()
        .map(|x| shorter_symbol(x)).collect());
    context["bucket_imports"] = Value::Array(search_traces.bucket_imports.iter()
        .map(|x| shorter_symbol(x)).collect());

    let attached_files: Vec<_> = postprocessed_messages.iter().map(|x| {
        json!({
            "file_name": x.file_name,
            "file_content": x.file_content,
            "line1": x.line1,
            "line2": x.line2,
        })
    }).collect();
    context["attached_files"] = Value::Array(attached_files);
    context
}

fn color_with_gradient_type(omsg: &ContextFile, linevec: &mut Vec<Arc<FileLine>>) {
    fn find_line_parameters(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32) {
        if y2 - y1 == 0. || x2 - x1 == 0. {
            return (0., 0.);
        }
        let m = (y2 - y1) / (x2 - x1);
        let c = y1 - m * x1;
        (m, c)
    }

    if omsg.gradient_type < 0 || omsg.gradient_type > 4 {
        return;
    }

    let t_fade_away_lines = 50;
    let (m11, c11) = find_line_parameters(omsg.line1 as f32, omsg.usefulness, omsg.line1 as f32 - t_fade_away_lines as f32, 0. );
    let (m12, c12) = find_line_parameters(omsg.line1 as f32, omsg.usefulness, omsg.line1 as f32 + t_fade_away_lines as f32, 0. );
    let (m21, c21) = find_line_parameters(omsg.line2 as f32, omsg.usefulness, omsg.line2 as f32 - t_fade_away_lines as f32, 0. );
    let (m22, c22) = find_line_parameters(omsg.line2 as f32, omsg.usefulness, omsg.line2 as f32 + t_fade_away_lines as f32, 0. );

    for (line_n, line) in linevec.iter().enumerate() {
        let line_n = line_n + 1;
        let usefulness = match omsg.gradient_type {
            0 => omsg.usefulness - (line_n as f32) * 0.001,
            1 => if line_n < omsg.line1 {(line_n as f32 * m11 + c11).max(0.)} else {(line_n as f32 * m12 + c12).max(0.)},
            2 => if line_n <= omsg.line2 {(line_n as f32 * m21 + c21).max(0.) } else {-1.},
            3 => if line_n < omsg.line1 {-1.} else {(line_n as f32 * m12 + c12).max(0.)},
            4 => {
                if line_n < omsg.line1 {
                    line_n as f32 * m11 + c11
                } else if line_n >= omsg.line1 && line_n <= omsg.line2 {
                    100.
                } else {
                    line_n as f32 * m22 + c22
                }
            }.max(0.),
            _ => 0.0,
        };
        set_useful_for_line(line, usefulness, &format!("gradient_type: {:?}", omsg.gradient_type));
    }
}

fn set_useful_for_line(line: &Arc<FileLine>, useful: f32, color: &String) {
    let lineref_mut: *mut FileLine = Arc::as_ptr(line) as *mut FileLine;
    unsafe {
        if (line.useful < useful) || useful < 0. {
            (*lineref_mut).useful = useful;
            (*lineref_mut).color = color.clone();
        }
    }
}

fn calculate_hash(path: &PathBuf) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

pub async fn postprocess_rag_load_ast_markup(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ContextFile>,
) -> HashMap<String, Arc<File>> {
    // 2. Load AST markup
    let mut files_markup: HashMap<String, Arc<File>> = HashMap::new();
    let ast_module = global_context.read().await.ast_module.clone();
    for message in messages {
        let file_name = message.file_name.clone();
        if files_markup.contains_key(&file_name) {
            continue;
        }
        let path = crate::files_correction::canonical_path(&file_name.clone());
        let cpath_symmetry_breaker: f32 = (calculate_hash(&path) as f32) / (u64::MAX as f32) / 100.0;
        let mut doc = Document::new(&path);
        let text = get_file_text_from_memory_or_disk(global_context.clone(), &doc.path).await.unwrap_or_default();
        doc.update_text(&text);
        let mut f: Option<Arc<File>> = None;
        if let Some(astmod) = &ast_module {
            match astmod.read().await.file_markup(&doc).await {
                Ok(markup) => {
                    f = Some(Arc::new(File { markup, cpath: path, cpath_symmetry_breaker }));
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
                    file_content: text,
                    symbols_sorted_by_path_len: Vec::new(),
                },
                cpath: doc.path.clone(),
                cpath_symmetry_breaker,
            }));
        }
        if f.is_some() {
            files_markup.insert(file_name.clone(), f.unwrap());
        }
    }
    files_markup
}

pub struct PostprocessSettings {
    pub useful_background: f32,          // first, fill usefulness of all lines with this
    pub useful_symbol_default: f32,      // when a symbol present, set usefulness higher
    // search results fill usefulness as it passed from outside
    pub degrade_parent_coef: f32,        // goto parent from search results and mark it useful, with this coef
    pub degrade_body_coef: f32,          // multiply body usefulness by this, so it's less useful than the declaration
    pub comments_propogate_up_coef: f32, // mark comments above a symbol as useful, with this coef
    pub close_small_gaps: bool,
    pub take_floor: f32,                 // take/dont value
    pub max_files_n: usize,              // don't produce more than n files in output
}

impl PostprocessSettings {
    pub fn new() -> Self {
        PostprocessSettings {
            degrade_body_coef: 0.8,
            degrade_parent_coef: 0.6,
            useful_background: 5.0,
            useful_symbol_default: 10.0,
            close_small_gaps: true,
            comments_propogate_up_coef: 0.99,
            take_floor: 0.0,
            max_files_n: 10,
        }
    }
}

fn colorize_if_more_useful(linevec: &mut Vec<Arc<FileLine>>, line1: usize, line2: usize, color: &String, useful: f32) {
    if DEBUG >= 2 {
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
}

pub async fn context_msgs_from_paths(
    global_context: Arc<ARwLock<GlobalContext>>,
    files_set: HashSet<String>
) -> Vec<ContextFile> {
    let mut messages = vec![];
    for file_name in files_set {
        let path = crate::files_correction::canonical_path(&file_name.clone());
        let text = get_file_text_from_memory_or_disk(global_context.clone(), &path).await.unwrap_or_default();
        messages.push(ContextFile {
            file_name: file_name.clone(),
            file_content: text.clone(),
            line1: 0,
            line2: text.lines().count(),
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness: 0.,
            is_body_important: false
        });
    }
    messages
}

fn colorize_parentof(linevec: &mut Vec<Arc<FileLine>>, long_child_path: &String, bg: f32, maxuseful: f32) {
    if DEBUG >= 2 {
        info!("    colorize_parentof long_child_path={} bg={} maxuseful={}", long_child_path, bg, maxuseful);
    }
    for i in 0 .. linevec.len() {
        let lineref_mut: *mut FileLine = Arc::as_ptr(&linevec[i]) as *mut FileLine;
        unsafe {
            let color = &(*lineref_mut).color;
            if long_child_path.starts_with(color) && color.len() > 0 {
                let plen = (*lineref_mut).color.len();
                let long = long_child_path.len();
                let mut u = bg + (maxuseful - bg)*(plen as f32)/(long as f32);
                u -= (i as f32) * 0.001;
                if (*lineref_mut).useful < u {
                    if DEBUG >= 2 {
                        info!("    colorize_parentof line{:04} {} <= {:>7.3}", i, color, u);
                    }
                    (*lineref_mut).useful = u;
                }
            }
        }
    }
}

fn colorize_minus_one(linevec: &mut Vec<Arc<FileLine>>, line1: usize, line2: usize) {
    for i in line1 .. line2 {
        if i >= linevec.len() {
            continue;
        }
        let l = &linevec[i];
        let l_mut: *mut FileLine = Arc::as_ptr(l) as *mut FileLine;
        unsafe {
            (*l_mut).useful = -1.;
            (*l_mut).color = "disabled".to_string();
        }
    }
}

fn colorize_comments_up(linevec: &mut Vec<Arc<FileLine>>, settings: &PostprocessSettings) {
    for i in (0 .. linevec.len() - 1).rev() {
        let thisline: *mut FileLine = Arc::as_ptr(&linevec[i]) as *mut FileLine;
        let nextline: *mut FileLine = Arc::as_ptr(&linevec[i + 1]) as *mut FileLine;
        unsafe {
            let u = (*nextline).useful * settings.comments_propogate_up_coef;
            if (*thisline).color == "comment" && (*thisline).useful < u {
                (*thisline).useful = u;
                if DEBUG >= 2 {
                    info!("    comments_up_from_symbol line{:04} <= {:>7.3}", i, u);
                }
            }
        }
    }
}

fn downgrade_lines_if_subsymbol(linevec: &mut Vec<Arc<FileLine>>, line1_base0: usize, line2_base0: usize, subsymbol: &String, downgrade_coef: f32) {
    let mut changes_cnt = 0;
    for i in line1_base0 .. line2_base0 {
        if i >= linevec.len() {
            continue;
        }
        let lineref_mut: *mut FileLine = Arc::as_ptr(&linevec[i]) as *mut FileLine;
        unsafe {
            if subsymbol.starts_with(&(*lineref_mut).color) {
                if i == line2_base0-1 || i == line1_base0 {
                    if (*lineref_mut).line_content.trim().len() == 1 {  // only closing bracket -- don't degrade, for C++ void f()  { ... }  last line with "}" only
                        continue;
                    }
                }
                (*lineref_mut).useful *= downgrade_coef;
                (*lineref_mut).color = subsymbol.clone();
                changes_cnt += 1;
            }
        }
    }
    if DEBUG >= 2 {
        info!("        {}..{} ({} affected) <= subsymbol {:?} downgrade {}", changes_cnt, line1_base0, line2_base0, subsymbol, downgrade_coef);
    }
}

pub async fn postprocess_rag_stage_3_6(
    global_context: Arc<ARwLock<GlobalContext>>,
    origmsgs: &Vec<ContextFile>,
    files: &HashMap<String, Arc<File>>,
    settings: &PostprocessSettings,
) -> (HashMap<PathBuf,Vec<Arc<FileLine>>>, Vec<Arc<FileLine>>) {
    // 3. Generate line refs, fill background scopes found in a file (not search results yet)
    let mut lines_by_useful: Vec<Arc<FileLine>> = vec![];
    let mut lines_in_files: HashMap<PathBuf, Vec<Arc<FileLine>>> = HashMap::new();
    for fref in files.values() {
        for (line_n, line) in fref.markup.file_content.lines().enumerate() {
            let a = Arc::new(FileLine {
                fref: fref.clone(),
                line_n,
                line_content: line.to_string(),
                useful: 0.0,
                color: "".to_string(),
                take: false,
            });
            lines_by_useful.push(a.clone());
            let lines_in_files_mut = lines_in_files.entry(fref.cpath.clone()).or_insert(vec![]);
            lines_in_files_mut.push(a.clone());
        }
    }
    for linevec in lines_in_files.values_mut() {
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        if DEBUG >= 2 {
            info!("fref {:?} has {} bytes, {} symbols", fref.cpath, fref.markup.file_content.len(), fref.markup.symbols_sorted_by_path_len.len());
        }
        for s in fref.markup.symbols_sorted_by_path_len.iter() {
            if DEBUG >= 2 {
                info!("    {} {:?} {}-{}", s.symbol_path, s.symbol_type, s.full_range.start_point.row, s.full_range.end_point.row);
            }
            if s.symbol_type == SymbolType::CommentDefinition {
                let useful = settings.useful_symbol_default;
                colorize_if_more_useful(linevec, s.full_range.start_point.row, s.full_range.end_point.row+1, &"comment".to_string(), useful);
            } else {
                let useful = settings.useful_symbol_default;  // depends on symbol type?
                colorize_if_more_useful(linevec, s.full_range.start_point.row, s.full_range.end_point.row+1, &format!("{}", s.symbol_path), useful);
            }
        }
        colorize_if_more_useful(linevec, 0, linevec.len(), &"empty".to_string(), settings.useful_background);
    }

    // 4. Fill in usefulness from search results
    for omsg in origmsgs.iter() {
        // Do what we can to match omsg.file_name to something real
        let nearest = crate::files_correction::correct_to_nearest_filename(global_context.clone(), &omsg.file_name, false, 1).await;
        let cpath = if nearest.is_empty() {
            crate::files_correction::canonical_path(&omsg.file_name)
        } else {
            crate::files_correction::canonical_path(&nearest[0])
        };
        let linevec: &mut Vec<Arc<FileLine>> = match lines_in_files.get_mut(&cpath) {
            Some(x) => x,
            None => {
                warn!("file not found by name {:?} or cpath {:?}", omsg.file_name, cpath);
                continue;
            }
        };
        if linevec.len() == 0 {
            continue;
        }

        color_with_gradient_type(omsg, linevec);
        let fref = linevec[0].fref.clone();
        if omsg.usefulness < 0.0 {  // used in FIM to disable lines already in suffix or prefix
            colorize_minus_one(linevec, omsg.line1-1, omsg.line2);
            continue;
        }
        let mut maybe_symbol: Option<&SymbolInformation> = None;
        if !omsg.symbol.is_nil() {
            for x in fref.markup.symbols_sorted_by_path_len.iter() {
                if x.guid == omsg.symbol {
                    maybe_symbol = Some(x);
                    break;
                }
            }
            if maybe_symbol.is_none() {
                warn!("- cannot find symbol {} in file {}:{}-{}", omsg.symbol, omsg.file_name, omsg.line1, omsg.line2);
            }
        }
        if !omsg.is_body_important && maybe_symbol.is_some() {
            if let Some(s) = maybe_symbol {
                if DEBUG >= 1 {
                    info!("+ search result {} {:?} {:.2}", s.symbol_path, s.symbol_type, omsg.usefulness);
                }
                colorize_if_more_useful(linevec, s.full_range.start_point.row, s.full_range.end_point.row+1, &format!("{}", s.symbol_path), omsg.usefulness);
                let mut parent_path = s.symbol_path.split("::").collect::<Vec<&str>>();
                if parent_path.len() > 1 {
                    // MyClass::f  ->  MyClass
                    // make parent stand out from background as well, to make it more clear to the model where the symbol is
                    parent_path.pop();
                    let parent_path_str = parent_path.join("::");
                    colorize_parentof(linevec, &parent_path_str, settings.useful_symbol_default, omsg.usefulness*settings.degrade_parent_coef);
                }
            }
        } else {
            // no symbol set in search result, go head with just line numbers, omsg.line1, omsg.line2 numbers starts from 1, not from 0
            info!("+ search result from vecdb or @file {:.2}", omsg.usefulness);
            if omsg.line1 == 0 || omsg.line2 == 0 || omsg.line1 > omsg.line2 || omsg.line1 > linevec.len() || omsg.line2 > linevec.len() {
                warn!("range in search results is outside of file lines that actually exist {}:{}-{}", omsg.file_name, omsg.line1, omsg.line2);
            }
            colorize_if_more_useful(linevec, omsg.line1.max(1)-1, omsg.line2, &"nosymb".to_string(), omsg.usefulness);
        }
        // example: see comment in class Toad
        colorize_comments_up(linevec, settings);
    }

    // 5. Downgrade sub-symbols and uninteresting regions
    for linevec in lines_in_files.values_mut() {
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        if DEBUG >= 2 {
            info!("degrading body of symbols in {:?}", fref.cpath);
        }
        for s in fref.markup.symbols_sorted_by_path_len.iter() {
            if s.definition_range.end_byte != 0 {
                if DEBUG >= 2 {
                    info!("    {} {:?} {}-{}", s.symbol_path, s.symbol_type, s.full_range.start_point.row, s.full_range.end_point.row);
                }
                // decl  void f() {
                // def      int x = 5;
                // def   }
                let (def0, def1) = (
                    s.definition_range.start_point.row.max(s.declaration_range.end_point.row + 1),   // definition must stay clear of declaration
                    s.definition_range.end_point.row + 1
                );
                if def1 > def0 {
                    downgrade_lines_if_subsymbol(linevec, def0, def1, &format!("{}::body", s.symbol_path), settings.degrade_body_coef);
                    // NOTE: this will not downgrade function body of a function that is a search result, because it's not a subsymbol it's the symbol itself (equal path)
                }
            }
        }
    }

    // 6. A-la mathematical morphology, removes one-line holes
    if settings.close_small_gaps {
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
    messages: &Vec<ContextFile>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
    single_file_mode: bool,
    max_files_n: usize,
) -> Vec<ContextFile> {
    let files_markup = postprocess_rag_load_ast_markup(global_context.clone(), &messages).await;

    let mut settings = PostprocessSettings::new();
    settings.max_files_n = max_files_n;
    let (mut lines_in_files, mut lines_by_useful) = postprocess_rag_stage_3_6(
        global_context.clone(),
        &messages,
        &files_markup,
        &settings,
    ).await;
    postprocess_rag_stage_7_9(&mut lines_in_files, &mut lines_by_useful, tokenizer, tokens_limit, single_file_mode, &settings).await
}

pub async fn postprocess_plain_text_messages(
    messages: Vec<&ChatMessage>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
) -> (Vec<ChatMessage>, usize) {
    if messages.is_empty() {
        return (vec![], tokens_limit);
    }
    let mut messages_sorted = messages.clone();
    // ascending sorting
    messages_sorted.sort_by(|a, b| a.content.len().cmp(&b.content.len()));

    let mut tok_used_global = 0;
    let mut tok_per_m = tokens_limit / messages_sorted.len();
    let mut results = vec![];

    let tokenizer_guard = tokenizer.read().unwrap();
    for (idx, msg) in messages_sorted.iter().cloned().enumerate() {
        let mut out = vec![];
        let mut tok_used: usize = 0;
        for line in msg.content.lines() {
            let line_tokens = count_tokens(&tokenizer_guard, &line);
            if tok_used + line_tokens > tok_per_m {
                break;
            }
            tok_used += line_tokens;
            out.push(line);
        }
        if idx != messages_sorted.len() - 1 {
            // distributing non-used rest of tokens among the others
            tok_per_m += (tok_per_m - tok_used) / (messages_sorted.len() - idx - 1);
        }
        tok_used_global += tok_used;
        let mut m_cloned = msg.clone();
        m_cloned.content = out.join("\n");
        results.push(m_cloned);
    }

    let tok_unused = tokens_limit.saturating_sub(tok_used_global);
    (results, tok_unused)
}

pub async fn postprocess_rag_stage_7_9(
    lines_in_files: &mut HashMap<PathBuf, Vec<Arc<FileLine>>>,
    lines_by_useful: &mut Vec<Arc<FileLine>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
    single_file_mode: bool,
    settings: &PostprocessSettings,
) -> Vec<ContextFile> {
    // 7. Sort
    lines_by_useful.sort_by(|a, b| {
        let av = a.useful + a.fref.cpath_symmetry_breaker;
        let bv = b.useful + b.fref.cpath_symmetry_breaker;
        bv.partial_cmp(&av).unwrap()
    });

    // 8. Convert line_content to tokens up to the limit
    let mut tokens_count: usize = 0;
    let mut lines_take_cnt: usize = 0;
    let mut files_mentioned_set: HashSet<String> = HashSet::new();
    let mut files_mentioned_sequence: Vec<PathBuf> = vec![];
    for lineref in lines_by_useful.iter_mut() {
        if lineref.useful <= settings.take_floor {
            continue;
        }
        let mut ntokens = count_tokens(&tokenizer.read().unwrap(), &lineref.line_content);
        let filename = lineref.fref.cpath.to_string_lossy().to_string();

        if !files_mentioned_set.contains(&filename) {
            if files_mentioned_set.len() >= settings.max_files_n {
                continue;
            }
            files_mentioned_set.insert(filename.clone());
            files_mentioned_sequence.push(lineref.fref.cpath.clone());
            if !single_file_mode {
                ntokens += count_tokens(&tokenizer.read().unwrap(), &filename.as_str());
                ntokens += 5;  // a margin for any overhead: file_sep, new line, etc
            }
        }
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
    if DEBUG >= 1 {
        info!("{} lines in {} files  =>  tokens {} < {} tokens limit  =>  {} lines in {} files", lines_by_useful.len(), lines_in_files.len(), tokens_count, tokens_limit, lines_take_cnt, files_mentioned_sequence.len());
    }
    if DEBUG >= 2 {
        for linevec in lines_in_files.values() {
            for lineref in linevec.iter() {
                info!("{} {}:{:04} {:>7.3} {}",
                if lineref.take { "take" } else { "dont" },
                crate::nicer_logs::last_n_chars(&lineref.fref.cpath.to_string_lossy().to_string(), 30),
                lineref.line_n,
                lineref.useful,
                crate::nicer_logs::first_n_chars(&lineref.line_content, 20)
            );
            }
        }
    }

    // 9. Generate output
    let mut merged: Vec<ContextFile> = vec![];
    for cpath in files_mentioned_sequence.iter() {
        let linevec = lines_in_files.get_mut(cpath).unwrap();
        if linevec.len() == 0 {
            continue;
        }
        let fref = linevec[0].fref.clone();
        let cpath = fref.cpath.clone();
        let mut out = String::new();
        let mut first_line: usize = 0;
        let mut last_line: usize = 0;
        let mut prev_line: usize = 0;
        let mut anything = false;
        for (i, lineref) in linevec.iter_mut().enumerate() {
            last_line = i;
            if !lineref.take {
                continue;
            }
            anything = true;
            if first_line == 0 { first_line = i; }
            if i > prev_line + 1 {
                // out.push_str(format!("...{} lines\n", i - prev_line - 1).as_str());
                out.push_str(format!("...\n").as_str());
            }
            out.push_str(&lineref.line_content);
            out.push_str("\n");
            prev_line = i;
        }
        if last_line > prev_line + 1 {
            out.push_str("...\n");
        }
        if DEBUG >= 2 {
            info!("file {:?}:\n{}", cpath, out);
        } else if DEBUG == 1 {
            info!("file {:?}:{}-{}", cpath, first_line, last_line);
        }
        if !anything {
            continue;
        }
        merged.push(ContextFile {
            file_name: cpath.to_string_lossy().to_string(),
            file_content: out.clone(),
            line1: first_line,
            line2: last_line,
            symbol: Uuid::default(),
            gradient_type: -1,
            usefulness: 0.0,
            is_body_important: false
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

pub struct HasRagResults {
    pub was_sent: bool,
    pub in_json: Vec<Value>,
}

impl HasRagResults {
    pub fn new() -> Self {
        HasRagResults {
            was_sent: false,
            in_json: vec![],
        }
    }
}

impl HasRagResults {
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
