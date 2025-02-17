use serde::Serialize;
use serde::Deserialize;
use regex::Regex;


#[derive(Deserialize, Serialize, Clone)]
pub struct CmdlineOutputFilter {
    #[serde(default = "default_limit_lines")]
    pub limit_lines: usize,
    #[serde(default = "default_limit_chars")]
    pub limit_chars: usize,
    #[serde(default = "default_valuable_top_or_bottom")]
    pub valuable_top_or_bottom: String,
    #[serde(default = "default_grep")]
    pub grep: String,
    #[serde(default = "default_grep_context_lines")]
    pub grep_context_lines: usize,
    #[serde(default = "default_remove_from_output")]
    pub remove_from_output: String,
}

impl Default for CmdlineOutputFilter {
    fn default() -> Self {
        CmdlineOutputFilter {
            limit_lines: default_limit_lines(),
            limit_chars: default_limit_chars(),
            valuable_top_or_bottom: default_valuable_top_or_bottom(),
            grep: default_grep(),
            grep_context_lines: default_grep_context_lines(),
            remove_from_output: default_remove_from_output(),
        }
    }
}

fn default_limit_lines() -> usize {
    100
}

fn default_limit_chars() -> usize {
    10000
}

fn default_valuable_top_or_bottom() -> String {
    "top".to_string()
}

fn default_grep() -> String {
    "(?i)error".to_string()
}

fn default_grep_context_lines() -> usize {
    5
}

fn default_remove_from_output() -> String {
    "".to_string()
}

pub fn output_mini_postprocessing(filter: &CmdlineOutputFilter, output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut ratings: Vec<f64> = vec![0.0; lines.len()];
    let mut approve: Vec<bool> = vec![false; lines.len()];

    if filter.valuable_top_or_bottom == "bottom" {
        for i in 0..lines.len() {
            ratings[i] += 0.9 * ((i + 1) as f64) / lines.len() as f64;
        }
    } else {
        for i in 0..lines.len() {
            ratings[i] += 0.9 * (lines.len() - i) as f64 / lines.len() as f64;
        }
    }

    if !filter.grep.is_empty() {
        let re = Regex::new(&filter.grep).unwrap();
        for (i, line) in lines.iter().enumerate() {
            if re.is_match(line) {
                ratings[i] = 1.0;
                for j in 1..=filter.grep_context_lines {
                    let lower_bound = i.saturating_sub(j);
                    let upper_bound = i + j;
                    if lower_bound < lines.len() {
                        ratings[lower_bound] = 1.0;
                    }
                    if upper_bound < lines.len() {
                        ratings[upper_bound] = 1.0;
                    }
                }
            }
        }
    }

    let mut line_indices: Vec<usize> = (0..lines.len()).collect();
    line_indices.sort_by(|&a, &b| ratings[b].partial_cmp(&ratings[a]).unwrap());

    let mut current_lines = 0;
    let mut current_chars = 0;
    let remove_re = Regex::new(&filter.remove_from_output).unwrap();

    for &index in &line_indices {
        if current_lines > filter.limit_lines || current_chars > filter.limit_chars {
            break;
        }
        if filter.remove_from_output.is_empty() || !remove_re.is_match(lines[index]) {
            if ratings[index] > 0.0 {
                approve[index] = true;
            }
            current_lines += 1;
            current_chars += lines[index].len();
        }
    }

    let mut result = String::new();
    let mut skipped_lines = 0;
    for (i, &line) in lines.iter().enumerate() {
        if approve[i] {
            if skipped_lines > 0 {
                result.push_str(&format!("...{} lines skipped...\n", skipped_lines));
                skipped_lines = 0;
            }
            result.push_str(line);
            result.push('\n');
        } else {
            skipped_lines += 1;
        }
    }
    if skipped_lines > 0 {
        result.push_str(&format!("...{} lines skipped...\n", skipped_lines));
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmdline_output_filter() {
        let output_to_filter = r#"line1
line2
line3
line4
line5
line6
"#;

        let result = output_mini_postprocessing(&CmdlineOutputFilter {
            limit_lines: 2,
            limit_chars: 1000,
            valuable_top_or_bottom: "top".to_string(),
            grep: "".to_string(),
            grep_context_lines: 1,
            remove_from_output: "".to_string(),
        }, output_to_filter);
        assert_eq!(result, "line1\nline2\nline3\n...3 lines skipped...\n");

        let result = output_mini_postprocessing(&CmdlineOutputFilter {
            limit_lines: 2,
            limit_chars: 1000,
            valuable_top_or_bottom: "bottom".to_string(),
            grep: "".to_string(),
            grep_context_lines: 1,
            remove_from_output: "".to_string(),
        }, output_to_filter);
        assert_eq!(result, "...3 lines skipped...\nline4\nline5\nline6\n");

        let result = output_mini_postprocessing(&CmdlineOutputFilter {
            limit_lines: 2,
            limit_chars: 1000,
            valuable_top_or_bottom: "".to_string(),
            grep: "line4".to_string(),
            grep_context_lines: 1,
            remove_from_output: "".to_string(),
        }, output_to_filter);
        assert_eq!(result, "...2 lines skipped...\nline3\nline4\nline5\n...1 lines skipped...\n");

        let result = output_mini_postprocessing(&CmdlineOutputFilter {
            limit_lines: 100,
            limit_chars: 8000,
            valuable_top_or_bottom: "bottom".to_string(),
            ..Default::default()
        }, output_to_filter);
        assert_eq!(result, "line1\nline2\nline3\nline4\nline5\nline6\n");
    }
}

