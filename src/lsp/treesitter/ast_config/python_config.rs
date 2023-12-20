use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct PythonConfig;

impl Language for PythonConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "False", "def", "if", "raise", "None", "del", "import", "return", "True", "elif", "in",
                "try", "and", "else", "is", "while", "as", "except", "lambda", "with", "assert", "finally",
                "nonlocal", "yield", "break", "for", "not", "class", "from", "or", "continue", "global", "pass",
                "__init__", "__str__", "__repr__", "__len__", "__getitem__", "__setitem__", "__delitem__",
                "__del__", "__iter__", "__reversed__", "__cmp__", "__lt__", "__gt__", "__le__", "__ge__", "__all__",
                "__format__", "__sizeof__", "__str__", "__repr__", "__hash__", "__cmp__", "__lt__", "__gt__",
                "__call__", "Dict", "List", "Tuple", "Set", "Dict", "String", "Bytes", "Bytes", "self", "str", "dict",
                "int", "float", "str", "bool", "None", "bytes", "bytes"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}