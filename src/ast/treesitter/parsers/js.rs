use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use parking_lot::RwLock;

use similar::DiffableStr;
use tree_sitter::{Node, Parser, Range};
use tree_sitter_javascript::language;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{CandidateInfo, get_guid};

pub(crate) struct JSParser {
    pub parser: Parser,
}

static LAMBDA_KINDS: [&str; 2] = ["function_expression", "arrow_function"];

fn parse_type_from_value(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    return match kind {
        "number" | "null" | "string" | "true" | "false" | "undefined" => {
            Some(TypeDef {
                name: None,
                inference_info: Some(text),
                is_pod: true,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            })
        }
        &_ => {
            Some(TypeDef {
                name: None,
                inference_info: Some(text),
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            })
        }
    }
}

fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    match kind {
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

impl JSParser {
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
        candidates: &mut VecDeque<CandidateInfo<'a>>,
        name_from_var: Option<String>)
        -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else if let Some(name) = name_from_var {
            decl.ast_fields.name = name;
        } else {
            decl.ast_fields.name = format!("anon-{}", decl.ast_fields.guid);
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
                    if let Some(dtype) = parse_type(&extends_clause, code) {
                        decl.inherited_types.push(dtype);
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
        } else if info.node.kind() == "object" {
            for i in 0..info.node.child_count() {
                let child = info.node.child(i).unwrap();
                candidates.push_back(CandidateInfo {
                    ast_fields: decl.ast_fields.clone(),
                    node: child,
                    parent_guid: decl.ast_fields.guid.clone(),
                })
            }
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
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(value) = info.node.child_by_field_name("value") {
            match value.kind() {
                "number" | "string" | "boolean" | "null" | "undefined" | "false" | "true" => {
                    decl.type_.is_pod = true;
                }
                &_ => {}
            }
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

    fn parse_field_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = ClassFieldDeclaration::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        if let Some(name) = info.node.child_by_field_name("property") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else if let Some(key) = info.node.child_by_field_name("key") {
            decl.ast_fields.name = code.slice(key.byte_range()).to_string();
        } else if info.node.kind() == "shorthand_property_identifier" {
            decl.ast_fields.name = code.slice(info.node.byte_range()).to_string();
        }

        if let Some(value) = info.node.child_by_field_name("value") {
            if let Some(value) = parse_type_from_value(&value, code) {
                decl.type_ = value;
            }
            candidates.push_back(CandidateInfo {
                ast_fields: info.ast_fields.clone(),
                node: value,
                parent_guid: info.parent_guid.clone(),
            })
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_function_declaration<'a>(
        &mut self,
        info: &CandidateInfo<'a>,
        code: &str, candidates:
        &mut VecDeque<CandidateInfo<'a>>,
        name_from_var: Option<String>,
    )
        -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &decl.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else if let Some(name) = name_from_var {
            decl.ast_fields.name = name.clone();
        } else {
            decl.ast_fields.name = format!("lambda-{}", decl.ast_fields.guid);
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
                let kind = child.kind();
                match kind {
                    "identifier" => {
                        let mut arg = FunctionArg::default();
                        arg.name = code.slice(child.byte_range()).to_string();
                        decl.args.push(arg);
                    }
                    "assignment_pattern" => {
                        let mut arg = FunctionArg::default();
                        if let Some(left) = child.child_by_field_name("left") {
                            arg.name = code.slice(left.byte_range()).to_string();
                        }
                        if let Some(right) = child.child_by_field_name("right") {
                            arg.type_ = parse_type_from_value(&right, code);
                            candidates.push_back(CandidateInfo {
                                ast_fields: info.ast_fields.clone(),
                                node: right,
                                parent_guid: info.ast_fields.guid.clone(),
                            })
                        }
                    }
                    &_ => {
                        candidates.push_back(CandidateInfo {
                            ast_fields: info.ast_fields.clone(),
                            node: child,
                            parent_guid: info.ast_fields.guid.clone(),
                        });
                    }
                }
            }
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
        #[allow(unused)]
        let text = code.slice(info.node.byte_range());
        match kind {
            "object" | "class_declaration" => {
                symbols.extend(self.parse_struct_declaration(info, code, candidates, None));
            }
            "variable_declarator" => {
                if let Some(value) = info.node.child_by_field_name("value") {
                    let kind = value.kind();
                    if let Some(name) = info.node.child_by_field_name("name") {
                        let name = code.slice(name.byte_range()).to_string();
                        let new_info = CandidateInfo {
                            ast_fields: info.ast_fields.clone(),
                            node: value,
                            parent_guid: info.parent_guid.clone(),
                        };
                        if LAMBDA_KINDS.contains(&kind) {
                            symbols.extend(self.parse_function_declaration(&new_info, code, candidates, Some(name)));
                        } else if kind == "class" {
                            symbols.extend(self.parse_struct_declaration(&new_info, code, candidates, Some(name)));
                        } else {
                            symbols.extend(self.parse_variable_definition(info, code, candidates));
                        }
                    } else {
                        symbols.extend(self.parse_variable_definition(info, code, candidates));
                    }
                } else {
                    symbols.extend(self.parse_variable_definition(info, code, candidates));
                }
            }
            "method_definition" | "function_declaration" => {
                symbols.extend(self.parse_function_declaration(info, code, candidates, None));
            }
            "call_expression" => {
                symbols.extend(self.parse_call_expression(info, code, candidates));
            }
            "pair" => {
                if let Some(parent) = info.node.parent() {
                    if parent.kind() == "object" {
                        let value = info.node.child_by_field_name("value").unwrap();
                        if LAMBDA_KINDS.contains(&value.kind()) {
                            let name = info.node.child_by_field_name("key").unwrap();
                            let name = code.slice(name.byte_range()).to_string();
                            let new_info = CandidateInfo {
                                ast_fields: info.ast_fields.clone(),
                                node: value,
                                parent_guid: info.parent_guid.clone(),
                            };
                            symbols.extend(self.parse_function_declaration(&new_info, code, candidates, Some(name)));
                        } else {
                            symbols.extend(self.parse_field_declaration(info, code, candidates));
                        }
                    } else {
                        for i in 0..info.node.child_count() {
                            let child = info.node.child(i).unwrap();
                            candidates.push_back(CandidateInfo {
                                ast_fields: info.ast_fields.clone(),
                                node: child,
                                parent_guid: info.parent_guid.clone(),
                            })
                        }
                    }
                } else {
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
            "field_definition" | "shorthand_property_identifier" => {
                symbols.extend(self.parse_field_declaration(info, code, candidates));
            }
            "identifier" /*| "field_identifier"*/ => {
                let mut usage = VariableUsage::default();
                usage.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
                usage.ast_fields.name = code.slice(info.node.byte_range()).to_string();
                usage.ast_fields.full_range = info.node.range();
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
            "comment" => {
                let mut def = CommentDefinition::default();
                def.ast_fields = AstSymbolFields::from_fields(&info.ast_fields);
                def.ast_fields.full_range = info.node.range();
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
        ast_fields.language = LanguageId::from(language());

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
            .map(|s| (s.clone().read().guid().clone(), s.clone())).collect::<HashMap<_, _>>();
        for symbol in symbols.iter_mut() {
            let guid = symbol.read().guid().clone();
            if let Some(parent_guid) = symbol.read().parent_guid() {
                if let Some(parent) = guid_to_symbol_map.get(parent_guid) {
                    parent.write().fields_mut().childs_guid.push(guid);
                }
            }
        }

        #[cfg(test)]
        {
            use itertools::Itertools;
            for symbol in symbols.iter_mut() {
                let mut sym = symbol.write();
                sym.fields_mut().childs_guid = sym.fields_mut().childs_guid.iter()
                    .sorted_by_key(|x| {
                        guid_to_symbol_map.get(*x).unwrap().read().full_range().start_byte
                    }).map(|x| x.clone()).collect();
            }
        }

        symbols
    }
}

impl AstLanguageParser for JSParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let symbols = self.parse_(&tree.root_node(), code, path);
        symbols
    }
}


