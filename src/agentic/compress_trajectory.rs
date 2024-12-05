use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use crate::agentic::generate_commit_message::remove_fencing;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;

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
5. Each 📍-ticket should become a separate record, starts with "coding". Start with 📍REWRITE_ONE_SYMBOL, 📍REWRITE_WHOLE_FILE, 📍PARTIAL_EDIT, 📍NEW_FILE, 📍OTHER and
the three digit ticket number, summarize what the assistant wrote, give some stats, how is the new code different.
6. Skip unsuccesful calls that are later corrected. Keep the corrected one.
7. When writing paths to files, only output short relative paths from the project dir.
8. The last line is the outcome, pick SUCCESS/FAIL/PROGRESS

Output format is list of tuples, each tuple is has:
EITHER (1) call with all parameters, maybe shortened, but all parameters, (2) explanation of significance of tool output
OR     (1) goal/thinking/coding/outcome (2) string according to the guidelines

Example:

[
["goal", "Rename my_function1 to my_function2"],
["thinking", "There are definition(), search() and locate() tools, all can be used to find my_function1, system prompt says I need to start with locate()."],
["locate(problem_statement=\"Rename my_function1 to my_function2\")", "The file my_script.py (1337 lines) has my_function1 on line 42."],
["thinking", "I can rewrite my_function1 inside my_script.py using 📍-notation, so I'll do that."],
["coding", "📍REWRITE_ONE_SYMBOL 000 wrote my_function1 replacement, in my new version the name is my_function2."],
["patch(path=\"my_script\", tickets=\"000\")", "The output of patch() has 15 lines_add and 15 lines_remove, confirming the operation."],
["outcome", "SUCCESS"]
]

Write only the json and nothing else.
"#;
// TODO: N_CTX, probably from model max?
const N_CTX: usize = 32000;
const TEMPERATURE: f32 = 0.3;

pub async fn compress_trajectory(
    gcx: Arc<ARwLock<GlobalContext>>,
    messages: &Vec<ChatMessage>,
) -> Result<String, String> {
    if messages.is_empty() {
        return Err("The provided chat is empty".to_string());
    }
    let model_name = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => caps
            .read()
            .map(|x| Ok(x.code_chat_default_model.clone()))
            .map_err(|_| "Caps are not available".to_string())?,
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
            N_CTX,
            1,
            false,
            messages_compress.clone(),
            "".to_string(),
            false,
        ).await,
    ));
    let new_messages = subchat_single(
        ccx.clone(),
        model_name.as_str(),
        messages_compress,
        vec![],
        None,
        false,
        Some(TEMPERATURE),
        None,
        1,
        None,
        None,
        None,
    ).await.map_err(|e| format!("Error: {}", e))?;

    let commit_message = new_messages
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
        .ok_or("No commit message was generated".to_string())?;

    Ok(remove_fencing(&commit_message))
}
