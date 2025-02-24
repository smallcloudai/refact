use std::fmt;

use serde::{Deserialize, Serialize};
use tree_sitter::Language;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LanguageId {
    Apex,
    Bash,
    C,
    Cpp,
    CSharp,
    Css,
    D,
    Elm,
    // Elixir,
    // Erlang,
    Go,
    Html,
    Kotlin,
    Java,
    JavaScript,
    // Json,
    Lua,
    Ocaml,
    Php,
    // Markdown,
    // ObjectiveC,
    Python,
    R,
    Ruby,
    Rust,
    Scala,
    // Solidity,
    Sql,
    Swift,
    // Toml,
    TypeScript,
    TypeScriptReact,
    // Vue,
    Unknown,
}

impl Default for LanguageId {
    fn default() -> Self {
        LanguageId::Unknown
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Apex => write!(f, "apex"),
            Self::Bash => write!(f, "shellscript"),
            Self::C => write!(f, "c"),
            Self::Cpp => write!(f, "cpp"),
            Self::Css => write!(f, "css"),
            Self::CSharp => write!(f, "csharp"),
            Self::D => write!(f, "d"),
            Self::Elm => write!(f, "elm"),
            // Self::Elixir => write!(f, "elixir"),
            // Self::Erlang => write!(f, "erlang"),
            Self::Go => write!(f, "go"),
            Self::Html => write!(f, "html"),
            Self::Kotlin => write!(f, "kotlin"),
            Self::Java => write!(f, "java"),
            Self::JavaScript => write!(f, "javascript"),
            // Self::Json => write!(f, "json"),
            Self::Lua => write!(f, "lua"),
            Self::Ocaml => write!(f, "ocaml"),
            Self::Php => write!(f, "php"),
            // Self::Markdown => write!(f, "markdown"),
            // Self::ObjectiveC => write!(f, "objective-c"),
            Self::Python => write!(f, "python"),
            Self::R => write!(f, "r"),
            Self::Ruby => write!(f, "ruby"),
            Self::Rust => write!(f, "rust"),
            Self::Scala => write!(f, "scala"),
            // Self::Solidity => write!(f, "solidity"),
            Self::Sql => write!(f, "sql"),
            Self::Swift => write!(f, "swift"),
            // Self::Toml => write!(f, "toml"),
            Self::TypeScript => write!(f, "typescript"),
            Self::TypeScriptReact => write!(f, "typescriptreact"),
            // Self::Vue => write!(f, "vue"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for LanguageId {
    fn from(value: &str) -> Self {
        match value {
            "apex" => Self::Apex,
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "csharp" => Self::CSharp,
            "css" => Self::Css,
            "d" => Self::D,
            // "elixir" => Self::Elixir,
            // "erlang" => Self::Erlang,
            "go" => Self::Go,
            "html" => Self::Html,
            "java" => Self::Java,
            "javascript" => Self::JavaScript,
            // "json" => Self::Json,
            "lua" => Self::Lua,
            // "markdown" => Self::Markdown,
            // "objective-c" => Self::ObjectiveC,
            "python" => Self::Python,
            "r" => Self::R,
            "ruby" => Self::Ruby,
            "rust" => Self::Rust,
            "scala" => Self::Scala,
            "shellscript" => Self::Bash,
            "swift" => Self::Swift,
            // "toml" => Self::Toml,
            "typescript" => Self::TypeScript,
            "typescriptreact" => Self::TypeScriptReact,
            // "vue" => Self::Vue,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for LanguageId {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<Language> for LanguageId {
    fn from(value: Language) -> Self {
        if value == tree_sitter_cpp::language() {
            Self::Cpp
        } else if value == tree_sitter_python::language() {
            Self::Python
        } else if value == tree_sitter_java::language() {
            Self::Java
        } else if value == tree_sitter_javascript::language() {
            Self::JavaScript
        } else if value == tree_sitter_rust::language() {
            Self::Rust
        } else if value == tree_sitter_typescript::language_typescript() {
            Self::TypeScript
        } else if value == tree_sitter_typescript::language_tsx() {
            Self::TypeScriptReact
        } else {
            Self::Unknown
        }
    }
}
