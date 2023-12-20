use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct RubyConfig;

impl Language for RubyConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("method".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class".to_string(), vec!["constant".to_string()]),
                TypeDeclarationSearchInfo::new("module".to_string(), vec!["constant".to_string()]),
                TypeDeclarationSearchInfo::new("singleton_method".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "BEGIN", "END", "alias", "and", "begin", "break", "case", "class", "def", "module", "next",
                "nil", "not", "or", "redo", "rescue", "retry", "return", "elsif", "end", "false", "ensure", "for",
                "if", "true", "undef", "unless", "do", "else", "super", "then", "until", "when", "defined?", "self"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}