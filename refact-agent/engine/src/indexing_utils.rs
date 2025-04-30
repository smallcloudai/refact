use std::time::Duration;
use tracing::info;
use tokio::sync::RwLock as ARwLock;
use std::sync::Arc;

use crate::global_context::GlobalContext;
use crate::http::routers::v1::status::get_rag_status;

/// Waits for both AST and VecDB indexing to complete before proceeding.
/// This is useful for operations that require up-to-date indexes.
pub async fn wait_for_indexing(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let cmdline = {
        let gcx_locked = gcx.read().await;
        gcx_locked.cmdline.clone()
    };

    let mut waiting_ast = cmdline.wait_ast;
    #[cfg(feature = "vecdb")]
    let mut waiting_vecdb = cmdline.wait_vecdb;
    #[cfg(not(feature = "vecdb"))]
    let mut waiting_vecdb = false;

    while waiting_ast || waiting_vecdb {
        let status = get_rag_status(gcx.clone()).await;
        info!("Waiting for LSP to finish indexing:\n{:?}", status);
        if let Some(ast_status) = &status.ast {
            if waiting_ast && ast_status.astate == "done" {
                info!("Ast finished indexing.");
                waiting_ast = false;
            }
        }
        #[cfg(feature = "vecdb")]
        if let Some(vecdb_status) = &status.vecdb {
            if waiting_vecdb && vecdb_status.state == "done" {
                info!("Vecdb finished indexing.");
                waiting_vecdb = false;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
