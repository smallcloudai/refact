use std::sync::Arc;
use std::vec;

use tokio::sync::RwLock as ARwLock;
use tokio::task::JoinHandle;

use crate::{global_context, telemetry_storage};
use crate::global_context::GlobalContext;
use crate::snippets_transmit;

pub struct BackgroundTasksHolder {
    tasks: Vec<JoinHandle<()>>,
}

impl BackgroundTasksHolder {
    pub fn new(tasks: Vec<JoinHandle<()>>) -> Self {
        BackgroundTasksHolder {
            tasks
        }
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
        tokio::spawn(global_context::caps_background_reload(global_context.clone())),
        tokio::spawn(telemetry_storage::telemetry_background_task(global_context.clone())),
        tokio::spawn(snippets_transmit::tele_snip_background_task(global_context.clone()));
    ])
}
