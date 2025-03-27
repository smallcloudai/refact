use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use crate::agentic::generate_commit_message::remove_fencing;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::caps::strip_model_from_finetune;

const COMPRESSION_MESSAGE: &str = r#"
Compress the chat above.

Guidelines:

1. Always prefer specifics over generic phrases. Write file names, symbol names, folder names, actions, facts, user attitude
towards entities in the project. If something is junk according to the user, that's the first priority to remember.
2. The first message in the chat is the goal. Summarize it up to 15 words, always prefer specifics.
3. The most important part is decision making by assistant. What new information assistant has learned? Skip the plans,
fluff, explanations for the user. Write one sentense: the evidence (specifics and facts), the thought process, motivated decision.
4. Each tool call should be a separate record. Write all the parameters. Summarize facts about output of a tool, especially the facts
useful for the goal, what the assistant learned, what was surprising to see?
5. Skip unsuccesful calls that are later corrected. Keep the corrected one.
6. When writing paths to files, only output short relative paths from the project dir.
7. The last line is the outcome, pick SUCCESS/FAIL/PROGRESS

Output format is list of tuples, each tuple is has:
EITHER (1) call with all parameters, maybe shortened, but all parameters, (2) explanation of significance of tool output
OR     (1) goal/thinking/coding/outcome (2) string according to the guidelines

Example:
[
["goal", "Rename my_function1 to my_function2"],
["thinking", "There are definition(), search(), regex_search() and locate() tools, all can be used to find my_function1, system prompt says I need to start with locate()."],
["locate(problem_statement=\"Rename my_function1 to my_function2\")", "The file my_script.py (1337 lines) has my_function1 on line 42."],
["thinking", "I can rewrite my_function1 inside my_script.py, so I'll do that."],
["update_textdoc(path=\"my_script\", old_str=\"...\", replacement=\"...\", multiple=false)", "The output of update_textdoc() has 15 lines_add and 15 lines_remove, confirming the operation."],
["outcome", "SUCCESS"]
]

Write only the json and nothing else.
"#;
const TEMPERATURE: f32 = 0.0;

fn parse_goal(trajectory: &String) -> Option<String> {
    let traj_message_parsed: Vec<(String, String)> = match serde_json::from_str(trajectory.as_str()) {
        Ok(data) => data,
        Err(e) => {
            warn!("Error while parsing: {}\nTrajectory:\n{}", e, trajectory);
            return None;
        }
    };
    let (name, content) = match traj_message_parsed.first() {
        Some(data) => data,
        None => {
            warn!("Empty trajectory:\n{}", trajectory);
            return None;
        }
    };
    if name != "goal" {
        warn!("Trajectory does not have a goal message");
        None
    } else {
        Some(content.clone())
    }
}

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
            let model_id = caps.default_models.chat_model.clone();
            if let Some(model_rec) = caps.completion_models.get(&strip_model_from_finetune(&model_id)) {
                Ok((model_id, model_rec.base.n_ctx))
            } else {
                Err(format!(
                    "Model '{}' not found, server has these models: {:?}",
                    model_id, caps.completion_models.keys()
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
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(
        AtCommandsContext::new(
            gcx.clone(),
            n_ctx,
            1,
            false,
            messages_compress.clone(),
            "".to_string(),
            false,
        ).await,
    ));
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
    let code_blocks = remove_fencing(&content);
    let trajectory = if !code_blocks.is_empty() {
        code_blocks[0].clone()
    } else {
        content.clone()
    };
    let goal = parse_goal(&trajectory).unwrap_or("".to_string());
    Ok((goal, trajectory))
}
