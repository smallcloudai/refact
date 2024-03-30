use std::cmp::min;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;
use async_trait::async_trait;
use dyn_partial_eq::*;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;
use tree_sitter::{Range, Point};
use crate::ast::treesitter::language_id::LanguageId;


#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(remote = "tree_sitter::Point")]
pub(crate) struct PointDef {
    pub row: usize,
    pub column: usize,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(remote = "tree_sitter::Range")]
pub(crate) struct RangeDef {
    pub start_byte: usize,
    pub end_byte: usize,
    #[serde(with = "PointDef")]
    pub start_point: Point,
    #[serde(with = "PointDef")]
    pub end_point: Point,
}

#[async_trait]
#[typetag::serde]
#[dyn_partial_eq]
pub trait UsageSymbolInfo: Debug + Send + Sync {
    fn meta_path(&self) -> String;
    fn distance_to_cursor(&self, cursor: &Point) -> usize;
    fn type_str(&self) -> String;
    fn get_range(&self) -> Range;
    fn set_definition_meta_path(&mut self, meta_path: String);
    fn get_declaration_meta_path(&self) -> Option<String>;
}


#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct VariableInfo {
    pub name: String,
    #[serde(with = "RangeDef")]
    pub range: Range,
    pub type_names: Vec<String>,
    pub meta_path: Option<String>,
}

#[async_trait]
#[typetag::serde]
impl UsageSymbolInfo for VariableInfo {
    fn meta_path(&self) -> String {
        if self.type_names.len() > 0 {
            format!("{}::{}", self.type_names.first().unwrap(), self.name)
        } else {
            self.name.clone()
        }
    }
    fn distance_to_cursor(&self, cursor: &Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }

    fn type_str(&self) -> String {
        String::from("variable_info")
    }

    fn get_range(&self) -> Range {
        self.range.clone()
    }

    fn set_definition_meta_path(&mut self, meta_path: String) {
        self.meta_path = Some(meta_path);
    }

    fn get_declaration_meta_path(&self) -> Option<String> {
        self.meta_path.clone()
    }
}

#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCallInfo {
    pub name: String,
    #[serde(with = "RangeDef")]
    pub range: Range,
    pub caller_type_name: Option<String>,
    pub meta_path: Option<String>,
}

#[async_trait]
#[typetag::serde]
impl UsageSymbolInfo for FunctionCallInfo {
    fn meta_path(&self) -> String {
        match self.caller_type_name.as_ref() {
            Some(t) => format!("{}::{}", self.name, t),
            None => self.name.clone(),
        }
    }
    fn distance_to_cursor(&self, cursor: &Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }
    fn type_str(&self) -> String {
        String::from("function_call_info")
    }

    fn get_range(&self) -> Range {
        self.range.clone()
    }

    fn set_definition_meta_path(&mut self, meta_path: String) {
        self.meta_path = Some(meta_path);
    }

    fn get_declaration_meta_path(&self) -> Option<String> {
        self.meta_path.clone()
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum StaticType {
    Comment,
    Literal,
}

#[derive(DynPartialEq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StaticInfo {
    pub data: String,
    pub static_type: StaticType,
    #[serde(with = "RangeDef")]
    pub range: Range,
    pub meta_path: Option<String>,
}

#[async_trait]
#[typetag::serde]
impl UsageSymbolInfo for StaticInfo {
    fn meta_path(&self) -> String {
        format!("{}", self.data)
    }
    fn distance_to_cursor(&self, cursor: &Point) -> usize {
        cursor.row.abs_diff(self.range.start_point.row)
    }
    fn type_str(&self) -> String {
        String::from("static_info")
    }

    fn get_range(&self) -> Range {
        self.range.clone()
    }

    fn set_definition_meta_path(&mut self, meta_path: String) {
        self.meta_path = Some(meta_path);
    }

    fn get_declaration_meta_path(&self) -> Option<String> {
        self.meta_path.clone()
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum SymbolType {
    StructDeclaration,
    TypeAlias,
    ClassFieldDeclaration,
    ImportDeclaration,
    VariableDefinition,
    FunctionDeclaration,
    CommentDefinition,
    FunctionCall,
    VariableUsage,
    Unknown,
}

impl FromStr for SymbolType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return Ok(match s {
            "struct_declaration" => SymbolType::StructDeclaration,
            "type_alias" => SymbolType::TypeAlias,
            "class_field_declaration" => SymbolType::ClassFieldDeclaration,
            "import_declaration" => SymbolType::ImportDeclaration,
            "variable_definition" => SymbolType::VariableDefinition,
            "function_declaration" => SymbolType::FunctionDeclaration,
            "comment_definition" => SymbolType::CommentDefinition,
            "function_call" => SymbolType::FunctionCall,
            "variable_usage" => SymbolType::VariableUsage,
            _ => SymbolType::Unknown
        });
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SymbolInfo {
    pub path: PathBuf,
    #[serde(with = "RangeDef")]
    pub range: Range,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SymbolDeclarationStruct {
    pub name: String,
    pub definition_info: SymbolInfo,
    pub children: Vec<SymbolDeclarationStruct>,
    pub symbol_type: SymbolType,
    pub meta_path: String,
    pub language: LanguageId,
    pub extra_declarations: Vec<SymbolInfo>,
}

impl SymbolDeclarationStruct {
    pub async fn get_content(&self) -> io::Result<String> {
        let content = read_to_string(&self.definition_info.path).await?;
        let text = Rope::from_str(content.as_str());

        let mut start_row = min(self.definition_info.range.start_point.row, text.len_lines());
        let end_row = min(self.definition_info.range.end_point.row + 1, text.len_lines());
        start_row = min(start_row, end_row);

        Ok(text.slice(text.line_to_char(start_row)..text.line_to_char(end_row)).to_string())
    }

    // pub fn get_content_blocked(&self) -> io::Result<String> {
    //     let content = std::fs::read_to_string(&self.definition_info.path)?;
    //     let text = Rope::from_str(content.as_str());
    //     Ok(text
    //         .slice(text.line_to_char(self.definition_info.range.start_point.row)..
    //             text.line_to_char(self.definition_info.range.end_point.row))
    //         .to_string())
    // }
}

