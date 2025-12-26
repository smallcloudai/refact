use std::sync::Arc;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatModelType, ChatUsage, SubchatParameters};
use crate::custom_error::MapErrToString;
use crate::global_context::try_load_caps_quickly_if_not_present;
use crate::yaml_configs::customization_loader::load_customization;
use crate::caps::{is_cloud_model, resolve_chat_model, resolve_model};

pub async fn unwrap_subchat_params(ccx: Arc<AMutex<AtCommandsContext>>, tool_name: &str) -> Result<SubchatParameters, String> {
    let (gcx, params_mb) = {
        let ccx_locked = ccx.lock().await;
        let gcx = ccx_locked.global_context.clone();
        let params = ccx_locked.subchat_tool_parameters.get(tool_name).cloned();
        (gcx, params)
    };

    let mut params = match params_mb {
        Some(params) => params,
        None => {
            let mut error_log = Vec::new();
            let tconfig = load_customization(gcx.clone(), true, &mut error_log).await;
            for e in error_log.iter() {
                tracing::error!("{e}");
            }
            tconfig.subchat_tool_parameters.get(tool_name).cloned()
                .ok_or_else(|| format!("subchat params for tool {} not found (checked in Post and in Customization)", tool_name))?
        }
    };

    let caps = try_load_caps_quickly_if_not_present(gcx.clone(), 0).await.map_err_to_string()?;

    if !params.subchat_model.is_empty() {
        match resolve_chat_model(caps.clone(), &params.subchat_model) {
            Ok(_) => return Ok(params),
            Err(e) => {
                tracing::warn!("Specified subchat_model {} is not available: {}", params.subchat_model, e);
            }
        }
    }

    let current_model = ccx.lock().await.current_model.clone();
    let model_to_resolve = match params.subchat_model_type {
        ChatModelType::Light => &caps.defaults.chat_light_model,
        ChatModelType::Default => &caps.defaults.chat_default_model,
        ChatModelType::Thinking => &caps.defaults.chat_thinking_model,
    };

    params.subchat_model = match resolve_model(&caps.chat_models, model_to_resolve) {
        Ok(model_rec) => {
            if !is_cloud_model(&current_model) && is_cloud_model(&model_rec.base.id)
                && params.subchat_model_type != ChatModelType::Light {
                current_model.to_string()
            } else {
                model_rec.base.id.clone()
            }
        },
        Err(e) => {
            tracing::warn!("{:?} model is not available: {}. Using {} model as a fallback.",
                params.subchat_model_type, e, current_model);
            current_model
        }
    };

    tracing::info!("using model for subchat: {}", params.subchat_model);
    Ok(params)
}

pub fn update_usage_from_message(usage: &mut ChatUsage, message: &ChatMessage) {
    if let Some(u) = message.usage.as_ref() {
        usage.total_tokens += u.total_tokens;
        usage.completion_tokens += u.completion_tokens;
        usage.prompt_tokens += u.prompt_tokens;
    }
}
