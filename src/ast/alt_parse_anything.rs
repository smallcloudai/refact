use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
use uuid::Uuid;
use crate::ast::alt_minimalistic::{AltDefinition, AltLink};
use crate::ast::treesitter::parsers::{get_ast_parser_by_filename, AstLanguageParser};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::ast_instance_structs::{VariableUsage, VariableDefinition, AstSymbolInstance, FunctionDeclaration, FunctionCall, TypeDef};
use std::any::Any;


// fn resolve_declaration_symbols(symbols: &mut Vec<AstSymbolInstanceRc>)
// {
//     for symbol in symbols.iter_mut() {
//         let (type_names, symb_type, symb_path) = {
//             let s_ref = symbol.borrow();
//             (s_ref.types(), s_ref.symbol_type(), s_ref.file_path().clone())
//         };
//         if symb_type == SymbolType::ImportDeclaration
//             || symb_type == SymbolType::CommentDefinition
//             || symb_type == SymbolType::FunctionCall
//             || symb_type == SymbolType::VariableUsage {
//             continue;
//         }

//         let mut new_guids = vec![];
//         for (_, t) in type_names.iter().enumerate() {
//             if t.is_pod || t.name.is_none() {
//                 new_guids.push(t.guid.clone());
//                 continue;
//             }

//             if let Some(guid) = t.guid {
//                 if self.symbols_by_guid.contains_key(&guid) {
//                     new_guids.push(t.guid.clone());
//                     continue;
//                 }
//             }

//             let name = t.name.clone().expect("filter has invalid condition");
//             let maybe_guid = match self.declaration_symbols_by_name.get(&name) {
//                 Some(symbols) => {
//                     symbols
//                         .iter()
//                         .filter(|s| s.borrow().is_type())
//                         .min_by(|a, b| {
//                             // TODO: use import-based distance
//                             let path_a = a.borrow().file_path().clone();
//                             let path_b = b.borrow().file_path().clone();
//                             FilePathIterator::compare_paths(&symb_path, &path_a, &path_b)
//                         })
//                         .map(|s| s.borrow().guid().clone())
//                 }
//                 None => {
//                     new_guids.push(None);
//                     continue;
//                 }
//             };

//             match maybe_guid {
//                 Some(guid) => {
//                     new_guids.push(Some(guid));
//                 }
//                 None => {
//                     new_guids.push(None);
//                 }
//             }
//         }
//         assert_eq!(new_guids.len(), type_names.len());
//         {
//             let mut symbol_ref = symbol.borrow_mut();
//             symbol_ref.set_guids_to_types(&new_guids);
//         }
//     }
//     stats
// }



// fn merge_usages_to_declarations_simple(symbols: &mut Vec<Box<dyn AstSymbolInstance>>) -> IndexingStats {
//     fn get_caller_depth(
//         symbol: &dyn AstSymbolInstance,
//         guid_by_symbols: &HashMap<Uuid, &dyn AstSymbolInstance>,
//     ) -> Option<usize> {
//         let mut current_symbol = symbol;
//         let mut current_depth = 0;
//         loop {
//             let caller_guid = match current_symbol.get_caller_guid() {
//                 Some(g) => g,
//                 None => {
//                     return Some(current_depth);
//                 }
//             };
//             match guid_by_symbols.get(&caller_guid) {
//                 Some(s) => {
//                     current_symbol = *s;
//                     current_depth += 1;
//                 }
//                 None => {
//                     return Some(current_depth);
//                 }
//             }
//         }
//     }

//     fn get_struct_guid(
//         symbol: &dyn AstSymbolInstance,
//         guid_by_symbols: &HashMap<Uuid, &dyn AstSymbolInstance>,
//     ) -> Option<Uuid> {
//         let mut current_symbol = symbol;
//         loop {
//             let guid = match current_symbol.parent_guid() {
//                 Some(g) => g,
//                 None => {
//                     return None;
//                 }
//             };
//             match guid_by_symbols.get(&guid) {
//                 Some(s) => {
//                     if s.symbol_type() == SymbolType::StructDeclaration {
//                         return Some(guid);
//                     } else {
//                         current_symbol = *s;
//                     }
//                 }
//                 None => {
//                     return None;
//                 }
//             }
//         }
//     }

