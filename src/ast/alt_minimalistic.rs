use std::sync::{Arc, Weak};
use std::fmt;
// use std::cell::RefCell;
// use std::collections::HashSet;
// use std::rc::Rc;
use serde::{Deserialize, Serialize};
use tree_sitter::{Point, Range};
use uuid::Uuid;
use crate::ast::treesitter::structs::{RangeDef, SymbolType};

use tokio::sync::{Mutex as AMutex, Notify as ANotify};


#[derive(Serialize, Deserialize, Clone)]
pub struct AltLink {
    // The idea behind these links:
    // * It's an empty guid when it's unresolved yet
    // * Linking means trying to match target_for_guesswork against path_for_guesswork, the longer the matched path the more
    //   probability the linking was correct
    pub guid: Uuid,
    pub target_for_guesswork: Vec<String>
}

#[derive(Serialize, Deserialize, Clone)]
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

impl fmt::Debug for AltDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let usages_paths: Vec<String> = self.usages.iter()
            .map(|link| link.target_for_guesswork.join("::"))
            .collect();
        let derived_from_paths: Vec<String> = self.derived_from.iter()
            .map(|link| link.target_for_guesswork.join("::"))
            .collect();
        write!(
            f,
            "AltDefinition {{ path: {:?}, usages: {:?}, derived_from: {:?} }}",
            self.path_for_guesswork,
            usages_paths,
            derived_from_paths
        )
    }
}
impl fmt::Debug for AltLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AltLink {{ path: {:?}{} }}",
            self.target_for_guesswork,
            if self.guid != Uuid::nil() { ", resolved" } else { "" }
        )
    }
}


// pub type AstSymbolInstanceRc = Rc<RefCell<Box<dyn AstSymbolInstance>>>;

pub struct AltIndex {
    pub sleddb: sled::Db,
}

pub struct AltStatus {
    pub astate_notify: Arc<ANotify>,
    pub astate: String,
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: usize,
    pub ast_index_symbols_total: usize,
}

pub struct AltState {
    pub alt_index: Arc<AMutex<AltIndex>>,
    pub alt_status: Arc<AMutex<AltStatus>>,
}
