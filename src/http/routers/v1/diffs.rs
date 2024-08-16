use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use tracing::warn;

use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock as ARwLock;

use crate::call_validation::DiffChunk;
use crate::custom_error::ScratchError;
use crate::diffs::{unwrap_diff_apply_outputs, correct_and_validate_chunks, ApplyDiffResult, read_files_n_apply_diff_chunks};
use crate::files_in_workspace::{Document, read_file_from_disk};
use crate::global_context::GlobalContext;
use crate::vecdb::vdb_highlev::memories_block_until_vectorized;
use crate::vecdb::vdb_thread::vectorizer_enqueue_files;


const MAX_FUZZY_N: usize = 10;


#[derive(Deserialize)]
pub struct DiffPost {
    pub apply: Vec<bool>,
    pub chunks: Vec<DiffChunk>,
    #[serde(skip_serializing, default)]
    pub id: u64
}

impl DiffPost {
    pub fn set_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.chunks.hash(&mut hasher);
        self.id = hasher.finish();
    }
}

async fn write_results_on_disk(results: Vec<ApplyDiffResult>) -> Result<Vec<Document>, String> {
    async fn write_to_file(path: &String, text: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .await
            .map_err(|e| {
                format!("Failed to open file: {}", e)
            })?;

        file.write_all(text.as_bytes()).await.map_err(|e| {
            format!("Failed to write to file: {}", e)
        })?;
        Ok(())
    }

    fn apply_add_action(file_path: &String, file_text: &String) -> Result<(), String> {
        fs::write(&PathBuf::from(file_path), file_text).map_err(|e| {
            eprintln!("Failed to write file: {}", e);
            format!("Failed to write file: {}", e)
        })
    }

    fn apply_remove_action(file_path: &String) -> Result<(), String> {
        let path = PathBuf::from(file_path);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                eprintln!("Failed to remove file: {}", e);
                format!("Failed to remove file: {}", e)
            })
        } else {
            Err(format!("File `{}` does not exist", &file_path))
        }
    }

    fn apply_rename_action(file_path_rename: &String, file_path: &String) -> Result<(), String> {
        if PathBuf::from(file_path).exists() {
            return Err(format!("File `{}` already exists", &file_path_rename));
        }
        if PathBuf::from(file_path_rename).exists() {
            fs::rename(&file_path_rename, &file_path).map_err(|e| {
                eprintln!("Failed to rename file: {}", e);
                format!("Failed to rename file: {}", e)
            })
        } else {
            Err(format!("File `{}` does not exist", &file_path_rename))
        }
    }

    let mut docs2index = vec![];

    for r in results {
        if r.file_name_edit.is_some() && r.file_text.is_some() {
            write_to_file(&r.file_name_edit.clone().unwrap(), &r.file_text.clone().unwrap()).await?;
            let mut doc = Document::new(&PathBuf::from(&r.file_name_edit.unwrap()));
            doc.update_text(&r.file_text.unwrap());
            docs2index.push(doc);
        }
        else if r.file_name_delete.is_some() && r.file_name_add.is_some() {
            apply_rename_action(&r.file_name_delete.unwrap(), &r.file_name_add.clone().unwrap())?;
            let mut doc = Document::new(&PathBuf::from(&r.file_name_add.unwrap()));
            let text = read_file_from_disk(&doc.path).await?.to_string();
            doc.update_text(&text);
            docs2index.push(doc);
        }
        else if r.file_name_add.is_some() && r.file_text.is_some() {
            apply_add_action(&r.file_name_add.clone().unwrap(), &r.file_text.clone().unwrap())?;
            let mut doc = Document::new(&PathBuf::from(&r.file_name_add.unwrap()));
            doc.update_text(&r.file_text.unwrap());
            docs2index.push(doc);
        }
        else if r.file_name_delete.is_some() {
            apply_remove_action(&r.file_name_delete.unwrap())?;
        }
    }
    Ok(docs2index)
}

