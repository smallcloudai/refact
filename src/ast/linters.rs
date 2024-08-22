use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::treesitter::language_id::LanguageId;
use crate::files_in_workspace::Document;
use std::collections::HashMap;


fn check_python_indentation(code: &str) -> Vec<String> {
    let mut indent_levels: HashMap<usize, usize> = HashMap::new(); // Tracks the frequency of indent levels
    let mut uses_tabs = false;
    let mut uses_spaces = false;
    let mut last_indent_level = 0;
    let mut line_number = 0;
    let mut problems = Vec::new();
    for line in code.lines() {
        line_number += 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue; // Ignore empty lines and comments
        }

        let indent_level = line.chars().take_while(|&c| c == ' ' || c == '\t').count();

        if line.contains('\t') {
            uses_tabs = true;
        }
        if line.contains(' ') {
            uses_spaces = true;
        }

        *indent_levels.entry(indent_level).or_insert(0) += 1;

        if last_indent_level != 0 && indent_level != last_indent_level && indent_level > last_indent_level {
            let diff = indent_level - last_indent_level;
            if !indent_levels.contains_key(&diff) && diff % last_indent_level != 0 {
                problems.push(format!("Inconsistent indentation at line {}: {}", line_number, line));
            }
        }

        last_indent_level = indent_level;
    }

    if uses_tabs && uses_spaces {
        problems.push("Mixed tabs and spaces detected".to_string());
    }

    problems
}


pub fn lint(doc: &Document) -> Result<(), Vec<String>> {
    let maybe_language_id = get_language_id_by_filename(&doc.path);
    if let Some(language_id) = maybe_language_id {
        let code = doc.text.as_ref().map(|x| x.to_string()).expect("Document text is not available");
        match language_id {
            LanguageId::Python => {
                let mut problems = vec![];
                problems.extend(check_python_indentation(&code));
                if problems.is_empty() { Ok(()) } else { Err(problems) }
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}