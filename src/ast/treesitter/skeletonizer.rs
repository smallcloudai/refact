use std::collections::{HashMap, VecDeque};

use itertools::Itertools;
use uuid::Uuid;

use crate::ast::treesitter::ast_instance_structs::SymbolInformation;
use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::python::PythonSkeletonFormatter;
use crate::ast::treesitter::structs::SymbolType;

struct BaseSkeletonFormatter;

pub trait SkeletonFormatter {
    fn make_skeleton(&self,
                     symbol: &SymbolInformation,
                     guid_to_children: &HashMap<Uuid, Vec<Uuid>>,
                     guid_to_info: &HashMap<Uuid, &SymbolInformation>) -> String {
        let mut res_line = symbol.get_declaration_content_blocked().unwrap()
            .split("\n")
            .map(|x| x.trim_start().to_string())
            .collect::<Vec<_>>();
        let children = guid_to_children.get(&symbol.guid).unwrap();
        let last: &mut String = res_line.last_mut().unwrap();
        if children.is_empty() {
            last.push_str(" { ... }");
            return res_line.join("\n");
        }
        last.push_str("{");
        for child in children {
            let child_symbol = guid_to_info.get(&child).unwrap();
            match child_symbol.symbol_type {
                SymbolType::FunctionDeclaration | SymbolType::ClassFieldDeclaration => {
                    let mut content = child_symbol.get_declaration_content_blocked().unwrap()
                        .split("\n")
                        .map(|x| x.trim_start().to_string())
                        .collect::<Vec<_>>();
                    let last_: &mut String = content.last_mut().unwrap();
                    if !last_.ends_with(";") {
                        if child_symbol.symbol_type == SymbolType::FunctionDeclaration {
                            last_.push_str("{ ... }");
                        } else if child_symbol.symbol_type == SymbolType::ClassFieldDeclaration {
                            last_.push_str(",");
                        }
                    }
                    for content in content.iter() {
                        res_line.push(format!("  {}", content));
                    }
                }
                _ => {}
            }
        }

        res_line.push("}".to_string());
        res_line.join("\n")
    }

    fn get_declaration_with_comments(&self,
                                     symbol: &SymbolInformation,
                                     _guid_to_children: &HashMap<Uuid, Vec<Uuid>>,
                                     guid_to_info: &HashMap<Uuid, &SymbolInformation>) -> (String, (usize, usize)) {
        let mut res_line: VecDeque<String> = Default::default();
        let mut top_row = symbol.full_range.start_point.row;
        let mut all_top_syms = guid_to_info.values().filter(|info| info.full_range.start_point.row < top_row).collect::<Vec<_>>();
        // reverse sort
        all_top_syms.sort_by(|a, b| b.full_range.start_point.row.cmp(&a.full_range.start_point.row));
        for sym in all_top_syms {
            if sym.symbol_type != SymbolType::CommentDefinition {
                break;
            }
            top_row = sym.full_range.start_point.row;
            let content = sym.get_content_blocked().unwrap();
            let lines = content.split("\n").collect::<Vec<_>>();
            let lines = lines.iter()
                .map(|x| x.trim_start().to_string())
                .collect::<Vec<_>>();
            lines.into_iter().rev().for_each(|x| res_line.push_front(x));
            // res_line.extend(lines);
        }
        if res_line.is_empty() {
            return ("".to_string(), (0, 0));
        }
        let content = symbol.get_declaration_content_blocked().unwrap().split("\n")
            .map(|x| x.trim_start().to_string())
            .collect::<Vec<_>>();
        let mut declaration = format!("{}\n{}", res_line.into_iter().join("\n"), content.join("\n"));
        if vec![SymbolType::FunctionDeclaration, SymbolType::StructDeclaration].contains(&symbol.symbol_type) {
            if !declaration.ends_with(";") {
                declaration.push_str("{ ... }");
            }
        }
        (declaration, (top_row, symbol.full_range.start_point.row))
    }
}

impl SkeletonFormatter for BaseSkeletonFormatter {}

pub fn make_formatter(language_id: &LanguageId) -> Box<dyn SkeletonFormatter> {
    match language_id {
        LanguageId::Python => Box::new(PythonSkeletonFormatter {}),
        _ => Box::new(BaseSkeletonFormatter {})
    }
}
