use std::collections::{HashMap, HashSet};
use std::iter::Iterator;
use std::ops::Deref;
use std::string::ToString;
use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Parser, Range};
use crate::ast::treesitter::parsers::{AstConfig, Language, TypeDeclarationSearchInfo};
use crate::ast::treesitter::index::{Index, SymbolInfo, SymbolType};

pub struct PythonConfig;

impl Language for PythonConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "False", "def", "if", "raise", "None", "del", "import", "return", "True", "elif", "in",
                "try", "and", "else", "is", "while", "as", "except", "lambda", "with", "assert", "finally",
                "nonlocal", "yield", "break", "for", "not", "class", "from", "or", "continue", "global", "pass",
                "__init__", "__str__", "__repr__", "__len__", "__getitem__", "__setitem__", "__delitem__",
                "__del__", "__iter__", "__reversed__", "__cmp__", "__lt__", "__gt__", "__le__", "__ge__", "__all__",
                "__format__", "__sizeof__", "__str__", "__repr__", "__hash__", "__cmp__", "__lt__", "__gt__",
                "__call__", "Dict", "List", "Tuple", "Set", "Dict", "String", "Bytes", "Bytes", "self", "str", "dict",
                "int", "float", "str", "bool", "None", "bytes", "bytes",
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}
// 
// const PARSER_QUERY_CPP_GLOBAL_VARIABLE: QueryInfo = QueryInfo::new(
//     "(module (expression_statement (assignment left: (identifier) @global_var.name) @global_var.declaration))",
//     &["name", "declaration"],
// );
// 
// const PARSER_QUERY_CPP_FUNCTION: QueryInfo = QueryInfo::new(
//     "(module ((function_definition name: (identifier) @function.name) @function.declaration))",
//     &["name", "declaration"],
// );
// 
// const PARSER_QUERY_CPP_CLASS: QueryInfo = QueryInfo::new(
//     "((class_definition name: (identifier) @class.name) @class.declaration)",
//     &["name", "declaration"],
// );
// 
// const PARSER_QUERY_CPP_CALL_FUNCTION: QueryInfo = QueryInfo::new(
//     "((call function: (identifier) @call_function.name arguments: (argument_list) @call_function.args))",
//     &["name", "args"],
// );
// 
// const PARSER_QUERY_CPP_IMPORT_STATEMENT: QueryInfo = QueryInfo::new(
//     "((import_statement name: (dotted_name) @import.name))",
//     &["name"],
// );
// 
// const PARSER_QUERY_CPP_IMPORT_FROM_STATEMENT: QueryInfo = QueryInfo::new(
//     "(import_from_statement module_name: (dotted_name) @module.name)",
//     &["name", "module"],
// );
// 
// const PARSER_QUERY_CPP_CLASS_METHOD: QueryInfo = QueryInfo::new(
//     "method",
//     "(class_definition (block (function_definition name: (identifier) @method.name) @method.declaration))",
//     &["name", "declaration"],
// );
// 
// lazy_static! {
//     static ref PYTHON_ALL_QUERIES: HashMap<&'static str, QueryInfo<'static>> = {
//         let mut m = HashMap::new();
//         m.insert(PARSER_QUERY_CPP_GLOBAL_VARIABLE.prefix, PARSER_QUERY_CPP_GLOBAL_VARIABLE);
//         m.insert(PARSER_QUERY_CPP_FUNCTION.prefix, PARSER_QUERY_CPP_FUNCTION);
//         m.insert(PARSER_QUERY_CPP_CLASS.prefix, PARSER_QUERY_CPP_CLASS);
//         m.insert(PARSER_QUERY_CPP_CALL_FUNCTION.prefix, PARSER_QUERY_CPP_CALL_FUNCTION);
//         m.insert(PARSER_QUERY_CPP_IMPORT_STATEMENT.prefix, PARSER_QUERY_CPP_IMPORT_STATEMENT);
//         m.insert(PARSER_QUERY_CPP_IMPORT_FROM_STATEMENT.prefix, PARSER_QUERY_CPP_IMPORT_FROM_STATEMENT);
//         m.insert(PARSER_QUERY_CPP_CLASS_METHOD.prefix, PARSER_QUERY_CPP_CLASS_METHOD);
//         m
//     };
//     static ref PARSER_QUERY_PYTHON: String = QueryInfo::compose_query(PYTHON_ALL_QUERIES.deref());
// }
// 
// struct Candidate {
//     pub capture_name: String,
//     pub content: String,
//     pub range: Range,
// }
// 
// fn make_index_from_candidates(candidates: &Vec<Candidate>, query_info: &QueryInfo) -> Index {
//     let mut index = Index {
//         name: Default::default(),
//         used: Default::default(),
//         definition_info: None,
//         children: vec![].into(),
//         symbol_type: SymbolType::GlobalVar,
//     };
//     for c in candidates {
//         let mut split = c.capture_name.split(".");
//         let prefix = split.nth(0).unwrap();
//         let capture_name_wo_pfx = split.nth(0).unwrap();
//         index.symbol_type = prefix.parse().unwrap();
//         assert!(query_info.statement_names.contains(&capture_name_wo_pfx));
//         match capture_name_wo_pfx {
//             "name" => index.name = c.content.clone(),
//             "declaration" => index.definition_info = Some(DefinitionInfo {
//                 symbol_info: SymbolInfo { path: Default::default(), range: c.range },
//                 text: c.content.clone(),
//             }),
//             &_ => {}
//         }
//     }
//     index
// }
// 
// pub fn get_indexes(parser: &mut Parser, code: &str) -> HashMap<String, Index> {
//     let mut indexes: Vec<Index> = Default::default();
//     let tree = parser.parse(code, None).unwrap();
//     let mut qcursor = tree_sitter::QueryCursor::new();
//     let query = tree_sitter::Query::new(tree_sitter_python::language(), &PARSER_QUERY_PYTHON).unwrap();
//     let mut matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
//     let mut candidates: Vec<Candidate> = vec![];
//     for match_ in matches {
//         for capture in match_.captures {
//             let text = code.slice(capture.node.byte_range());
//             let capture_name = &query.capture_names()[capture.index as usize];
//             let prefix = capture_name.split(".").nth(0).unwrap();
//             candidates.push(Candidate {
//                 content: text.to_string(),
//                 capture_name: capture_name.to_string(),
//                 range: capture.node.range(),
//             });
//             if let Some(q_info) = PYTHON_ALL_QUERIES.get(prefix) {
//                 if q_info.statement_names.len() == candidates.len() {
//                     let index = make_index_from_candidates(&candidates, q_info);
//                     indexes.push(index);
//                     candidates.clear();
//                 }
//             }
//         }
//     }
//     match_indexes(indexes.clone());
//     indexes.into_iter().map(|i| (i.name.clone(), i)).collect()
// }
// 
// #[cfg(test)]
// mod tests {
//     use crate::lsp::treesitter::ast_config::python_config::get_indexes;
// 
//     const TEST_CODE: &str =
//         r#"import numpy as np
//   
// global_var = "pip"
// bar = true
// 
// class BabyClass:
//     def __init__(self):
//         self.xyi = 2
// class AdultClass:
//     def __init__(self):
//         self.xyi = 2
//         self.zxc = 4
//     class NestedClass:
//         def __init__(self):
//             self.c = 2
// 
// def baz(asd, zxc):
//     pass
// 
// @tits
// def foo():
//     if bar:
//         baz(asd, zxc)	
// "#;
// 
//     #[test]
//     fn test_query_CPP_function() {
//         let mut parser = tree_sitter::Parser::new();
//         parser.set_language(tree_sitter_python::language()).unwrap();
//         let indexes = get_indexes(&mut parser, TEST_CODE);
//         assert_eq!(indexes.len(), 1);
//         assert_eq!(indexes.get("function").unwrap().name, "foo");
//     }
// }
// 
// fn match_indexes(mut indexes: Vec<Index>) -> Vec<Index> {
//     let mut all_methods = indexes.iter().filter(|i| i.symbol_type == SymbolType::Method).map(|x| x.clone()).collect::<Vec<_>>();
//     let mut all_classes = indexes.iter_mut().filter(|i| i.symbol_type == SymbolType::Class).collect::<Vec<_>>();
//     all_classes.sort_by(|x, y| {
//         let xrange = x.definition_info.clone().unwrap().symbol_info.range;
//         let yrange = y.definition_info.clone().unwrap().symbol_info.range;
//         let xb = xrange.end_byte - xrange.start_byte;
//         let yb = yrange.end_byte - yrange.start_byte;
//         xb.partial_cmp(&yb).unwrap()
//     });
//     {
//         let mut matched_methods: HashSet<usize> = Default::default();
//         for class in &mut all_classes {
//             let class_range = class.definition_info.clone().unwrap().symbol_info.range;
//             for (idx, method) in all_methods.iter().clone().enumerate() {
//                 if matched_methods.contains(&idx) {
//                     continue;
//                 }
//                 let method_range = method.definition_info.clone().unwrap().symbol_info.range;
//                 if method_range.start_byte >= class_range.start_byte && method_range.end_byte <= class_range.end_byte {
//                     class.children.push(method.clone());
//                     matched_methods.insert(idx);
//                 }
//             }
//         }
//     }
//     let mut final_classes: Vec<Index> = vec![];
//     {
//         let copy_all_classes: Vec<Index> = all_classes.iter_mut().map(|x| x.clone()).collect();
//         let mut check_all = false;
//         let mut nested_classes: HashSet<String> = Default::default();
//         while !check_all {
//             check_all = true;
//             let len_classes = all_classes.len();
//             for class_idx in 0..len_classes {
//                 let class = &mut all_classes[class_idx];
//                 if nested_classes.contains(&class.name.clone()) {
//                     continue;
//                 }
//                 for class_idx2 in 0..len_classes {
//                     let nested_class = copy_all_classes[class_idx2].clone();
//                     if class.name == nested_class.name || nested_classes.contains(&nested_class.name) {
//                         continue;
//                     }
//                     let class_range = class.definition_info.clone().unwrap().symbol_info.range;
//                     let nested_class_range = nested_class.definition_info.clone().unwrap().symbol_info.range;
//                     if nested_class_range.start_byte >= class_range.start_byte && nested_class_range.end_byte <= class_range.end_byte {
//                         class.children.push(nested_class.clone());
//                         nested_classes.insert(nested_class.name.clone());
//                         check_all &= false;
//                     }
//                 }
//             }
//         }
//         final_classes = all_classes.iter_mut()
//             .filter(|x| !nested_classes.contains(&x.name.clone()))
//             .map(|x| x.clone()).collect();
//     }
//     final_classes
// }
// 
