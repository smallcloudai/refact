use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use crate::global_context::GlobalContext;
use crate::call_validation::{ChatContent, ChatMessage, ChatMeta};
use crate::integrations::setting_up_integrations::integrations_all;
use crate::scratchpads::chat_utils_prompts::system_prompt_add_workspace_info;
use crate::scratchpads::scratchpad_utils::HasRagResults;


pub async fn mix_project_summary_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_meta: &ChatMeta,
    messages: &mut Vec<ChatMessage>,
    stream_back_to_user: &mut HasRagResults,
) {
    assert!(messages[0].role != "system");  // we are here to add this, can't already exist

    let mut error_log = Vec::new();
    let custom = crate::yaml_configs::customization_loader::load_customization(gcx.clone(), true, &mut error_log).await;
    for e in error_log.iter() {
        tracing::error!(
            "{}:{} {:?}",
            crate::nicer_logs::last_n_chars(&e.integr_config_path, 30),
            e.error_line,
            e.error_msg,
        );
    }


    let sp: &crate::yaml_configs::customization_loader::SystemPrompt = custom.system_prompts.get("project_summary").unwrap();
    let mut sp_text = sp.text.clone();

    if sp_text.contains("%ALL_INTEGRATIONS%") {
        let allow_experimental = gcx.read().await.cmdline.experimental;
        let all_integrations = crate::integrations::integrations_list(allow_experimental);
        sp_text = sp_text.replace("%ALL_INTEGRATIONS%", &all_integrations.join(", "));
    }

    if sp_text.contains("%AVAILABLE_INTEGRATIONS%") {
        let integrations_all = integrations_all(gcx.clone()).await.integrations;
        let integrations = integrations_all.iter().filter(|x|x.integr_config_exists && x.project_path.is_empty()).collect::<Vec<_>>();
        sp_text = sp_text.replace("%AVAILABLE_INTEGRATIONS%", &integrations.iter().map(|x|x.integr_name.clone()).collect::<Vec<_>>().join(", "));
    }

    sp_text = system_prompt_add_workspace_info(gcx.clone(), &sp_text).await;    // print inside

    let system_message = ChatMessage {
        role: "system".to_string(),
        content: ChatContent::SimpleText(sp_text),
        ..Default::default()
    };

    if messages.len() == 1 {
        stream_back_to_user.push_in_json(serde_json::json!(system_message));
    } else {
        tracing::error!("more than 1 message when mixing configuration chat context, bad things might happen!");
    }

    messages.splice(0..0, vec![system_message]);
}

