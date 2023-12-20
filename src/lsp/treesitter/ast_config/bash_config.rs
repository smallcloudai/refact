use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct BashConfig;

impl Language for BashConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec![
                    "word".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("declaration_command".to_string(), vec![
                    "variable_name".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("variable_assignment".to_string(), vec![
                    "variable_name".to_string(),
                ]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "if", "then", "elif", "else", "fi", "time", "for", "in", "until", "while",
                "do", "done", "case", "esac", "coproc", "select", "function", "{", "}",
                "[[", "]]", "!",
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![]
        }
    }
}