use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_web::execute_at_web;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::postprocessing::pp_command_output::OutputFilter;


pub struct ToolWeb {
    pub config_path: String,
}

const DEFAULT_OUTPUT_LIMIT: usize = 200;

fn parse_output_filter(args: &HashMap<String, Value>) -> OutputFilter {
    let output_filter = args.get("output_filter").and_then(|v| v.as_str()).unwrap_or("");
    let output_limit = args.get("output_limit").and_then(|v| v.as_str()).unwrap_or("");

    let is_unlimited = output_limit.eq_ignore_ascii_case("all");

    let limit_lines = if is_unlimited {
        usize::MAX
    } else {
        output_limit.parse::<usize>().unwrap_or(DEFAULT_OUTPUT_LIMIT)
    };

    OutputFilter {
        limit_lines,
        limit_chars: usize::MAX,
        valuable_top_or_bottom: "top".to_string(),
        grep: output_filter.to_string(),
        grep_context_lines: 3,
        remove_from_output: "".to_string(),
        limit_tokens: if is_unlimited { None } else { Some(limit_lines * 50) },
    }
}

#[async_trait]
impl Tool for ToolWeb {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "web".to_string(),
            display_name: "Web".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            agentic: false,
            experimental: false,
            description: "Fetch a web page and convert to readable plain text. Supports regular web pages, PDFs, and JavaScript-rendered pages. Uses Jina Reader API with automatic fallback.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "url".to_string(),
                    description: "URL of the web page to fetch.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "options".to_string(),
                    description: r#"Optional object with additional parameters:
- "respond_with": Response format - "markdown", "html", "text", or "screenshot"
- "target_selector": CSS selector to extract specific element
- "wait_for_selector": CSS selector to wait for (useful for SPAs)
- "timeout": Timeout in seconds for slow pages
- "no_cache": Set to true to bypass cache
- "streaming": Set to true for JS-heavy pages that need more time to render
- "with_generated_alt": Set to true to generate alt text for images using AI"#.to_string(),
                    param_type: "object".to_string(),
                },
                ToolParam {
                    name: "output_filter".to_string(),
                    description: "Optional regex pattern to filter output lines. Only lines matching this pattern (and context) will be shown.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "output_limit".to_string(),
                    description: "Optional. Max lines to show (default: 200). Use higher values like '500' or 'all' to see more output.".to_string(),
                    param_type: "string".to_string(),
                },
            ],
            parameters_required: vec!["url".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let url = match args.get("url") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `url` is not a string: {:?}", v)),
            None => return Err("Missing argument `url`".to_string())
        };

        let options: Option<HashMap<String, Value>> = match args.get("options") {
            Some(Value::Object(obj)) => {
                Some(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            }
            Some(Value::Null) | None => None,
            Some(v) => return Err(format!("argument `options` is not an object: {:?}", v)),
        };

        let text = execute_at_web(&url, options.as_ref()).await?;
        let output_filter = parse_output_filter(args);

        let result = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(text),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            output_filter: Some(output_filter),
            ..Default::default()
        })];

        Ok((false, result))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
