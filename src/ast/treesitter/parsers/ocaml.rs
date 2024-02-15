use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};

pub struct OcamlConfig;

impl Language for OcamlConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("value_definition".to_string(), vec![
                    "value_name".to_string(),
                    "rec".to_string(),
                ]),
                TypeDeclarationSearchInfo::new("type_definition".to_string(), vec!["type_constructor".to_string()]),
                TypeDeclarationSearchInfo::new("module_definition".to_string(), vec!["module_name".to_string()]),
                TypeDeclarationSearchInfo::new("module_type_definition".to_string(), vec!["module_name".to_string()]),
                TypeDeclarationSearchInfo::new("class_definition".to_string(), vec!["module_name".to_string()]),
                TypeDeclarationSearchInfo::new("class_type_definition".to_string(), vec!["module_name".to_string()]),
                TypeDeclarationSearchInfo::new("exception_definition".to_string(), vec!["constructor_name".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "!=", "#", "&", "&&", "'", "(", ")", "*", "+", ",", "-", "-.", "->", ".", "..", ".~",
                ":", "::", ":=", ":>", ";", ";;", "<", "<-", "=", ">", ">]", ">}", "?", "[", "[<", "[>",
                "[|", "]", "_", "`", "{", "{<", "|", "|]", "||", "}", "~",
                "and", "as", "assert", "asr", "begin", "class", "constraint", "do", "done", "downto", "else", "end",
                "exception", "external", "false", "for", "fun", "function", "functor", "if", "in", "include", "inherit",
                "initializer", "land", "lazy", "let", "lor", "lsl", "lsr", "lxor", "match", "method", "mod", "module",
                "mutable", "new", "nonrec", "object", "of", "open", "or", "private", "rec", "sig", "struct", "then",
                "to", "true", "try", "type", "val", "virtual", "when", "with"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}