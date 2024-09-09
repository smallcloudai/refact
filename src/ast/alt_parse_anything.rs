use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
use uuid::Uuid;
use crate::ast::alt_minimalistic::{AltDefinition, AltLink};
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::ast_instance_structs::{VariableUsage, VariableDefinition, AstSymbolInstance, FunctionDeclaration, StructDeclaration, FunctionCall, TypeDef};


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
    // XXX UGLY: the only way to detect top level is to map.get(parent) if it's not found => then it's top level.
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

            if let Some(function_declaration) = node.as_any().downcast_ref::<FunctionDeclaration>() {
                for arg in &function_declaration.args {
                    if arg.name == name_of_anything {
                        eprintln!("{:?} is an argument in a function {:?} => ignore, no path at all, no link", name_of_anything, function_declaration.name());
                        return vec![];
                    }
                }
            }

            if let Some(struct_declaration) = node.as_any().downcast_ref::<StructDeclaration>() {
                // Add all children nodes (shallow)
                for child_guid in struct_declaration.childs_guid() {
                    if let Some(child_node) = map.get(child_guid) {
                        look_here.push(child_node.clone());
                    }
                }
                for _base_class_guid in struct_declaration.inherited_types.iter() {
                    // pub struct TypeDef {
                    //     pub name: Option<String>,
                    //     pub inference_info: Option<String>,
                    //     pub inference_info_guid: Option<Uuid>,
                    //     pub is_pod: bool,
                    //     pub namespace: String,
                    //     pub guid: Option<Uuid>,
                    //     pub nested_types: Vec<TypeDef>, // for nested types, presented in templates
                    // }
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
            eprintln!("_attempt_name2path {:?} looking in {:?}", name_of_anything, node.name());
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

fn _usage_or_typeof_caller_colon_colon_usage(
    caller_guid: Option<Uuid>,
    orig_map: &HashMap<Uuid, std::sync::Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, Box<dyn AstSymbolInstance>>>>,
    global_path: &Vec<String>,
    symbol: &dyn AstSymbolInstance,
) -> (Vec<String>, String) {
    // let mut where_is_this = vec!["?".to_string()];
    // let mut debug_hint = "".to_string();
    let where_is_this;
    let debug_hint;
    if let Some(caller) = caller_guid.and_then(|guid| orig_map.get(&guid)) {
        let caller_node = caller.read();
        let caller_name = caller_node.name();
        eprintln!("Resolved caller: {:?}, Name: {:?}", caller_guid, caller_name); // Print the name
        let typeof_caller = _attempt_typeof_path(&orig_map, &global_path, caller_node.guid().clone(), caller_node.name().to_string());
        where_is_this = [
            typeof_caller,
            vec![symbol.name().to_string()]
        ].concat();
        debug_hint = caller_node.name().to_string();
        eprintln!("where_is_this1: {:?}", where_is_this);
    } else {
        // Handle the case where caller_guid is None or not found in orig_map
        // XXX UGLY: unfortunately, unresolved caller means no caller in C++, maybe in other languages
        // caller is about caller.function_call(1, 2, 3), in this case means just function_call(1, 2, 3) without anything on the left
        // just look for a name in function's parent and above
        eprintln!("where_is_this2: looking for  {:?}", symbol.name().to_string());
        where_is_this = _attempt_name2path(&orig_map, &global_path, symbol.parent_guid().clone(), symbol.name().to_string());
        if where_is_this.is_empty() {
            // empty means ignore it, unresolved will be ?::something
            debug_hint = "ignore".to_string();
        } else {
            debug_hint = "up".to_string();
        }
        eprintln!("where_is_this2: {:?} hint={:?}", where_is_this, debug_hint);
    }
    (where_is_this, debug_hint)
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
                        // guid: symbol.guid().clone(),
                        // parent_guid: symbol.parent_guid().clone().unwrap_or_default(),
                        official_path: _path_of_node(&orig_map, Some(symbol.guid().clone())),
                        symbol_type: symbol.symbol_type().clone(),
                        derived_from: vec![],
                        usages: vec![],
                        full_range: symbol.full_range().clone(),
                        declaration_range: symbol.declaration_range().clone(),
                        definition_range: symbol.definition_range().clone(),
                    };
                    definitions.insert(symbol.guid().clone(), definition);
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
                if function_call.name().is_empty() {
                    tracing::error!("Error parsing {}:{} no name in the call", cpath, fields.full_range.start_point.row + 1);
                    continue;
                }
                let (where_is_this, debug_hint) = _usage_or_typeof_caller_colon_colon_usage(caller_guid, &orig_map, &global_path, function_call);
                eprintln!("function call name={} where_is_this={:?} debug_hint={:?}", function_call.name(), where_is_this, debug_hint);
                if where_is_this.is_empty() {
                    continue;
                }
                let parent_decl_guid = _go_to_parent_until_declaration(&orig_map, symbol.parent_guid().unwrap_or_default());
                if let Some(definition) = definitions.get_mut(&parent_decl_guid) {
                    definition.usages.push(AltLink {
                        guid: symbol.guid().clone(),
                        target_for_guesswork: where_is_this,
                        debug_hint,
                    });
                }
            }
            SymbolType::VariableUsage => {
                let variable_usage = symbol.as_any().downcast_ref::<VariableUsage>().expect("xxx1001");
                let fields = variable_usage.fields();
                let caller_guid = fields.caller_guid.clone();
                if variable_usage.name().is_empty() {
                    tracing::error!("Error parsing {}:{} no name in variable usage", cpath, fields.full_range.start_point.row + 1);
                    continue;
                }
                let (where_is_this, debug_hint) = _usage_or_typeof_caller_colon_colon_usage(caller_guid, &orig_map, &global_path, variable_usage);
                eprintln!("variable usage name={} where_is_this={:?} debug_hint={:?}", variable_usage.name(), where_is_this, debug_hint);
                if where_is_this.is_empty() {
                    continue;
                }
                let parent_decl_guid = _go_to_parent_until_declaration(&orig_map, symbol.parent_guid().unwrap_or_default());
                if let Some(definition) = definitions.get_mut(&parent_decl_guid) {
                    definition.usages.push(AltLink {
                        guid: symbol.guid().clone(),
                        target_for_guesswork: where_is_this,
                        debug_hint,
                    });
                }
            }
        }
        eprintln!("");
    }

    let mut sorted_definitions: Vec<(Uuid, AltDefinition)> = definitions.clone().into_iter().collect();
    sorted_definitions.sort_by(|a, b| a.1.official_path.cmp(&b.1.official_path));
    IndexMap::from_iter(sorted_definitions)
}

