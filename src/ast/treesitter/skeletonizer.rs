use std::collections::{HashMap, VecDeque};

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
            .map(|x| x.trim_start().trim_end().to_string())
            .collect::<Vec<_>>();
        let children = guid_to_children.get(&symbol.guid).unwrap();
        let last: &mut String = res_line.last_mut().unwrap();
        if children.is_empty() {
            last.push_str(" { ... }");
            return res_line.join("\n");
        }
        last.push_str(" {");
        for child in children {
            let child_symbol = guid_to_info.get(&child).unwrap();
            match child_symbol.symbol_type {
                SymbolType::FunctionDeclaration | SymbolType::ClassFieldDeclaration => {
                    let mut content = child_symbol.get_declaration_content_blocked().unwrap()
                        .split("\n")
                        .map(|x| x.trim_start().trim_end().to_string())
                        .collect::<Vec<_>>();
                    let last_: &mut String = content.last_mut().unwrap();
                    if !last_.ends_with(";") && !last_.ends_with("}") {
                        if child_symbol.symbol_type == SymbolType::FunctionDeclaration {
                            last_.push_str(" { ... }");
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

    fn preprocess_content(&self, content: Vec<String>) -> Vec<String> {
        let lines = content.iter()
            .map(|x| x.replace("\r", "")
                .replace("\t", "    ").to_string())
            .collect::<Vec<_>>();
        let indent_n = content.iter().map(|x| {
            if x.is_empty() {
                return usize::MAX;
            } else {
                x.len() - x.trim_start().len()
            }
        }).min().unwrap_or(0);
        let intent = " ".repeat(indent_n).to_string();

        lines.iter().map(|x| if x.starts_with(&intent) {
            x[indent_n..x.len()].to_string()
        } else {x.to_string()}).collect::<Vec<_>>()
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

        let mut need_syms: Vec<&&SymbolInformation> = vec![];
        {
            for idx in 0..all_top_syms.len() {
                let sym = all_top_syms[idx];
                if sym.symbol_type != SymbolType::CommentDefinition {
                    break;
                }
                let all_sym_on_this_line = all_top_syms.iter()
                    .filter(|info|
                        info.full_range.start_point.row == sym.full_range.start_point.row ||
                            info.full_range.end_point.row == sym.full_range.start_point.row).collect::<Vec<_>>();

                if all_sym_on_this_line.iter().all(|info| info.symbol_type == SymbolType::CommentDefinition) {
                    need_syms.push(sym);
                } else {
                    break
                }
            }
        }


        for sym in need_syms {
            if sym.symbol_type != SymbolType::CommentDefinition {
                break;
            }
            top_row = sym.full_range.start_point.row;
            let mut content = sym.get_content_blocked().unwrap();
            if content.ends_with("\n") {
                content.pop();
            }
            let lines = content.split("\n").collect::<Vec<_>>();
            let lines = lines.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            lines.into_iter().rev().for_each(|x| res_line.push_front(x));
        }

        let mut bottom_row = symbol.full_range.start_point.row;
        if symbol.symbol_type == SymbolType::StructDeclaration {
            if res_line.is_empty() {
                return ("".to_string(), (top_row, bottom_row));
            }
            let mut content = symbol.get_declaration_content_blocked().unwrap().split("\n")
                .map(|x| x.trim_end().to_string())
                .collect::<Vec<_>>();
            if let Some(last) = content.last_mut() {
                if !last.ends_with(";") {
                    last.push_str(" { ... }");
                }
            }
            res_line.extend(content.into_iter());
        } else if symbol.symbol_type == SymbolType::FunctionDeclaration {
            let content = symbol.get_content_blocked().unwrap().split("\n")
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            res_line.extend(content.into_iter());
            bottom_row = symbol.full_range.end_point.row;
        }
        let res_line = self.preprocess_content(Vec::from_iter(res_line.into_iter()));
        let declaration = res_line.join("\n");
        (declaration, (top_row, bottom_row))
    }
}

impl SkeletonFormatter for BaseSkeletonFormatter {}

pub fn make_formatter(language_id: &LanguageId) -> Box<dyn SkeletonFormatter> {
    match language_id {
        LanguageId::Python => Box::new(PythonSkeletonFormatter {}),
        _ => Box::new(BaseSkeletonFormatter {})
    }
}
