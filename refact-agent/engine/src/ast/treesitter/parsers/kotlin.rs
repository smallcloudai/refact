use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;

#[cfg(test)]
use itertools::Itertools;

use parking_lot::RwLock;
use similar::DiffableStr;
use tree_sitter::{Node, Parser, Range};
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, ImportDeclaration, ImportType, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{CandidateInfo, get_guid};

pub(crate) struct KotlinParser {
    pub parser: Parser,
}

static KOTLIN_KEYWORDS: [&str; 64] = [
    "abstract", "actual", "annotation", "as", "break", "by", "catch", "class", "companion", "const",
    "constructor", "continue", "crossinline", "data", "do", "dynamic", "else", "enum", "expect", "external",
    "final", "finally", "for", "fun", "get", "if", "import", "in", "infix", "init", "inline", "inner",
    "interface", "internal", "is", "lateinit", "noinline", "object", "open", "operator", "out", "override",
    "package", "private", "protected", "public", "reified", "return", "sealed", "set", "super", "suspend",
    "tailrec", "this", "throw", "try", "typealias", "typeof", "val", "var", "vararg", "when", "where", "while"
];

static SYSTEM_MODULES: [&str; 2] = [
    "kotlin", "java",
];

pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    
    match kind {
        "type_identifier" | "identifier" | "user_type" => {
            return Some(TypeDef {
                name: Some(text),
                inference_info: None,
                inference_info_guid: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "void_type" | "integral_type" | "floating_point_type" | "boolean_type" => {
            return Some(TypeDef {
                name: None,
                inference_info: Some(text),
                inference_info_guid: None,
                is_pod: true,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "nullable_type" => {
            if let Some(non_null_type) = parent.child_by_field_name("type") {
                if let Some(mut dtype) = parse_type(&non_null_type, code) {
                    dtype.name = Some(format!("{}?", dtype.name.unwrap_or_default()));
                    return Some(dtype);
                }
            }
        }
        "generic_type" => {
            let mut decl = TypeDef {
                name: None,
                inference_info: None,
                inference_info_guid: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            for i in 0..parent.child_count() {
                let child = parent.child(i).unwrap();
                match child.kind() {
                    "type_identifier" => {
                        decl.name = Some(code.slice(child.byte_range()).to_string());
                    }
                    "type_arguments" => {
                        for i in 0..child.child_count() {
                            let child = child.child(i).unwrap();
                            if let Some(t) = parse_type(&child, code) {
                                decl.nested_types.push(t);
                            }
                        }
                    }
                    _ => {}
                }
            }
            return Some(decl);
        }
        "array_type" => {
            let mut decl = TypeDef {
                name: Some("[]".to_string()),
                inference_info: None,
                inference_info_guid: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            if let Some(element) = parent.child_by_field_name("element") {
                if let Some(dtype) = parse_type(&element, code) {
                    decl.nested_types.push(dtype);
                }
            }
            return Some(decl);
        }
        "scoped_type_identifier" => {
            let mut decl = TypeDef {
                name: None,
                inference_info: None,
                inference_info_guid: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };

            let mut parts = Vec::new();
            for i in 0..parent.child_count() {
                let child = parent.child(i).unwrap();
                if child.kind() == "type_identifier" {
                    parts.push(code.slice(child.byte_range()).to_string());
                    }
                    }
            
            if !parts.is_empty() {
                decl.name = Some(parts.join("."));
            }
            return Some(decl);
        }
        "function_type" => {
            let mut decl = TypeDef {
                name: Some("Function".to_string()),
                inference_info: None,
                inference_info_guid: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            
            if let Some(parameters) = parent.child_by_field_name("parameters") {
                for i in 0..parameters.child_count() {
                    let child = parameters.child(i).unwrap();
                    if let Some(t) = parse_type(&child, code) {
                        decl.nested_types.push(t);
                    }
                }
            }
            
            if let Some(return_type) = parent.child_by_field_name("return_type") {
                if let Some(t) = parse_type(&return_type, code) {
                    decl.nested_types.push(t);
                }
            }
            
            return Some(decl);
        }
        _ => {}
    }
    None
}

fn parse_function_arg(parent: &Node, code: &str) -> FunctionArg {
    let mut arg = FunctionArg::default();
    
    if let Some(name) = parent.child_by_field_name("name") {
        arg.name = code.slice(name.byte_range()).to_string();
    }

    if let Some(type_node) = parent.child_by_field_name("type") {
        if let Some(dtype) = parse_type(&type_node, code) {
                arg.type_ = Some(dtype);
        }
    }

    arg
}

impl KotlinParser {
    pub fn new() -> Result<KotlinParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
            .map_err(internal_error)?;
        Ok(KotlinParser { parser })
    }

    fn parse_class_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = StructDeclaration::default();

        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = info.ast_fields.is_error;

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name_node) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        } else {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                if child.kind() == "identifier" {
                    decl.ast_fields.name = code.slice(child.byte_range()).to_string();
                    break;
                }
            }
        }

        if let Some(node) = info.node.child_by_field_name("supertype") {
            symbols.extend(self.find_error_usages(&node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                if let Some(dtype) = parse_type(&child, code) {
                    decl.inherited_types.push(dtype);
                }
            }
        }
        
        if let Some(node) = info.node.child_by_field_name("delegation_specifiers") {
            symbols.extend(self.find_error_usages(&node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                match child.kind() {
                    "type_list" => {
                        for i in 0..child.child_count() {
                            let child = child.child(i).unwrap();
                            if let Some(dtype) = parse_type(&child, code) {
                                decl.inherited_types.push(dtype);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        if let Some(_) = info.node.child_by_field_name("type_parameters") {}

        if let Some(body) = info.node.child_by_field_name("body") {
            decl.ast_fields.definition_range = body.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        } else if let Some(body) = info.node.child_by_field_name("class_body") {
            decl.ast_fields.definition_range = body.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        } else if let Some(body) = info.node.child_by_field_name("members") {
            decl.ast_fields.definition_range = body.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        } else {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                if child.kind() == "class_body" || child.kind() == "body" || child.kind() == "members" || 
                   child.kind() == "{" || child.kind().contains("body") {
                    candidates.push_back(CandidateInfo {
                        ast_fields: decl.ast_fields.clone(),
                        node: child,
                        parent_guid: decl.ast_fields.guid.clone(),
                    });
                }
            }
        }

        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_function_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = FunctionDeclaration::default();

                    decl.ast_fields.language = info.ast_fields.language;
                    decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
                    decl.ast_fields.file_path = info.ast_fields.file_path.clone();
                    decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
                    decl.ast_fields.guid = get_guid();
                    decl.ast_fields.is_error = info.ast_fields.is_error;

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name_node) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        } else {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                if child.kind() == "identifier" {
                    decl.ast_fields.name = code.slice(child.byte_range()).to_string();
                    break;
                }
            }
        }

        if let Some(parameters_node) = info.node.child_by_field_name("parameters") {
            symbols.extend(self.find_error_usages(&parameters_node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: parameters_node.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: parameters_node.end_position(),
            };

            let mut function_args = vec![];
            for i in 0..parameters_node.child_count() {
                let child = parameters_node.child(i).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                if child.kind() == "parameter" {
                    function_args.push(parse_function_arg(&child, code));
                }
            }
            decl.args = function_args;
        }

        if let Some(return_type) = info.node.child_by_field_name("type") {
            decl.return_type = parse_type(&return_type, code);
            symbols.extend(self.find_error_usages(&return_type, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
        }

        if let Some(body_node) = info.node.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };
            
            for i in 0..body_node.child_count() {
                let child = body_node.child(i).unwrap();
                candidates.push_back(CandidateInfo {
                    ast_fields: {
                        let mut ast_fields = AstSymbolFields::default();
                        ast_fields.language = info.ast_fields.language;
                        ast_fields.full_range = child.range();
                        ast_fields.file_path = info.ast_fields.file_path.clone();
                        ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                        ast_fields.guid = get_guid();
                        ast_fields.is_error = false;
                        ast_fields.caller_guid = None;
                        ast_fields
                    },
                    node: child,
                    parent_guid: decl.ast_fields.guid.clone(),
                });
            }
        } else {
            decl.ast_fields.declaration_range = decl.ast_fields.full_range;
        }

        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_property_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        
        let mut decl = ClassFieldDeclaration::default();

                    decl.ast_fields.language = info.ast_fields.language;
                    decl.ast_fields.full_range = info.node.range();
                    decl.ast_fields.declaration_range = info.node.range();
                    decl.ast_fields.file_path = info.ast_fields.file_path.clone();
                    decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
                    decl.ast_fields.guid = get_guid();
                    decl.ast_fields.is_error = info.ast_fields.is_error;

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                
                if child.kind() == "variable_declaration" {
                    for j in 0..child.child_count() {
                        let subchild = child.child(j).unwrap();
                        if subchild.kind() == "identifier" {
                            decl.ast_fields.name = code.slice(subchild.byte_range()).to_string();
                            break;
                        }
                    }
                    if !decl.ast_fields.name.is_empty() {
                        break;
                    }
                } else if child.kind() == "identifier" {
                    decl.ast_fields.name = code.slice(child.byte_range()).to_string();
                    break;
                }
            }
        }

        if let Some(type_node) = info.node.child_by_field_name("type") {
            if let Some(dtype) = parse_type(&type_node, code) {
                decl.type_ = dtype;
            }
        } else {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                
                if child.kind() == "variable_declaration" {
                    for j in 0..child.child_count() {
                        let subchild = child.child(j).unwrap();
                        if subchild.kind() == "function_type" || subchild.kind() == "type_identifier" || 
                           subchild.kind() == "nullable_type" || subchild.kind() == "generic_type" ||
                           subchild.kind() == "user_type" {
                            if let Some(dtype) = parse_type(&subchild, code) {
                                decl.type_ = dtype;
                                break;
                            }
                        }
                    }
                    if decl.type_.name.is_some() {
                        break;
                    }
                } else if child.kind() == "function_type" || child.kind() == "type_identifier" || 
                          child.kind() == "nullable_type" || child.kind() == "generic_type" ||
                          child.kind() == "user_type" {
                    if let Some(dtype) = parse_type(&child, code) {
                        decl.type_ = dtype;
                        break;
                    }
                }
            }
        }

        if let Some(initializer) = info.node.child_by_field_name("initializer") {
            decl.type_.inference_info = Some(code.slice(initializer.byte_range()).to_string());
            
            for i in 0..initializer.child_count() {
                let child = initializer.child(i).unwrap();
                if child.kind() == "lambda_literal" || child.kind() == "lambda_expression" {
                        candidates.push_back(CandidateInfo {
                        ast_fields: {
                            let mut ast_fields = AstSymbolFields::default();
                            ast_fields.language = info.ast_fields.language;
                            ast_fields.full_range = child.range();
                            ast_fields.file_path = info.ast_fields.file_path.clone();
                            ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                            ast_fields.guid = get_guid();
                            ast_fields.is_error = false;
                            ast_fields.caller_guid = None;
                            ast_fields
                        },
                        node: child,
                        parent_guid: decl.ast_fields.guid.clone(),
                    });
                }
            }
        }

        for i in 0..info.node.child_count() {
            let child = info.node.child(i).unwrap();
            match child.kind() {
                "getter" | "setter" => {
                    candidates.push_back(CandidateInfo {
                        ast_fields: {
                            let mut ast_fields = AstSymbolFields::default();
                            ast_fields.language = info.ast_fields.language;
                            ast_fields.full_range = child.range();
                            ast_fields.file_path = info.ast_fields.file_path.clone();
                            ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                            ast_fields.guid = get_guid();
                            ast_fields.is_error = false;
                            ast_fields.caller_guid = None;
                            ast_fields
                        },
                        node: child,
                        parent_guid: decl.ast_fields.guid.clone(),
                    });
                }
                _ => {}
            }
        }

        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_variable_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, _candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut type_ = TypeDef::default();

        if let Some(type_node) = info.node.child_by_field_name("type") {
            if let Some(dtype) = parse_type(&type_node, code) {
                type_ = dtype;
            }
        }

        for i in 0..info.node.child_count() {
            let child = info.node.child(i).unwrap();
            match child.kind() {
                "variable_declarator" => {
                    let mut decl = VariableDefinition::default();
        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = info.ast_fields.is_error;
                    decl.type_ = type_.clone();

                    if let Some(name) = child.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }

                    if let Some(value) = child.child_by_field_name("value") {
                        decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                    }

                    symbols.push(Arc::new(RwLock::new(Box::new(decl))));
                }
                _ => {}
            }
        }

        symbols
    }

    fn parse_identifier<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, _candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let name = code.slice(info.node.byte_range()).to_string();
        
        if KOTLIN_KEYWORDS.contains(&name.as_str()) {
            return symbols;
        }

        let mut usage = VariableUsage::default();
        usage.ast_fields.name = name;
        usage.ast_fields.language = info.ast_fields.language;
        usage.ast_fields.full_range = info.node.range();
        usage.ast_fields.file_path = info.ast_fields.file_path.clone();
        usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
        usage.ast_fields.guid = get_guid();
        usage.ast_fields.is_error = info.ast_fields.is_error;
        if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
            usage.ast_fields.guid = caller_guid;
        }

        symbols.push(Arc::new(RwLock::new(Box::new(usage))));
        symbols
    }

    fn parse_call_expression<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = FunctionCall::default();

        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = info.ast_fields.is_error;
        if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
            decl.ast_fields.guid = caller_guid;
        }
        decl.ast_fields.caller_guid = Some(get_guid());

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(type_) = info.node.child_by_field_name("type") {
            symbols.extend(self.find_error_usages(&type_, code, &info.ast_fields.file_path, &info.parent_guid));
            if let Some(dtype) = parse_type(&type_, code) {
                if let Some(name) = dtype.name {
                    decl.ast_fields.name = name;
                } else {
                    decl.ast_fields.name = code.slice(type_.byte_range()).to_string();
                }
            } else {
                decl.ast_fields.name = code.slice(type_.byte_range()).to_string();
            }
        }
        if let Some(arguments) = info.node.child_by_field_name("arguments") {
            symbols.extend(self.find_error_usages(&arguments, code, &info.ast_fields.file_path, &info.parent_guid));
            let mut new_ast_fields = info.ast_fields.clone();
            new_ast_fields.caller_guid = None;
            for i in 0..arguments.child_count() {
                let child = arguments.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                    ast_fields: new_ast_fields.clone(),
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
            }
        if let Some(object) = info.node.child_by_field_name("receiver") {
                    candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: object,
                        parent_guid: info.parent_guid.clone(),
                    });
                }

        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
            }

    fn parse_annotation<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, _candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
                let mut usage = VariableUsage::default();
        
                usage.ast_fields.name = code.slice(info.node.byte_range()).to_string();
                usage.ast_fields.language = info.ast_fields.language;
                usage.ast_fields.full_range = info.node.range();
                usage.ast_fields.file_path = info.ast_fields.file_path.clone();
                usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = info.ast_fields.is_error;
        
        if usage.ast_fields.name.starts_with('@') {
            usage.ast_fields.name = usage.ast_fields.name[1..].to_string();
        }
        
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
        symbols
            }

    fn parse_field_access<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        
                if let (Some(object), Some(field)) = (info.node.child_by_field_name("receiver"), info.node.child_by_field_name("field")) {
                    let mut usage = VariableUsage::default();
                    usage.ast_fields.name = code.slice(field.byte_range()).to_string();
                    usage.ast_fields.language = info.ast_fields.language;
                    usage.ast_fields.full_range = info.node.range();
                    usage.ast_fields.file_path = info.ast_fields.file_path.clone();
                    usage.ast_fields.guid = get_guid();
                    usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                    usage.ast_fields.caller_guid = Some(get_guid());
                    if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                        usage.ast_fields.guid = caller_guid;
                    }
                    candidates.push_back(CandidateInfo {
                        ast_fields: usage.ast_fields.clone(),
                        node: object,
                        parent_guid: info.parent_guid.clone(),
                    });
                    symbols.push(Arc::new(RwLock::new(Box::new(usage))));
                }
        
        symbols
    }

    fn parse_lambda_expression<'a>(&mut self, info: &CandidateInfo<'a>, _code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let symbols: Vec<AstSymbolInstanceArc> = vec![];
        
        if let Some(parameters) = info.node.child_by_field_name("parameters") {
            for i in 0..parameters.child_count() {
                let child = parameters.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                    ast_fields: {
                        let mut ast_fields = AstSymbolFields::default();
                        ast_fields.language = info.ast_fields.language;
                        ast_fields.full_range = child.range();
                        ast_fields.file_path = info.ast_fields.file_path.clone();
                        ast_fields.parent_guid = Some(info.parent_guid.clone());
                        ast_fields.guid = get_guid();
                        ast_fields.is_error = false;
                        ast_fields.caller_guid = None;
                        ast_fields
                    },
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    });
            }
        }
        
        if let Some(body) = info.node.child_by_field_name("body") {
            for i in 0..body.child_count() {
                let child = body.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                    ast_fields: {
                        let mut ast_fields = AstSymbolFields::default();
                        ast_fields.language = info.ast_fields.language;
                        ast_fields.full_range = child.range();
                        ast_fields.file_path = info.ast_fields.file_path.clone();
                        ast_fields.parent_guid = Some(info.parent_guid.clone());
                        ast_fields.guid = get_guid();
                        ast_fields.is_error = false;
                        ast_fields.caller_guid = None;
                        ast_fields
                    },
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                });
                }
            }
        
        symbols
    }

    fn find_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            if child.kind() == "ERROR" {
                symbols.extend(self.parse_error_usages(&child, code, path, parent_guid));
            }
        }
        symbols
    }

    fn parse_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &Uuid) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        match parent.kind() {
            "identifier" => {
                let name = code.slice(parent.byte_range()).to_string();
                if KOTLIN_KEYWORDS.contains(&name.as_str()) {
                    return symbols;
                }

                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Kotlin;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
            }
            "field_access" | "navigation_expression" => {
                if let (Some(object), Some(field)) = (parent.child_by_field_name("receiver"), parent.child_by_field_name("field")) {
                    let usages = self.parse_error_usages(&object, code, path, parent_guid);
                    let mut usage = VariableUsage::default();
                    usage.ast_fields.name = code.slice(field.byte_range()).to_string();
                    usage.ast_fields.language = LanguageId::Kotlin;
                    usage.ast_fields.full_range = parent.range();
                    usage.ast_fields.file_path = path.clone();
                    usage.ast_fields.guid = get_guid();
                    usage.ast_fields.parent_guid = Some(parent_guid.clone());
                    if let Some(last) = usages.last() {
                        usage.ast_fields.caller_guid = last.read().fields().parent_guid.clone();
                    }
                    symbols.extend(usages);
                    if !KOTLIN_KEYWORDS.contains(&usage.ast_fields.name.as_str()) {
                        symbols.push(Arc::new(RwLock::new(Box::new(usage))));
                    }
                }
            }
            _ => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.parse_error_usages(&child, code, path, parent_guid));
                }
            }
        }
        symbols
    }

    fn parse_usages_<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let kind = info.node.kind();
        
        
        match kind {
            "class_declaration" | "interface_declaration" | "enum_declaration" | "object_declaration" => {
                self.parse_class_declaration(info, code, candidates)
            }
            "function_declaration" | "fun" | "method_declaration" | "method" | "constructor" | "init" | "getter" | "setter" |
            "function" | "member_function" | "class_function" | "method_definition" | "function_definition" => {
                self.parse_function_declaration(info, code, candidates)
            }
            "property_declaration" | "val" | "var" | "property" | "mutable_property" | "immutable_property" | "lateinit" |
            "val_declaration" | "var_declaration" | "const_declaration" | "member_property" | "class_property" => {
                self.parse_property_declaration(info, code, candidates)
            }
            "companion_object" => {
                let symbols: Vec<AstSymbolInstanceArc> = vec![];
                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                        ast_fields: {
                            let mut ast_fields = AstSymbolFields::default();
                            ast_fields.language = info.ast_fields.language;
                            ast_fields.full_range = child.range();
                            ast_fields.file_path = info.ast_fields.file_path.clone();
                            ast_fields.parent_guid = Some(info.parent_guid.clone());
                            ast_fields.guid = get_guid();
                            ast_fields.is_error = false;
                            ast_fields.caller_guid = None;
                            ast_fields
                        },
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
                symbols
            }
            "local_variable_declaration" | "variable_declaration" => {
                self.parse_variable_declaration(info, code, candidates)
            }
            "call_expression" | "function_call" => {
                self.parse_call_expression(info, code, candidates)
            }
            "lambda_literal" | "lambda_expression" => {
                self.parse_lambda_expression(info, code, candidates)
            }
            "identifier" => {
                self.parse_identifier(info, code, candidates)
            }
            "field_access" | "navigation_expression" => {
                self.parse_field_access(info, code, candidates)
            }
            "annotation" => {
                self.parse_annotation(info, code, candidates)
            }
            "import_declaration" => {
                let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
                let mut def = ImportDeclaration::default();
                def.ast_fields.language = info.ast_fields.language;
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.file_path = info.ast_fields.file_path.clone();
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                def.ast_fields.guid = get_guid();
                
                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                    if ["scoped_identifier", "identifier"].contains(&child.kind()) {
                        let path = code.slice(child.byte_range()).to_string();
                        def.path_components = path.split(".").map(|x| x.to_string()).collect();
                        if let Some(first) = def.path_components.first() {
                            if SYSTEM_MODULES.contains(&first.as_str()) {
                                def.import_type = ImportType::System;
                            }
                        }
                    }
                }
                
                symbols.push(Arc::new(RwLock::new(Box::new(def))));
        symbols
    }
            "block_comment" | "line_comment" => {
                let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
                let mut def = CommentDefinition::default();
                def.ast_fields.language = info.ast_fields.language;
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.file_path = info.ast_fields.file_path.clone();
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                def.ast_fields.guid = get_guid();
                def.ast_fields.is_error = info.ast_fields.is_error;
                symbols.push(Arc::new(RwLock::new(Box::new(def))));
                symbols
            }
            "ERROR" => {
                let symbols: Vec<AstSymbolInstanceArc> = vec![];
                let mut ast = info.ast_fields.clone();
                ast.is_error = true;

                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                        ast_fields: ast.clone(),
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
                symbols
            }
            "package_declaration" => {
                vec![]
            }
            "class_body" | "members" | "body" => {
                let symbols: Vec<AstSymbolInstanceArc> = vec![];
                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                candidates.push_back(CandidateInfo {
                        ast_fields: {
                            let mut ast_fields = AstSymbolFields::default();
                            ast_fields.language = info.ast_fields.language;
                            ast_fields.full_range = child.range();
                            ast_fields.file_path = info.ast_fields.file_path.clone();
                            ast_fields.parent_guid = Some(info.parent_guid.clone());
                            ast_fields.guid = get_guid();
                            ast_fields.is_error = false;
                            ast_fields.caller_guid = None;
                            ast_fields
                        },
                    node: child,
                    parent_guid: info.parent_guid.clone(),
                });
            }
                symbols
            }
            _ => {
                let symbols: Vec<AstSymbolInstanceArc> = vec![];
                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                candidates.push_back(CandidateInfo {
                        ast_fields: {
                            let mut ast_fields = AstSymbolFields::default();
                            ast_fields.language = info.ast_fields.language;
                            ast_fields.full_range = child.range();
                            ast_fields.file_path = info.ast_fields.file_path.clone();
                            ast_fields.parent_guid = Some(info.parent_guid.clone());
                            ast_fields.guid = get_guid();
                            ast_fields.is_error = false;
                            ast_fields.caller_guid = None;
                            ast_fields
                        },
                        node: child,
                    parent_guid: info.parent_guid.clone(),
                });
            }
        symbols
            }
        }
    }

    fn parse_(&mut self, parent: &Node, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut ast_fields = AstSymbolFields::default();
        ast_fields.file_path = path.clone();
        ast_fields.is_error = false;
        ast_fields.language = LanguageId::Kotlin;

        let mut candidates = VecDeque::from(vec![CandidateInfo {
            ast_fields,
            node: parent.clone(),
            parent_guid: get_guid(),
        }]);

        while let Some(candidate) = candidates.pop_front() {
            let symbols_l = self.parse_usages_(&candidate, code, &mut candidates);
            symbols.extend(symbols_l);
        }

        let guid_to_symbol_map: HashMap<Uuid, AstSymbolInstanceArc> = symbols.iter()
            .map(|s| (s.read().guid().clone(), s.clone()))
            .collect();

        for symbol in symbols.iter_mut() {
            let guid = symbol.read().guid().clone();
            if let Some(parent_guid) = symbol.read().parent_guid() {
                if let Some(parent) = guid_to_symbol_map.get(parent_guid) {
                    parent.write().fields_mut().childs_guid.push(guid);
                }
            }
        }

        #[cfg(test)]
        for symbol in symbols.iter_mut() {
            let mut sym = symbol.write();
            sym.fields_mut().childs_guid = sym.fields_mut().childs_guid.iter()
                .sorted_by_key(|x| {
                    guid_to_symbol_map.get(*x).unwrap().read().full_range().start_byte
                }).map(|x| x.clone()).collect();
        }

        symbols
    }
}

impl AstLanguageParser for KotlinParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let symbols = self.parse_(&tree.root_node(), code, path);
        symbols
    }
}