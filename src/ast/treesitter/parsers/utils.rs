use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::AstSymbolInstanceArc;

pub(crate) fn get_guid() -> String {
    let id = Uuid::new_v4();
    id.to_string()
}

pub(crate) fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub(crate) fn get_children_guids(parent_guid: &String, children: &Vec<AstSymbolInstanceArc>) -> Vec<String> {
    let mut result = Vec::new();
    for child in children {
        let child_ref = child.read().expect("the data might be broken");
        if let Some(child_guid) = child_ref.parent_guid() {
            if child_guid == parent_guid {
                result.push(child_ref.guid().to_string());
            }
        }
    }
    result
}
