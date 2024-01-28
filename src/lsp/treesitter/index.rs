use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use tree_sitter::Range;

#[derive(Clone, PartialEq)]
pub enum SymbolType {
    GlobalVar,
    Function,
    Class,
    Method,
    Unknown,
}

impl FromStr for SymbolType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return Ok(match s {
            "method" => SymbolType::Method,
            "class" => SymbolType::Class,
            "global_var" => SymbolType::GlobalVar,
            "function" => SymbolType::Function,
            _ => SymbolType::Unknown
        })
    }
}

#[derive(Clone)]
pub struct SymbolInfo {
    pub path: PathBuf,
    pub range: Range,
}

#[derive(Clone)]
pub struct DefinitionInfo {
    pub symbol_info: SymbolInfo,
    pub text: String,
}

#[derive(Clone)]
pub struct Index {
    pub name: String,
    pub used: HashMap<PathBuf, SymbolInfo>,
    pub definition_info: Option<DefinitionInfo>,
    pub children: Vec<Index>,
    pub symbol_type: SymbolType
}

impl Index {
    pub fn merge(&mut self, other: &mut Index) {
        self.children.append(&mut other.children);
        if self.definition_info.is_none() {
            self.definition_info = other.definition_info.clone();
        }
        self.used.extend(other.used.clone());
    }
    
}
