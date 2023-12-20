use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct DConfig;

impl Language for DConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("module_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("func_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("enum_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("struct_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("class_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("interface_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("union_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("mixin_declaration".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("template_declaration".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "abstract", "alias", "align", "asm", "assert", "auto", "body", "bool", "break", "byte", "case", "cast",
                "catch", "cdouble", "cent", "cfloat", "char", "class", "const", "continue", "creal", "dchar", "debug",
                "default", "delegate", "delete", "deprecated", "do", "double", "else", "enum", "export", "extern",
                "false", "final", "finally", "float", "for", "foreach", "foreach_reverse", "function", "goto",
                "idouble", "if", "ifloat", "immutable", "import", "in", "inout", "int", "interface", "invariant",
                "isreal", "is", "lazy", "long", "macro", "mixin", "module", "new", "nothrow", "null", "out",
                "override", "package", "pragma", "private", "protected", "public", "pure", "real", "ref", "return",
                "scope", "shared", "short", "static", "struct", "super", "switch", "synchronized", "template", "this",
                "throw", "true", "try", "typeid", "typeof", "ubyte", "ucent", "uint", "ulong", "union", "unittest",
                "ushort", "version", "void", "wchar", "while", "with", "__FILE__", "__FILE_FULL_PATH__", "__MODULE__",
                "__LINE__", "__FUNCTION__", "__PRETTY_FUNCTION__", "__gshared", "__traits", "__vector", "__parameters"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}