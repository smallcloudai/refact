use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct ApexConfig;

impl Language for ApexConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("trigger_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("interface_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("enum_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("field_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("method_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("constructor_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("constant_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("local_variable_declaration".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "abstract", "activate", "and", "any", "array", "as", "asc", "autonomous", "begin", "bigdecimal", "blob",
                "boolean", "break", "bulk", "by", "byte", "case", "cast", "catch", "char", "class", "collect", "commit",
                "const", "continue", "currency", "date", "datetime", "decimal", "default", "delete", "desc", "do", "double",
                "else", "end", "enum", "exception", "exit", "export", "extends", "false", "final", "finally", "float",
                "for", "from", "global", "goto", "group", "having", "hint", "if", "implements", "import", "in", "inner",
                "insert", "instanceof", "int", "integer", "interface", "into", "join", "like", "limit", "list", "long",
                "loop", "map", "merge", "new", "not", "null", "nulls", "number", "object", "of", "on", "or", "outer",
                "override", "package", "parallel", "pragma", "private", "protected", "public", "retrieve", "return",
                "rollback", "select", "set", "short", "sObject", "sort", "static", "string", "super", "switch",
                "synchronized", "system", "testmethod", "then", "this", "throw", "time", "transaction", "trigger", "true",
                "try", "undelete", "update", "upsert", "using", "virtual", "void", "webservice", "when", "where", "while",
                "after", "before", "count", "excludes", "first", "includes", "last", "order", "sharing", "with"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![]
        }
    }
}