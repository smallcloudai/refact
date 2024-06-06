use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use itertools::Itertools;

use parking_lot::RwLock;
use similar::DiffableStr;
use tree_sitter::{Node, Parser, Range};
use tree_sitter_java::language;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, ImportDeclaration, ImportType, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{CandidateInfo, get_guid};

pub(crate) struct JavaParser {
    pub parser: Parser,
}

static JAVA_KEYWORDS: [&str; 50] = [
    "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class", "const",
    "continue", "default", "do", "double", "else", "enum", "extends", "final", "finally", "float",
    "for", "if", "goto", "implements", "import", "instanceof", "int", "interface", "long", "native",
    "new", "package", "private", "protected", "public", "return", "short", "static", "strictfp", "super",
    "switch", "synchronized", "this", "throw", "throws", "transient", "try", "void", "volatile", "while"
];

static SYSTEM_MODULES: [&str; 2] = [
    "java", "jdk",
];

pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    match kind {
        "type_parameters" | "type_list" => {
            let child = parent.child(0).unwrap();
            return parse_type(&child, code);
        }
        "type_identifier" | "identifier" => {
            return Some(TypeDef {
                name: Some(text),
                inference_info: None,
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
                is_pod: true,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "generic_type" => {
            let mut decl = TypeDef {
                name: None,
                inference_info: None,
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
                    &_ => {}
                }
            }

            return Some(decl);
        }
        "array_type" => {
            let mut decl = TypeDef {
                name: Some("[]".to_string()),
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            if let Some(dimensions) = parent.child_by_field_name("dimensions") {
                decl.name = Some(code.slice(dimensions.byte_range()).to_string());
            }

            if let Some(element) = parent.child_by_field_name("element") {
                if let Some(dtype) = parse_type(&element, code) {
                    decl.nested_types.push(dtype);
                }
            }
            return Some(decl);
        }
        "type_parameter" => {
            let mut def = TypeDef::default();
            for i in 0..parent.child_count() {
                let child = parent.child(i).unwrap();
                match child.kind() {
                    "type_identifier" => {
                        def.name = Some(code.slice(child.byte_range()).to_string());
                    }
                    "type_bound" => {
                        if let Some(dtype) = parse_type(&child, code) {
                            def.nested_types.push(dtype);
                        }
                    }
                    &_ => {}
                }
            }
        }
        "scoped_type_identifier" => {
            fn _parse(&parent: &Node, code: &str) -> String {
                let mut result = String::default();
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    match child.kind() {
                        "type_identifier" => {
                            if result.is_empty() {
                                result = code.slice(child.byte_range()).to_string();
                            } else {
                                result = result + "." + &*code.slice(child.byte_range()).to_string();
                            }
                        }
                        "scoped_type_identifier" => {
                            if result.is_empty() {
                                result = _parse(&child, code);
                            } else {
                                result = _parse(&child, code) + "." + &*result;
                            }
                        }
                        &_ => {}
                    }
                }
                result
            }
            let mut decl = TypeDef {
                name: None,
                inference_info: None,
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
                    "scoped_type_identifier" => {
                        decl.namespace = _parse(&child, code);
                    }
                    &_ => {}
                }
            }
            return Some(decl);
        }
        &_ => {}
    }
    None
}

fn parse_function_arg(parent: &Node, code: &str) -> FunctionArg {
    let mut arg = FunctionArg::default();
    if let Some(name) = parent.child_by_field_name("name") {
        arg.name = code.slice(name.byte_range()).to_string();
    }
    if let Some(dimensions) = parent.child_by_field_name("dimensions") {
        arg.type_ = Some(TypeDef {
            name: Some(code.slice(dimensions.byte_range()).to_string()),
            inference_info: None,
            is_pod: false,
            namespace: "".to_string(),
            guid: None,
            nested_types: vec![],
        })
    }

    if let Some(type_node) = parent.child_by_field_name("type") {
        if let Some(dtype) = parse_type(&type_node, code) {
            if let Some(arg_dtype) = &mut arg.type_ {
                arg_dtype.nested_types.push(dtype);
            } else {
                arg.type_ = Some(dtype);
            }
        }
    }

    arg
}

