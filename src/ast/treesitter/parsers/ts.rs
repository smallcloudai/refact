use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct TSConfig;

impl Language for TSConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("enum_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_declaration".to_string(), vec!["identifier".to_string()]),
                
                TypeDeclarationSearchInfo::new("interface_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("ambient_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("type_alias_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("ambient_declaration".to_string(), vec!["identifier".to_string()]),
                
                TypeDeclarationSearchInfo::new("object_type".to_string(), vec![
                    "type_identifier".to_string(),
                    "property_identifier".to_string(),
                    "identifier".to_string(),
                ]),

            ],
            namespace_search_info: Option::from(TypeDeclarationSearchInfo::new("internal_module".to_string(), vec!["identifier".to_string()])),
            keywords: vec![
                "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else",
                "enum", "export", "extends", "false", "finally", "for", "function", "if", "import", "in", "instanceof", "new",
                "null", "return", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with",

                "as", "implements", "interface", "let", "package", "private", "protected", "public", "static", "yield",

                "any", "boolean", "constructor", "declare", "get", "module", "require", "number",
                "set", "string", "symbol", "type", "from", "of"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}