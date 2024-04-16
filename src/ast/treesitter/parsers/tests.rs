use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use itertools::Itertools;
use similar::DiffableStr;
use tree_sitter::Node;
use uuid::Uuid;
use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc};

// mod cpp;
mod rust;
mod python;
mod java;
mod cpp;
mod ts;
// pub(crate) fn test_query_function(mut parser: Box<dyn LanguageParser>,
//                                   path: &PathBuf,
//                                   code: &str,
//                                   ref_indexes: HashMap<String, SymbolDeclarationStruct>,
//                                   ref_usages: Vec<Box<dyn UsageSymbolInfo>>) {
//     let indexes = parser.parse_declarations(code, &path).unwrap();
//     let usages = parser.parse_usages(code, true).unwrap();
//
//     indexes.iter().for_each(|(key, index)| {
//         assert_eq!(index, ref_indexes.get(key).unwrap());
//     });
//     ref_indexes.iter().for_each(|(key, index)| {
//         assert_eq!(index, indexes.get(key).unwrap());
//     });
//
//     usages.iter().for_each(|usage| {
//         assert!(ref_usages.contains(usage));
//     });
//     ref_usages.iter().for_each(|usage| {
//         assert!(usages.contains(usage));
//     });
// }

pub(crate) fn print(symbols: &Vec<AstSymbolInstanceArc>, code: &str) {
    let guid_to_symbol_map = symbols.iter()
        .map(|s| (s.clone().read().unwrap().guid().clone(), s.clone())).collect::<HashMap<_, _>>();
    let sorted = symbols.iter().sorted_by_key(|x| x.read().unwrap().full_range().start_byte).collect::<Vec<_>>();
    let mut used_guids: HashSet<Uuid> = Default::default();

    for sym in sorted {
        let guid = sym.read().unwrap().guid().clone();
        if used_guids.contains(&guid) {
            continue;
        }
        let caller_guid = sym.read().unwrap().get_caller_guid().clone();
        let mut name = sym.read().unwrap().name().to_string();
        if let Some(caller_guid) = caller_guid {
            if guid_to_symbol_map.contains_key(&caller_guid) {
                name = format!("{} -> {}", name, caller_guid.to_string().slice(0..6));
            }
        }
        let full_range = sym.read().unwrap().full_range().clone();
        let range = full_range.start_byte..full_range.end_byte;
        println!("{0} {1} [{2}]", guid.to_string().slice(0..6), name, code.slice(range).lines().collect::<Vec<_>>().first().unwrap());
        used_guids.insert(guid.clone());
        let mut candidates: VecDeque<(i32, Uuid)> = VecDeque::from_iter(sym.read().unwrap().childs_guid().iter().map(|x| (4, x.clone())));
        while let Some((offest, cand)) = candidates.pop_front() {
            used_guids.insert(cand.clone());
            if let Some(sym_l) = guid_to_symbol_map.get(&cand) {
                let caller_guid = sym_l.read().unwrap().get_caller_guid().clone();
                let mut name = sym_l.read().unwrap().name().to_string();
                if let Some(caller_guid) = caller_guid {
                    if guid_to_symbol_map.contains_key(&caller_guid) {
                        name = format!("{} -> {}", name, caller_guid.to_string().slice(0..6));
                    }
                }
                let full_range = sym_l.read().unwrap().full_range().clone();
                let range = full_range.start_byte..full_range.end_byte;
                println!("{0} {1} {2} [{3}]", cand.to_string().slice(0..6), str::repeat(" ", offest as usize),name, code.slice(range).lines().collect::<Vec<_>>().first().unwrap());
                let mut new_candidates = VecDeque::from_iter(sym_l.read().unwrap().childs_guid().iter().map(|x| (offest + 2, x.clone())));
                new_candidates.extend(candidates.clone());
                candidates = new_candidates;
            }
        }

    }

}
