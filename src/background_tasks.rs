use std::iter::IntoIterator;
use std::sync::Arc;
use std::vec;

use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;

use crate::vecdb;
use crate::global_context::GlobalContext;
use crate::snippets_transmit;
use crate::telemetry::basic_transmit;

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

    pub async fn abort(self) {
        for task in self.tasks {
            task.abort();
            let _ = task.await;
        }
    }
}

pub async fn start_background_tasks(gcx: Arc<ARwLock<GlobalContext>>) -> BackgroundTasksHolder {
    let mut bg = BackgroundTasksHolder::new(vec![
        tokio::spawn(basic_transmit::telemetry_background_task(gcx.clone())),
        tokio::spawn(snippets_transmit::tele_snip_background_task(gcx.clone())),
        tokio::spawn(vecdb::vecdb::vecdb_background_reload(gcx.clone())),   // this in turn can create global_context::vec_db
    ]);
    match *gcx.clone().read().await.ast_module.lock().await {
        Some(ref ast) => bg.extend(ast.ast_start_background_tasks().await),
        None => ()
    };
    
    let files_jsonl_path = gcx.clone().read().await.cmdline.files_jsonl_path.clone();
    if !files_jsonl_path.is_empty() {
        bg.extend(vec![
            tokio::spawn(crate::files_in_jsonl::reload_if_jsonl_changes_background_task(gcx.clone()))
        ]);
    }
    bg
}
