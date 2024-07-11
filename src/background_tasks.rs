use std::iter::IntoIterator;
use std::sync::Arc;
use std::vec;

use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;

use crate::ast::ast_module::AstModule;
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

    pub async fn abort(self) {
        for task in self.tasks {
            task.abort();
            let _ = task.await;
        }
    }
}

pub async fn start_background_tasks(gcx: Arc<ARwLock<GlobalContext>>) -> BackgroundTasksHolder {
    let mut bg = BackgroundTasksHolder::new(vec![
        tokio::spawn(crate::telemetry::basic_transmit::telemetry_background_task(gcx.clone())),
        tokio::spawn(crate::snippets_transmit::tele_snip_background_task(gcx.clone())),
        tokio::spawn(crate::vecdb::vdb_highlev::vecdb_background_reload(gcx.clone())),   // this in turn can create global_context::vec_db
    ]);
    let ast: Option<Arc<ARwLock<AstModule>>> = gcx.clone().read().await.ast_module.clone();
    if ast.is_some() {
        bg.extend(ast.unwrap().read().await.ast_start_background_tasks(gcx.clone()).await);
    }
    let files_jsonl_path = gcx.clone().read().await.cmdline.files_jsonl_path.clone();
    if !files_jsonl_path.is_empty() {
        bg.extend(vec![
            tokio::spawn(crate::files_in_jsonl::reload_if_jsonl_changes_background_task(gcx.clone()))
        ]);
    }
    bg
}
