use std::path::PathBuf;
use std::string::ToString;
use std::sync::{Arc, RwLock};
use itertools::Itertools;

use similar::DiffableStr;
use tree_sitter::{Node, Parser, Range};
use tree_sitter_cpp::language;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, StructDeclaration, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{get_children_guids, get_guid, str_hash};

pub(crate) struct CppParser {
    pub parser: Parser,
}

static CPP_KEYWORDS: [&str; 92] = [
    "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor",
    "bool", "break", "case", "catch", "char", "char8_t", "char16_t", "char32_t",
    "class", "compl", "concept", "const", "consteval", "constexpr", "constinit",
    "const_cast", "continue", "co_await", "co_return", "co_yield", "decltype", "default",
    "delete", "do", "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern",
    "false", "float", "for", "friend", "goto", "if", "inline", "int", "long", "mutable",
    "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator", "or", "or_eq",
    "private", "protected", "public", "register", "reinterpret_cast", "requires", "return",
    "short", "signed", "sizeof", "static", "static_assert", "static_cast", "struct", "switch",
    "template", "this", "thread_local", "throw", "true", "try", "typedef", "typeid", "typename",
    "union", "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq"
];

pub fn parse_type(parent: &Node, code: &str) -> Option<TypeDef> {
    let kind = parent.kind();
    let text = code.slice(parent.byte_range()).to_string();
    match kind {
        "primitive_type" | "type_identifier" | "identifier" => {
            return Some(TypeDef {
                name: Some(text),
                inference_info: None,
                is_pod: kind == "primitive_type",
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            });
        }
        "type_descriptor" => {
            if let Some(type_node) = parent.child_by_field_name("type") {
                return parse_type(&type_node, code);
            }
        }
        "template_type" => {
            let mut type_ = TypeDef {
                name: None,
                inference_info: None,
                is_pod: false,
                namespace: "".to_string(),
                guid: None,
                nested_types: vec![],
            };
            if let Some(name) = parent.child_by_field_name("name") {
                type_.name = Some(code.slice(name.byte_range()).to_string());
            }
            if let Some(arguments) = parent.child_by_field_name("arguments") {
                for i in 0..arguments.child_count() {
                    let child = arguments.child(i).unwrap();
                    if let Some(dtype) = parse_type(&child, code) {
                        type_.nested_types.push(dtype);
                    }
                }
            }
            return Some(type_);
        }
        &_ => {}
    }
    None
}

