use std::collections::HashSet;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Point, Query, QueryCapture, Range, Tree};
use tree_sitter_rust::language;
use url::Url;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstance, FunctionArg, FunctionCall, FunctionCaller, FunctionDeclaration, StructDeclaration, TypeDef, VariableDefinition};

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::{SymbolInfo, VariableInfo};

const RUST_PARSER_QUERY_GLOBAL_VARIABLE: &str = "((static_item name: (identifier)) @global_variable)";
const RUST_PARSER_QUERY_FUNCTION: &str = "((function_item name: (identifier)) @function)
((function_signature_item name: (identifier)) @function)";
const RUST_PARSER_QUERY_CLASS: &str = "((struct_item name: (_)) @struct)\
((trait_item name: (_)) @trait)\
((enum_item name: (_)) @enum)\
((impl_item type: (_)) @impl)";
const RUST_PARSER_QUERY_CALL_FUNCTION: &str = "";
const RUST_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const RUST_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const RUST_PARSER_QUERY_CLASS_METHOD: &str = "";

const RUST_PARSER_QUERY_VARIABLES: &str = r#"((let_declaration pattern: (_)) @variable)
((let_condition) @variable)"#;
const RUST_PARSER_QUERY_FIND_VARIABLES: &str = r#"((let_declaration pattern: (_)) @variable)"#;

const RUST_PARSER_QUERY_CALLS: &str = r#"((call_expression function: (_)) @call)"#;
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
        m.push(RUST_PARSER_QUERY_FIND_CALLS);
        m.push(RUST_PARSER_QUERY_VARIABLES);
        m.push(RUST_PARSER_QUERY_CALLS);
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

fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

