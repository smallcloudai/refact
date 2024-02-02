use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

use lazy_static::lazy_static;
use similar::DiffableStr;
use tree_sitter::{LanguageError, Node, Parser, Query, QueryCapture, Range, Tree};

use crate::ast::treesitter::index::{Index, SymbolInfo, SymbolType};
use crate::ast::treesitter::parsers::{internal_error, Language, LanguageParser, ParserError};
use crate::ast::treesitter::structs::{FunctionCallInfo, StaticInfo, StaticType, VariableInfo};

pub struct CppConfig;


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

pub fn get_variable(captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
    let mut var = VariableInfo {
        name: "".to_string(),
        range: Range {
            start_byte: 0,
            end_byte: 0,
            start_point: Default::default(),
            end_point: Default::default(),
        },
        type_name: None,
    };
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        match capture_name.as_str() {
            "variable" => {
                var.range = capture.node.range();
            }
            "variable_name" => {
                let text = code.slice(capture.node.byte_range());
                var.name = text.to_string();
            }
            "variable_type" => {
                let text = code.slice(capture.node.byte_range());
                var.type_name = Some(text.to_string());
            }
            &_ => {}
        }
    }
    if var.name.is_empty() {
        return None;
    }

    Some(var)
}

pub fn get_call(captures: &[QueryCapture], query: &Query, code: &str) -> Option<FunctionCallInfo> {
    let mut var = FunctionCallInfo {
        name: "".to_string(),
        range: Range {
            start_byte: 0,
            end_byte: 0,
            start_point: Default::default(),
            end_point: Default::default(),
        },
    };
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        match capture_name.as_str() {
            "call" => {
                var.range = capture.node.range();
            }
            "call_name" => {
                let text = code.slice(capture.node.byte_range());
                var.name = text.to_string();
            }
            &_ => {}
        }
    }
    if var.name.is_empty() {
        return None;
    }
    Some(var)
}

pub fn get_static(captures: &[QueryCapture], query: &Query, code: &str) -> Option<StaticInfo> {
    let text = code.slice(captures[0].node.byte_range());
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        return match capture_name.as_str() {
            "comment" => {
                Some(StaticInfo {
                    static_type: StaticType::Comment,
                    range: capture.node.range(),
                })
            }
            "string_literal" => {
                Some(StaticInfo {
                    static_type: StaticType::Literal,
                    range: capture.node.range(),
                })
            }
            &_ => {
                None
            }
        };
    }
    None
}

const CPP_PARSER_QUERY_FIND_VARIABLES: &str = r#"((declaration type: [
(template_type name: (type_identifier) @variable_type)
(primitive_type) @variable_type
] 
(init_declarator declarator: [
(identifier) @variable_name
(array_declarator (identifier) @variable_name)
])) @variable)"#;

const CPP_PARSER_QUERY_FIND_CALLS: &str = r#"((call_expression function: [
(field_expression (field_identifier) @call_name)
(identifier) @call_name
]) @call)"#;

const CPP_PARSER_QUERY_FIND_STATICS: &str = r#"(
([
(comment) @comment
(string_literal) @string_literal
])
)"#;

lazy_static! {
    static ref CPP_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        CPP_PARSER_QUERY_FIND_VARIABLES, CPP_PARSER_QUERY_FIND_CALLS, CPP_PARSER_QUERY_FIND_STATICS);
}


pub(crate) struct CppParser {
    pub parser: Parser,
}

impl CppParser {
    pub fn new() -> Result<CppParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .map_err(internal_error)?;
        Ok(CppParser { parser })
    }
}

impl LanguageParser for CppParser {
    fn parse_declarations(&mut self, code: &str, path: &PathBuf) -> Result<HashMap<String, Index>, String> {
        let mut indexes: HashMap<String, Index> = Default::default();
        let tree: Tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(tree_sitter_cpp::language(), &**CPP_PARSER_QUERY).unwrap();
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
                                           name,
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
                                           name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::GlobalVar,
                                       });
                    }
                    &_ => {}
                }
            }
        }
        Ok(indexes)
    }
    fn parse_usages(&mut self, code: &str) -> Result<(Vec<VariableInfo>, Vec<FunctionCallInfo>, Vec<StaticInfo>), String> {
        let tree: Tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut vars: Vec<VariableInfo> = Default::default();
        let mut calls: Vec<FunctionCallInfo> = Default::default();
        let mut statics: Vec<StaticInfo> = Default::default();
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(tree_sitter_cpp::language(), &**CPP_PARSER_QUERY_FIND_ALL).unwrap();
        let mut matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            match match_.pattern_index {
                0 => {
                    match get_variable(match_.captures, &query, code) {
                        None => {}
                        Some(var) => { vars.push(var) }
                    }
                }
                1 => {
                    match get_call(match_.captures, &query, code) {
                        None => {}
                        Some(var) => { calls.push(var) }
                    }
                }
                2 => {
                    match get_static(match_.captures, &query, code) {
                        None => {}
                        Some(var) => { statics.push(var) }
                    }
                }
                _ => {}
            }
        }
        Ok((vars, calls, statics))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::cpp::CppParser;
    use crate::ast::treesitter::parsers::LanguageParser;

    const TEST_CODE: &str =
        r#"
#include <iostream>
using namespace std;
 
int b = 0;

// comment
String cat = "cat";

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
    print();
    return 0;
}
"#;

    #[test]
    fn test_query_cpp_function() {
        let mut parser = CppParser::new();
        let zxc = parser.parse_usages(TEST_CODE);
        let indexes = parser.parse_declarations(TEST_CODE, PathBuf::from("test.cpp").as_ref());
        assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}

