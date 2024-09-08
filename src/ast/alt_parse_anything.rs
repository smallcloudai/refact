use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
use uuid::Uuid;
use crate::ast::alt_minimalistic::{AltDefinition, AltLink};
use crate::ast::treesitter::parsers::{get_ast_parser_by_filename, AstLanguageParser};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::ast_instance_structs::{VariableUsage, VariableDefinition, AstSymbolInstance, FunctionDeclaration, FunctionCall, TypeDef};
use std::any::Any;


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
        let node = node_option.unwrap().read();
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

fn _path_of_node(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    start_node_guid: Option<Uuid>,
) -> Vec<String> {
    let mut path = vec![];
    if start_node_guid.is_none() {
        return path;
    }
    let mut current_guid = start_node_guid.unwrap();
    while current_guid != Uuid::nil() {
        if let Some(node_arc) = map.get(&current_guid) {
            let node = node_arc.read();
            let name_or_guid = if !node.name().is_empty() {
                node.name().to_string()
            } else {
                node.guid().to_string()
            };
            path.push(name_or_guid);
            current_guid = node.parent_guid().unwrap_or(Uuid::nil());
        } else {
            break;
        }
    }
    path.into_iter().rev().collect()
}

fn _find_top_level_nodes(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
) -> Vec<std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> {
    //
    // XXX UGLY: the only way to detect top level is to map.get(parent) if it's not found, then it's top level.
    //
    let mut top_level: Vec<std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> = Vec::new();
    for (_, node_arc) in map.iter() {
        let node = node_arc.read();
        assert!(node.parent_guid().is_some());  // parent always exists for some reason :/
        if _is_declaration(node.symbol_type()) {
            if !map.contains_key(&node.parent_guid().unwrap()) {
                top_level.push(node_arc.clone());
            }
        }
    }
    top_level
}

fn _attempt_name2path(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    file_global_path: &Vec<String>,
    start_node_guid: Option<Uuid>,
    name_of_anything: String,
) -> Vec<String> {
    if start_node_guid.is_none() {
        return vec![];
    }
    let mut node_guid = start_node_guid.unwrap();
    let mut look_here: Vec<std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> = Vec::new();
    loop {
        let node_option = map.get(&node_guid);
        if node_option.is_none() {
            break;
        }
        let node = node_option.unwrap().read();
        if _is_declaration(node.symbol_type()) {
            look_here.push(node_option.unwrap().clone());
            if node.symbol_type() == SymbolType::StructDeclaration {
                // Add all children nodes (shallow)
                for child_guid in node.childs_guid() {
                    if let Some(child_node) = map.get(child_guid) {
                        look_here.push(child_node.clone());
                    }
                }
            }
        }
        if let Some(parent_guid) = node.parent_guid() {
            node_guid = parent_guid.clone();
        } else {
            break;
        }
    }

    let top_level_nodes = _find_top_level_nodes(map);
    look_here.extend(top_level_nodes);

    for node_arc in look_here {
        let node = node_arc.read();
        if _is_declaration(node.symbol_type()) {
            if node.name() == name_of_anything {
                return [
                    file_global_path.clone(),
                    _path_of_node(map, Some(node.guid().clone()))
                ].concat();
            }
        }
    }

    vec!["?".to_string(), name_of_anything]
}

