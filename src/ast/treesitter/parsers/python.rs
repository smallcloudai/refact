use std::collections::VecDeque;
use std::string::ToString;
use std::sync::{Arc, RwLock};

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Point, Range};
use tree_sitter_python::language;
use url::Url;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, FunctionArg, FunctionCall, FunctionDeclaration, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{internal_error, AstLanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::{get_children_guids, get_guid, str_hash};

const PYTHON_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(expression_statement (assignment left: (identifier)) @global_variable)";
const PYTHON_PARSER_QUERY_FUNCTION: &str = "((function_definition name: (identifier)) @function)";
const PYTHON_PARSER_QUERY_CLASS: &str = "((class_definition name: (identifier)) @class)";
const PYTHON_PARSER_QUERY_CALL_FUNCTION: &str = "";
const PYTHON_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const PYTHON_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const PYTHON_PARSER_QUERY_CLASS_METHOD: &str = "";

const PYTHON_PARSER_QUERY_FIND_VARIABLES: &str = r#"(expression_statement
(assignment left: (identifier) @variable_left type: (_)? @variable_type right: (_) @variable_right) @variable)"#;

const PYTHON_PARSER_QUERY_FIND_CALLS: &str = r#"
((call function: [
(identifier) @call_name
(attribute attribute: (identifier) @call_name)
]) @call)"#;

const PYTHON_PARSER_QUERY_FIND_STATICS: &str = r#"(
([
(comment) @comment
(string) @string_literal
])
)"#;

