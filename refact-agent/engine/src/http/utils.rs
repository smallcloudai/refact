use tracing::{error, info};
use axum::middleware::Next;
use axum::Extension;
use axum::http::{Method, Request, Uri};
use axum::response::Response;

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;
use crate::telemetry::telemetry_structs;

const SPAM_HANDLERS: &[&str] = &["rag-status", "ping"];

pub async fn telemetry_middleware<B>(
    path: Uri,
    method: Method,
    ex: Extension<SharedGlobalContext>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, ScratchError> {
    let handler_name = path.path().trim_start_matches('/');
    let spam = SPAM_HANDLERS.contains(&handler_name);

    if !spam {
        info!("\n--- HTTP {} starts ---\n", handler_name);
    }
    let t0 = std::time::Instant::now();

    let mut response = next.run(request).await;

    // ScratchError::into_response creates an extension that is used to let us know that this
    // response used to be a ScratchError. This is useful for logging and telemetry.
    if let Some(e) = response.extensions_mut().remove::<ScratchError>() {
        if !e.telemetry_skip {
            let tele_storage = &ex.read().await.telemetry;
            let mut tele_storage_locked = tele_storage.write().unwrap();
            tele_storage_locked
                .tele_net
                .push(telemetry_structs::TelemetryNetwork::new(
                    path.path().to_string(),
                    format!("{}", method),
                    false,
                    format!("{}", e),
                ));
        }
        error!("{} returning, client will see \"{}\"", path, e);
        return Err(e);
    }

    if !spam {
        info!("{} completed {}ms", path, t0.elapsed().as_millis());
    }

    Ok(response)
}
