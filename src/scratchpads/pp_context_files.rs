use std::sync::Arc;
use std::sync::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{info, warn};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use uuid::Uuid;
use crate::ast::treesitter::structs::SymbolType;

use crate::call_validation::ContextFile;
use crate::global_context::GlobalContext;
use crate::ast::structs::FileASTMarkup;
use crate::files_correction::{canonical_path, correct_to_nearest_filename};
use crate::nicer_logs::{first_n_chars, last_n_chars};
use crate::scratchpads::pp_utils::{add_usefulness_to_lines, color_with_gradient_type, colorize_comments_up, colorize_if_more_useful, colorize_minus_one, colorize_parentof, count_tokens, downgrade_lines_if_subsymbol, pp_ast_markup_files};


pub const RESERVE_FOR_QUESTION_AND_FOLLOWUP: usize = 1024;  // tokens
pub const DEBUG: usize = 0;  // 0 nothing, 1 summary "N lines in K files => X tokens", 2 everything


#[derive(Debug)]
pub struct PPFile {
    pub markup: FileASTMarkup,
    pub cpath: PathBuf,
    pub cpath_symmetry_breaker: f32,
}

#[derive(Debug, Clone)]
pub struct FileLine {
    pub file_ref: Arc<PPFile>,
    pub line_n: usize,
    pub line_content: String,
    pub useful: f32,
    pub color: String,
    pub take: bool,
}

pub struct PostprocessSettings {
    pub useful_background: f32,          // first, fill usefulness of all lines with this
    pub useful_symbol_default: f32,      // when a symbol present, set usefulness higher
    // search results fill usefulness as it passed from outside
    pub downgrade_parent_coef: f32,        // goto parent from search results and mark it useful, with this coef
    pub downgrade_body_coef: f32,          // multiply body usefulness by this, so it's less useful than the declaration
    pub comments_propagate_up_coef: f32, // mark comments above a symbol as useful, with this coef
    pub close_small_gaps: bool,
    pub take_floor: f32,                 // take/dont value
    pub max_files_n: Option<usize>,              // don't produce more than n files in output
}

impl PostprocessSettings {
    pub fn new() -> Self {
        PostprocessSettings {
            downgrade_body_coef: 0.8,
            downgrade_parent_coef: 0.6,
            useful_background: 5.0,
            useful_symbol_default: 10.0,
            close_small_gaps: true,
            comments_propagate_up_coef: 0.99,
            take_floor: 0.0,
            max_files_n: Some(10),
        }
    }
}

fn collect_lines_from_files(files: Vec<Arc<PPFile>>, settings: &PostprocessSettings) -> HashMap<PathBuf, Vec<FileLine>> {
    let mut lines_in_files = HashMap::new();
    for file_ref in files {
        for (line_n, line) in file_ref.markup.file_content.lines().enumerate() {
            let a = FileLine {
                file_ref: file_ref.clone(),
                line_n,
                line_content: line.to_string(),
                useful: 0.0,
                color: "".to_string(),
                take: false,
            };
            let lines_in_files_mut = lines_in_files.entry(file_ref.cpath.clone()).or_insert(vec![]);
            lines_in_files_mut.push(a);
        }
    }
    for lines in lines_in_files.values_mut().filter(|x|!x.is_empty()) {
        let file = lines.first().unwrap().file_ref.clone();
        if DEBUG >= 2 {
            info!("file_ref {:?} has {} bytes, {} symbols", file.cpath, file.markup.file_content.len(), file.markup.symbols_sorted_by_path_len.len());
        }
        colorize_if_more_useful(lines, 0, lines.len(), "empty".to_string(), settings.useful_background);
        for s in file.markup.symbols_sorted_by_path_len.iter() {
            if DEBUG >= 2 {
                info!("    {} {:?} {}-{}", s.symbol_path, s.symbol_type, s.full_range.start_point.row, s.full_range.end_point.row);
            }
            if s.symbol_type == SymbolType::CommentDefinition {
                let useful = settings.useful_symbol_default;
                colorize_if_more_useful(lines, s.full_range.start_point.row, s.full_range.end_point.row+1, "comment".to_string(), useful);
            } else {
                let useful = settings.useful_symbol_default;  // depends on symbol type?
                add_usefulness_to_lines(lines, s.full_range.start_point.row, s.full_range.end_point.row+1, format!("{}", s.symbol_path), useful);
            }
        }
    }
    lines_in_files
}

