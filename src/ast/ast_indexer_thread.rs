use indexmap::{IndexSet, IndexMap};
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex as AMutex, Notify as ANotify};
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;
use tracing::info;
use crate::files_in_workspace::Document;
use crate::global_context::GlobalContext;

use crate::ast::ast_minimalistic::{AstDB, AstStatus, AstCounters, ErrorStats};
use crate::ast::ast_db::{ast_index_init, fetch_counters, doc_add, doc_remove, flush_sled_batch, ConnectUsageContext, connect_usages, connect_usages_look_if_full_reset_needed};


pub struct AstIndexService {
    pub ast_index: Arc<AMutex<AstDB>>,
    pub ast_status: Arc<AMutex<AstStatus>>,
    pub ast_sleeping_point: Arc<ANotify>,
    pub ast_todo: IndexSet<String>,
}

async fn ast_indexer_thread(
    gcx_weak: Weak<ARwLock<GlobalContext>>,
    ast_service: Arc<AMutex<AstIndexService>>,
) {
    let mut reported_idle = true;
    let mut stats_parsed_cnt = 0;
    let mut stats_symbols_cnt = 0;
    let mut stats_t0 = std::time::Instant::now();
    let mut stats_update_ts = std::time::Instant::now() - std::time::Duration::from_millis(1000);
    let mut stats_failure_reasons: IndexMap<String, usize> = IndexMap::new();
    let mut stats_success_languages: IndexMap<String, usize> = IndexMap::new();
    let mut stats_parsing_errors = ErrorStats::default();
    let mut ast_max_files_hit = false;
    let (ast_index, ast_status, ast_sleeping_point) = {
        let ast_service_locked = ast_service.lock().await;
        (
            ast_service_locked.ast_index.clone(),
            ast_service_locked.ast_status.clone(),
            ast_service_locked.ast_sleeping_point.clone(),
        )
    };
    let ast_max_files = ast_index.lock().await.ast_max_files;  // cannot change

    loop {
        let (cpath, left_todo_count) = {
            let mut ast_service_locked = ast_service.lock().await;
            let mut cpath;
            let mut left_todo_count;
            loop {
                cpath = ast_service_locked.ast_todo.pop();
                left_todo_count = ast_service_locked.ast_todo.len();
                if left_todo_count < ast_max_files {
                    break;
                }
                ast_max_files_hit = true;
            }
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

                    match doc_add(ast_index.clone(), &cpath, &file_text, &mut stats_parsing_errors).await {
                        Ok((defs, language)) => {
                            let elapsed = start_time.elapsed().as_secs_f32();
                            if elapsed > 0.1 {
                                tracing::info!("{}/{} doc_add {:.3?}s {}", stats_parsed_cnt, (stats_parsed_cnt+left_todo_count), elapsed, crate::nicer_logs::last_n_chars(&cpath, 40));
                            }
                            stats_parsed_cnt += 1;
                            stats_symbols_cnt += defs.len();
                            *stats_success_languages.entry(language).or_insert(0) += 1;
                        }
                        Err(reason) => {
                            *stats_failure_reasons.entry(reason).or_insert(0) += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::info!("cannot read file {}: {}", crate::nicer_logs::last_n_chars(&cpath, 30), e);
                }
            }

            if stats_update_ts.elapsed() >= std::time::Duration::from_millis(1000) { // can't be lower, because flush_sled_batch() happens not very often at all
                let counters: AstCounters = fetch_counters(ast_index.clone()).await;
                {
                    let mut status_locked = ast_status.lock().await;
                    status_locked.files_unparsed = left_todo_count;
                    status_locked.files_total = stats_parsed_cnt;
                    status_locked.ast_index_files_total = counters.counter_defs;
                    status_locked.ast_index_symbols_total = counters.counter_usages;
                    status_locked.ast_max_files_hit = ast_max_files_hit;
                    status_locked.astate = "indexing".to_string();
                    status_locked.astate_notify.notify_waiters();
                }
                stats_update_ts = std::time::Instant::now();
            }

            continue;
        }

        let mut todo_count = ast_service.lock().await.ast_todo.len();
        if todo_count > 0 {
            continue;
        }

        flush_sled_batch(ast_index.clone(), 0).await;  // otherwise bad stats

        if !reported_idle {
            if !stats_parsing_errors.errors.is_empty() {
                let error_count = stats_parsing_errors.errors_counter;
                let display_count = std::cmp::min(5, error_count);
                let mut error_messages = String::new();
                for error in &stats_parsing_errors.errors[..display_count] {
                    error_messages.push_str(&format!("(E) {}:{} {}\n", error.err_cpath, error.err_line, error.err_message));
                }
                if error_count > 5 {
                    error_messages.push_str(&format!("...and {} more", error_count - 5));
                }
                info!("parsing errors, this would be a mixture of real code problems and our language-specific parser problems:\n{}", error_messages);
                stats_parsing_errors = ErrorStats::default();
            }
            info!("AST finished parsing, got {} symbols by processing {} files in {:>.3}s",
                stats_symbols_cnt,
                stats_parsed_cnt,
                stats_t0.elapsed().as_secs_f64()
            );
            let language_stats: String = if stats_success_languages.is_empty() {
                "no files".to_string()
            } else {
                stats_success_languages.iter()
                    .map(|(lang, count)| format!("{:>30} {}", lang, count))
                    .collect::<Vec<String>>()
                    .join("\n")
            };
            let problem_stats: String = if stats_failure_reasons.is_empty() {
                "no errors".to_string()
            } else {
                stats_failure_reasons.iter()
                    .map(|(reason, count)| format!("{:>30} {}", reason, count))
                    .collect::<Vec<String>>()
                    .join("\n")
            };
            let sum_of_successes = stats_success_languages.values().sum::<usize>();
            let sum_of_errors = stats_failure_reasons.values().sum::<usize>();
            if sum_of_errors > 0 {
                info!("error stats:\n{}", problem_stats);
            }
            if sum_of_successes > 1 {
                info!("language stats:\n{}", language_stats);
            }
            stats_success_languages.clear();
            stats_failure_reasons.clear();
            stats_parsed_cnt = 0;
            stats_symbols_cnt = 0;
            reported_idle = true;
            let counters: AstCounters = fetch_counters(ast_index.clone()).await;
            {
                let mut status_locked = ast_status.lock().await;
                status_locked.files_unparsed = 0;
                status_locked.files_total = 0;
                status_locked.ast_index_files_total = counters.counter_docs;
                status_locked.ast_index_symbols_total = counters.counter_defs;
                status_locked.ast_max_files_hit = ast_max_files_hit;
                status_locked.astate = "done".to_string();
            }
            ast_sleeping_point.notify_waiters();
        }

        // Connect usages, unless we have files in the todo
        let mut usagecx: ConnectUsageContext = connect_usages_look_if_full_reset_needed(ast_index.clone()).await;
        loop {
            todo_count = ast_service.lock().await.ast_todo.len();
            if todo_count > 0 {
                break;
            }
            let did_anything = connect_usages(ast_index.clone(), &mut usagecx).await;
            if !did_anything {
                break;
            }
        }

        flush_sled_batch(ast_index.clone(), 0).await;

        if !usagecx.errstats.errors.is_empty() {
            let error_count = usagecx.errstats.errors_counter;
            let display_count = std::cmp::min(5, error_count);
            let mut error_messages = String::new();
            for error in &usagecx.errstats.errors[..display_count] {
                error_messages.push_str(&format!("(U) {}:{} {}\n", error.err_cpath, error.err_line, error.err_message));
            }
            if error_count > 5 {
                error_messages.push_str(&format!("...and {} more", error_count - 5));
            }
            info!("AST connection graph errors:\n{}", error_messages);
        }
        if usagecx.usages_connected + usagecx.usages_not_found + usagecx.usages_ambiguous + usagecx.usages_homeless > 0 {
            info!("AST connection graph stats: homeless={}, connected={}, not_found={}, ambiguous={} in {:.3}s",
                usagecx.usages_homeless,
                usagecx.usages_connected,
                usagecx.usages_not_found,
                usagecx.usages_ambiguous,
                usagecx.t0.elapsed().as_secs_f32()
            );
        }

        if todo_count > 0 {
            info!("stopped processing links because there's a file to parse");
            continue;
        }

        tokio::time::timeout(tokio::time::Duration::from_secs(10), ast_sleeping_point.notified()).await.ok();
    }
}

