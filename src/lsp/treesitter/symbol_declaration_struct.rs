use tree_sitter::Point;

pub struct SymbolDeclarationStruct {
    pub id: usize,
    pub node_type: String,
    pub name: String,
    pub content: String,
    pub start_point: Point,
    pub end_point: Point,
    pub path: String,
    pub parent_ids: Option<Vec<usize>>,
    pub namespaces_name: Option<Vec<String>>,
}