impl CppParser {
    pub fn new() -> Result<CppParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(CppParser { parser })
    }

    pub fn parse_struct_declaration(
        &mut self, parent: &Node,
        code: &str, path: &PathBuf,
        parent_guid: &String,
        is_error: bool,
    ) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields.language = LanguageId::Cpp;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.declaration_range = parent.range();
        decl.ast_fields.definition_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        symbols.extend(self.find_error_usages(&parent, code, path, &decl.ast_fields.guid));

        if let Some(name) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        } else {
            decl.ast_fields.name = format!("anon-{}", decl.ast_fields.guid);
        }

        let mut template_parent_node = parent.parent();
        while let Some(parent) = template_parent_node {
            match parent.kind() {
                "enum_specifier" | "class_specifier" | "struct_specifier" |
                "template_declaration" | "namespace_definition" | "function_definition" => {
                    break;
                }
                &_ => {}
            }
            template_parent_node = parent.parent();
        }

        if let Some(template_parent) = template_parent_node {
            symbols.extend(self.find_error_usages(&template_parent, code, path, &decl.ast_fields.guid));
            if template_parent.kind() == "template_declaration" {
                if let Some(parameters) = template_parent.child_by_field_name("parameters") {
                    for i in 0..parameters.child_count() {
                        let child = parameters.child(i).unwrap();
                        symbols.extend(self.find_error_usages(&child, code, path, &decl.ast_fields.guid));
                        if let Some(arg) = parse_type(&child, code) {
                            decl.template_types.push(arg);
                        }
                    }
                }
            }
        }
        // find base classes
        for i in 0..parent.child_count() {
            let base_class_clause = parent.child(i).unwrap();
            symbols.extend(self.find_error_usages(&base_class_clause, code, path, &decl.ast_fields.guid));
            if base_class_clause.kind() == "base_class_clause" {
                for i in 0..base_class_clause.child_count() {
                    let child = base_class_clause.child(i).unwrap();
                    symbols.extend(self.find_error_usages(&child, code, path, &decl.ast_fields.guid));
                    if let Some(base_class) = parse_type(&child, code) {
                        decl.inherited_types.push(base_class);
                    }
                }
            }
        }
        if let Some(body) = parent.child_by_field_name("body") {
            decl.ast_fields.declaration_range = body.range();
            decl.ast_fields.definition_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.declaration_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.declaration_range.start_point,
            };
            symbols.extend(self.parse_usages(&body, code, path, &decl.ast_fields.guid, is_error));
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    fn parse_variable_definition(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut type_ = TypeDef::default();
        if let Some(type_node) = parent.child_by_field_name("type") {
            if vec!["class_specifier", "struct_specifier", "enum_specifier"].contains(&type_node.kind()) {
                let usages = self.parse_struct_declaration(&type_node, code, path, parent_guid, is_error);
                type_.guid = Some(usages.last().unwrap().read().unwrap().guid().to_string());
                type_.name = Some(usages.last().unwrap().read().unwrap().name().to_string());
                symbols.extend(usages);
            } else {
                if let Some(dtype) = parse_type(&type_node, code) {
                    type_ = dtype;
                }
            }
        }

        symbols.extend(self.find_error_usages(&parent, code, path, &parent_guid));

        let mut cursor = parent.walk();
        for child in parent.children_by_field_name("declarator", &mut cursor) {
            symbols.extend(self.find_error_usages(&child, code, path, &parent_guid));
            let (symbols_l, _, name_l, namespace_l) = 
                self.parse_declaration(&child, code, path, parent_guid, is_error);
            symbols.extend(symbols_l);
            
            let mut decl = VariableDefinition::default();
            decl.ast_fields.language = LanguageId::Cpp;
            decl.ast_fields.full_range = parent.range();
            decl.ast_fields.file_path = path.clone();
            decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
            decl.ast_fields.parent_guid = Some(parent_guid.clone());
            decl.ast_fields.guid = get_guid();
            decl.ast_fields.is_error = is_error;
            decl.type_ = type_.clone();
            decl.ast_fields.name = name_l;
            decl.ast_fields.namespace = namespace_l;
            decl.type_ = type_.clone();
            symbols.push(Arc::new(RwLock::new(decl)));
        }
        symbols
    }

    fn parse_field_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut dtype = TypeDef::default();
        if let Some(type_node) = parent.child_by_field_name("type") {
            if let Some(type_) = parse_type(&type_node, code) {
                dtype = type_;
            }
        }
        symbols.extend(self.find_error_usages(&parent, code, path, parent_guid));

        let mut cursor = parent.walk();
        let declarators = parent.children_by_field_name("declarator", &mut cursor).collect::<Vec<Node>>();
        cursor = parent.walk();
        let default_values = parent.children_by_field_name("default_value", &mut cursor).collect::<Vec<Node>>();

        let match_declarators_to_default_value = || {
            let mut result: Vec<(Node, Option<Node>)> = vec![];
            for i in 0..declarators.len() {
                let current = declarators.get(i).unwrap();
                let current_range = current.range();
                let next_mb = declarators.get(i + 1);
                
                let mut default_value_candidate = None;

                for default_value in &default_values {
                    let default_value_range = default_value.range();
                    if let Some(next) = next_mb {
                        let next_range = next.range();
                        if default_value_range.start_byte > current_range.end_byte && default_value_range.end_byte < next_range.start_byte {
                            default_value_candidate = Some(default_value.clone());
                            break;
                        }
                    } else {
                        if default_value_range.start_byte > current_range.end_byte {
                            default_value_candidate = Some(default_value.clone());
                            break;
                        }
                    }
                }
                result.push((current.clone(), default_value_candidate));
            }
            result
        };


        for (declarator, default_value_mb) in match_declarators_to_default_value() {
            let (symbols_l, _, name_l, _) = 
                self.parse_declaration(&declarator, code, path, parent_guid, is_error);
            if name_l.is_empty() {
                continue;
            }
            symbols.extend(symbols_l);

            let mut decl = ClassFieldDeclaration::default();
            decl.ast_fields.language = LanguageId::Cpp;
            decl.ast_fields.full_range = parent.range();
            decl.ast_fields.file_path = path.clone();
            decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
            decl.ast_fields.parent_guid = Some(parent_guid.clone());
            decl.ast_fields.guid = get_guid();
            decl.ast_fields.is_error = is_error;
            decl.ast_fields.name = name_l;
            
            let local_dtype = dtype.clone();
            if let Some(default_value) = default_value_mb {
                symbols.extend(self.parse_usages(&default_value, code, path, parent_guid, is_error));
                decl.type_.inference_info = Some(code.slice(default_value.byte_range()).to_string());
            }
            decl.type_ = local_dtype;
            symbols.push(Arc::new(RwLock::new(decl)));
        }
        symbols
    }

    fn parse_enum_field_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let mut decl = ClassFieldDeclaration::default();
        decl.ast_fields.language = LanguageId::Cpp;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;
        
        symbols.extend(self.find_error_usages(&parent, code, path, parent_guid));
        
        if let Some(name) = parent.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name.byte_range()).to_string();
        }
        if let Some(value) = parent.child_by_field_name("value") {
            decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
            symbols.extend(self.parse_usages(&value, code, path, parent_guid, is_error));
        }
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let kind = parent.kind();
        #[cfg(test)]
            let text = code.slice(parent.byte_range());
        match kind {
            "enum_specifier" | "class_specifier" | "struct_specifier" => {
                symbols.extend(self.parse_struct_declaration(&parent, code, path, parent_guid, is_error));
            }
            "declaration" => {
                symbols.extend(self.parse_variable_definition(&parent, code, path, parent_guid, is_error));
            }
            "function_definition" => {
                symbols.extend(self.parse_function_declaration(&parent, code, path, parent_guid, is_error));
            }
            "call_expression" => {
                symbols.extend(self.parse_call_expression(&parent, code, path, parent_guid, is_error));
            }
            "field_declaration" => {
                symbols.extend(self.parse_field_declaration(&parent, code, path, parent_guid, is_error));
            }
            "enumerator" => {
                symbols.extend(self.parse_enum_field_declaration(&parent, code, path, parent_guid, is_error));
            }
            "identifier" | "field_identifier" | "type_identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(parent.byte_range()).to_string();
                usage.ast_fields.language = LanguageId::Cpp;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = is_error;
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "field_expression" => {
                let mut usage = VariableUsage::default();
                if let Some(field) = parent.child_by_field_name("field") {
                    usage.ast_fields.name = code.slice(field.byte_range()).to_string();
                }
                usage.ast_fields.language = LanguageId::Cpp;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                if let Some(argument) = parent.child_by_field_name("argument") {
                    let usages = self.parse_usages(&argument, code, path, parent_guid, is_error);
                    // TODO check comments
                    if let Some(last) = usages.last() {
                        usage.ast_fields.caller_guid = Some(last.read().unwrap().guid().to_string());
                    }
                    symbols.extend(self.find_error_usages(&argument, code, path, parent_guid));
                    symbols.extend(usages);
                }
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "new_expression" => {
                if let Some(type_) = parent.child_by_field_name("type") {
                    symbols.extend(self.parse_usages(&type_, code, path, parent_guid, is_error));
                }
                if let Some(arguments) = parent.child_by_field_name("arguments") {
                    symbols.extend(self.parse_usages(&arguments, code, path, parent_guid, is_error));
                }
           
            }
            
            "comment" => {
                let mut def = CommentDefinition::default();
                def.ast_fields.language = LanguageId::Cpp;
                def.ast_fields.full_range = parent.range();
                def.ast_fields.file_path = path.clone();
                def.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                def.ast_fields.parent_guid = Some(parent_guid.clone());
                def.ast_fields.guid = get_guid();
                def.ast_fields.is_error = is_error;
                symbols.push(Arc::new(RwLock::new(def)));
            }
            "ERROR" => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.find_error_usages(&child, code, path, parent_guid));
                }
            }
            _ => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
                }
            }
        }
        symbols
    }

    fn parse_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> (Vec<AstSymbolInstanceArc>, Vec<TypeDef>, String, String) {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut types: Vec<TypeDef> = Default::default();
        let mut name: String = String::new();
        let mut namespace: String = String::new();
        #[cfg(test)]
            let text = code.slice(parent.byte_range());
        let kind = parent.kind();
        match kind {
            "identifier" | "field_identifier" => {
                name = code.slice(parent.byte_range()).to_string();
            }
            "template_function" | "template_type" => {
                if let Some(name_node) = parent.child_by_field_name("name") {
                    name = code.slice(name_node.byte_range()).to_string();
                    symbols.extend(self.find_error_usages(&name_node, code, path, &parent_guid));
                }
                if let Some(arguments_node) = parent.child_by_field_name("arguments") {
                    symbols.extend(self.find_error_usages(&arguments_node, code, path, &parent_guid));
                    self.find_error_usages(&arguments_node, code, path, &parent_guid);
                    for i in 0..arguments_node.child_count() {
                        let child = arguments_node.child(i).unwrap();
                        #[cfg(test)]
                            let text = code.slice(child.byte_range());
                        symbols.extend(self.find_error_usages(&child, code, path, &parent_guid));
                        self.find_error_usages(&child, code, path, &parent_guid);
                        if let Some(dtype) = parse_type(&child, code) {
                            types.push(dtype);
                        }
                    }
                }
            }
            "init_declarator" => {
                if let Some(declarator) = parent.child_by_field_name("declarator") {
                    let (symbols_l, _, name_l, _) = 
                        self.parse_declaration(&declarator, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    name = name_l;
                }
                if let Some(value) = parent.child_by_field_name("value") {
                    symbols.extend(self.parse_usages(&value, code, path, parent_guid, is_error));
                }
            }
            "qualified_identifier" => {
                if let Some(scope) = parent.child_by_field_name("scope") {
                    symbols.extend(self.find_error_usages(&scope, code, path, &parent_guid));
                    let (symbols_l, types_l, name_l, namespace_l) = 
                        self.parse_declaration(&scope, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    types.extend(types_l);
                    namespace = vec![namespace, name_l, namespace_l].iter().filter(|x| !x.is_empty()).join("::");
                }
                if let Some(name_node) = parent.child_by_field_name("name") {
                    symbols.extend(self.find_error_usages(&name_node, code, path, &parent_guid));
                    let (symbols_l, types_l, name_l, namespace_l) = 
                        self.parse_declaration(&name_node, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    types.extend(types_l);
                    name = name_l;
                    namespace = vec![namespace, namespace_l].iter().filter(|x| !x.is_empty()).join("::");
                }
            }
            "pointer_declarator" => {
                if let Some(declarator) = parent.child_by_field_name("declarator") {
                    let (symbols_l, _, name_l, _) = 
                        self.parse_declaration(&declarator, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    name = name_l;
                }
            }
            "reference_declarator" => {
                for i in 0..parent.child_count() {
                    let child = parent.child(i).unwrap();
                    symbols.extend(self.find_error_usages(&child, code, path, &parent_guid));
                    let (symbols_l, _, name_l, _) = 
                        self.parse_declaration(&child, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    if !name_l.is_empty() {
                        name = name_l;
                    }
                }
            }
            "parameter_declaration" => {
                if let Some(type_) = parent.child_by_field_name("type") {
                    if let Some(type_) = parse_type(&type_, code) {
                        types.push(type_);
                    }
                }
                if let Some(declarator) = parent.child_by_field_name("declarator") {
                    let (symbols_l, _, name_l, _) = 
                        self.parse_declaration(&declarator, code, path, parent_guid, is_error);
                    symbols.extend(symbols_l);
                    name = name_l;
                }
            }
            &_ => {}
        }

        (symbols, types, name, namespace)
    }

    pub fn parse_function_declaration(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.language = LanguageId::Cpp;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.declaration_range = parent.range();
        decl.ast_fields.definition_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.is_error = is_error;
        decl.ast_fields.guid = get_guid();

        symbols.extend(self.find_error_usages(&parent, code, path, &decl.ast_fields.guid));

        let mut template_parent_node = parent.parent();
        while let Some(parent) = template_parent_node {
            match parent.kind() {
                "enum_specifier" | "class_specifier" | "struct_specifier" |
                "template_declaration" | "namespace_definition" | "function_definition" => {
                    break;
                }
                &_ => {}
            }
            template_parent_node = parent.parent();
        }
        if let Some(template_parent) = template_parent_node {
            if template_parent.kind() == "template_declaration" {
                if let Some(parameters) = template_parent.child_by_field_name("parameters") {
                    for i in 0..parameters.child_count() {
                        let child = parameters.child(i).unwrap();
                        symbols.extend(self.find_error_usages(&child, code, path, &decl.ast_fields.guid));
                        if let Some(arg) = parse_type(&child, code) {
                            decl.template_types.push(arg);
                        }
                    }
                }
            }
        }

        if let Some(declarator) = parent.child_by_field_name("declarator") {
            symbols.extend(self.find_error_usages(&declarator, code, path, &decl.ast_fields.guid));
            if let Some(declarator) = declarator.child_by_field_name("declarator") {
                symbols.extend(self.find_error_usages(&declarator, code, path, &decl.ast_fields.guid));
                let (symbols_l, types_l, name_l, namespace_l) = 
                    self.parse_declaration(&declarator, code, path, parent_guid, is_error);
                symbols.extend(symbols_l);
                decl.ast_fields.name = name_l;
                decl.ast_fields.namespace = namespace_l;
                decl.template_types = types_l;
            }
            if let Some(parameters) = declarator.child_by_field_name("parameters") {
                symbols.extend(self.find_error_usages(&parameters, code, path, &decl.ast_fields.guid));
                for i in 0..parameters.child_count() {
                    let child = parameters.child(i).unwrap();
                    symbols.extend(self.find_error_usages(&child, code, path, &decl.ast_fields.guid));
                    match child.kind() {
                        "parameter_declaration" => {
                            let mut arg = FunctionArg::default();
                            if let Some(type_) = child.child_by_field_name("type") {
                                arg.type_ = parse_type(&type_, code);
                            }
                            if let Some(declarator) = child.child_by_field_name("declarator") {
                                let (symbols_l, _, name_l, _) = 
                                    self.parse_declaration(&declarator, code, path, parent_guid, is_error);
                                symbols.extend(symbols_l);
                                arg.name = name_l;
                            }
                            decl.args.push(arg);
                        }
                        &_ => {}
                    }
                }

                decl.ast_fields.declaration_range = Range {
                    start_byte: decl.ast_fields.full_range.start_byte,
                    end_byte: parameters.end_byte(),
                    start_point: decl.ast_fields.full_range.start_point,
                    end_point: parameters.end_position(),
                };
                decl.ast_fields.definition_range = decl.ast_fields.declaration_range;
            }
        }

        if let Some(return_type) = parent.child_by_field_name("type") {
            decl.return_type = parse_type(&return_type, code);
        }

        if let Some(body_node) = parent.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl.ast_fields.definition_range.start_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl.ast_fields.definition_range.start_point,
            };

            symbols.extend(self.parse_usages(&body_node, code, path, &decl.ast_fields.guid, is_error));
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }

    pub fn parse_call_expression(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String, is_error: bool) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionCall::default();
        decl.ast_fields.language = LanguageId::Cpp;
        decl.ast_fields.full_range = parent.range();
        decl.ast_fields.file_path = path.clone();
        decl.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
        decl.ast_fields.parent_guid = Some(parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = is_error;

        symbols.extend(self.find_error_usages(&parent, code, path, parent_guid));

        if let Some(function) = parent.child_by_field_name("function") {
            symbols.extend(self.find_error_usages(&function, code, path, parent_guid));
            let mut usages = self.parse_usages(&function, code, path, parent_guid, is_error);
            if let Some(last) = usages.pop() {
                decl.ast_fields = last.read().unwrap().fields().clone();
            }
            symbols.extend(usages);
        }
        if let Some(arguments) = parent.child_by_field_name("arguments") {
            symbols.extend(self.find_error_usages(&arguments, code, path, parent_guid));
            for i in 0..arguments.child_count() {
                let child = arguments.child(i).unwrap();
                symbols.extend(self.parse_usages(&child, code, path, parent_guid, is_error));
            }
        }
        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(decl)));
        symbols
    }


    fn find_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        for i in 0..parent.child_count() {
            let child = parent.child(i).unwrap();
            if child.kind() == "ERROR" {
                symbols.extend(self.parse_error_usages(&child, code, path, parent_guid));
            }
        }
        symbols
    }

    fn parse_error_usages(&mut self, parent: &Node, code: &str, path: &PathBuf, parent_guid: &String) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        match parent.kind() {
            "identifier" | "field_identifier" => {
                let text = code.slice(parent.byte_range());
                if CPP_KEYWORDS.contains(&text) {
                    return symbols;
                }
                
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = text.to_string();
                usage.ast_fields.language = LanguageId::Cpp;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(usage)));
            }
            "field_expression" => {
                let mut usage = VariableUsage::default();
                if let Some(field) = parent.child_by_field_name("field") {
                    usage.ast_fields.name = code.slice(field.byte_range()).to_string();
                }
                
                usage.ast_fields.language = LanguageId::Cpp;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.content_hash = str_hash(&code.slice(parent.byte_range()).to_string());
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                if let Some(argument) = parent.child_by_field_name("argument") {
                    let usages = self.parse_usages(&argument, code, path, parent_guid, true);
                    // TODO check comments
                    if let Some(last) = usages.last() {
                        usage.ast_fields.caller_guid = Some(last.read().unwrap().guid().to_string());
                    }
                    symbols.extend(self.find_error_usages(&argument, code, path, parent_guid));
                    symbols.extend(usages);
                }
                if CPP_KEYWORDS.contains(&usage.ast_fields.name.as_str()) {
                    symbols.push(Arc::new(RwLock::new(usage)));
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
}

impl AstLanguageParser for CppParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let parent_guid = get_guid();
        let symbols = self.parse_usages(&tree.root_node(), code, path, &parent_guid, false);
        symbols
    }
}


