use std::sync::{Arc, Weak, Mutex as StdMutex};
use std::cell::RefCell;
use std::cmp::min;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use async_trait::async_trait;
use dyn_partial_eq::{dyn_partial_eq, DynPartialEq};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;
use tree_sitter::{Point, Range};
use uuid::Uuid;

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::structs::{RangeDef, SymbolType};

use tokio::sync::{Mutex as AMutex, RwLock as ARwLock, Notify as ANotify};
use crate::files_in_workspace::Document;


struct AltStatus {
    pub astate_notify: Arc<ANotify>,
    pub astate: String,
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: usize,
    pub ast_index_symbols_total: usize,
}

struct AltIndex {
}

struct AltState {
}

// pub type AstSymbolInstanceRc = Rc<RefCell<Box<dyn AstSymbolInstance>>>;

pub struct AltLink {
    pub guid: Uuid,
    pub link_target_for_guesswork: Vec<String>
}

pub struct AltDefinition {
    pub guid: Uuid,
    pub path_for_guesswork: Vec<String>,   // file::namespace::class::method becomes ["file", "namespace", "class", "method"]
    pub parent_guid: Uuid,
    pub derived_from: Vec<Uuid>,
    pub usage_guids: Vec<Uuid>,
    pub file_guid: Uuid,
}

pub struct AltUsage {
    pub path_guesses: Vec<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInformation {
    pub guid: Uuid,                          // Unique identifier for the symbol, used to reference and link symbols uniquely
    pub name: String,                        // Name of the symbol, such as a variable name, function name, etc.
    pub parent_guid: Uuid,                   // GUID of the parent symbol, indicating the hierarchical structure (e.g., a method within a class)
    pub linked_decl_guid: Uuid,              // GUID of the declaration this symbol is linked to, useful for resolving references to their declarations
    pub caller_guid: Uuid,                   // GUID of the symbol that calls this symbol, used for tracking function calls and variable usages
    pub symbol_type: SymbolType,             // Type of the symbol (e.g., function, variable), helps in categorizing the symbol
    pub symbol_path: String,                 // Full path of the symbol within the codebase, including namespaces and parent structures
    pub language: LanguageId,                // Language identifier for the symbol, indicating the programming language of the source file
    pub file_path: PathBuf,                  // File path where the symbol is located, used for locating the symbol in the filesystem
    pub namespace: String,                   // Namespace of the symbol, providing context about the symbol's scope and visibility
    pub is_error: bool,                      // Indicates if the symbol has an error, useful for error detection and reporting
    #[serde(with = "RangeDef")]
    pub full_range: Range,                   // Full range of the symbol in the source code, covering its entire span
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,            // Range of the symbol's declaration, where it is initially declared
    #[serde(with = "RangeDef")]
    pub definition_range: Range,             // Range of the symbol's definition, where its implementation or value is provided
}

async fn doc_add(index: Arc<AMutex<AltIndex>>, doc: &Document)
{
}

async fn doc_remove(index: Arc<AMutex<AltIndex>>, cpath: &String)
{
}

async fn doc_symbols(index: Arc<AMutex<AltState>>, cpath: &String) -> Vec<Arc<Symbol>>
{
}

