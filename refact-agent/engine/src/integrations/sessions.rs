use std::{any::Any, sync::Arc};
use tokio::sync::RwLock as ARwLock;
use tokio::sync::Mutex as AMutex;
use std::future::Future;

use crate::global_context::GlobalContext;

pub trait IntegrationSession: Any + Send + Sync
{
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn is_expired(&self) -> bool;

    fn try_stop(&mut self, self_arc: Arc<AMutex<Box<dyn IntegrationSession>>>) -> Box<dyn Future<Output = String> + Send>;
}

pub fn get_session_hashmap_key(integration_name: &str, base_key: &str) -> String {
    format!("{} ⚡ {}", integration_name, base_key)
}

async fn remove_expired_sessions(gcx: Arc<ARwLock<GlobalContext>>) {
    let expired_sessions = {
        let mut gcx_locked = gcx.write().await;
        let sessions = gcx_locked.integration_sessions.iter()
            .map(|(key, session)| (key.to_string(), session.clone()))
            .collect::<Vec<_>>();
        let mut expired_sessions = vec![];
        for (key, session) in &sessions {
            let session_locked = session.lock().await;
            if session_locked.is_expired() {
                gcx_locked.integration_sessions.remove(key);
                expired_sessions.push(session.clone());
            }
        }
        expired_sessions
    };
    for session in expired_sessions {
        let future = Box::into_pin(session.lock().await.try_stop(session.clone()));
        // no session lock
        future.await;
    }
    // sessions still keeps a reference on all sessions, just in case a destructor is called in the block above
}

pub async fn remove_expired_sessions_background_task(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        remove_expired_sessions(gcx.clone()).await;
    }
}

pub async fn stop_sessions(gcx: Arc<ARwLock<GlobalContext>>) {
    let sessions = {
        let mut gcx_locked = gcx.write().await;
        let sessions = gcx_locked.integration_sessions.iter()
            .map(|(_, session)| Arc::clone(session))
            .collect::<Vec<_>>();
        gcx_locked.integration_sessions.clear();
        sessions
    };
    for session in sessions {
        let future = Box::into_pin(session.lock().await.try_stop(session.clone()));
        // no session lock
        future.await;
    }
}