impl JavaParser {
    pub fn new() -> Result<JavaParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(JavaParser { parser })
    }

    pub fn parse_struct_declaration<'a>(
        &mut self,
        info: &CandidateInfo<'a>,
        code: &str,
        candidates: &mut VecDeque<CandidateInfo<'a>>)
        -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
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
        }

        if let Some(node) = info.node.child_by_field_name("superclass") {
            symbols.extend(self.find_error_usages(&node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                if let Some(dtype) = parse_type(&child, code) {
                    decl.inherited_types.push(dtype);
                }
            }
        }
        if let Some(node) = info.node.child_by_field_name("interfaces") {
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
                    &_ => {}
                }
            }
        }
        if let Some(_) = info.node.child_by_field_name("type_parameters") {}


        if let Some(body) = info.node.child_by_field_name("body") {
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
        
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_variable_definition<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut type_ = TypeDef::default();
        if let Some(type_node) = info.node.child_by_field_name("type") {
            symbols.extend(self.find_error_usages(&type_node, code,  &info.ast_fields.file_path, &info.parent_guid));
            if let Some(dtype) = parse_type(&type_node, code) {
                type_ = dtype;
            }
        }

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));

        for i in 0..info.node.child_count() {
            let child = info.node.child(i).unwrap();
            symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &info.parent_guid));
            match child.kind() {
                "variable_declarator" => {
                    let local_dtype = type_.clone();
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
                        symbols.extend(self.find_error_usages(&value, code, &info.ast_fields.file_path, &info.parent_guid));
                        decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                        candidates.push_back(CandidateInfo {
                            ast_fields: decl.ast_fields.clone(),
                            node: value,
                            parent_guid: info.parent_guid.clone(),
                        });
                    }
                    if let Some(dimensions) = child.child_by_field_name("dimensions") {
                        symbols.extend(self.find_error_usages(&dimensions, code, &info.ast_fields.file_path, &info.parent_guid));
                        decl.type_ = TypeDef {
                            name: Some(code.slice(dimensions.byte_range()).to_string()),
                            inference_info: None,
                            is_pod: false,
                            namespace: "".to_string(),
                            guid: None,
                            nested_types: vec![local_dtype],
                        };
                    } else {
                        decl.type_ = local_dtype;
                    }
                    symbols.push(Arc::new(RwLock::new(Box::new(decl))));
                }
                &_ => {}
            }
        }

        symbols
    }

    fn parse_field_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut dtype = TypeDef::default();
        if let Some(type_node) = info.node.child_by_field_name("type") {
            symbols.extend(self.find_error_usages(&type_node, code, &info.ast_fields.file_path, &info.parent_guid));
            if let Some(type_) = parse_type(&type_node, code) {
                dtype = type_;
            }
        }

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));

        for i in 0..info.node.child_count() {
            let child = info.node.child(i).unwrap();
            match child.kind() {
                "variable_declarator" => {
                    let local_dtype = dtype.clone();

                    let mut decl = ClassFieldDeclaration::default();
                    decl.ast_fields.language = info.ast_fields.language;
                    decl.ast_fields.full_range = info.node.range();
                    decl.ast_fields.file_path = info.ast_fields.file_path.clone();
                    decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
                    decl.ast_fields.guid = get_guid();
                    decl.ast_fields.is_error = info.ast_fields.is_error;
                    if let Some(name) = child.child_by_field_name("name") {
                        decl.ast_fields.name = code.slice(name.byte_range()).to_string();
                    }
                    if let Some(value) = child.child_by_field_name("value") {
                        symbols.extend(self.find_error_usages(&value, code, &info.ast_fields.file_path, &info.parent_guid));
                        decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                        candidates.push_back(CandidateInfo {
                            ast_fields: info.ast_fields.clone(),
                            node: value,
                            parent_guid: info.parent_guid.clone(),
                        });
                    }
                    if let Some(dimensions) = child.child_by_field_name("dimensions") {
                        symbols.extend(self.find_error_usages(&dimensions, code, &info.ast_fields.file_path, &info.parent_guid));
                        decl.type_ = TypeDef {
                            name: Some(code.slice(dimensions.byte_range()).to_string()),
                            inference_info: None,
                            is_pod: false,
                            namespace: "".to_string(),
                            guid: None,
                            nested_types: vec![local_dtype],
                        };
                    } else {
                        decl.type_ = local_dtype;
                    }
                    symbols.push(Arc::new(RwLock::new(Box::new(decl))));
                }
                _ => {}
            }
        }
        symbols
    }

    fn parse_enum_field_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = ClassFieldDeclaration::default();
        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = info.ast_fields.is_error;
        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));
        
        if let Some(name) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(arguments) = info.node.child_by_field_name("arguments") {
            symbols.extend(self.find_error_usages(&arguments, code, &info.ast_fields.file_path, &info.parent_guid));
            decl.type_.inference_info = Some(code.slice(arguments.byte_range()).to_string());
            for i in 0..arguments.child_count() {
                let child = arguments.child(i).unwrap();
                if let Some(dtype) = parse_type(&child, code) {
                    decl.type_.nested_types.push(dtype);
                }
                candidates.push_back(CandidateInfo {
                    ast_fields: info.ast_fields.clone(),
                    node: child,
                    parent_guid: info.parent_guid.clone(),
                });
            }
        }
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_usages_<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let kind = info.node.kind();
        #[cfg(test)]
        #[allow(unused)]
            let text = code.slice(info.node.byte_range());
        match kind {
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                symbols.extend(self.parse_struct_declaration(info, code, candidates));
            }
            "local_variable_declaration" => {
                symbols.extend(self.parse_variable_definition(info, code, candidates));
            }
            "method_declaration" => {
                symbols.extend(self.parse_function_declaration(info, code, candidates));
            }
            "method_invocation" | "object_creation_expression" => {
                symbols.extend(self.parse_call_expression(info, code, candidates));
            }
            "field_declaration" => {
                symbols.extend(self.parse_field_declaration(info, code, candidates));
            }
            "enum_constant" => {
                symbols.extend(self.parse_enum_field_declaration(info, code, candidates));
            }
            "identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(info.node.byte_range()).to_string();
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
            }
            "field_access" => {
                let object = info.node.child_by_field_name("object").unwrap();
                let field = info.node.child_by_field_name("field").unwrap();
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
            "block_comment" | "line_comment" => {
                let mut def = CommentDefinition::default();
                def.ast_fields.language = info.ast_fields.language;
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.file_path = info.ast_fields.file_path.clone();
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                def.ast_fields.guid = get_guid();
                def.ast_fields.is_error = info.ast_fields.is_error;
                symbols.push(Arc::new(RwLock::new(Box::new(def))));
            }
            "import_declaration" => {
                let mut def = ImportDeclaration::default();
                def.ast_fields.language = info.ast_fields.language;
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.file_path = info.ast_fields.file_path.clone();
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
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                def.ast_fields.guid = get_guid();
                symbols.push(Arc::new(RwLock::new(Box::new(def))));
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
            "identifier" => {
                let name = code.slice(parent.byte_range()).to_string();
                if JAVA_KEYWORDS.contains(&name.as_str()) {
                    return symbols;
                }
                
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Java;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
            }
            "field_access" => {
                let object = parent.child_by_field_name("object").unwrap();
                let usages = self.parse_error_usages(&object, code, path, parent_guid);
                let field = parent.child_by_field_name("field").unwrap();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(field.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Java;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                if let Some(last) = usages.last() {
                    usage.ast_fields.caller_guid = last.read().fields().parent_guid.clone();
                }
                symbols.extend(usages);
                if !JAVA_KEYWORDS.contains(&usage.ast_fields.name.as_str()) {
                    symbols.push(Arc::new(RwLock::new(Box::new(usage))));
                }
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

    pub fn parse_function_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.declaration_range = info.node.range();
        decl.ast_fields.definition_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.is_error = info.ast_fields.is_error;
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(name_node) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }

        if let Some(parameters_node) = info.node.child_by_field_name("parameters") {
            symbols.extend(self.find_error_usages(&parameters_node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: parameters_node.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: parameters_node.end_position(),
            };

            let params_len = parameters_node.child_count();
            let mut function_args = vec![];
            for idx in 0..params_len {
                let child = parameters_node.child(idx).unwrap();
                symbols.extend(self.find_error_usages(&child, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
                function_args.push(parse_function_arg(&child, code));
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
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body_node,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        }
        
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    pub fn parse_call_expression<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
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
            if let Some(dtype) =  parse_type(&type_, code) { 
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
            symbols.extend(self.find_error_usages(&arguments, code, &info.ast_fields.file_path,
                                                  &info.parent_guid));
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
        if let Some(object) = info.node.child_by_field_name("object") {
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: object,
                parent_guid: info.parent_guid.clone(),
            });
        }
        
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_(&mut self, parent: &Node, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut ast_fields = AstSymbolFields::default();
        ast_fields.file_path = path.clone();
        ast_fields.is_error = false;
        ast_fields.language = LanguageId::Java;

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

impl AstLanguageParser for JavaParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let symbols = self.parse_(&tree.root_node(), code, path);
        symbols
    }
}
