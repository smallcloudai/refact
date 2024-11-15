use std::sync::Arc;
use std::fs;
use tokio::sync::RwLock as ARwLock;
use std::collections::HashMap;

use crate::global_context::GlobalContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextFile};


pub async fn mix_config_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &mut Vec<ChatMessage>,
    current_config_file: &String,
) {
    let config_dir = gcx.read().await.config_dir.clone();
    let file_path = config_dir.join("integrations.d");

    let mut context_file_vec = Vec::new();
    if let Ok(entries) = fs::read_dir(&file_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    if let Ok(file_content) = fs::read_to_string(&path) {
                        let context_file = ContextFile {
                            file_name: path.to_string_lossy().to_string(),
                            file_content,
                            line1: 0,
                            line2: 0,
                            symbols: vec![],
                            gradient_type: -1,
                            usefulness: 100.0,
                        };
                        context_file_vec.push(context_file);
                    }
                }
            }
        }
    }
    let custom: crate::yaml_configs::customization_loader::CustomizationYaml = match crate::yaml_configs::customization_loader::load_customization(gcx, true).await {
        Ok(x) => x,
        Err(why) => {
            tracing::error!("Failed to load customization.yaml, will use compiled-in default for the configurator system prompt:\n{:?}", why);
            crate::yaml_configs::customization_loader::load_and_mix_with_users_config(
                crate::yaml_configs::customization_compiled_in::COMPILED_IN_INITIAL_USER_YAML,
                "", "", true, true, &HashMap::new(),
            ).unwrap()
        }
    };
    let sp: &crate::yaml_configs::customization_loader::SystemPrompt = custom.system_prompts.get("configurator").unwrap();

    // let json_vec = context_file_vec.iter().map(|p| serde_json::json!(p)).collect::<Vec<_>>();
    messages.insert(0, ChatMessage {
        role: "context_file".to_string(),
        content: ChatContent::SimpleText(serde_json::to_string(&context_file_vec).unwrap()),
        tool_calls: None,
        tool_call_id: String::new(),
        usage: None,
    });
    messages.insert(0, ChatMessage {
        role: "system".to_string(),
        content: ChatContent::SimpleText(sp.text.clone()),
        tool_calls: None,
        tool_call_id: String::new(),
        usage: None,
    });

    for msg in messages.iter_mut() {
        if let ChatContent::SimpleText(ref mut content) = msg.content {
            *content = content.replace("%CURRENT_CONFIG%", current_config_file);
        }
    }

    tracing::info!("AAAAA\n{:#?}", messages);
}

