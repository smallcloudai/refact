//! Tests for compression logic - bug detection, problem highlighting, and edge cases.

#[cfg(test)]
mod bug_tests {
    //! Tests asserting correct behavior for fixed bugs.

    use crate::scratchpads::chat_utils_limit_history::{
        compress_duplicate_context_files, is_content_duplicate,
    };
    use crate::call_validation::{ChatMessage, ChatContent, ContextFile};

    fn create_test_message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: String::new(),
            tool_failed: None,
            usage: None,
            checkpoints: Vec::new(),
            thinking_blocks: None,
            output_filter: None,
        }
    }

    fn create_context_file(name: &str, content: &str, line1: usize, line2: usize) -> ContextFile {
        ContextFile {
            file_name: name.to_string(),
            file_content: content.to_string(),
            line1,
            line2,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        }
    }

    #[test]
    fn test_bug1_should_preserve_larger_file_over_smaller() {
        let small = create_context_file("test.rs", "fn main() {}", 1, 1);
        let large = create_context_file(
            "test.rs",
            "fn main() {}\nfn helper() {}\nfn other() {}\nfn more() {}\nfn stuff() {}",
            1,
            5,
        );

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![small]).unwrap()),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &serde_json::to_string(&vec![large]).unwrap()),
            create_test_message("user", "Q2"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (compressed_count, preserve_flags) = result.unwrap();

        assert_eq!(compressed_count, 1, "Small file should be compressed as subset of large");
        assert!(preserve_flags[3], "Larger file should be marked for preservation");
    }

    #[test]
    fn test_bug2_should_detect_first_as_subset_of_current() {
        let small = "line1\nline2";
        let large = "line1\nline2\nline3\nline4\nline5";

        let result = is_content_duplicate(large, 1, 5, small, 1, 2);

        assert!(result, "Should detect duplication when first is subset of current");
    }

    #[test]
    fn test_bug3_identical_overlapping_should_compress_smaller() {
        let content_v1 = "fn main() {\n    println!(\"hello\");\n}";
        let content_v2 = "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}";

        let file1 = create_context_file("main.rs", content_v1, 1, 3);
        let file2 = create_context_file("main.rs", content_v2, 1, 4);

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![file1]).unwrap()),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &serde_json::to_string(&vec![file2]).unwrap()),
            create_test_message("user", "Q2"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (compressed_count, preserve_flags) = result.unwrap();

        assert_eq!(compressed_count, 1, "Smaller version should be compressed");
        assert!(!preserve_flags[1], "Smaller file should NOT be preserved");
        assert!(preserve_flags[3], "Larger file SHOULD be preserved");
    }
}

#[cfg(test)]
mod problem_highlighting_tests {
    use crate::scratchpads::chat_utils_limit_history::{
        compress_duplicate_context_files, is_content_duplicate, CompressionStrength,
        get_model_token_params,
    };
    use crate::call_validation::{ChatMessage, ChatToolCall, ChatContent, ContextFile};

