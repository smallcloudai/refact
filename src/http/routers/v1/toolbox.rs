use axum::response::Result;
use axum::Extension;
use serde_json::json;
use serde::{Serialize, Deserialize};
use hyper::{Body, Response, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use std::collections::HashMap;
use tracing::error;

use crate::call_validation::ChatMessage;
use crate::global_context::GlobalContext;
use crate::custom_error::ScratchError;
use crate::toolbox::toolbox_config::load_customization;


pub async fn handle_v1_customization(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    _body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
	let tconfig = match load_customization(global_context.clone()).await {
		Ok(config) => config,
		Err(err) => {
			error!("load_customization: {}", err);
			return Ok(Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from(serde_json::to_string_pretty(&json!({ "detail": err.to_string() })).unwrap()))
				.unwrap());
		}
	};
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string_pretty(&tconfig).unwrap()))
        .unwrap())
}


#[derive(Serialize, Deserialize, Clone)]
struct SnippetAcceptedPostData {
    pub messages: Vec<ChatMessage>,
}


pub async fn handle_v1_rewrite_assistant_says_to_at_commands(
	Extension(_global_context): Extension<Arc<ARwLock<GlobalContext>>>,
	body_bytes: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<SnippetAcceptedPostData>(&body_bytes).map_err(|e| {
	    ScratchError::new(StatusCode::BAD_REQUEST, format!("JSON problem: {}", e))
	})?;

	let mut assistant_says = String::new();
	let mut auto_response_map = HashMap::<String, String>::new();
	let mut auto_response_str = String::new();
	let mut original_toolbox_command = String::new();
	for msg in post.messages.iter() {
		if msg.role == "ignore" {
	    	// msg.content="original-toolbox-command why\nauto-reponse 👣PROVIDE_COMMANDS_STEP 👣GENERATE_DOCUMENTATION_STEP\n"
	    	for line in msg.content.lines() {
	    		if line.starts_with("original-toolbox-command ") {
	    			original_toolbox_command = line.replace("original-toolbox-command ", "").to_string();
	    		}
	    		if line.starts_with("auto-reponse ") {
	    			let parts: Vec<&str> = line.split_whitespace().collect();
	    			if parts.len() != 3 {
	    				error!("auto-reponse has wrong format: {}", line);
	    				continue;
	    			} else {
	    				auto_response_map.insert(parts[1].to_string(), parts[2].to_string());
	    			}
	    		}
	    	}
		}
		if msg.role == "assistant" {
			assistant_says = msg.content.clone();
		}
		if msg.role == "user" {
			auto_response_str = "".to_string();
			for user_says_line in msg.content.lines() {
		        for (k, v) in auto_response_map.iter() {
		        	if user_says_line.starts_with(k) {
	        			auto_response_str = v.clone();
	        		}
	        	}
	        }
		}
	}

    let mut out = String::new();
    for s in assistant_says.lines() {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("🔍SEARCH ") {
            out += "@workspace ";
            out += &s[11..];
            out += "\n";
        }
        if s.starts_with("🔍FILE ") {
            out += "@file ";
            out += &s[9..];
            out += "\n";
        }
        if s.starts_with("🔍DEFINITION ") {
            out += "@definition ";
            out += &s[15..];
            out += "\n";
        }
    }
    if auto_response_str != "" {
    	out += &auto_response_str.to_string();
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(json!({
        	"success": out!= "",
        	"suggested_user_message": out,
        	"auto_response": auto_response_str != "",
        	"original_toolbox_command": original_toolbox_command,
        }).to_string()))
        .unwrap())
}
