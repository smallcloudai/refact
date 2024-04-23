use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use parking_lot::RwLock;

use similar::DiffableStr;
use tree_sitter::{Node, Parser, Point, Range};
use tree_sitter_rust::language;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstance, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, StructDeclaration, TypeAlias, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{get_children_guids, get_guid};

pub(crate) struct RustParser {
    pub parser: Parser,
}

static RUST_KEYWORDS: [&str; 37] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while"
];

impl RustParser {
    pub fn new() -> Result<RustParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(RustParser { parser })
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
                let namespace = {
                    if let Some(namespace) = parent.child_by_field_name("path") {
                        code.slice(namespace.byte_range()).to_string()
                    } else {
                        "".to_string()
                    }
                };
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
                for i in 0..parent.child_count() {
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
                for i in 0..type_arguments.child_count() {
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

    pub fn parse_function_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.language = LanguageId::Rust;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.is_error = is_error;
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&parent, code, path, &decl.ast_fields.guid));

        let name_node = parent.child_by_field_name("name").unwrap();
        let parameters_node = parent.child_by_field_name("parameters").unwrap();
        symbols.extend(self.find_error_usages(&parameters_node, code, path, &decl.ast_fields.guid));
        let mut decl_end_byte: usize = parameters_node.end_byte();
        let mut decl_end_point: Point = parameters_node.end_position();

        let params_len = parameters_node.child_count();
        let mut function_args = vec![];
        for idx in 0..params_len {
            let child = parameters_node.child(idx).unwrap();
            match child.kind() {
                "parameter" => {
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
                _ => {}
            }
        }

        decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        if let Some(return_type) = parent.child_by_field_name("return_type") {
            symbols.extend(self.find_error_usages(&return_type, code, path, &decl.ast_fields.guid));
            decl.return_type = RustParser::parse_type(&return_type, code);
            decl_end_byte = return_type.end_byte();
            decl_end_point = return_type.end_position();
        }
        if let Some(type_parameters) = parent.child_by_field_name("type_parameters") {
            let mut templates = vec![];
            for idx in 0..type_parameters.child_count() {
                if let Some(t) = RustParser::parse_type(&type_parameters.child(idx).unwrap(), code) {
                    templates.push(t);
                }
            }
            symbols.extend(self.find_error_usages(&type_parameters, code, path, &decl.ast_fields.guid));
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
            };
            symbols.extend(self.parse_block(&body_node, code, path, &decl.ast_fields.guid, is_error));
        } else {
            decl.ast_fields.declaration_range = decl.ast_fields.full_range.clone();
        }
        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_struct_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields.language = LanguageId::Rust;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        symbols.extend(self.find_error_usages(&parent, code, path, &decl.ast_fields.guid));