pub fn filesystem_path_to_double_colon_path(cpath: &str) -> Vec<String> {
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

pub fn parse_anything_and_add_file_path(cpath: &str, text: &str) -> IndexMap<Uuid, AltDefinition> {
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let mut definitions = parse_anything(cpath, text);
    for definition in definitions.values_mut() {
        definition.official_path = [
            file_global_path.clone(),
            definition.official_path.clone()
        ].concat();
        for usage in &mut definition.usages {
            if usage.target_for_guesswork.starts_with(&vec!["file".to_string()]) {
                usage.target_for_guesswork = [
                    file_global_path.clone(),
                    usage.target_for_guesswork[1..].to_vec()
                ].concat();
            }
        }
    }
    definitions
}


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

    fn must_be_no_diff(expected: &str, produced: &str) -> String {
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
        let mut mistakes = String::new();

        if !missing_in_expected.is_empty() {
            mistakes.push_str("bad output:\n");
            for line in missing_in_expected.iter() {
                mistakes.push_str(&format!("  {}\n", *line));
            }
        }
        if !missing_in_produced.is_empty() {
            mistakes.push_str("should be:\n");
            for line in missing_in_produced.iter() {
                mistakes.push_str(&format!("  {}\n", *line));
            }
        }
        mistakes
    }

    fn run_parse_test(input_file: &str, correct_file: &str) {
        init_tracing();
        let absfn1 = std::fs::canonicalize(input_file).unwrap();
        let text = read_file(absfn1.to_str().unwrap());
        let definitions = parse_anything(absfn1.to_str().unwrap(), &text);
        let mut produced_output = String::new();
        for d in definitions.values() {
            produced_output.push_str(&format!("{:?}\n", d));
        }
        println!("\n --- {:#?} ---\n{} ---\n", absfn1, produced_output.clone());
        let absfn2 = std::fs::canonicalize(correct_file).unwrap();
        let errors = must_be_no_diff(read_file(absfn2.to_str().unwrap()).as_str(), &produced_output);
        if !errors.is_empty() {
            println!("PROBLEMS {:#?}:\n{}/PROBLEMS", absfn1, errors);
        }
    }

    #[test]
    fn test_parse_cpp_library() {
        run_parse_test(
            "src/ast/alt_testsuite/cpp_goat_library.h",
            "src/ast/alt_testsuite/cpp_goat_library.correct"
        );
    }

    #[test]
    fn test_parse_cpp_main() {
        run_parse_test(
            "src/ast/alt_testsuite/cpp_goat_main.cpp",
            "src/ast/alt_testsuite/cpp_goat_main.correct"
        );
    }
}

