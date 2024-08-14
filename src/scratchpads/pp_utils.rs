use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{info, warn};
use serde_json::Value;
use tokenizers::Tokenizer;
use tokio::sync::RwLock as ARwLock;
use std::hash::{Hash, Hasher};

use crate::call_validation::ContextFile;
use crate::global_context::GlobalContext;
use crate::ast::structs::FileASTMarkup;
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};
use crate::scratchpads::pp_context_files::{PPFile, FileLine, PostprocessSettings, DEBUG, RESERVE_FOR_QUESTION_AND_FOLLOWUP};


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

pub fn count_tokens(
    tokenizer: &Tokenizer,
    text: &str,
) -> usize {
    match tokenizer.encode(text, false) {
        Ok(tokens) => tokens.len(),
        Err(_) => 0,
    }
}

pub fn max_tokens_for_rag_chat(n_ctx: usize, maxgen: usize) -> usize {
    (n_ctx/2).saturating_sub(maxgen).saturating_sub(RESERVE_FOR_QUESTION_AND_FOLLOWUP)
}

pub fn color_with_gradient_type(msg: &ContextFile, lines: &mut Vec<FileLine>) {
    fn find_line_parameters(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32) {
        if y2 - y1 == 0. || x2 - x1 == 0. {
            return (0., 0.);
        }
        let m = (y2 - y1) / (x2 - x1);
        let c = y1 - m * x1;
        (m, c)
    }

    if msg.gradient_type < 0 || msg.gradient_type > 4 {
        return;
    }

    let t_fade_away_lines = 50;
    let (m11, c11) = find_line_parameters(msg.line1 as f32, msg.usefulness, msg.line1 as f32 - t_fade_away_lines as f32, 0. );
    let (m12, c12) = find_line_parameters(msg.line1 as f32, msg.usefulness, msg.line1 as f32 + t_fade_away_lines as f32, 0. );
    let (m21, c21) = find_line_parameters(msg.line2 as f32, msg.usefulness, msg.line2 as f32 - t_fade_away_lines as f32, 0. );
    let (m22, c22) = find_line_parameters(msg.line2 as f32, msg.usefulness, msg.line2 as f32 + t_fade_away_lines as f32, 0. );

    for (line_n, line) in lines.iter_mut().enumerate() {
        let line_n = line_n + 1;
        let usefulness = match msg.gradient_type {
            0 => msg.usefulness - (line_n as f32) * 0.001,
            1 => if line_n < msg.line1 {(line_n as f32 * m11 + c11).max(0.)} else {(line_n as f32 * m12 + c12).max(0.)},
            2 => if line_n <= msg.line2 {(line_n as f32 * m21 + c21).max(0.) } else {-1.},
            3 => if line_n < msg.line1 {-1.} else {(line_n as f32 * m12 + c12).max(0.)},
            4 => {
                if line_n < msg.line1 {
                    line_n as f32 * m11 + c11
                } else if line_n >= msg.line1 && line_n <= msg.line2 {
                    100.
                } else {
                    line_n as f32 * m22 + c22
                }
            }.max(0.),
            _ => 0.0,
        };
        info!("applying gradient type {:?} to line {:04}", msg.gradient_type, line_n);
        set_useful_for_line(line, usefulness, format!("gradient_type: {:?}", msg.gradient_type));
    }
}

fn set_useful_for_line(line: &mut FileLine, useful: f32, color: String) {
    if (line.useful < useful) || useful < 0. {
        line.useful = useful;
        line.color = color;
    }
}

fn calculate_hash(path: &PathBuf) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

pub async fn pp_ast_markup_files(
    global_context: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ContextFile>,
) -> Vec<Arc<PPFile>> {
    let mut files_markup: HashMap<String, Arc<PPFile>> = HashMap::new();
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
        let mut f: Option<Arc<PPFile>> = None;
        if let Some(ast) = &ast_module {
            match ast.read().await.file_markup(&doc).await {
                Ok(markup) => {
                    f = Some(Arc::new(PPFile { markup, cpath: path, cpath_symmetry_breaker }));
                },
                Err(err) => {
                    warn!("postprocess_rag_stage1 query file {:?} markup problem: {}", file_name, err);
                }
            }
        }
        match f {
            None => {
                f = Some(Arc::new(PPFile {
                    markup: FileASTMarkup {
                        file_path: doc.path.clone(),
                        file_content: text,
                        symbols_sorted_by_path_len: Vec::new(),
                    },
                    cpath: doc.path.clone(),
                    cpath_symmetry_breaker,
                }));
                files_markup.insert(file_name.clone(), f.unwrap());
            },
            Some(_) => {
                files_markup.insert(file_name.clone(), f.unwrap());
            }
        }
    }
    files_markup.values().cloned().collect::<Vec<_>>()
}

