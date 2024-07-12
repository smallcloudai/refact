use std::mem;
use std::path::PathBuf;
use hashbrown::{HashMap, HashSet};
use crate::call_validation::DiffChunk;


#[derive(Clone, Debug, Default)]
struct DiffLine {
    line_n: usize,
    text: String,
    overwritten_by_id: Option<usize>,
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
) -> (Option<usize>, Vec<DiffLine>) {
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
        return (Some(0), new_lines);
    }
    
    let mut fuzzy_n_used = 0;
    for fuzzy_n in 0..=max_fuzzy_n {
        let search_from = (chunk.line1 as i32 - fuzzy_n as i32).max(0) as usize;
        let search_till = (chunk.line2 as i32 - 1 + fuzzy_n as i32) as usize;
        let search_in_window: Vec<_> = lines_orig.iter()
            .filter(|l| l.overwritten_by_id.is_none() && l.line_n >= search_from && l.line_n <= search_till).collect();

        let matches = find_chunk_matches(&chunk_lines_remove, &search_in_window);

        let best_match = match matches {
            Ok(m) => {
                fuzzy_n_used = fuzzy_n;
                m[0].clone()
            },
            Err(_) => {
                if fuzzy_n >= max_fuzzy_n {
                    return (None, new_lines);
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
        return (None, new_lines)
    }
    (Some(fuzzy_n_used), new_lines)
}

fn apply_chunks(
    chunks: Vec<(usize, &DiffChunk)>,
    file_text: &String,
    max_fuzzy_n: usize,
    line_ending: &str,
) -> (HashMap<usize, Option<usize>>, Vec<DiffLine>) {
    let mut lines_orig = file_text.split(line_ending).enumerate().map(|(line_n, l)| DiffLine { line_n: line_n + 1, text: l.to_string(), ..Default::default()}).collect::<Vec<_>>();

    let mut results_fuzzy_ns = HashMap::new();
    for (chunk_id, chunk) in chunks.iter().map(|(id, c)|(*id, *c)) {
        let (fuzzy_n_used, lines_orig_new) = apply_chunk_to_text_fuzzy(chunk_id, &lines_orig, &chunk, max_fuzzy_n);
        if fuzzy_n_used.is_some() {
            lines_orig = lines_orig_new;
        }
        results_fuzzy_ns.insert(chunk_id, fuzzy_n_used);
    }
    (results_fuzzy_ns, lines_orig)
}

fn undo_chunks(
    chunks: Vec<(usize, &DiffChunk)>,
    file_text: &String,
    max_fuzzy_n: usize,
    line_ending: &str,
) -> (HashMap<usize, Option<usize>>, Vec<DiffLine>) {
    let mut lines_orig = file_text.split(line_ending).enumerate().map(|(line_n, l)| DiffLine { line_n: line_n + 1, text: l.to_string(), ..Default::default()}).collect::<Vec<_>>();

    let mut results_fuzzy_ns = HashMap::new();
    for (chunk_id, chunk) in chunks.iter().map(|(id, c)|(*id, *c)) {
        let mut chunk_copy = chunk.clone();
        
        mem::swap(&mut chunk_copy.lines_remove, &mut chunk_copy.lines_add);
        chunk_copy.line2 = chunk_copy.line1 + chunk_copy.lines_remove.lines().count();

        let (fuzzy_n_used, mut lines_orig_new) = apply_chunk_to_text_fuzzy(chunk_id, &lines_orig, &chunk_copy, max_fuzzy_n);
        if fuzzy_n_used.is_some() {
            lines_orig_new = lines_orig_new.iter_mut().enumerate().map(|(idx, l)| {
                l.line_n = idx + 1;
                return l.clone();
            }).collect::<Vec<_>>();
            lines_orig = lines_orig_new;
        }
        results_fuzzy_ns.insert(chunk_id, fuzzy_n_used);
    }
    (results_fuzzy_ns, lines_orig)
}

pub fn apply_diff_chunks_to_text(
    file_text: &String,
    chunks_apply: Vec<(usize, &DiffChunk)>,
    chunks_undo: Vec<(usize, &DiffChunk)>,
    max_fuzzy_n: usize,
) -> (String, HashMap<usize, Option<usize>>) {
    let line_ending = if file_text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut file_text_copy = file_text.clone();
    let mut fuzzy_ns = HashMap::new();

    if !chunks_undo.is_empty() {
        let mut chunks_undo_copy = chunks_undo.clone();
        chunks_undo_copy.sort_by_key(|c| c.0);
        let (_, new_lines) = undo_chunks(chunks_undo_copy, &file_text, max_fuzzy_n, line_ending); // XXX: only undo what is necessary
        file_text_copy = new_lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(line_ending);
    }

    if !chunks_apply.is_empty() {
        let mut chunks_apply_copy = chunks_apply.clone();
        chunks_apply_copy.sort_by_key(|c| c.0);
        let (new_fuzzy_ns, new_lines) = apply_chunks(chunks_apply_copy, &file_text, max_fuzzy_n, line_ending);
        fuzzy_ns.extend(new_fuzzy_ns);
        file_text_copy = new_lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(line_ending);
    }
    (file_text_copy, fuzzy_ns)
}

pub fn read_files_n_apply_diff_chunks(
    chunks: &Vec<DiffChunk>,
    applied_state: &Vec<bool>,
    desired_state: &Vec<bool>,
    max_fuzzy_n: usize,
) -> (HashMap<String, String>, HashMap<usize, Option<usize>>) {

    let chunks_undo = chunks.iter().enumerate().filter(|(idx, _)|applied_state.get(*idx) == Some(&true)).collect::<Vec<_>>();
    let chunks_apply = chunks.iter().enumerate().filter(|(idx, _)|desired_state.get(*idx) == Some(&true)).collect::<Vec<_>>();
    
    let mut chunk_apply_groups = HashMap::new();
    for c in chunks_apply.iter().cloned() {
        chunk_apply_groups.entry(c.1.file_name.clone()).or_insert(Vec::new()).push(c);
    }
    let mut chunk_undo_groups = HashMap::new();
    for c in chunks_undo.iter().cloned() {
        chunk_undo_groups.entry(c.1.file_name.clone()).or_insert(Vec::new()).push(c);
    }

    let file_names = chunk_apply_groups.keys().cloned().chain(chunk_undo_groups.keys().cloned()).collect::<HashSet<_>>();
    let mut fuzzy_n_used = HashMap::new();
    let mut texts_after_patch = HashMap::new();

    for file_name in file_names {
        let chunks_apply = chunk_apply_groups.get(&file_name).unwrap_or(&vec![]).clone();
        let chunks_undo = chunk_undo_groups.get(&file_name).unwrap_or(&vec![]).clone();

        let file_text = match crate::files_in_workspace::read_file_from_disk_sync(&PathBuf::from(&file_name)) {
            Ok(t) => t.to_string(),
            Err(_) => { 
                for (c, _) in chunks_apply.iter() {
                    fuzzy_n_used.insert(*c, None);
                }
                continue; 
            }
        };

        let (new_text, fuzzy_ns) = apply_diff_chunks_to_text(&file_text, chunks_apply, chunks_undo, max_fuzzy_n);

        fuzzy_n_used.extend(fuzzy_ns);
        texts_after_patch.insert(file_name.clone(), new_text);
    }
    
    (texts_after_patch, fuzzy_n_used)
}

pub fn fuzzy_results_into_state_vector(results: &HashMap<usize, Option<usize>>, total: usize) -> Vec<usize> {
    let mut state_vector = vec![0; total];
    for (k, v) in results {
        if *k < total {
            state_vector[*k] = if v.is_some() { 1 } else { 2 };
        }
    }
    state_vector
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    const TEST_MAX_FUZZY: usize = 10;

    const FILE1_FN: &str = "/tmp/file1.txt";
    const FILE1: &str = r#"# line 1
class Point2d:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def __str__(self):
        return "Point2d(x=%0.2f, y=%0.2f)" % (self.x, self.y)
"#;
    const FILE2_FN: &str = "/tmp/file2.txt";
    const FILE2: &str = r#"    # Third jump
    frog1.jump()
    frog2.jump()"#;

    const FILE3_FN: &str = "/tmp/frog.py";
    const FILE3: &str = r#"import numpy as np

DT = 0.01

class Frog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)

    "#;

    fn delete_file_if_exists(file_name: &str) {
        if fs::metadata(file_name).is_ok() {
            fs::remove_file(file_name).expect("Failed to delete file");
        }
    }

    fn write_file(file_name: &str, content: &str) {
        let mut file = fs::File::create(file_name).expect("Failed to create file");
        file.write_all(content.as_bytes()).expect("Failed to write to file");
    }

    fn read_file(file_name: &str) -> String {
        fs::read_to_string(file_name).expect(&format!("Failed to read file: {}", file_name))
    }

    #[test]
    fn test_chunks() {
        // Run this to see println:
        //     cargo test diffs::tests::test_chunks -- --nocapture

        let chunk1 = DiffChunk {
            file_name: "/tmp/file1.txt".to_string(),
            file_action: "edit".to_string(),
            line1: 4,
            line2: 5,
            lines_remove: "        self.x = x\n        self.y = y\n".to_string(),
            lines_add: "        self.x, self.y = x, y\n".to_string(),
        };
        let chunks = vec![chunk1];

        let applied_state = vec![false];
        let desired_state = vec![true];

        delete_file_if_exists(FILE1_FN);
        let (_file_texts, results_fuzzy_n) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let r1_state = fuzzy_results_into_state_vector(&results_fuzzy_n, chunks.len());

        println!("r1 state: {:?}", r1_state);
        assert_eq!(vec![2], r1_state);

        write_file(FILE1_FN, FILE1);
        let (_file_texts, results_fuzzy_n) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let r2_state = fuzzy_results_into_state_vector(&results_fuzzy_n, chunks.len());

        println!("r2 state: {:?}", r2_state);
        assert_eq!(vec![1], r2_state);
    }
    #[test]
    fn test_frogs() {
        let c1 = DiffChunk {
            file_name: FILE2_FN.to_string(),
            file_action: "edit".to_string(),
            line1: 1,
            line2: 2,
            lines_remove: "    # Third jump\n".to_string(),
            lines_add: "    # Third extra jump\n".to_string(),
        };

        let c2 = DiffChunk {
            file_name: FILE2_FN.to_string(),
            file_action: "edit".to_string(),
            line1: 3,
            line2: 4,
            lines_remove: "    frog2.jump()\n".to_string(),
            lines_add: "    frog2.jump()\n    frog3.jump()\n".to_string(),
        };
        let chunks = vec![c1, c2];
        let applied_state = vec![false, false];
        let desired_state = vec![true, true];

        write_file(FILE2_FN, FILE2);
        let (_file_texts, results_fuzzy_n) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        println!("results_fuzzy_n: {:?}", results_fuzzy_n);
        let state = fuzzy_results_into_state_vector(&results_fuzzy_n, chunks.len());

        assert_eq!(vec![1, 1], state);
    }

    #[test]
    fn test_frog() {
        let file3_must_be: &str = r#"import numpy as np

DT = 0.01

class AnotherFrog:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        if self.x < 0:
            self.vx = np.abs(self.vx)
        elif self.x > pond_width:
            self.vx = -np.abs(self.vx)
        if self.y < 0:
            self.vy = np.abs(self.vy)
        elif self.y > pond_height:
            self.vy = -np.abs(self.vy)

    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)

    "#;

        let c1 = DiffChunk {
            file_name: FILE3_FN.to_string(),
            file_action: "edit".to_string(),
            line1: 5,
            line2: 6,
            lines_remove: "class Frog:\n".to_string(),
            lines_add: "class AnotherFrog:\n".to_string(),
        };
        let chunks = vec![c1];
        let applied_state = vec![false];
        let desired_state = vec![true];

        write_file(FILE3_FN, FILE3);
        let (file_texts, results_fuzzy_n) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        println!("results_fuzzy_n: {:?}", results_fuzzy_n);
        let state = fuzzy_results_into_state_vector(&results_fuzzy_n, chunks.len());
        assert_eq!(vec![1], state);
        let changed_text = file_texts.get(FILE3_FN).unwrap();
        assert_eq!(changed_text, file3_must_be);
        write_file(FILE3_FN, changed_text);

        let applied_state = vec![true];
        let desired_state = vec![false];
        let (file_texts, results_fuzzy_n) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        println!("results_fuzzy_n: {:?}", results_fuzzy_n);
        let state = fuzzy_results_into_state_vector(&results_fuzzy_n, chunks.len());
        assert_eq!(vec![0], state);
        let new_text = file_texts.get(FILE3_FN).unwrap();
        write_file(FILE3_FN, new_text);
        assert_eq!(read_file(FILE3_FN), FILE3);

        let (text, data) = apply_diff_chunks_to_text(&FILE3.to_string(), chunks.iter().enumerate().collect::<Vec<_>>(), vec![], 1);
        assert_eq!(file3_must_be, &text);
    }
}
