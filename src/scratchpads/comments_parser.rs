enum ParserState {
    Normal,
    InSingleLineComment,
    InMultiLineComment { end_delimiter: &'static str },
}

struct CommentSyntax {
    single_line: Option<&'static str>,
    multi_line: Option<Vec<(&'static str, &'static str)>>,
}

fn get_comment_syntax(extension: &str) -> Option<CommentSyntax> {
    match extension {
        // Languages with C-style comments
        "c" | "cpp" | "h" | "hpp" | "java" | "js" | "cs" | "swift" | "kt" | "rs" => Some(CommentSyntax {
            single_line: Some("//"),
            multi_line: Some(vec![("/*", "*/")]),
        }),
        // Python with triple-quoted strings as multi-line comments
        "py" => Some(CommentSyntax {
            single_line: Some("#"),
            multi_line: Some(vec![("'''", "'''"), ("\"\"\"", "\"\"\"")]),
        }),
        // Languages with hash (#) comments but no multi-line comments
        "sh" | "rb" | "pl" | "yaml" | "yml" => Some(CommentSyntax {
            single_line: Some("#"),
            multi_line: None,
        }),
        // HTML and XML comments
        "html" | "xml" => Some(CommentSyntax {
            single_line: None,
            multi_line: Some(vec![("<!--", "-->")]),
        }),
        // Haskell comments
        "hs" => Some(CommentSyntax {
            single_line: Some("--"),
            multi_line: Some(vec![("{-", "-}")]),
        }),
        _ => None, // Unsupported extension
    }
}

fn matches_at(chars: &[char], pos: usize, pattern: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let len = pattern_chars.len();

    if pos + len > chars.len() {
        return false;
    }

    for i in 0..len {
        if chars[pos + i] != pattern_chars[i] {
            return false;
        }
    }
    true
}

pub struct Comment {
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_inline: bool,
}

pub fn parse_comments(text: &str, extension: &str) -> Vec<Comment> {
    let syntax = match get_comment_syntax(extension) {
        Some(s) => s,
        None => return Vec::new(), // Unsupported language
    };

    let mut comments = Vec::new();
    let mut current_comment = String::new();
    let mut start_line = 1;
    let mut end_line = 1;
    let mut is_inline = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let len = chars.len();

    let mut state = ParserState::Normal;
    let mut line_number = 1;
    let mut code_on_line = false;

    while i < len {
        match state {
            ParserState::Normal => {
                // Check for multi-line comment start
                if let Some(multi_line_vec) = &syntax.multi_line {
                    let mut found = false;
                    for (start_delimiter, end_delimiter) in multi_line_vec.iter() {
                        if matches_at(&chars, i, start_delimiter) {
                            // Determine if the comment is inline
                            is_inline = code_on_line;
                            current_comment.push_str(start_delimiter);
                            i += start_delimiter.len();
                            start_line = line_number;
                            state = ParserState::InMultiLineComment {
                                end_delimiter: *end_delimiter,
                            };
                            found = true;
                            break;
                        }
                    }
                    if found {
                        continue;
                    }
                }
                // Check for single-line comment start
                if let Some(single_line) = syntax.single_line {
                    if matches_at(&chars, i, single_line) {
                        // Determine if the comment is inline
                        is_inline = code_on_line;
                        current_comment.push_str(single_line);
                        i += single_line.len();
                        start_line = line_number;
                        state = ParserState::InSingleLineComment;
                        continue;
                    }
                }
                // Update code_on_line
                if chars[i] == '\n' {
                    code_on_line = false;
                    line_number += 1;
                } else if !chars[i].is_whitespace() {
                    code_on_line = true;
                }
                i += 1; // Move to the next character
            }
            ParserState::InSingleLineComment => {
                if chars[i] == '\n' {
                    current_comment.push('\n');
                    end_line = line_number;
                    comments.push(Comment {
                        text: current_comment.clone(),
                        start_line,
                        end_line,
                        is_inline,
                    });
                    current_comment.clear();
                    state = ParserState::Normal;
                    code_on_line = false;
                    line_number += 1;
                } else {
                    current_comment.push(chars[i]);
                }
                i += 1;
            }
            ParserState::InMultiLineComment { end_delimiter } => {
                if matches_at(&chars, i, end_delimiter) {
                    current_comment.push_str(end_delimiter);
                    i += end_delimiter.len();
                    end_line = line_number;
                    comments.push(Comment {
                        text: current_comment.clone(),
                        start_line,
                        end_line,
                        is_inline,
                    });
                    current_comment.clear();
                    state = ParserState::Normal;
                    continue;
                }
                if chars[i] == '\n' {
                    current_comment.push('\n');
                    line_number += 1;
                } else {
                    current_comment.push(chars[i]);
                }
                i += 1;
            }
        }
    }

    // Add any remaining comment
    if !current_comment.is_empty() {
        end_line = line_number;
        comments.push(Comment {
            text: current_comment,
            start_line,
            end_line,
            is_inline,
        });
    }

    comments
}