//     fn make_fields_index<'a>(
//         symbols: &'a Vec<Box<dyn AstSymbolInstance>>,
//         guid_by_symbols: &'a HashMap<Uuid, &dyn AstSymbolInstance>,
//         is_var_index: bool
//     ) -> HashMap<(Uuid, String), &dyn AstSymbolInstance> {
//         let mut index: HashMap<(Uuid, String), &dyn AstSymbolInstance> = HashMap::new();
//         for s in symbols.iter() {
//             if is_var_index {
//                 if s.symbol_type() == SymbolType::VariableDefinition
//                     || s.symbol_type() == SymbolType::ClassFieldDeclaration {
//                     if let Some(struct_guid) = get_struct_guid(s.as_ref(), guid_by_symbols) {
//                         index.insert((struct_guid, s.name().to_string()), s.as_ref());
//                     }
//                 }
//             } else {
//                 if s.symbol_type() == SymbolType::FunctionDeclaration
//                     || s.symbol_type() == SymbolType::StructDeclaration {
//                     if let Some(struct_guid) = get_struct_guid(s.as_ref(), guid_by_symbols) {
//                         index.insert((struct_guid, s.name().to_string()), s.as_ref());
//                     }
//                 }
//             }
//         }
//         index
//     }

//     fn try_link_type_to_decl(
//         type_inference_linked_guid_index: &HashMap<Uuid, &dyn AstSymbolInstance>,
//         symbols_by_guid: &HashMap<Uuid, &dyn AstSymbolInstance>,
//         usage_symbol: &mut dyn AstSymbolInstance,
//         type_def: &TypeDef,
//     ) {
//         let mut first_caller_symbol = usage_symbol;
//         loop {
//             if let Some(s) = first_caller_symbol
//                 .get_caller_guid()
//                 .and_then(|x| symbols_by_guid.get(&x))
//             {
//                 first_caller_symbol = *s;
//             } else {
//                 break;
//             }
//         }
//         if let Some(decl_symbol) = type_inference_linked_guid_index.get(first_caller_symbol.guid()) {
//             let symbol_type = decl_symbol.symbol_type();
//             match symbol_type {
//                 SymbolType::ClassFieldDeclaration => {
//                     if let Some(class_field_decl) = decl_symbol.as_any_mut().downcast_mut::<ClassFieldDeclaration>() {
//                         class_field_decl.type_.guid = type_def.guid.clone();
//                     }
//                 }
//                 SymbolType::VariableDefinition => {
//                     if let Some(var_def) = decl_symbol.as_any_mut().downcast_mut::<VariableDefinition>() {
//                         var_def.type_.guid = type_def.guid.clone();
//                     }
//                 }
//                 _ => {}
//             }
//         }
//     }

//     let type_inference_linked_guid_index = symbols
//         .iter()
//         .filter(|x| x.symbol_type() == SymbolType::VariableDefinition
//             || x.symbol_type() == SymbolType::ClassFieldDeclaration)
//         .filter_map(|x| {
//             let type_def = x.types().get(0).cloned()?;
//             type_def.inference_info_guid.and_then(|guid| {
//                 symbols.iter().find(|s| s.guid() == &guid).map(|s| (guid, s.as_ref()))
//             })
//         })
//         .collect::<HashMap<Uuid, &dyn AstSymbolInstance>>();

//     let symbols_by_guid = symbols.iter().map(|s| (s.guid().clone(), s.as_ref())).collect::<HashMap<_, _>>();

//     for s in symbols.iter_mut() {
//         let caller_depth = get_caller_depth(s.as_ref(), &symbols_by_guid);
//         s.set_caller_depth(caller_depth);
//     }

//     let mut stats = IndexingStats { found: 0, non_found: 0 };
//     let search_by_caller_var_index = make_fields_index(
//         &symbols, &symbols_by_guid, true
//     );
//     let search_by_caller_func_index = make_fields_index(
//         &symbols, &symbols_by_guid, false
//     );

