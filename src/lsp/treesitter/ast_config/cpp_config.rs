use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct CppConfig;

impl Language for CppConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("struct_specifier".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class_specifier".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("union_specifier".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("enum_specifier".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("concept_definition".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("alias_declaration".to_string(), vec!["type_identifier".to_string()]),
                TypeDeclarationSearchInfo::new("template_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("preproc_def".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: Option::from(TypeDeclarationSearchInfo::new("namespace_definition".to_string(), vec!["identifier".to_string()])),
            keywords: vec![
                "alignas", "alignof", "and", "and_eq", "asm", "atomic_cancel",
                "atomic_commit", "atomic_noexcept", "auto", "bitand", "bitor",
                "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
                "char32_t", "class", "compl", "concept", "const", "consteval",
                "constexpr", "constinit", "const_cast", "continue", "co_await",
                "co_return", "co_yield", "decltype", "default", "delete", "do",
                "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern",
                "false", "float", "for", "friend", "goto", "if", "inline", "int",
                "long", "mutable", "namespace", "new", "noexcept", "not", "not_eq",
                "nullptr", "operator", "or", "or_eq", "private", "protected", "public",
                "reflexpr", "register", "reinterpret_cast", "requires", "return", "short",
                "signed", "sizeof", "static", "static_assert", "static_cast", "struct",
                "switch", "synchronized", "template", "this", "thread_local", "throw",
                "true", "try", "typedef", "typeid", "typename", "union", "unsigned",
                "using", "virtual", "void", "volatile", "wchar_t", "while", "xor",
                "xor_eq"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![
                "primitive_type", "statement_identifier"
            ].iter().map(|s| s.to_string()).collect(),
        }
    }
}