use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct CConfig;

impl Language for CConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("type_definition".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("attribute_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("preproc_def".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("declaration".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "alignas", "alignof", "auto", "bool", "break", "case", "char", "const", "constexpr", "continue",
                "default", "do", "double", "else", "enum", "extern", "false", "float", "for", "goto", "if", "inline",
                "int", "long", "nullptr", "register", "restrict", "return", "short", "signed", "sizeof", "static",
                "static_assert", "struct", "switch", "thread_local", "true", "typedef", "typeof", "typeof_unqual",
                "union", "unsigned", "void", "volatile", "while", "_Alignas", "_Alignof", "_Atomic", "_BitInt",
                "_Bool", "_Complex", "_Decimal128", "_Decimal32", "_Decimal64", "_Generic", "_Imaginary", "_Noreturn",
                "_Static_assert", "_Thread_local"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![]
        }
    }
}