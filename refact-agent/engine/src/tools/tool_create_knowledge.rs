use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};

pub struct ToolCreateKnowledge;

#[async_trait]
impl Tool for ToolCreateKnowledge {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "create_knowledge".to_string(),
            agentic: true,
            experimental: false,
            description: "Creates a new knowledge entry in the vector database to help with future tasks.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "im_going_to_use_tools".to_string(),
                    param_type: "string".to_string(),
                    description: "Which tools have you used? Comma-separated list, examples: hg, git, gitlab, rust debugger".to_string(),
                },
                ToolParam {
                    name: "im_going_to_apply_to".to_string(),
                    param_type: "string".to_string(),
                    description: "What have your actions been applied to? List all you can identify, starting with the project name. Comma-separated list, examples: project1, file1.cpp, MyClass, PRs, issues".to_string(),
                },
                ToolParam {
                    name: "search_key".to_string(),
                    param_type: "string".to_string(),
                    description: "Search keys for the knowledge database. Write combined elements from all fields (tools, project components, objectives, and language/framework). This field is used for vector similarity search.".to_string(),
                },
                ToolParam {
                    name: "language_slash_framework".to_string(),
                    param_type: "string".to_string(),
                    description: "What programming language and framework has the current project used? Use lowercase, dashes and dots. Examples: python/django, typescript/node.js, rust/tokio, ruby/rails, php/laravel, c++/boost-asio".to_string(),
                },
                ToolParam {
                    name: "knowledge_entry".to_string(),
                    param_type: "string".to_string(),
                    description: "The detailed knowledge content to store. Include comprehensive information about implementation details, code patterns, architectural decisions, troubleshooting steps, or solution approaches. Document what you did, how you did it, why you made certain choices, and any important observations or lessons learned. This field should contain the rich, detailed content that future searches will retrieve.".to_string(),
                }
            ],
            parameters_required: vec![
                "im_going_to_use_tools".to_string(),
                "im_going_to_apply_to".to_string(),
                "search_key".to_string(),
                "language_slash_framework".to_string(),
                "knowledge_entry".to_string(),
            ],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        info!("run @create-knowledge with args: {:?}", args);

        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };

        let im_going_to_use_tools = match args.get("im_going_to_use_tools") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `im_going_to_use_tools` is not a string: {:?}", v)),
            None => return Err("argument `im_going_to_use_tools` is missing".to_string())
        };

        let im_going_to_apply_to = match args.get("im_going_to_apply_to") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `im_going_to_apply_to` is not a string: {:?}", v)),
            None => return Err("argument `im_going_to_apply_to` is missing".to_string())
        };

        let search_key = match args.get("search_key") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `search_key` is not a string: {:?}", v)),
            None => return Err("argument `search_key` is missing".to_string())
        };

        let language_slash_framework = match args.get("language_slash_framework") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `language_slash_framework` is not a string: {:?}", v)),
            None => return Err("argument `language_slash_framework` is missing".to_string())
        };

        let knowledge_entry = match args.get("knowledge_entry") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `knowledge_entry` is not a string: {:?}", v)),
            None => return Err("argument `knowledge_entry` is missing".to_string())
        };

        let vec_db = gcx.read().await.vec_db.clone();
        
        // Store the memory with type "knowledge-entry"
        let memid = match crate::vecdb::vdb_highlev::memories_add(
            vec_db.clone(),
            "knowledge-entry",
            &search_key,
            &im_going_to_apply_to,
            &knowledge_entry,
            "user-created"
        ).await {
            Ok(id) => id,
            Err(e) => return Err(format!("Failed to store knowledge: {}", e))
        };

        let message = format!("Knowledge entry created successfully with ID: {}\nTools: {}\nApply to: {}\nSearch Key: {}\nLanguage/Framework: {}\nEntry: {}", 
            memid,
            im_going_to_use_tools,
            im_going_to_apply_to,
            search_key,
            language_slash_framework,
            knowledge_entry
        );

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(message),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}