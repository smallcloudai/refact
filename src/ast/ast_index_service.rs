use std::collections::{HashMap, VecDeque};
use std::iter::zip;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex as AMutex;
use tokio::task::JoinHandle;
use tracing::info;
use rayon::prelude::*;
use crate::ast::ast_index::AstIndex;
use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceArc;
use crate::files_in_workspace::DocumentInfo;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum EventType {
    Add,
    Reset,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AstEvent {
    pub docs: Vec<DocumentInfo>,
    pub typ: EventType,
}

impl AstEvent {
    pub fn add_docs(docs: Vec<DocumentInfo>) -> Self {
        AstEvent { docs, typ: EventType::Add }
    }

    pub fn reset() -> Self {
        AstEvent { docs: Vec::new(), typ: EventType::Reset }
    }
}

#[derive(Debug)]
pub struct AstIndexService {
    update_request_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    output_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    ast_index: Arc<AMutex<AstIndex>>,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    out_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    cooldown_secs: u64,
) {
    let mut last_updated: HashMap<AstEvent, SystemTime> = HashMap::new();
    loop {
        let (event_maybe, _unprocessed_files_count) = {
            let mut queue_locked = update_request_queue.lock().await;
            let queue_len = queue_locked.len();
            if !queue_locked.is_empty() {
                (Some(queue_locked.pop_front().unwrap()), queue_len)
            } else {
                (None, 0)
            }
        };

        if let Some(event) = event_maybe {
            last_updated.insert(event, SystemTime::now());
        }

        let mut events_to_process: Vec<AstEvent> = Vec::new();
        let mut stat_too_new = 0;
        let mut stat_proceed = 0;
        for (event, time) in &last_updated {
            if time.elapsed().unwrap().as_secs() > cooldown_secs {
                events_to_process.push(event.clone());
                stat_proceed += 1;
            } else {
                stat_too_new += 1;
            }
        }
        if stat_proceed > 0 || stat_too_new > 0 {
            info!("{} events to process, {} events too new", stat_proceed, stat_too_new);
        }
        for event in events_to_process {
            last_updated.remove(&event);
            out_queue.lock().await.push_back(event);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}


async fn ast_indexer_thread(
    queue: Arc<AMutex<VecDeque<AstEvent>>>,
    ast_index: Arc<AMutex<AstIndex>>,
) {
    loop {
        let events = {
            let mut queue_locked = queue.lock().await;
            let events: Vec<AstEvent> = Vec::from(queue_locked.to_owned());
            queue_locked.clear();
            events
        };

        if events.len() == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }

for event in events {
            let list_of_path = event.docs;
            match event.typ {
                EventType::Add => {
                    let ast_index = ast_index.clone();
                    let all_symbols: Vec<Result<Vec<AstSymbolInstanceArc>, String>> = list_of_path
            .par_iter()
            .map(move |document| AstIndex::parse(&document))
            .collect();

                    let mut ast_index = ast_index.lock().await;
                    zip(list_of_path, all_symbols).for_each(|(doc, res)| {
                        match res {
                            Ok(symbols) => {
                                match ast_index.add_or_update_symbols_index(&doc, &symbols) {
                                    Ok(_) => {}
                                    Err(e) => { info!("Error adding/updating records in AST index: {}", e); }
                                }
                            }
                            Err(e) => { info!("Error adding/updating records in AST index: {}", e); }
                        }
                    })
                }
                EventType::Reset => {
                    ast_index.lock().await.clear_index();
                    info!("Reset AST Index");
                }
            }
        }
    }
}

async fn ast_indexer(
    update_request_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    out_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    ast_index: Arc<AMutex<AstIndex>>,
) {
    loop {
        let mut q_len = update_request_queue.lock().await.len();
        q_len += out_queue.lock().await.len();

        if q_len > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            continue;
        }

        {
            let mut ast_index = ast_index.lock().await;
            ast_index.rebuild_index();
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

const COOLDOWN_SECS: u64 = 2;

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
        let rebuild_index_handle = tokio::spawn(
            ast_indexer(
                self.update_request_queue.clone(),
                self.output_queue.clone(),
                self.ast_index.clone(),
            )
        );
        return vec![cooldown_queue_join_handle, indexer_handle, rebuild_index_handle];
    }

    pub async fn ast_indexer_enqueue_files(&self, event: AstEvent, force: bool) {
        info!("adding to indexer queue {} events", event.docs.len());
        if !force {
            self.update_request_queue.lock().await.push_back(event);
        } else {
            self.output_queue.lock().await.push_back(event);
        }
    }
}