lazy_static! {
    static ref PYTHON_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(PYTHON_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(PYTHON_PARSER_QUERY_FUNCTION);
        m.push(PYTHON_PARSER_QUERY_CLASS);
        m.push(PYTHON_PARSER_QUERY_CALL_FUNCTION);
        m.push(PYTHON_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(PYTHON_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(PYTHON_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };

    static ref PYTHON_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}",
        PYTHON_PARSER_QUERY_FIND_VARIABLES, PYTHON_PARSER_QUERY_FIND_CALLS, PYTHON_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct PythonParser {
    pub parser: Parser,
}

pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    match kind {
        "type" | "splat_type" => {
            let child = parent.child(0).unwrap();
            return parse_type(&child, code);
        }
        "identifier" => {
            return Some(TypeDef {
                name: Some(text),
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "integer" | "string" | "float" | "false" | "true" => {
            return Some(TypeDef {
                name: None,
                inference_info: Some(text),
                is_pod: true,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "generic_type" => {
            let name = parent.child(0).unwrap();
            let name = code.slice(name.byte_range()).to_string();
            let type_arguments = parent.child(1).unwrap();
            let mut nested_types = vec![];
            for i in 0..type_arguments.child_count() {
                let child = type_arguments.child(i).unwrap();
                if let Some(t) = parse_type(&child, code) {
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
        "attribute" => {
            let attribute = parent.child_by_field_name("attribute").unwrap();
            let name = code.slice(attribute.byte_range()).to_string();
            let object = parent.child_by_field_name("object").unwrap();
            let nested_types = {
                if let Some(dtype) = parse_type(&object, code) {
                    vec![dtype]
                } else {
                    vec![]
                }
            };
            return Some(TypeDef {
                name: Some(name),
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types,
            });
        }
        "call" => {
            let function = parent.child_by_field_name("function").unwrap();
            let mut dtype = parse_type(&function, code).unwrap_or(TypeDef::default());
            dtype.inference_info = Some(code.slice(parent.byte_range()).to_string());
            return Some(dtype);
        }
        &_ => {}
    }
    None
}

fn parse_function_arg(parent: &Node, code: &str) -> Vec<FunctionArg> {
    let mut args: Vec<FunctionArg> = vec![];
    // let text = code.slice(parent.byte_range()).to_string();
    let kind = parent.kind();
    match kind {
        "identifier" | "typed_parameter" => {
            let arg = FunctionArg {
                name: code.slice(parent.byte_range()).to_string(),
                type_: None,
            };
            args.push(arg);
        }
        "typed_default_parameter" | "default_parameter" => {
            let name = parent.child_by_field_name("name").unwrap();
            if name.kind() == "identifier" {
                let arg = FunctionArg {
                    name: code.slice(name.byte_range()).to_string(),
                    type_: None,
                };
                args.push(arg);
            } else {
                args.extend(parse_function_arg(&name, code));
            }
        }
        "tuple_pattern" => {
            for i in 0..parent.child_count() - 1 {
                let child = parent.child(i).unwrap();
                args.extend(parse_function_arg(&child, code));
            }
        }
        _ => {}
    }

    for arg in args.iter_mut() {
        if let Some(type_node) = parent.child_by_field_name("type") {
            if let Some(dtype) = parse_type(&type_node, code) {
                if let Some(type_) = &mut arg.type_ {
                    type_.inference_info = dtype.inference_info;
                } else {
                    arg.type_ = Some(dtype);
                }
            }
        }
    }

    if let Some(value_node) = parent.child_by_field_name("value") {
        let value_text = code.slice(value_node.byte_range()).to_string();
        for arg in args.iter_mut() {
            if arg.type_.is_some() {
                let type_ = arg.type_.as_mut().unwrap();
                type_.inference_info = Some(value_text.clone());
            } else {
                arg.type_ = Some(TypeDef {
                    name: None,
                    inference_info: Some(value_text.clone()),
                    is_pod: false,
                    namespace: "".to_string(),
                    guid: None,
                    nested_types: vec![],
                })
            }
        }
    }

    args
}

const SPECIAL_SYMBOLS: &str = "{}(),.;_|&";

impl PythonParser {
    pub fn new() -> Result<PythonParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(PythonParser { parser })
    }


    pub fn parse_struct_declaration(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields.language = LanguageId::Python;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        if let Some(parent_node) = parent.parent() {
            if parent_node.kind() == "decorated_definition" {
                decl.ast_fields.full_range = parent_node.range();
            }
        }

        if let Some(name_node) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }
        if let Some(superclasses) = parent.child_by_field_name("superclasses") {
            for i in 0..superclasses.child_count() {
                let child = superclasses.child(i).unwrap();
                if let Some(dtype) = parse_type(&child, code) {
                    decl.inherited_types.push(dtype);
                }
            }
        }
        if let Some(body) = parent.child_by_field_name("body") {
            symbols.extend(self.parse_block(&body, code, path, &decl.ast_fields.guid, is_error));
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    fn parse_assignment(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut is_class_field = false;
        {
            let mut parent_mb = parent.parent();
            while parent_mb.is_some() {
                let p = parent_mb.unwrap();
                match p.kind() {
                    "class_definition" => {
                        is_class_field = true;
                        break;
                    }
                    "function_definition" => {
                        break;
                    }
                    &_ => {}
                }
                parent_mb = p.parent();
            }
        }


        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        if let Some(right) = parent.child_by_field_name("right") {
            symbols.extend(self.parse_usages(&right, code, path, parent_guid, is_error));
        }
        if let Some(body) = parent.child_by_field_name("body") {
            symbols.extend(self.parse_block(&body, code, path, parent_guid, is_error));
        }

        let mut candidates: VecDeque<(Option<Node>, Option<Node>, Option<Node>)> = VecDeque::from(vec![
            (parent.child_by_field_name("left"),
             parent.child_by_field_name("type"),
             parent.child_by_field_name("right"))]);
        let mut right_for_all = false;
        while !candidates.is_empty() {
            let (left_mb, type_mb, right_mb) = candidates.pop_front().unwrap();
            if let Some(left) = left_mb {
                let text = code.slice(left.byte_range());
                if SPECIAL_SYMBOLS.contains(text) || text == "self" {
                    continue;
                }
                let kind = left.kind();
                match kind {
                    "identifier" => {
                        let mut fields = AstSymbolFields::default();
                        fields.language = LanguageId::Python;
                        fields.full_range = parent.range();
                        fields.file_url = path.clone();
                        fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                        fields.parent_guid = Some(parent_guid.clone());
                        fields.guid = get_guid();
                        fields.name = code.slice(left.byte_range()).to_string();
                        fields.is_error = is_error;

                        if is_class_field {
                            let mut decl = ClassFieldDeclaration::default();
                            decl.ast_fields = fields;
                            if let Some(type_node) = type_mb {
                                if let Some(type_) = parse_type(&type_node, code) {
                                    decl.type_ = type_;
                                }
                            }
                            symbols.push(Arc::new(RwLock::new(decl)));
                        } else {
                            let mut decl = VariableDefinition::default();
                            decl.ast_fields = fields;
                            if let Some(type_) = type_mb {
                                if let Some(dtype) = parse_type(&type_, code) {
                                    decl.type_ = dtype;
                                }
                            }
                            if let Some(right) = right_mb {
                                decl.type_.inference_info = Some(code.slice(right.byte_range()).to_string());
                                decl.type_.is_pod = vec!["integer", "string", "float", "false", "true"]
                                    .contains(&right.kind());
                            }
                            symbols.push(Arc::new(RwLock::new(decl)));
                        }
                    }
                    "attribute" => {
                        let usages = self.parse_usages(&left, code, path, parent_guid, is_error);
                        symbols.extend(usages);
                    }
                    "list_pattern" | "tuple_pattern" | "pattern_list" => {
                        let lefts: Vec<_> = (0..left.child_count())
                            .map(|i| left.child(i))
                            .filter(|node| !SPECIAL_SYMBOLS.contains(node.unwrap().kind()))
                            .collect();
                        let mut rights = vec![right_mb];
                        if let Some(right) = right_mb {
                            rights = (0..right.child_count())
                                .map(|i| right.child(i))
                                .filter(|node| !SPECIAL_SYMBOLS.contains(node.unwrap().kind()))
                                .collect();
                        }
                        if lefts.len() != rights.len() {
                            right_for_all = true;
                        }
                        for i in 0..lefts.len() {
                            let r = if right_for_all { right_mb } else { rights[i] };
                            candidates.push_back((*lefts.get(i).unwrap(), None, r));
                        }
                    }
                    "list_splat_pattern" => {
                        let child = left.child(0);
                        candidates.push_back((child, type_mb, right_mb));
                    }
                    &_ => {}
                }
            }
        }

        // https://github.com/tree-sitter/tree-sitter-python/blob/master/grammar.js#L844
        symbols
    }

    pub fn parse_usages(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let kind = parent.kind();
        // let text = code.slice(parent.byte_range()).to_string();
        // TODO lambda https://github.com/tree-sitter/tree-sitter-python/blob/master/grammar.js#L830
        match kind {
            "await" | "list_splat" | "yield" | "list_splat_pattern" |
            "tuple" | "set" | "list" | "dictionary" | "expression_list" | "comparison_operator" |
            "conditional_expression" | "as_pattern_target" | "print_statement" |
            "list_comprehension" | "dictionary_comprehension" | "set_comprehension" | "if_clause" |
            "with_statement" | "with_clause" | "case_clause" | "case_pattern" | "dotted_name" |
            "try_statement" | "except_clause" | "if_statement" | "elif_clause" | "else_clause" => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
            }
            "with_item" => {
                let value = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value, code, path, parent_guid, is_error));
            }
            "as_pattern" => {
                let value = parent.child(0).unwrap();
                if let Some(alias) = parent.child_by_field_name("alias") {
                    let mut candidates = VecDeque::from(vec![alias.child(0).unwrap()]);
                    while !candidates.is_empty() {
                        let child = candidates.pop_front().unwrap();
                        let text = code.slice(child.byte_range());
                        if SPECIAL_SYMBOLS.contains(text) || text == "self" {
                            continue;
                        }
                        match child.kind() {
                            "identifier" => {
                                let mut decl = VariableDefinition::default();
                                decl.ast_fields.language = LanguageId::Python;
                                decl.ast_fields.full_range = parent.range();
                                decl.ast_fields.file_url = path.clone();
                                decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                                decl.ast_fields.parent_guid = Some(parent_guid.clone());
                                decl.ast_fields.guid = get_guid();
                                decl.ast_fields.name = text.to_string();
                                decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                                decl.ast_fields.is_error = is_error;
                                symbols.push(Arc::new(RwLock::new(decl)));
                            }
                            "list" | "set" | "tuple" => {
                                for i in 0..child.child_count() {
                                    candidates.push_back(child.child(i).unwrap());
                                }
                            }
                            &_ => {
                                symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                            }
                        }
                    }
                }
            }
            "not_operator" | "unary_operator" => {
                let argument = parent.child_by_field_name("argument").unwrap();
                symbols.extend(self.parse_usages(&argument, code, path, parent_guid, is_error));
            }
            "boolean_operator" | "binary_operator" | "for_in_clause" | "augmented_assignment" => {
                let left = parent.child_by_field_name("left").unwrap();
                symbols.extend(self.parse_usages(&left, code, path, parent_guid, is_error));
                let right = parent.child_by_field_name("right").unwrap();
                symbols.extend(self.parse_usages(&right, code, path, parent_guid, is_error));
            }
            "pair" => {
                let key = parent.child_by_field_name("key").unwrap();
                symbols.extend(self.parse_usages(&key, code, path, parent_guid, is_error));
                let value = parent.child_by_field_name("value").unwrap();
                symbols.extend(self.parse_usages(&value, code, path, parent_guid, is_error));
            }
            "identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Python;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_url = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = is_error;
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "attribute" => {
                let attribute = parent.child_by_field_name("attribute").unwrap();
                let name = code.slice(attribute.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Python;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_url = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = is_error;

                let object_node = parent.child_by_field_name("object").unwrap();
                let usages = self.parse_usages(&object_node, code, path, parent_guid, is_error);
                if let Some(last) = usages.last() {
                    usage.ast_fields.caller_guid = last.read().expect("the data might be broken").fields().parent_guid.clone();
                }
                symbols.extend(usages);
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "assignment" | "for_statement" => {
                symbols.extend(self.parse_assignment(&parent, code, path, parent_guid, is_error));
            }
            "while_statement" => {
                let condition = parent.child_by_field_name("condition").unwrap();
                symbols.extend(self.parse_usages(&condition, code, path, parent_guid, is_error));
                let body = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_block(&body, code, path, parent_guid, is_error));
                if let Some(alternative) = parent.child_by_field_name("alternative") {
                    if let Some(body) = alternative.child_by_field_name("body") {
                        symbols.extend(self.parse_block(&body, code, path, parent_guid, is_error));
                    }
                }
            }
            "block" => {
                symbols.extend(self.parse_block(&parent, code, path, parent_guid, is_error));
            }
            "match_statement" => {
                let subject = parent.child_by_field_name("subject").unwrap();
                symbols.extend(self.parse_usages(&subject, code, path, parent_guid, is_error));
                let body = parent.child_by_field_name("body").unwrap();
                symbols.extend(self.parse_block(&body, code, path, parent_guid, is_error));
            }
            "call" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid, is_error));
            }
            "lambda" => {
                symbols.extend(self.parse_function_declaration(&parent, code, path, parent_guid, is_error));
            }
            "ERROR" => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, true));
                }
            }
            _ => {}
        }
        symbols
    }

    pub fn parse_expression_statement(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols = vec![];
        let kind = parent.kind();
        // let text = code.slice(parent.byte_range()).to_string();
        match kind {
            &_ => {
                let usages = self.parse_usages(&parent, code, path, parent_guid, is_error);
                symbols.extend(usages);
            }
        }

        symbols
    }

    pub fn parse_function_declaration(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.language = LanguageId::Python;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.is_error = is_error;
        if let Some(parent_node) = parent.parent() {
            if parent_node.kind() == "decorated_definition" {
                decl.ast_fields.full_range = parent_node.range();
            }
        }

        let mut decl_end_byte: usize = parent.end_byte();
        let mut decl_end_point: Point = parent.end_position();

        if let Some(name_node) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }

        if let Some(parameters_node) = parent.child_by_field_name("parameters") {
            decl_end_byte = parameters_node.end_byte();
            decl_end_point = parameters_node.end_position();

            let params_len = parameters_node.child_count();
            let mut function_args = vec![];
            for idx in 0..params_len {
                let child = parameters_node.child(idx).unwrap();
                function_args.extend(parse_function_arg(&child, code));
            }
            decl.args = function_args;
        }
        decl.ast_fields.guid = get_guid();
        if let Some(return_type) = parent.child_by_field_name("return_type") {
            decl.return_type = parse_type(&return_type, code);
            decl_end_byte = return_type.end_byte();
            decl_end_point = return_type.end_position();
        }

        if let Some(body_node) = parent.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl_end_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl_end_point,
            }
        } else {
            decl.ast_fields.declaration_range = decl.ast_fields.full_range.clone();
        }
        if let Some(body_node) = parent.child_by_field_name("body") {
            symbols.extend(self.parse_block(&body_node, code, path, &decl.ast_fields.guid, is_error));
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_block(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            let kind = child.kind();
            // let text = code.slice(child.byte_range()).to_string();
            match kind {
                "import_statement" => {
                    // TODO
                }
                "import_from_statement" => {
                    // TODO
                }
                "class_definition" => {
                    symbols.extend(self.parse_struct_declaration(&child, code, path, parent_guid, is_error));
                }
                "expression_statement" => {
                    if let Some(child) = child.child(0) {
                        symbols.extend(self.parse_expression_statement(&child, code, path, parent_guid, is_error));
                    }
                }
                "function_definition" => {
                    symbols.extend(self.parse_function_declaration(&child, code, path, parent_guid, is_error));
                }
                "decorated_definition" => {
                    if let Some(definition) = child.child_by_field_name("definition") {
                        match definition.kind() {
                            "class_definition" => {
                                symbols.extend(self.parse_struct_declaration(&definition, code, path, parent_guid, is_error));
                            }
                            "function_definition" => {
                                symbols.extend(self.parse_function_declaration(&definition, code, path, parent_guid, is_error));
                            }
                            &_ => {}
                        }
                    }
                }
                _ => {
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
            }
        }

        symbols
    }

    pub fn parse_call_expression(&mut self, parent: &Node, code: &str, path: &Url, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionCall::default();
        decl.ast_fields.language = LanguageId::Python;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_url = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        let arguments_node = parent.child_by_field_name("arguments").unwrap();
        for i in 0..arguments_node.child_count() {
            let child = arguments_node.child(i).unwrap();
            let text = code.slice(child.byte_range());
            if SPECIAL_SYMBOLS.contains(&text) { continue; }
            symbols.extend(self.parse_usages(&child, code, path, &decl.ast_fields.guid, is_error));
        }

        let function_node = parent.child_by_field_name("function").unwrap();
        let text = code.slice(function_node.byte_range());
        let kind = function_node.kind();
        match kind {
            "identifier" => {
                decl.ast_fields.name = text.to_string();
            }
            "attribute" => {
                let object = function_node.child_by_field_name("object").unwrap();
                let usages = self.parse_usages(&object, code, path, parent_guid, is_error);
                if let Some(last) = usages.last() {
                    decl.ast_fields.caller_guid = last.read().expect("the data might be broken").fields().parent_guid.clone();
                }
                symbols.extend(usages);
                let attribute = function_node.child_by_field_name("attribute").unwrap();
                decl.ast_fields.name = code.slice(attribute.byte_range()).to_string();
            }
            _ => {
                let usages = self.parse_usages(&function_node, code, path, parent_guid, is_error);
                if let Some(last) = usages.last() {
                    decl.ast_fields.caller_guid = last.read().expect("the data might be broken").fields().parent_guid.clone();
                }
                symbols.extend(usages);
            }
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }
}

impl AstLanguageParser for PythonParser {
    fn parse(&mut self, code: &str, path: &Url) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let parent_guid = get_guid();
        let symbols = self.parse_block(&tree.root_node(), code, path, &parent_guid, false);
        symbols
    }
}
