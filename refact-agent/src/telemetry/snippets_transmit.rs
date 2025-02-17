use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::global_context;


const SNIP_NOT_ACCEPTED_TIMEOUT_AFTER : i64 = 30;
const SNIP_ACCEPTED_NOT_FINISHED_TIMEOUT_AFTER: i64 = 600;


pub async fn send_finished_snippets(gcx: Arc<ARwLock<global_context::GlobalContext>>) {
    let tele_storage;
    let now = chrono::Local::now().timestamp();
    {
        let cx = gcx.read().await;
        tele_storage = cx.telemetry.clone();
    }

    {
        let mut to_remove: Vec<usize> = vec![];
        let mut storage_locked = tele_storage.write().unwrap();
        for (idx, snip) in &mut storage_locked.tele_snippets.iter().enumerate() {
            if snip.accepted_ts != 0 {
                if snip.finished_ts != 0 {
                    to_remove.push(idx);
                } else if snip.created_ts + SNIP_ACCEPTED_NOT_FINISHED_TIMEOUT_AFTER < now {
                    to_remove.push(idx)
                }
                continue;
            }
            if snip.accepted_ts == 0 && snip.created_ts + SNIP_NOT_ACCEPTED_TIMEOUT_AFTER < now {
                to_remove.push(idx);
                continue;
            }
        }
        // Sort in reverse order to remove from the end
        to_remove.sort_by(|a, b| b.cmp(a));
        to_remove.dedup();
        for idx in to_remove {
            storage_locked.tele_snippets.remove(idx);
        }
    }

    // Snippet sending code was here, but it was removed because we at Refact didn't find a good way to
    // use it (in cloud or self-hosting), so we don't have an option to collect it anymore.
}


pub async fn tele_snip_background_task(
    global_context: Arc<ARwLock<global_context::GlobalContext>>,
) -> () {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        send_finished_snippets(global_context.clone()).await;
    }
}
