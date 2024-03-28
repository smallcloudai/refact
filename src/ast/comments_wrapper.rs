use std::path::PathBuf;

use crate::ast::treesitter::language_id::LanguageId;

pub fn get_language_id_by_filename(filename: &PathBuf) -> Option<LanguageId> {
    let suffix = filename.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match suffix.as_str() {
        "cpp" | "cc" | "cxx" | "c++" | "c" | "h" | "hpp" | "hxx" | "hh" => Some(LanguageId::Cpp),
        "inl" | "inc" | "tpp" | "tpl" => Some(LanguageId::Cpp),
        "py" | "py3" | "pyx" => Some(LanguageId::Python),
        "java" => Some(LanguageId::Java),
        "js" | "jsx" => Some(LanguageId::JavaScript),
        "rs" => Some(LanguageId::Rust),
        "ts" => Some(LanguageId::TypeScript),
        "tsx" => Some(LanguageId::TypeScriptReact),
        _ => None
    }
}


pub fn wrap_comments(code: &str, language: &LanguageId) -> String {
    match language {
        LanguageId::Html => code.lines()
            .map(|line| format!("<!-- {} -->", line))
            .collect::<Vec<String>>()
            .join("\n"),
        _ => {
            let comment_prefix = match language {
                LanguageId::Python | LanguageId::Ruby | LanguageId::R | LanguageId::Bash => "#",
                LanguageId::Cpp | LanguageId::C | LanguageId::CSharp | LanguageId::Java |
                LanguageId::JavaScript | LanguageId::Kotlin | LanguageId::Rust | LanguageId::Scala |
                LanguageId::Swift | LanguageId::TypeScript | LanguageId::TypeScriptReact | LanguageId::Go => "//",
                LanguageId::Css => "/*", // For CSS, you might want to adjust to block comments or handle line by line
                LanguageId::Php | LanguageId::Sql => "--",
                LanguageId::D | LanguageId::Lua => "--",
                LanguageId::Elm | LanguageId::Ocaml => "//", // Adjust accordingly
                LanguageId::Apex => "//", // Assuming Apex is similar to Java
                LanguageId::Unknown => "//", // Default case
                _ => "//", // Default for other languages not explicitly handled
            };

            code.lines()
                .map(|line| format!("{} {}", comment_prefix, line))
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}
