use tree_sitter::Range;


pub struct VariableInfo {
    pub name: String,
    pub range: Range,
    pub type_name: Option<String>,
}

pub struct FunctionCallInfo {
    pub name: String,
    pub range: Range,
}

pub enum StaticType {
    Comment,
    Literal,
}

pub struct StaticInfo {
    pub static_type: StaticType,
    pub range: Range,
}

