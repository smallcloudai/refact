use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct SwiftConfig;

impl Language for SwiftConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("protocol_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_declaration".to_string(), vec!["simple_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("operator_declaration".to_string(), vec!["simple_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("typealias_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("property_declaration".to_string(), vec![
                    "simple_identifier".to_string(),
                    "pattern".to_string(),
                ]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "associatedtype", "class", "deinit", "enum", "extension", "fileprivate", "func", "import",
                "init", "inout", "internal", "let", "open", "operator", "private", "precedencegroup", "protocol",
                "public", "rethrows", "static", "struct", "subscript", "typealias", "var",
                
                "break", "case", "catch", "continue", "default", "defer", "do", "else", "fallthrough", "for",
                "guard", "if", "in", "repeat", "return", "throw", "switch", "where", "while",
                
                "Any", "as", "await", "catch", "false", "is", "nil", "rethrows", "self", "Self", "super", "throw",
                "throws", "true", "try",
                
                "#available", "#colorLiteral", "#elseif", "#else", "#endif", "#fileLiteral", "#if", "#imageLiteral",
                "#keyPath", "#selector", "#sourceLocation"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}