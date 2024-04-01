use std::collections::{HashMap, VecDeque};
use std::iter::zip;
use std::sync::Arc;
use std::time::SystemTime;

use rayon::prelude::*;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;

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
    is_busy: Arc<AMutex<bool>>,
    ast_index: Arc<ARwLock<AstIndex>>,
}

async fn cooldown_queue_thread(
    update_request_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    out_queue: Arc<AMutex<VecDeque<AstEvent>>>,
    cooldown_secs: u64,
) {
    let mut last_updated: HashMap<AstEvent, SystemTime> = HashMap::new();
    loop {
        let mut events: Vec<AstEvent> = Vec::new();
        {
            let mut queue_locked = update_request_queue.lock().await;
            for _ in 0..queue_locked.len() {
                if let Some(e) = queue_locked.pop_front() {
                    events.push(e);
                }
            }
        };
        for doc in events {
            last_updated.insert(doc, SystemTime::now());
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
    ast_index: Arc<ARwLock<AstIndex>>,
    is_busy_flag: Arc<AMutex<bool>>
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
            *is_busy_flag.lock().await = false;
            continue;
        } else {
            *is_busy_flag.lock().await = true;
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

                    for (doc, res) in zip(list_of_path, all_symbols) {
                        match res {
                            Ok(symbols) => {
                                match ast_index.write().await.add_or_update_symbols_index(&doc, &symbols) {
                                    Ok(_) => {}
                                    Err(e) => { info!("Error adding/updating records in AST index: {}", e); }
                                }
                            }
                            Err(e) => { info!("Error adding/updating records in AST index: {}", e); }
                        }
                    }
                }
                EventType::Reset => {
                    ast_index.write().await.clear_index();
                    info!("Reset AST Index");
                }
            }
        }
    }
}

async fn ast_indexer(
    is_busy_flag: Arc<AMutex<bool>>,
    ast_index: Arc<ARwLock<AstIndex>>,
) {
    loop {
        if *is_busy_flag.lock().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        {
            if !ast_index.read().await.need_update() {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }

            let symbols = ast_index.read().await
                .symbols_by_guid()
                .values()
                .cloned()
                .collect::<Vec<_>>();
            info!("Building ast declarations");
            let t0 = std::time::Instant::now();
            ast_index.read().await.resolve_types(&symbols);
            info!("Building ast declarations finished, took {:.3}s", t0.elapsed().as_secs_f64());

            info!("Merging usages and declarations");
            let t1 = std::time::Instant::now();
            ast_index.read().await.merge_usages_to_declarations(&symbols);
            info!("Merging usages and declarations finished, took {:.3}s", t1.elapsed().as_secs_f64());

            info!("Creating extra indexes");
            let t2 = std::time::Instant::now();
            {
                let mut ast_index_ref = ast_index.write().await;
                ast_index_ref.create_extra_indexes(&symbols);
                ast_index_ref.set_updated();
            }
            info!("Creating extra indexes finished, took {:.3}s", t2.elapsed().as_secs_f64());
        }
    }
}

const COOLDOWN_SECS: u64 = 2;

impl AstIndexService {
    pub fn init(
        ast_index: Arc<ARwLock<AstIndex>>
    ) -> Self {
        let update_request_queue = Arc::new(AMutex::new(VecDeque::new()));
        let output_queue = Arc::new(AMutex::new(VecDeque::new()));
        AstIndexService {
            update_request_queue: update_request_queue.clone(),
            output_queue: output_queue.clone(),
            is_busy: Arc::new(AMutex::new(false)),
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
                self.is_busy.clone(),
            )
        );
        let rebuild_index_handle = tokio::spawn(
            ast_indexer(
                self.is_busy.clone(),
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

