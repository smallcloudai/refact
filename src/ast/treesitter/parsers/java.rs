use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct JavaConfig;

impl Language for JavaConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("interface_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("enum_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("field_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("method_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("constructor_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("destructor_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("operator_declaration".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class", "const", "continue",
                "default", "double", "do", "else", "enum", "extends", "false", "final", "finally", "float", "for", "goto",
                "if", "implements", "import", "instanceof", "int", "interface", "long", "native", "new", "null", "package",
                "private", "protected", "public", "return", "short", "static", "strictfp", "super", "switch", "synchronized",
                "this", "throw", "throws", "transient", "true", "try", "void", "volatile", "while"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}