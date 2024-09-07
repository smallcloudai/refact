use std::path::PathBuf;
use indexmap::IndexMap;
use uuid::Uuid;
use crate::ast::alt_minimalistic::{AltDefinition, AltLink};
use crate::ast::treesitter::parsers::{get_ast_parser_by_filename, AstLanguageParser};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::treesitter::ast_instance_structs::{VariableUsage, VariableDefinition, AstSymbolInstance};
use std::any::Any;


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

    for symbol in symbols {
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
                eprintln!("must be definition: {:?}\n", symbol);
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
                if let Some(definition) = definitions.get_mut(&symbol.parent_guid().unwrap_or_default()) {
                    // eprintln!("Parent definition found for function call: {:?}", definition);
                    // definition.usages.push(AltLink {
                    //     guid: symbol.guid().clone(),
                    //     target_for_guesswork: symbol.path_for_guesswork().clone(),
                    // });
                }
            }
            SymbolType::VariableUsage => {
                let variable_usage = symbol.as_any().downcast_ref::<VariableUsage>().expect("Expected VariableUsage type");
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
                if let Some(parent_guid) = parent_guid {
                    if let Some(parent_definition) = definitions.get_mut(&parent_guid) {
                        eprintln!("Resolved parent definition: {:?}", parent_definition.path());
                        parent_definition.usages.push(AltLink {
                            guid: Uuid::nil(),
                            target_for_guesswork: vec![name],
                        });
                    } else {
                        eprintln!("Unresolved parent definition: {:?}", parent_guid);
                    }
                }

                // caller_guid in this case refers to "self" in self.x
                // we need to discover type of "caller", and parent.usages += type(caller)
                if let Some(caller_guid) = caller_guid {
                    if let Some(caller_definition) = definitions.get(&caller_guid) {
                        eprintln!("Resolved caller definition: {:?}", caller_definition);
                    } else {
                        eprintln!("Unresolved caller definition: {:?}", caller_guid);
                    }
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

    #[test]
    fn test_parse_anything_frog_py() {
        init_tracing();
        let text = read_file("tests/emergency_frog_situation/frog.py");
        let definitions = parse_anything("tests/emergency_frog_situation/frog.py", &text);
        for d in definitions.values() {
            println!("{:#?}", d);
        }
        assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"Frog".to_string())));
        assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"__init__".to_string())));
        assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"bounce_off_banks".to_string())));
        assert!(definitions.values().any(|d| d.path_for_guesswork.contains(&"jump".to_string())));
    }

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
    // fn test_parse_anything_set_as_avatar_py() {
    //     let text = read_file("emergency_frog_situation/set_as_avatar.py");
    //     let definitions = parse_anything("emergency_frog_situation/set_as_avatar.py", &text);
    //     // Add assertions to check the parsed definitions
    //     assert!(definitions.iter().any(|d| d.path().contains("Toad")));
    //     assert!(definitions.iter().any(|d| d.path().contains("EuropeanCommonToad")));
    //     assert!(definitions.iter().any(|d| d.path().contains("__init__")));
    // }

    // #[test]
    // fn test_parse_anything_work_day_py() {
    //     let text = read_file("emergency_frog_situation/work_day.py");
    //     let definitions = parse_anything("emergency_frog_situation/work_day.py", &text);
    //     // Add assertions to check the parsed definitions
    //     assert!(definitions.iter().any(|d| d.path().contains("bring_your_own_frog_to_work_day")));
    // }
}

