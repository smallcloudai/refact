use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::string::ToString;
use std::sync::{Arc, RwLock};
#[allow(unused_imports)]
use itertools::Itertools;

use similar::DiffableStr;
use tree_sitter::{Node, Parser, Range};
use tree_sitter_typescript::language_typescript as language;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{CandidateInfo, get_guid, str_hash};

pub(crate) struct TSParser {
    pub parser: Parser,
}

pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    match kind {
        "type_annotation" => {
            for i in 0..parent.child_count() {
                let child = parent.child(i).unwrap();
                if let Some(nested_dtype) = parse_type(&child, code) {
                    return Some(nested_dtype);
                }
            }
        }
        "type_parameter" => {
            if let Some(name) = parent.child_by_field_name("name") {
                return Some(TypeDef {
                    name: Some(code.slice(name.byte_range()).to_string()),
                    inference_info: None,
                    is_pod: false,
                    namespace: "".to_string(),
                    guid: None,
                    nested_types: vec![],
                });
            }
        }
        "predefined_type" | "type_identifier" | "identifier" => {
            return Some(TypeDef {
                name: Some(text),
                inference_info: None,
                is_pod: kind == "predefined_type",
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "generic_type" => {
            let mut dtype = TypeDef {
                name: None,
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };

            if let Some(name) = parent.child_by_field_name("name") {
                dtype.name = Some(code.slice(name.byte_range()).to_string());
            }
            if let Some(type_arguments) = parent.child_by_field_name("type_arguments") {
                for i in 0..type_arguments.child_count() {
                    let child = type_arguments.child(i).unwrap();
                    if let Some(nested_dtype) = parse_type(&child, code) {
                        dtype.nested_types.push(nested_dtype);
                    }
                }
            }
            return Some(dtype);
        }
        "union_type" | "array_type" | "tuple_type" => {
            let mut dtype = TypeDef {
                name: Some(kind[0..kind.len() - 5].to_string()),
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            for i in 0..parent.child_count() {
                let child = parent.child(i).unwrap();
                if let Some(nested_dtype) = parse_type(&child, code) {
                    dtype.nested_types.push(nested_dtype);
                }
            }
            return Some(dtype);
        }
        "function_type" => {
            let mut dtype = TypeDef {
                name: Some("function".to_string()),
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            if let Some(parameters) = parent.child_by_field_name("parameters") {
                for i in 0..parameters.child_count() {
                    let child = parameters.child(i).unwrap();
                    if let Some(type_) = child.child_by_field_name("type") {
                        if let Some(dtype_) = parse_type(&type_, code) {
                            dtype.nested_types.push(dtype_);
                        }
                    }
                }
            }
            if let Some(return_type) = parent.child_by_field_name("return_type") {
                if let Some(dtype_) = parse_type(&return_type, code) {
                    dtype.nested_types.push(dtype_);
                }
            }
            return Some(dtype);
        }
        &_ => {}
    }
    None
}

impl TSParser {
    pub fn new() -> Result<Self, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(Self { parser })
    }

    pub fn parse_struct_declaration<'a>(
        &mut self,
        info: &CandidateInfo<'a>,
        code: &str,
        candidates: &mut VecDeque<CandidateInfo<'a>>)
        -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else {
            decl.ast_fields.name = format!("anon-{}", decl.ast_fields.guid);
        }

        if let Some(type_parameters) = info.node.child_by_field_name("type_parameters") {
            for i in 0..type_parameters.child_count() {
                let child = type_parameters.child(i).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                if let Some(dtype) = parse_type(&child, code) {
                    decl.template_types.push(dtype);
                }
            }
        }

        // find base classes
        for i in 0..info.node.child_count() {
            let class_heritage = info.node.child(i).unwrap();
            symbols.extend(self.find_error_usages(&class_heritage, code, &info.ast_fields.file_path,
                                                  &decl.ast_fields.guid));
            if class_heritage.kind() == "class_heritage" {
                decl.ast_fields.declaration_range = Range {
                    start_byte: decl.ast_fields.full_range.start_byte,
                    end_byte: class_heritage.end_byte(),
                    start_point: decl.ast_fields.full_range.start_point,
                    end_point: class_heritage.end_position(),
                };

                for i in 0..class_heritage.child_count() {
                    let extends_clause = class_heritage.child(i).unwrap();
                    symbols.extend(self.find_error_usages(&extends_clause, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                    if extends_clause.kind() == "extends_clause" {
                        let mut current_dtype: Option<TypeDef> = None;
                        for i in 0..extends_clause.child_count() {
                            let child = extends_clause.child(i).unwrap();
                            if let Some(field_name) = extends_clause.field_name_for_child(i as u32) {
                                match field_name {
                                    "value" => {
                                        if let Some(current_dtype) = &current_dtype {
                                            decl.inherited_types.push(current_dtype.clone());
                                        }
                                        if let Some(dtype) = parse_type(&child, code) {
                                            current_dtype = Some(dtype);
                                        }
                                    }
                                    "type_arguments" => {
                                        for i in 0..child.child_count() {
                                            let child = child.child(i).unwrap();
                                            symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                                            if let Some(dtype) = parse_type(&child, code) {
                                                if let Some(current_dtype) = current_dtype.as_mut() {
                                                    current_dtype.nested_types.push(dtype);
                                                }
                                            }
                                        }
                                    }
                                    &_ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut body_mb = info.node.child_by_field_name("body");
        // type_alias_declaration
        if let None = body_mb {
            body_mb = info.node.child_by_field_name("value");
        }

        if let Some(body) = body_mb {
            decl.ast_fields.declaration_range = body.range();
            decl.ast_fields.definition_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.declaration_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.declaration_range.start_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body,
                parent_guid: decl.ast_fields.guid.clone(),
            })
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    fn parse_variable_definition<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));

        let mut decl = VariableDefinition::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(type_node) = info.node.child_by_field_name("type") {
            if let Some(type_) = parse_type(&type_node, code) {
                decl.type_ = type_;
            }
        }
        if let Some(value) = info.node.child_by_field_name("value") {
            decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
            candidates.push_back(CandidateInfo {
                ast_fields: info.ast_fields.clone(),
                node: value,
                parent_guid: info.parent_guid.clone(),
            });
        }

        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    fn parse_field_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, _: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = ClassFieldDeclaration::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(type_) = info.node.child_by_field_name("type") {
            if let Some(type_) = parse_type(&type_, code) {
                decl.type_ = type_;
            }
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    fn parse_enum_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = StructDeclaration::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &decl.ast_fields.file_path, &info.parent_guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(body) = info.node.child_by_field_name("body") {
            for i in 0..body.child_count() {
                let child = body.child(i).unwrap();
                let kind = child.kind();
                match kind {
                    "enum_assignment" => {
                        let mut field = ClassFieldDeclaration::default();
                        field.ast_fields = AstSymbolFields::from_fields(&decl.ast_fields);
                        field.ast_fields.full_range = child.range();
                        field.ast_fields.declaration_range = child.range();
                        field.ast_fields.content_hash = str_hash(&code.slice(child.byte_range()).to_string());
                        field.ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                        field.ast_fields.guid = get_guid();
                        if let Some(name) = child.child_by_field_name("name") {
                            field.ast_fields.name = code.slice(name.byte_range()).to_string();
                        }
                        if let Some(value) = child.child_by_field_name("value") {
                            field.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                        }
                        symbols.push(Arc::new(RwLock::new(field)));
                    }
                    "property_identifier" => {
                        let mut field = ClassFieldDeclaration::default();
                        field.ast_fields = AstSymbolFields::from_fields(&decl.ast_fields);
                        field.ast_fields.full_range = child.range();
                        field.ast_fields.declaration_range = child.range();
                        field.ast_fields.content_hash = str_hash(&code.slice(child.byte_range()).to_string());
                        field.ast_fields.parent_guid = Some(decl.ast_fields.guid.clone());
                        field.ast_fields.guid = get_guid();
                        field.ast_fields.name = code.slice(child.byte_range()).to_string();
                        symbols.push(Arc::new(RwLock::new(field)));
                    }
                    &_ => {
                        candidates.push_back(CandidateInfo {
                            ast_fields: decl.ast_fields.clone(),
                            node: child,
                            parent_guid: info.parent_guid.clone(),
                        });
                    }
                }
            }
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_function_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &decl.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }

        if let Some(type_parameters) = info.node.child_by_field_name("type_parameters") {
            for i in 0..type_parameters.child_count() {
                let child = type_parameters.child(i).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                if let Some(dtype) = parse_type(&child, code) {
                    decl.template_types.push(dtype);
                }
            }
        }

        if let Some(parameters) = info.node.child_by_field_name("parameters") {
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: parameters.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: parameters.end_position(),
            };
            symbols.extend(self.find_error_usages(&parameters, code, &decl.ast_fields.file_path, &decl.ast_fields.guid));
            for i in 0..parameters.child_count() {
                let child = parameters.child(i).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                match child.kind() {
                    "optional_parameter" | "required_parameter" => {
                        let mut arg = FunctionArg::default();
                        if let Some(pattern) = child.child_by_field_name("pattern") {
                            arg.name = code.slice(pattern.byte_range()).to_string();
                        }
                        if let Some(type_) = child.child_by_field_name("type") {
                            arg.type_ = parse_type(&type_, code);
                        }
                        if let Some(value) = child.child_by_field_name("value") {
                            if let Some(dtype) = arg.type_.as_mut() {
                                dtype.inference_info = Some(code.slice(value.byte_range()).to_string());
                            } else {
                                let mut dtype = TypeDef::default();
                                dtype.inference_info = Some(code.slice(value.byte_range()).to_string());
                                arg.type_ = Some(dtype);
                            }
                        }
                        decl.args.push(arg);
                    }
                    &_ => {
                        candidates.push_back(CandidateInfo {
                            ast_fields: decl.ast_fields.clone(),
                            node: child,
                            parent_guid: decl.ast_fields.guid.clone(),
                        });
                    }
                }
            }
        }

        if let Some(return_type) = info.node.child_by_field_name("return_type") {
            decl.return_type = parse_type(&return_type, code);
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: return_type.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: return_type.end_position(),
            };
        }

        if let Some(body_node) = info.node.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body_node,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_call_expression<'a>(
        &mut self,
        info: &CandidateInfo<'a>,
        code: &str,
        candidates: &mut VecDeque<CandidateInfo<'a>>)
        -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionCall::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
            decl.ast_fields.guid = caller_guid;
        }
        decl.ast_fields.caller_guid = Some(get_guid());

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));

        if let Some(function) = info.node.child_by_field_name("function") {
            let kind = function.kind();
            match kind {
                "identifier" => {
                    decl.ast_fields.name = code.slice(function.byte_range()).to_string();
                }
                "member_expression" => {
                    if let Some(property) = function.child_by_field_name("property") {
                        decl.ast_fields.name = code.slice(property.byte_range()).to_string();
                    }
                    if let Some(object) = function.child_by_field_name("object") {
                        candidates.push_back(CandidateInfo {
                            ast_fields: decl.ast_fields.clone(),
                            node: object,
                            parent_guid: info.parent_guid.clone(),
                        });
                    }
                }
                &_ => {
                    candidates.push_back(CandidateInfo {
                        ast_fields: decl.ast_fields.clone(),
                        node: function,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
            }
        }

        if let Some(type_arguments) = info.node.child_by_field_name("type_arguments") {
            for i in 0..type_arguments.child_count() {
                let child = type_arguments.child(i).unwrap();
                if let Some(type_) = parse_type(&child, code) {
                    decl.template_types.push(type_);
                } else {
                    candidates.push_back(CandidateInfo {
                        ast_fields: decl.ast_fields.clone(),
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
            }
        }

        if let Some(arguments) = info.node.child_by_field_name("arguments") {
            for i in 0..arguments.child_count() {
                let child = arguments.child(i).unwrap();
                candidates.push_back(CandidateInfo {
                    ast_fields: info.ast_fields.clone(),
                    node: child,
                    parent_guid: info.parent_guid.clone(),
                });
            }
        }
        symbols.push(Arc::new(RwLock::new(decl)));
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
            "identifier" /*| "field_identifier"*/ => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.language = LanguageId::TypeScript;
                usage.ast_fields.is_error = true;
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                // if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                //     usage.ast_fields.guid = caller_guid;
                // }
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "member_expression" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.language = LanguageId::TypeScript;
                usage.ast_fields.is_error = true;
                if let Some(property) = parent.child_by_field_name("property") {
                    usage.ast_fields.name = code.slice(property.byte_range()).to_string();
                }
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.guid = get_guid();
                // if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                //     usage.ast_fields.guid = caller_guid;
                // }
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.caller_guid = Some(get_guid());
                if let Some(object) = parent.child_by_field_name("object") {
                    symbols.extend(self.find_error_usages(&object, code, path, parent_guid));
                }
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

    fn parse_usages_<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];

        let kind = info.node.kind();
        #[cfg(test)]
            let text = code.slice(info.node.byte_range());
        match kind {
            "class_declaration" | "class" | "interface_declaration" | "type_alias_declaration" => {
                symbols.extend(self.parse_struct_declaration(info, code, candidates));
            }
            /*"lexical_declaration" |*/ "variable_declarator" => {
                symbols.extend(self.parse_variable_definition(info, code, candidates));
            }
            "function_declaration" | "method_definition" | "arrow_function" | "function_expression" => {
                symbols.extend(self.parse_function_declaration(info, code, candidates));
            }
            "call_expression" => {
                symbols.extend(self.parse_call_expression(info, code, candidates));
            }
            "property_signature" | "public_field_definition" => {
                symbols.extend(self.parse_field_declaration(info, code, candidates));
            }
            "enum_declaration" => {
                symbols.extend(self.parse_enum_declaration(info, code, candidates));
            }
            "identifier" /*| "field_identifier"*/ => {
                let mut usage = VariableUsage::default();
                usage.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
                usage.ast_fields.name = code.slice(info.node.byte_range()).to_string();
                usage.ast_fields.full_range = info.node.range();
                usage.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                    usage.ast_fields.guid = caller_guid;
                }
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "member_expression" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
                if let Some(property) = info.node.child_by_field_name("property") {
                    usage.ast_fields.name = code.slice(property.byte_range()).to_string();
                }
                usage.ast_fields.full_range = info.node.range();
                usage.ast_fields.guid = get_guid();
                if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                    usage.ast_fields.guid = caller_guid;
                }
                usage.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                usage.ast_fields.caller_guid = Some(get_guid());
                if let Some(object) = info.node.child_by_field_name("object") {
                    candidates.push_back(CandidateInfo {
                        ast_fields: usage.ast_fields.clone(),
                        node: object,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "new_expression" => {
                if let Some(constructor) = info.node.child_by_field_name("constructor") {
                    candidates.push_back(CandidateInfo {
                        ast_fields: info.ast_fields.clone(),
                        node: constructor,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
                if let Some(arguments) = info.node.child_by_field_name("arguments") {
                    candidates.push_back(CandidateInfo {
                        ast_fields: info.ast_fields.clone(),
                        node: arguments,
                        parent_guid: info.parent_guid.clone(),
                    })
                }
            }
            "comment" => {
                let mut def = CommentDefinition::default();
                def.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.content_hash = str_hash(&code.slice(info.node.byte_range()).to_string());
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                def.ast_fields.guid = get_guid();
                symbols.push(Arc::new(RwLock::new(def)));
            }
            "ERROR" => {
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
            }
            _ => {
                for i in 0..info.node.child_count() {
                    let child = info.node.child(i).unwrap();
                    candidates.push_back(CandidateInfo {
                        ast_fields: info.ast_fields.clone(),
                        node: child,
                        parent_guid: info.parent_guid.clone(),
                    })
                }
            }
        }
        symbols
    }

    fn parse_(&mut self, parent: &Node, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut ast_fields = AstSymbolFields::default();
        ast_fields.file_path = path.clone();
        ast_fields.is_error = false;
        ast_fields.language = LanguageId::TypeScript;

        let mut candidates = VecDeque::from(vec![CandidateInfo {
            ast_fields,
            node: parent.clone(),
            parent_guid: get_guid(),
        }]);
        while let Some(candidate) = candidates.pop_front() {
            let symbols_l = self.parse_usages_(&candidate, code, &mut candidates);
            symbols.extend(symbols_l);
        }
        let guid_to_symbol_map = symbols.iter()
            .map(|s| (s.clone().read().unwrap().guid().clone(), s.clone())).collect::<HashMap<_, _>>();
        for symbol in symbols.iter_mut() {
            let guid = symbol.read().unwrap().guid().clone();
            if let Some(parent_guid) = symbol.read().unwrap().parent_guid() {
                if let Some(parent) = guid_to_symbol_map.get(parent_guid) {
                    parent.write().unwrap().fields_mut().childs_guid.push(guid);
                }
            }
        }

        #[cfg(test)]
        for symbol in symbols.iter_mut() {
            let mut sym = symbol.write().unwrap();
            sym.fields_mut().childs_guid = sym.fields_mut().childs_guid.iter()
                .sorted_by_key(|x| {
                    guid_to_symbol_map.get(*x).unwrap().read().unwrap().full_range().start_byte
                }).map(|x| x.clone()).collect();
        }

        symbols
    }
}

impl AstLanguageParser for TSParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let symbols = self.parse_(&tree.root_node(), code, path);
        symbols
    }
}


