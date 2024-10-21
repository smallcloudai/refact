use std::path::PathBuf;
use std::collections::HashMap;
use indexmap::IndexMap;
use uuid::Uuid;
use std::path::Path;
use sha2::{Sha256, Digest};

use crate::ast::ast_structs::{AstDefinition, AstUsage, AstErrorStats};
use crate::ast::treesitter::parsers::get_ast_parser_by_filename;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::ast_instance_structs::{VariableUsage, VariableDefinition, AstSymbolInstance, FunctionDeclaration, StructDeclaration, FunctionCall, AstSymbolInstanceArc};
use crate::ast::parse_common::line12mid_from_ranges;


const TOO_MANY_SYMBOLS_IN_FILE: usize = 10000;

fn _is_declaration(t: SymbolType) -> bool {
    match t {
        SymbolType::Module |
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
    map: &HashMap<Uuid, AstSymbolInstanceArc>,
    start_node: AstSymbolInstanceArc,
    errors: &mut AstErrorStats,
) -> Uuid {
    let start_node_read = start_node.read();
    let mut node_guid = start_node_read.parent_guid().unwrap_or_default();
    loop {
        let node_option = map.get(&node_guid);
        if node_option.is_none() {
            // XXX: legit in Python (assignment at top level, function call at top level)
            errors.add_error(
                "".to_string(), start_node_read.full_range().start_point.row + 1,
                format!("go_to_parent: parent decl not found for {:?}", start_node_read.name()).as_str(),
            );
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
    map: &HashMap<Uuid, AstSymbolInstanceArc>,
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

struct ParseContext {
    pub top_level: Vec<AstSymbolInstanceArc>,
    pub map: HashMap<Uuid, AstSymbolInstanceArc>,
    pub definitions: IndexMap<Uuid, AstDefinition>,
    pub file_global_path: Vec<String>,
    pub language: String,
}

fn _find_top_level_nodes(pcx: &mut ParseContext) -> &Vec<AstSymbolInstanceArc> {
    //
    // XXX UGLY: the only way to detect top level is to map.get(parent) if it's not found => then it's top level.
    //
    if pcx.top_level.is_empty() {
        let mut top_level: Vec<AstSymbolInstanceArc> = Vec::new();
        for (_, node_arc) in pcx.map.iter() {
            let node = node_arc.read();
            assert!(node.parent_guid().is_some());  // parent always exists for some reason :/
            if _is_declaration(node.symbol_type()) {
                if !pcx.map.contains_key(&node.parent_guid().unwrap()) {
                    top_level.push(node_arc.clone());
                }
            }
        }
        pcx.top_level = top_level;
    }
    &pcx.top_level
}

fn _name_to_usage(
    pcx: &mut ParseContext,
    uline: usize,
    start_node_guid: Option<Uuid>,
    name_of_anything: String,
    allow_global_ref: bool,
) -> Option<AstUsage> {
    if start_node_guid.is_none() {
        return None;
    }
    let mut result = AstUsage {
        targets_for_guesswork: vec![],
        resolved_as: "".to_string(),
        debug_hint: "n2p".to_string(),
        uline,
    };
    let mut node_guid = start_node_guid.unwrap();
    let mut look_here: Vec<AstSymbolInstanceArc> = Vec::new();
    loop {
        let node_option = pcx.map.get(&node_guid);
        if node_option.is_none() {
            break;
        }
        let node = node_option.unwrap().read();
        if _is_declaration(node.symbol_type()) {
            look_here.push(node_option.unwrap().clone());

            if let Some(function_declaration) = node.as_any().downcast_ref::<FunctionDeclaration>() {
                for arg in &function_declaration.args {
                    if arg.name == name_of_anything {
                        // eprintln!("{:?} is an argument in a function {:?} => ignore, no path at all, no link", name_of_anything, function_declaration.name());
                        return None;
                    }
                }
                // Add all children nodes (shallow)
                for child_guid in function_declaration.childs_guid() {
                    if let Some(child_node) = pcx.map.get(child_guid) {
                        if _is_declaration(child_node.read().symbol_type()) {
                            look_here.push(child_node.clone());
                        }
                    }
                }
            }

            if let Some(struct_declaration) = node.as_any().downcast_ref::<StructDeclaration>() {
                result.targets_for_guesswork.push(format!("?::{}🔎{}::{}", node.language().to_string(), struct_declaration.name(), name_of_anything));
                // Add all children nodes (shallow)
                for child_guid in struct_declaration.childs_guid() {
                    if let Some(child_node) = pcx.map.get(child_guid) {
                        if _is_declaration(child_node.read().symbol_type()) {
                            look_here.push(child_node.clone());
                        }
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

    let top_level_nodes = _find_top_level_nodes(pcx);
    look_here.extend(top_level_nodes.clone());

    for node_arc in look_here {
        let node = node_arc.read();

        if _is_declaration(node.symbol_type()) {
            // eprintln!("_name_to_usage {:?} looking in {:?}", name_of_anything, node.name());
            if node.name() == name_of_anything {
                result.resolved_as = [pcx.file_global_path.clone(), _path_of_node(&pcx.map, Some(node.guid().clone()))].concat().join("::");
                result.debug_hint = "up".to_string();
            }
        }
    }

    if allow_global_ref {
        result.targets_for_guesswork.push(format!("?::{}", name_of_anything));
        Some(result)
    } else {
        // ?::DerivedFrom1::f ?::DerivedFrom2::f f
        result.targets_for_guesswork.push(format!("{}", name_of_anything));
        Some(result)
    }
}

fn _typeof(
    pcx: &mut ParseContext,
    start_node_guid: Uuid,
    variable_or_param_name: String,
    errors: &mut AstErrorStats,
) -> Vec<String> {
    let mut node_guid = start_node_guid.clone();
    let mut look_here: Vec<AstSymbolInstanceArc> = Vec::new();

    // collect look_here by going higher
    loop {
        let node_option = pcx.map.get(&node_guid);
        if node_option.is_none() {
            break;
        }
        let node = node_option.unwrap().read();
        if _is_declaration(node.symbol_type()) {
            look_here.push(node_option.unwrap().clone());
            // Add all children nodes (shallow)
            for child_guid in node.childs_guid() {
                if let Some(child_node) = pcx.map.get(child_guid) {
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

    // add top level
    let top_level_nodes = _find_top_level_nodes(pcx);
    look_here.extend(top_level_nodes.clone());

    // now uniform code to look in each
    for node_arc in look_here {
        let node = node_arc.read();
        // eprintln!("attempt_typeof: look_here {:?} {:?}", node.guid(), node.name());

        // Check for VariableDefinition and match name
        if let Some(variable_definition) = node.as_any().downcast_ref::<VariableDefinition>() {
            // eprintln!("variable_definition.name {:?} {:?}", variable_definition.name(), variable_or_param_name);
            if variable_definition.name() == variable_or_param_name {
                if let Some(first_type) = variable_definition.types().get(0) {
                    let type_name = first_type.name.clone().unwrap_or_default();
                    if type_name.is_empty() {
                        errors.add_error("".to_string(), node.full_range().start_point.row + 1, "nameless type for variable definition");
                    } else {
                        return vec!["?".to_string(), format!("{}🔎{}", node.language().to_string(), type_name)];
                    }
                }
            }
        }

        // Check for FunctionDeclaration and match argument names
        if let Some(function_declaration) = node.as_any().downcast_ref::<FunctionDeclaration>() {
            for arg in &function_declaration.args {
                // eprintln!("function_declaration.arg.name {:?} {:?}", arg.name, variable_or_param_name);
                if arg.name == variable_or_param_name {
                    if let Some(arg_type) = &arg.type_ {
                        if arg_type.name.is_none() || arg_type.name.clone().unwrap().is_empty() {
                            errors.add_error("".to_string(), node.full_range().start_point.row + 1, "nameless type for function argument");
                        } else {
                            return vec!["?".to_string(), format!("{}🔎{}", node.language().to_string(), arg_type.name.clone().unwrap())];
                        }
                    }
                }
            }
        }
    }

    // vec!["?".to_string()]    -- don't produce resolvable links, produce homeless links instead
    // XXX: the "?" is still valid for C++, because there's no way to know if a symbol legitimately top level
    vec![]
}

fn _usage_or_typeof_caller_colon_colon_usage(
    pcx: &mut ParseContext,
    caller_guid: Option<Uuid>,
    uline: usize,
    symbol: &dyn AstSymbolInstance,
    errors: &mut AstErrorStats,
) -> Option<AstUsage> {
    // my_object.something_inside
    // ^^^^^^^^^ caller (can be None)
    //           ^^^^^^^^^^^^^^^^ symbol
    let caller_option = if let Some(guid) = caller_guid {
        pcx.map.get(&guid).cloned()
    } else {
        None
    };
    if let Some(caller) = caller_option {
        let mut result = AstUsage {
            targets_for_guesswork: vec![],
            resolved_as: "".to_string(),
            debug_hint: "caller".to_string(),
            uline,
        };
        let caller_node = caller.read();
        let typeof_caller = _typeof(pcx, caller_node.guid().clone(), caller_node.name().to_string(), errors);
        // typeof_caller will be "?" if nothing found, start with "file" if type found in the current file
        if typeof_caller.first() == Some(&"file".to_string()) {
            // actually fully resolved!
            result.resolved_as = [typeof_caller, vec![symbol.name().to_string()]].concat().join("::");
            result.debug_hint = caller_node.name().to_string();
        } else {
            // not fully resolved
            result.targets_for_guesswork.push([typeof_caller, vec![symbol.name().to_string()]].concat().join("::"));
            result.debug_hint = caller_node.name().to_string();
        }
        Some(result)
    } else {
        // Handle the case where caller_guid is None or not found in pcx.map
        //
        // XXX UGLY: unfortunately, unresolved caller means no caller in C++, maybe in other languages
        // caller is about caller.function_call(1, 2, 3), in this case means just function_call(1, 2, 3) without anything on the left
        // just look for a name in function's parent and above
        //
        let tmp = _name_to_usage(pcx, uline, symbol.parent_guid().clone(), symbol.name().to_string(), false);
        // eprintln!("    _usage_or_typeof_caller_colon_colon_usage {} _name_to_usage={:?}", symbol.name().to_string(), tmp);
        tmp
    }
}

pub fn parse_anything(
    cpath: &str,
    text: &str,
    errors: &mut AstErrorStats,
) -> Result<(Vec<AstDefinition>, String), String>
{
    let path = PathBuf::from(cpath);
    let (mut parser, language_id) = get_ast_parser_by_filename(&path).map_err(|err| err.message)?;
    let language = language_id.to_string();
    tracing::info!("PARSE {} {}", language, cpath);
    if language == "python" {
        let mut cx = crate::ast::parse_python::py_parse(text);
        return Ok((cx.ap.export_defs(cpath), "python".to_string()));
    }
    let file_global_path = vec!["file".to_string()];

    let symbols = parser.parse(text, &path);
    if symbols.len() > TOO_MANY_SYMBOLS_IN_FILE {
        return Err(format!("more than {} symbols, generated?", TOO_MANY_SYMBOLS_IN_FILE));
    }
    let symbols2 = symbols.clone();

    let mut pcx = ParseContext {
        top_level: Vec::new(),
        map: HashMap::new(),
        definitions: IndexMap::new(),
        file_global_path,
        language,
    };

    for symbol in symbols {
        let symbol_arc_clone = symbol.clone();
        let symbol = symbol.read();
        pcx.map.insert(symbol.guid().clone(), symbol_arc_clone);
        match symbol.symbol_type() {
            SymbolType::StructDeclaration |
            SymbolType::TypeAlias |
            SymbolType::ClassFieldDeclaration |
            SymbolType::VariableDefinition |
            SymbolType::FunctionDeclaration |
            SymbolType::Unknown => {
                let mut this_is_a_class = "".to_string();
                let mut this_class_derived_from = vec![];
                let mut usages = vec![];
                if let Some(struct_declaration) = symbol.as_any().downcast_ref::<StructDeclaration>() {
                    this_is_a_class = format!("{}🔎{}", pcx.language, struct_declaration.name());
                    for base_class in struct_declaration.inherited_types.iter() {
                        let base_class_name = base_class.name.clone().unwrap_or_default();
                        if base_class_name.is_empty() {
                            errors.add_error("".to_string(), struct_declaration.full_range().start_point.row + 1, "nameless base class");
                            continue;
                        }
                        this_class_derived_from.push(format!("{}🔎{}", pcx.language, base_class_name));
                        if let Some(usage) = _name_to_usage(&mut pcx, symbol.full_range().start_point.row + 1, symbol.parent_guid().clone(), base_class_name, true) {
                            usages.push(usage);
                        } else {
                            errors.add_error("".to_string(), struct_declaration.full_range().start_point.row + 1, "unable to create base class usage");
                        }
                    }
                }
                let mut skip_var_because_parent_is_function = false;
                if let Some(_) = symbol.as_any().downcast_ref::<VariableDefinition>() {
                    if let Some(parent_guid) = symbol.parent_guid() {
                        if let Some(parent_symbol) = pcx.map.get(&parent_guid) {
                            let parent_symbol = parent_symbol.read();
                            if parent_symbol.as_any().downcast_ref::<FunctionDeclaration>().is_some() {
                                skip_var_because_parent_is_function = true;
                            }
                        }
                    }
                }
                if !symbol.name().is_empty() && !skip_var_because_parent_is_function {
                    let (line1, line2, line_mid) = line12mid_from_ranges(symbol.full_range(), symbol.definition_range());
                    let definition = AstDefinition {
                        official_path: _path_of_node(&pcx.map, Some(symbol.guid().clone())),
                        symbol_type: symbol.symbol_type().clone(),
                        resolved_type: "".to_string(),
                        this_is_a_class,
                        this_class_derived_from,
                        usages,
                        cpath: cpath.to_string(),
                        decl_line1: line1 + 1,
                        decl_line2: line2 + 1,
                        body_line1: line_mid + 1,
                        body_line2: line2 + 1,
                        // full_range: symbol.full_range().clone(),
                        // declaration_range: symbol.declaration_range().clone(),
                        // definition_range: symbol.definition_range().clone(),
                    };
                    pcx.definitions.insert(symbol.guid().clone(), definition);
                } else if symbol.name().is_empty() {
                    errors.add_error("".to_string(), symbol.full_range().start_point.row + 1, "nameless decl");
                }
            }
            SymbolType::Module |
            SymbolType::CommentDefinition |
            SymbolType::ImportDeclaration |
            SymbolType::FunctionCall |
            SymbolType::VariableUsage => {
                // do nothing
            }
        }
    }

    for symbol_arc in symbols2 {
        let symbol = symbol_arc.read();
        // eprintln!("pass2: {:?}", symbol);
        match symbol.symbol_type() {
            SymbolType::StructDeclaration |
            SymbolType::Module |
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
                let function_call = symbol.as_any().downcast_ref::<FunctionCall>().expect("xxx1000");
                let uline = function_call.full_range().start_point.row + 1;
                if function_call.name().is_empty() {
                    errors.add_error("".to_string(), uline, "nameless call");
                    continue;
                }
                let usage = _usage_or_typeof_caller_colon_colon_usage(&mut pcx, function_call.get_caller_guid().clone(), uline, function_call, errors);
                // eprintln!("function call name={} usage={:?} debug_hint={:?}", function_call.name(), usage, debug_hint);
                if usage.is_none() {
                    continue;
                }
                let my_parent = _go_to_parent_until_declaration(&pcx.map, symbol_arc.clone(), errors);
                if let Some(my_parent_def) = pcx.definitions.get_mut(&my_parent) {
                    my_parent_def.usages.push(usage.unwrap());
                }
            }
            SymbolType::VariableUsage => {
                let variable_usage = symbol.as_any().downcast_ref::<VariableUsage>().expect("xxx1001");
                let uline = variable_usage.full_range().start_point.row + 1;
                if variable_usage.name().is_empty() {
                    errors.add_error("".to_string(), uline, "nameless variable usage");
                    continue;
                }
                let usage = _usage_or_typeof_caller_colon_colon_usage(&mut pcx, variable_usage.fields().caller_guid.clone(), uline, variable_usage, errors);
                // eprintln!("variable usage name={} usage={:?}", variable_usage.name(), usage);
                if usage.is_none() {
                    continue;
                }
                let my_parent = _go_to_parent_until_declaration(&pcx.map, symbol_arc.clone(), errors);
                if let Some(my_parent_def) = pcx.definitions.get_mut(&my_parent) {
                    my_parent_def.usages.push(usage.unwrap());
                }
            }
        }
    }

    let mut sorted_definitions: Vec<(Uuid, AstDefinition)> = pcx.definitions.into_iter().collect();
    sorted_definitions.sort_by(|a, b| a.1.official_path.cmp(&b.1.official_path));
    let definitions: IndexMap<Uuid, AstDefinition> = IndexMap::from_iter(sorted_definitions);
    Ok((definitions.into_values().collect(), pcx.language))
}

pub fn filesystem_path_to_double_colon_path(cpath: &str) -> Vec<String> {
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
    let mut hasher = Sha256::new();
    hasher.update(cpath);
    let result = hasher.finalize();

    const ALPHANUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    let mut x = 0usize;
    let short_alphanum: String = result.iter()
        .map(|&byte| {
            x += byte as usize;
            x %= ALPHANUM.len();
            ALPHANUM[x] as char
        })
        .take(6)
        .collect();

    components.push(format!("${}", short_alphanum));
    components.iter().rev().take(3).cloned().collect::<Vec<_>>()
}

pub fn parse_anything_and_add_file_path(
    cpath: &str,
    text: &str,
    errstats: &mut AstErrorStats,
) -> Result<(Vec<AstDefinition>, String), String>
{
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let file_global_path_str = file_global_path.join("::");
    let errors_count_before = errstats.errors.len();
    let (mut definitions, language) = parse_anything(cpath, text, errstats)?;
    for error in errstats.errors.iter_mut().skip(errors_count_before) {
        error.err_cpath = cpath.to_string();
    }

    for definition in definitions.iter_mut() {
        if !definition.official_path.is_empty() && definition.official_path[0] == "root" {
            definition.official_path.remove(0);
        }
        definition.official_path = [
            file_global_path.clone(),
            definition.official_path.clone()
        ].concat();
        for usage in &mut definition.usages {
            for t in &mut usage.targets_for_guesswork {
                if t.starts_with("file::") || t.starts_with("root::") {
                    let path_within_file = t[4..].to_string();
                    t.clear();
                    t.push_str(file_global_path_str.as_str());
                    t.push_str(path_within_file.as_str());
                }
            }
            if usage.resolved_as.starts_with("file::") || usage.resolved_as.starts_with("root::") {
                let path_within_file = usage.resolved_as[4..].to_string();
                usage.resolved_as.clear();
                usage.resolved_as.push_str(file_global_path_str.as_str());
                usage.resolved_as.push_str(path_within_file.as_str());
            }
        }
    }
    Ok((definitions, language))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_subscriber;
    use std::io::stderr;
    use tracing_subscriber::fmt::format;

    fn _init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_writer(stderr)
            .with_max_level(tracing::Level::INFO)
            .event_format(format::Format::default())
            .try_init();
    }

    fn _read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    fn _must_be_no_diff(expected: &str, produced: &str) -> String {
        let expected_lines: Vec<_> = expected.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
        let produced_lines: Vec<_> = produced.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
        let mut mistakes = String::new();
        let missing_in_produced: Vec<_> = expected_lines.iter().filter(|line| !produced_lines.contains(line)).collect();
        let missing_in_expected: Vec<_> = produced_lines.iter().filter(|line| !expected_lines.contains(line)).collect();
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

    fn _run_parse_test(input_file: &str, correct_file: &str) {
        _init_tracing();
        let mut errstats = AstErrorStats::default();
        let absfn1 = std::fs::canonicalize(input_file).unwrap();
        let text = _read_file(absfn1.to_str().unwrap());
        let (definitions, _language) = parse_anything(absfn1.to_str().unwrap(), &text, &mut errstats).unwrap();
        let mut defs_str = String::new();
        for d in definitions.iter() {
            defs_str.push_str(&format!("{:?}\n", d));
        }
        println!("\n --- {:#?} ---\n{} ---\n", absfn1, defs_str.clone());
        let absfn2 = std::fs::canonicalize(correct_file).unwrap();
        let oops = _must_be_no_diff(_read_file(absfn2.to_str().unwrap()).as_str(), &defs_str);
        if !oops.is_empty() {
            println!("PROBLEMS {:#?}:\n{}/PROBLEMS", absfn1, oops);
        }
        for error in errstats.errors {
            println!("(E) {}:{} {}", error.err_cpath, error.err_line, error.err_message);
        }
    }

    #[test]
    fn test_ast_parse_cpp_library() {
        _run_parse_test(
            "src/ast/alt_testsuite/cpp_goat_library.h",
            "src/ast/alt_testsuite/cpp_goat_library.correct"
        );
    }

    #[test]
    fn test_ast_parse_cpp_main() {
        _run_parse_test(
            "src/ast/alt_testsuite/cpp_goat_main.cpp",
            "src/ast/alt_testsuite/cpp_goat_main.correct"
        );
    }

    #[test]
    fn test_ast_parse_py_library() {
        _run_parse_test(
            "src/ast/alt_testsuite/py_goat_library.py",
            "src/ast/alt_testsuite/py_goat_library.correct"
        );
    }
}

