use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use serde::Serialize;

use tokio::sync::RwLock as ARwLock;
use hashbrown::{HashMap, HashSet};
use tracing::info;
use crate::at_commands::at_file::file_repair_candidates;
use crate::call_validation::DiffChunk;
use crate::global_context::GlobalContext;


const DEBUG: usize = 0;


#[derive(Clone, Debug, Default)]
struct DiffLine {
    line_n: usize,
    text: String,
    overwritten_by_id: Option<usize>,
}

#[derive(PartialEq, Debug)]
pub enum ApplyDiffOutput {
    Ok(),
    Err(String),
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct ApplyDiffResult {
    pub file_text: Option<String>,
    pub file_name_edit: Option<String>,
    pub file_name_delete: Option<String>,
    pub file_name_add: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ApplyDiffUnwrapped {
    pub chunk_id: usize,
    pub applied: bool,
    pub can_unapply: bool,
    pub success: bool,
    pub detail: Option<String>
}

fn validate_chunk(chunk: &DiffChunk) -> Result<(), String> {
    if chunk.line1 < 1 {
        return Err("Invalid line range: line1 cannot be < 1".to_string());
    }
    if chunk.line2 < chunk.line1 {
        return Err("Invalid line range: line2 cannot be < line1".to_string());
    }
    if !vec!["edit", "add", "rename", "remove"].contains(&chunk.file_action.as_str()) {
        return Err("Invalid file action: file_action must be one of `edit, add, rename, remove`".to_string());
    }
    if chunk.file_name_rename.is_some() && chunk.file_action != "rename" {
        return Err(format!("file_name_rename is not allowed for file_action `{}`. file_action must've been `rename`.", chunk.file_action));
    }
    Ok(())
}

pub async fn correct_and_validate_chunks(
    chunks: &mut Vec<DiffChunk>,
    global_context: Arc<ARwLock<GlobalContext>>
) -> Result<(), String> {
    for c in chunks.iter_mut() {
        let file_path = PathBuf::from(&c.file_name);
        if !file_path.is_file() && c.file_action == "edit" {
            let candidates = file_repair_candidates(&c.file_name, global_context.clone(), 5, false).await;
            let fuzzy_candidates = file_repair_candidates(&c.file_name, global_context.clone(), 5, true).await;

            if candidates.len() > 1 {
                return Err(format!("file_name `{}` is ambiguous.\nIt could be interpreted as:\n{}", &c.file_name, candidates.join("\n")));
            }
            if candidates.is_empty() {
                return if !fuzzy_candidates.is_empty() {
                    Err(format!("file_name `{}` is not found.\nHowever, there are similar paths:\n{}", &c.file_name, fuzzy_candidates.join("\n")))
                } else {
                    Err(format!("file_name `{}` is not found", &c.file_name))
                }
            }
            let candidate = candidates.get(0).unwrap();
            if !PathBuf::from(&candidate).is_file() {
                return Err(format!("file_name `{}` is not found.\nHowever, there are similar paths:\n{}", &c.file_name, fuzzy_candidates.join("\n")));
            }
            c.file_name = candidate.clone();
        }

        validate_chunk(c).map_err(|e| format!("error validating chunk {:?}:\n{}", c, e))?;
    }
    Ok(())
}

fn find_chunk_matches(chunk_lines_remove: &Vec<DiffLine>, orig_lines: &Vec<&DiffLine>) -> Result<Vec<Vec<usize>>, String> {
    let chunk_len = chunk_lines_remove.len();
    let orig_len = orig_lines.len();

    if chunk_len == 0 || orig_len < chunk_len {
        return Err("Invalid input: chunk_lines is empty or orig_lines is smaller than chunk_lines".to_string());
    }

    let mut matches = vec![];
    for i in 0..=(orig_len - chunk_len) {
        let mut match_found = true;

        for j in 0..chunk_len {
            if orig_lines[i + j].text != chunk_lines_remove[j].text {
                match_found = false;
                break;
            }
        }
        if match_found {
            let positions = (i..i + chunk_len).map(|index| orig_lines[index].line_n).collect::<Vec<usize>>();
            matches.push(positions);
        }
    }
    if matches.is_empty() {
        return Err("Chunk text not found in original text".to_string());
    }
    Ok(matches)
}

fn apply_chunk_to_text_fuzzy(
    chunk_id: usize,
    lines_orig: &Vec<DiffLine>,
    chunk: &DiffChunk,
    max_fuzzy_n: usize,
) -> (Vec<DiffLine>, ApplyDiffOutput) {
    let chunk_lines_remove: Vec<_> = chunk.lines_remove.lines().map(|l| DiffLine { line_n: 0, text: l.to_string(), overwritten_by_id: None}).collect();
    let chunk_lines_add: Vec<_> = chunk.lines_add.lines().map(|l| DiffLine { line_n: 0, text: l.to_string(), overwritten_by_id: Some(chunk_id)}).collect();
    let mut new_lines = vec![];

    if chunk_lines_remove.is_empty() {
        new_lines.extend(
            lines_orig
                .iter()
                .take_while(|l| l.line_n < chunk.line1 || l.overwritten_by_id.is_some())
                .cloned()
        );
        new_lines.extend(chunk_lines_add.iter().cloned());
        new_lines.extend(
            lines_orig
                .iter()
                .skip_while(|l| l.line_n < chunk.line1 || l.overwritten_by_id.is_some())
                .cloned()
        );
        return (new_lines, ApplyDiffOutput::Ok());
    }

    for fuzzy_n in 0..=max_fuzzy_n {
        let search_from = (chunk.line1 as i32 - fuzzy_n as i32).max(0) as usize;
        let search_till = (chunk.line2 as i32 - 1 + fuzzy_n as i32) as usize;
        let search_in_window: Vec<_> = lines_orig.iter()
            .filter(|l| l.overwritten_by_id.is_none() && l.line_n >= search_from && l.line_n <= search_till).collect();

        let matches = find_chunk_matches(&chunk_lines_remove, &search_in_window);

        let best_match = match matches {
            Ok(m) => {
                m[0].clone()
            },
            Err(_) => {
                if fuzzy_n >= max_fuzzy_n {
                    return (new_lines, ApplyDiffOutput::Err("no chunk in original text".to_string()));
                }
                continue;
            }
        };

        for l in lines_orig.iter() {
            if best_match.ends_with(&[l.line_n]) {
                new_lines.extend(chunk_lines_add.clone());
            }
            if !best_match.contains(&l.line_n) {
                new_lines.push(l.clone());
            }
        }
        break;
    }
    if new_lines.is_empty() {
        return (new_lines, ApplyDiffOutput::Err("error applying new lines".to_string()));
    }
    (new_lines, ApplyDiffOutput::Ok())
}

fn apply_chunks(
    chunks: Vec<(usize, &DiffChunk)>,
    file_text: &String,
    max_fuzzy_n: usize,
    line_ending: &str,
) -> (Vec<DiffLine>, HashMap<usize, ApplyDiffOutput>) {
    let mut lines_orig = file_text.split(line_ending).enumerate().map(|(line_n, l)| DiffLine { line_n: line_n + 1, text: l.to_string(), ..Default::default()}).collect::<Vec<_>>();

    let mut outputs = HashMap::new();
    for (chunk_id, chunk) in chunks.iter().map(|(id, c)|(*id, *c)) {
        let (lines_orig_new, out) = apply_chunk_to_text_fuzzy(chunk_id, &lines_orig, &chunk, max_fuzzy_n);
        if let ApplyDiffOutput::Ok() = out {
            lines_orig = lines_orig_new;
        }
        outputs.insert(chunk_id, out);
    }
    (lines_orig, outputs)
}

fn undo_chunks(
    chunks: Vec<(usize, &DiffChunk)>,
    file_text: &String,
    max_fuzzy_n: usize,
    line_ending: &str,
) -> (Vec<DiffLine>, HashMap<usize, ApplyDiffOutput>) {
    let mut lines_orig = file_text.split(line_ending).enumerate().map(|(line_n, l)| DiffLine { line_n: line_n + 1, text: l.to_string(), ..Default::default()}).collect::<Vec<_>>();

    let mut outputs = HashMap::new();
    
    for (chunk_id, chunk) in chunks.iter().map(|(id, c)|(*id, *c)) {
        let mut chunk_copy = chunk.clone();

        mem::swap(&mut chunk_copy.lines_remove, &mut chunk_copy.lines_add);
        chunk_copy.line2 = chunk_copy.line1 + chunk_copy.lines_remove.lines().count();

        let (mut lines_orig_new, output) = apply_chunk_to_text_fuzzy(chunk_id, &lines_orig, &chunk_copy, max_fuzzy_n);
        if output == ApplyDiffOutput::Ok() {
            lines_orig_new = lines_orig_new.iter_mut().enumerate().map(|(idx, l)| {
                l.line_n = idx + 1;
                return l.clone();
            }).collect::<Vec<_>>();
            lines_orig = lines_orig_new;
        }
        outputs.insert(chunk_id, output);
    }
    (lines_orig, outputs)
}

fn check_add(c: &DiffChunk) -> ApplyDiffOutput {
    let file_path = PathBuf::from(&c.file_name);
    if let Some(parent) = file_path.parent() {
        if !parent.is_dir() {
            return ApplyDiffOutput::Err(format!("cannot create a file: parent dir `{:?}` does not exist or is not a dir", &parent));
        }
        if file_path.exists() {
            return ApplyDiffOutput::Err(format!("cannot create a file: file `{}` already exists", &c.file_name));
        }
        return ApplyDiffOutput::Ok();
    } else {
        ApplyDiffOutput::Err(format!("cannot create a file: file `{}` doesn't have a parent (probably path is relative)", &c.file_name))
    }
}

fn check_remove(c: &DiffChunk) -> ApplyDiffOutput {
    let file_path = PathBuf::from(&c.file_name);
    if !file_path.is_file() {
        return ApplyDiffOutput::Err(format!("cannot remove file: file `{}` does not exist", &c.file_name));
    }
    ApplyDiffOutput::Ok()
}

fn check_rename(c: &DiffChunk) -> ApplyDiffOutput {
    let file_path = PathBuf::from(&c.file_name);
    let file_path_rename = PathBuf::from(c.file_name_rename.clone().unwrap_or_default());
    if let Some(parent) = file_path_rename.parent() {
        if !parent.is_dir() {
            return ApplyDiffOutput::Err(format!("cannot rename file: parent dir `{:?}` does not exist or is not a dir", &parent));
        }
        if !file_path.exists() {
            return ApplyDiffOutput::Err(format!("cannot rename file: file `{:?}` doesn't exist", &c.file_name_rename));
        }
        if file_path_rename.exists() {
            return ApplyDiffOutput::Err(format!("cannot rename file: file `{}` already exists", &c.file_name));
        }
        ApplyDiffOutput::Ok()
    } else {
        ApplyDiffOutput::Err(format!("cannot rename file: file `{}` doesn't have a parent (probably path is relative)", &c.file_name))
    }
}

pub fn apply_diff_chunks_to_text(
    file_text: &String,
    chunks_apply: Vec<(usize, &DiffChunk)>,
    chunks_undo: Vec<(usize, &DiffChunk)>,
    max_fuzzy_n: usize,
) -> (Vec<ApplyDiffResult>, HashMap<usize, ApplyDiffOutput>) {
    
    let mut results = vec![];
    let mut outputs = HashMap::new();

    let chunks_apply_edit = chunks_apply.iter().filter(|(_, c)|c.file_action == "edit").cloned().collect::<Vec<_>>();
    let chunks_undo_edit = chunks_undo.iter().filter(|(_, c)|c.file_action == "edit").cloned().collect::<Vec<_>>();
    
    let other_actions = vec!["add", "remove", "rename"];
    let chunks_apply_other = chunks_apply.iter().filter(|(_, c)|other_actions.contains(&c.file_action.as_str())).cloned().collect::<Vec<_>>();
    let chunks_undo_other = chunks_undo.iter().filter(|(_, c)|other_actions.contains(&c.file_action.as_str())).cloned().collect::<Vec<_>>();

    fn process_chunks_edit(
        chunks_apply_edit: Vec<(usize, &DiffChunk)>,
        chunks_undo_edit: Vec<(usize, &DiffChunk)>,
        file_text: &String,
        max_fuzzy_n: usize,
        results: &mut Vec<ApplyDiffResult>,
        outputs: &mut HashMap<usize, ApplyDiffOutput>,
    ) {
        let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut file_text_copy = file_text.clone();
        
        if chunks_apply_edit.is_empty() && chunks_undo_edit.is_empty() {
            return;
        }
        let file_names = chunks_undo_edit.iter().map(|c|c.1.file_name.clone()).chain(
            chunks_apply_edit.iter().map(|c|c.1.file_name.clone())
        ).collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
        
        let file_name_edit = match file_names.len() {
            1 => file_names[0].clone(),
            _ => {
                for id in chunks_apply_edit.iter().map(|c|c.0) {
                    outputs.insert(id, ApplyDiffOutput::Err("process_edit_chunks: cannot edit multiple files at once".to_string()));
                }
                return;
            }
        };

        if !chunks_undo_edit.is_empty() {
            let mut chunks_undo_copy = chunks_undo_edit.clone();
            chunks_undo_copy.sort_by_key(|c| c.0);
            let (new_lines, _) = undo_chunks(chunks_undo_copy, &file_text, max_fuzzy_n, line_ending); // XXX: only undo what is necessary
            file_text_copy = new_lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(line_ending);
        }

        if !chunks_apply_edit.is_empty() {
            let mut chunks_apply_copy = chunks_apply_edit.clone();
            chunks_apply_copy.sort_by_key(|c| c.0);
            let (new_lines, new_outputs) = apply_chunks(chunks_apply_copy, &file_text_copy, max_fuzzy_n, line_ending);
            outputs.extend(new_outputs);
            file_text_copy = new_lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(line_ending);
        }
        results.push(ApplyDiffResult {
            file_text: Some(file_text_copy),
            file_name_edit: Some(file_name_edit),
            ..Default::default()
        });
    }
    
    fn process_chunks_other(
        chunks_apply_other: Vec<(usize, &DiffChunk)>,
        chunks_undo_other: Vec<(usize, &DiffChunk)>,
        results: &mut Vec<ApplyDiffResult>,
        outputs: &mut HashMap<usize, ApplyDiffOutput>,
    ) {
        let undo_ids: HashSet<_> = chunks_undo_other.iter().map(|c| c.0).collect();
        let chunks_todo = chunks_apply_other
            .into_iter()
            .filter(|c| !undo_ids.contains(&c.0))
            .collect::<Vec<_>>();
        
        if DEBUG == 1 {
            info!("process_chunks_other starts");
            info!("chunks_undo_other_ids: {:?}", chunks_undo_other.iter().map(|c|c.0).collect::<Vec<_>>());
            info!("chunks_todo {:?}", chunks_todo);
        }
        for (c_idx, chunk) in chunks_todo {
            if DEBUG == 1 {
                info!("idx {} {:#?}", c_idx, chunk);
            }
            match chunk.file_action.as_str() {
                "add" => {
                    let out = check_add(chunk);
                    if out == ApplyDiffOutput::Ok() {
                        let res = ApplyDiffResult {
                            file_text: Some(chunk.lines_add.clone()),
                            file_name_add: Some(chunk.file_name.clone()),
                            ..Default::default()
                        };
                        if DEBUG == 1 {
                            info!("idx res {} {:#?}", c_idx, res);
                        }
                        results.push(res);
                    }
                    if DEBUG == 1 {
                        info!("idx {} {:#?}", c_idx, out);
                    }
                    outputs.insert(c_idx, out);
                },
                "remove" => {
                    let out = check_remove(chunk);
                    if out == ApplyDiffOutput::Ok() {
                        let res = ApplyDiffResult {
                            file_name_delete: Some(chunk.file_name.clone()),
                            ..Default::default()
                        };
                        if DEBUG == 1 {
                            info!("idx res {} {:#?}", c_idx, res);
                        }
                        results.push(res);
                    }
                    if DEBUG == 1 {
                        info!("idx {} {:#?}", c_idx, out);
                    }
                    outputs.insert(c_idx, out);
                },
                "rename" => {
                    let out = check_rename(chunk);
                    if out == ApplyDiffOutput::Ok() {
                        let res = ApplyDiffResult {
                            file_name_delete: Some(chunk.file_name_rename.clone().unwrap_or_default()),
                            file_name_add: Some(chunk.file_name.clone()),
                            ..Default::default()
                        };
                        if DEBUG == 1 {
                            info!("idx res {} {:#?}", c_idx, res);
                        }
                        results.push(res);
                    }
                    if DEBUG == 1 {
                        info!("idx {} {:#?}", c_idx, out);
                    }
                    outputs.insert(c_idx, out);
                },
                _ => continue,
            } 
        }
    }

    process_chunks_edit(chunks_apply_edit, chunks_undo_edit, file_text, max_fuzzy_n, &mut results, &mut outputs);
    process_chunks_other(chunks_apply_other, chunks_undo_other, &mut results, &mut outputs);
    
    (results, outputs)
}

pub fn read_files_n_apply_diff_chunks(
    chunks: &Vec<DiffChunk>,
    applied_state: &Vec<bool>,
    desired_state: &Vec<bool>,
    max_fuzzy_n: usize,
) -> (Vec<ApplyDiffResult>, HashMap<usize, ApplyDiffOutput>) {

    let mut results = vec![];
    let mut outputs = HashMap::new();

    let chunks_undo_edit = chunks.iter().enumerate().filter(|(idx, c)|applied_state.get(*idx) == Some(&true) && c.file_action == "edit").collect::<Vec<_>>();
    let chunks_apply_edit = chunks.iter().enumerate().filter(|(idx, c)|desired_state.get(*idx) == Some(&true) && c.file_action == "edit").collect::<Vec<_>>();

    let other_actions = vec!["add", "remove", "rename"];
    let chunks_undo_other = chunks.iter().enumerate().filter(|(idx, c)|applied_state.get(*idx) == Some(&true) && other_actions.contains(&c.file_action.as_str())).collect::<Vec<_>>();
    let chunks_apply_other = chunks.iter().enumerate().filter(|(idx, c)|desired_state.get(*idx) == Some(&true) && other_actions.contains(&c.file_action.as_str())).collect::<Vec<_>>();

    fn process_chunks_edit(
        chunks_apply_edit: Vec<(usize, &DiffChunk)>,
        chunks_undo_edit: Vec<(usize, &DiffChunk)>,
        max_fuzzy_n: usize,
        results: &mut Vec<ApplyDiffResult>,
        outputs: &mut HashMap<usize, ApplyDiffOutput>,
    ) {
        let mut chunk_apply_groups = HashMap::new();
        for c in chunks_apply_edit.iter().cloned() {
            chunk_apply_groups.entry(c.1.file_name.clone()).or_insert(Vec::new()).push(c);
        }
        let mut chunk_undo_groups = HashMap::new();
        for c in chunks_undo_edit.iter().cloned() {
            chunk_undo_groups.entry(c.1.file_name.clone()).or_insert(Vec::new()).push(c);
        }

        let file_names = chunk_apply_groups.keys().cloned().chain(chunk_undo_groups.keys().cloned()).collect::<HashSet<_>>();
        let mut apply_output = HashMap::new();

        for file_name in file_names {
            let chunks_apply = chunk_apply_groups.get(&file_name).unwrap_or(&vec![]).clone();
            let chunks_undo = chunk_undo_groups.get(&file_name).unwrap_or(&vec![]).clone();

            let file_text = match crate::files_in_workspace::read_file_from_disk_sync(&PathBuf::from(&file_name)) {
                Ok(t) => t.to_string(),
                Err(_) => {
                    for (c, _) in chunks_apply.iter() {
                        apply_output.insert(*c, ApplyDiffOutput::Err("Failed to read file".to_string()));
                    }
                    continue;
                }
            };

            let (new_results, new_outputs) = apply_diff_chunks_to_text(&file_text, chunks_apply, chunks_undo, max_fuzzy_n);
            results.extend(new_results);
            outputs.extend(new_outputs);
        }
    }
    fn process_chunks_other(
        chunks_apply_other: Vec<(usize, &DiffChunk)>,
        chunks_undo_other: Vec<(usize, &DiffChunk)>,
        results: &mut Vec<ApplyDiffResult>,
        outputs: &mut HashMap<usize, ApplyDiffOutput>,
    ) {
        let (new_results, new_outputs) = apply_diff_chunks_to_text(&"".to_string(), chunks_apply_other, chunks_undo_other, 0);
        results.extend(new_results);
        outputs.extend(new_outputs);
    }

    process_chunks_edit(chunks_apply_edit, chunks_undo_edit, max_fuzzy_n, &mut results, &mut outputs);
    process_chunks_other(chunks_apply_other, chunks_undo_other, &mut results, &mut outputs);

    (results, outputs)
}

pub fn unwrap_diff_apply_outputs(
    outputs: HashMap<usize, ApplyDiffOutput>, 
    chunks_default: Vec<DiffChunk>
) -> Vec<ApplyDiffUnwrapped> {
    let mut out_results = vec![];
    let other_actions = vec!["add", "remove", "rename"];

    for (chunk_id, c) in chunks_default.into_iter().enumerate() {
        if let Some(res) = outputs.get(&chunk_id) {
            if let ApplyDiffOutput::Ok() = res {
                let can_unapply = !other_actions.contains(&c.file_action.as_str());
                out_results.push(ApplyDiffUnwrapped {
                    chunk_id,
                    applied: true,
                    can_unapply,
                    success: true,
                    detail: None,
                });
            }
            else if let ApplyDiffOutput::Err(e) = res {
                out_results.push(ApplyDiffUnwrapped {
                    chunk_id,
                    applied: false,
                    can_unapply: false,
                    success: false,
                    detail: Some(e.clone()),
                });
            }
        } else {
            out_results.push(ApplyDiffUnwrapped {
                chunk_id,
                applied: false,
                can_unapply: false,
                success: true,
                detail: None,
            });
        }
    }
    out_results
}
