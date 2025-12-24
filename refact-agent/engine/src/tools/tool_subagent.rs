use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;

use crate::subchat::subchat;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ChatUsage, ContextEnum, SubchatParameters};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::memories::{memories_add_enriched, EnrichmentParams};

pub struct ToolSubagent {
    pub config_path: String,
}

static SUBAGENT_SYSTEM_PROMPT: &str = r#"You are a focused sub-agent executing a specific task. You have been delegated this task by a parent agent.

Your task is clearly defined below. Execute it efficiently and report your findings.

Guidelines:
- Stay focused on the assigned task only
- Use the provided tools to accomplish the task
- Be thorough but efficient - you have a limited step budget
- Report progress and findings clearly
- When you achieve the expected result, summarize what you found/did
- If you cannot complete the task, explain why and what you tried

Do NOT:
- Deviate from the assigned task
- Ask clarifying questions - work with what you have
- Exceed your step budget unnecessarily"#;

static WRAP_UP_PROMPT: &str = r#"Summarize your work. What did you accomplish? What are the key findings or results?"#;

fn build_task_prompt(task: &str, expected_result: &str, tools: &[String], max_steps: usize) -> String {
    format!(
        r#"# Your Task
{task}

# Expected Result
{expected_result}

# Available Tools
You have access to these tools: {tools_list}

# Constraints
- Maximum steps allowed: {max_steps}
- Focus only on this specific task
- Report findings clearly when done"#,
        task = task,
        expected_result = expected_result,
        tools_list = if tools.is_empty() { "all available".to_string() } else { tools.join(", ") },
        max_steps = max_steps
    )
}

async fn execute_subagent(
    ccx_subchat: Arc<AMutex<AtCommandsContext>>,
    subchat_params: &SubchatParameters,
    task: &str,
    expected_result: &str,
    tools: Vec<String>,
    max_steps: usize,
    usage_collector: &mut ChatUsage,
    tool_call_id: &String,
    log_prefix: &str,
) -> Result<ChatMessage, String> {
    let task_prompt = build_task_prompt(task, expected_result, &tools, max_steps);

    let messages = vec![
        ChatMessage::new("system".to_string(), SUBAGENT_SYSTEM_PROMPT.to_string()),
        ChatMessage::new("user".to_string(), task_prompt),
    ];

    let tools_subset = if tools.is_empty() {
        vec![]
    } else {
        tools
    };

    let choices = subchat(
        ccx_subchat.clone(),
        subchat_params.subchat_model.as_str(),
        messages,
        tools_subset,
        max_steps,
        subchat_params.subchat_n_ctx - subchat_params.subchat_max_new_tokens,
        WRAP_UP_PROMPT,
        1,
        subchat_params.subchat_temperature,
        subchat_params.subchat_reasoning_effort.clone(),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-subagent")),
        Some(true),
    ).await?;

    let session = choices.into_iter().next().unwrap();
    let reply = session.last().unwrap().clone();
    crate::tools::tools_execute::update_usage_from_message(usage_collector, &reply);

    Ok(reply)
}

#[async_trait]
impl Tool for ToolSubagent {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "subagent".to_string(),
            display_name: "Subagent".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Delegate a specific task to a sub-agent that works independently. Use this when you need to perform a focused task that requires multiple tool calls without cluttering the main conversation. The subagent has its own context and does not see the parent conversation.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "task".to_string(),
                    param_type: "string".to_string(),
                    description: "Clear description of what the subagent should do. Be specific about the goal and any constraints.".to_string(),
                },
                ToolParam {
                    name: "expected_result".to_string(),
                    param_type: "string".to_string(),
                    description: "Description of what the successful result should look like. This helps the subagent know when it has completed the task.".to_string(),
                },
                ToolParam {
                    name: "tools".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of tool names the subagent should use (e.g., 'cat,tree,search'). Leave empty to allow all available tools.".to_string(),
                },
                ToolParam {
                    name: "max_steps".to_string(),
                    param_type: "string".to_string(),
                    description: "Maximum number of steps (tool calls) the subagent can make. Default is 10. Use lower values for simple tasks, higher for complex ones.".to_string(),
                },
            ],
            parameters_required: vec!["task".to_string(), "expected_result".to_string(), "tools".to_string(), "max_steps".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let task = match args.get("task") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `task` is not a string: {:?}", v)),
            None => return Err("Missing argument `task`".to_string())
        };

        let expected_result = match args.get("expected_result") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `expected_result` is not a string: {:?}", v)),
            None => return Err("Missing argument `expected_result`".to_string())
        };

        let tools: Vec<String> = match args.get("tools") {
            Some(Value::String(s)) if !s.trim().is_empty() => {
                s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
            },
            _ => vec![]
        };

        let max_steps: usize = match args.get("max_steps") {
            Some(Value::String(s)) => s.parse().unwrap_or(10),
            Some(Value::Number(n)) => n.as_u64().unwrap_or(10) as usize,
            _ => 10
        };
        let max_steps = max_steps.min(50).max(1);

        let mut usage_collector = ChatUsage { ..Default::default() };
        let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let subchat_params: SubchatParameters = crate::tools::tools_execute::unwrap_subchat_params(ccx.clone(), "subagent").await?;

        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                subchat_params.subchat_n_ctx,
                8,
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

        tracing::info!("Starting subagent for task: {}", task);
        let subagent_result = execute_subagent(
            ccx_subchat.clone(),
            &subchat_params,
            &task,
            &expected_result,
            tools,
            max_steps,
            &mut usage_collector,
            tool_call_id,
            &log_prefix,
        ).await?;

        let final_message = format!(
            "# Subagent Report\n\n**Task:** {}\n\n**Expected Result:** {}\n\n## Result\n{}",
            task,
            expected_result,
            subagent_result.content.content_text_only()
        );
        tracing::info!("Subagent completed task");

        let title = if task.len() > 80 {
            format!("{}...", &task[..80])
        } else {
            task.clone()
        };
        let enrichment_params = EnrichmentParams {
            base_tags: vec!["subagent".to_string(), "delegation".to_string()],
            base_filenames: vec![],
            base_kind: "subagent".to_string(),
            base_title: Some(title),
        };
        if let Err(e) = memories_add_enriched(ccx.clone(), &final_message, enrichment_params).await {
            tracing::warn!("Failed to create enriched memory from subagent: {}", e);
        } else {
            tracing::info!("Created enriched memory from subagent");
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(final_message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            usage: Some(usage_collector),
            output_filter: Some(crate::postprocessing::pp_command_output::OutputFilter::no_limits()),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
