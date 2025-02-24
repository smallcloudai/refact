use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use crate::agentic::generate_commit_message::remove_fencing;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
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
["thinking", "There are definition(), search() and locate() tools, all can be used to find my_function1, system prompt says I need to start with locate()."],
["locate(problem_statement=\"Rename my_function1 to my_function2\")", "The file my_script.py (1337 lines) has my_function1 on line 42."],
["thinking", "I can rewrite my_function1 inside my_script.py, so I'll do that."],
["update_textdoc(path=\"my_script\", old_str=\"...\", replacement=\"...\", multiple=false)", "The output of update_textdoc() has 15 lines_add and 15 lines_remove, confirming the operation."],
["outcome", "SUCCESS"]
]

Write only the json and nothing else.
"#;
const TEMPERATURE: f32 = 0.0;

fn parse_goal(trajectory: &String) -> Result<String, String> {
    let traj_message_parsed: Vec<(String, String)> = serde_json::from_str(trajectory.as_str())
        .map_err(|e| format!("Error while parsing: {}\nTrajectory:\n{}", e, trajectory))?;
    let (name, content) = traj_message_parsed.first().ok_or("Empty trajectory".to_string())?;
    if name != "goal" {
        Err("Goal should be first item in trajectory".to_string())
    } else {
        Ok(content.clone())
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
    let (model_name, n_ctx) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            let caps_locked = caps.read().unwrap();
            let model_name = caps_locked.code_chat_default_model.clone();
            if let Some(model_rec) = caps_locked.code_completion_models.get(&strip_model_from_finetune(&model_name)) {
                Ok((model_name, model_rec.n_ctx))
            } else {
                Err(format!(
                    "Model '{}' not found. Server has these models: {:?}",
                    model_name, caps_locked.code_completion_models.keys()
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
        model_name.as_str(),
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
    let trajectory = remove_fencing(&content);
    let goal = parse_goal(&trajectory)?;

    Ok((goal, trajectory))
}