pub async fn ast_indexer_block_until_finished(ast_service: Arc<AMutex<AstIndexService>>, max_blocking_time_ms: usize, wake_up_indexer: bool)
{
    let max_blocking_duration = tokio::time::Duration::from_millis(max_blocking_time_ms as u64);
    let start_time = std::time::Instant::now();
    let ast_sleeping_point = {
        let ast_service_locked = ast_service.lock().await;
        ast_service_locked.ast_sleeping_point.clone()
    };
    let mut wake_up_indexer = wake_up_indexer;

    loop {
        let future: tokio::sync::futures::Notified = ast_sleeping_point.notified();
        if wake_up_indexer {
            ast_sleeping_point.notify_waiters();
            wake_up_indexer = false;
        }
        {
            let ast_service_locked = ast_service.lock().await;
            let ast_status_locked = ast_service_locked.ast_status.lock().await;
            if ast_status_locked.astate == "done" || start_time.elapsed() >= max_blocking_duration {
                break;
            }
        }
        let remaining_time = max_blocking_duration
            .checked_sub(start_time.elapsed())
            .unwrap_or_else(|| tokio::time::Duration::from_millis(0));
        let sleep_duration = remaining_time
            .checked_add(tokio::time::Duration::from_millis(50))
            .unwrap_or_else(|| tokio::time::Duration::from_millis(50))
            .max(tokio::time::Duration::from_millis(1));
        tokio::select! {
            _ = future => {},
            _ = tokio::time::sleep(sleep_duration) => {},
        }
    }
}

