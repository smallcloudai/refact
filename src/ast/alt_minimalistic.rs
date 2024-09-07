use std::sync::{Arc, Weak};
// use std::cell::RefCell;
// use std::collections::HashSet;
// use std::rc::Rc;
use serde::{Deserialize, Serialize};
use tree_sitter::{Point, Range};
use uuid::Uuid;
use crate::ast::treesitter::structs::{RangeDef, SymbolType};

use tokio::sync::{Mutex as AMutex, Notify as ANotify};


pub struct AltStatus {
    pub astate_notify: Arc<ANotify>,
    pub astate: String,
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: usize,
    pub ast_index_symbols_total: usize,
}

#[derive(Serialize, Deserialize)]
pub struct AltLink {
    // The idea behind these links:
    // * It's an empty guid when it's unresolved yet
    // * Linking means trying to match target_for_guesswork against path_for_guesswork, the longer the matched path the more
    //   probability the linking was correct
    pub guid: Uuid,
    pub target_for_guesswork: Vec<String>
}

#[derive(Serialize, Deserialize)]
pub struct AltDefinition {
    pub guid: Uuid,
    pub parent_guid: Uuid,
    pub path_for_guesswork: Vec<String>,   // file::namespace::class::method becomes ["file", "namespace", "class", "method"]
    pub symbol_type: SymbolType,
    pub derived_from: Vec<AltLink>,
    pub usages: Vec<AltLink>,
    #[serde(with = "RangeDef")]
    pub full_range: Range,
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,
    #[serde(with = "RangeDef")]
    pub definition_range: Range,
}

impl AltDefinition {
    pub fn path(&self) -> String {
        self.path_for_guesswork.join("::")
    }

    pub fn name(&self) -> String {
        self.path_for_guesswork.last().cloned().unwrap_or_default()
    }
}

// pub type AstSymbolInstanceRc = Rc<RefCell<Box<dyn AstSymbolInstance>>>;

pub struct AltIndex {
    pub sleddb: sled::Db,
}

pub struct AltState {
    pub alt_index: Arc<AMutex<AltIndex>>,
    pub alt_status: Arc<AMutex<AltStatus>>,
}