fn _attempt_typeof_path(
    map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    file_global_path: &Vec<String>,
    start_node_guid: Uuid,
    variable_or_param_name: String,
) -> Vec<String> {
    let mut node_guid = start_node_guid.clone();
    let mut look_here: Vec<std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> = Vec::new();

    loop {
        let node_option = map.get(&node_guid);
        if node_option.is_none() {
            break;
        }
        let node = node_option.unwrap().read();

        if _is_declaration(node.symbol_type()) {
            look_here.push(node_option.unwrap().clone());
            // Add all children nodes (shallow)
            for child_guid in node.childs_guid() {
                if let Some(child_node) = map.get(child_guid) {
                    look_here.push(child_node.clone());
                }
            }
        }

        if let Some(parent_guid) = node.parent_guid() {
            node_guid = parent_guid.clone();
        } else {
            break;
        }
    }

    let top_level_nodes = _find_top_level_nodes(map);
    look_here.extend(top_level_nodes);

    for (_, node_arc) in map.iter() {
        let node = node_arc.read();
        assert!(node.parent_guid().is_some());  // parent always exists for some reason
        if _is_declaration(node.symbol_type()) {
            if !map.contains_key(&node.parent_guid().unwrap()) {
                look_here.push(node_arc.clone());
            }
        }
    }

    for node_arc in look_here {
        let node = node_arc.read();
        eprintln!("attempt_typeof: look_here {:?} {:?}", node.guid(), node.name());

        // Check for VariableDefinition and match name
        if let Some(variable_definition) = node.as_any().downcast_ref::<VariableDefinition>() {
            eprintln!("variable_definition.name {:?} {:?}", variable_definition.name(), variable_or_param_name);
            if variable_definition.name() == variable_or_param_name {
                if let Some(first_type) = variable_definition.types().get(0) {
                    return [
                        file_global_path.clone(),
                        // vec!["<type-of-vardef>".to_string()],
                        vec![first_type.name.clone().unwrap_or_default()],
                    ].concat();
                }
            }
        }

        // Check for FunctionDeclaration and match argument names
        if let Some(function_declaration) = node.as_any().downcast_ref::<FunctionDeclaration>() {
            for arg in &function_declaration.args {
                eprintln!("function_declaration.arg.name {:?} {:?}", arg.name, variable_or_param_name);
                if arg.name == variable_or_param_name {
                    if let Some(arg_type) = &arg.type_ {
                        return [
                            file_global_path.clone(),
                            // vec!["<type-of-arg>".to_string()],
                            vec![arg_type.name.clone().unwrap_or_default()]
                        ].concat();
                    }
                }
            }
        }
    }

    vec!["?".to_string()]
}


