use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct CSharpConfig;

impl Language for CSharpConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("struct_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("interface_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("enum_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("delegate_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("record_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("record_struct_declaration".to_string(), vec!["identifier".to_string()]),
                
                TypeDeclarationSearchInfo::new("conversion_operator_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("destructor_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("event_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("event_field_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("field_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("indexer_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("method_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("operator_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("property_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("local_function_statement".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: Option::from(TypeDeclarationSearchInfo::new("namespace_declaration".to_string(), vec!["identifier".to_string()])),
            keywords: vec![
                "abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char", "checked", "class", "const",
                "continue", "decimal", "default", "delegate", "do", "double", "else", "enum", "event", "explicit", "extern",
                "false", "finally", "fixed", "float", "for", "foreach", "goto", "if", "implicit", "in", "int", "interface",
                "internal", "is", "lock", "long", "namespace", "new", "null", "object", "operator", "out", "override", "params",
                "private", "protected", "public", "readonly", "ref", "return", "sbyte", "sealed", "short", "sizeof", "stackalloc",
                "static", "string", "struct", "switch", "this", "throw", "true", "try", "typeof", "uint", "ulong", "unchecked",
                "unsafe", "ushort", "using", "virtual", "void", "volatile", "while", "add", "and", "alias", "ascending", "args",
                "async", "await", "by", "descending", "dynamic", "equals", "file", "from", "get", "global", "group", "init",
                "into", "join", "let", "managed", "nameof", "nint", "not", "notnull", "nuint", "on", "or", "orderby", "partial",
                "record", "remove", "required", "scoped", "select", "set", "unmanaged", "value", "var", "when", "where", "with",
                "yield"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![]
        }
    }
}