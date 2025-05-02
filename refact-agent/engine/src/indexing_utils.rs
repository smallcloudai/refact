use std::time::Duration;
use tracing::info;
use tokio::sync::RwLock as ARwLock;
use std::sync::Arc;

use crate::global_context::GlobalContext;
use crate::http::routers::v1::status::get_rag_status;

/// Waits for both AST and VecDB indexing to complete based on --wait-ast and --wait-vecdb.
pub async fn wait_for_indexing_if_needed(
    gcx: Arc<ARwLock<GlobalContext>>,
) {
    let cmdline = {
        let gcx_locked = gcx.read().await;
        gcx_locked.cmdline.clone()
    };

    let ast_done = async || get_rag_status(gcx.clone()).await.ast.is_some_and(|ast_status| ast_status.astate == "done");
    #[cfg(feature = "vecdb")]
    let vecdb_done = async || get_rag_status(gcx.clone()).await.vecdb.is_some_and(|vecdb_status| vecdb_status.state == "done");

    let mut waiting_ast = cmdline.wait_ast && !ast_done().await;
    #[cfg(feature = "vecdb")]
    let mut waiting_vecdb = cmdline.wait_vecdb && !vecdb_done().await;
    #[cfg(not(feature = "vecdb"))]
    let mut waiting_vecdb = false;

    if waiting_ast {
        info!("Waiting for AST to finish indexing.");
    }
    if waiting_vecdb {
        info!("Waiting for Vecdb to finish indexing.");
    }

    while waiting_ast || waiting_vecdb {
        if waiting_ast && ast_done().await {
            info!("Ast finished indexing.");
            waiting_ast = false;
        }
        #[cfg(feature = "vecdb")]
        if waiting_vecdb && vecdb_done().await {
            info!("Vecdb finished indexing.");
            waiting_vecdb = false;
        }

        if waiting_ast || waiting_vecdb {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}
