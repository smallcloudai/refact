use std::cmp::min;
use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::{ChatContent, ChatMessage};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tokenizers::Tokenizer;
use crate::scratchpads::multimodality::MultimodalElement;

const MESSAGE_TOKEN_LIMIT: i32 = 12_000;

fn compress_string(text: &String, tokenizer: Arc<RwLock<Tokenizer>>) -> Result<String, String> {
    let tokenizer_lock = tokenizer.read().unwrap();
    let tokens = tokenizer_lock.encode(&**text, false).map_err(|e| e.to_string())?;
    let first_tokens = &tokens.get_ids()[0..(MESSAGE_TOKEN_LIMIT / 2) as usize];
    let last_tokens = &tokens.get_ids()[tokens.len() - (MESSAGE_TOKEN_LIMIT / 2) as usize ..];
    let mut text = tokenizer_lock.decode(first_tokens, false).map_err(|e| e.to_string())?;
    text.push_str("\n...\n");
    text.push_str(&tokenizer_lock.decode(last_tokens, false).map_err(|e| e.to_string())?);
    Ok(text)
}

fn compress_message(msg: &ChatMessage, tokenizer: Arc<RwLock<Tokenizer>>) -> Result<ChatMessage, String> {
    let mut message = msg.clone();
    match message.content.clone() {
        ChatContent::SimpleText(simple_text) => {
            message.content = ChatContent::SimpleText(compress_string(&simple_text, tokenizer.clone())?);
        }
        ChatContent::Multimodal(elements) => {
            let mut new_elements: Vec<MultimodalElement> = vec![];
            for element in elements {
                if element.is_text() {
                    new_elements.push(MultimodalElement::new("text".to_string(), compress_string(&element.m_content, tokenizer.clone())?)?);
                } else {
                    new_elements.push(element.clone());
                }
            }
            message.content = ChatContent::Multimodal(new_elements);
        }
    };
    Ok(message)
}

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
    let mut message_can_be_compressed: Vec<bool> = vec![false; messages.len()];
    let message_roles: Vec<String> = messages.iter().map(|x| x.role.clone()).collect();
    
    for (i, msg) in messages.iter().enumerate() {
        let tcnt = 3 + msg.content.count_tokens(t.tokenizer.clone(), &None)?;
        message_token_count[i] = tcnt;
        if i==0 && msg.role == "system" {
            message_take[i] = true;
            tokens_used += tcnt;
        } else if i==1 && msg.role == "user" {
            // we cannot drop the user message which comes right after the system message according to Anthropic API
            message_take[i] = true;
            tokens_used += min(tcnt, MESSAGE_TOKEN_LIMIT + 3);
        } else if i >= last_user_msg_starts {
            message_take[i] = true;
            tokens_used += min(tcnt, MESSAGE_TOKEN_LIMIT + 3);
        }
    }
    
    // Need to save uncompressed last messages of assistant, tool_calls and user between assistant. It could be patch tool calls
    for i in (0..message_roles.len()).rev() {
        if message_roles[i] == "user" {
            message_can_be_compressed[i] = true;            
        }
    }
    
    let mut log_buffer = Vec::new();
    let mut dropped = false;

    for i in (0..messages.len()).rev() {
        let tcnt = 3 + message_token_count[i];
        if !message_take[i] {
            if message_can_be_compressed[i] && tcnt > MESSAGE_TOKEN_LIMIT + 3 && tokens_used + MESSAGE_TOKEN_LIMIT + 3 < tokens_limit {
                message_take[i] = true;
                tokens_used += MESSAGE_TOKEN_LIMIT + 3;
                log_buffer.push(format!("take compressed {:?}, tokens_used={} < {}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30), tokens_used, tokens_limit));
            } else if tokens_used + tcnt < tokens_limit {
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
    let mut messages_out: Vec<ChatMessage> = Vec::new();
    for i in 0..messages.len() {
        if message_take[i] {
            if message_can_be_compressed[i] && message_token_count[i] > MESSAGE_TOKEN_LIMIT {
                messages_out.push(compress_message(&messages[i], t.tokenizer.clone())?);
            } else { 
                messages_out.push(messages[i].clone());                
            }
        }
    }
    
    Ok(messages_out)
}