impl RustParser {
    pub fn new() -> Result<RustParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(RustParser { parser })
    }

    pub fn get_parent_guid(&mut self, node: &Node, code: &str, path: &Url) -> Option<String> {
        let namespaces = RustParser::get_namespace(node.parent(), code);
        if namespaces.is_empty() {
            return None;
        }
        let mut key = path.to_string();
        namespaces.iter().for_each(|ns| {
            key += format!("::{}", ns).as_str();
        });
        Some(str_hash(&key))
    }

    fn get_namespace(mut parent: Option<Node>, text: &str) -> Vec<String> {
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
            let child = parent.unwrap();
            match child.kind() {
                "struct_item" | "impl_item" | "trait_item" => {
                    if let Some(child) = child.child_by_field_name("name") {
                        if child.kind() == "type_identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
                        }
                    } else if let Some(type_node) = child.child_by_field_name("type") {
                        if let Some(trait_node) = child.child_by_field_name("trait") {
                            namespaces.push(format!("{}_{}", text.slice(type_node.byte_range()),
                                                    text.slice(trait_node.byte_range())));
                        } else {
                            namespaces.push(format!("{}", text.slice(type_node.byte_range())));
                        }
                    }
                }
                "function_item" => {
                    namespaces.push(RustParser::get_id_for_function(child, text));
                }
                _ => {}
            }
            parent = child.parent();
        }
        namespaces.reverse();
        namespaces
    }
    
    pub fn get_guid(name: Option<String>, node: &Node, code: &str, path: &Url) -> String {
        let mut namespaces = RustParser::get_namespace(Some(*node), code);
        if let Some(name) = name {
            namespaces.push(name.clone());
        }
        let mut key = path.to_string();
        namespaces.iter().for_each(|ns| {
            key += format!("::{}", ns).as_str();
        });
        str_hash(&key)
    }

    pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
        let kind = parent.kind();
        let text = code.slice(parent.byte_range()).to_string();
        match kind {
            "type_identifier" | "primitive_type" => {
                return Some(TypeDef {
                    name: Some(text),
                    inference_info: None,
                    is_pod: kind == "primitive_type",
                    namespace: "".to_string(),
                    guid: None,
                    nested_types: vec![],
                })
            }
            "scoped_type_identifier" => {
                let namespace = parent.child_by_field_name("path").unwrap();
                let namespace = code.slice(namespace.byte_range()).to_string();
                let name = parent.child_by_field_name("name").unwrap();
                let name = code.slice(name.byte_range()).to_string();
                return Some(TypeDef {
                    name: Some(name),
                    inference_info: None,
                    is_pod: false,
                    namespace,
                    guid: None,
                    nested_types: vec![],
                })
            }
            "tuple_type" => {
                // TODO
            }
            "dynamic_type" => {
                let trait_node = parent.child_by_field_name("trait").unwrap();
                return RustParser::parse_type(&trait_node, code)
            }
            "array_type" => {
                let element = parent.child_by_field_name("element").unwrap();
                return RustParser::parse_type(&element, code)
            }
            "generic_type" => {
                let name = parent.child_by_field_name("type").unwrap();
                let name = code.slice(name.byte_range()).to_string();
                let type_arguments = parent.child_by_field_name("type_arguments").unwrap();
                let mut nested_types = vec![];
                for i in (1..type_arguments.child_count()-1).step_by(2) {
                    let child = type_arguments.child(i).unwrap();
                    if let Some(t) = RustParser::parse_type(&child, code) {
                        nested_types.push(t);
                    }
                }
                return Some(TypeDef {
                    name: Some(name),
                    inference_info: None,
                    is_pod: false,
                    namespace: "".to_string(),
                    guid: None,
                    nested_types,
                })
            }
            "reference_type" => {
                return RustParser::parse_type(&parent.child_by_field_name("type").unwrap(), code)
            }
            &_ => {} 
        }
        None
    }
    
    pub fn parse_function_declaration(&mut self, parent: &Node, code: &str, path: &Url) -> FunctionDeclaration {
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = self.get_parent_guid(parent, code, path);
        
        let name_node = parent.child_by_field_name("name").unwrap();
        let parameters_node = parent.child_by_field_name("parameters").unwrap();
        let mut decl_end_byte: usize = parameters_node.end_byte();
        let mut decl_end_point: Point = parameters_node.end_position();
        
        let params_len = parameters_node.child_count();
        let mut function_args = vec![];
        for idx in (1..params_len-1).step_by(2) {
            let child = parameters_node.child(idx).unwrap();
            if child.kind() == "self_parameter" {
                continue;
            }
            let name = child.child_by_field_name("pattern").unwrap();
            let mut arg = FunctionArg {
                name: code.slice(name.byte_range()).to_string(),
                type_: None,
            };
            if let Some(type_node) = child.child_by_field_name("type") {
                let a = RustParser::parse_type(&type_node, code);
                arg.type_ = a;
            }
            function_args.push(arg);
        }
        
        decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        decl.ast_fields.guid = RustParser::get_guid(None, parent, code, path);
        if let Some(return_type) = parent.child_by_field_name("return_type") {
            decl.return_type = RustParser::parse_type(&return_type, code);
            decl_end_byte = return_type.end_byte();
            decl_end_point = return_type.end_position();
        }
        if let Some(type_parameters) = parent.child_by_field_name("type_parameters") {
            let mut templates = vec![];
            for idx in (1..type_parameters.child_count()-1).step_by(2) {
                if let Some(t) = RustParser::parse_type(&type_parameters.child(idx).unwrap(), code) {
                    templates.push(t);
                }
            }
            decl.template_types = templates;
        }
        decl.args = function_args;
        if let Some(body_node) = parent.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl_end_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl_end_point,
            }
        } else {
            decl.ast_fields.declaration_range = parent.range();
        }
        decl
    }
    
    pub fn parse_struct_declaration(&mut self, parent: &Node, code: &str, path: &Url) -> StructDeclaration {
        let mut decl = StructDeclaration::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = self.get_parent_guid(parent, code, path);
        decl.ast_fields.guid = RustParser::get_guid(None, parent, code, path);
        
        if let Some(name_node) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }
        if let Some(type_node) = parent.child_by_field_name("type") {
            if let Some(trait_node) = parent.child_by_field_name("trait") {
                if let Some(trait_name) = RustParser::parse_type(&trait_node, code) {
                    decl.template_types.push(trait_name);
                }
            }
            if let Some(type_name) = RustParser::parse_type(&type_node, code) {
                if let Some(name) = type_name.name {
                    decl.ast_fields.name = name.clone();
                    decl.template_types.extend(type_name.nested_types);
                } else {
                    decl.ast_fields.name = code.slice(type_node.byte_range()).to_string();
                }
            } else {
                decl.ast_fields.name = code.slice(type_node.byte_range()).to_string();
            }
            
        }
        decl
    }

    
    fn parse_argument(parent: &Node, code: &str, path: &Url) -> HashSet<FunctionArg> {
        let mut res: HashSet<FunctionArg> = Default::default();
        let kind = parent.kind();
        match kind {
            "unary_expression" | "parenthesized_expression" => {
                let arg = parent.child(1).unwrap();
                res.extend(RustParser::parse_argument(&arg, code, path));
            }
            "try_expression" => {
                let arg = parent.child(0).unwrap();
                res.extend(RustParser::parse_argument(&arg, code, path));
            }
            "type_cast_expression" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                res.extend(RustParser::parse_argument(&value_node, code, path));
                let type_node = parent.child_by_field_name("type").unwrap();
                // TODO think about this
                // res.extend(RustParser::parse_argument(&right, code, path));
            }
            "reference_expression" => {
                let arg = parent.child_by_field_name("value").unwrap();
                res.extend(RustParser::parse_argument(&arg, code, path));
            }
            "binary_expression" => {
                let left = parent.child_by_field_name("left").unwrap();
                res.extend(RustParser::parse_argument(&left, code, path));
                let right = parent.child_by_field_name("right").unwrap();
                res.extend(RustParser::parse_argument(&right, code, path));
            }
            "identifier" => {
                let name = code.slice(parent.byte_range()).to_string();
                let mut type_ = TypeDef::default();
                
                if let Some(dtype) = RustParser::parse_type(parent, code) {
                    type_ = dtype;
                }
                let guid = RustParser::get_guid(Some(name.clone()), parent, code, path);
                type_.guid = Some(guid);
                
                res.insert(FunctionArg {
                    name: code.slice(parent.byte_range()).to_string(),
                    type_: RustParser::parse_type(parent, code),
                });
            }

            _ => {}
        }
        res
    }
    pub fn parse_call_expression(&mut self, parent: &Node, code: &str, path: &Url) -> FunctionCall {
        let mut decl = FunctionCall::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = self.get_parent_guid(parent, code, path);
        
        let function_node = parent.child_by_field_name("function").unwrap();
        match function_node.kind() {
            "field_expression" => {
                let field = function_node.child_by_field_name("field").unwrap();
                decl.ast_fields.name = code.slice(field.byte_range()).to_string();
                // TODO FunctionCaller
            }
            "scoped_identifier" => {
                let path = function_node.child_by_field_name("path").unwrap();
                decl.ast_fields.namespace = code.slice(path.byte_range()).to_string();
                let name = function_node.child_by_field_name("name").unwrap();
                decl.ast_fields.name = code.slice(name.byte_range()).to_string();
            }
            "identifier" => {
                decl.ast_fields.name = code.slice(function_node.byte_range()).to_string();
            }
            &_ => {}
        }
        
        let arguments_node = parent.child_by_field_name("arguments").unwrap();
        for idx in (1..arguments_node.child_count()-1).step_by(2) {
            let arg_node = arguments_node.child(idx).unwrap();
            let arg_type = RustParser::parse_argument(&arg_node, code, path);
            let arg_type = RustParser::parse_argument(&arg_node, code, path);
            
        }
        
        
        decl
    }
    
    pub fn parse_variable_definition(&mut self, parent: &Node, code: &str, path: &Url) -> VariableDefinition {
        let mut decl = VariableDefinition::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = self.get_parent_guid(parent, code, path);

        let pattern_node = parent.child_by_field_name("pattern").unwrap();
        match pattern_node.kind() {
            "identifier" => {
                decl.ast_fields.name = code.slice(pattern_node.byte_range()).to_string();
            }
            "tuple_pattern" => {
                // TODO
            }
            &_ => {}
        }
        decl.ast_fields.guid = RustParser::get_guid(Some(decl.ast_fields.name.clone()), parent, code, path);
        
        if let Some(type_node) = parent.child_by_field_name("type") {
            if let Some(type_) = RustParser::parse_type(&type_node, code) {
                decl.type_ = type_;
            }
        }
        
        if let Some(value_node) = parent.child_by_field_name("value") {
            decl.type_.inference_info = Some(code.slice(value_node.byte_range()).to_string());
            if decl.type_.name.is_none() {
                // float_literal, integer_literal, boolean_literal, string_literal, char_literal
                decl.type_.is_pod = value_node.kind().ends_with("literal");
            }
        }
        
        decl
    }
    
    pub fn parse_expression_statement(&mut self, parent: &Node, code: &str, path: &Url) -> Vec<Arc<dyn AstSymbolInstance>>  {
        let mut symbols = vec![];
        let kind = parent.kind();
        match kind {
            "block" => {
                let v = self.parse_block(parent, code, path);
                symbols.extend(v);
            }
            "try_expression" => {
                let arg = parent.child(0).unwrap();
            }
            "call_expression" => {
                let f = self.parse_call_expression(&parent, code, path);
                symbols.push(Arc::new(f));
            }
            &_ => {}
        }

        symbols
    }
    
    pub fn parse_block(&mut self, parent: &Node, code: &str, path: &Url) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = vec![];
        for i in 1..parent.child_count() - 1 {
            let child = parent.child(i).unwrap();
            let kind = child.kind();
            let text = code.slice(child.byte_range()).to_string();
            match kind {
                "let_declaration" => {
                    let v = self.parse_variable_definition(&child, code, path);
                    // TODO parse right with usages
                    symbols.push(Arc::new(v));
                }
                "expression_statement" => {
                    let child = child.child(0).unwrap();
                    let v = self.parse_expression_statement(&child, code, path);
                    symbols.extend(v);
                }
                // return without keyword
                "identifier" => {
                    
                }
                // return without keyword
                "call_expression" => {
                    let f = self.parse_call_expression(&child, code, path);
                    symbols.push(Arc::new(f));
                }
                &_ => {}
            }
        }
        symbols
    }
    
    pub fn parse(&mut self, code: &str, path: &Url) -> Vec<String> {
        let tree = self.parser.parse(code, None).unwrap();
        let mut res = vec![];
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(self.parser.language().unwrap(), &RUST_PARSER_QUERY).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let text = code.slice(capture.node.byte_range()).to_string();
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "enum" | "class" | "struct" | "trait" | "impl" => {
                        let f = self.parse_struct_declaration(&capture.node, code, path);
                        res.push(text.clone());
                    }
                    "function" => {
                        let f = self.parse_function_declaration(&capture.node, code, path);
                        if let Some(body_node) = capture.node.child_by_field_name("body") {
                            let x = self.parse_block(&body_node, code, path);
                        }
                        res.push(text.clone());
                    }
                    "variable" => {
                        // let f = self.parse_variable_definition(&capture.node, code, path);
                        // res.push(text.clone());
                    }
                    "call" => {
                        // let f = self.parse_call_expression(&capture.node, code, path);
                        // res.push(text.clone());
                    }
                    _ => {}
                }
            }
        }
        res
    }

    fn get_id_for_function(parent: Node, code: &str) -> String {
        let mut res = String::from("___");
        let name_node = parent.child_by_field_name("name").unwrap();
        res.push_str(&code.slice(name_node.byte_range()).to_string());
        if let Some(type_parameters) = parent.child_by_field_name("type_parameters") {
            for idx in (1..type_parameters.child_count()-1).step_by(2) {
                if let Some(dtype) = RustParser::parse_type(&type_parameters.child(idx).unwrap(), code) {
                    res.push_str(&format!("_{}", &dtype.to_string()));
                }
            }
        }
        let parameters_node = parent.child_by_field_name("parameters").unwrap();
        let params_len = parameters_node.child_count();
        for idx in (1..params_len-1).step_by(2) {
            let child = parameters_node.child(idx).unwrap();
            if child.kind() == "self_parameter" {
                continue;
            }
            if let Some(type_node) = child.child_by_field_name("type") {
                if let Some(dtype) = RustParser::parse_type(&type_node, code) {
                    res.push_str(&format!("_{}", &dtype.to_string()));
                }
            }
        }
        if let Some(return_type) = parent.child_by_field_name("return_type") {
            if let Some(dtype) = RustParser::parse_type(&return_type, code) {
                res.push_str(&format!("_{}", &dtype.to_string()));
            }
        }
        res
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
