use std::sync::Arc;
use tokenizers::Tokenizer;

use crate::call_validation::{ChatContent, ChatMessage};
use crate::scratchpads::multimodality::MultimodalElement;
use crate::tokens::count_text_tokens_with_fallback;


fn limit_text_content(
    tokenizer: Option<Arc<Tokenizer>>,
    text: &String,
    tok_used: &mut usize,
    tok_per_m: usize,
) -> String {
    let mut new_text_lines = vec![];
    for line in text.lines() {
        let line_tokens = count_text_tokens_with_fallback(tokenizer.clone(), &line);
        if tok_used.clone() + line_tokens > tok_per_m {
            if new_text_lines.is_empty() {
                new_text_lines.push("No content: tokens limit reached");
            }
            new_text_lines.push("Truncated: too many tokens\n");
            break;
        }
        *tok_used += line_tokens;
        new_text_lines.push(line);
    }
    new_text_lines.join("\n")
}

pub async fn postprocess_plain_text(
    plain_text_messages: Vec<&ChatMessage>,
    tokenizer: Option<Arc<Tokenizer>>,
    tokens_limit: usize,
    style: &Option<String>,
) -> (Vec<ChatMessage>, usize) {
    if plain_text_messages.is_empty() {
        return (vec![], tokens_limit);
    }
    let mut messages_sorted = plain_text_messages.clone();
    messages_sorted.sort_by(|a, b| a.content.size_estimate(tokenizer.clone(), style).cmp(&b.content.size_estimate(tokenizer.clone(), style)));

    let mut tok_used_global = 0;
    let mut tok_per_m = tokens_limit / messages_sorted.len();
    let mut new_messages = vec![];

    for (idx, msg) in messages_sorted.iter().cloned().enumerate() {
        let mut tok_used = 0;
        let mut m_cloned = msg.clone();
        
        m_cloned.content = match &msg.content {
            ChatContent::SimpleText(text) => {
                let new_content = limit_text_content(tokenizer.clone(), text, &mut tok_used, tok_per_m);
                ChatContent::SimpleText(new_content)
            },
            ChatContent::Multimodal(elements) => {
                let mut new_content = vec![];
                
                for element in elements {
                    if element.is_text() {
                        let mut el_cloned = element.clone();
                        el_cloned.m_content = limit_text_content(tokenizer.clone(), &el_cloned.m_content, &mut tok_used, tok_per_m);
                        new_content.push(el_cloned)
                    } else if element.is_image() {
                        let tokens = element.count_tokens(None, style).unwrap() as usize;
                        if tok_used + tokens > tok_per_m {
                            let new_el = MultimodalElement {
                                m_type: "text".to_string(),
                                m_content: "Image truncated: too many tokens".to_string(),
                            };
                            new_content.push(new_el);
                        } else {
                            new_content.push(element.clone());
                            tok_used += tokens;
                        }
                    }
                }
                ChatContent::Multimodal(new_content)
            }
        };

        if idx != messages_sorted.len() - 1 {
            // distributing non-used rest of tokens among the others
            tok_per_m += (tok_per_m - tok_used) / (messages_sorted.len() - idx - 1);
        }
        tok_used_global += tok_used;

        new_messages.push(m_cloned);
    }

    let tok_unused = tokens_limit.saturating_sub(tok_used_global);
    (new_messages, tok_unused)
}
