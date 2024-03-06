use std::collections::HashSet;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Point, Query, QueryCapture, Range, Tree};
use tree_sitter_rust::language;
use url::Url;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionDeclaration, StructDeclaration, TypeAlias, TypeDef, VariableDefinition, VariableUsage};
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
        // m.push(RUST_PARSER_QUERY_CALLS);
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

    pub fn get_guid() -> String {
        let id = Uuid::new_v4();
        id.to_string()
    }

    pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
        let kind = parent.kind();
        let text = code.slice(parent.byte_range()).to_string();
        match kind {
            "identifier" | "type_identifier" | "primitive_type" => {
                return Some(TypeDef {
                    name: Some(text),
                    inference_info: None,
                    is_pod: kind == "primitive_type",
                    namespace: "".to_string(),
                    guid: None,
                    nested_types: vec![],
                });
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
                });
            }
            "tuple_type" => {
                let mut nested_types = vec![];
                for i in (1..parent.child_count() - 1).step_by(2) {
                    let child = parent.child(i).unwrap();
                    if let Some(t) = RustParser::parse_type(&child, code) {
                        nested_types.push(t);
                    }
                }
                return Some(TypeDef {
                    name: Some("tuple".to_string()),
                    inference_info: None,
                    is_pod: false,
                    namespace: "".to_string(),
                    guid: None,
                    nested_types,
                });
            }
            "dynamic_type" => {
                let trait_node = parent.child_by_field_name("trait").unwrap();
                return RustParser::parse_type(&trait_node, code);
            }
            "array_type" => {
                let element = parent.child_by_field_name("element").unwrap();
                return RustParser::parse_type(&element, code);
            }
            "generic_type" => {
                let name = parent.child_by_field_name("type").unwrap();
                let name = code.slice(name.byte_range()).to_string();
                let type_arguments = parent.child_by_field_name("type_arguments").unwrap();
                let mut nested_types = vec![];
                for i in (1..type_arguments.child_count() - 1).step_by(2) {
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
                });
            }
            "reference_type" => {
                return RustParser::parse_type(&parent.child_by_field_name("type").unwrap(), code);
            }
            &_ => {}
        }
        None
    }

    pub fn parse_function_declaration(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());

        let name_node = parent.child_by_field_name("name").unwrap();
        let parameters_node = parent.child_by_field_name("parameters").unwrap();
        let mut decl_end_byte: usize = parameters_node.end_byte();
        let mut decl_end_point: Point = parameters_node.end_position();

        let params_len = parameters_node.child_count();
        let mut function_args = vec![];
        for idx in (1..params_len - 1).step_by(2) {
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
        decl.ast_fields.guid = RustParser::get_guid();
        if let Some(return_type) = parent.child_by_field_name("return_type") {
            decl.return_type = RustParser::parse_type(&return_type, code);
            decl_end_byte = return_type.end_byte();
            decl_end_point = return_type.end_position();
        }
        if let Some(type_parameters) = parent.child_by_field_name("type_parameters") {
            let mut templates = vec![];
            for idx in (1..type_parameters.child_count() - 1).step_by(2) {
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
        if let Some(body_node) = parent.child_by_field_name("body") {
            symbols.extend(self.parse_block(&body_node, code, path, &decl.ast_fields.guid));
        }
        symbols.push(Arc::new(decl));
        symbols
    }

    pub fn parse_struct_declaration(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = Default::default();
        let mut decl = StructDeclaration::default();
        
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = RustParser::get_guid();

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

        if let Some(body_node) = parent.child_by_field_name("body") {
            match body_node.kind() {
                "field_declaration_list" => {
                    for idx in (1..body_node.child_count() - 1).step_by(2) {
                        let field_declaration_node = body_node.child(idx).unwrap();
                        let name_node = field_declaration_node.child_by_field_name("name").unwrap();
                        let type_node = field_declaration_node.child_by_field_name("type").unwrap();
                        let mut decl_ = ClassFieldDeclaration::default();
                        decl_.ast_fields.full_range = field_declaration_node.range();
                        decl_.ast_fields.file_url = path.clone();
                        decl_.ast_fields.content_hash = str_hash(&code.slice(field_declaration_node.byte_range()).to_string());
                        decl_.ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                        decl_.ast_fields.guid = RustParser::get_guid();
                        decl_.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                        if let Some(type_) = RustParser::parse_type(&type_node, code) {
                            decl_.type_ = type_;
                        }
                        symbols.push(Arc::new(decl_));
                    }
                }
                "declaration_list" => {
                    symbols.extend(self.parse_block(&body_node, code, path, &decl.ast_fields.guid));
                }
                &_ => {}
            }
        }

        symbols.push(Arc::new(decl));
        symbols
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
                // let type_node = parent.child_by_field_name("type").unwrap();
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
                let mut type_ = TypeDef::default();

                if let Some(dtype) = RustParser::parse_type(parent, code) {
                    type_ = dtype;
                }
                let guid = RustParser::get_guid();
                type_.guid = Some(guid);

                res.insert(FunctionArg {
                    name: code.slice(parent.byte_range()).to_string(),
                    type_: Some(type_),
                });
            }
            "field_initializer" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                res.extend(RustParser::parse_argument(&value_node, code, path));
            }
            "shorthand_field_initializer" => {
                let value_node = parent.child(0).unwrap();
                res.extend(RustParser::parse_argument(&value_node, code, path));
            }

            _ => {}
        }
        res
    }
    pub fn parse_call_expression(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = RustParser::get_guid();
        let mut arguments_node: Option<Node> = None;
        let kind = parent.kind();
        match kind {
            "call_expression" => {
                let function_node = parent.child_by_field_name("function").unwrap();
                match function_node.kind() {
                    "field_expression" => {
                        let field = function_node.child_by_field_name("field").unwrap();
                        decl.ast_fields.name = code.slice(field.byte_range()).to_string();
                        let value_node = function_node.child_by_field_name("value").unwrap();
                        symbols.extend(self.parse_usages(&value_node, code, path, &decl.ast_fields.guid));
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
                arguments_node = parent.child_by_field_name("arguments");
            }
            "struct_expression" => {
                let name_node = parent.child_by_field_name("name").unwrap();
                decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                arguments_node = parent.child_by_field_name("body");
            }
            &_ => {}
        }

        let mut args: HashSet<FunctionArg> = Default::default();
        if let Some(arguments_node) = arguments_node {
            for idx in (1..arguments_node.child_count() - 1).step_by(2) {
                let arg_node = arguments_node.child(idx).unwrap();
                let arg_type = RustParser::parse_argument(&arg_node, code, path);
                args.extend(arg_type);
            }
        }
        decl.args = args.into_iter().collect::<Vec<_>>();
        symbols.push(Arc::new(decl));
        symbols
    }

    pub fn parse_variable_definition(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        fn parse_type_in_value(parent: &Node, code: &str, path: &Url) -> TypeDef {
            let mut dtype = TypeDef::default();
            let kind = parent.kind();
            match kind {
                "struct_expression" => {
                    let name_node = parent.child_by_field_name("name").unwrap();
                    dtype.name = Some(code.slice(name_node.byte_range()).to_string());
                }
                &_ => {}
            }
            dtype.inference_info = Some(code.slice(parent.byte_range()).to_string());
            if dtype.name.is_none() {
                // float_literal, integer_literal, boolean_literal, string_literal, char_literal
                dtype.is_pod = parent.kind().ends_with("literal");
            }
            dtype
        }

        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = vec![];
        let mut decl = VariableDefinition::default();
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = RustParser::get_guid();

        if let Some(type_node) = parent.child_by_field_name("type") {
            if let Some(type_) = RustParser::parse_type(&type_node, code) {
                decl.type_ = type_;
            }
        }

        if let Some(value_node) = parent.child_by_field_name("value") {
            decl.type_ = parse_type_in_value(&value_node, code, path);

            symbols.extend(self.parse_usages(&value_node, code, path, &decl.ast_fields.guid.clone()));
        }
        
        let pattern_node = if parent.kind() == "let_declaration" {
            parent.child_by_field_name("pattern").unwrap()
        } else {
            parent.child_by_field_name("name").unwrap()
        };
        let kind = pattern_node.kind();

        match kind {
            "identifier" => {
                decl.ast_fields.name = code.slice(pattern_node.byte_range()).to_string();
            }
            "tuple_pattern" => {
                let first_child = pattern_node.child(1).unwrap();
                decl.ast_fields.name = code.slice(first_child.byte_range()).to_string();

                let value_node = parent.child_by_field_name("value").unwrap();
                let mut is_value_tuple = (value_node.kind() == "tuple_expression"
                    && value_node.child_count() == pattern_node.child_count());
                if is_value_tuple {
                    decl.type_ = parse_type_in_value(&value_node.child(1).unwrap(), code, path);
                }

                for i in (3..pattern_node.child_count() - 1).step_by(2) {
                    let child = pattern_node.child(i).unwrap();
                    let mut decl_ = decl.clone();
                    decl_.ast_fields.name = code.slice(child.byte_range()).to_string();
                    decl_.ast_fields.guid = RustParser::get_guid();
                    if is_value_tuple {
                        let val = value_node.child(i).unwrap();
                        decl_.type_ = parse_type_in_value(&val, code, path, );
                    }
                    symbols.push(Arc::new(decl_));
                }
            }
            "tuple_struct_pattern" => {
                let child = pattern_node.child(2).unwrap();
                decl.ast_fields.name = code.slice(child.byte_range()).to_string();
            }
            &_ => {}
        }
        symbols.push(Arc::new(decl));
        symbols
    }

    pub fn parse_usages(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = vec![];
        let kind = parent.kind();
        let text = code.slice(parent.byte_range()).to_string();
        match kind {
            "unary_expression" | "parenthesized_expression" | "return_expression" => {
                let arg = parent.child(1).unwrap();
                symbols.extend(self.parse_usages(&arg, code, path, parent_guid));
            }
            "try_expression" | "match_pattern" | "await_expression" => {
                let arg = parent.child(0).unwrap();
                symbols.extend(self.parse_usages(&arg, code, path, parent_guid));
            }
            "type_cast_expression" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid));
                // let type_node = parent.child_by_field_name("type").unwrap();
                // TODO think about this
                // res.extend(RustParser::parse_argument(&right, code, path));
            }
            "reference_expression" => {
                let arg = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&arg, code, path, parent_guid));
            }
            "binary_expression" => {
                let left = parent.child_by_field_name("left").unwrap();
                symbols.extend(self.parse_usages(&left, code, path, parent_guid));
                let right = parent.child_by_field_name("right").unwrap();
                symbols.extend(self.parse_usages(&right, code, path, parent_guid));
            }
            "call_expression" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid));
            }
            "let_condition" => {
                symbols.extend(self.parse_variable_definition(&parent, code, path, parent_guid));
            }
            "field_expression" => {
                let field_node = parent.child_by_field_name("field").unwrap();
                let name = code.slice(field_node.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_url = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = RustParser::get_guid();
                symbols.push(Arc::new(usage));

                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid));
            }
            "identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_url = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = RustParser::get_guid();
                // usage.var_decl_guid = Some(RustParser::get_guid(Some(usage.ast_fields.name.clone()), parent, code, path));
                symbols.push(Arc::new(usage));
            }
            "scoped_identifier" => {
                let mut usage = VariableUsage::default();
                let path_node = parent.child_by_field_name("path").unwrap();
                let name_node = parent.child_by_field_name("name").unwrap();

                usage.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                usage.ast_fields.namespace = code.slice(path_node.byte_range()).to_string();
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_url = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = RustParser::get_guid();
                // usage.var_decl_guid = Some(RustParser::get_guid(None, parent, code, path));
                symbols.push(Arc::new(usage));
            }
            "tuple_expression" => {
                for idx in (1..parent.child_count() - 1).step_by(2) {
                    let tuple_child_node = parent.child(idx).unwrap();
                    symbols.extend(self.parse_usages(&tuple_child_node, code, path, parent_guid));
                }
            }
            "struct_expression" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid));
            }
            "if_expression" => {
                let condition_node = parent.child_by_field_name("condition").unwrap();
                symbols.extend(self.parse_usages(&condition_node, code, path, parent_guid));
                let consequence_node = parent.child_by_field_name("consequence").unwrap();
                symbols.extend(self.parse_expression_statement(&consequence_node, code, path, parent_guid));
                if let Some(alternative_node) = parent.child_by_field_name("alternative") {
                    let child = alternative_node.child(1).unwrap();
                    let v = self.parse_expression_statement(&child, code, path, parent_guid);
                    symbols.extend(v);
                }
            }
            "match_expression" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid));
                let body_node = parent.child_by_field_name("body").unwrap();
                for i in (1..body_node.child_count() - 1) {
                    let child = body_node.child(i).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid));
                }
            }
            "match_arm" => {
                let pattern_node = parent.child_by_field_name("pattern").unwrap();
                let mut symbols = self.parse_usages(&pattern_node, code, path,parent_guid );
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid));
            }
            "or_pattern" | "range_expression" | "index_expression" => {
                for idx in (0..parent.child_count()).step_by(2) {
                    let child = parent.child(idx).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid));
                }
            }
            "for_expression" => {
                let symbols_ = self.parse_variable_definition(&parent, code, path, parent_guid);
                symbols.extend(symbols_);
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid));
            }
            "while_expression" => {
                let condition_node = parent.child_by_field_name("condition").unwrap();
                symbols.extend(self.parse_usages(&condition_node, code, path, parent_guid));
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid));
            }
            "loop_expression" => {
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid));
            }
            _ => {}
        }
        symbols
    }

    pub fn parse_expression_statement(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols = vec![];
        let kind = parent.kind();
        let text = code.slice(parent.byte_range()).to_string();
        match kind {
            "block" => {
                let v = self.parse_block(parent, code, path, parent_guid);
                symbols.extend(v);
            }
            "unsafe_block" => {
                let child = parent.child(1).unwrap();
                symbols.extend(self.parse_block(&child, code, path, parent_guid));
            }
            "assignment_expression" => {
                let left_node = parent.child_by_field_name("left").unwrap();
                let usages = self.parse_usages(&left_node, code, path, parent_guid);
                symbols.extend(usages);
                let right_node = parent.child_by_field_name("right").unwrap();
                let usages = self.parse_usages(&right_node, code, path, parent_guid);
                symbols.extend(usages);
            }
            &_ => {
                let usages = self.parse_usages(&parent, code, path, parent_guid);
                symbols.extend(usages);
            }
        }

        symbols
    }

    pub fn parse_block(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String) -> Vec<Arc<dyn AstSymbolInstance>> {
        let mut symbols: Vec<Arc<dyn AstSymbolInstance>> = vec![];
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            let kind = child.kind();
            let text = code.slice(child.byte_range()).to_string();
            match kind {
                "use_declaration" => {
                    let argument_node = child.child_by_field_name("argument").unwrap();
                    if argument_node.kind() == "use_as_clause" {
                        let alias_node = argument_node.child_by_field_name("alias").unwrap();
                        let mut type_alias = TypeAlias::default();
                        type_alias.ast_fields.name = code.slice(alias_node.byte_range()).to_string();
                        type_alias.ast_fields.full_range = parent.range();
                        type_alias.ast_fields.file_url = path.clone();
                        type_alias.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                        type_alias.ast_fields.parent_guid = Some(parent_guid.clone());
                        type_alias.ast_fields.guid = RustParser::get_guid();

                        let path_node = argument_node.child_by_field_name("path").unwrap();
                        if let Some(dtype) = RustParser::parse_type(&path_node, code) {
                            type_alias.types.push(dtype);
                        }

                        symbols.push(Arc::new(type_alias));
                    }
                }
                "type_item" => {
                    let name_node = child.child_by_field_name("name").unwrap();
                    let mut type_alias = TypeAlias::default();
                    type_alias.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                    type_alias.ast_fields.full_range = parent.range();
                    type_alias.ast_fields.file_url = path.clone();
                    type_alias.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                    type_alias.ast_fields.parent_guid = Some(parent_guid.clone());
                    type_alias.ast_fields.guid = RustParser::get_guid();

                    let type_node = child.child_by_field_name("type").unwrap();
                    if let Some(dtype) = RustParser::parse_type(&type_node, code) {
                        type_alias.types.push(dtype);
                    }
                    symbols.push(Arc::new(type_alias));
                }
                "block" => {
                    let v = self.parse_block(parent, code, path, parent_guid);
                    symbols.extend(v);
                }
                "let_declaration" | "const_item" | "static_item" => {
                    let symbols_ = self.parse_variable_definition(&child, code, path, parent_guid);
                    symbols.extend(symbols_);
                }
                "expression_statement" => {
                    let child = child.child(0).unwrap();
                    let v = self.parse_expression_statement(&child, code, path, parent_guid);
                    symbols.extend(v);
                }
                // return without keyword
                "identifier" => {
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid));
                }
                // return without keyword
                "call_expression" => {
                    let symbols_ = self.parse_call_expression(&child, code, path, parent_guid);
                    symbols.extend(symbols_);
                }
                "enum_item" | "struct_item" | "trait_item" | "impl_item" | "union_item" => {
                    symbols.extend(self.parse_struct_declaration(&child, code, path, parent_guid));
                }
                "function_item" | "function_signature_item" => {
                    symbols.extend(self.parse_function_declaration(&child, code, path, parent_guid));
                }
                "line_comment" | "block_comment"  => {
                    let mut def = CommentDefinition::default();
                    def.ast_fields.full_range = parent.range();
                    def.ast_fields.file_url = path.clone();
                    def.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                    def.ast_fields.guid = RustParser::get_guid();
                    def.ast_fields.parent_guid = Some(parent_guid.clone());
                    symbols.push(Arc::new(def));
                }
                
                &_ => {
                    let usages = self.parse_usages(&child, code, path, parent_guid);
                    symbols.extend(usages);
                }
            }
        }
        symbols
    }

    pub fn parse(&mut self, code: &str, path: &Url) -> Vec<Arc<dyn AstSymbolInstance>> {
        let tree = self.parser.parse(code, None).unwrap();
        let parent_guid = RustParser::get_guid();
        self.parse_block(&tree.root_node(), code, path, &parent_guid)
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
