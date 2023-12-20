use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct GoConfig;

impl Language for GoConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("function_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("var_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("const_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("type_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("field_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("method_declaration".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "break", "default", "func", "interface", "select", "case", "defer", "go", "map", "struct",
                "chan", "else", "goto", "package", "switch", "const", "fallthrough", "if", "range", "type",
                "continue", "for", "import", "return", "var"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}