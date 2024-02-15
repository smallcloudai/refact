use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct JSConfig;

impl Language for JSConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("lexical_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("method_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "abstract", "arguments", "await", "boolean", "break", "byte", "case", "catch", "char", "class", "const",
                "continue", "debugger", "default", "delete", "do", "double", "else", "enum", "eval", "export", "extends",
                "false", "final", "finally", "float", "for", "function", "goto", "if", "implements", "import", "in",
                "instanceof", "int", "interface", "let", "long", "native", "new", "null", "package", "private",
                "protected", "public", "return", "short", "static", "super", "switch", "synchronized", "this", "throw",
                "throws", "transient", "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}