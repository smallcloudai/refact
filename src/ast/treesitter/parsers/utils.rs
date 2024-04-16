use tree_sitter::Node;
use uuid::Uuid;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc};


pub(crate) fn get_guid() -> Uuid {
    Uuid::new_v4()
}

pub(crate) fn str_hash(s: &String) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}

pub(crate) fn get_children_guids(parent_guid: &Uuid, children: &Vec<AstSymbolInstanceArc>) -> Vec<Uuid> {
    let mut result = Vec::new();
    for child in children {
        let child_ref = child.read().expect("the data might be broken");
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

#[cfg(test)]
pub(crate) fn print(symbols: &Vec<AstSymbolInstanceArc>, code: &str) {
    use std::collections::{HashMap, HashSet, VecDeque};

    let guid_to_symbol_map = symbols.iter()
        .map(|s| (s.clone().read().unwrap().guid().clone(), s.clone())).collect::<HashMap<_, _>>();
    let sorted = symbols.iter().sorted_by_key(|x| x.read().unwrap().full_range().start_byte).collect::<Vec<_>>();
    let mut used_guids: HashSet<String> = Default::default();

    for sym in sorted {
        let guid = sym.read().unwrap().guid().clone();
        if used_guids.contains(&guid) {
            continue;
        }
        let name = sym.read().unwrap().name().to_string();
        let full_range = sym.read().unwrap().full_range().clone();
        let range = full_range.start_byte..full_range.end_byte;
        println!("{0} {1} {2}", guid.slice(0..6), name, code.slice(range).lines().collect::<Vec<_>>().first().unwrap());
        used_guids.insert(guid.clone());
        let mut candidates: VecDeque<(i32, String)> = VecDeque::from_iter(sym.read().unwrap().childs_guid().iter().map(|x| (4, x.clone())));
        while let Some((offest, cand)) = candidates.pop_front() {
            used_guids.insert(cand.clone());
            if let Some(sym_l) = guid_to_symbol_map.get(&*cand) {
                let name = sym_l.read().unwrap().name().to_string();
                let full_range = sym_l.read().unwrap().full_range().clone();
                let range = full_range.start_byte..full_range.end_byte;
                println!("{0} {1} {2} {3}", cand.slice(0..6), str::repeat(" ", offest as usize),name, code.slice(range).lines().collect::<Vec<_>>().first().unwrap());
                let mut new_candidates = VecDeque::from_iter(sym_l.read().unwrap().childs_guid().iter().map(|x| (offest + 2, x.clone())));
                new_candidates.extend(candidates.clone());
                candidates = new_candidates;
            }
        }

    }

}
