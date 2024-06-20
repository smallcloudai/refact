use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, ChatPost, SamplingParameters};
use crate::scratchpads;
use tracing::{info, warn};


pub struct ToolPatch {
}


const PATCH_SYSTEM_PROMPT: &str = r#"
You are a diff generator.
Use this format:

<<<<<<<< SEARCH
original code
========
replacement code
>>>>>>>> END

In addition to changing the existing code, you are also responsible for adding and removing entire files.

To add a file write:

<<<<<<<< NEW
new file code
>>>>>>>> END

To remove a file:

<<<<<<<< REMOVE
>>>>>>>> END
"#;


fn parse_diff_message(path: &String, content: &str) -> Result<serde_json::Value, String> {
    let search_marker = "<<<<<<<< SEARCH";
    let replace_marker = ">>>>>>>> END";
    let equals_marker = "========";

    let search_pos = content.find(search_marker).ok_or("SEARCH marker not found")?;
    let replace_pos = content.find(replace_marker).ok_or("REPLACE marker not found")?;
    let equals_pos = content.find(equals_marker).ok_or("EQUALS marker not found")?;

    if search_pos >= equals_pos || equals_pos >= replace_pos {
        return Err("Markers are in the wrong order".to_string());
    }

    let original_code = &content[search_pos + search_marker.len()..equals_pos].trim();
    let replacement_code = &content[equals_pos + equals_marker.len()..replace_pos].trim();

    let line1 = 1;
    let line2 = 1;

    let file_action = if original_code.is_empty() {
        "new"
    } else if replacement_code.is_empty() {
        "remove"
    } else {
        "edit"
    };

    let edit_jdict = serde_json::json!({
        "file_name": path,
        "file_action": file_action,
        "line1": line1,
        "line2": line2,
        "lines_remove": original_code,
        "lines_add": replacement_code
    });
    return Ok(edit_jdict);
}


#[async_trait]
impl Tool for ToolPatch {
    async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String>
    {
        let path = match args.get("path") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("argument `path` is not a string: {:?}", v)) },
            None => { return Err("argument `path` is not a string".to_string()) }
        };

        let todo = match args.get("todo") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `todo` is not a string: {:?}", v)) },
            None => { "".to_string() }
        };

        let mut chat_post = ChatPost {
            messages: ccx.messages.clone(),
            parameters: SamplingParameters {
                max_new_tokens: 300,
                temperature: Some(0.0),
                top_p: None,
                stop: vec![],
            },
            model: "gpt-3.5-turbo".to_string(),
            scratchpad: "".to_string(),
            stream: Some(false),
            temperature: Some(0.0),
            max_tokens: 300,
            tools: None,
            only_deterministic_messages: false,
            chat_id: "".to_string(),
        };

        let caps = crate::global_context::try_load_caps_quickly_if_not_present(ccx.global_context.clone(), 0).await.map_err(|e| {
            warn!("No caps: {:?}", e);
            format!("Network error communicating with the model (1)")
        })?;

        {
            let message_first: &mut ChatMessage = chat_post.messages.first_mut().unwrap();
            if message_first.role == "system" {
                message_first.content = PATCH_SYSTEM_PROMPT.to_string();
            } else {
                chat_post.messages.insert(0, ChatMessage {
                    role: "system".to_string(),
                    content: PATCH_SYSTEM_PROMPT.to_string(),
                    tool_calls: None,
                    tool_call_id: "".to_string(),
                });
            }
        }
        {
            let message_last: &mut ChatMessage = chat_post.messages.last_mut().unwrap();
            assert!(message_last.role == "assistant");
            assert!(message_last.tool_calls.is_some());
            message_last.tool_calls = None;
        }
        chat_post.messages.push(
            ChatMessage {
                role: "user".to_string(),
                content: format!("You are a diff generator. Use the format in the system prompt exactly. Your goal is the following:\n\n{}", todo),
                tool_calls: None,
                tool_call_id: "".to_string(),
            }
        );

        let (model_name, scratchpad_name, scratchpad_patch, n_ctx, _) = crate::http::routers::v1::chat::lookup_chat_scratchpad(caps.clone(), &chat_post).await?;
        let (client1, api_key) = {
            let cx_locked = ccx.global_context.write().await;
            (cx_locked.http_client.clone(), cx_locked.cmdline.api_key.clone())
        };
        let mut scratchpad = scratchpads::create_chat_scratchpad(
            ccx.global_context.clone(),
            caps,
            model_name.clone(),
            chat_post.clone(),
            &scratchpad_name,
            &scratchpad_patch,
            false,
            false,
        ).await?;
        let t1 = std::time::Instant::now();
        let prompt = scratchpad.prompt(
            n_ctx,
            &mut chat_post.parameters,
        ).await?;
        info!("diff prompt {:?}", t1.elapsed());
        let j = crate::restream::scratchpad_interaction_not_stream_json(
            ccx.global_context.clone(),
            scratchpad,
            "chat".to_string(),
            &prompt,
            model_name,
            client1,
            api_key,
            &chat_post.parameters,
            chat_post.only_deterministic_messages,
        ).await.map_err(|e| {
            warn!("Network error communicating with the (2): {:?}", e);
            format!("Network error communicating with the model (2)")
        })?;

        let choices_array = match j["choices"].as_array() {
            Some(array) => array,
            None => return Err("Unable to get choices array from JSON".to_string()),
        };

        let choice0 = match choices_array.get(0) {
            Some(Value::Object(o)) => o,
            Some(v) => { return Err(format!("choice[0] is not a dict: {:?}", v)) },
            None => { return Err("choice[0] doesn't exist".to_string()) }
        };

        let choice0_message = match choice0.get("message") {
            Some(Value::Object(o)) => o,
            Some(v) => { return Err(format!("choice[0].message is not a dict: {:?}", v)) },
            None => { return Err("choice[0].message doesn't exist".to_string()) }
        };

        let choice0_message_content = match choice0_message.get("content") {
            Some(Value::String(s)) => s,
            Some(v) => { return Err(format!("choice[0].message.content is not a string: {:?}", v)) },
            None => { return Err("choice[0].message.content doesn't exist".to_string()) }
        };

        info!("choice0_message_content: {:?}", choice0_message_content);
        let mut to_parse = choice0_message_content.clone();
        let mut chunks = vec![];
        loop {
            let gt_end = to_parse.find(">>>>>>>> END");
            if gt_end.is_none() {
                break;
            }
            let (eat_now, eat_later) = to_parse.split_at(gt_end.unwrap() + ">>>>>>>> END".len());
            let edit_jdict = parse_diff_message(path, eat_now)?;
            chunks.push(edit_jdict);
            to_parse = eat_later.into();
        }
        info!("chunks: {:?}", chunks);
        if chunks.is_empty() {
            return Err(choice0_message_content.clone());
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "diff".to_string(),
            content: serde_json::to_string_pretty(&chunks).unwrap(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
        }));
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_message() {
        let input = "<<<<<<<< SEARCH\nimport sys, impotlib, os\n========\nimport sys, importlib, os\n>>>>>>>> END";
        let expected_output = serde_json::json!({
            "file_name": "file1.py",
            "file_action": "edit",
            "line1": 1,
            "line2": 1,
            "lines_remove": "import sys, impotlib, os",
            "lines_add": "import sys, importlib, os"
        });

        let result = parse_diff_message(&"file1.py".to_string(), input).unwrap();
        assert_eq!(result, expected_output);
    }
}
