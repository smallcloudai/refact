use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::iter::zip;
use std::ops::Div;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::Mutex as AMutex;
use tokio::task::JoinHandle;
use tracing::info;
use rayon::prelude::*;
use crate::ast::ast_index::AstIndex;
use crate::ast::treesitter::structs::{SymbolDeclarationStruct, UsageSymbolInfo};
use crate::files_in_workspace::DocumentInfo;

#[derive(Debug)]
pub struct AstIndexService {
    update_request_queue: Arc<AMutex<VecDeque<DocumentInfo>>>,
    output_queue: Arc<AMutex<VecDeque<DocumentInfo>>>,
    ast_index: Arc<AMutex<AstIndex>>,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<DocumentInfo>>>,
    out_queue: Arc<AMutex<VecDeque<DocumentInfo>>>,
    cooldown_secs: u64,
) {
    let mut last_updated: HashMap<DocumentInfo, SystemTime> = HashMap::new();
    loop {
        let (path_maybe, _unprocessed_files_count) = {
            let mut queue_locked = update_request_queue.lock().await;
            let queue_len = queue_locked.len();
            if !queue_locked.is_empty() {
                (Some(queue_locked.pop_front().unwrap()), queue_len)
            } else {
                (None, 0)
            }
        };

        if let Some(path) = path_maybe {
            last_updated.insert(path, SystemTime::now());
        }

        let mut paths_to_process: Vec<DocumentInfo> = Vec::new();
        let mut stat_too_new = 0;
        let mut stat_proceed = 0;
        for (doc, time) in &last_updated {
            if time.elapsed().unwrap().as_secs() > cooldown_secs {
                paths_to_process.push(doc.clone());
                stat_proceed += 1;
            } else {
                stat_too_new += 1;
            }
        }
        if stat_proceed > 0 || stat_too_new > 0 {
            info!("{} files to process, {} files too new", stat_proceed, stat_too_new);
        }
        for path in paths_to_process {
            last_updated.remove(&path);
            out_queue.lock().await.push_back(path);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}



async fn ast_indexer_thread(
    queue: Arc<AMutex<VecDeque<DocumentInfo>>>,
    ast_index: Arc<AMutex<AstIndex>>,
) {
    let mut reported_unprocessed: usize = 0;
    let mut reported_astindex_complete: bool = false;

    loop {
        let (list_of_path, unprocessed_files_count) = {
            let mut queue_locked = queue.lock().await;
            let docs: Vec<DocumentInfo> = Vec::from(queue_locked.to_owned());
            let queue_len = docs.len();
            queue_locked.clear();
            (docs, queue_len)
            
        };
        if (unprocessed_files_count + 99).div(100) != (reported_unprocessed + 99).div(100) {
            info!("have {} unprocessed files", unprocessed_files_count);
            reported_unprocessed = unprocessed_files_count;
        }
        reported_astindex_complete &= unprocessed_files_count == 0;
        if list_of_path.is_empty() {
            if !reported_astindex_complete {
                reported_astindex_complete = true;
                write!(std::io::stderr(), "AST COMPLETED\n").unwrap();
                info!("AST COMPLETED");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
        
        
        let ast_index = ast_index.clone();
        let declarations_and_usages: Vec<Result<(HashMap<String, SymbolDeclarationStruct>, Vec<Box<dyn UsageSymbolInfo>>), String>> 
            = list_of_path.par_iter().map(move |document| {
            AstIndex::get_declarations_and_usages(&document)
        }).collect();

        let mut ast_index = ast_index.lock().await;
        zip(list_of_path, declarations_and_usages).for_each(|(doc, res)| {
            match res {
                Ok((declaration, usages)) => {
                    match ast_index.add_or_update_declarations_and_usages(&doc, declaration, usages) {
                        Ok(_) => {}
                        Err(e) => { info!("Error adding/updating records in AST index: {}", e);}
                    }
                }
                Err(e) => { info!("Error adding/updating records in AST index: {}", e);}
            }
        })
    }
}

const COOLDOWN_SECS: u64 = 5;

impl AstIndexService {
    pub fn init(
        ast_index: Arc<AMutex<AstIndex>>
    ) -> Self {
        let update_request_queue = Arc::new(AMutex::new(VecDeque::new()));
        let output_queue = Arc::new(AMutex::new(VecDeque::new()));
        AstIndexService {
            update_request_queue: update_request_queue.clone(),
            output_queue: output_queue.clone(),
            ast_index: ast_index.clone(),
        }
    }

    pub async fn ast_start_background_tasks(&mut self) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                COOLDOWN_SECS,
            )
        );
        let indexer_handle = tokio::spawn(
            ast_indexer_thread(
                self.output_queue.clone(),
                self.ast_index.clone(),
            )
        );
        return vec![cooldown_queue_join_handle, indexer_handle];
    }

    pub async fn ast_indexer_enqueue_files(&self, documents: &Vec<DocumentInfo>, force: bool) {
        info!("adding to indexer queue {} files", documents.len());
        if !force {
            self.update_request_queue.lock().await.extend(documents.clone());
        } else {
            self.output_queue.lock().await.extend(documents.clone());
        }
    }
}
