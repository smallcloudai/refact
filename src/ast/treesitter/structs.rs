use tree_sitter::{Point, Range};

pub trait UsageSymbolInfo {
    fn dump_path(&self) -> String;
    fn distance_to_cursor(&self, cursor: Point) -> usize;
}

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

pub enum StaticType {
    Comment,
    Literal,
}

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