pub async fn ast_service_init(ast_permanent: String, ast_max_files: usize) -> Arc<AMutex<AstIndexService>>
{
    let ast_index = ast_index_init(ast_permanent, ast_max_files, false).await;
    let ast_status = Arc::new(AMutex::new(AstStatus {
        astate_notify: Arc::new(ANotify::new()),
        astate: String::from("starting"),
        files_unparsed: 0,
        files_total: 0,
        ast_index_files_total: 0,
        ast_index_symbols_total: 0,
        ast_max_files_hit: false
    }));
    let ast_service = AstIndexService {
        ast_sleeping_point: Arc::new(ANotify::new()),
        ast_index,
        ast_status,
        ast_todo: IndexSet::new(),
    };
    Arc::new(AMutex::new(ast_service))
}

pub async fn ast_indexer_start(
    ast_service: Arc<AMutex<AstIndexService>>,
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<JoinHandle<()>>
{
    let indexer_handle = tokio::spawn(
        ast_indexer_thread(
            Arc::downgrade(&gcx),
            ast_service.clone(),
        )
    );
    return vec![indexer_handle];
}

pub async fn ast_indexer_enqueue_files(ast_service: Arc<AMutex<AstIndexService>>, cpaths: Vec<String>, wake_up_indexer: bool)
{
    let ast_status;
    {
        let mut ast_service_locked = ast_service.lock().await;
        ast_status = ast_service_locked.ast_status.clone();
        for cpath in cpaths {
            ast_service_locked.ast_todo.insert(cpath);
        }
    }
    {
        let mut status_locked = ast_status.lock().await;
        status_locked.astate = "indexing".to_string();
        status_locked.astate_notify.notify_waiters();
    }
    if wake_up_indexer {
        let ast_service_locked = ast_service.lock().await;
        ast_service_locked.ast_sleeping_point.notify_waiters();
    }
}
