use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AMutex, Notify as ANotify};
pub use crate::ast::treesitter::structs::SymbolType;


#[derive(Serialize, Deserialize, Clone)]
pub struct AstUsage {
    // Linking means trying to match targets_for_guesswork against official_path, the longer
    // the matched path the more probability the linking was correct
    pub targets_for_guesswork: Vec<String>, // ?::DerivedFrom1::f ?::DerivedFrom2::f ?::f
    pub resolved_as: String,
    pub debug_hint: String,
    pub uline: usize,     // starts from 0, TODO make it start from 1
}

#[derive(Serialize, Deserialize)]
pub struct AstDefinition {
    pub official_path: Vec<String>,  // file::namespace::class::method becomes ["file", "namespace", "class", "method"]
    pub symbol_type: SymbolType,
    pub usages: Vec<AstUsage>,
    pub resolved_type: String,                // for type derivation at pass2 or something, not used much now
    pub this_is_a_class: String,              // cpp🔎Goat
    pub this_class_derived_from: Vec<String>, // cpp🔎Animal, cpp🔎CosmicJustice
    pub cpath: String,
    pub decl_line1: usize,                    // starts from 1, guaranteed > 0
    pub decl_line2: usize,                    // guaranteed >= line1
    pub body_line1: usize,                    // use full_line1() full_line2() if not sure
    pub body_line2: usize,
}

impl AstDefinition {
    pub fn path(&self) -> String {
        self.official_path.join("::")
    }

    pub fn path_drop0(&self) -> String {
        if self.official_path.len() > 3 {  // new style long path, starts with hex code we don't want users to see
            self.official_path.iter().skip(1).cloned().collect::<Vec<String>>().join("::")
        } else {  // there's not much to cut
            self.official_path.join("::")
        }
    }

    pub fn name(&self) -> String {
        self.official_path.last().cloned().unwrap_or_default()
    }

    pub fn full_line1(&self) -> usize {
        self.decl_line1
    }

    pub fn full_line2(&self) -> usize {
        self.body_line2.max(self.decl_line2)
    }
}

pub struct AstDB {
    pub sleddb: Arc<sled::Db>,
    pub sledbatch: Arc<AMutex<sled::Batch>>,
    pub batch_counter: usize,
    pub counters_increase: HashMap<String, i32>,
    pub ast_max_files: usize,
}

#[derive(Serialize, Clone)]
pub struct AstStatus {
    #[serde(skip)]
    pub astate_notify: Arc<ANotify>,
    #[serde(rename = "state")]
    pub astate: String,
    pub files_unparsed: usize,
    pub files_total: usize,
    pub ast_index_files_total: i32,
    pub ast_index_symbols_total: i32,
    pub ast_index_usages_total: i32,
    pub ast_max_files_hit: bool,
}

pub struct AstCounters {
    pub counter_defs: i32,
    pub counter_usages: i32,
    pub counter_docs: i32,
}


const TOO_MANY_ERRORS: usize = 1000;

pub struct AstError {
    pub err_cpath: String,
    pub err_message: String,
    pub err_line: usize,
}

pub struct AstErrorStats {
    pub errors: Vec<AstError>,
    pub errors_counter: usize,
}

impl AstErrorStats {
    pub fn add_error(
        self: &mut AstErrorStats,
        err_cpath: String,
        err_line: usize,
        err_message: &str,
    ) {
        if self.errors.len() < TOO_MANY_ERRORS {
            self.errors.push(AstError {
                err_cpath,
                err_message: err_message.to_string(),
                err_line,
            });
        }
        self.errors_counter += 1;
    }
}

impl Default for AstErrorStats {
    fn default() -> Self {
        AstErrorStats {
            errors: Vec::new(),
            errors_counter: 0,
        }
    }
}


impl fmt::Debug for AstDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let usages_paths: Vec<String> = self.usages.iter()
            .map(|link| format!("{:?}", link))
            .collect();
        let derived_from_paths: Vec<String> = self.this_class_derived_from.iter()
            .map(|link| format!("{:?}", link))
            .collect();

        let usages_str = if usages_paths.is_empty() {
            String::new()
        } else {
            format!(", usages: {}", usages_paths.join(" "))
        };

        let class_str = if self.this_is_a_class.is_empty() {
            String::new()
        } else {
            format!(", this_is_a_class: {}", self.this_is_a_class)
        };

        let derived_from_str = if derived_from_paths.is_empty() {
            String::new()
        } else {
            format!(", derived_from: {}", derived_from_paths.join(" "))
        };

        write!(
            f,
            "AstDefinition {{ {}{}{}{} }}",
            self.official_path.join("::"),
            usages_str,
            class_str,
            derived_from_str,
        )
    }
}

impl fmt::Debug for AstUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // self.target_for_guesswork
        write!(
            f,
            "U{{ {} {} }}",
            self.debug_hint,
            if self.resolved_as.len() > 0 { self.resolved_as.clone() } else { format!("guess {}", self.targets_for_guesswork.join(" ")) }
        )
    }
}
