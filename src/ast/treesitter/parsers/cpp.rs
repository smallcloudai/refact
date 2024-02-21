use lazy_static::lazy_static;
use similar::DiffableStr;
use tree_sitter::{Node, Parser, Query};
use tree_sitter_cpp::language;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};

const CPP_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(translation_unit (declaration declarator: (init_declarator)) @global_variable)\n\
(namespace_definition (declaration_list (declaration (init_declarator)) @global_variable))";
const CPP_PARSER_QUERY_FUNCTION: &str = "((function_definition declarator: (function_declarator)) @function)";
const CPP_PARSER_QUERY_CLASS: &str = "((class_specifier name: (type_identifier)) @class)\n\
((struct_specifier name: (type_identifier)) @struct)\n\
((enum_specifier name: (type_identifier)) @enum)\n\
((declaration type: (enum_specifier)) @enum)";
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

fn get_function_name_and_scope_req(parent: Node, text: &str) -> (String, Vec<String>) {
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
            .set_language(language())
            .map_err(internal_error)?;
        Ok(CppParser { parser })
    }
}

impl LanguageParser for CppParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }


    fn get_parser_query(&self) -> &String {
        &CPP_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &CPP_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
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

    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        let mut name: String = Default::default();
        let mut values: Vec<String> = vec![];
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(language(),
                               "(enum_specifier name: (type_identifier) @name (_ (_ (identifier) @element)))\
                           ((declaration type: (enum_specifier (_ (_ (identifier) @element))) declarator: (identifier) @name))").unwrap();
        let matches = qcursor.matches(&query, parent, text.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "name" => {
                        name = text.slice(capture.node.byte_range()).to_string();
                    }
                    "element" => {
                        let text = text.slice(capture.node.byte_range());
                        values.push(text.to_string());
                    }
                    &_ => {}
                }
            }
        }
        (name, values)
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>) {
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

    fn get_variable_name(&self, parent: Node, text: &str) -> String {
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
}
