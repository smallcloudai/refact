use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::{Arc, Weak};
use std::time::{SystemTime, Duration};

use tokio::sync::{Mutex as AMutex, Notify};
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;

use crate::global_context::GlobalContext;
use crate::ast::ast_index::AstIndex;
use crate::files_in_workspace::Document;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AstEventType {
    Add,
    AstReset,
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
    ast_hold_off_indexes_rebuild_notify: Arc<Notify>,
    ast_index: Arc<ARwLock<AstIndex>>,
}

use std::path::PathBuf;

async fn cooldown_queue_thread(
    ast_delayed_requests_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    ast_immediate_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    cooldown_secs: u64,
) {
    let mut latest_events: HashMap<PathBuf, Arc<AstEvent>> = HashMap::new();
    loop {
        let mut have_reset: bool = false;
        {
            let mut queue_locked = ast_delayed_requests_q.lock().await;
            while let Some(e) = queue_locked.pop_front() {
                if e.typ == AstEventType::AstReset {
                    have_reset = true;
                    latest_events.clear();
                    break;
                }
                for doc in e.docs.iter() {
                    latest_events.insert(doc.path.clone(), e.clone());
                }
            }
        }

        let now = SystemTime::now();
        if have_reset {
            let mut q = ast_immediate_q.lock().await;
            q.clear();
            q.push_back(Arc::new(AstEvent { docs: Vec::new(), typ: AstEventType::AstReset, posted_ts: now }));
            continue;
        }

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


async fn ast_indexer_thread(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    ast_immediate_q: Arc<AMutex<VecDeque<Arc<AstEvent>>>>,
    ast_index: Arc<ARwLock<AstIndex>>,
    ast_hold_off_indexes_rebuild_notify: Arc<Notify>,
) {
    let mut reported_stats = false;
    let mut stats_parsed_cnt = 0;    // by language?
    let mut stats_symbols_cnt = 0;
    let mut stats_t0 = std::time::Instant::now();
    let mut hold_on_after_reset = false;
    loop {
        let mut events = {
            let mut q = ast_immediate_q.lock().await;
            let events: Vec<Arc<AstEvent>> = Vec::from(q.to_owned());
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

        let mut unparsed_suffixes = HashMap::new();
        for event in events.iter_mut() {
            let gcx = match gcx_weak.upgrade() {
                Some(x) => x,
                None => {
                    info!("detected program shutdown, quit");
                    break;
                }
            };
            let mut docs_with_text: Vec<Document> = Vec::new();
            for doc in event.docs.iter() {
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
            match event.typ {
                AstEventType::Add => {
                    for doc in docs_with_text.iter() {
                        match ast_index.write().await.add_or_update(&doc, true) {
                            Ok(len) => {
                                stats_symbols_cnt += len;
                            }
                            Err(e) => {
                                *unparsed_suffixes.entry(e).or_insert(0) += 1;
                            }
                        }
                    }
                }
                AstEventType::AstReset => {
                    info!("Reset AST Index");
                    ast_index.write().await.clear_index();
                    hold_on_after_reset = true;
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
    ast_index: Arc<ARwLock<AstIndex>>,
) {
    loop {
        ast_hold_off_indexes_rebuild_notify.notified().await;

        if !ast_index.read().await.needs_update() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let ast_index_clone = Arc::clone(&ast_index);
        tokio::task::spawn_blocking(move || {
            let mut ast = ast_index_clone.blocking_write();
            ast.reindex();
        }).await.expect("cannot reindex")
    }
}

const COOLDOWN_SECS: u64 = 2;

impl AstIndexService {
    pub fn init(
        ast_index: Arc<ARwLock<AstIndex>>
    ) -> Self {
        let ast_delayed_requests_q = Arc::new(AMutex::new(VecDeque::new()));
        let ast_immediate_q = Arc::new(AMutex::new(VecDeque::new()));
        AstIndexService {
            ast_delayed_requests_q: ast_delayed_requests_q.clone(),
            ast_immediate_q: ast_immediate_q.clone(),
            ast_hold_off_indexes_rebuild_notify: Arc::new(Notify::new()),
            ast_index: ast_index.clone(),
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
            )
        );
        let rebuild_index_handle = tokio::spawn(
            ast_index_rebuild_thread(
                self.ast_hold_off_indexes_rebuild_notify.clone(),
                self.ast_index.clone(),
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

