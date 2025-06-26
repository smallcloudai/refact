use std::fmt::{Display, Formatter};
use std::fmt::Debug;
use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use reqwest::Client;
use serde_json::{Value, json};
use log::error;
use crate::constants::get_graphql_url;

/// Configuration for GraphQL requests
pub struct GraphQLRequestConfig {
    pub address: String,
    pub api_key: String,
    pub user_agent: Option<String>,
    pub additional_headers: Option<HashMap<String, String>>,
}

impl Default for GraphQLRequestConfig {
    fn default() -> Self {
        Self {
            address: String::new(),
            api_key: String::new(),
            user_agent: Some("refact-lsp".to_string()),
            additional_headers: None,
        }
    }
}

/// Generic error type for GraphQL operations
#[derive(Debug)]
pub enum GraphQLError {
    Network(reqwest::Error),
    Json(serde_json::Error),
    GraphQL(String),
    Response { 
        status: reqwest::StatusCode, 
        message: String 
    },
    UnexpectedFormat(String),
    DataNotFound(String),
}

impl Display for GraphQLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphQLError::Network(err) => write!(f, "Network error: {}", err),
            GraphQLError::Json(err) => write!(f, "JSON error: {}", err),
            GraphQLError::GraphQL(err) => write!(f, "GraphQL error: {}", err),
            GraphQLError::Response { status, message } => {
                write!(f, "Response error: HTTP status {}, error: {}", status, message)
            }
            GraphQLError::UnexpectedFormat(err) => write!(f, "Unexpected response format: {}", err),
            GraphQLError::DataNotFound(err) => write!(f, "Data not found in response: {}", err),
        }
    }
}

impl std::error::Error for GraphQLError {}

impl From<reqwest::Error> for GraphQLError {
    fn from(err: reqwest::Error) -> Self {
        GraphQLError::Network(err)
    }
}

impl From<serde_json::Error> for GraphQLError {
    fn from(err: serde_json::Error) -> Self {
        GraphQLError::Json(err)
    }
}

/// Type alias for GraphQL results
pub type GraphQLResult<T> = Result<T, GraphQLError>;

/// Execute a GraphQL operation and return the deserialized result
pub async fn execute_graphql<T, V>(
    config: GraphQLRequestConfig,
    operation: &str,
    variables: V,
    result_path: &str,
) -> GraphQLResult<T>
where
    T: DeserializeOwned + Debug,
    V: Serialize,
{
    let client = Client::new();
    
    let mut request_builder = client
        .post(&get_graphql_url(&config.address))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    if let Some(user_agent) = config.user_agent {
        request_builder = request_builder.header("User-Agent", user_agent);
    }
    
    if let Some(headers) = config.additional_headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }
    
    let request_body = json!({
        "query": operation,
        "variables": variables
    });
    
    let response = request_builder
        .json(&request_body)
        .send()
        .await
        .map_err(GraphQLError::Network)?;
    
    if response.status().is_success() {
        let response_body = response
            .text()
            .await
            .map_err(|e| GraphQLError::Network(e))?;
            
        let response_json: Value = serde_json::from_str(&response_body)
            .map_err(GraphQLError::Json)?;
            
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(GraphQLError::GraphQL(error_msg));
        }
        
        if let Some(data) = response_json.get("data") {
            if let Some(result_value) = data.get(result_path) {
                if result_value.is_null() {
                    return Err(GraphQLError::DataNotFound(format!(
                        "Result at path '{}' is null", result_path
                    )));
                }
                
                let result = serde_json::from_value(result_value.clone())
                    .map_err(|e| GraphQLError::Json(e))?;
                return Ok(result);
            }
            
            return Err(GraphQLError::DataNotFound(format!(
                "Result path '{}' not found in response data", result_path
            )));
        }
        
        Err(GraphQLError::UnexpectedFormat(format!(
            "Unexpected response format: {}",
            response_body
        )))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
            
        Err(GraphQLError::Response {
            status,
            message: error_text,
        })
    }
}

/// Execute a GraphQL operation that doesn't return a specific result
pub async fn execute_graphql_no_result<V>(
    config: GraphQLRequestConfig,
    operation: &str,
    variables: V,
    result_path: &str,
) -> GraphQLResult<()>
where
    V: Serialize,
{
    let client = Client::new();
    
    let mut request_builder = client
        .post(&get_graphql_url(&config.address))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    if let Some(user_agent) = config.user_agent {
        request_builder = request_builder.header("User-Agent", user_agent);
    }
    
    if let Some(headers) = config.additional_headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }
    
    let request_body = json!({
        "query": operation,
        "variables": variables
    });
    
    let response = request_builder
        .json(&request_body)
        .send()
        .await
        .map_err(GraphQLError::Network)?;
    
    if response.status().is_success() {
        let response_body = response
            .text()
            .await
            .map_err(|e| GraphQLError::Network(e))?;
            
        let response_json: Value = serde_json::from_str(&response_body)
            .map_err(GraphQLError::Json)?;
            
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(GraphQLError::GraphQL(error_msg));
        }
        
        if let Some(data) = response_json.get("data") {
            if data.get(result_path).is_some() {
                return Ok(());
            }
            
            return Err(GraphQLError::DataNotFound(format!(
                "Result path '{}' not found in response data", result_path
            )));
        }
        
        Err(GraphQLError::UnexpectedFormat(format!(
            "Unexpected response format: {}",
            response_body
        )))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
            
        Err(GraphQLError::Response {
            status,
            message: error_text,
        })
    }
}

/// Execute a GraphQL operation that returns a boolean success indicator
pub async fn execute_graphql_bool_result<V>(
    config: GraphQLRequestConfig,
    operation: &str,
    variables: V,
    result_path: &str,
) -> GraphQLResult<bool>
where
    V: Serialize,
{
    let client = Client::new();
    
    let mut request_builder = client
        .post(&get_graphql_url(&config.address))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json");
    
    if let Some(user_agent) = config.user_agent {
        request_builder = request_builder.header("User-Agent", user_agent);
    }
    
    if let Some(headers) = config.additional_headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }
    
    let request_body = json!({
        "query": operation,
        "variables": variables
    });
    
    let response = request_builder
        .json(&request_body)
        .send()
        .await
        .map_err(GraphQLError::Network)?;
    
    if response.status().is_success() {
        let response_body = response
            .text()
            .await
            .map_err(|e| GraphQLError::Network(e))?;
            
        let response_json: Value = serde_json::from_str(&response_body)
            .map_err(GraphQLError::Json)?;
            
        if let Some(errors) = response_json.get("errors") {
            let error_msg = errors.to_string();
            error!("GraphQL error: {}", error_msg);
            return Err(GraphQLError::GraphQL(error_msg));
        }
        
        if let Some(data) = response_json.get("data") {
            if let Some(result_value) = data.get(result_path) {
                if let Some(bool_value) = result_value.as_bool() {
                    return Ok(bool_value);
                }
                
                return Err(GraphQLError::UnexpectedFormat(format!(
                    "Expected boolean value at path '{}', got: {:?}", 
                    result_path, result_value
                )));
            }
            
            return Err(GraphQLError::DataNotFound(format!(
                "Result path '{}' not found in response data", result_path
            )));
        }
        
        Err(GraphQLError::UnexpectedFormat(format!(
            "Unexpected response format: {}",
            response_body
        )))
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
            
        Err(GraphQLError::Response {
            status,
            message: error_text,
        })
    }
}

/// Convert GraphQLError to string error message for backward compatibility
pub fn graphql_error_to_string(error: GraphQLError) -> String {
    error.to_string()
}
