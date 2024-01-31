use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use lazy_static::lazy_static;
use similar::DiffableStr;
use tracing_subscriber::fmt::format;
use tree_sitter::{Node, Parser, Tree};
use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};
use crate::lsp::treesitter::index::{Index, SymbolInfo, SymbolType};

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
                "xor_eq",
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![
                "primitive_type", "statement_identifier",
            ].iter().map(|s| s.to_string()).collect(),
        }
    }
}

const CPP_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(translation_unit (declaration declarator: (init_declarator)) @global_variable)\n\
(namespace_definition (declaration_list (declaration (init_declarator)) @global_variable))";
const CPP_PARSER_QUERY_FUNCTION: &str = "((function_definition declarator: (function_declarator)) @function)";
const CPP_PARSER_QUERY_CLASS: &str = "((class_specifier name: (type_identifier)) @class)\n((struct_specifier name: (type_identifier)) @struct)";
// const CPP_PARSER_QUERY_CLASS: &str = "";
const CPP_PARSER_QUERY_CALL_FUNCTION: &str = "";
const CPP_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const CPP_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const CPP_PARSER_QUERY_CLASS_METHOD: &str = "";

lazy_static! {
    static ref CPP_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(CPP_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(CPP_PARSER_QUERY_FUNCTION);
        m.push(CPP_PARSER_QUERY_CLASS);
        m.push(CPP_PARSER_QUERY_CALL_FUNCTION);
        m.push(CPP_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(CPP_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(CPP_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
}

fn get_namespace(mut parent: Option<Node>, text: &str) -> Vec<String> {
    let mut namespaces: Vec<String> = vec![];
    while parent.is_some() {
        let a = parent.unwrap().kind();
        match parent.unwrap().kind() {
            "namespace_definition" => {
                let children_len = parent.unwrap().child_count();
                for i in 0..children_len {
                    if let Some(child) = parent.unwrap().child(i) {
                        if child.kind() == "namespace_identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
                            break;
                        }
                    }
                }
            }
            "class_specifier" | "struct_specifier" => {
                let children_len = parent.unwrap().child_count();
                for i in 0..children_len {
                    if let Some(child) = parent.unwrap().child(i) {
                        if child.kind() == "type_identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
        parent = parent.unwrap().parent();
    }
    namespaces.reverse();
    namespaces
}

fn get_function_name_and_scope_req(mut parent: Node, text: &str) -> (String, Vec<String>) {
    let mut scope: Vec<String> = Default::default();
    let mut name: String = String::new();
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            let kind = child.kind();
            match kind {
                "identifier" => {
                    name = text.slice(child.byte_range()).to_string();
                }
                "qualified_identifier" | "template_type" => {
                    let (name_, scope_) = get_function_name_and_scope_req(child, text);
                    scope.extend(scope_);
                    name = name_;
                }
                "type_identifier" => {
                    scope.push(text.slice(child.byte_range()).to_string());
                }
                &_ => {}
            }
        }
    }
    (name, scope)
}

fn get_function_name_and_scope(mut parent: Node, text: &str) -> (String, Vec<String>) {
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            let kind = child.kind();
            match kind {
                "function_declarator" => {
                    for i in 0..child.child_count() {
                        if let Some(child) = child.child(i) {
                            let kind = child.kind();
                            match kind {
                                "identifier" => {
                                    let name = text.slice(child.byte_range());
                                    return (name.to_string(), vec![]);
                                }
                                "qualified_identifier" => {
                                    return get_function_name_and_scope_req(child, text);
                                }
                                &_ => {}
                            }
                        }
                    }
                }
                &_ => {}
            }
        }
    }
    ("".parse().unwrap(), vec![])
}

fn get_variable_name(mut parent: Node, text: &str) -> String {
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            let kind = child.kind();
            match kind {
                "init_declarator" => {
                    for i in 0..child.child_count() {
                        if let Some(child) = child.child(i) {
                            let kind = child.kind();
                            match kind {
                                "identifier" => {
                                    let name = text.slice(child.byte_range());
                                    return name.to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    return "".to_string();
}

pub fn get_indexes(parser: &mut Parser, code: &str, path: PathBuf) -> HashMap<String, Index> {
    let mut indexes: HashMap<String, Index> = Default::default();
    let tree: Tree = parser.parse(code, None).unwrap();
    let mut qcursor = tree_sitter::QueryCursor::new();
    let query = tree_sitter::Query::new(tree_sitter_cpp::language(), &**CPP_PARSER_QUERY).unwrap();
    let mut matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
    for match_ in matches {
        for capture in match_.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            match capture_name.as_str() {
                "class" | "struct" => {
                    let range = capture.node.range();
                    let mut namespaces = get_namespace(Some(capture.node), code);
                    let class_name = namespaces.last().unwrap().clone();
                    let mut key = path.to_str().unwrap().to_string();
                    namespaces.iter().for_each(|ns| {
                        key += format!("::{}", ns).as_str();
                    });
                    indexes.insert(key,
                                   Index {
                                       name: class_name,
                                       definition_info: SymbolInfo { path: path.clone(), range },
                                       children: vec![],
                                       symbol_type: SymbolType::Class,
                                   });
                }
                "function" => {
                    let range = capture.node.range();
                    let mut namespaces = get_namespace(Some(capture.node), code);
                    let (name, scopes) = get_function_name_and_scope(capture.node.clone(), code);
                    namespaces.extend(scopes);
                    namespaces.push(name.clone());
                    let mut key = path.to_str().unwrap().to_string();
                    namespaces.iter().for_each(|ns| {
                        key += format!("::{}", ns).as_str();
                    });
                    indexes.insert(key,
                                   Index {
                                       name: name,
                                       definition_info: SymbolInfo { path: path.clone(), range },
                                       children: vec![],
                                       symbol_type: SymbolType::Function,
                                   });
                }
                "global_variable" => {
                    let range = capture.node.range();
                    let mut namespaces = get_namespace(Some(capture.node), code);
                    let name = get_variable_name(capture.node, code);
                    let mut key = path.to_str().unwrap().to_string();
                    namespaces.iter().for_each(|ns| {
                        key += format!("::{}", ns).as_str();
                    });
                    indexes.insert(key,
                                   Index {
                                       name: name,
                                       definition_info: SymbolInfo { path: path.clone(), range },
                                       children: vec![],
                                       symbol_type: SymbolType::GlobalVar,
                                   });
                }
                &_ => {}
            }
        }
    }
    indexes
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::lsp::treesitter::ast_config::cpp_config::get_indexes;

    const TEST_CODE: &str =
        r#"
#include <iostream>
using namespace std;
 
int b = 0;
 
struct asd {};
namespace internal {
int a = 0;
template <typename T> class Array {
private:
    T* ptr;
    int size;
 
public:
    Array(T arr[], int s);
    void print();
};
}
 
template <typename T> Array<T>::Array(T arr[], int s)
{
    ptr = new T[s];
    size = s;
    for (int i = 0; i < size; i++)
        ptr[i] = arr[i];
}
void print() {
}
template <typename T> void asd<T>::Array<T>::print()
{
    for (int i = 0; i < size; i++)
        cout << " " << *(ptr + i);
    cout << endl;
}
 Array<int> as(arr, 5);
 Array<int> as = Array<int>(arr, 5);
int main()
{
    int arr[5] = { 1, 2, 3, 4, 5 };
    Array<int> a(arr, 5);
    a.print();
    return 0;
}
"#;

    #[test]
    fn test_query_cpp_function() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_cpp::language()).unwrap();
        let indexes = get_indexes(&mut parser, TEST_CODE, PathBuf::from("test.cpp"));
        assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}

