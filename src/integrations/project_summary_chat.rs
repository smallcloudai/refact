use std::sync::Arc;
use std::fs;
use tokio::sync::RwLock as ARwLock;
use std::collections::HashMap;
use itertools::Itertools;
use crate::global_context::GlobalContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextFile};
use crate::scratchpads::chat_utils_prompts::system_prompt_add_workspace_info;

pub async fn mix_config_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &mut Vec<ChatMessage>,
    current_config_file: &String,
) {
    let custom: crate::yaml_configs::customization_loader::CustomizationYaml = match crate::yaml_configs::customization_loader::load_customization(gcx.clone(), true).await {
        Ok(x) => x,
        Err(why) => {
            tracing::error!("Failed to load customization.yaml, will use compiled-in default for the configurator system prompt:\n{:?}", why);
            crate::yaml_configs::customization_loader::load_and_mix_with_users_config(
                crate::yaml_configs::customization_compiled_in::COMPILED_IN_INITIAL_USER_YAML,
                "", "", true, true, &HashMap::new(),
            ).unwrap()
        }
    };
    let sp: &crate::yaml_configs::customization_loader::SystemPrompt = custom.system_prompts.get("project_summary").unwrap();
    let mut sp_text = sp.text.clone();
    sp_text = system_prompt_add_workspace_info(gcx.clone(), &sp_text.replace("%CONFIG_PATH%", current_config_file)).await;
    
    let available_integrations = crate::integrations::setting_up_integrations::integrations_all_with_icons(
        gcx.clone()
    ).await;
    let mut available_integrations_text: String = "Choose tools from this list:\n".to_string();
    for integration in available_integrations.integrations.iter().map(|x| x.integr_name.clone()).unique() {
        available_integrations_text.push_str(&format!("- {}\n", integration))
    }
    
    if messages.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: ChatContent::SimpleText(sp_text),
            tool_calls: None,
            tool_call_id: String::new(),
            usage: None,
        });
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(available_integrations_text),
            tool_calls: None,
            tool_call_id: String::new(),
            usage: None,
        });
    };
}