fn _global_path_from_file_path(cpath: &str) -> Vec<String> {
    use std::path::Path;
    let path = Path::new(cpath);
    let mut components = vec![];
    let silly_names_list = vec!["__init__.py", "mod.rs"];
    if let Some(file_name) = path.file_stem() {
        let file_name_str = file_name.to_string_lossy().to_string();
        if !silly_names_list.contains(&file_name_str.as_str()) {
            components.push(file_name_str);
        }
    }
    if let Some(parent) = path.parent() {
        if let Some(parent_name) = parent.file_name() {
            components.push(parent_name.to_string_lossy().to_string());
        }
    }
    components.iter().rev().take(2).cloned().collect::<Vec<_>>()
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
    // let global_path = _global_path_from_file_path(cpath);
    let global_path = vec!["file".to_string()];
    eprintln!("global_path = {:?}", global_path);

    let symbols = parser.parse(text, &path);
    let symbols2 = symbols.clone();
    let mut definitions = IndexMap::new();
    let mut orig_map: HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>> = HashMap::new();

    for symbol in symbols {
        let symbol_arc_clone = symbol.clone();
        let symbol = symbol.read();
        orig_map.insert(symbol.guid().clone(), symbol_arc_clone);
        for (i, t) in symbol.types().iter().enumerate() {
            eprintln!("type[{}] = {:?}", i, t);
        }
        eprintln!("");
        match symbol.symbol_type() {
            SymbolType::StructDeclaration |
            SymbolType::TypeAlias |
            SymbolType::ClassFieldDeclaration |
            SymbolType::VariableDefinition |
            SymbolType::FunctionDeclaration |
            SymbolType::CommentDefinition |
            SymbolType::Unknown => {
                if !symbol.name().is_empty() {
                    let definition = AltDefinition {
                        guid: symbol.guid().clone(),
                        parent_guid: symbol.parent_guid().clone().unwrap_or_default(),
                        path_for_guesswork: _path_of_node(&orig_map, Some(symbol.guid().clone())),
                        symbol_type: symbol.symbol_type().clone(),
                        derived_from: vec![],
                        usages: vec![],
                        full_range: symbol.full_range().clone(),
                        declaration_range: symbol.declaration_range().clone(),
                        definition_range: symbol.definition_range().clone(),
                    };
                    definitions.insert(definition.guid.clone(), definition);
                } else {
                    tracing::info!("No name decl {}:{}", cpath, symbol.full_range().start_point.row + 1);
                }
            }
            SymbolType::ImportDeclaration |
            SymbolType::FunctionCall |
            SymbolType::VariableUsage => {
                // do nothing
            }
        }
    }

    for symbol in symbols2 {
        let symbol = symbol.read();
        eprintln!("something: {:?}", symbol);
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
                let mut where_is_this = vec!["?".to_string()];
                let mut debug_hint = "".to_string();

                if function_call.name().is_empty() {
                    tracing::error!("Error parsing {}:{}\nNo name in the call present", cpath, fields.full_range.start_point.row + 1);
                    continue;
                }

                // caller_guid in this case refers to "self" in self.x
                // we need to discover type of "caller", and parent.usages += type(caller)
                if let Some(caller_guid) = caller_guid {
                    if let Some(caller_node_arc) = orig_map.get(&caller_guid) {
                        let caller_node = caller_node_arc.read();
                        let caller_name = caller_node.name();
                        eprintln!("Resolved caller: {:?}, Name: {:?}", caller_guid, caller_name); // Print the name
                        let typeof_caller = _attempt_typeof_path(&orig_map, &global_path, caller_guid, caller_node.name().to_string());
                        where_is_this = [
                            typeof_caller,
                            vec![function_call.name().to_string()]
                        ].concat();
                        debug_hint = caller_node.name().to_string();
                        eprintln!("where_is_this1: {:?}\n", where_is_this);
                    } else {
                        // XXX UGLY: unfortunately, unresolved caller means no caller in C++, maybe in other languages
                        // caller is about caller.function_call(1, 2, 3), in this case means just function_call(1, 2, 3) without anything on the left
                        eprintln!("where_is_this2: looking for  {:?}\n", function_call.name().to_string());
                        where_is_this = _attempt_name2path(&orig_map, &global_path, function_call.parent_guid().clone(), function_call.name().to_string());
                        eprintln!("where_is_this2: {:?}\n", where_is_this);
                    }
                }

                let parent_decl_guid = _go_to_parent_until_declaration(&orig_map, symbol.parent_guid().unwrap_or_default());
                if let Some(definition) = definitions.get_mut(&parent_decl_guid) {
                    // eprintln!("Parent definition found for function call: {:?}", definition);
                    definition.usages.push(AltLink {
                        guid: symbol.guid().clone(),
                        target_for_guesswork: where_is_this,
                        debug_hint: debug_hint,
                    });
                }
            }
            SymbolType::VariableUsage => {
            //     let variable_usage = symbol.as_any().downcast_ref::<VariableUsage>().expect("xxx1001");
            //     let fields = variable_usage.fields();
            //     let guid = fields.guid.clone();
            //     let parent_guid = fields.parent_guid.clone();
            //     let name = fields.name.clone();
            //     let full_range = fields.full_range.clone();
            //     let childs_guid = fields.childs_guid.clone();
            //     let caller_guid = fields.caller_guid.clone();
            //     let linked_decl_guid = fields.linked_decl_guid.clone();
            //     eprintln!(
            //         "Variable usage found: guid: {:?}, parent_guid: {:?}, name: {:?}, full_range: {:?}, childs_guid: {:?}, caller_guid: {:?}, linked_decl_guid: {:?}",
            //         guid, parent_guid, name, full_range, childs_guid, caller_guid, linked_decl_guid
            //     );
            //     // if let Some(parent_guid) = parent_guid {
            //     //     if let Some(parent_definition) = definitions.get_mut(&parent_guid) {
            //     //         eprintln!("Resolved parent definition: {:?}", parent_definition.path());
            //     //         parent_definition.usages.push(AltLink {
            //     //             guid: Uuid::nil(),
            //     //             target_for_guesswork: vec![name],
            //     //         });
            //     //     } else {
            //     //         eprintln!("Unresolved parent definition: {:?}", parent_guid);
            //     //     }
            //     // }

            //     // caller logic

            //     if let Some(linked_decl_typedef) = symbol.get_linked_decl_type() {
            //         // #[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
            //         // pub struct TypeDef {
            //         //  pub name: Option<String>,
            //         //  pub inference_info: Option<String>,
            //         //  pub inference_info_guid: Option<Uuid>,
            //         //  pub is_pod: bool,
            //         //  pub namespace: String,
            //         //  pub guid: Option<Uuid>,
            //         //  pub nested_types: Vec<TypeDef>, // for nested types, presented in templates
            //         // }
            //         eprintln!("typedef: {:?}", linked_decl_typedef);
            //     }

            //     if let Some(linked_decl_guid) = linked_decl_guid {
            //         if let Some(linked_decl_definition) = definitions.get(&linked_decl_guid) {
            //             eprintln!("Resolved linked declaration definition: {:?}", linked_decl_definition);
            //         } else {
            //             eprintln!("Unresolved linked declaration definition: {:?}", linked_decl_guid);
            //         }
            //     }
            //     for child_guid in &childs_guid {
            //         if let Some(child_definition) = definitions.get(child_guid) {
            //             eprintln!("Resolved child definition: {:?}", child_definition);
            //         } else {
            //             eprintln!("Unresolved child definition: {:?}", child_guid);
            //         }
            //     }
            //     eprintln!("");
            }
        }
        eprintln!("");
    }

    let mut sorted_definitions: Vec<(Uuid, AltDefinition)> = definitions.clone().into_iter().collect();
    sorted_definitions.sort_by(|a, b| a.1.path_for_guesswork.cmp(&b.1.path_for_guesswork));
    IndexMap::from_iter(sorted_definitions)
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
    use std::io::stderr;
    use tracing_subscriber::fmt::format;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_writer(stderr)
            .with_max_level(tracing::Level::INFO)
            .event_format(format::Format::default())
            .try_init();
    }

    fn read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    #[test]
    fn test_parse_anything_frog_py() {
        init_tracing();
        let absfn = std::fs::canonicalize("tests/emergency_frog_situation/frog.py").unwrap();
        let text = read_file(absfn.to_str().unwrap());
        let definitions = parse_anything(absfn.to_str().unwrap(), &text);
        let mut produced_output = String::new();
        for d in definitions.values() {
            produced_output.push_str(&format!("{:?}\n", d));
        }
        println!("\n --- {:#?} ---\n{}", absfn, produced_output.clone());
    }

    #[test]
    fn test_parse_anything_compiled_frog_cpp() {
        init_tracing();
        let absfn = std::fs::canonicalize("tests/emergency_frog_situation/compiled_frog.cpp").unwrap();
        let text = read_file(absfn.to_str().unwrap());
        let definitions = parse_anything(absfn.to_str().unwrap(), &text);
        const EXPECTED_COMPILED_FROG_CPP: &str = r#"
            AltDefinition { Animal }
            AltDefinition { Animal::Animal }
            AltDefinition { Animal::age }
            AltDefinition { CompiledFrog }
            AltDefinition { CompiledFrog::CompiledFrog }
            AltDefinition { CompiledFrog::say_hi, usages: Link{  ?::printf } }
            AltDefinition { HasMass }
            AltDefinition { HasMass::HasMass }
            AltDefinition { HasMass::mass }
            AltDefinition { global_frog }
            AltDefinition { main, usages: Link{  file::some_fun } Link{  file::some_variable_usage } }
            AltDefinition { main::shared_frog }
            AltDefinition { main::teh_frog }
            AltDefinition { some_fun, usages: Link{ f1 file::CompiledFrog::say_hi } Link{ f2 file::CompiledFrog::say_hi } Link{ f3 file::CompiledFrog::say_hi } Link{ f4 ?::say_hi } Link{ f_local_frog file::CompiledFrog::say_hi } Link{ global_frog file::CompiledFrog::say_hi } }
            AltDefinition { some_fun::f_local_frog }
            AltDefinition { some_variable_usage }
            AltDefinition { some_variable_usage::v_local_frog }
        "#;
        let mut produced_output = String::new();
        for d in definitions.values() {
            produced_output.push_str(&format!("{:?}\n", d));
        }
        println!("\n --- {:#?} ---\n{}", absfn, produced_output.clone());
        must_be_no_diff(EXPECTED_COMPILED_FROG_CPP, &produced_output);
    }

    fn must_be_no_diff(expected: &str, produced: &str) {
        use std::collections::HashSet;
        let expected_lines: HashSet<_> = expected.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        let produced_lines: HashSet<_> = produced.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        let missing_in_produced: Vec<_> = expected_lines.difference(&produced_lines).collect();
        let missing_in_expected: Vec<_> = produced_lines.difference(&expected_lines).collect();
        if !missing_in_produced.is_empty() {
            println!("Missing in produced output:");
            for line in missing_in_produced.iter() {
                println!("{}", *line);
            }
        }
        if !missing_in_expected.is_empty() {
            println!("Missing in expected output:");
            for line in missing_in_expected.iter() {
                println!("{}", *line);
            }
        }
        assert!(missing_in_expected.is_empty() && missing_in_produced.is_empty());
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

