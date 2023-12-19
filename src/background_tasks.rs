use std::iter::IntoIterator;
use std::sync::Arc;
use std::vec;

use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;

use crate::{global_context, vecdb};
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

pub fn start_background_tasks(global_context: Arc<ARwLock<GlobalContext>>) -> BackgroundTasksHolder {
    BackgroundTasksHolder::new(vec![
        tokio::spawn(basic_transmit::telemetry_background_task(global_context.clone())),
        tokio::spawn(snippets_transmit::tele_snip_background_task(global_context.clone())),
        tokio::spawn(vecdb::vecdb::vecdb_background_reload(global_context.clone())),
    ])
}
