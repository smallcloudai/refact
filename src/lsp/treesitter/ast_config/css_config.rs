use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct CssConfig;

impl Language for CssConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_selector".to_string(), vec![
                    "class_name".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("id_selector".to_string(), vec![
                    "id_name".to_string(),
                ]),
            ],
            namespace_search_info: None,
            keywords: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}