use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct KotlinConfig;

impl Language for KotlinConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_declaration".to_string(), vec!["simple_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("property_declaration".to_string(), vec!["variable_declaration".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "as", "as?", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in", "!in",
                "interface", "is", "!is", "null", "object", "package", "return", "super", "this", "throw", "true", "try",
                "typealias", "typeof", "val", "var", "when", "while", "by", "catch", "constructor", "delegate", "dynamic",
                "field", "file", "finally", "get", "import", "init", "param", "property", "receiver", "set", "setparam",
                "value", "where", "actual", "abstract", "annotation", "companion", "const", "crossinline", "data", "enum",
                "expect", "external", "final", "infix", "inline", "inner", "internal", "lateinit", "noinline", "open",
                "operator", "out", "override", "private", "protected", "public", "reified", "sealed", "suspend", "tailrec",
                "vararg"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}