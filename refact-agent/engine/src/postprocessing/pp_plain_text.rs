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
    tokens_limit: usize,
    style: &Option<String>,
) -> (Vec<ChatMessage>, usize) {
    if plain_text_messages.is_empty() {
        return (vec![], tokens_limit);
    }

    let mut remaining_budget = tokens_limit;
    let mut new_messages = vec![];

    for mut msg in plain_text_messages.into_iter() {
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
                    },
                    ChatContent::ContextFiles(files) => {
                        ChatContent::ContextFiles(files)
                    }
                };
            }
        }

        let per_msg_limit = msg.output_filter.as_ref().and_then(|f| f.limit_tokens);
        msg.output_filter = None;

        let effective_limit = match per_msg_limit {
            Some(msg_limit) => msg_limit.min(remaining_budget),
            None => remaining_budget,
        };

        if effective_limit < 50 {
            msg.content = ChatContent::SimpleText("... truncated (token limit reached)".to_string());
            new_messages.push(msg);
            continue;
        }

        let tokens_used = match msg.content {
            ChatContent::SimpleText(ref text) => {
                let (new_content, used) = limit_text_by_tokens(tokenizer.clone(), text, effective_limit);
                msg.content = ChatContent::SimpleText(new_content);
                used
            },
            ChatContent::Multimodal(ref elements) => {
                let mut new_content = vec![];
                let mut used_in_msg = 0;

                for element in elements {
                    if element.is_text() {
                        let remaining = effective_limit.saturating_sub(used_in_msg);
                        let (new_text, used) = limit_text_by_tokens(tokenizer.clone(), &element.m_content, remaining);
                        used_in_msg += used;
                        new_content.push(MultimodalElement {
                            m_type: element.m_type.clone(),
                            m_content: new_text,
                        });
                    } else if element.is_image() {
                        let tokens = element.count_tokens(None, style).unwrap_or(0) as usize;
                        if used_in_msg + tokens > effective_limit {
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
                msg.content = ChatContent::Multimodal(new_content);
                used_in_msg
            },
            ChatContent::ContextFiles(_) => {
                msg.content.size_estimate(tokenizer.clone(), style)
            }
        };

        remaining_budget = remaining_budget.saturating_sub(tokens_used);
        new_messages.push(msg);
    }

    (new_messages, remaining_budget)
}
