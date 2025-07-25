use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Thread {
    pub owner_fuser_id: String,
    pub owner_shared: bool,
    pub located_fgroup_id: String,
    pub ft_id: String,
    pub ft_fexp_id: Option<String>,
    pub ft_title: String,
    pub ft_toolset: Option<Vec<Value>>,
    pub ft_error: Option<Value>,
    pub ft_need_assistant: i64,
    pub ft_need_tool_calls: i64,
    pub ft_need_user: i64,
    pub ft_created_ts: f64,
    pub ft_updated_ts: f64,
    pub ft_archived_ts: f64,
    pub ft_locked_by: String,
    pub ft_confirmation_request: Option<Value>,
    pub ft_confirmation_response: Option<Value>,
}

pub async fn create_thread(
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str,
    ft_fexp_id: &str,
    ft_title: &str,
    ft_app_capture: &str,
    ft_app_searchable: &str,
    ft_app_specific: Value,
    ft_toolset: Option<Vec<Value>>,
) -> Result<Thread, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};

    let mutation = r#"
    mutation CreateThread($input: FThreadInput!) {
        thread_create(input: $input) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_fexp_id
            ft_title
            ft_error
            ft_toolset
            ft_need_assistant
            ft_need_tool_calls
            ft_need_user
            ft_created_ts
            ft_updated_ts
            ft_archived_ts
            ft_locked_by
            ft_confirmation_request
            ft_confirmation_response
        }
    }
    "#;

    let toolset_str = match ft_toolset {
        Some(toolset) => serde_json::to_string(&toolset).map_err(|e| format!("Failed to serialize toolset: {}", e))?,
        None => "null".to_string(),
    };

    let input = json!({
        "owner_shared": false,
        "located_fgroup_id": located_fgroup_id,
        "ft_fexp_id": ft_fexp_id,
        "ft_title": ft_title,
        "ft_toolset": toolset_str,
        "ft_app_capture": ft_app_capture,
        "ft_app_searchable": ft_app_searchable,
        "ft_app_specific": serde_json::to_string(&ft_app_specific).unwrap(),
    });

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };
    tracing::info!("create_thread: address={}, ft_title={}, ft_app_capture={}, ft_app_searchable={}",
        config.address, ft_title, ft_app_capture, ft_app_searchable
    );
    execute_graphql::<Thread, _>(
        config,
        mutation,
        json!({"input": input}),
        "thread_create"
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_thread(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
) -> Result<Thread, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};

    let query = r#"
    query GetThread($id: String!) {
        thread_get(id: $id) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_fexp_id,
            ft_title
            ft_error
            ft_toolset
            ft_need_assistant
            ft_need_tool_calls
            ft_need_user
            ft_created_ts
            ft_updated_ts
            ft_archived_ts
            ft_locked_by
            ft_confirmation_request
            ft_confirmation_response
        }
    }
    "#;

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };
    tracing::info!("get_thread: address={}, thread_id={}", config.address, thread_id);
    execute_graphql::<Thread, _>(
        config,
        query,
        json!({"id": thread_id}),
        "thread_get"
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_threads_app_captured(
    cmd_address_url: &str,
    api_key: &str,
    located_fgroup_id: &str,
    ft_app_searchable: &str,
    ft_app_capture: &str,
    tool_call_id: Option<&str>,
) -> Result<Vec<Thread>, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};

    let query = r#"
    query GetThread($located_fgroup_id: String!, $ft_app_capture: String!, $ft_app_searchable: String!, $ft_app_specific_filters: [FTAppSpecificFilter!]) {
        threads_app_captured(
            located_fgroup_id: $located_fgroup_id,
            ft_app_capture: $ft_app_capture,
            ft_app_searchable: $ft_app_searchable,
            ft_app_specific_filters: $ft_app_specific_filters
        ) {
            owner_fuser_id
            owner_shared
            located_fgroup_id
            ft_id
            ft_fexp_id,
            ft_title
            ft_error
            ft_toolset
            ft_need_assistant
            ft_need_tool_calls
            ft_need_user
            ft_created_ts
            ft_updated_ts
            ft_archived_ts
            ft_locked_by
            ft_confirmation_request
            ft_confirmation_response
        }
    }
    "#;

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    let variables = json!({
        "located_fgroup_id": located_fgroup_id,
        "ft_app_capture": ft_app_capture,
        "ft_app_searchable": ft_app_searchable,
        "ft_app_specific_filters": match tool_call_id {
            Some(id) => vec![json!({ "path": "tool_call_id", "equals": id })],
            None => Vec::new(),
        },
    });
    tracing::info!("get_threads_app_captured: address={}, located_fgroup_id={}, ft_app_capture={}, ft_app_searchable={}",
        config.address, located_fgroup_id, ft_app_capture, ft_app_searchable
    );
    execute_graphql::<Vec<Thread>, _>(
        config,
        query,
        variables,
        "threads_app_captured"
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn set_thread_toolset(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    ft_toolset: Vec<Value>
) -> Result<Vec<Value>, String> {
    use crate::cloud::graphql_client::{execute_graphql, GraphQLRequestConfig};

    let mutation = r#"
    mutation UpdateThread($thread_id: String!, $patch: FThreadPatch!) {
        thread_patch(id: $thread_id, patch: $patch) {
            ft_toolset
        }
    }
    "#;

    let variables = json!({
        "thread_id": thread_id,
        "patch": {
            "ft_toolset": serde_json::to_string(&ft_toolset).unwrap()
        }
    });

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    tracing::info!("set_thread_toolset: address={}, thread_id={}, ft_toolset={:?}",
        config.address, thread_id, ft_toolset
    );
    let result = execute_graphql::<Value, _>(
        config,
        mutation,
        variables,
        "thread_patch"
    )
    .await
    .map_err(|e| e.to_string())?;
    if let Some(ft_toolset_json) = result.get("ft_toolset") {
        let ft_toolset: Vec<Value> = serde_json::from_value(ft_toolset_json.clone())
            .map_err(|e| format!("Failed to parse updated thread: {}", e))?;
        Ok(ft_toolset)
    } else {
        Err("ft_toolset not found in response".to_string())
    }
}

pub async fn lock_thread(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    hash: &str,
) -> Result<(), String> {
    use crate::cloud::graphql_client::{execute_graphql_bool_result, GraphQLRequestConfig};

    let worker_name = format!("refact-lsp:{hash}");
    let query = r#"
        mutation AdvanceLock($ft_id: String!, $worker_name: String!) {
            thread_lock(ft_id: $ft_id, worker_name: $worker_name)
        }
    "#;

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    let variables = json!({
        "ft_id": thread_id,
        "worker_name": worker_name
    });

    tracing::info!("lock_thread: address={}, thread_id={}, worker_name={}",
        config.address, thread_id, worker_name
    );
    let result = execute_graphql_bool_result(
        config,
        query,
        variables,
        "thread_lock"
    )
    .await
    .map_err(|e| e.to_string())?;

    if result {
        Ok(())
    } else {
        Err(format!("Thread {thread_id} is locked by another worker"))
    }
}

pub async fn unlock_thread(
    cmd_address_url: &str,
    api_key: &str ,
    thread_id: &str,
    hash: &str,
) -> Result<(), String> {
    use crate::cloud::graphql_client::{execute_graphql_bool_result, GraphQLRequestConfig};

    let worker_name = format!("refact-lsp:{hash}");
    let query = r#"
        mutation AdvanceUnlock($ft_id: String!, $worker_name: String!) {
            thread_unlock(ft_id: $ft_id, worker_name: $worker_name)
        }
    "#;

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    let variables = json!({
        "ft_id": thread_id,
        "worker_name": worker_name
    });

    tracing::info!("unlock_thread: address={}, thread_id={}, worker_name={}",
        config.address, thread_id, worker_name
    );
    let result = execute_graphql_bool_result(
        config,
        query,
        variables,
        "thread_unlock"
    )
    .await
    .map_err(|e| e.to_string())?;

    if result {
        Ok(())
    } else {
        Err(format!("Thread {thread_id} is locked by another worker"))
    }
}

pub async fn set_error_thread(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    error: &str,
) -> Result<(), String> {
    use crate::cloud::graphql_client::{execute_graphql_no_result, GraphQLRequestConfig};

    let mutation = r#"
    mutation SetThreadError($thread_id: String!, $patch: FThreadPatch!) {
        thread_patch(id: $thread_id, patch: $patch) {
            ft_error
        }
    }
    "#;

    let variables = json!({
        "thread_id": thread_id,
        "patch": {
            "ft_error": serde_json::to_string(&json!({"source": "refact_lsp", "error": error})).unwrap()
        }
    });

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };

    tracing::info!("unlock_thread: address={}, thread_id={}, ft_error={}",
        config.address, thread_id, error
    );
    execute_graphql_no_result(
        config,
        mutation,
        variables,
        "thread_patch"
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn set_thread_confirmation_request(
    cmd_address_url: &str,
    api_key: &str,
    thread_id: &str,
    confirmation_request: Value,
) -> Result<bool, String> {
    use crate::cloud::graphql_client::{execute_graphql_bool_result, GraphQLRequestConfig};

    let mutation = r#"
    mutation SetThreadConfirmationRequest($ft_id: String!, $confirmation_request: String!) {
        thread_set_confirmation_request(ft_id: $ft_id, confirmation_request: $confirmation_request)
    }
    "#;

    let confirmation_request_str = serde_json::to_string(&confirmation_request)
        .map_err(|e| format!("Failed to serialize confirmation request: {}", e))?;

    let variables = json!({
        "ft_id": thread_id,
        "confirmation_request": confirmation_request_str
    });

    let config = GraphQLRequestConfig {
        address: cmd_address_url.to_string(),
        api_key: api_key.to_string(),
        ..Default::default()
    };
    tracing::info!("unlock_thread: address={}, thread_id={}, confirmation_request_str={:?}",
        config.address, thread_id, confirmation_request_str
    );
    execute_graphql_bool_result(
        config,
        mutation,
        variables,
        "thread_set_confirmation_request"
    )
    .await
    .map_err(|e| e.to_string())
}