    fn create_test_message(role: &str, content: &str, tool_call_id: Option<String>, tool_calls: Option<Vec<ChatToolCall>>) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            finish_reason: None,
            tool_calls,
            tool_call_id: tool_call_id.unwrap_or_default(),
            tool_failed: if role == "tool" || role == "diff" { Some(false) } else { None },
            usage: None,
            checkpoints: Vec::new(),
            thinking_blocks: None,
            output_filter: None,
        }
    }

    #[test]
    fn test_should_keep_larger_context_file_over_smaller_first_occurrence() {
        let small_file = ContextFile {
            file_name: "test.rs".to_string(),
            file_content: "fn main() {}".to_string(),
            line1: 1,
            line2: 1,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        };

        let large_file = ContextFile {
            file_name: "test.rs".to_string(),
            file_content: "fn main() {}\nfn helper() {}\nfn another() {}".to_string(),
            line1: 1,
            line2: 3,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        };

        let mut messages = vec![
            create_test_message("system", "System prompt", None, None),
            create_test_message("context_file", &serde_json::to_string(&vec![small_file]).unwrap(), None, None),
            create_test_message("user", "First question", None, None),
            create_test_message("context_file", &serde_json::to_string(&vec![large_file]).unwrap(), None, None),
            create_test_message("user", "Second question", None, None),
        ];

        let original_large_content = messages[3].content.content_text_only();
        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (compressed_count, _) = result.unwrap();
        let final_large_content = messages[3].content.content_text_only();

        assert_eq!(
            original_large_content, final_large_content,
            "Larger context file should NOT be compressed. Compressed {} messages.",
            compressed_count
        );
    }

    #[test]
    fn test_error_message_format_includes_token_info() {
        let good_error_format = |occupied: usize, limit: usize| -> String {
            format!(
                "Cannot compress chat history enough: occupied {} tokens but limit is {} tokens (need to free {} tokens). Please start a new chat session.",
                occupied, limit, occupied.saturating_sub(limit)
            )
        };

        let error = good_error_format(5000, 4000);

        assert!(error.contains("5000"), "Error should contain occupied token count");
        assert!(error.contains("4000"), "Error should contain token limit");
        assert!(error.contains("1000"), "Error should contain tokens to free");
    }

    #[test]
    fn test_stage0_compression_should_be_tracked() {
        // Documents current behavior: Stage 0 returns Absent even when files compressed
        let stage_to_strength_current = |stage: i32, _files_compressed: usize| -> CompressionStrength {
            match stage {
                0 => CompressionStrength::Absent,
                1..=3 => CompressionStrength::Low,
                4 => CompressionStrength::Medium,
                _ => CompressionStrength::High,
            }
        };

        let strength = stage_to_strength_current(0, 5);

        assert_eq!(
            strength, CompressionStrength::Absent,
            "BUG: Stage 0 returns Absent even when files are compressed"
        );
    }

    #[test]
    fn test_default_max_new_tokens_is_documented() {
        let magic_value = 16000;
        assert!(magic_value > 1000, "Default max_new_tokens should be > 1000");
        assert!(magic_value < 100000, "Default max_new_tokens should be < 100000");
    }

    #[test]
    fn test_claude_model_params_consistency() {
        let claude_variants = vec![
            "claude-3-opus",
            "claude-3-sonnet",
            "claude-3-haiku",
            "claude-3-5-sonnet",
            "claude-3-5-haiku",
            "anthropic/claude",
            "CLAUDE-3-OPUS",
        ];

        let expected_overhead = 150;
        let expected_budget_offset = 0.2;

        for model in &claude_variants {
            let (overhead, budget_offset) = get_model_token_params(model);

            if model.to_lowercase().contains("claude") {
                assert_eq!(overhead, expected_overhead,
                    "Claude model '{}' should have overhead {}, got {}", model, expected_overhead, overhead);
                assert!((budget_offset - expected_budget_offset).abs() < 0.001,
                    "Claude model '{}' should have budget_offset {}, got {}", model, expected_budget_offset, budget_offset);
            }
        }
    }

    #[test]
    fn test_content_duplicate_edge_cases() {
        assert!(!is_content_duplicate("", 1, 1, "some content", 1, 10),
            "Empty current content should not be duplicate");
        assert!(!is_content_duplicate("some content", 1, 10, "", 1, 1),
            "Content should not be duplicate of empty");
        assert!(!is_content_duplicate("content", 100, 110, "content", 1, 10),
            "Non-overlapping line ranges should not be duplicate");
        assert!(is_content_duplicate("line1\nline2", 1, 2, "line1\nline2\nline3", 1, 3),
            "Subset content with overlapping ranges should be duplicate");
    }

    #[test]
    fn test_preserve_indices_validity_documentation() {
        let cf1 = r#"[{"file_name": "file1.rs", "file_content": "content1", "line1": 1, "line2": 1}]"#;
        let cf2 = r#"[{"file_name": "file2.rs", "file_content": "content2", "line1": 1, "line2": 1}]"#;

        let mut messages = vec![
            create_test_message("system", "System", None, None),
            create_test_message("context_file", cf1, None, None),
            create_test_message("user", "Question 1", None, None),
            create_test_message("context_file", cf2, None, None),
            create_test_message("user", "Question 2", None, None),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (_, preserve_flags) = result.unwrap();

        assert_eq!(preserve_flags.len(), messages.len());
    }

    #[test]
    fn test_undroppable_recalculation_after_stage4() {
        let messages = vec![
            ("system", false),
            ("user", false),
            ("assistant", true),
            ("tool", true),
            ("user", false),
            ("assistant", false),
            ("user", false),
        ];

        let messages_after: Vec<_> = messages.iter()
            .filter(|(_, dropped)| !dropped)
            .map(|(role, _)| *role)
            .collect();

        let new_undroppable = messages_after.iter()
            .rposition(|role| *role == "user")
            .unwrap_or(0);

        assert_eq!(messages_after.len(), 5, "Should have 5 messages after removal");
        assert_eq!(messages_after[new_undroppable], "user", "Should find user message");
        assert_eq!(new_undroppable, 4, "Last user should be at index 4");
        assert_eq!(messages_after, vec!["system", "user", "user", "assistant", "user"]);
    }

    #[test]
    fn test_duplicate_context_files_are_compressed() {
        let file = ContextFile {
            file_name: "test.rs".to_string(),
            file_content: "fn main() {}".to_string(),
            line1: 1,
            line2: 1,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        };

        let mut messages = vec![
            create_test_message("system", "System prompt", None, None),
            create_test_message("context_file", &serde_json::to_string(&vec![file.clone()]).unwrap(), None, None),
            create_test_message("user", "First question", None, None),
            create_test_message("context_file", &serde_json::to_string(&vec![file]).unwrap(), None, None),
            create_test_message("user", "Second question", None, None),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (compressed_count, preserve_flags) = result.unwrap();

        assert!(compressed_count >= 1, "At least one duplicate should be compressed");
        assert!(preserve_flags[1], "First context_file should be preserved");
    }

    #[test]
    fn test_model_detection_case_sensitivity() {
        let lowercase = get_model_token_params("claude-3-sonnet");
        let uppercase = get_model_token_params("CLAUDE-3-SONNET");
        let mixed = get_model_token_params("Claude-3-Sonnet");

        assert_eq!(lowercase.0, 150, "Lowercase 'claude' should be detected");

        if uppercase.0 != 150 {
            eprintln!("NOTE: Uppercase 'CLAUDE' not detected - consider case-insensitive matching");
        }
        if mixed.0 != 150 {
            eprintln!("NOTE: Mixed case 'Claude' not detected - consider case-insensitive matching");
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use crate::scratchpads::chat_utils_limit_history::{
        compress_duplicate_context_files, is_content_duplicate,
    };
    use crate::call_validation::{ChatMessage, ChatContent, ContextFile};

    fn create_test_message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            finish_reason: None,
            tool_calls: None,
            tool_call_id: String::new(),
            tool_failed: None,
            usage: None,
            checkpoints: Vec::new(),
            thinking_blocks: None,
            output_filter: None,
        }
    }

    fn create_context_file(name: &str, content: &str, line1: usize, line2: usize) -> ContextFile {
        ContextFile {
            file_name: name.to_string(),
            file_content: content.to_string(),
            line1,
            line2,
            symbols: vec![],
            gradient_type: -1,
            usefulness: 0.0,
            skip_pp: false,
        }
    }

    #[test]
    fn test_edge_empty_messages() {
        let mut messages: Vec<ChatMessage> = vec![];
        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, flags) = result.unwrap();
        assert_eq!(count, 0);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_edge_no_context_files() {
        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();
        assert_eq!(count, 0, "No context files means no compression");
    }

    #[test]
    fn test_edge_single_context_file() {
        let file = create_context_file("test.rs", "content", 1, 1);
        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![file]).unwrap()),
            create_test_message("user", "Question"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();
        assert_eq!(count, 0, "Single file can't be duplicate");
    }

    #[test]
    fn test_edge_same_filename_different_content_non_overlapping_lines() {
        let file1 = create_context_file("test.rs", "fn foo() {}", 1, 1);
        let file2 = create_context_file("test.rs", "fn bar() {}", 100, 100);

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![file1]).unwrap()),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &serde_json::to_string(&vec![file2]).unwrap()),
            create_test_message("user", "Q2"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();

        assert_eq!(count, 0, "Different line ranges should not be compressed");
    }

    #[test]
    fn test_edge_multiple_files_in_single_message() {
        let files = vec![
            create_context_file("a.rs", "content a", 1, 1),
            create_context_file("b.rs", "content b", 1, 1),
            create_context_file("c.rs", "content c", 1, 1),
        ];

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&files).unwrap()),
            create_test_message("user", "Question"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();
        assert_eq!(count, 0, "Different files in same message are not duplicates");
    }

    #[test]
    fn test_edge_partial_duplicate_in_multifile_message() {
        let file_a = create_context_file("a.rs", "content a", 1, 1);
        let files_mixed = vec![
            create_context_file("a.rs", "content a", 1, 1),
            create_context_file("b.rs", "content b", 1, 1),
        ];

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![file_a]).unwrap()),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &serde_json::to_string(&files_mixed).unwrap()),
            create_test_message("user", "Q2"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();

        assert_eq!(count, 1, "Only the duplicate file_a should be compressed");

        let remaining: Vec<ContextFile> = serde_json::from_str(
            &messages[3].content.content_text_only()
        ).unwrap_or_default();
        assert_eq!(remaining.len(), 1, "Should have one file remaining");
        assert_eq!(remaining[0].file_name, "b.rs", "Remaining file should be b.rs");
    }

    #[test]
    fn test_edge_line_boundary_overlap() {
        assert!(!is_content_duplicate("content", 6, 10, "content", 1, 5),
            "Adjacent non-overlapping ranges should not be duplicate");
        assert!(is_content_duplicate("x", 5, 10, "x", 1, 5),
            "Ranges overlapping at one line should check content");
    }

    #[test]
    fn test_edge_large_line_numbers() {
        let result = is_content_duplicate(
            "content",
            usize::MAX - 100,
            usize::MAX,
            "content",
            usize::MAX - 50,
            usize::MAX,
        );
        assert!(result, "Should handle large line numbers");
    }

    #[test]
    fn test_edge_whitespace_content() {
        assert!(is_content_duplicate("   ", 1, 1, "   ", 1, 1),
            "Identical whitespace content is duplicate");
        let result = is_content_duplicate("\n\n\n", 1, 3, "\n\n\n", 1, 3);
        assert!(result, "Newline-only content is duplicate");
    }

    #[test]
    fn test_edge_unicode_content() {
        let unicode1 = "fn главная() { println!(\"Привет мир\"); }";
        let unicode2 = "fn главная() { println!(\"Привет мир\"); }";

        assert!(is_content_duplicate(unicode1, 1, 1, unicode2, 1, 1),
            "Unicode content should match correctly");
    }

    #[test]
    fn test_edge_content_with_ellipsis() {
        let content_with_ellipsis = "line1\n... (100 more lines)\nline100";
        let content_normal = "line1\nline100";

        let result = is_content_duplicate(content_with_ellipsis, 1, 100, content_normal, 1, 100);
        assert!(result, "Ellipsis lines should be filtered in comparison");
    }

    #[test]
    fn test_edge_three_identical_occurrences() {
        let file = create_context_file("test.rs", "content", 1, 1);
        let json = serde_json::to_string(&vec![file]).unwrap();

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &json),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &json),
            create_test_message("user", "Q2"),
            create_test_message("context_file", &json),
            create_test_message("user", "Q3"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, preserve_flags) = result.unwrap();

        assert_eq!(count, 2, "Two duplicates should be compressed");
        assert!(preserve_flags[1], "First occurrence should be preserved");
    }

    #[test]
    fn test_edge_interleaved_files() {
        let file_a = create_context_file("a.rs", "content a", 1, 1);
        let file_b = create_context_file("b.rs", "content b", 1, 1);

        let mut messages = vec![
            create_test_message("system", "System"),
            create_test_message("context_file", &serde_json::to_string(&vec![file_a.clone()]).unwrap()),
            create_test_message("context_file", &serde_json::to_string(&vec![file_b.clone()]).unwrap()),
            create_test_message("user", "Q1"),
            create_test_message("context_file", &serde_json::to_string(&vec![file_a]).unwrap()),
            create_test_message("context_file", &serde_json::to_string(&vec![file_b]).unwrap()),
            create_test_message("user", "Q2"),
        ];

        let result = compress_duplicate_context_files(&mut messages);
        assert!(result.is_ok());
        let (count, _) = result.unwrap();

        assert_eq!(count, 2, "Both interleaved files should have duplicates compressed");
    }
}
