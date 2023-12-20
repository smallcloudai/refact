use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct PhpConfig;

impl Language for PhpConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["name".to_string()]),
                TypeDeclarationSearchInfo::new("property_declaration".to_string(), vec!["name".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["module_name".to_string()]),
                TypeDeclarationSearchInfo::new("trait_declaration".to_string(), vec!["module_name".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "__halt_compiler()", "abstract", "and", "array()", "as", "break", "callable", "case", "catch", "class",
                "clone", "const", "continue", "declare", "default", "die()", "do", "echo", "else", "elseif", "empty()",
                "enddeclare", "endfor", "endforeach", "endif", "endswitch", "endwhile", "eval()", "exit()", "extends",
                "final", "finally", "fn", "for", "foreach", "function", "global", "goto", "if", "implements", "include",
                "include_once", "instanceof", "insteadof", "interface", "isset()", "list()", "match", "namespace", "new",
                "or", "print", "private", "protected", "public", "readonly", "require", "require_once", "return", "static",
                "switch", "throw", "trait", "try", "unset()", "use", "var", "while", "yield from",
                "__CLASS__", "__DIR__", "__FILE__", "__FUNCTION__", "__LINE__", "__METHOD__", "__NAMESPACE__", "__TRAIT__"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}