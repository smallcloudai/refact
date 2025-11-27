use std::collections::HashMap;
use std::sync::Arc;
use serde_json::{Value, json};
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::subchat::subchat_single;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType, MatchConfirmDeny, MatchConfirmDenyResult};
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::integrations::integr_abstract::IntegrationConfirmation;

pub struct ToolDeepResearch {
    pub config_path: String,
}

static RESEARCHER_SYSTEM_PROMPT: &str = r#"You are a professional researcher preparing a structured, data-driven report. Your task is to analyze the research question the user poses.

Do:
- Focus on data-rich insights: include specific figures, trends, statistics, and measurable outcomes.
- When appropriate, summarize data in a way that could be turned into charts or tables, and call this out in the response.
- Prioritize reliable, up-to-date sources: official documentation, peer-reviewed research, reputable technical blogs, and official project repositories.
- Include inline citations and return all source metadata.

Be analytical, avoid generalities, and ensure that each section supports data-backed reasoning that could inform technical decisions or implementation strategies."#;

static ENTERTAINMENT_MESSAGES: &[&str] = &[
    "🔬 Deep research in progress... This may take up to 20 minutes, please be patient!",
    "🌐 Browsing the web and gathering relevant sources...",
    "📚 Reading through documentation and articles...",
    "🔍 Cross-referencing information from multiple sources...",
    "🧠 Analyzing and synthesizing the findings...",
    "📊 Organizing data and preparing insights...",
    "✍️ Composing comprehensive report with citations...",
    "⏳ Still working... Almost there!",
    "🔄 Continuing deep research... Thank you for your patience!",
];

async fn send_entertainment_message(
    subchat_tx: &Arc<AMutex<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>,
    tool_call_id: &str,
    message_idx: usize,
) {
    let message_text = ENTERTAINMENT_MESSAGES[message_idx % ENTERTAINMENT_MESSAGES.len()];
    let entertainment_msg = json!({
        "tool_call_id": tool_call_id,
        "subchat_id": message_text,
        "add_message": {
            "role": "assistant",
            "content": message_text
        }
    });
    let _ = subchat_tx.lock().await.send(entertainment_msg);
}

fn spawn_entertainment_task(
    subchat_tx: Arc<AMutex<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>,
    tool_call_id: String,
    cancel_token: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        let mut message_idx = 0usize;
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
                    send_entertainment_message(&subchat_tx, &tool_call_id, message_idx).await;
                    message_idx += 1;
                }
            }
        }
    });
}

async fn execute_deep_research(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    research_query: &str,
    usage_collector: &mut ChatUsage,
    tool_call_id: &String,
    log_prefix: &str,
) -> Result<ChatMessage, String> {
    let subchat_tx = ccx_subchat.lock().await.subchat_tx.clone();

    send_entertainment_message(&subchat_tx, tool_call_id, 0).await;

    let cancel_token = tokio_util::sync::CancellationToken::new();
    spawn_entertainment_task(subchat_tx, tool_call_id.clone(), cancel_token.clone());

    let messages = vec![
        ChatMessage::new("system".to_string(), RESEARCHER_SYSTEM_PROMPT.to_string()),
        ChatMessage::new("user".to_string(), research_query.to_string()),
    ];

    let result = subchat_single(
        ccx_subchat.clone(),
        subchat_params.subchat_model.as_str(),
        messages,
        Some(vec![]),
        None,
        false,
        subchat_params.subchat_temperature,
        Some(subchat_params.subchat_max_new_tokens),
        1,
        subchat_params.subchat_reasoning_effort.clone(),
        false,
        Some(usage_collector),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-deep-research")),
    ).await;

    cancel_token.cancel();

    let choices = result?;
    let session = choices.into_iter().next().unwrap();
    let reply = session.last().unwrap().clone();
    crate::tools::tools_execute::update_usage_from_message(usage_collector, &reply);

    Ok(reply)
}

#[async_trait]
impl Tool for ToolDeepResearch {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "deep_research".to_string(),
            display_name: "Deep Research".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Conduct comprehensive web research on a topic. Use this tool when you need up-to-date information from the internet, market analysis, technical documentation research, or synthesis of information from multiple web sources. The research takes several minutes and produces a detailed, citation-rich report. Do NOT use for questions about the current codebase - use code exploration tools instead.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "research_query".to_string(),
                    param_type: "string".to_string(),
                    description: "A detailed research question or topic. Be specific: include the scope, what comparisons or metrics you need, any preferred sources, and the desired output format. Example: 'Research the current best practices for Rust async error handling in 2024, comparing tokio vs async-std approaches, with code examples and performance considerations.'".to_string(),
                }
            ],
            parameters_required: vec!["research_query".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let research_query = match args.get("research_query") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `research_query` is not a string: {:?}", v)),
            None => return Err("Missing argument `research_query`".to_string())
        };

        let mut usage_collector = ChatUsage { ..Default::default() };
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let subchat_params: SubchatParameters = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "deep_research").await?;

        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                subchat_params.subchat_n_ctx,
                0,
                false,
                vec![],
                ccx_lock.chat_id.clone(),
                ccx_lock.should_execute_remotely,
                ccx_lock.current_model.clone(),
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        tracing::info!("Starting deep research for query: {}", research_query);
        let research_result = execute_deep_research(
            ccx_subchat.clone(),
            &subchat_params,
            &research_query,
            &mut usage_collector,
            tool_call_id,
            &log_prefix,
        ).await?;

        let final_message = format!("# Deep Research Report\n\n{}", research_result.content.content_text_only());
        tracing::info!("Deep research completed");

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(final_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage_collector),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let query = match args.get("research_query") {
            Some(Value::String(s)) => s.clone(),
            _ => return Ok("".to_string()),
        };
        let truncated_query = if query.len() > 100 {
            format!("{}...", &query[..100])
        } else {
            query
        };
        Ok(format!("deep_research \"{}\"", truncated_query))
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(IntegrationConfirmation {
            ask_user: vec!["*".to_string()],
            deny: vec![],
        })
    }

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self.command_to_match_against_confirm_deny(ccx.clone(), &args).await.map_err(|e| {
            format!("Error getting tool command to match: {}", e)
        })?;
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::CONFIRMATION,
            command: command_to_match,
            rule: "default".to_string(),
        })
    }
}
