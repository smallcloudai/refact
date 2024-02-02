use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct RustConfig;

impl Language for RustConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("enum_item".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("struct_item".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("impl_item".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("trait_item".to_string(), vec!["type_identifier".to_string()]),
                
                TypeDeclarationSearchInfo::new("function_signature_item".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_item".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("let_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("const_item".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("static_item".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("macro_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
                "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
                "Self", "self", "static", "struct", "super", "trait", "true", "type", "union", "unsafe", "use", "where",
                "while",
                "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized",
                "virtual", "yield"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}