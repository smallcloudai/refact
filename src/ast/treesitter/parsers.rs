use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

use tracing::error;

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::structs::{SymbolDeclarationStruct, UsageSymbolInfo};

pub mod cpp;
pub mod python;
pub mod java;
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
    fn parse_declarations(&mut self, code: &str, path: &PathBuf) -> Result<HashMap<String, SymbolDeclarationStruct>, String>;

    fn parse_usages(&mut self, code: &str) -> Result<Vec<Box<dyn UsageSymbolInfo + 'static>>, String>;
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
        },
        LanguageId::Python => {
            let parser = python::PythonParser::new()?;
            Ok(Box::new(parser))
        }
        _ => Err(ParserError { message: "Unsupported language id".to_string() }),
    }
}


pub fn get_parser_by_filename(filename: &PathBuf) -> Result<Box<dyn LanguageParser + 'static>, ParserError> {
    let suffix = filename.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match suffix.as_str() {
        "cpp" | "cc" | "cxx" | "c++" | "c" | "h" | "hpp" | "hxx" | "hh" => get_parser(LanguageId::Cpp),
        "inl" | "inc" | "tpp" | "tpl" => get_parser(LanguageId::Cpp),
        "py" | "pyo" | "py3" | "pyx" => get_parser(LanguageId::Python),
        "java" => get_parser(LanguageId::Java),
        _ => Err(ParserError { message: "Unsupported filename suffix".to_string() }),
    }
}

