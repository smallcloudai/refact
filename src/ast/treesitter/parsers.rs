use std::fmt::Display;
use std::path::PathBuf;

use tracing::error;

use crate::ast::comments_wrapper::get_language_id_by_filename;
use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceArc;
use crate::ast::treesitter::language_id::LanguageId;

pub(crate) mod python;
pub(crate) mod rust;
#[cfg(test)]
mod tests;
mod utils;
mod java;
mod cpp;


#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
}

pub trait AstLanguageParser: Send {
    fn parse(&mut self, code: &str, path: &PathBuf) -> Vec<AstSymbolInstanceArc>;
}

fn internal_error<E: Display>(err: E) -> ParserError {
    let err_msg = err.to_string();
    error!(err_msg);
    ParserError {
        message: err_msg.into(),
    }
}

fn get_ast_parser(language_id: LanguageId) -> Result<Box<dyn AstLanguageParser + 'static>, ParserError> {
    match language_id {
        LanguageId::Rust => {
            let parser = rust::RustParser::new()?;
            Ok(Box::new(parser))
        }
        LanguageId::Python => {
            let parser = python::PythonParser::new()?;
            Ok(Box::new(parser))
        }
        // temporary turned off due to parser bugs
        // LanguageId::Java => {
        //     let parser = java::JavaParser::new()?;
        //     Ok(Box::new(parser))
        // }
        other => Err(ParserError {
            message: "Unsupported language id: ".to_string() + &other.to_string()
        }),
    }
}


pub fn get_ast_parser_by_filename(filename: &PathBuf) -> Result<Box<dyn AstLanguageParser + 'static>, ParserError> {
    let suffix = filename.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let maybe_language_id = get_language_id_by_filename(filename);
    match maybe_language_id {
        Some(language_id) => get_ast_parser(language_id),
        None => Err(ParserError { message: suffix.to_string() }),
    }
}

