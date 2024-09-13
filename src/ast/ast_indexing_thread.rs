use indexmap::IndexSet;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex as AMutex, Notify as ANotify};
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

use crate::ast::alt_minimalistic::{AstDB, AstStatus, AstCounters};
use crate::ast::alt_db::{alt_index_init, fetch_counters, doc_add, doc_remove};


pub struct AstIndexService {
    pub ast_index: Arc<AMutex<AstDB>>,
    pub alt_status: Arc<AMutex<AstStatus>>,
    pub ast_sleeping_point: Arc<ANotify>,
    pub ast_todo: IndexSet<String>,
}

async fn ast_indexing_thread(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    ast_service: Arc<AMutex<AstIndexService>>,
) {
    let mut reported_idle = true;
    let mut stats_parsed_cnt = 0;
    let mut stats_symbols_cnt = 0;
    let mut stats_t0 = std::time::Instant::now();
    let mut stats_update_ts = std::time::Instant::now() - std::time::Duration::from_millis(200);
    let (ast_index, alt_status, ast_sleeping_point) = {
        let ast_service_locked = ast_service.lock().await;
        (
            ast_service_locked.ast_index.clone(),
            ast_service_locked.alt_status.clone(),
            ast_service_locked.ast_sleeping_point.clone(),
        )
    };
    loop {
        let (cpath, left_todo_count) = {
            let mut ast_service_locked = ast_service.lock().await;
            let cpath = ast_service_locked.ast_todo.shift_remove_index(0);
            let left_todo_count = ast_service_locked.ast_todo.len();
            (cpath, left_todo_count)
        };
        if let Some(cpath) = cpath {
            reported_idle = false;
            if stats_parsed_cnt == 0 {
                stats_t0 = std::time::Instant::now();
            }
            let gcx = match gcx_weak.upgrade() {
                Some(x) => x,
                None => {
                    info!("detected program shutdown, quit");
                    break;
                }
            };
            let mut doc = Document { doc_path: cpath.clone().into(), doc_text: None };

            doc_remove(ast_index.clone(), &cpath).await;

            match crate::files_in_workspace::get_file_text_from_memory_or_disk(gcx.clone(), &doc.doc_path).await {
                Ok(file_text) => {
                    doc.update_text(&file_text);
                    let start_time = std::time::Instant::now();

                    let defs = doc_add(ast_index.clone(), &cpath, &file_text).await;

                    tracing::info!("doc_add {:.3?}s {}", start_time.elapsed().as_secs_f32(), crate::nicer_logs::last_n_chars(&cpath, 30));
                    stats_parsed_cnt += 1;
                    stats_symbols_cnt += defs.len();
                    stats_parsed_cnt += 1;
                    stats_symbols_cnt += defs.len();
                }
                Err(e) => {
                    tracing::info!("cannot read file {}: {}", crate::nicer_logs::last_n_chars(&cpath, 30), e);
                }
            }

            if stats_update_ts.elapsed() >= std::time::Duration::from_millis(200) {
                let counters: AstCounters = fetch_counters(ast_index.clone()).await;
                {
                    let mut status_locked = alt_status.lock().await;
                    status_locked.files_unparsed = left_todo_count;
                    status_locked.files_total = stats_parsed_cnt;
                    status_locked.ast_index_files_total = counters.counter_defs;
                    status_locked.ast_index_symbols_total = counters.counter_usages;
                    status_locked.astate = "parsing".to_string();
                    status_locked.astate_notify.notify_one();
                }
                stats_update_ts = std::time::Instant::now();
            }

            continue;
        }

        if !reported_idle {
            info!("finished parsing, got {} symbols by processing {} files in {:>.3}s",
                stats_symbols_cnt,
                stats_parsed_cnt,
                stats_t0.elapsed().as_secs_f64()
            );
            stats_parsed_cnt = 0;
            stats_symbols_cnt = 0;
            reported_idle = true;
            let counters: AstCounters = fetch_counters(ast_index.clone()).await;
            {
                let mut status_locked = alt_status.lock().await;
                status_locked.files_unparsed = 0;
                status_locked.files_total = 0;
                status_locked.ast_index_files_total = counters.counter_defs;
                status_locked.ast_index_symbols_total = counters.counter_usages;
                status_locked.astate = "idle".to_string();
            }
            ast_sleeping_point.notify_one();
        }

        tokio::time::timeout(tokio::time::Duration::from_secs(10), ast_sleeping_point.notified()).await.ok();
    }
}


pub async fn ast_service_init() -> Arc<AMutex<AstIndexService>> {
    let ast_index = alt_index_init().await;
    let alt_status = Arc::new(AMutex::new(AstStatus {
        astate_notify: Arc::new(ANotify::new()),
        astate: String::from("starting"),
        files_unparsed: 0,
        files_total: 0,
        ast_index_files_total: 0,
        ast_index_symbols_total: 0,
    }));
    let ast_service = AstIndexService {
        ast_sleeping_point: Arc::new(ANotify::new()),
        ast_index,
        alt_status,
        ast_todo: IndexSet::new(),
    };
    Arc::new(AMutex::new(ast_service))
}

pub async fn ast_start_background_tasks(
    ast_service: Arc<AMutex<AstIndexService>>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<JoinHandle<()>>
{
    let indexer_handle = tokio::spawn(
        ast_indexing_thread(
            Arc::downgrade(&gcx),
            ast_service.clone(),
        )
    );
    return vec![indexer_handle];
}

pub async fn ast_indexer_enqueue_files(ast_service: Arc<AMutex<AstIndexService>>, cpaths: Vec<String>, wake_up_indexer: bool)
{
    let mut ast_service_locked = ast_service.lock().await;
    for cpath in cpaths {
        ast_service_locked.ast_todo.insert(cpath);
    }
    if wake_up_indexer {
        ast_service_locked.ast_sleeping_point.notify_one();
    }
}

pub async fn ast_indexer_block_until_finished(ast_service: Arc<AMutex<AstIndexService>>)
{
    let _x = ast_service;
}