//     let max_depth: usize = 20;
//     let mut depth: usize = 0; // depth means "a.b.c" it's 2 for c
//     loop {
//         let mut symbols_to_process = symbols
//             .iter_mut()
//             .filter(|symbol| {
//                 let s_ref = symbol.as_ref();
//                 let has_linked_type = s_ref.get_linked_decl_type().is_some();
//                 if has_linked_type {
//                     return false;
//                 }
//                 let has_no_valid_linked_decl = s_ref.get_linked_decl_guid().map_or(true, |guid| !symbols_by_guid.contains_key(&guid));
//                 let valid_depth = symbol.get_caller_depth().map_or(false, |d| d == depth);
//                 has_no_valid_linked_decl && valid_depth && (s_ref.symbol_type() == SymbolType::FunctionCall
//                     || s_ref.symbol_type() == SymbolType::VariableUsage)
//             })
//             .collect::<Vec<_>>();
//         if depth >= max_depth {
//             break;
//         }
//         if symbols_to_process.is_empty() {
//             depth += 1;
//             continue;
//         }

//         let mut symbols_cache: HashMap<
//             (Uuid, String),
//             (Option<&dyn AstSymbolInstance>, Option<&dyn AstSymbolInstance>)
//         > = HashMap::new();
//         for usage_symbol in symbols_to_process.iter_mut() {
//             let guids_pair = (
//                 usage_symbol.parent_guid().unwrap_or_default(),
//                 usage_symbol.name().to_string()
//             );
//             let decl_searching_result = if !symbols_cache.contains_key(&guids_pair) {
//                 match if depth == 0 {
//                     find_decl_by_name(
//                         usage_symbol.as_ref(),
//                         &symbols_by_guid,
//                         &search_by_caller_var_index,
//                     )
//                 } else {
//                     find_decl_by_caller_guid(
//                         usage_symbol.as_ref(),
//                         &symbols_by_guid,
//                         &search_by_caller_var_index,
//                         &search_by_caller_func_index,
//                     ).or_else(|| find_decl_by_name(
//                         usage_symbol.as_ref(),
//                         &symbols_by_guid,
//                         &search_by_caller_var_index,
//                     ))
//                 } {
//                     Some(res) => {
//                         if depth == 0 {
//                             symbols_cache.insert(guids_pair, res.clone());
//                         }
//                         res
//                     }
//                     None => {
//                         stats.non_found += 1;
//                         continue;
//                     }
//                 }
//             } else {
//                 symbols_cache.get(&guids_pair).cloned().unwrap()
//             };

//             match decl_searching_result {
//                 (None, Some(type_symbol)) => {
//                     let typedef = TypeDef {
//                         name: Some(type_symbol.name().to_string()),
//                         guid: Some(type_symbol.guid().clone()),
//                         ..TypeDef::default()
//                     };
//                     try_link_type_to_decl(&type_inference_linked_guid_index, &symbols_by_guid, usage_symbol.as_mut(), &typedef);
//                     usage_symbol.set_linked_decl_type(typedef);
//                     stats.found += 1;
//                 }
//                 (Some(decl_symbol), None) => {
//                     usage_symbol.set_linked_decl_guid(Some(decl_symbol.guid().clone()));
//                     stats.found += 1;
//                 }
//                 (Some(decl_symbol), Some(type_symbol)) => {
//                     let typedef = TypeDef {
//                         name: Some(type_symbol.name().to_string()),
//                         guid: Some(type_symbol.guid().clone()),
//                         ..TypeDef::default()
//                     };
//                     try_link_type_to_decl(&type_inference_linked_guid_index, &symbols_by_guid, usage_symbol.as_mut(), &typedef);
//                     usage_symbol.set_linked_decl_guid(Some(decl_symbol.guid().clone()));
//                     usage_symbol.set_linked_decl_type(typedef);
//                     stats.found += 1;
//                 }
//             }
//         }
//     }
// }


