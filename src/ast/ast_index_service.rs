use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::zip;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;
use tokio::sync::{Mutex as AMutex, Notify};
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;

use crate::ast::ast_index::AstIndex;
use crate::ast::ast_module::AstIndexStatus;
use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceArc;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AstEventType {
    Add,
    AstReset,
    AddDummy,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AstEvent {
    pub docs: Vec<Document>,
    pub typ: AstEventType,
    pub posted_ts: SystemTime,
}

impl AstEvent {
    pub fn add_docs(docs: Vec<Document>) -> Self {
        AstEvent { docs, typ: AstEventType::Add, posted_ts: SystemTime::now() }
    }
}

#[derive(Debug)]
pub struct AstIndexService {
    ast_delayed_requests_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    ast_immediate_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    pub ast_hold_off_indexes_rebuild_notify: Arc<Notify>,
    ast_index: Arc<AMutex<AstIndex>>,
    status: Arc<AMutex<AstIndexStatus>>
}

async fn cooldown_queue_thread(
    ast_delayed_requests_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    ast_immediate_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    cooldown_secs: u64,
) {
    let mut latest_events: HashMap<PathBuf, Arc<AstEvent>> = HashMap::new();
    loop {
        // let mut have_service_events: bool = false;
        {
            let mut queue_locked = ast_delayed_requests_q.lock().await;
            while let Some(e) = queue_locked.pop_front() {
                match e.typ {
                    AstEventType::Add => {
                        for doc in e.docs.iter() {
                            latest_events.insert(doc.path.clone(), e.clone());
                        }
                    }
                    AstEventType::AstReset => {
                        // have_service_events = true;
                        latest_events = latest_events
                            .into_iter()
                            .filter(|(_, e)| e.typ != AstEventType::Add)
                            .collect::<HashMap<_, _>>();
                        let mut q = ast_immediate_q.lock().await;
                        q.clear();
                        q.push_back(e);
                        break;
                    }
                    AstEventType::AddDummy => {
                        ast_immediate_q.lock().await.push_back(e);
                        // have_service_events = true;
                        break;
                    }
                }
            }
        }

        let now = SystemTime::now();
        let mut paths_to_launch = HashSet::new();
        for (_path, original_event) in latest_events.iter() {
            if original_event.posted_ts + Duration::from_secs(cooldown_secs) < now {  // old enough
                for doc in original_event.docs.iter() {
                    paths_to_launch.insert(doc.path.clone());
                }
            }
            if paths_to_launch.len() >= 32 {
                break;
            }
        }

        if paths_to_launch.len() > 0 {
            info!("cooldown see {} files on stack, launch parse for {} of them", latest_events.len(), paths_to_launch.len());
            let mut launch_event = AstEvent { docs: Vec::new(), typ: AstEventType::Add, posted_ts: now };
            for path in paths_to_launch {
                latest_events.remove(&path);
                launch_event.docs.push(Document { path: path.clone(), text: None });
            }
            ast_immediate_q.lock().await.push_back(Arc::new(launch_event));
            continue;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

fn pop_back_n<T>(data: &mut VecDeque<T>, n: usize) -> Vec<T> {
    let mut output = Vec::with_capacity(n);
    for _ in 0..n {
        match data.pop_back() {
            Some(item) => {
                output.push(item)
            }
            None => {
                break;
            }
        }
    }
    output
}


async fn ast_indexer_thread(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    ast_immediate_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    ast_index: Arc<AMutex<AstIndex>>,
    ast_hold_off_indexes_rebuild_notify: Arc<Notify>,
    status: Arc<AMutex<AstIndexStatus>>,
) {
    let mut reported_stats = true;
    let mut stats_parsed_cnt = 0;    // by language?
    let mut stats_symbols_cnt = 0;
    let mut stats_t0 = std::time::Instant::now();
    let mut hold_on_after_reset = false;
    loop {
        let mut events = {
            let mut q = ast_immediate_q.lock().await;
            let events: VecDeque<Arc<AstEvent>> = VecDeque::from(q.to_owned());
            q.clear();
            events
        };

        if events.is_empty() {
            if hold_on_after_reset {
                // hold on, don't report anything, don't say this thread isn't busy.
                // after reset, real data will follow, now sleep and do nothing.
            } else if !reported_stats {
                info!("finished parsing, got {} symbols by processing {} files in {:>.3}s",
                    stats_symbols_cnt,
                    stats_parsed_cnt,
                    stats_t0.elapsed().as_secs_f64()
                );
                stats_parsed_cnt = 0;
                stats_symbols_cnt = 0;
                reported_stats = true;
                let (ast_index_files_total, ast_index_symbols_total) = {
                    let ast_ref = ast_index.lock().await;
                    (ast_ref.total_files(), ast_ref.total_symbols())
                };
                {
                    let mut locked_status = status.lock().await;
                    locked_status.files_unparsed = 0;
                    locked_status.files_total = 0;
                    locked_status.ast_index_files_total = ast_index_files_total;
                    locked_status.ast_index_symbols_total = ast_index_symbols_total;
                    locked_status.state = "idle".to_string();
                }
                ast_hold_off_indexes_rebuild_notify.notify_one();
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        } else {
            hold_on_after_reset = false;
            reported_stats = false;
            if stats_parsed_cnt == 0 {
                stats_t0 = std::time::Instant::now();
            }
        }

        let files_total = events.iter().map(|e| e.docs.len()).sum();
        let mut unparsed_suffixes = HashMap::new();
        while !events.is_empty() {
            let processing_events = pop_back_n(&mut events, 8);
            if processing_events.is_empty() {
                break;
            };
            let left_docs_count: usize = events.iter().map(|e| e.docs.len()).sum();
            let (ast_index_files_total, ast_index_symbols_total) = {
                let ast_ref = ast_index.lock().await;
                (ast_ref.total_files(), ast_ref.total_symbols())
            };
            {
                let mut locked_status = status.lock().await;
                locked_status.files_unparsed = left_docs_count;
                locked_status.files_total = files_total;
                locked_status.ast_index_files_total = ast_index_files_total;
                locked_status.ast_index_symbols_total = ast_index_symbols_total;
                locked_status.state = "parsing".to_string();
            }
            let gcx = match gcx_weak.upgrade() {
                Some(x) => x,
                None => {
                    info!("detected program shutdown, quit");
                    break;
                }
            };


            let is_ast_full = ast_index.lock().await.is_overflowed();
            let mut docs_with_text: Vec<Document> = Vec::new();
            for doc in processing_events.iter().flat_map(|x| x.docs.iter()) {
                if !is_ast_full {
                    match crate::files_in_workspace::get_file_text_from_memory_or_disk(gcx.clone(), &doc.path).await {
                        Ok(file_text) => {
                            stats_parsed_cnt += 1;
                            let mut doc_copy = doc.clone();
                            doc_copy.update_text(&file_text);
                            docs_with_text.push(doc_copy);
                        }
                        Err(e) => {
                            tracing::warn!("cannot read file {}: {}", crate::nicer_logs::last_n_chars(&doc.path.display().to_string(), 30), e);
                            continue;
                        }
                    }
                }
            }

            if !docs_with_text.is_empty() {
                let symbols: Vec<Result<Vec<AstSymbolInstanceArc>, String>> = docs_with_text
                    .par_iter()
                    .map(move |doc| {
                        if !is_ast_full {
                            AstIndex::parse(&doc)
                        } else {
                            Ok(vec![])
                        }
                    })
                    .collect();

                for (doc, res) in zip(docs_with_text, symbols) {
                    match res {
                        Ok(symbols) => {
                            stats_symbols_cnt += symbols.len();
                            match ast_index.lock().await.add_or_update_symbols_index(&doc, symbols, true) {
                                Ok(_) => {}
                                Err(e) => {
                                    *unparsed_suffixes.entry(e).or_insert(0) += 1;
                                }
                            }
                        }
                        Err(e) => {
                            *unparsed_suffixes.entry(e).or_insert(0) += 1;
                        }
                    }
                }
            }
            for event in processing_events
                .iter()
                .filter(|x| x.typ != AstEventType::Add) {
                match event.typ {
                    AstEventType::AddDummy => {
                        info!("No files was added to AST Index");
                        hold_on_after_reset = false;
                    }
                    AstEventType::AstReset => {
                        info!("Reset AST Index");
                        ast_index.lock().await.clear_index();
                        hold_on_after_reset = true;
                    }
                    _ => {}
                }
            }
        }

        if !unparsed_suffixes.is_empty() {
            info!("AST didn't parse these files, even though they were passed in input queue:\n{:#?}", unparsed_suffixes);
        }
    }
}

async fn ast_index_rebuild_thread(
    ast_hold_off_indexes_rebuild_notify: Arc<Notify>,
    ast_index: Arc<AMutex<AstIndex>>,
    status: Arc<AMutex<AstIndexStatus>>,
) {
    loop {
        ast_hold_off_indexes_rebuild_notify.notified().await;

        if !ast_index.lock().await.needs_update() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let (ast_index_files_total, ast_index_symbols_total) = {
            let ast_ref = ast_index.lock().await;
            (ast_ref.total_files(), ast_ref.total_symbols())
        };
        {
            let mut locked_status = status.lock().await;
            locked_status.files_unparsed = 0;
            locked_status.files_total = 0;
            locked_status.ast_index_files_total = ast_index_files_total;
            locked_status.ast_index_symbols_total = ast_index_symbols_total;
            locked_status.state = "indexing".to_string();
        }
        let ast_index_clone = Arc::clone(&ast_index);
        tokio::task::spawn_blocking(move || {
            let mut ast = ast_index_clone.blocking_lock();
            ast.reindex();
        }).await.expect("cannot reindex");
        {
            let mut locked_status = status.lock().await;
            locked_status.state = "done".to_string();
        }
    }
}

const COOLDOWN_SECS: u64 = 2;

impl AstIndexService {
    pub fn init(
        ast_index: Arc<AMutex<AstIndex>>,
        status: Arc<AMutex<AstIndexStatus>>,
    ) -> Self {
        let ast_delayed_requests_q = Arc::new(AMutex::new(VecDeque::new()));
        let ast_immediate_q = Arc::new(AMutex::new(VecDeque::new()));
        AstIndexService {
            ast_delayed_requests_q: ast_delayed_requests_q.clone(),
            ast_immediate_q: ast_immediate_q.clone(),
            ast_hold_off_indexes_rebuild_notify: Arc::new(Notify::new()),
            ast_index,
            status
        }
    }

    pub async fn ast_start_background_tasks(
        &mut self,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) -> Vec<JoinHandle<()>> {
        let cooldown_queue_join_handle = tokio::spawn(
            cooldown_queue_thread(
                self.ast_delayed_requests_q.clone(),
                self.ast_immediate_q.clone(),
                COOLDOWN_SECS,
            )
        );
        let indexer_handle = tokio::spawn(
            ast_indexer_thread(
                Arc::downgrade(&gcx),
                self.ast_immediate_q.clone(),
                self.ast_index.clone(),
                self.ast_hold_off_indexes_rebuild_notify.clone(),
                self.status.clone(),
            )
        );
        let rebuild_index_handle = tokio::spawn(
            ast_index_rebuild_thread(
                self.ast_hold_off_indexes_rebuild_notify.clone(),
                self.ast_index.clone(),
                self.status.clone(),

            )
        );
        return vec![cooldown_queue_join_handle, indexer_handle, rebuild_index_handle];
    }

    pub async fn ast_indexer_enqueue_files(&self, event: AstEvent, force: bool)
    {
        if event.typ == AstEventType::AstReset {
            info!("adding to indexer a reset instruction, force={}", force as i32);
        } else {
            info!("adding to indexer queue an event with {} documents, force={}", event.docs.len(), force as i32);
        }
        if !force {
            self.ast_delayed_requests_q.lock().await.push_back(Arc::new(event));
        } else {
            self.ast_immediate_q.lock().await.push_back(Arc::new(event));
        }
    }
}