pub fn colorize_if_more_useful(lines: &mut Vec<FileLine>, line1: usize, line2: usize, color: String, useful: f32) {
    if DEBUG >= 2 {
        info!("    colorize_if_more_useful {}..{} <= color {:?} useful {}", line1, line2, color, useful);
    }
    for i in line1..line2 {
        if i >= lines.len() {
            warn!("    {} has faulty range {}..{}", color, line1, line2);
            continue;
        }
        if let Some(line) = lines.get_mut(i) {
            let u = useful - (i as f32) * 0.001;
            if line.useful < u || line.color.is_empty() {
                line.useful = u;
                line.color = color.clone();
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
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.,
            is_body_important: false
        });
    }
    messages
}

pub fn colorize_parentof(lines: &mut Vec<FileLine>, long_child_path: &String, bg: f32, maxuseful: f32) {
    if DEBUG >= 2 {
        info!("    colorize_parentof long_child_path={} bg={} maxuseful={}", long_child_path, bg, maxuseful);
    }
    for i in 0..lines.len() {
        if let Some(line) = lines.get_mut(i) {
            let color = &line.color;
            if long_child_path.starts_with(color) && color.len() > 0 {
                let plen = line.color.len();
                let long = long_child_path.len();
                let mut u = bg + (maxuseful - bg)*(plen as f32)/(long as f32);
                u -= (i as f32) * 0.001;
                if line.useful < u {
                    if DEBUG >= 2 {
                        info!("    colorize_parentof line{:04} {} <= {:>7.3}", i, color, u);
                    }
                    line.useful = u;
                }
            }
        }
    }
}

pub fn colorize_minus_one(lines: &mut Vec<FileLine>, line1: usize, line2: usize) {
    for i in line1..line2 {
        if let Some(line) = lines.get_mut(i) {
            line.useful = -1.;
            line.color = "disabled".to_string();
        }
    }
}

pub fn colorize_comments_up(lines: &mut Vec<FileLine>, settings: &PostprocessSettings) {
    for i in (0 .. lines.len() - 1).rev() {
        let next_line = lines.get(i+1).map(|x|x.clone());
        let this_line = lines.get_mut(i);
        if this_line.is_none() || next_line.is_none() {
            continue;
        }
        let this_line = this_line.unwrap();
        let next_line = next_line.unwrap();
        
        let u = next_line.useful * settings.comments_propagate_up_coef;
        if this_line.color == "comment" && this_line.useful < u {
            this_line.useful = u;
            if DEBUG >= 2 {
                info!("    comments_up_from_symbol line{:04} <= {:>7.3}", i, u);
            }
        }
    }
}

pub fn downgrade_lines_if_subsymbol(lines: &mut Vec<FileLine>, line1_base0: usize, line2_base0: usize, subsymbol: &String, downgrade_coef: f32) {
    let mut changes_cnt = 0;
    for i in line1_base0 .. line2_base0 {
        if i >= lines.len() {
            continue;
        }
        if let Some(line) = lines.get_mut(i) {
            if subsymbol.starts_with(&line.color) {
                if i == line2_base0-1 || i == line1_base0 {
                    if line.line_content.trim().len() == 1 {  // only closing bracket -- don't degrade, for C++ void f()  { ... }  last line with "}" only
                        continue;
                    }
                }
                line.useful *= downgrade_coef;
                line.color = subsymbol.clone();
                changes_cnt += 1;
            }
        }
    }
    if DEBUG >= 2 {
        info!("        {}..{} ({} affected) <= subsymbol {:?} downgrade {}", changes_cnt, line1_base0, line2_base0, subsymbol, downgrade_coef);
    }
}