// let mut doc = doc.clone();
// doc.update_text(&code.to_string());
// let mut symbols = AstIndex::parse(&doc)
//     .unwrap_or_default()
//     .iter()
//     .map(|sym| {
//         let mut write_lock = sym.write();
//         Rc::new(RefCell::new(std::mem::replace(&mut *write_lock, Box::new(ImportDeclaration::default()))))
//     }).collect::<Vec<_>>();
// _ = self.resolve_imports(&mut symbols, &self.import_components_succ_solution_index);
// self.resolve_declaration_symbols(&mut symbols);
// loop {
//     let stats = self.merge_usages_to_declarations(&mut symbols);
//     if stats.found == 0 {
//         break;
//     }
// }

fn _is_declaration(t: SymbolType) -> bool {
    match t {
        SymbolType::StructDeclaration |
        SymbolType::TypeAlias |
        SymbolType::ClassFieldDeclaration |
        SymbolType::ImportDeclaration |
        SymbolType::VariableDefinition |
        SymbolType::FunctionDeclaration |
        SymbolType::CommentDefinition |
        SymbolType::Unknown => {
            true
        }
        SymbolType::FunctionCall |
        SymbolType::VariableUsage => {
            false
        }
    }
}

fn _go_to_parent_until_declaration(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    start_node_guid: Uuid,
) -> Uuid {
    let mut node_guid = start_node_guid;
    loop {
        let node_option = map.get(&node_guid);
        if node_option.is_none() {
            tracing::error!("find_parent_of_types: node not found");
            return Uuid::nil();
        }
        let node = node_option.expect("xxx1003").read();
        if _is_declaration(node.symbol_type()) {
            return node.guid().clone();
        } else {
            if let Some(parent_guid) = node.parent_guid() {
                node_guid = parent_guid.clone();
            } else {
                return Uuid::nil();
            }
        }
    }
}

fn _attempt_typeof(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    start_node_guid: Uuid,
    variable_or_param_name: String,
) -> Vec<String> {
    let mut result = vec![];
    let mut node_guid = start_node_guid.clone();
    loop {
        let node_option = map.get(&node_guid);
        if node_option.is_none() {
            break;
        }
        let node = node_option.unwrap().read();
        eprintln!("attempt_typeof: visiting {:?} {:?}", node.guid(), node.name());

        // There we go, the logic

        // 1. type[0].name if VariableDefinition and name matches
        if let Some(variable_definition) = node.as_any().downcast_ref::<VariableDefinition>() {
            eprintln!("variable_definition.name {:?} {:?}", variable_definition.name(), variable_or_param_name);
            if variable_definition.name() == variable_or_param_name {
                if let Some(first_type) = variable_definition.types().get(0) {
                    result.insert(0, first_type.name.clone().unwrap_or_default());
                }
            }
        }

        // 2. Check for FunctionDeclaration and match argument names
        if let Some(function_declaration) = node.as_any().downcast_ref::<FunctionDeclaration>() {
            for arg in &function_declaration.args {
                eprintln!("function_declaration.arg.name {:?} {:?}", arg.name, variable_or_param_name);
                if arg.name == variable_or_param_name {
                    if let Some(arg_type) = &arg.type_ {
                        result.insert(0, arg_type.name.clone().unwrap_or_default());
                    }
                }
            }
        }

        // logic over

        if let Some(parent_guid) = node.parent_guid() {
            node_guid = parent_guid.clone();
        } else {
            break;
        }
    }
    result
}