fn validate_post(post: &DiffPost) -> Result<(), ScratchError> {
    if post.chunks.is_empty() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` shouldn't be empty".to_string()));
    }
    if post.chunks.len() != post.apply.len() {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "`chunks` and `apply` arrays are not of the same length".to_string()));
    }
    Ok(())
}

async fn sync_documents_ast_vecdb(global_context: Arc<ARwLock<GlobalContext>>, docs: Vec<Document>) -> Result<(), ScratchError> {
    if let Some(ast) = global_context.write().await.ast_module.clone() {
        let mut ast_write = ast.write().await;
        for doc in docs.iter() {
            let _ = ast_write.ast_add_file_no_queue(doc, true).await.map_err(|e| {
                let e_text = format!("Failed to sync doc {:?} with AST. Error:\n{}", doc.path, e);
                warn!(e_text);
            });
        }
    }

    let vecdb_enqueued = if let Some(vservice) = {
        let cx = global_context.write().await;
        let vec_db_guard = cx.vec_db.lock().await;
        vec_db_guard.as_ref().map(|v| v.vectorizer_service.clone())
    } {
        vectorizer_enqueue_files(vservice, &docs, true).await;
        true
    } else {
        false
    };

    if vecdb_enqueued {
        let vecdb = global_context.write().await.vec_db.clone();
        memories_block_until_vectorized(vecdb).await.map_err(|e|
            ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
        )?;
    }

    Ok(())
}

pub async fn handle_v1_diff_apply(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    validate_post(&post)?;
    correct_and_validate_chunks(&mut post.chunks, global_context.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).map(|x| x.clone()).unwrap_or_default()
    };
    let desired_state = post.apply.clone();
    let (results, outputs) = read_files_n_apply_diff_chunks(&post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);

    let docs2index = write_results_on_disk(results.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    sync_documents_ast_vecdb(global_context.clone(), docs2index).await?;

    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, post.chunks);

    global_context.write().await.documents_state.diffs_applied_state.insert(post.id, outputs_unwrapped.iter().map(|x|x.applied == true).collect::<Vec<_>>());

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&outputs_unwrapped).unwrap()))
        .unwrap())
}

#[derive(Deserialize)]
struct DiffStatePost {
    pub chunks: Vec<DiffChunk>,
    #[serde(skip_serializing, default)]
    pub id: u64,
}

impl DiffStatePost {
    pub fn set_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.chunks.hash(&mut hasher);
        self.id = hasher.finish();
    }
}

#[derive(Serialize)]
struct DiffStateResponse {
    id: u64,
    state: Vec<bool>,
    can_apply: Vec<bool>,
}

pub async fn handle_v1_diff_state(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let mut post = serde_json::from_slice::<DiffStatePost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    post.set_id();

    correct_and_validate_chunks(&mut post.chunks, global_context.clone()).await.map_err(|e|ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let applied_state = {
        let diff_state = global_context.read().await.documents_state.diffs_applied_state.clone();
        diff_state.get(&post.id).cloned().unwrap_or_default()
    };
    let desired_state = vec![true; post.chunks.len()];

    let (_, outputs) = read_files_n_apply_diff_chunks(&post.chunks, &applied_state, &desired_state, MAX_FUZZY_N);
    let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, post.chunks);

    let can_apply = outputs_unwrapped.iter().map(|x|x.applied).collect::<Vec<_>>();

    let response = DiffStateResponse {
        id: post.id,
        state: applied_state,
        can_apply,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use crate::diffs::apply_diff_chunks_to_text;

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
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1: 4,
            line2: 5,
            lines_remove: "        self.x = x\n        self.y = y\n".to_string(),
            lines_add: "        self.x, self.y = x, y\n".to_string(),
            ..Default::default()
        };
        let chunks = vec![chunk1];

        let applied_state = vec![false];
        let desired_state = vec![true];

        delete_file_if_exists(FILE1_FN);
        let (_, outputs) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());

        assert_eq!(outputs_unwrapped[0].success, true);

        write_file(FILE1_FN, FILE1);
        let (_, outputs) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());

        assert_eq!(outputs_unwrapped[0].success, true);
        assert_eq!(outputs_unwrapped[0].applied, true);
    }

    #[test]
    fn test_frogs() {
        let c1 = DiffChunk {
            file_name: FILE2_FN.to_string(),
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1: 1,
            line2: 2,
            lines_remove: "    # Third jump\n".to_string(),
            lines_add: "    # Third extra jump\n".to_string(),
            ..Default::default()
        };

        let c2 = DiffChunk {
            file_name: FILE2_FN.to_string(),
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1: 3,
            line2: 4,
            lines_remove: "    frog2.jump()\n".to_string(),
            lines_add: "    frog2.jump()\n    frog3.jump()\n".to_string(),
            ..Default::default()
        };
        let chunks = vec![c1, c2];
        let applied_state = vec![false, false];
        let desired_state = vec![true, true];

        write_file(FILE2_FN, FILE2);
        let (_, outputs) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());

        assert_eq!(outputs_unwrapped.iter().map(|x|x.applied).collect::<Vec<_>>(), vec![true, true]);
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
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1: 5,
            line2: 6,
            lines_remove: "class Frog:\n".to_string(),
            lines_add: "class AnotherFrog:\n".to_string(),
            ..Default::default()
        };

        let c2 = DiffChunk {
            file_name: FILE3_FN.to_string(),
            file_name_rename: None,
            file_action: "edit".to_string(),
            line1: 4,
            line2: 4,
            lines_remove: "".to_string(),
            lines_add: "\n\n".to_string(),
            ..Default::default()
        };
        let chunks = vec![c1, c2];
        let applied_state = vec![false, false];
        let desired_state = vec![true, true];

        write_file(FILE3_FN, FILE3);
        let (results, outputs) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());

        assert_eq!(outputs_unwrapped.into_iter().map(|x|x.applied).collect::<Vec<_>>(), vec![true, true]);

        let changed_text = results[0].clone().file_text.unwrap();

        assert_eq!(changed_text.as_str(), file3_must_be);
        write_file(FILE3_FN, changed_text.as_str());

        let applied_state = vec![true, true];
        let desired_state = vec![false, false];
        let (results, outputs) = read_files_n_apply_diff_chunks(&chunks, &applied_state, &desired_state, TEST_MAX_FUZZY);
        let outputs_unwrapped = unwrap_diff_apply_outputs(outputs, chunks.clone());

        assert_eq!(outputs_unwrapped.iter().map(|x|x.applied).collect::<Vec<bool>>(), vec![false, false]);
        assert_eq!(outputs_unwrapped.iter().map(|x|x.success).collect::<Vec<bool>>(), vec![true, true]);

        let new_text = results[0].clone().file_text.unwrap();
        write_file(FILE3_FN, &new_text);
        assert_eq!(read_file(FILE3_FN), FILE3);

        let (results, _) = apply_diff_chunks_to_text(&FILE3.to_string(), chunks.iter().enumerate().collect::<Vec<_>>(), vec![], 1);
        let new_text = results[0].clone().file_text.unwrap();
        assert_eq!(file3_must_be, new_text.as_str());
    }
}
