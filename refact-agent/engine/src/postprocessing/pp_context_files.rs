use std::sync::Arc;
use std::collections::HashSet;
use tracing::{info, warn};
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use indexmap::IndexMap;
use crate::ast::treesitter::structs::SymbolType;

use crate::call_validation::{ContextFile, PostprocessSettings};
use crate::ast::ast_structs::AstDefinition;
use crate::global_context::GlobalContext;
use crate::nicer_logs::{first_n_chars, last_n_chars};
use crate::postprocessing::pp_utils::{color_with_gradient_type, colorize_comments_up, colorize_if_more_useful, colorize_minus_one, colorize_parentof, downgrade_lines_if_subsymbol, pp_ast_markup_files};
use crate::tokens::count_text_tokens_with_fallback;


pub const RESERVE_FOR_QUESTION_AND_FOLLOWUP: usize = 1024;  // tokens
pub const DEBUG: usize = 0;  // 0 nothing, 1 summary "N lines in K files => X tokens", 2 everything


#[derive(Debug)]
pub struct PPFile {
    pub symbols_sorted_by_path_len: Vec<Arc<AstDefinition>>,
    pub file_content: String,
    pub cpath: String,
    pub cpath_symmetry_breaker: f32,
    pub shorter_path: String,
}

#[derive(Debug, Clone)]
pub struct FileLine {
    pub file_ref: Arc<PPFile>,
    pub line_n: usize,
    pub line_content: String,
    pub useful: f32,
    pub color: String,
    pub take: bool,
    pub take_ignoring_floor: bool,  // if no ast for this file, then ignore the take_floor
}


fn collect_lines_from_files(
    files: Vec<Arc<PPFile>>,
    settings: &PostprocessSettings
) -> IndexMap<String, Vec<FileLine>> {
    let mut lines_in_files = IndexMap::new();
    for file_ref in files {
        for (line_n, line) in file_ref.file_content.lines().enumerate() {
            let a = FileLine {
                file_ref: file_ref.clone(),
                line_n,
                line_content: line.to_string(),
                useful: 0.0,
                color: "".to_string(),
                take: false,
                take_ignoring_floor: false,
            };
            let lines_in_files_mut = lines_in_files.entry(file_ref.cpath.clone()).or_insert(vec![]);
            lines_in_files_mut.push(a);
        }
    }
    for lines in lines_in_files.values_mut().filter(|x|!x.is_empty()) {
        let file = lines.first().unwrap().file_ref.clone();
        if DEBUG >= 2 {
            info!("file_ref {:?} has {} bytes, {} symbols", file.cpath, file.file_content.len(), file.symbols_sorted_by_path_len.len());
        }
        for s in file.symbols_sorted_by_path_len.iter() {
            if DEBUG >= 2 {
                info!("    {} {:?} {}-{}", s.path(), s.symbol_type, s.full_line1(), s.full_line2());
            }
            if s.symbol_type == SymbolType::CommentDefinition {
                let useful = settings.useful_symbol_default;
                colorize_if_more_useful(lines, s.full_line1() - 1, s.full_line2(), "comment".to_string(), useful);
            } else {
                let mut useful = settings.useful_symbol_default;
                if s.symbol_type == SymbolType::StructDeclaration {
                    useful = 65.0;
                }
                if s.symbol_type == SymbolType::FunctionDeclaration {
                    useful = 55.0;
                }
                colorize_if_more_useful(lines, s.full_line1() - 1, s.full_line2(), format!("{}", s.path()), useful);
            }
        }
        colorize_if_more_useful(lines, 0, lines.len(), "empty".to_string(), settings.useful_background);
    }

    for (file_name, lines) in lines_in_files.iter_mut() {
        let file = lines.first().unwrap().file_ref.clone();
        if file.symbols_sorted_by_path_len.is_empty() {
            info!("{file_name} ignoring skeletonize because no symbols found in the file, maybe the file format is not supported or the file is empty");
            lines.iter_mut().for_each(|x| x.take_ignoring_floor = true);
        }
    }

    lines_in_files
}