async fn set_lines_usefulness(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ContextFile>,
    lines_in_files: &mut HashMap<PathBuf, Vec<FileLine>>,
    settings: &PostprocessSettings,
) {
    for msg in messages.iter() {
        // Do what we can to match msg.file_name to something real
        let candidates = correct_to_nearest_filename(global_context.clone(), &msg.file_name, false, 1).await;
        let c_path = match candidates.first() {
            Some(c) => canonical_path(&c),
            None => canonical_path(&msg.file_name)
        };
        let lines = match lines_in_files.get_mut(&c_path) {
            Some(x) => x,
            None => {
                warn!("file not found by name {:?} or cpath {:?}", msg.file_name, c_path);
                continue;
            }
        };
        if lines.is_empty() {
            continue;
        }
        if msg.usefulness.is_sign_negative() {  // used in FIM to disable lines already in suffix or prefix
            colorize_minus_one(lines, msg.line1-1, msg.line2);
            continue;
        }

        color_with_gradient_type(msg, lines);

        let file_ref = lines.first().unwrap().file_ref.clone();

        let mut symbols_to_color = vec![];
        if !msg.symbol.is_empty() && !(msg.symbol.len() == 1 && msg.symbol.first().unwrap_or(&Uuid::default()).is_nil()) {
            for sym in msg.symbol.iter() {
                if sym.is_nil() {
                    continue;
                }
                let init_len = symbols_to_color.len();
                for x in file_ref.markup.symbols_sorted_by_path_len.iter() {
                    if x.guid == *sym {
                        symbols_to_color.push(x);
                        break;
                    }
                }
                if init_len == symbols_to_color.len() {
                    warn!("- cannot find symbol {} in file {}:{}-{}", sym, msg.file_name, msg.line1, msg.line2);
                }
            }
        }

        if !msg.is_body_important && !symbols_to_color.is_empty() {
            for s in symbols_to_color {
                if DEBUG >= 1 {
                    info!("+ search result {} {:?} {:.2}", s.symbol_path, s.symbol_type, msg.usefulness);
                }
                colorize_if_more_useful(lines, s.full_range.start_point.row, s.full_range.end_point.row+1, format!("{}", s.symbol_path), msg.usefulness);
                let mut parent_path = s.symbol_path.split("::").collect::<Vec<_>>();
                if parent_path.len() > 1 {
                    // MyClass::f  ->  MyClass
                    // make parent stand out from background as well, to make it clearer to the model where the symbol is
                    parent_path.pop();
                    let parent_path_str = parent_path.join("::");
                    colorize_parentof(lines, &parent_path_str, settings.useful_symbol_default, msg.usefulness*settings.downgrade_parent_coef);
                }
            }
        } else {
            // no symbol set in search result, go head with just line numbers, omsg.line1, omsg.line2 numbers starts from 1, not from 0
            info!("+ search result from vecdb or @file {:.2}", msg.usefulness);
            if msg.line1 == 0 || msg.line2 == 0 || msg.line1 > msg.line2 || msg.line1 > lines.len() || msg.line2 > lines.len() {
                warn!("range in search results is outside of file lines that actually exist {}:{}-{}; actual len: {}", msg.file_name, msg.line1, msg.line2, lines.len());
            }
            colorize_if_more_useful(lines, msg.line1.saturating_sub(1), msg.line2.saturating_sub(1), "nosymb".to_string(), msg.usefulness);
        }
        // example: see comment in class Toad
        colorize_comments_up(lines, settings);
    }
}

fn downgrade_sub_symbols(lines_in_files: &mut HashMap<PathBuf, Vec<FileLine>>, settings: &PostprocessSettings) {
    for lines in lines_in_files.values_mut().filter(|x|!x.is_empty()) {
        let file_ref = lines.first().unwrap().file_ref.clone();
        if DEBUG >= 2 {
            info!("downgrading body of symbols in {:?}", file_ref.cpath);
        }
        for s in file_ref.markup.symbols_sorted_by_path_len.iter() {
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
                    downgrade_lines_if_subsymbol(lines, def0, def1, &format!("{}::body", s.symbol_path), settings.downgrade_body_coef);
                    // NOTE: this will not downgrade function body of a function that is a search result, because it's not a subsymbol it's the symbol itself (equal path)
                }
            }
        }
    }
}

fn close_small_gaps(lines_in_files: &mut HashMap<PathBuf, Vec<FileLine>>, settings: &PostprocessSettings) {
    if settings.close_small_gaps {
        for lines in lines_in_files.values_mut().filter(|x|!x.is_empty()) {
            let mut useful_copy = lines.iter().map(|x| x.useful).collect::<Vec<_>>();
            for i in 1..lines.len() - 1 {
                let l = lines[i-1].useful;
                let m = lines[i].useful;
                let r = lines[i+1].useful;
                let both_l_and_r_support = l.min(r);
                useful_copy[i] = m.max(both_l_and_r_support);
            }
            for i in 0..lines.len() {
                if let Some(line) = lines.get_mut(i) {
                    line.useful = useful_copy[i];
                }
            }
        }
    }
}

