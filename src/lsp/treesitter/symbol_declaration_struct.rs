pub struct SymbolDeclarationStruct {
    pub id: i32,
    pub node_type: String,
    pub name: String,
    pub content: String,
    pub start_point: (i32, i32),
    pub end_point: (i32, i32),
    pub path: String,
    pub parent_ids: Option<Vec<i32>>,
    pub namespaces_name: Option<Vec<String>>,
}