async fn convert_input_into_usefullness(
    context_file_vec: &Vec<ContextFile>,
    lines_in_files: &mut IndexMap<String, Vec<FileLine>>,
    settings: &PostprocessSettings,
) {
    for msg in context_file_vec.iter() {
        let lines = match lines_in_files.get_mut(&msg.file_name) {
            Some(x) => x,
            None => {
                warn!("file not found by name {:?} or cpath {:?}", msg.file_name, msg.file_name);
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
        let file_nice_path = last_n_chars(&file_ref.cpath, 30);

        let mut symdefs = vec![];
        if !msg.symbols.is_empty() {
            for looking_for in msg.symbols.iter() {
                let colon_colon_looking_for = format!("::{}", looking_for.trim());
                for x in file_ref.symbols_sorted_by_path_len.iter() {
                    if x.path().ends_with(colon_colon_looking_for.as_str()) {
                        symdefs.push(x);
                        break;
                    }
                }
            }
        }

        if !symdefs.is_empty() {
            for s in symdefs {
                info!("+ symbol {} at {}:{}-{} usefulness={:.2}", s.path_drop0(), file_nice_path, msg.line1, msg.line2, msg.usefulness);
                if DEBUG >= 1 {
                    info!("+ search result {} {:?} {:.2}", s.path(), s.symbol_type, msg.usefulness);
                }
                colorize_if_more_useful(lines, s.full_line1() - 1, s.full_line2(), format!("{}", s.path()), msg.usefulness);
                let mut parent_path = s.official_path.clone();
                if parent_path.len() > 1 {
                    // MyClass::f  ->  MyClass
                    // make parent stand out from background as well, to make it clearer to the model where the symbol is
                    parent_path.pop();
                    let parent_path_str = parent_path.join("::");
                    colorize_parentof(lines, &parent_path_str, settings.useful_symbol_default, msg.usefulness*settings.downgrade_parent_coef);
                }
            }

        } else if msg.line1 == 0 && msg.line2 == 0 && msg.symbols.is_empty() {
            info!("+ file mention without specifics, {}:{}-{} usefulness={:.2}", file_nice_path, msg.line1, msg.line2, msg.usefulness);
            colorize_if_more_useful(lines, 0, lines.len(), "nosymb".to_string(), msg.usefulness);

        } else if msg.line1 == 0 && msg.line2 == 0 && !msg.symbols.is_empty() {
            info!("- symbols {:?} not found in {}:{}-{} usefulness={:.2}", msg.symbols, file_nice_path, msg.line1, msg.line2, msg.usefulness);
            colorize_if_more_useful(lines, 0, lines.len(), "nosymb".to_string(), msg.usefulness);

        } else {
            // no symbol set in search result, go ahead with just line numbers, msg.line1, msg.line2 numbers starts from 1, not from 0
            info!("+ search result without symbol, {}:{}-{} usefulness={:.2}", file_nice_path, msg.line1, msg.line2, msg.usefulness);
            if msg.line1 == 0 || msg.line2 == 0 || msg.line1 > msg.line2 || msg.line1 > lines.len() || msg.line2 > lines.len() {
                warn!("range in search results is outside of file lines that actually exist {}:{}-{}; actual len: {}", file_nice_path, msg.line1, msg.line2, lines.len());
            }
            colorize_if_more_useful(lines, msg.line1.saturating_sub(1), msg.line2, "nosymb".to_string(), msg.usefulness);
        }

        // example: see comment in class Toad
        colorize_comments_up(lines, settings);
    }
}

fn downgrade_sub_symbols(lines_in_files: &mut IndexMap<String, Vec<FileLine>>, settings: &PostprocessSettings)
{
    for lines in lines_in_files.values_mut().filter(|x|!x.is_empty()) {
        let file_ref = lines.first().unwrap().file_ref.clone();
        if DEBUG >= 2 {
            info!("downgrading body of symbols in {:?}", file_ref.cpath);
        }
        for s in file_ref.symbols_sorted_by_path_len.iter() {
            if DEBUG >= 2 {
                info!("    {} {:?} {}-{}", s.path(), s.symbol_type, s.full_line1(), s.full_line2());
            }
            if s.body_line1 > 0 && s.body_line1 >= s.body_line2 {
                downgrade_lines_if_subsymbol(lines, s.body_line1 - 1, s.body_line1, &format!("{}::body", s.path()), settings.downgrade_body_coef);
                // NOTE: this will not downgrade function body of a function that is a search result, because it's not a subsymbol it's the symbol itself (equal path)
            }
        }
    }
}

fn close_small_gaps(lines_in_files: &mut IndexMap<String, Vec<FileLine>>, settings: &PostprocessSettings) {
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
    context_file_vec: &Vec<ContextFile>,
    files: Vec<Arc<PPFile>>,
    settings: &PostprocessSettings,
) -> IndexMap<String, Vec<FileLine>> {
    // Generate line refs, fill background scopes found in a file (not search results yet)
    let mut lines_in_files = collect_lines_from_files(files, settings);

    // Fill in usefulness from search results
    convert_input_into_usefullness(context_file_vec, &mut lines_in_files, settings).await;

    // Downgrade sub-symbols and uninteresting regions
    downgrade_sub_symbols(&mut lines_in_files, settings);

    // A-la mathematical morphology, removes one-line holes
    close_small_gaps(&mut lines_in_files, settings);

    lines_in_files
}

async fn pp_limit_and_merge(
    lines_in_files: &mut IndexMap<String, Vec<FileLine>>,
    tokenizer: Option<Arc<Tokenizer>>,
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
        if !line_ref.take_ignoring_floor && line_ref.useful <= settings.take_floor {
            continue;
        }
        let mut ntokens = count_text_tokens_with_fallback(tokenizer.clone(), &line_ref.line_content);

        if !files_mentioned_set.contains(&line_ref.file_ref.cpath) {
            if files_mentioned_set.len() >= settings.max_files_n {
                continue;
            }
            files_mentioned_set.insert(line_ref.file_ref.cpath.clone());
            files_mentioned_sequence.push(line_ref.file_ref.cpath.clone());
            if !single_file_mode {
                ntokens += count_text_tokens_with_fallback(tokenizer.clone(), &line_ref.file_ref.cpath.as_str());
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
            let mut t = String::new();
            for line_ref in lines.iter() {
                t.push_str(format!("{} {}:{:04} {:>7.3} {:43} {:43}\n",
                    if line_ref.take { "take" } else { "dont" },
                    last_n_chars(&line_ref.file_ref.cpath, 30),
                    line_ref.line_n,
                    line_ref.useful,
                    first_n_chars(&line_ref.line_content, 40),
                    first_n_chars(&line_ref.color, 40),
                ).as_str());
            }
            info!("\n{}", t);
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
            file_name: file_ref.shorter_path.clone(),
            file_content: out.clone(),
            line1: first_line,
            line2: last_line,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
        });
    }
    context_files_merged
}

pub async fn postprocess_context_files(
    gcx: Arc<ARwLock<GlobalContext>>,
    context_file_vec: &mut Vec<ContextFile>,
    tokenizer: Option<Arc<Tokenizer>>,
    tokens_limit: usize,
    single_file_mode: bool,
    settings: &PostprocessSettings,
) -> Vec<ContextFile> {
    assert!(settings.max_files_n > 0);
    let files_marked_up = pp_ast_markup_files(gcx.clone(), context_file_vec).await;  // this modifies context_file.file_name to make it cpath

    let mut lines_in_files = pp_color_lines(
        context_file_vec,
        files_marked_up,
        settings,
    ).await;

    pp_limit_and_merge(
        &mut lines_in_files,
        tokenizer,
        tokens_limit,
        single_file_mode,
        settings
    ).await
}
