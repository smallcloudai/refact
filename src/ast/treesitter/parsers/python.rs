use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;

#[cfg(test)]
use itertools::Itertools;
use parking_lot::RwLock;
use similar::DiffableStr;
use tree_sitter::{Node, Parser, Point, Range};
use tree_sitter_python::language;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc, ClassFieldDeclaration, CommentDefinition, FunctionArg, FunctionCall, FunctionDeclaration, ImportDeclaration, ImportType, StructDeclaration, SymbolInformation, TypeDef, VariableDefinition, VariableUsage};
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::{AstLanguageParser, internal_error, ParserError};
use crate::ast::treesitter::parsers::utils::{CandidateInfo, get_children_guids, get_guid};
use crate::ast::treesitter::skeletonizer::SkeletonFormatter;
use crate::ast::treesitter::structs::SymbolType;

static PYTHON_MODULES: [&str; 203] = [
    "abc", "aifc", "argparse", "array", "asynchat", "asyncio", "asyncore", "atexit", "audioop",
    "base64", "bdb", "binascii", "binhex", "bisect", "builtins", "bz2", "calendar", "cgi", "cgitb",
    "chunk", "cmath", "cmd", "code", "codecs", "codeop", "collections", "colorsys", "compileall",
    "concurrent", "configparser", "contextlib", "contextvars", "copy", "copyreg", "crypt", "csv",
    "ctypes", "curses", "datetime", "dbm", "decimal", "difflib", "dis", "distutils", "doctest",
    "email", "encodings", "ensurepip", "enum", "errno", "faulthandler", "fcntl", "filecmp",
    "fileinput", "fnmatch", "formatter", "fractions", "ftplib", "functools", "gc", "getopt",
    "getpass", "gettext", "glob", "grp", "gzip", "hashlib", "heapq", "hmac", "html", "http",
    "idlelib", "imaplib", "imghdr", "imp", "importlib", "inspect", "io", "ipaddress", "itertools",
    "json", "keyword", "lib2to3", "linecache", "locale", "logging", "lzma", "macpath", "mailbox",
    "mailcap", "marshal", "math", "mimetypes", "mmap", "modulefinder", "msilib", "msvcrt",
    "multiprocessing", "netrc", "nntplib", "numbers", "operator", "optparse", "os", "ossaudiodev",
    "parser", "pathlib", "pdb", "pickle", "pickletools", "pipes", "pkgutil", "platform", "plistlib",
    "poplib", "posix", "pprint", "profile", "pstats", "pty", "pwd", "py_compile", "pyclbr", "pydoc",
    "queue", "quopri", "random", "re", "readline", "reprlib", "resource", "rlcompleter", "runpy",
    "sched", "secrets", "select", "selectors", "shelve", "shlex", "shutil", "signal", "site", "smtpd",
    "smtplib", "sndhdr", "socket", "socketserver", "spwd", "sqlite3", "ssl", "stat", "statistics",
    "string", "stringprep", "struct", "subprocess", "sunau", "symbol", "symtable", "sys", "sysconfig",
    "syslog", "tabnanny", "tarfile", "telnetlib", "tempfile", "termios", "test", "textwrap",
    "threading", "time", "timeit", "tkinter", "token", "tokenize", "trace", "traceback",
    "tracemalloc", "tty", "turtle", "turtledemo", "types", "typing", "unicodedata", "unittest",
    "urllib", "uu", "uuid", "venv", "warnings", "wave", "weakref", "webbrowser", "winreg", "winsound",
    "wsgiref", "xdrlib", "xml", "xmlrpc", "zipapp", "zipfile", "zipimport", "zoneinfo"
];


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
const PYTHON_KEYWORDS: [&'static str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
    "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
    "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
    "return", "try", "while", "with", "yield"
];

