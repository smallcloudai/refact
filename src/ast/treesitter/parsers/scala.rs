use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct ScalaConfig;

impl Language for ScalaConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("val_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("var_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("object_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("trait_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "abstract", "case", "catch", "class", "def", "do", "else", "extends", "false", "final", "finally",
                "for", "forSome", "if", "implicit", "import", "lazy", "match", "new", "null", "object", "override",
                "package", "private", "protected", "return", "sealed", "super", "this", "throw", "trait", "true", "try",
                "type", "val", "var", "while", "with", "yield", ">:", "⇒", "=>", "=", "<%", "<:", "←", "<-", "#", "@",
                ":", "_"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}