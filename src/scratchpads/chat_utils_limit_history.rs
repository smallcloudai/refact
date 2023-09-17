use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatPost;
use crate::call_validation::ChatMessage;


pub fn limit_messages_history(
    t: &HasTokenizerAndEot,
    post: &ChatPost,
    context_size: usize,
    default_system_mesage: &String,
) -> Result<Vec<ChatMessage>, String>
{
    let tokens_limit: i32 = context_size as i32 - post.parameters.max_new_tokens as i32;
    let mut tokens_used: i32 = 0;
    let mut message_token_count: Vec<i32> = vec![0; post.messages.len()];
    let mut message_take: Vec<bool> = vec![false; post.messages.len()];
    let mut have_system = false;
    for (i, msg) in post.messages.iter().enumerate() {
        let tcnt = (3 + t.count_tokens(msg.content.as_str())?) as i32;  // 3 for role "\n\nASSISTANT:" kind of thing
        message_token_count[i] = tcnt;
        if i==0 && msg.role == "system" {
            message_take[i] = true;
            tokens_used += tcnt;
            have_system = true;
        }
    }
    let need_default_system_msg = !have_system && default_system_mesage.len() > 0;
    if need_default_system_msg {
        let tcnt = t.count_tokens(default_system_mesage.as_str())? as i32;
        tokens_used += tcnt;
    }
    for i in (0..post.messages.len()).rev() {
        let tcnt = message_token_count[i];
        if !message_take[i] {
            if tokens_used + tcnt < tokens_limit {
                message_take[i] = true;
                tokens_used += tcnt;
            }
        }
    }
    let mut messages_out: Vec<ChatMessage> = post.messages.iter().enumerate().filter(|(i, x)| message_take[*i]).map(|(_, x)| x.clone()).collect();
    if need_default_system_msg {
        messages_out.insert(0, ChatMessage {
            role: "system".to_string(),
            content: default_system_mesage.clone(),
        });
    }
    Ok(messages_out)
}
