use std::sync::Arc;
use tokenizers::Tokenizer;

use crate::call_validation::{ChatContent, ChatMessage};
use crate::scratchpads::multimodality::MultimodalElement;
use crate::tokens::count_text_tokens_with_fallback;
use crate::postprocessing::pp_command_output::output_mini_postprocessing;


fn limit_text_by_tokens(
    tokenizer: Option<Arc<Tokenizer>>,
    text: &str,
    limit_tokens: usize,
) -> (String, usize) {
    let mut new_text_lines = vec![];
    let mut tok_used = 0;
    for line in text.lines() {
        let line_tokens = count_text_tokens_with_fallback(tokenizer.clone(), line);
        if tok_used + line_tokens > limit_tokens {
            if new_text_lines.is_empty() {
                new_text_lines.push("No content: tokens limit reached");
            }
            new_text_lines.push("Truncated: too many tokens\n");
            break;
        }
        tok_used += line_tokens;
        new_text_lines.push(line);
    }
    (new_text_lines.join("\n"), tok_used)
}

pub async fn postprocess_plain_text(
    plain_text_messages: Vec<ChatMessage>,
    tokenizer: Option<Arc<Tokenizer>>,
    _tokens_limit: usize,
    style: &Option<String>,
) -> (Vec<ChatMessage>, usize) {
    if plain_text_messages.is_empty() {
        return (vec![], _tokens_limit);
    }

    let mut tok_used_global = 0;
    let mut new_messages = vec![];

    for mut msg in plain_text_messages.into_iter() {
        let limit_tokens = msg.output_filter.as_ref().and_then(|f| f.limit_tokens);

        // Apply line-based filtering first (if filter exists and has line limits)
        if let Some(ref filter) = msg.output_filter {
            if filter.limit_lines < usize::MAX || filter.limit_chars < usize::MAX || !filter.grep.is_empty() {
                msg.content = match msg.content {
                    ChatContent::SimpleText(text) => {
                        ChatContent::SimpleText(output_mini_postprocessing(filter, &text))
                    },
                    ChatContent::Multimodal(elements) => {
                        let filtered_elements = elements.into_iter().map(|mut el| {
                            if el.is_text() {
                                el.m_content = output_mini_postprocessing(filter, &el.m_content);
                            }
                            el
                        }).collect();
                        ChatContent::Multimodal(filtered_elements)
                    }
                };
            }
        }
        msg.output_filter = None;

        // Apply token-based truncation (if limit_tokens is Some)
        let tok_used = if let Some(tok_limit) = limit_tokens {
            msg.content = match msg.content {
                ChatContent::SimpleText(text) => {
                    let (new_content, used) = limit_text_by_tokens(tokenizer.clone(), &text, tok_limit);
                    tok_used_global += used;
                    ChatContent::SimpleText(new_content)
                },
                ChatContent::Multimodal(elements) => {
                    let mut new_content = vec![];
                    let mut used_in_msg = 0;

                    for element in elements {
                        if element.is_text() {
                            let remaining = tok_limit.saturating_sub(used_in_msg);
                            let (new_text, used) = limit_text_by_tokens(tokenizer.clone(), &element.m_content, remaining);
                            used_in_msg += used;
                            new_content.push(MultimodalElement {
                                m_type: element.m_type,
                                m_content: new_text,
                            });
                        } else if element.is_image() {
                            let tokens = element.count_tokens(None, style).unwrap() as usize;
                            if used_in_msg + tokens > tok_limit {
                                new_content.push(MultimodalElement {
                                    m_type: "text".to_string(),
                                    m_content: "Image truncated: too many tokens".to_string(),
                                });
                            } else {
                                new_content.push(element.clone());
                                used_in_msg += tokens;
                            }
                        }
                    }
                    tok_used_global += used_in_msg;
                    ChatContent::Multimodal(new_content)
                }
            };
            tok_used_global
        } else {
            // No token limit - just count tokens used
            msg.content.size_estimate(tokenizer.clone(), style)
        };
        tok_used_global = tok_used;

        new_messages.push(msg);
    }

    let tok_unused = _tokens_limit.saturating_sub(tok_used_global);
    (new_messages, tok_unused)
}
