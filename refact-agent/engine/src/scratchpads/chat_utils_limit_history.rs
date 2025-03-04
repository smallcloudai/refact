use crate::scratchpad_abstract::HasTokenizerAndEot;
use crate::call_validation::ChatMessage;
use std::collections::{HashSet, HashMap};

fn _user_role(m: &ChatMessage) -> bool
{
    m.role == "user" || m.role == "context_file" || m.role == "plain_text" || m.role == "cd_instruction"
}

fn _check_invariant(messages: &Vec<ChatMessage>) -> Result<(), String> {
    if messages.len() == 0 {
        return Ok(());
    }
    if messages.len() > 0 && messages[0].role == "system" {
        if messages.len() == 1 {
            return Ok(());
        }
        if _user_role(&messages[1]) {
            return Ok(());
        }
    }
    let mut err_text = String::new();
    for msg in messages.iter() {
        err_text.push_str(format!("{}/", msg.role).as_str());
    }
    err_text = format!("invariant doesn't hold: {}", err_text);
    return Err(err_text);
}

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
    if let Err(e) = _check_invariant(messages) {
        tracing::error!("input problem: {}", e);
    }
    let tokens_limit = (n_ctx - max_new_tokens) as i32;

    let token_counts: Vec<i32> = messages
        .iter()
        .map(|msg| -> Result<i32, String> { Ok(3 + msg.content.count_tokens(t.tokenizer.clone(), &None)?) })
        .collect::<Result<Vec<_>, String>>()?;

    // Always include messages from last_user_msg_starts to the end
    let mut included_indices: Vec<usize> = (last_user_msg_starts..messages.len()).collect();
    let mut tokens_used: i32 = included_indices.iter().map(|&i| token_counts[i]).sum();
    if tokens_used > tokens_limit {
        return Err("User message exceeds token limit :/".to_string());
    }

    if messages.len() > 0 && messages[0].role == "system" {
        included_indices.push(0);
        tokens_used += token_counts[0];
    }

    // Find the most recent complete conversation blocks that fit within the token limit
    // A complete block is: user -> assistant -> [tool -> assistant ->]*
    let mut user_message_indices: Vec<usize> = Vec::new();
    for i in (1 .. last_user_msg_starts).rev() {
        if _user_role(&messages[i]) {
            user_message_indices.push(i);
        }
    }

    for &user_idx in &user_message_indices {
        let next_user_idx = user_message_indices.iter().filter(|&&idx| idx > user_idx).min().copied().unwrap_or(last_user_msg_starts);
        let block_indices: Vec<usize> = (user_idx .. next_user_idx).collect();
        let block_tokens: i32 = block_indices.iter().map(|&i| token_counts[i]).sum();
        if tokens_used + block_tokens <= tokens_limit {
            for idx in block_indices {
                included_indices.push(idx);
                tokens_used += token_counts[idx];
                tracing::info!("take {:?}, tokens_used={} < {}", crate::nicer_logs::first_n_chars(&messages[idx].content.content_text_only(), 30), tokens_used, tokens_limit);
            }
        } else {
            // If this block doesn't fit, earlier blocks won't fit either
            break;
        }
    }

    let mut tool_call_to_index: HashMap<String, usize> = HashMap::new();
    for (i, msg) in messages.iter().enumerate() {
        if let Some(tool_calls) = &msg.tool_calls {
            for call in tool_calls {
                tool_call_to_index.insert(call.id.clone(), i);
            }
        }
    }

    // Remove tool results if their tool calls aren't included
    let mut included_set: HashSet<usize> = included_indices.iter().cloned().collect();
    let mut to_remove = Vec::new();
    for &i in &included_indices {
        if !messages[i].tool_call_id.is_empty() {
            if let Some(call_index) = tool_call_to_index.get(&messages[i].tool_call_id) {
                if !included_set.contains(call_index) {
                    tracing::info!("DROP TOOL RESULT {:?}", crate::nicer_logs::first_n_chars(&messages[i].content.content_text_only(), 30));
                    to_remove.push(i);
                }
            }
        }
    }
    for i in to_remove {
        included_set.remove(&i);
        tokens_used -= token_counts[i];
    }

    included_indices = included_set.into_iter().collect();
    included_indices.sort(); // Sort indices to match original order

    // Log dropped messages
    for (i, msg) in messages.iter().enumerate() {
        if !included_indices.contains(&i) {
            tracing::info!("DROP {:?} with {} tokens", crate::nicer_logs::first_n_chars(&msg.content.content_text_only(), 30), token_counts[i]);
        }
    }

    let messages_out: Vec<ChatMessage> = included_indices
        .iter()
        .map(|&i| messages[i].clone())
        .collect();

    _check_invariant(&messages_out)?;
    tracing::info!("original {} messages => keep {} messages with {} tokens", messages.len(), messages_out.len(), tokens_used);
    Ok(messages_out)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatContent, ChatToolCall, ChatToolFunction};
    use std::sync::Arc;
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
            create_test_message("user", "block 1 user message", None, None),
            create_test_message("assistant", "block 1 assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool1".to_string(),
                    function: ChatToolFunction {
                        name: "tool1".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "block 1 tool result", Some("tool1".to_string()), None),
            create_test_message("assistant", "block 1 another assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool2".to_string(),
                    function: ChatToolFunction {
                        name: "tool2".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "block 1 another tool result", Some("tool2".to_string()), None),
            create_test_message("user", "block 2 user message", None, None),
            create_test_message("assistant", "block 2 assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool3".to_string(),
                    function: ChatToolFunction {
                        name: "tool3".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "block 2 tool result", Some("tool3".to_string()), None),
            create_test_message("assistant", "block 2 assistant response", None, Some(vec![
                ChatToolCall {
                    id: "tool4".to_string(),
                    function: ChatToolFunction {
                        name: "tool4".to_string(),
                        arguments: "{}".to_string()
                    },
                    tool_type: "function".to_string()
                }
            ])),
            create_test_message("tool", "block 2 another tool result", Some("tool4".to_string()), None),
            create_test_message("user", "block 3 user message A", None, None),
            create_test_message("user", "block 3 user message B", None, None),
        ];

        let last_user_msg_starts = x.iter().position(|msg| {
            if let ChatContent::SimpleText(text) = &msg.content {
                text == "block 3 user message A"
            } else {
                false
            }
        }).unwrap() + 1;   // note + 1

        (x, last_user_msg_starts)
    }

    fn _msgdump(messages: &Vec<ChatMessage>, title: String) -> String
    {
        let mut output = format!("=== {} ===\n", title);
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
            output.push_str(&format!("{:2}: {:10} | {}{}{}\n",
                i,
                msg.role,
                content.chars().take(50).collect::<String>(),
                if content.len() > 50 { "..." } else { "" },
                format!("{}{}", tool_call_info, tool_calls_info)
            ));
        }
        output
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_writer(stderr)
            .with_max_level(tracing::Level::INFO)
            .event_format(format::Format::default())
            .try_init();
    }

    #[test]
    fn test_chatlimit_test_a_lot_of_limits() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 5;
        for n_ctx in (10..=50).step_by(10) {
            let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
            let title = format!("n_ctx={}", n_ctx);
            if result.is_err() {
                eprintln!("{} => {}", title, result.clone().err().unwrap());
                continue;
            }
            let dump = _msgdump(&result.unwrap(), title);
            eprintln!("{}", dump);
        }
    }

    #[test]
    fn test_chatlimit_exact_outputs() {
        init_tracing();
        let (messages, last_user_msg_starts) = create_mock_chat_history();
        let max_new_tokens = 5;

        let expected_dumps = [
            "\
=== n_ctx=10 ===
 0: system     | System prompt
 1: user       | block 3 user message B
",
            "\
=== n_ctx=20 ===
 0: system     | System prompt
 1: user       | block 3 user message A
 2: user       | block 3 user message B
",
            "\
=== n_ctx=30 ===
 0: system     | System prompt
 1: user       | block 2 user message
 2: assistant  | block 2 assistant response [has 1 tool calls]
 3: tool       | block 2 tool result [tool_call_id: tool3]
 4: assistant  | block 2 assistant response [has 1 tool calls]
 5: tool       | block 2 another tool result [tool_call_id: tool4]
 6: user       | block 3 user message A
 7: user       | block 3 user message B
",
            "\
=== n_ctx=40 ===
 0: system     | System prompt
 1: user       | block 2 user message
 2: assistant  | block 2 assistant response [has 1 tool calls]
 3: tool       | block 2 tool result [tool_call_id: tool3]
 4: assistant  | block 2 assistant response [has 1 tool calls]
 5: tool       | block 2 another tool result [tool_call_id: tool4]
 6: user       | block 3 user message A
 7: user       | block 3 user message B
",
            "\
=== n_ctx=50 ===
 0: system     | System prompt
 1: user       | block 1 user message
 2: assistant  | block 1 assistant response [has 1 tool calls]
 3: tool       | block 1 tool result [tool_call_id: tool1]
 4: assistant  | block 1 another assistant response [has 1 tool calls]
 5: tool       | block 1 another tool result [tool_call_id: tool2]
 6: user       | block 2 user message
 7: assistant  | block 2 assistant response [has 1 tool calls]
 8: tool       | block 2 tool result [tool_call_id: tool3]
 9: assistant  | block 2 assistant response [has 1 tool calls]
10: tool       | block 2 another tool result [tool_call_id: tool4]
11: user       | block 3 user message A
12: user       | block 3 user message B
"
        ];
        for (i, n_ctx) in (10..=50).step_by(10).enumerate() {
            let result = limit_messages_history(&HasTokenizerAndEot::mock(), &messages, last_user_msg_starts, max_new_tokens, n_ctx);
            assert!(result.is_ok(), "Failed for n_ctx={}: {:?}", n_ctx, result.err());
            let dump = _msgdump(&result.unwrap(), format!("n_ctx={}", n_ctx));
            assert_eq!(dump, expected_dumps[i], "Output mismatch for n_ctx={}", n_ctx);
        }
    }

    #[test]
    fn test_chatlimit_invalid_sequence() {
        init_tracing();
        let messages = vec![
            create_test_message("system", "System prompt", None, None),
            create_test_message("globglogabgalab", "Strange message", None, None),
            create_test_message("user", "User message", None, None),
        ];
        let last_user_msg_starts = 2; // Index of the "user" message
        let max_new_tokens = 5;
        let n_ctx = 20;

        let result = limit_messages_history(
            &HasTokenizerAndEot::mock(),
            &messages,
            last_user_msg_starts,
            max_new_tokens,
            n_ctx,
        );

        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let output = result.unwrap();

        let dump = _msgdump(&output, format!("n_ctx={}", n_ctx));
        tracing::info!("{}", dump);

        assert_eq!(output.len(), 2, "Expected 2 messages, got {}", output.len());
        assert_eq!(output[0].role, "system", "First message should be 'system'");
        assert_eq!(output[1].role, "user", "Second message should be 'user'");

        if let ChatContent::SimpleText(text) = &output[0].content {
            assert_eq!(text, "System prompt", "System message content mismatch");
        } else {
            panic!("Expected SimpleText for system message");
        }
        if let ChatContent::SimpleText(text) = &output[1].content {
            assert_eq!(text, "User message", "User message content mismatch");
        } else {
            panic!("Expected SimpleText for user message");
        }
    }
}