pub fn parse_anything(cpath: &str, text: &str) -> IndexMap<Uuid, AltDefinition> {
    let path = PathBuf::from(cpath);
    let mut parser = match get_ast_parser_by_filename(&path) {
        Ok(x) => x,
        Err(err) => {
            tracing::error!("Error getting parser: {}", err.message);
            return IndexMap::new();
        }
    };

    let symbols = parser.parse(text, &path);
    let symbols2 = symbols.clone();
    let mut definitions = IndexMap::new();
    let mut orig_map: HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> = HashMap::new();

    for symbol in symbols {
        let symbol_arc_clone = symbol.clone();
        let symbol = symbol.read();
        eprintln!("something: {:?}", symbol);
        orig_map.insert(symbol.guid().clone(), symbol_arc_clone);
        for (i, t) in symbol.types().iter().enumerate() {
            eprintln!("type[{}] = {:?}", i, t);
        }
        eprintln!("");
        match symbol.symbol_type() {
            SymbolType::StructDeclaration |
            SymbolType::TypeAlias |
            SymbolType::ClassFieldDeclaration |
            SymbolType::ImportDeclaration |
            SymbolType::VariableDefinition |
            SymbolType::FunctionDeclaration |
            SymbolType::CommentDefinition |
            SymbolType::Unknown => {
                let definition = AltDefinition {
                    guid: symbol.guid().clone(),
                    parent_guid: symbol.parent_guid().clone().unwrap_or_default(),
                    path_for_guesswork: vec![symbol.name().to_string()],
                    symbol_type: symbol.symbol_type().clone(),
                    derived_from: vec![],
                    usages: vec![],
                    full_range: symbol.full_range().clone(),
                    declaration_range: symbol.declaration_range().clone(),
                    definition_range: symbol.definition_range().clone(),
                };
                definitions.insert(definition.guid.clone(), definition);
            }
            SymbolType::FunctionCall |
            SymbolType::VariableUsage => {
                // do nothing
            }
        }
    }

    // path_for_guesswork is easy to build, go through parents
    let a_copy_for_looking_up_stuff = definitions.clone();
    for x in definitions.values_mut() {
        let mut path = vec![];
        let mut current_guid = x.guid.clone();
        while current_guid != Uuid::nil() {
            if let Some(parent_def) = a_copy_for_looking_up_stuff.get(&current_guid) {
                let name_or_guid = if !parent_def.name().is_empty() {
                    parent_def.name().clone()
                } else {
                    parent_def.guid.to_string()
                };
                path.push(name_or_guid);
                current_guid = parent_def.parent_guid.clone();
            } else {
                current_guid = Uuid::nil();
            }
        }
        x.path_for_guesswork = path.into_iter().rev().collect();
    }

    /*
    types of f1 f2 f3 f4
    FunctionDeclaration {
        ast_fields: AstSymbolFields {
            guid: f1ce82cd-9bc2-4d42-ac7b-728799ace039,
            name: "some_fun",
            language: Cpp,
            file_path: "tests/emergency_frog_situation/compiled_frog.cpp",
            namespace: "",
            parent_guid: Some(3fbdfe24-1498-4033-be15-cff9c88842ee),
            childs_guid: [
                0a2de525-f197-43f3-a4c2-663b7f8481a0,
                b0f73c8d-234a-4236-9652-de41600886cc,
                be34627d-07f1-401d-8cc9-a0dbbec08cc2,
                14a34d59-f18a-48b7-b697-71c8b6e2807a,
                dd7168d1-c2b0-4c17-8501-ac723ff2ca12,
                bbc8eeb9-b378-44be-b24e-16cc5d53591b,
                5cefc8da-6479-46af-a554-79ada3c9277e,
                4177bf8a-341f-41fd-8e67-a5550184e4f2
            ],
            linked_decl_guid: None,
            linked_decl_type: None,
            caller_guid: None,
            is_error: false,
            caller_depth: None
        },
        template_types: [],
        args: [
            FunctionArg {
                name: "f1",
                type_: Some(TypeDef {
                    name: Some("CompiledFrog"),
                    inference_info: None,
                    inference_info_guid: None,
                    is_pod: false,
                    namespace: "",
                    guid: None,
                    nested_types: []
                })
            },
            FunctionArg {
                name: "f2",
                type_: Some(TypeDef {
                    name: Some("CompiledFrog"),
                    inference_info: None,
                    inference_info_guid: None,
                    is_pod: false,
                    namespace: "",
                    guid: None,
                    nested_types: []
                })
            },
            FunctionArg {
                name: "f3",
                type_: Some(TypeDef {
                    name: Some("CompiledFrog"),
                    inference_info: None,
                    inference_info_guid: None,
                    is_pod: false,
                    namespace: "",
                    guid: None,
                    nested_types: []
                })
            },
            FunctionArg {
                name: "f4",
                type_: None
            }
        ],
        return_type: Some(TypeDef {
            name: Some("void"),
            inference_info: None,
            inference_info_guid: None,
            is_pod: true,
            namespace: "",
            guid: None,
            nested_types: []
        })
    }
    */
    // say_hi() -> caller_guid -> f4
    // logic: have caller_guid, prepend type of caller to path


    for symbol in symbols2 {
        let symbol = symbol.read();
        match symbol.symbol_type() {
            SymbolType::StructDeclaration |
            SymbolType::TypeAlias |
            SymbolType::ClassFieldDeclaration |
            SymbolType::ImportDeclaration |
            SymbolType::VariableDefinition |
            SymbolType::FunctionDeclaration |
            SymbolType::CommentDefinition |
            SymbolType::Unknown => {
                continue;
            }
            SymbolType::FunctionCall => {
                // eprintln!("Function call usage: {:?}", symbol);
                let function_call = symbol.as_any().downcast_ref::<FunctionCall>().expect("xxx1000");
                let fields = function_call.fields();
                let caller_guid = fields.caller_guid.clone();
                let mut prepend_to_path = Vec::<String>::new();
                let mut debug_hint = "".to_string();

                // caller_guid in this case refers to "self" in self.x
                // we need to discover type of "caller", and parent.usages += type(caller)
                if let Some(caller_guid) = caller_guid {
                    if let Some(caller_node_arc) = orig_map.get(&caller_guid) {
                        let caller_node = caller_node_arc.read();
                        eprintln!("Resolved caller: {:?}", caller_guid);
                        prepend_to_path = _attempt_typeof(&orig_map, caller_guid, caller_node.name().to_string());
                        debug_hint = caller_node.name().to_string();
                        eprintln!("xxx: {:?}", prepend_to_path);
                    } else {
                        eprintln!("Unresolved caller: {:?}", caller_guid);
                    }
                }

                let parent_decl_guid = _go_to_parent_until_declaration(&orig_map, symbol.parent_guid().unwrap_or_default());
                if let Some(definition) = definitions.get_mut(&parent_decl_guid) {
                    // eprintln!("Parent definition found for function call: {:?}", definition);
                    definition.usages.push(AltLink {
                        guid: symbol.guid().clone(),
                        target_for_guesswork: [prepend_to_path, vec![function_call.name().to_string()]].concat(),
                        debug_hint: debug_hint,
                    });
                }
            }
            SymbolType::VariableUsage => {
                let variable_usage = symbol.as_any().downcast_ref::<VariableUsage>().expect("xxx1001");
                let fields = variable_usage.fields();
                let guid = fields.guid.clone();
                let parent_guid = fields.parent_guid.clone();
                let name = fields.name.clone();
                let full_range = fields.full_range.clone();
                let childs_guid = fields.childs_guid.clone();
                let caller_guid = fields.caller_guid.clone();
                let linked_decl_guid = fields.linked_decl_guid.clone();
                eprintln!(
                    "Variable usage found: guid: {:?}, parent_guid: {:?}, name: {:?}, full_range: {:?}, childs_guid: {:?}, caller_guid: {:?}, linked_decl_guid: {:?}",
                    guid, parent_guid, name, full_range, childs_guid, caller_guid, linked_decl_guid
                );
                // if let Some(parent_guid) = parent_guid {
                //     if let Some(parent_definition) = definitions.get_mut(&parent_guid) {
                //         eprintln!("Resolved parent definition: {:?}", parent_definition.path());
                //         parent_definition.usages.push(AltLink {
                //             guid: Uuid::nil(),
                //             target_for_guesswork: vec![name],
                //         });
                //     } else {
                //         eprintln!("Unresolved parent definition: {:?}", parent_guid);
                //     }
                // }

                // caller logic

                if let Some(linked_decl_typedef) = symbol.get_linked_decl_type() {
                    // #[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
                    // pub struct TypeDef {
                    //  pub name: Option<String>,
                    //  pub inference_info: Option<String>,
                    //  pub inference_info_guid: Option<Uuid>,
                    //  pub is_pod: bool,
                    //  pub namespace: String,
                    //  pub guid: Option<Uuid>,
                    //  pub nested_types: Vec<TypeDef>, // for nested types, presented in templates
                    // }
                    eprintln!("typedef: {:?}", linked_decl_typedef);
                }

                if let Some(linked_decl_guid) = linked_decl_guid {
                    if let Some(linked_decl_definition) = definitions.get(&linked_decl_guid) {
                        eprintln!("Resolved linked declaration definition: {:?}", linked_decl_definition);
                    } else {
                        eprintln!("Unresolved linked declaration definition: {:?}", linked_decl_guid);
                    }
                }
                for child_guid in &childs_guid {
                    if let Some(child_definition) = definitions.get(child_guid) {
                        eprintln!("Resolved child definition: {:?}", child_definition);
                    } else {
                        eprintln!("Unresolved child definition: {:?}", child_guid);
                    }
                }
                eprintln!("");
            }
        }
    }
    definitions
}


