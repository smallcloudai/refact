use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatMessage;
use std::collections::HashSet;


pub fn limit_messages_history(
    t: &HasTokenizerAndEot,
    messages: &Vec<ChatMessage>,
    last_user_msg_starts: usize,
    max_new_tokens: usize,
    context_size: usize,
) -> Result<Vec<ChatMessage>, String>
{
    let tokens_limit: i32 = context_size as i32 - max_new_tokens as i32;
    tracing::info!("limit_messages_history tokens_limit={} because context_size={} and max_new_tokens={}", tokens_limit, context_size, max_new_tokens);
    let mut tokens_used: i32 = 0;
    let mut message_token_count: Vec<i32> = vec![0; messages.len()];
    let mut message_take: Vec<bool> = vec![false; messages.len()];
    for (i, msg) in messages.iter().enumerate() {
        let tcnt = 3 + msg.content.count_tokens(t.tokenizer.clone(), &None)?;
        message_token_count[i] = tcnt;
        if i==0 && msg.role == "system" {
            message_take[i] = true;
            tokens_used += tcnt;
        } else if i==1 && msg.role == "user" {
            // we cannot drop the user message which comes right after the system message according to Antropic API
            message_take[i] = true;
            tokens_used += tcnt;
        } else if i >= last_user_msg_starts {
            message_take[i] = true;
            tokens_used += tcnt;
        }
    }
    let mut log_buffer = Vec::new();
    let mut dropped = false;

    for i in (0..messages.len()).rev() {
        let tcnt = 3 + message_token_count[i];
        if !message_take[i] {
            if tokens_used + tcnt < tokens_limit {
                message_take[i] = true;
                tokens_used += tcnt;
                log_buffer.push(format!("take {:?}, tokens_used={} < {}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30), tokens_used, tokens_limit));
            } else {
                log_buffer.push(format!("DROP {:?} with {} tokens, quit", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30), tcnt));
                dropped = true;
                break;
            }
        } else {
            message_take[i] = true;
            log_buffer.push(format!("not allowed to drop {:?}, tokens_used={} < {}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30), tokens_used, tokens_limit));
        }
    }

    if dropped {
        tracing::info!("\n{}", log_buffer.join("\n"));
    }

    // additinally, drop tool results if we drop the calls
    let mut tool_call_id_drop = HashSet::new();
    for i in 0..messages.len() {
        if message_take[i] {
            continue;
        }
        if let Some(tool_calls) = &messages[i].tool_calls {
            for call in tool_calls {
                tool_call_id_drop.insert(call.id.clone());
            }
        }
    }
    for i in 0..messages.len() {
        if !message_take[i] {
            continue;
        }
        if tool_call_id_drop.contains(messages[i].tool_call_id.as_str()) {
            message_take[i] = false;
            tracing::info!("drop {:?} because of drop tool result rule", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30));
        }
    }

    let messages_out: Vec<ChatMessage> = messages.iter().enumerate().filter(|(i, _)| message_take[*i]).map(|(_, x)| x.clone()).collect();
    Ok(messages_out)
}
