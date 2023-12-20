use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct ElmConfig;

impl Language for ElmConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("module_declaration".to_string(), vec![
                    "lower_case_identifier".to_string(),
                    "upper_case_identifier".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("type_declaration".to_string(), vec![
                    "lower_case_identifier".to_string(),
                    "upper_case_identifier".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("type_alias_declaration".to_string(), vec![
                    "lower_case_identifier".to_string(),
                    "upper_case_identifier".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("type_annotation".to_string(), vec![
                    "lower_case_identifier".to_string(),
                    "upper_case_identifier".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("value_declaration".to_string(), vec!["function_declaration_left".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "if", "then", "else", "case", "of", "let", "in", "type", "module", "where",
                "import", "exposing", "as", "port"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}