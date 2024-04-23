use tree_sitter::Node;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc};

pub(crate) fn get_guid() -> Uuid {
    Uuid::new_v4()
}

pub(crate) fn get_children_guids(parent_guid: &Uuid, children: &Vec<AstSymbolInstanceArc>) -> Vec<Uuid> {
    let mut result = Vec::new();
    for child in children {
        let child_ref = child.read();
        if let Some(child_guid) = child_ref.parent_guid() {
            if child_guid == parent_guid {
                result.push(child_ref.guid().clone());
            }
        }
    }
    result
}


pub(crate) struct CandidateInfo<'a> {
    pub ast_fields: AstSymbolFields,
    pub node: Node<'a>,
    pub parent_guid: Uuid
} 
