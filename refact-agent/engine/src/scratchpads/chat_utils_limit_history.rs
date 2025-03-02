use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatMessage;
use std::collections::{HashSet, HashMap};


pub fn limit_messages_history(
    t: &HasTokenizerAndEot,
    messages: &Vec<ChatMessage>,
    last_user_msg_starts: usize,
    max_new_tokens: usize,
    n_ctx: usize,
) -> Result<Vec<ChatMessage>, String> {
    if n_ctx <= max_new_tokens {
        return Err(format!("bad input, n_ctx={}, max_new_tokens={}", n_ctx, max_new_tokens));
    }
    let tokens_limit = (n_ctx - max_new_tokens) as i32;

    let token_counts: Vec<i32> = messages
        .iter()
        .map(|msg| -> Result<i32, String> { Ok(3 + msg.content.count_tokens(t.tokenizer.clone(), &None)?) })
        .collect::<Result<Vec<_>, String>>()?;

    let mut included_indices: Vec<usize> = (last_user_msg_starts..messages.len()).collect();
    let mut tokens_used: i32 = included_indices.iter().map(|&i| token_counts[i]).sum();
    if tokens_used > tokens_limit {
        return Err("User message exceeds token limit :/".to_string());
    }

    if messages.len() > 0 && messages[0].role == "system" {
        included_indices.push(0);
        tokens_used += token_counts[0];
    }

    // Add earlier messages from end to beginning, respecting token limit
    for i in (0..last_user_msg_starts).rev() {
        let msg_tokens = token_counts[i];
        if i==0 && messages[i].role == "system" {
            continue;
        }
        if tokens_used + msg_tokens <= tokens_limit {
            included_indices.push(i);
            tokens_used += msg_tokens;
            tracing::info!("take {:?}, tokens_used={} < {}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30), tokens_used, tokens_limit);
        } else {
            break;
        }
    }

    // Handle tool call dependencies
    let mut tool_call_to_index: HashMap<String, usize> = HashMap::new();
    for (i, msg) in messages.iter().enumerate() {
        if let Some(tool_calls) = &msg.tool_calls {
            for call in tool_calls {
                tool_call_to_index.insert(call.id.clone(), i);
            }
        }
    }

    // Remove tool results if their tool calls aren’t included
    let mut included_set: HashSet<usize> = included_indices.iter().cloned().collect();
    let mut to_remove = Vec::new();
    for &i in &included_indices {
        if let Some(call_index) = tool_call_to_index.get(&messages[i].tool_call_id) {
            if !included_set.contains(call_index) {
                tracing::info!("DROP TOOL RESULT {:?}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30));
                to_remove.push(i);
            }
        }
    }
    for i in to_remove {
        included_set.remove(&i);
        tokens_used -= token_counts[i];
    }

    included_indices = included_set.into_iter().collect();
    included_indices.sort(); // Sort indices to match original order

    let messages_out: Vec<ChatMessage> = included_indices
        .iter()
        .map(|&i| messages[i].clone())
        .collect();

    for (i, msg) in messages.iter().enumerate() {
        if !included_indices.contains(&i) {
            tracing::info!("DROP {:?} with {} tokens", crate::nicer_logs::first_n_chars(&msg.content.content_text_only(), 30), token_counts[i]);
        }
    }

    let messages_out: Vec<ChatMessage> = included_indices
        .iter()
        .map(|&i| messages[i].clone())
        .collect();

    tracing::info!("original {} messages => keep {} messages with {} tokens", messages.len(), messages_out.len(), tokens_used);
    Ok(messages_out)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatContent, ChatToolCall, ChatToolFunction};
    use std::sync::{Arc, RwLock};
    use tracing_subscriber;
    use std::io::stderr;
    use tracing_subscriber::fmt::format;


    impl HasTokenizerAndEot {
        fn mock() -> Arc<Self> {
            use std::sync::RwLock;
            use tokenizers::Tokenizer;
            use tokenizers::models::wordpiece::WordPiece;
            let wordpiece = WordPiece::default();
            let mock_tokenizer = Tokenizer::new(wordpiece);

            Arc::new(Self {
                tokenizer: Arc::new(RwLock::new(mock_tokenizer)),
                eot: "".to_string(),
                eos: "".to_string(),
                context_format: "".to_string(),
                rag_ratio: 0.5,
            })
        }
    }

    fn create_test_message(role: &str, content: &str, tool_call_id: Option<String>, tool_calls: Option<Vec<ChatToolCall>>) -> ChatMessage {
        let tool_call_id_str = tool_call_id.unwrap_or_default();
        ChatMessage {
            role: role.to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            finish_reason: None,
            tool_calls,
            tool_call_id: tool_call_id_str,
            usage: None,
            checkpoints: Vec::new(),
        }
    }

    fn create_mock_chat_history() -> (Vec<ChatMessage>, usize) {
        let x = vec![
            create_test_message("system", "System prompt", None, None),
            create_test_message("user", "First user message", None, None),
            create_test_message("assistant", "First assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool1".to_string(),
                    function: ChatToolFunction {
                        name: "tool1".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "First tool result", Some("tool1".to_string()), None),
            create_test_message("assistant", "Second assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool2".to_string(),
                    function: ChatToolFunction {
                        name: "tool2".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "Second tool result", Some("tool2".to_string()), None),
            create_test_message("user", "Second user message", None, None),
            create_test_message("assistant", "Third assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool3".to_string(),
                    function: ChatToolFunction {
                        name: "tool3".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "Third tool result", Some("tool3".to_string()), None),
            create_test_message("assistant", "Fourth assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool4".to_string(),
                    function: ChatToolFunction {
                        name: "tool4".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "Fourth tool result", Some("tool4".to_string()), None),
            create_test_message("user", "Third user message", None, None),
            create_test_message("user", "Fourth user message", None, None),
        ];

        let last_user_msg_starts = 1 + x.iter().position(|msg| {
            if let ChatContent::SimpleText(text) = &msg.content {
                text == "Third user message"
            } else {
                false
            }
        }).unwrap_or(0);

        (x, last_user_msg_starts)
    }

    fn _msgdump(messages: &Vec<ChatMessage>, title: Option<&str>)
    {
        if let Some(t) = title {
            eprintln!("=== {} ===", t);
        } else {
            eprintln!("=== Message Dump ===");
        }

        for (i, msg) in messages.iter().enumerate() {
            let content = msg.content.content_text_only();
            let tool_call_info = if !msg.tool_call_id.is_empty() {
                format!(" [tool_call_id: {}]", msg.tool_call_id)
            } else {
                String::new()
            };

            let tool_calls_info = if let Some(tool_calls) = &msg.tool_calls {
                format!(" [has {} tool calls]", tool_calls.len())
            } else {
                String::new()
            };

            eprintln!("{:2}: {:10} | {}{}{}",
                i,
                msg.role,
                content.chars().take(50).collect::<String>(),
                if content.len() > 50 { "..." } else { "" },
                format!("{}{}", tool_call_info, tool_calls_info)
            );
        }
        eprintln!("=== End of Message Dump ===");
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_writer(stderr)
            .with_max_level(tracing::Level::INFO)
            .event_format(format::Format::default())
            .try_init();
    }

    #[test]
    fn test_chatlimit_high_enough_to_fit_all() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 100;
        let n_ctx = 1000; // High enough to fit all messages
        let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
        assert!(result.is_ok());
        let limited_messages = result.unwrap();
        _msgdump(&limited_messages, Some("test_chatlimit_doesnt_fit_last_user_message"));
        assert_eq!(limited_messages.len(), messages.len());
        assert_eq!(limited_messages[0].role, "system");
        assert_eq!(limited_messages[limited_messages.len() - 1].role, "user");
    }

    #[test]
    fn test_chatlimit_drop_first_user() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 1;
        let n_ctx = 20; // Limited context size
        let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
        assert!(result.is_ok());
        let limited_messages = result.unwrap();
        _msgdump(&limited_messages, Some("test_chatlimit_doesnt_fit_last_user_message"));
        assert_eq!(limited_messages[0].role, "system");
        // drop first
        let first_user_index = limited_messages.iter().position(|msg| msg.role == "user" && msg.content.content_text_only() == "First user message");
        assert!(first_user_index.is_none());
        // included last
        assert_eq!(limited_messages[limited_messages.len() - 1].content.content_text_only(), "Fourth user message");
    }

    #[test]
    fn test_chatlimit_doesnt_fit_last_user_message() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 1;
        let n_ctx = 20;
        let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
        // eprintln!("test_chatlimit_doesnt_fit_last_user_message:\n{:?}", &result);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "User message exceeds token limit :/");
    }

    #[test]
    fn test_chatlimit_test_a_lot_of_limits() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 5;
        for n_ctx in (100..300).step_by(20) {
            let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
            assert!(result.is_ok());
            let limited_messages = result.unwrap();
            assert_eq!(limited_messages[0].role, "system");
            let first_user_index = limited_messages.iter().position(|msg| msg.role == "user").unwrap_or(limited_messages.len());
            for i in 1..first_user_index {
                assert_eq!(limited_messages[i].role, "system");
            }
            assert_eq!(limited_messages[limited_messages.len() - 1].content.content_text_only(), "Third user message");
            if n_ctx > 120 {
                assert!(limited_messages.len() >= 3);
            }
        }
    }
}
