use std::fmt::Debug;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tree_sitter::Point;

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
