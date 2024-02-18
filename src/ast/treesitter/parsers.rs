use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use similar::DiffableStr;

use tracing::error;
use tree_sitter::{Node, Query, QueryCapture, Tree};

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::utils::{get_call, get_static, get_variable};
use crate::ast::treesitter::structs::{FunctionCallInfo, StaticInfo, SymbolDeclarationStruct, SymbolInfo, SymbolType, UsageSymbolInfo, VariableInfo};

pub(crate)  mod cpp;
pub(crate)  mod python;
pub(crate)  mod java;
pub(crate) mod rust;
pub(crate) mod ts;
pub(crate) mod tsx;
mod utils;


// Legacy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDeclarationSearchInfo {
    pub node_type: String,
    pub name_node_types: Vec<String>,
}

// Legacy
impl TypeDeclarationSearchInfo {
    pub fn default() -> Self {
        TypeDeclarationSearchInfo {
            node_type: "".to_string(),
            name_node_types: vec![],
        }
    }
    pub fn new(node_type: String, name_node_types: Vec<String>) -> Self {
        TypeDeclarationSearchInfo { node_type, name_node_types }
    }
}

// Legacy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstConfig {
    pub type_declaration_search_info: Vec<TypeDeclarationSearchInfo>,
    pub namespace_search_info: Option<TypeDeclarationSearchInfo>,
    pub keywords: Vec<String>,
    pub keywords_types: Vec<String>,
}

// Legacy
impl AstConfig {
    pub fn default() -> Self {
        Self {
            type_declaration_search_info: vec![],
            keywords: vec![],
            namespace_search_info: None,
            keywords_types: vec![],
        }
    }
}

// Legacy
pub trait Language {
    fn make_ast_config() -> AstConfig;
}

// Legacy
impl Language for AstConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig::default()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
}

pub trait LanguageParser: Send {
    fn get_parser(&mut self) -> &mut tree_sitter::Parser;
    fn get_parser_query(&self) -> &String;
    fn get_parser_query_find_all(&self) -> &String;
    fn get_namespace(&self, parent: Option<Node>, text: &str) -> Vec<String>;

    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        ("".to_string(), vec![])
    }
    
    fn get_extra_declarations_for_struct(&mut self, struct_name: String, tree: &Tree, text: &str, path: &PathBuf) -> Vec<SymbolInfo> {
        vec![]
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>);
    fn get_variable_name(&self, parent: Node, text: &str) -> String;
    fn get_variable(&mut self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        get_variable(captures, query, code)
    }

    fn get_call(&self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<FunctionCallInfo> {
        get_call(captures, query, code)
    }
    fn get_static(&self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<StaticInfo> {
        get_static(captures, query, code)
    }

    fn parse_declarations(&mut self, code: &str, path: &PathBuf) -> Result<HashMap<String, SymbolDeclarationStruct>, String> {
        let mut indexes: HashMap<String, SymbolDeclarationStruct> = Default::default();
        let tree: Tree = match self.get_parser().parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(self.get_parser().language().unwrap(), self.get_parser_query()).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "class" | "struct" | "trait" => {
                        let range = capture.node.range();
                        let text = code.slice(capture.node.byte_range());
                        let namespaces = self.get_namespace(Some(capture.node), code);
                        let class_name = namespaces.last().unwrap().clone();
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name: class_name.clone(),
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::Class,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                           extra_declarations: self.get_extra_declarations_for_struct(class_name, &tree, code, &path),
                                       });
                    }
                    "enum" => {
                        let range = capture.node.range();
                        let mut namespaces = self.get_namespace(Some(capture.node), code);
                        let (enum_name, values) = self.get_enum_name_and_all_values(capture.node, code);
                        namespaces.push(enum_name);
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        values.iter().for_each(|value| {
                            let key = format!("{}::{}", key, value);
                            indexes.insert(key.clone(),
                                           SymbolDeclarationStruct {
                                               name: value.clone(),
                                               definition_info: SymbolInfo { path: path.clone(), range },
                                               children: vec![],
                                               symbol_type: SymbolType::Enum,
                                               meta_path: key,
                                               language: LanguageId::from(capture.node.language()),
                                               extra_declarations: vec![],
                                           });
                        });
                    }
                    "function" => {
                        let range = capture.node.range();
                        let mut namespaces = self.get_namespace(Some(capture.node), code);
                        let text = code.slice(capture.node.byte_range());
                        let (name, scopes) = self.get_function_name_and_scope(capture.node.clone(), code);
                        namespaces.extend(scopes);
                        namespaces.push(name.clone());
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::Function,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                           extra_declarations: vec![],
                                       });
                    }
                    "global_variable" => {
                        let range = capture.node.range();
                        let text = code.slice(capture.node.byte_range());
                        let mut namespaces = self.get_namespace(Some(capture.node), code);
                        let name = self.get_variable_name(capture.node, code);
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.push(name.clone());
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::GlobalVar,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                           extra_declarations: vec![],
                                       });
                    }
                    &_ => {}
                }
            }
        }
        Ok(indexes)
    }

    fn parse_usages(&mut self, code: &str) -> Result<Vec<Box<dyn UsageSymbolInfo>>, String> {
        let tree: Tree = match self.get_parser().parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut usages: Vec<Box<dyn UsageSymbolInfo>> = vec![];
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(self.get_parser().language().unwrap(), self.get_parser_query_find_all()).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            match match_.pattern_index {
                0 => {
                    if let Some(var) = self.get_variable(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                1 => {
                    if let Some(var) = self.get_call(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                2 => {
                    if let Some(var) = self.get_static(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                _ => {}
            }
        }
        Ok(usages)
    }
}

fn internal_error<E: Display>(err: E) -> ParserError {
    let err_msg = err.to_string();
    error!(err_msg);
    ParserError {
        message: err_msg.into(),
    }
}

fn get_parser(language_id: LanguageId) -> Result<Box<dyn LanguageParser + 'static>, ParserError> {
    match language_id {
        LanguageId::Cpp => {
            let parser = cpp::CppParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::Python => {
            let parser = python::PythonParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::Java => {
            let parser = java::JavaParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::Rust => {
            let parser = rust::RustParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::TypeScript => {
            let parser = ts::TypescriptParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::TypeScriptReact => {
            let parser = tsx::TypescriptxParser::new()?;
            Ok(Box::new(parser))
        }
        other => Err(ParserError {
            message: "Unsupported language id: ".to_string() + &other.to_string()
        }),
    }
}


pub fn get_parser_by_filename(filename: &PathBuf) -> Result<Box<dyn LanguageParser + 'static>, ParserError> {
    let suffix = filename.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match suffix.as_str() {
        "cpp" | "cc" | "cxx" | "c++" | "c" | "h" | "hpp" | "hxx" | "hh" => get_parser(LanguageId::Cpp),
        "inl" | "inc" | "tpp" | "tpl" => get_parser(LanguageId::Cpp),
        "py" | "pyo" | "py3" | "pyx" => get_parser(LanguageId::Python),
        "java" => get_parser(LanguageId::Java),
        "rs" => get_parser(LanguageId::Rust),
        "ts" => get_parser(LanguageId::TypeScript),
        "tsx" => get_parser(LanguageId::TypeScriptReact),
        other => Err(ParserError { message: "Unsupported filename suffix: ".to_string() + &other }),
    }
}