impl PythonParser {
    pub fn new() -> Result<PythonParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(&language())
            .map_err(internal_error)?;
        Ok(PythonParser { parser })
    }

    pub fn parse_struct_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = StructDeclaration::default();

        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        decl.ast_fields.is_error = info.ast_fields.is_error;

        symbols.extend(self.find_error_usages(&info.node, code, &decl.ast_fields.file_path, &decl.ast_fields.guid));

        if let Some(parent_node) = info.node.parent() {
            if parent_node.kind() == "decorated_definition" {
                decl.ast_fields.full_range = parent_node.range();
            }
        }

        if let Some(name_node) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: name_node.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: name_node.end_position(),
            }
        }
        if let Some(superclasses) = info.node.child_by_field_name("superclasses") {
            for i in 0..superclasses.child_count() {
                let child = superclasses.child(i).unwrap();
                if let Some(dtype) = parse_type(&child, code) {
                    decl.inherited_types.push(dtype);
                }
            }
            symbols.extend(self.find_error_usages(&superclasses, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: superclasses.end_byte(),
                start_point: decl.ast_fields.full_range.start_point,
                end_point: superclasses.end_position(),
            }
        }
        if let Some(body) = info.node.child_by_field_name("body") {
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body,
                parent_guid: decl.ast_fields.guid.clone(),
            });

            decl.ast_fields.definition_range = body.range();
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_assignment<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut is_class_field = false;
        {
            let mut parent_mb = info.node.parent();
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
        if let Some(right) = info.node.child_by_field_name("right") {
            candidates.push_back(CandidateInfo {
                ast_fields: info.ast_fields.clone(),
                node: right,
                parent_guid: info.parent_guid.clone(),
            });
        }
        if let Some(body) = info.node.child_by_field_name("body") {
            candidates.push_back(CandidateInfo {
                ast_fields: info.ast_fields.clone(),
                node: body,
                parent_guid: info.parent_guid.clone(),
            });
        }

        let mut candidates_: VecDeque<(Option<Node>, Option<Node>, Option<Node>)> = VecDeque::from(vec![
            (info.node.child_by_field_name("left"),
             info.node.child_by_field_name("type"),
             info.node.child_by_field_name("right"))]);
        let mut right_for_all = false;
        while !candidates_.is_empty() {
            let (left_mb, type_mb, right_mb) = candidates_.pop_front().unwrap();
            if let Some(left) = left_mb {
                let text = code.slice(left.byte_range());
                if SPECIAL_SYMBOLS.contains(text) || text == "self" {
                    continue;
                }
                let kind = left.kind();
                match kind {
                    "identifier" => {
                        let mut fields = AstSymbolFields::default();
                        fields.language = info.ast_fields.language;
                        fields.full_range = info.node.range();
                        fields.file_path = info.ast_fields.file_path.clone();
                        fields.parent_guid = Some(info.parent_guid.clone());
                        fields.guid = get_guid();
                        fields.name = code.slice(left.byte_range()).to_string();
                        fields.is_error = info.ast_fields.is_error;

                        if is_class_field {
                            let mut decl = ClassFieldDeclaration::default();
                            decl.ast_fields = fields;
                            if let Some(type_node) = type_mb {
                                if let Some(type_) = parse_type(&type_node, code) {
                                    decl.type_ = type_;
                                }
                            }
                            symbols.push(Arc::new(RwLock::new(Box::new(decl))));
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
                            symbols.push(Arc::new(RwLock::new(Box::new(decl))));
                        }
                    }
                    "attribute" => {
                        candidates.push_back(CandidateInfo {
                            ast_fields: info.ast_fields.clone(),
                            node: left,
                            parent_guid: info.parent_guid.clone(),
                        });
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
                            candidates_.push_back((*lefts.get(i).unwrap(), None, r));
                        }
                    }
                    "list_splat_pattern" => {
                        let child = left.child(0);
                        candidates_.push_back((child, type_mb, right_mb));
                    }
                    &_ => {}
                }
            }
        }

        // https://github.com/tree-sitter/tree-sitter-python/blob/master/grammar.js#L844
        symbols
    }

    fn parse_usages_<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = vec![];
        let kind = info.node.kind();
        let _text = code.slice(info.node.byte_range());
        // TODO lambda https://github.com/tree-sitter/tree-sitter-python/blob/master/grammar.js#L830
        match kind {
            "class_definition" => {
                symbols.extend(self.parse_struct_declaration(info, code, candidates));
            }
            "function_definition" | "lambda" => {
                symbols.extend(self.parse_function_declaration(info, code, candidates));
            }
            "decorated_definition" => {
                if let Some(definition) = info.node.child_by_field_name("definition") {
                    candidates.push_back(CandidateInfo {
                        ast_fields: info.ast_fields.clone(),
                        node: definition,
                        parent_guid: info.parent_guid.clone(),
                    });
                }
            }
            "as_pattern" => {
                let value = info.node.child(0).unwrap();
                if let Some(alias) = info.node.child_by_field_name("alias") {
                    let mut candidates_ = VecDeque::from(vec![alias.child(0).unwrap()]);
                    while !candidates_.is_empty() {
                        let child = candidates_.pop_front().unwrap();
                        let text = code.slice(child.byte_range());
                        if SPECIAL_SYMBOLS.contains(text) || text == "self" {
                            continue;
                        }
                        match child.kind() {
                            "identifier" => {
                                let mut decl = VariableDefinition::default();
                                decl.ast_fields.language = info.ast_fields.language;
                                decl.ast_fields.full_range = info.node.range();
                                decl.ast_fields.file_path = info.ast_fields.file_path.clone();
                                decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
                                decl.ast_fields.guid = get_guid();
                                decl.ast_fields.name = text.to_string();
                                decl.type_.inference_info = Some(code.slice(value.byte_range()).to_string());
                                decl.ast_fields.is_error = info.ast_fields.is_error;
                                symbols.push(Arc::new(RwLock::new(Box::new(decl))));
                            }
                            "list" | "set" | "tuple" => {
                                for i in 0..child.child_count() {
                                    candidates_.push_back(child.child(i).unwrap());
                                }
                            }
                            &_ => {
                                candidates.push_back(CandidateInfo {
                                    ast_fields: info.ast_fields.clone(),
                                    node: child,
                                    parent_guid: info.parent_guid.clone(),
                                });
                            }
                        }
                    }
                }
            }
            "identifier" => {
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = code.slice(info.node.byte_range()).to_string();
                usage.ast_fields.language = info.ast_fields.language;
                usage.ast_fields.full_range = info.node.range();
                usage.ast_fields.file_path = info.ast_fields.file_path.clone();
                usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                    usage.ast_fields.guid = caller_guid;
                }
                usage.ast_fields.is_error = info.ast_fields.is_error;
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
            }
            "attribute" => {
                let attribute = info.node.child_by_field_name("attribute").unwrap();
                let name = code.slice(attribute.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = info.ast_fields.language;
                usage.ast_fields.full_range = info.node.range();
                usage.ast_fields.file_path = info.ast_fields.file_path.clone();
                usage.ast_fields.parent_guid = Some(info.parent_guid.clone());
                usage.ast_fields.caller_guid = Some(get_guid());
                usage.ast_fields.guid = get_guid();
                if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
                    usage.ast_fields.guid = caller_guid;
                }
                usage.ast_fields.is_error = info.ast_fields.is_error;

                let object_node = info.node.child_by_field_name("object").unwrap();
                candidates.push_back(CandidateInfo {
                    ast_fields: usage.ast_fields.clone(),
                    node: object_node,
                    parent_guid: info.parent_guid.clone(),
                });
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
            }
            "assignment" | "for_statement" => {
                symbols.extend(self.parse_assignment(info, code, candidates));
            }
            "call" => {
                symbols.extend(self.parse_call_expression(info, code, candidates));
            }
            "comment" | "string" => {
                let mut is_block = false;
                if let Some(parent_) = info.node.parent() {
                    is_block |= vec!["module", "block"].contains(&parent_.kind());
                    if let Some(parent_) = parent_.parent() {
                        is_block |= vec!["module", "block"].contains(&parent_.kind());
                    }
                }

                if kind != "string" || is_block {
                    let mut def = CommentDefinition::default();
                    def.ast_fields.language = info.ast_fields.language;
                    def.ast_fields.full_range = info.node.range();
                    def.ast_fields.file_path = info.ast_fields.file_path.clone();
                    def.ast_fields.parent_guid = Some(info.parent_guid.clone());
                    def.ast_fields.guid = get_guid();
                    def.ast_fields.is_error = false;
                    symbols.push(Arc::new(RwLock::new(Box::new(def))));
                }
            }
            "import_from_statement" | "import_statement" => {
                let mut def = ImportDeclaration::default();
                def.ast_fields.language = info.ast_fields.language;
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.file_path = info.ast_fields.file_path.clone();
                def.ast_fields.full_range = info.node.range();
                def.ast_fields.parent_guid = Some(info.parent_guid.clone());

                let mut base_path_component: Vec<String> = Default::default();
                if let Some(module_name) = info.node.child_by_field_name("module_name") {
                    if module_name.kind() == "relative_import" {
                        let base_path = code.slice(module_name.byte_range()).to_string();
                        if base_path.starts_with("..") {
                            base_path_component.push("..".to_string());
                            base_path_component.extend(base_path.slice(2..base_path.len()).split(".")
                                .map(|x| x.to_string())
                                .filter(|x| !x.is_empty())
                                .collect::<Vec<String>>());
                        } else if base_path.starts_with(".") {
                            base_path_component.push(".".to_string());
                            base_path_component.extend(base_path.slice(1..base_path.len()).split(".")
                                .map(|x| x.to_string())
                                .filter(|x| !x.is_empty())
                                .collect::<Vec<String>>());
                        } else {
                            base_path_component = base_path.split(".")
                                .map(|x| x.to_string())
                                .filter(|x| !x.is_empty())
                                .collect();
                        }
                    } else {
                        base_path_component = code.slice(module_name.byte_range()).to_string().split(".")
                            .map(|x| x.to_string())
                            .filter(|x| !x.is_empty())
                            .collect();
                    }
                }
                def.path_components = base_path_component.clone();
                if info.node.child_by_field_name("name").is_some() {
                    let mut cursor = info.node.walk();
                    for child in info.node.children_by_field_name("name", &mut cursor) {
                        let mut def_local = def.clone();
                        def_local.ast_fields.guid = get_guid();

                        let mut path_components: Vec<String> = Default::default();
                        let mut alias: Option<String> = None;
                        match child.kind() {
                            "dotted_name" => {
                                path_components = code.slice(child.byte_range()).to_string().split(".").map(|x| x.to_string()).collect();
                            }
                            "aliased_import" => {
                                if let Some(name) = child.child_by_field_name("name") {
                                    path_components = code.slice(name.byte_range()).to_string().split(".").map(|x| x.to_string()).collect();
                                }
                                if let Some(alias_node) = child.child_by_field_name("alias") {
                                    alias = Some(code.slice(alias_node.byte_range()).to_string());
                                }
                            }
                            _ => {}
                        }
                        def_local.path_components.extend(path_components);
                        if let Some(first) = def_local.path_components.first() {
                            if PYTHON_MODULES.contains(&first.as_str()) {
                                def_local.import_type = ImportType::System;
                            } else if first == "." || first == ".." {
                                def_local.import_type = ImportType::UserModule;
                            }
                        }
                        def_local.ast_fields.name = def_local.path_components.last().unwrap().to_string();
                        def_local.alias = alias;

                        symbols.push(Arc::new(RwLock::new(Box::new(def_local))));
                    }
                } else {
                    def.ast_fields.guid = get_guid();
                    symbols.push(Arc::new(RwLock::new(Box::new(def))));
                }
            }
            "ERROR" => {
                symbols.extend(self.parse_error_usages(&info.node, code, &info.ast_fields.file_path, &info.parent_guid));
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

    pub fn parse_function_declaration<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionDeclaration::default();
        decl.ast_fields.language = info.ast_fields.language;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.is_error = info.ast_fields.is_error;
        if let Some(parent_node) = info.node.parent() {
            if parent_node.kind() == "decorated_definition" {
                decl.ast_fields.full_range = parent_node.range();
            }
        }
        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        let mut decl_end_byte: usize = info.node.end_byte();
        let mut decl_end_point: Point = info.node.end_position();

        if let Some(name_node) = info.node.child_by_field_name("name") {
            decl.ast_fields.name = code.slice(name_node.byte_range()).to_string();
        }

        if let Some(parameters_node) = info.node.child_by_field_name("parameters") {
            decl_end_byte = parameters_node.end_byte();
            decl_end_point = parameters_node.end_position();
            symbols.extend(self.find_error_usages(&parameters_node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

            let params_len = parameters_node.child_count();
            let mut function_args = vec![];
            for idx in 0..params_len {
                let child = parameters_node.child(idx).unwrap();
                function_args.extend(parse_function_arg(&child, code));
            }
            decl.args = function_args;
        }
        decl.ast_fields.guid = get_guid();
        if let Some(return_type) = info.node.child_by_field_name("return_type") {
            decl.return_type = parse_type(&return_type, code);
            decl_end_byte = return_type.end_byte();
            decl_end_point = return_type.end_position();
            symbols.extend(self.find_error_usages(&return_type, code, &info.ast_fields.file_path, &decl.ast_fields.guid));
        }

        if let Some(body_node) = info.node.child_by_field_name("body") {
            decl.ast_fields.definition_range = body_node.range();
            decl.ast_fields.declaration_range = Range {
                start_byte: decl.ast_fields.full_range.start_byte,
                end_byte: decl_end_byte,
                start_point: decl.ast_fields.full_range.start_point,
                end_point: decl_end_point,
            };
            candidates.push_back(CandidateInfo {
                ast_fields: decl.ast_fields.clone(),
                node: body_node,
                parent_guid: decl.ast_fields.guid.clone(),
            });
        } else {
            decl.ast_fields.declaration_range = decl.ast_fields.full_range.clone();
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
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
                if PYTHON_KEYWORDS.contains(&name.as_str()) {
                    return vec![];
                }
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Python;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
            }
            "attribute" => {
                let attribute = parent.child_by_field_name("attribute").unwrap();
                let name = code.slice(attribute.byte_range()).to_string();
                let mut usage = VariableUsage::default();
                usage.ast_fields.name = name;
                usage.ast_fields.language = LanguageId::Python;
                usage.ast_fields.full_range = parent.range();
                usage.ast_fields.file_path = path.clone();
                usage.ast_fields.parent_guid = Some(parent_guid.clone());
                usage.ast_fields.guid = get_guid();
                usage.ast_fields.is_error = true;

                let object_node = parent.child_by_field_name("object").unwrap();
                let usages = self.parse_error_usages(&object_node, code, path, parent_guid);
                if let Some(last) = usages.last() {
                    usage.ast_fields.caller_guid = last.read().fields().parent_guid.clone();
                }
                symbols.extend(usages);
                symbols.push(Arc::new(RwLock::new(Box::new(usage))));
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

    pub fn parse_call_expression<'a>(&mut self, info: &CandidateInfo<'a>, code: &str, candidates: &mut VecDeque<CandidateInfo<'a>>) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut decl = FunctionCall::default();
        decl.ast_fields.language = LanguageId::Python;
        decl.ast_fields.full_range = info.node.range();
        decl.ast_fields.file_path = info.ast_fields.file_path.clone();
        decl.ast_fields.parent_guid = Some(info.parent_guid.clone());
        decl.ast_fields.guid = get_guid();
        if let Some(caller_guid) = info.ast_fields.caller_guid.clone() {
            decl.ast_fields.guid = caller_guid;
        }
        decl.ast_fields.caller_guid = Some(get_guid());
        decl.ast_fields.is_error = info.ast_fields.is_error;

        symbols.extend(self.find_error_usages(&info.node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        let arguments_node = info.node.child_by_field_name("arguments").unwrap();
        for i in 0..arguments_node.child_count() {
            let child = arguments_node.child(i).unwrap();
            let text = code.slice(child.byte_range());
            if SPECIAL_SYMBOLS.contains(&text) { continue; }

            let mut new_ast_fields = info.ast_fields.clone();
            new_ast_fields.caller_guid = None;
            candidates.push_back(CandidateInfo {
                ast_fields: new_ast_fields.clone(),
                node: child,
                parent_guid: info.parent_guid.clone(),
            });
        }
        symbols.extend(self.find_error_usages(&arguments_node, code, &info.ast_fields.file_path, &decl.ast_fields.guid));

        let function_node = info.node.child_by_field_name("function").unwrap();
        let text = code.slice(function_node.byte_range());
        let kind = function_node.kind();
        match kind {
            "identifier" => {
                decl.ast_fields.name = text.to_string();
            }
            "attribute" => {
                let object = function_node.child_by_field_name("object").unwrap();
                candidates.push_back(CandidateInfo {
                    ast_fields: decl.ast_fields.clone(),
                    node: object,
                    parent_guid: info.parent_guid.clone(),
                });
                let attribute = function_node.child_by_field_name("attribute").unwrap();
                decl.ast_fields.name = code.slice(attribute.byte_range()).to_string();
            }
            _ => {
                candidates.push_back(CandidateInfo {
                    ast_fields: info.ast_fields.clone(),
                    node: function_node,
                    parent_guid: info.parent_guid.clone(),
                });
            }
        }

        decl.ast_fields.childs_guid = get_children_guids(&decl.ast_fields.guid, &symbols);
        symbols.push(Arc::new(RwLock::new(Box::new(decl))));
        symbols
    }

    fn parse_(&mut self, parent: &Node, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let mut symbols: Vec<AstSymbolInstanceArc> = Default::default();
        let mut ast_fields = AstSymbolFields::default();
        ast_fields.file_path = path.clone();
        ast_fields.is_error = false;
        ast_fields.language = LanguageId::Python;

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

pub struct PythonSkeletonFormatter;

impl SkeletonFormatter for PythonSkeletonFormatter {
    fn make_skeleton(&self, symbol: &SymbolInformation,
                     text: &String,
                     guid_to_children: &HashMap<Uuid, Vec<Uuid>>,
                     guid_to_info: &HashMap<Uuid, &SymbolInformation>) -> String {
        let mut res_line = symbol.get_declaration_content(text).unwrap();
        let children = guid_to_children.get(&symbol.guid).unwrap();
        if children.is_empty() {
            return format!("{res_line}\n  ...");
        }
        res_line = format!("{}\n", res_line);
        for child in children {
            let child_symbol = guid_to_info.get(&child).unwrap();
            match child_symbol.symbol_type {
                SymbolType::FunctionDeclaration => {
                    let content = child_symbol.get_declaration_content(text).unwrap();
                    let lines = content.lines().collect::<Vec<_>>();
                    for line in lines {
                        let trimmed_line = line.trim_start();
                        res_line = format!("{}  {}\n", res_line, trimmed_line);
                    }
                    res_line = format!("{}    ...\n", res_line);
                }
                SymbolType::ClassFieldDeclaration => {
                    res_line = format!("{}  {}\n", res_line, child_symbol.get_content(text).unwrap());
                }
                _ => {}
            }
        }

        res_line
    }
    fn get_declaration_with_comments(&self,
                                     symbol: &SymbolInformation,
                                     text: &String,
                                     guid_to_children: &HashMap<Uuid, Vec<Uuid>>,
                                     guid_to_info: &HashMap<Uuid, &SymbolInformation>) -> (String, (usize, usize)) {
        if let Some(children) = guid_to_children.get(&symbol.guid) {
            let mut res_line: Vec<String> = Default::default();
            let mut row = symbol.full_range.start_point.row;
            let mut all_symbols = children.iter()
                .filter_map(|guid| guid_to_info.get(guid))
                .collect::<Vec<_>>();
            all_symbols.sort_by(|a, b|
                a.full_range.start_byte.cmp(&b.full_range.start_byte)
            );
            if symbol.symbol_type == SymbolType::FunctionDeclaration {
                res_line = symbol.get_content(text).unwrap().split("\n").map(|x| x.to_string()).collect::<Vec<_>>();
                row = symbol.full_range.end_point.row;
            } else {
                let mut content_lines = symbol.get_declaration_content(text).unwrap()
                    .split("\n")
                    .map(|x| x.to_string().replace("\t", "    ")).collect::<Vec<_>>();
                let mut intent_n = 0;
                if let Some(first) = content_lines.first_mut() {
                    intent_n = first.len() - first.trim_start().len();
                }
                for sym in all_symbols {
                    if sym.symbol_type != SymbolType::CommentDefinition {
                        break;
                    }
                    row = sym.full_range.end_point.row;
                    let content = sym.get_content(text).unwrap();
                    let lines = content.split("\n").collect::<Vec<_>>();
                    let lines = lines.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>();
                    res_line.extend(lines);
                }
                if res_line.is_empty() {
                    return ("".to_string(), (0, 0));
                }
                res_line.push(format!("{}...", " ".repeat(intent_n + 4)));
                content_lines.extend(res_line);
                res_line = content_lines;
            }

            let res_line = self.preprocess_content(Vec::from_iter(res_line.into_iter()));
            let declaration = res_line.join("\n");
            return (declaration, (symbol.full_range.start_point.row, row));
        }
        ("".to_string(), (0, 0))
    }
}

impl AstLanguageParser for PythonParser {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc> {
        let tree = self.parser.parse(code, None).unwrap();
        let symbols = self.parse_(&tree.root_node(), code, path);
        symbols
    }
}
