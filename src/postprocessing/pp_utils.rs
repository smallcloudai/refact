use std::sync::Arc;
use std::collections::HashSet;
use indexmap::IndexSet;
use std::path::PathBuf;
use tracing::{info, warn};
use tokio::sync::RwLock as ARwLock;
use std::hash::{Hash, Hasher};

use crate::call_validation::{ContextFile, PostprocessSettings};
use crate::global_context::GlobalContext;
use crate::files_in_workspace::{Document, get_file_text_from_memory_or_disk};
use crate::files_correction::shortify_paths;
use crate::postprocessing::pp_context_files::{PPFile, FileLine, DEBUG};


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
    gcx: Arc<ARwLock<GlobalContext>>,
    context_file_vec: &mut Vec<ContextFile>,
) -> Vec<Arc<PPFile>> {
    let mut unique_cpaths = IndexSet::<String>::new();
    for context_file in context_file_vec.iter_mut() {
        // Here we assume data came from outside, we can't trust it too much
        let path_as_presented = context_file.file_name.clone();
        let candidates = crate::files_correction::correct_to_nearest_filename(gcx.clone(), &path_as_presented, false, 5).await;
        let cpath = match candidates.first() {
            Some(c) => crate::files_correction::canonical_path(c),
            None => crate::files_correction::canonical_path(&path_as_presented)
        };
        context_file.file_name = cpath.to_string_lossy().to_string();
        if candidates.len() != 1 {
            tracing::warn!("{:?} -> snap {:?} -> {:?}", path_as_presented, candidates, context_file.file_name);
        }
        unique_cpaths.insert(context_file.file_name.clone());
    }

    let unique_cpaths_vec: Vec<String> = unique_cpaths.into_iter().collect();
    let shortified_vec: Vec<String> = shortify_paths(gcx.clone(), &unique_cpaths_vec).await;

    let mut result: Vec<Arc<PPFile>> = vec![];
    let ast_service = gcx.read().await.ast_service.clone();
    for (cpath, short) in unique_cpaths_vec.iter().zip(shortified_vec.iter()) {
        let cpath_pathbuf = PathBuf::from(cpath);
        let cpath_symmetry_breaker: f32 = (calculate_hash(&cpath_pathbuf) as f32) / (u64::MAX as f32) / 100.0;
        let mut doc = Document::new(&cpath_pathbuf);
        let text = match get_file_text_from_memory_or_disk(gcx.clone(), &doc.doc_path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("pp_ast_markup_files: cannot read file {:?}, not a big deal, will just skip the file. The problem was: {}", cpath, e);
                continue;
            }
        };
        doc.update_text(&text);
        let defs = if let Some(ast) = &ast_service {
            let ast_index = ast.lock().await.ast_index.clone();
            crate::ast::ast_db::doc_defs(ast_index.clone(), &doc.doc_path.to_string_lossy().to_string()).await
        } else {
            vec![]
        };
        let mut symbols_sorted_by_path_len = defs.clone();
        symbols_sorted_by_path_len.sort_by_key(|s| s.official_path.len());
        result.push(Arc::new(PPFile {  // doesn't matter what size the output vector is
            symbols_sorted_by_path_len,
            file_content: text,
            cpath: cpath.clone(),
            cpath_symmetry_breaker,
            shorter_path: short.clone(),
        }));
    }

    result
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
    // XXX: only used once in a test handler, maybe remove?
    let mut messages = vec![];
    for file_name in files_set {
        let path = crate::files_correction::canonical_path(&file_name);
        let text = match get_file_text_from_memory_or_disk(global_context.clone(), &path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("context_msgs_from_paths: cannot read file {:?}, not a big deal, will just skip the file.\nThe problem was: {}", file_name, e);
                continue;
            }
        };
        messages.push(ContextFile {
            file_name: file_name.clone(),
            file_content: text.clone(),
            line1: 0,
            line2: text.lines().count(),
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.,
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
            if i == line2_base0-1 || i == line1_base0 {
                if line.line_content.trim().len() == 1 {  // only closing bracket -- don't degrade, for C++ void f()  { ... }  last line with "}" only
                    continue;
                }
            }
            line.useful *= downgrade_coef;
            changes_cnt += 1;
            if subsymbol.starts_with(&line.color) {
                line.color = subsymbol.clone();
            }
        }
    }
    if DEBUG >= 2 {
        info!("        {}..{} ({} affected) <= subsymbol {:?} downgrade {}", line1_base0, line2_base0, changes_cnt, subsymbol, downgrade_coef);
    }
}
