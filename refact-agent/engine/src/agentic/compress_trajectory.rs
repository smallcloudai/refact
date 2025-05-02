use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use crate::caps::strip_model_from_finetune;

const COMPRESSION_MESSAGE: &str = r#"Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like file names, full code snippets, function signatures, file edits, etc
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Pay special attention to the most recent messages and include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
5. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
6. Current Work: Describe in detail precisely what was being worked on immediately before this summary request, paying special attention to the most recent messages from both user and assistant. Include file names and code snippets where applicable.
7. Optional Next Step: List the next step that you will take that is related to the most recent work you were doing. IMPORTANT: ensure that this step is DIRECTLY in line with the user's explicit requests, and the task you were working on immediately before this summary request. If your last task was concluded, then only list next steps if they are explicitly in line with the users request. Do not start on tangential requests without confirming with the user first.
8. If there is a next step, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no drift in task interpretation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]
   - [...]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Summary of the changes made to this file, if any]
      - [Important Code Snippet]
   - [File Name 2]
      - [Important Code Snippet]
   - [...]

4. Problem Solving:
   [Description of solved problems and ongoing troubleshooting]`

5. Pending Tasks:
   - [Task 1]
   - [Task 2]
   - [...]

6. Current Work:
   [Precise description of current work]

7. Optional Next Step:
   [Optional Next step to take]

</summary>
</example>

Please provide your summary based on the conversation so far, following this structure and ensuring precision and thoroughness in your response."#;
const TEMPERATURE: f32 = 0.0;

fn gather_used_tools(messages: &Vec<ChatMessage>) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    
    for message in messages {
        if let Some(tool_calls) = &message.tool_calls {
            for tool_call in tool_calls {
                if !tools.contains(&tool_call.function.name) {
                    tools.push(tool_call.function.name.clone());
                }
            }
        }
    }
    
    tools
}

pub async fn compress_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ChatMessage>,
) -> Result<(String, String), String> {
    if messages.is_empty() {
        return Err("The provided chat is empty".to_string());
    }
    let (model_id, n_ctx) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            let model_id = caps.defaults.chat_default_model.clone();
            if let Some(model_rec) = caps.chat_models.get(&strip_model_from_finetune(&model_id)) {
                Ok((model_id, model_rec.base.n_ctx))
            } else {
                Err(format!(
                    "Model '{}' not found, server has these models: {:?}",
                    model_id, caps.chat_models.keys()
                ))
            }
        },
        Err(_) => Err("No caps available".to_string()),
    }?;
    let mut messages_compress = messages.clone();
    messages_compress.push(
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(COMPRESSION_MESSAGE.to_string()),
            ..Default::default()
        },
    );
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        n_ctx,
        1,
        false,
        messages_compress.clone(),
        "".to_string(),
        false,
        model_id.clone(),
    ).await));
    let tools = gather_used_tools(&messages);
    let new_messages = subchat_single(
        ccx.clone(),
        &model_id,
        messages_compress,
        Some(tools),
        None,
        false,
        Some(TEMPERATURE),
        None,
        1,
        None, 
        true,
        None,
        None,
        None,
    ).await.map_err(|e| format!("Error: {}", e))?;

    let content = new_messages
        .into_iter()
        .next()
        .map(|x| {
            x.into_iter().last().map(|last_m| match last_m.content {
                ChatContent::SimpleText(text) => Some(text),
                ChatContent::Multimodal(_) => None,
            })
        })
        .flatten()
        .flatten()
        .ok_or("No traj message was generated".to_string())?;
    let compressed_message = format!("{content}\n\nPlease, continue the conversation based on the provided summary");
    Ok(("".to_string(), compressed_message))
}
