use std::sync::Arc;
use std::fmt;
use serde::{Deserialize, Serialize};
use tree_sitter::Range;
use tokio::sync::Notify as ANotify;
pub use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::structs::RangeDef;

const TOO_MANY_ERRORS: usize = 1000;


#[derive(Serialize, Deserialize)]
pub struct Usage {
    // Linking means trying to match targets_for_guesswork against official_path, the longer
    // the matched path the more probability the linking was correct
    pub targets_for_guesswork: Vec<String>, // ?::DerivedFrom1::f ?::DerivedFrom2::f ?::f
    pub resolved_as: String,
    pub debug_hint: String,
    pub uline: usize,
}

#[derive(Serialize, Deserialize)]
pub struct AstDefinition {
    pub official_path: Vec<String>,  // file::namespace::class::method becomes ["file", "namespace", "class", "method"]
    pub symbol_type: SymbolType,
    pub usages: Vec<Usage>,
    pub this_is_a_class: String,              // cpp🔎Goat
    pub this_class_derived_from: Vec<String>, // cpp🔎Animal, cpp🔎CosmicJustice
    pub cpath: String,
    #[serde(with = "RangeDef")]
    pub full_range: Range,
    #[serde(with = "RangeDef")]
    pub declaration_range: Range,
    #[serde(with = "RangeDef")]
    pub definition_range: Range,
}

impl AstDefinition {
    pub fn path(&self) -> String {
        self.official_path.join("::")
    }

    pub fn name(&self) -> String {
        self.official_path.last().cloned().unwrap_or_default()
    }
}

pub struct AstDB {
    pub sleddb: Arc<sled::Db>,
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
}

pub struct AstCounters {
    pub counter_defs: i32,
    pub counter_usages: i32,
}

pub struct AstError {
    pub err_cpath: String,
    pub err_message: String,
    pub err_line: usize,
}

pub struct ErrorStats {
    pub errors: Vec<AstError>,
    pub errors_counter: usize,
}

impl ErrorStats {
    pub fn add_error(
        self: &mut ErrorStats,
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

impl Default for ErrorStats {
    fn default() -> Self {
        ErrorStats {
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

impl fmt::Debug for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // self.target_for_guesswork
        write!(
            f,
            "U{{ {} {} }}",
            self.debug_hint,
            if self.resolved_as.len() > 0 { self.resolved_as.clone() } else { self.targets_for_guesswork.join(" ") }
        )
    }
}
