use std::iter::IntoIterator;
use std::path::PathBuf;
use std::sync::Arc;
use std::vec;
use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;

use crate::global_context::GlobalContext;


pub struct BackgroundTasksHolder {
    tasks: Vec<JoinHandle<()>>,
}

impl BackgroundTasksHolder {
    pub fn new(tasks: Vec<JoinHandle<()>>) -> Self {
        BackgroundTasksHolder {
            tasks
        }
    }

    pub fn push_back(&mut self, task: JoinHandle<()>) {
        self.tasks.push(task);
    }

    pub fn extend<T>(&mut self, tasks: T)
        where
            T: IntoIterator<Item=JoinHandle<()>>,
    {
        self.tasks.extend(tasks);
    }

    pub async fn abort(&mut self) {
        for task in self.tasks.iter_mut() {
            task.abort();
            let _ = task.await;
        }
        self.tasks.clear();
    }
}

pub async fn start_background_tasks(gcx: Arc<ARwLock<GlobalContext>>, config_dir: &PathBuf) -> BackgroundTasksHolder {
    let mut bg = BackgroundTasksHolder::new(vec![
        tokio::spawn(crate::files_in_workspace::files_in_workspace_init_task(gcx.clone())),
        tokio::spawn(crate::telemetry::basic_transmit::telemetry_background_task(gcx.clone())),
        tokio::spawn(crate::snippets_transmit::tele_snip_background_task(gcx.clone())),
        tokio::spawn(crate::vecdb::vdb_highlev::vecdb_background_reload(gcx.clone())),   // this in turn can create global_context::vec_db
        tokio::spawn(crate::integrations::sessions::remove_expired_sessions_background_task(gcx.clone())),
        tokio::spawn(crate::memories::memories_migration(gcx.clone(), config_dir.clone())),
        tokio::spawn(crate::git::cleanup::git_shadow_cleanup_background_task(gcx.clone())),
        tokio::spawn(crate::cloud::threads_sub::watch_threads_subscription(gcx.clone())),
    ]);
    let ast = gcx.clone().read().await.ast_service.clone();
    if let Some(ast_service) = ast {
        bg.extend(crate::ast::ast_indexer_thread::ast_indexer_start(ast_service, gcx.clone()).await);
    }
    let files_jsonl_path = gcx.clone().read().await.cmdline.files_jsonl_path.clone();
    if !files_jsonl_path.is_empty() {
        bg.extend(vec![
            tokio::spawn(crate::files_in_jsonl::reload_if_jsonl_changes_background_task(gcx.clone()))
        ]);
    }
    bg
}