// emergency_frog_situation/
//   frog.py (Frog, __init__, bounce_off_banks, jump)
//   holiday.py
//   jump_to_conclusions.py (draw_hello_frog, main_loop)
//   set_as_avatar.py (Toad, EuropeanCommonToad, __init__, __init__)
//   work_day.py (bring_your_own_frog_to_work_day)



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_subscriber;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    fn read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    // #[test]
    // fn test_parse_anything_frog_py() {
    //     init_tracing();
    //     let text = read_file("tests/emergency_frog_situation/frog.py");
    //     let definitions = parse_anything("tests/emergency_frog_situation/frog.py", &text);
    //     for d in definitions.values() {
    //         println!("{:#?}", d);
    //     }
    //     assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"Frog".to_string())));
    //     assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"__init__".to_string())));
    //     assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"bounce_off_banks".to_string())));
    //     assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"jump".to_string())));
    // }

    #[test]
    fn test_parse_anything_frog_py() {
        init_tracing();
        let text = read_file("tests/emergency_frog_situation/compiled_frog.cpp");
        let definitions = parse_anything("tests/emergency_frog_situation/compiled_frog.cpp", &text);
        for d in definitions.values() {
            println!("{:#?}", d);
        }
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"Frog".to_string())));
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"__init__".to_string())));
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"bounce_off_banks".to_string())));
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"jump".to_string())));
    }

    // #[test]
    // fn test_parse_anything_set_as_avatar_py() {
    //     let text = read_file("tests/emergency_frog_situation/set_as_avatar.py");
    //     let definitions = parse_anything("tests/emergency_frog_situation/set_as_avatar.py", &text);
    //     for d in definitions.values() {
    //         println!("{:#?}", d);
    //     }
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains("Toad")));
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains("EuropeanCommonToad")));
        // assert!(definitions.values().any(|d| d.path_for_guesswork.contains("__init__")));
    // }

    // #[test]
    // fn test_parse_anything_holiday_py() {
    //     let text = read_file("emergency_frog_situation/holiday.py");
    //     let definitions = parse_anything("emergency_frog_situation/holiday.py", &text);
    //     // Add assertions to check the parsed definitions
    //     // For example:
    //     // assert!(definitions.iter().any(|d| d.path().contains("SomeClassOrFunction")));
    // }

    // #[test]
    // fn test_parse_anything_jump_to_conclusions_py() {
    //     let text = read_file("emergency_frog_situation/jump_to_conclusions.py");
    //     let definitions = parse_anything("emergency_frog_situation/jump_to_conclusions.py", &text);
    //     // Add assertions to check the parsed definitions
    //     assert!(definitions.iter().any(|d| d.path().contains("draw_hello_frog")));
    //     assert!(definitions.iter().any(|d| d.path().contains("main_loop")));
    // }

    // #[test]
    // fn test_parse_anything_work_day_py() {
    //     let text = read_file("emergency_frog_situation/work_day.py");
    //     let definitions = parse_anything("emergency_frog_situation/work_day.py", &text);
    //     // Add assertions to check the parsed definitions
    //     assert!(definitions.iter().any(|d| d.path().contains("bring_your_own_frog_to_work_day")));
    // }
}