        if let Some(name_node) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }
        if let Some(type_node) = parent.child_by_field_name("type") {
            symbols.extend(self.find_error_usages(&type_node, code, path, &decl.ast_fields.guid));
            if let Some(trait_node) = parent.child_by_field_name("trait") {
                symbols.extend(self.find_error_usages(&trait_node, code, path, &decl.ast_fields.guid));
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
                    symbols.extend(self.find_error_usages(&body_node, code, path, &decl.ast_fields.guid));
                    for idx in 0..body_node.child_count() {
                        let field_declaration_node = body_node.child(idx).unwrap();
                        match field_declaration_node.kind() {
                            "field_declaration" => {
                                let _text = code.slice(field_declaration_node.byte_range()).to_string();
                                let name_node = field_declaration_node.child_by_field_name("name").unwrap();
                                let type_node = field_declaration_node.child_by_field_name("type").unwrap();
                                let mut decl_ = ClassFieldDeclaration::default();
                                decl_.ast_fields.full_range = field_declaration_node.range();
                                decl_.ast_fields.file_path = path.clone();
                                decl_.ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                                decl_.ast_fields.guid = get_guid();
                                decl_.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                                decl_.ast_fields.language = LanguageId::Rust;
                                if let Some(type_) = RustParser::parse_type(&type_node, code) {
                                    decl_.type_ = type_;
                                }
                                symbols.push(Arc::new(RwLock::new(decl_)));
                            }
                            &_ => {}
                        }
                    }
                }
                "declaration_list" => {
                    symbols.extend(self.parse_block(&body_node, code, path, &decl.ast_fields.guid, is_error));
                }
                &_ => {}
            }
        }
        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_call_expression(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionCall::default();
        decl.ast_fields.language = LanguageId::Rust;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&parent, code, path, &parent_guid));

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
                        let usages = self.parse_usages(&value_node, code, path, parent_guid, is_error);
                        if !usages.is_empty() {
                            if let Some(last) = usages.last() {
                                // dirty hack: last element is first element in the tree
                                decl.set_caller_guid(last.read().fields().guid.clone());
                            }
                        }
                        symbols.extend(usages);
                    }
                    "scoped_identifier" => {
                        let namespace = {
                            if let Some(namespace) = parent.child_by_field_name("path") {
                                symbols.extend(self.find_error_usages(&namespace, code, path, &parent_guid));
                                code.slice(namespace.byte_range()).to_string()
                            } else {
                                "".to_string()
                            }
                        };
                        decl.ast_fields.namespace = namespace;
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

        if let Some(arguments_node) = arguments_node {
            symbols.extend(self.find_error_usages(&arguments_node, code, path, &parent_guid));
            for idx in 0..arguments_node.child_count() {
                let arg_node = arguments_node.child(idx).unwrap();
                let arg_type = self.parse_usages(&arg_node, code, path, &decl.ast_fields.guid, is_error);
                symbols.extend(arg_type);
            }
        }
        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_variable_definition(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        fn parse_type_in_value(parent: &Node, code: &str) -> TypeDef {
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
        let _text = code.slice(parent.byte_range()).to_string();
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = VariableDefinition::default();
        decl.ast_fields.language = LanguageId::Rust;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        symbols.extend(self.find_error_usages(&parent, code, path, &parent_guid));

        if let Some(type_node) = parent.child_by_field_name("type") {
            symbols.extend(self.find_error_usages(&type_node, code, path, &parent_guid));
            if let Some(type_) = RustParser::parse_type(&type_node, code) {
                decl.type_ = type_;
            }
        }

        if let Some(value_node) = parent.child_by_field_name("value") {
            decl.type_ = parse_type_in_value(&value_node, code);

            symbols.extend(self.parse_usages(&value_node, code, path, &decl.ast_fields.guid.clone(), is_error));
        }

        let pattern_node = match parent.kind() {
            "const_item" | "static_item" => {
                parent.child_by_field_name("name").unwrap()
            }
            _ => {
                parent.child_by_field_name("pattern").unwrap()
            }
        };
        let kind = pattern_node.kind();

        match kind {
            "identifier" => {
                decl.ast_fields.name = code.slice(pattern_node.byte_range()).to_string();
            }
            "tuple_pattern" => {
                let first_child = pattern_node.child(1).unwrap();
                decl.ast_fields.name = code.slice(first_child.byte_range()).to_string();

                if let Some(value_node) = parent.child_by_field_name("value") {
                    let is_value_tuple = value_node.kind() == "tuple_expression"
                        && value_node.child_count() == pattern_node.child_count();
                    if is_value_tuple {
                        decl.type_ = parse_type_in_value(&value_node.child(1).unwrap(), code);
                    }

                    // TODO comment problem
                    for i in (3..pattern_node.child_count() - 1).step_by(2) {
                        let child = pattern_node.child(i).unwrap();
                        let mut decl_ = decl.clone();
                        decl_.ast_fields.name = code.slice(child.byte_range()).to_string();
                        decl_.ast_fields.guid = get_guid();
                        if is_value_tuple {
                            let val = value_node.child(i).unwrap();
                            decl_.type_ = parse_type_in_value(&val, code);
                        }
                        symbols.push(Arc::new(RwLock::new(decl_)));
                    }
                }
            }
            "tuple_struct_pattern" => {
                let child = pattern_node.child(2).unwrap();
                decl.ast_fields.name = code.slice(child.byte_range()).to_string();
            }
            &_ => {}
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let kind = parent.kind();
        let _text = code.slice(parent.byte_range()).to_string();
        match kind {
            "unary_expression" | "parenthesized_expression" | "return_expression" => {
                if let Some(arg) = parent.child(1) {
                    symbols.extend(self.parse_usages(&arg, code, path, parent_guid, is_error));
                }
            }
            "try_expression" | "match_pattern" | "await_expression" => {
                let arg = parent.child(0).unwrap();
                symbols.extend(self.parse_usages(&arg, code, path, parent_guid, is_error));
            }
            "type_cast_expression" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid, is_error));
                // let type_node = parent.child_by_field_name("type").unwrap();
                // TODO think about this
                // res.extend(RustParser::parse_argument(&right, code, path));
            }
            "reference_expression" => {
                let arg = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&arg, code, path, parent_guid, is_error));
            }
            "binary_expression" => {
                let left = parent.child_by_field_name("left").unwrap();
                symbols.extend(self.parse_usages(&left, code, path, parent_guid, is_error));
                let right = parent.child_by_field_name("right").unwrap();
                symbols.extend(self.parse_usages(&right, code, path, parent_guid, is_error));
            }
            "call_expression" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid, is_error));
            }
            "let_condition" => {
                symbols.extend(self.parse_variable_definition(&parent, code, path, parent_guid, is_error));
            }
            "field_expression" => {
                let field_node = parent.child_by_field_name("field").unwrap();
                let name = code.slice(field_node.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();

                let value_node = parent.child_by_field_name("value").unwrap();
                let usages = self.parse_usages(&value_node, code, path, parent_guid, is_error);
                if let Some(last) = usages.last() {
                    usage.ast_fields.caller_guid = Some(last.read().guid().clone());
                }
                symbols.extend(usages);
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                // usage.var_decl_guid = Some(RustParser::get_guid(Some(usage.ast_fields.name.clone()), parent, code, path));
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "scoped_identifier" => {
                let mut usage = VariableUsage::default();
                let namespace = {
                    if let Some(namespace) = parent.child_by_field_name("path") {
                        code.slice(namespace.byte_range()).to_string()
                    } else {
                        "".to_string()
                    }
                };
                let name_node = parent.child_by_field_name("name").unwrap();

                usage.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.namespace = namespace;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "tuple_expression" => {
                for idx in 0..parent.child_count() {
                    let tuple_child_node = parent.child(idx).unwrap();
                    symbols.extend(self.parse_usages(&tuple_child_node, code, path, parent_guid, is_error));
                }
            }
            "struct_expression" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid, is_error));
            }
            "if_expression" => {
                let condition_node = parent.child_by_field_name("condition").unwrap();
                symbols.extend(self.parse_usages(&condition_node, code, path, parent_guid, is_error));
                let consequence_node = parent.child_by_field_name("consequence").unwrap();
                symbols.extend(self.parse_expression_statement(&consequence_node, code, path, parent_guid, is_error));
                if let Some(alternative_node) = parent.child_by_field_name("alternative") {
                    let child = alternative_node.child(1).unwrap();
                    let v = self.parse_expression_statement(&child, code, path, parent_guid, is_error);
                    symbols.extend(v);
                }
            }
            "match_expression" => {
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid, is_error));
                let body_node = parent.child_by_field_name("body").unwrap();
                for i in 0..body_node.child_count() {
                    let child = body_node.child(i).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
            }
            "match_arm" => {
                let pattern_node = parent.child_by_field_name("pattern").unwrap();
                let mut symbols = self.parse_usages(&pattern_node, code, path, parent_guid, is_error);
                let value_node = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value_node, code, path, parent_guid, is_error));
            }
            "or_pattern" | "range_expression" | "index_expression" => {
                for idx in 0..parent.child_count() {
                    let child = parent.child(idx).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
            }
            "for_expression" => {
                let symbols_ = self.parse_variable_definition(&parent, code, path, parent_guid, is_error);
                symbols.extend(symbols_);
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid, is_error));
            }
            "while_expression" => {
                let condition_node = parent.child_by_field_name("condition").unwrap();
                symbols.extend(self.parse_usages(&condition_node, code, path, parent_guid, is_error));
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid, is_error));
            }
            "loop_expression" => {
                let body_node = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_expression_statement(&body_node, code, path, parent_guid, is_error));
            }
            "ERROR" => {
                symbols.extend(self.parse_error_usages(&parent, code, path, parent_guid));
            }
            _ => {}
        }
        symbols
    }

    fn find_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            if child.kind() == "ERROR" {
                symbols.extend(self.parse_error_usages(&child, code, path, parent_guid));
            }
        }
        symbols
    }

    fn parse_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        match parent.kind() {
            "field_expression" => {
                let field_node = parent.child_by_field_name("field").unwrap();
                let name = code.slice(field_node.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name.clone();
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;

                let value_node = parent.child_by_field_name("value").unwrap();
                let usages = self.parse_error_usages(&value_node, code, path, parent_guid);
                if let Some(last) = usages.last() {
                    usage.ast_fields.caller_guid = Some(last.read().guid().clone());
                }
                symbols.extend(usages);
                if !RUST_KEYWORDS.contains(&name.as_str()) {
                    symbols.push(Arc::new(RwLock::new(usage)));
                }
            }
            "identifier" => {
                let name = code.slice(parent.byte_range()).to_string();
                if RUST_KEYWORDS.contains(&name.as_str()) {
                    return vec![];
                }
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "scoped_identifier" => {
                let mut usage = VariableUsage::default();
                let namespace = {
                    if let Some(namespace) = parent.child_by_field_name("path") {
                        code.slice(namespace.byte_range()).to_string()
                    } else {
                        "".to_string()
                    }
                };
                let name_node = parent.child_by_field_name("name").unwrap();
                let name = code.slice(name_node.byte_range()).to_string();
                if RUST_KEYWORDS.contains(&name.as_str()) {
                    return vec![];
                }
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Rust;
                usage.ast_fields.namespace = namespace;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            &_ => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.parse_error_usages(&child, code, path, parent_guid));
                }
            }
        }

        symbols
    }

    pub fn parse_expression_statement(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols = vec![];
        let kind = parent.kind();
        let _text = code.slice(parent.byte_range()).to_string();
        match kind {
            "block" => {
                let v = self.parse_block(parent, code, path, parent_guid, is_error);
                symbols.extend(v);
            }
            "unsafe_block" => {
                let child = parent.child(1).unwrap();
                symbols.extend(self.parse_block(&child, code, path, parent_guid, is_error));
            }
            "assignment_expression" => {
                let left_node = parent.child_by_field_name("left").unwrap();
                let usages = self.parse_usages(&left_node, code, path, parent_guid, is_error);
                symbols.extend(usages);
                let right_node = parent.child_by_field_name("right").unwrap();
                let usages = self.parse_usages(&right_node, code, path, parent_guid, is_error);
                symbols.extend(usages);
            }
            &_ => {
                let usages = self.parse_usages(&parent, code, path, parent_guid, is_error);
                symbols.extend(usages);
            }
        }

        symbols
    }

    pub fn parse_block(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            let kind = child.kind();
            let _text = code.slice(child.byte_range()).to_string();
            match kind {
                "use_declaration" => {
                    let argument_node = child.child_by_field_name("argument").unwrap();
                    if argument_node.kind() == "use_as_clause" {
                        let alias_node = argument_node.child_by_field_name("alias").unwrap();
                        let mut type_alias = TypeAlias::default();
                        type_alias.ast_fields.name = code.slice(alias_node.byte_range()).to_string();
                        type_alias.ast_fields.language = LanguageId::Rust;
                        type_alias.ast_fields.full_range = child.range();
                        type_alias.ast_fields.file_path = path.clone();
                        type_alias.ast_fields.parent_guid = Some(parent_guid.clone());
                        type_alias.ast_fields.guid = get_guid();
                        type_alias.ast_fields.is_error = is_error;

                        let path_node = argument_node.child_by_field_name("path").unwrap();
                        if let Some(dtype) = RustParser::parse_type(&path_node, code) {
                            type_alias.types.push(dtype);
                        }
                        symbols.push(Arc::new(RwLock::new(type_alias)));
                    }
                }
                "type_item" => {
                    let name_node = child.child_by_field_name("name").unwrap();
                    let mut type_alias = TypeAlias::default();
                    type_alias.ast_fields.name = code.slice(name_node.byte_range()).to_string();
                    type_alias.ast_fields.language = LanguageId::Rust;
                    type_alias.ast_fields.full_range = child.range();
                    type_alias.ast_fields.file_path = path.clone();
                    type_alias.ast_fields.parent_guid = Some(parent_guid.clone());
                    type_alias.ast_fields.guid = get_guid();
                    type_alias.ast_fields.is_error = is_error;

                    let type_node = child.child_by_field_name("type").unwrap();
                    if let Some(dtype) = RustParser::parse_type(&type_node, code) {
                        type_alias.types.push(dtype);
                    }
                    symbols.push(Arc::new(RwLock::new(type_alias)));
                }
                "block" => {
                    let v = self.parse_block(&child, code, path, parent_guid, is_error);
                    symbols.extend(v);
                }
                "let_declaration" | "const_item" | "static_item" => {
                    let symbols_ = self.parse_variable_definition(&child, code, path, parent_guid, is_error);
                    symbols.extend(symbols_);
                }
                "expression_statement" => {
                    let child = child.child(0).unwrap();
                    let v = self.parse_expression_statement(&child, code, path, parent_guid, is_error);
                    symbols.extend(v);
                }
                // return without keyword
                "identifier" => {
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
                // return without keyword
                "call_expression" => {
                    let symbols_ = self.parse_call_expression(&child, code, path, parent_guid, is_error);
                    symbols.extend(symbols_);
                }
                "enum_item" | "struct_item" | "trait_item" | "impl_item" | "union_item" => {
                    symbols.extend(self.parse_struct_declaration(&child, code, path, parent_guid, is_error));
                }
                "function_item" | "function_signature_item" => {
                    symbols.extend(self.parse_function_declaration(&child, code, path, parent_guid, is_error));
                }
                "line_comment" | "block_comment" => {
                    let mut def = CommentDefinition::default();
                    def.ast_fields.language = LanguageId::Rust;
                    def.ast_fields.full_range = child.range();
                    def.ast_fields.file_path = path.clone();
                    def.ast_fields.guid = get_guid();
                    def.ast_fields.parent_guid = Some(parent_guid.clone());
                    def.ast_fields.is_error = is_error;
                    symbols.push(Arc::new(RwLock::new(def)));
                }

                &_ => {
                    let usages = self.parse_usages(&child, code, path, parent_guid, is_error);
                    symbols.extend(usages);
                }
            }
        }
        symbols
    }
}

impl AstLanguageParser for RustParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let parent_guid = get_guid();
        let symbols = self.parse_block(&tree.root_node(), code, path, &parent_guid, false);
        symbols
    }
}
