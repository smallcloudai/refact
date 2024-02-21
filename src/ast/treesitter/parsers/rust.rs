use std::path::PathBuf;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture, Range, Tree};
use tree_sitter_rust::language;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::{SymbolInfo, VariableInfo};

const RUST_PARSER_QUERY_GLOBAL_VARIABLE: &str = "((static_item name: (identifier)) @global_variable)";
const RUST_PARSER_QUERY_FUNCTION: &str = "((function_item name: (identifier)) @function)";
const RUST_PARSER_QUERY_CLASS: &str = "((struct_item name: (type_identifier)) @struct)\n((trait_item name: (type_identifier)) @trait)";
const RUST_PARSER_QUERY_CALL_FUNCTION: &str = "";
const RUST_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const RUST_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const RUST_PARSER_QUERY_CLASS_METHOD: &str = "";

const RUST_PARSER_QUERY_FIND_VARIABLES: &str = r#"((let_declaration pattern: (identifier) @variable_name) @variable)"#;

const RUST_PARSER_QUERY_FIND_CALLS: &str = r#"
    ((call_expression function: [
    (identifier) @call_name
    (field_expression field: (field_identifier) @call_name)
    ]) @call)"#;

const RUST_PARSER_QUERY_FIND_STATICS: &str = r#"(
([
(line_comment) @comment
(block_comment) @comment
(string_literal) @string_literal
])
)"#;

const TRY_TO_FIND_TYPE_QUERY: &str = "[
    (primitive_type) @variable_type
    (_ element: (type_identifier) @variable_type)
    (_ type: (type_identifier) @variable_type)
    ((scoped_type_identifier (_)) @variable_type)
    ]";

lazy_static! {
    static ref RUST_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(RUST_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(RUST_PARSER_QUERY_FUNCTION);
        m.push(RUST_PARSER_QUERY_CLASS);
        m.push(RUST_PARSER_QUERY_CALL_FUNCTION);
        m.push(RUST_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(RUST_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(RUST_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
    
    static ref RUST_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        RUST_PARSER_QUERY_FIND_VARIABLES, RUST_PARSER_QUERY_FIND_CALLS, RUST_PARSER_QUERY_FIND_STATICS);
    
    static ref IMPL_TYPE_ID: u16 = language().field_id_for_name("type").unwrap();
    static ref STRUCT_NAME_ID: u16 = language().field_id_for_name("name").unwrap();
}

pub(crate) struct RustParser {
    pub parser: Parser,
}

impl RustParser {
    pub fn new() -> Result<RustParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(RustParser { parser })
    }
}

fn try_to_find_type(parser: &mut Parser, parent: &Node, code: &str) -> Option<String> {
    let mut qcursor = tree_sitter::QueryCursor::new();
    let query = Query::new(parser.language().unwrap(), TRY_TO_FIND_TYPE_QUERY).unwrap();
    let matches = qcursor.matches(&query, *parent, code.as_bytes());
    for match_ in matches {
        for capture in match_.captures {
            return Some(code.slice(capture.node.byte_range()).to_string());
        }
    }
    None
}

impl LanguageParser for RustParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        &RUST_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &RUST_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
            match parent.unwrap().kind() {
                "struct_item" | "impl_item" | "trait_item" => {
                    if let Some(child) = parent.unwrap().child_by_field_id(*STRUCT_NAME_ID) {
                        if child.kind() == "type_identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
                        }
                    } else if let Some(child) = parent.unwrap().child_by_field_id(*IMPL_TYPE_ID) {
                        if child.kind() == "type_identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
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
    
    fn get_extra_declarations_for_struct(&mut self, struct_name: String, tree: &Tree, code: &str, path: &PathBuf) -> Vec<SymbolInfo> {
        let mut res: Vec<SymbolInfo> = vec![];
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(self.get_parser().language().unwrap(),
                               &*format!("((impl_item type: (type_identifier) @impl_type) @impl (#eq? @impl_type \"{}\"))", struct_name)).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "impl" => {
                        res.push(SymbolInfo {
                            path: path.clone(),
                            range: capture.node.range(),
                        })
                    }
                    &_ => {}
                }
            }
        }
        res
    }

    fn get_variable(&mut self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        let mut var = VariableInfo {
            name: "".to_string(),
            range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            type_names: vec![],
            meta_path: None,
        };
        for capture in captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            match capture_name.as_str() {
                "variable" => {
                    var.range = capture.node.range();
                    if let Some(var_type) = try_to_find_type(&mut self.parser, &capture.node, code) {
                        var.type_names.push(var_type);
                    }
                }
                "variable_name" => {
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
    
    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        let mut name: String = Default::default();
        let mut values: Vec<String> = vec![];
        for i in 0..parent.child_count() {
            if let Some(child) = parent.child(i) {
                let kind = child.kind();
                match kind {
                    "identifier" => {
                        name = text.slice(child.byte_range()).to_string();
                    }
                    "enum_body" => {
                        for i in 0..child.child_count() {
                            if let Some(child) = child.child(i) {
                                let kind = child.kind();
                                match kind {
                                    "enum_constant" => {
                                        for i in 0..child.child_count() {
                                            if let Some(child) = child.child(i) {
                                                let kind = child.kind();
                                                match kind {
                                                    "identifier" => {
                                                        let text = text.slice(child.byte_range());
                                                        values.push(text.to_string());
                                                        break;
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
                    }
                    _ => {}
                }
            }
        }
        (name, values)
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        (get_function_name(parent, text), vec![])
    }

    fn get_variable_name(&self, parent: Node, text: &str) -> String {
        for i in 0..parent.child_count() {
            if let Some(child) = parent.child(i) {
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
        return "".to_string();
    }
}
