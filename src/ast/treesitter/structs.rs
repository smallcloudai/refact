use std::path::PathBuf;
use std::str::FromStr;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;


#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Range {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: Point,
    pub end_point: Point,
}

pub trait UsageSymbolInfo {
    fn dump_path(&self) -> String;
    fn distance_to_cursor(&self, cursor: Point) -> usize;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub range: Range,
    pub type_name: Option<String>,
}

impl UsageSymbolInfo for VariableInfo {
    fn dump_path(&self) -> String {
        match self.type_name.as_ref() {
            Some(t) => format!("{}::{}", self.name, t),
            None => self.name.clone(),
        }
    }
    fn distance_to_cursor(&self, cursor: Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCallInfo {
    pub name: String,
    pub range: Range,
    pub caller_type_name: Option<String>,
}

impl UsageSymbolInfo for FunctionCallInfo {
    fn dump_path(&self) -> String {
        match self.caller_type_name.as_ref() {
            Some(t) => format!("{}::{}", self.name, t),
            None => self.name.clone(),
        }
    }
    fn distance_to_cursor(&self, cursor: Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StaticType {
    Comment,
    Literal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StaticInfo {
    pub data: String,
    pub static_type: StaticType,
    pub range: Range,
}

impl UsageSymbolInfo for StaticInfo {
    fn dump_path(&self) -> String {
        format!("{}", self.data)
    }
    fn distance_to_cursor(&self, cursor: Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
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
        });
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInfo {
    pub path: PathBuf,
    pub range: Range,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolDeclarationStruct {
    pub name: String,
    pub definition_info: SymbolInfo,
    pub children: Vec<SymbolDeclarationStruct>,
    pub symbol_type: SymbolType,
    pub meta_path: String
}

impl SymbolDeclarationStruct {
    pub fn merge(&mut self, other: &mut SymbolDeclarationStruct) {
        self.children.append(&mut other.children);
    }
}

impl SymbolDeclarationStruct {
    pub async fn get_content(&self) -> Option<String> {
        let content = read_to_string(&self.definition_info.path).await.ok()?;
        let text = Rope::from_str(content.as_str());
        Some(text
            .slice(text.line_to_char(self.definition_info.range.start_point.row)..
                text.line_to_char(self.definition_info.range.end_point.row))
            .to_string())
    }
}