pub async fn pp_color_lines(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ContextFile>,
    files: Vec<Arc<PPFile>>,
    settings: &PostprocessSettings,
) -> HashMap<PathBuf, Vec<FileLine>> {
    // Generate line refs, fill background scopes found in a file (not search results yet)
    let mut lines_in_files = collect_lines_from_files(files, settings);
    
    // Fill in usefulness from search results
    set_lines_usefulness(global_context.clone(), messages, &mut lines_in_files, settings).await;

    // Downgrade sub-symbols and uninteresting regions
    downgrade_sub_symbols(&mut lines_in_files, settings);

    // A-la mathematical morphology, removes one-line holes
    close_small_gaps(&mut lines_in_files, settings);

    lines_in_files
}

async fn pp_limit_and_merge(
    lines_in_files: &mut HashMap<PathBuf, Vec<FileLine>>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
    single_file_mode: bool,
    settings: &PostprocessSettings,
) -> Vec<ContextFile> {
    // Sort
    let mut lines_by_useful = lines_in_files.values_mut().flatten().collect::<Vec<_>>();
    
    lines_by_useful.sort_by(|a, b| {
        let av = a.useful + a.file_ref.cpath_symmetry_breaker;
        let bv = b.useful + b.file_ref.cpath_symmetry_breaker;
        bv.partial_cmp(&av).unwrap()
    });

    // Convert line_content to tokens up to the limit
    let mut tokens_count = 0;
    let mut lines_take_cnt = 0;
    let mut files_mentioned_set = HashSet::new();
    let mut files_mentioned_sequence = vec![];
    for line_ref in lines_by_useful.iter_mut() {
        if line_ref.useful <= settings.take_floor {
            continue;
        }
        let mut ntokens = count_tokens(&tokenizer.read().unwrap(), &line_ref.line_content);
        let filename = line_ref.file_ref.cpath.to_string_lossy().to_string();

        if !files_mentioned_set.contains(&filename) {
            if let Some(max_files_n) = settings.max_files_n {
                if files_mentioned_set.len() >= max_files_n {
                    continue;
                }
            }
            files_mentioned_set.insert(filename.clone());
            files_mentioned_sequence.push(line_ref.file_ref.cpath.clone());
            if !single_file_mode {
                ntokens += count_tokens(&tokenizer.read().unwrap(), &filename.as_str());
                ntokens += 5;  // a margin for any overhead: file_sep, new line, etc
            }
        }
        if tokens_count + ntokens > tokens_limit {
            break;
        }
        tokens_count += ntokens;
        line_ref.take = true;
        lines_take_cnt += 1;
    }
    if DEBUG >= 1 {
        info!("{} lines in {} files  =>  tokens {} < {} tokens limit  =>  {} lines in {} files", lines_by_useful.len(), lines_in_files.len(), tokens_count, tokens_limit, lines_take_cnt, files_mentioned_sequence.len());
    }
    if DEBUG >= 2 {
        for lines in lines_in_files.values() {
            for line_ref in lines.iter() {
                info!("{} {}:{:04} {:>7.3} {}",
                if line_ref.take { "take" } else { "dont" },
                last_n_chars(&line_ref.file_ref.cpath.to_string_lossy().to_string(), 30),
                line_ref.line_n,
                line_ref.useful,
                first_n_chars(&line_ref.line_content, 20)
            );
            }
        }
    }

    // Generate output
    let mut context_files_merged = vec![];
    for cpath in files_mentioned_sequence.iter() {
        let lines = lines_in_files.get_mut(cpath).unwrap();
        if lines.is_empty() {
            continue;
        }
        let file_ref = lines.first().unwrap().file_ref.clone();
        let cpath = file_ref.cpath.clone();
        let (mut out, mut first_line, mut last_line, mut prev_line, mut anything) = (String::new(), 0, 0, 0, false);
        for (i, line_ref) in lines.iter_mut().enumerate() {
            last_line = i;
            if !line_ref.take {
                continue;
            }
            anything = true;
            if first_line == 0 { first_line = i; }
            if i > prev_line + 1 {
                out.push_str("...\n".to_string().as_str());
            }
            out.push_str(&line_ref.line_content);
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
        context_files_merged.push(ContextFile {
            file_name: cpath.to_string_lossy().to_string(),
            file_content: out.clone(),
            line1: first_line,
            line2: last_line,
            symbol: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            is_body_important: false
        });
    }
    context_files_merged
}

pub async fn postprocess_context_files(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ContextFile>,
    tokenizer: Arc<RwLock<Tokenizer>>,
    tokens_limit: usize,
    single_file_mode: bool,
    max_files_n: Option<usize>,
) -> Vec<ContextFile> {
    let files_marked_up = pp_ast_markup_files(global_context.clone(), &messages).await;

    let mut settings = PostprocessSettings::new();
    settings.max_files_n = max_files_n;
    
    let mut lines_in_files = pp_color_lines(
        global_context.clone(),
        &messages,
        files_marked_up,
        &settings,
    ).await;

    pp_limit_and_merge(
        &mut lines_in_files, 
        tokenizer, 
        tokens_limit, 
        single_file_mode, 
        &settings
    ).await
}
