use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Point, Range, Query, QueryCursor};
use tree_sitter_python::language;

use crate::ast::ast_structs::{AstDefinition, AstUsage};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::parse_common::ContextAnyParser;


pub struct ContextPy<'a> {
    pub ap: ContextAnyParser<'a>,
    pub ass1: Query,
    pub ass2: Query,
    pub ass3: Query,
    pub ass4: Query,
    pub tuple1: Query,
    pub tuple2: Query,
    pub tuple3: Query,
    pub class1: Query,
}

fn generate_usage(cx: &mut ContextPy, node: &Node, debug_note: &str, lhs_target: &str, rhs_target: &String) {
    // print!("\x1b[34m<usage {} lhs_type={:?} rhs_type={:?}>\x1b[0m", debug_note, lhs_target, rhs_target);
}

fn py_typeof(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("TYPE[{}]", &cx.ap.code[node.byte_range()]));
}

fn py_lvalue(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("LVALUE[{}]", &cx.ap.code[node.byte_range()]));
}

fn py_rvalue(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("VAL[{}]", &cx.ap.code[node.byte_range()]));
}

fn py_assignment(cx: &mut ContextPy, node: &Node)
{
    let mut lhs_tuple: Vec<(String, Option<String>)> = Vec::new();
    for query in [&cx.ass1, &cx.ass2, &cx.ass3, &cx.ass4] {
        let mut query_cursor = QueryCursor::new();
        for m in query_cursor.matches(&query, *node, cx.ap.code.as_bytes()) {
            let mut lhs_text = None;
            let mut lhs_type = None;
            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                if capture_name == "lhs" {
                    lhs_text = py_lvalue(cx, &capture.node);
                } else if capture_name == "type" {
                    lhs_type = py_typeof(cx, &capture.node);
                }
            }
            if let Some(lhs_text) = lhs_text {
                lhs_tuple.push((lhs_text, lhs_type));
            }
        }
        if !lhs_tuple.is_empty() {
            break;
        }
    }

    let mut rhs_tuple: Vec<String> = Vec::new();
    for query in [&cx.tuple1, &cx.tuple2, &cx.tuple3] {
        let mut query_cursor = QueryCursor::new();
        for m in query_cursor.matches(&query, *node, cx.ap.code.as_bytes()) {
            let mut rhs_text = None;
            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                if capture_name == "rhs" {
                    rhs_text = py_rvalue(cx, &capture.node);
                }
            }
            if let Some(rhs_text) = rhs_text {
                rhs_tuple.push(rhs_text);
            }
        }
    }

    println!();
    for i in 0 .. (lhs_tuple.len().min(rhs_tuple.len())) {
        let (lhs_text, lhs_type) = &lhs_tuple[i];
        let rhs_text = &rhs_tuple[i];
        println!("{}: {:?} = {}", lhs_text, lhs_type, rhs_text);
    }
}

fn py_class(cx: &mut ContextPy, node: &Node, path: &Vec<String>)
{
    let mut derived_from = vec![];
    let mut query_cursor = QueryCursor::new();
    for m in query_cursor.matches(&cx.class1, *node, cx.ap.code.as_bytes()) {
        for capture in m.captures {
            let capture_name = cx.class1.capture_names()[capture.index as usize];
            if capture_name == "dfrom" {
                derived_from.push(format!("py🔎{}", cx.ap.code[capture.node.byte_range()].to_string()));
            }
        }
    }

    let mut body_line1 = usize::MAX;
    let mut body_line2 = 0;
    let mut class_name = "".to_string();
    let mut block = None;
    for i in 0 .. node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "identifier" {
            class_name = cx.ap.code[child.byte_range()].to_string();
        }
        if child.kind() == "block" {
            body_line1 = body_line1.min(child.range().start_point.row + 1);
            body_line2 = body_line2.max(child.range().end_point.row + 1);
            block = Some(child);
        }
    }
    if class_name == "" {
        return;
    }
    if block.is_none() {
        return;
    }

    let mut class_path = path.clone();
    class_path.push(class_name.clone());

    cx.ap.defs.insert(class_name.clone(), AstDefinition {
        official_path: class_path.clone(),
        symbol_type: SymbolType::StructDeclaration,
        usages: vec![],
        this_is_a_class: format!("py🔎{}", class_name),
        this_class_derived_from: derived_from,
        cpath: "".to_string(),
        decl_line1: node.range().start_point.row + 1,
        decl_line2: (node.range().start_point.row + 1).max(body_line1 - 1),
        body_line1,
        body_line2,
    });
    // println!("\n{}", serde_json::to_string_pretty(cx.defs.last().unwrap().1).unwrap());

    py_traverse(cx, &block.unwrap(), &class_path);
}


fn py_traverse(cx: &mut ContextPy, node: &Node, path: &Vec<String>)
{
    match node.kind() {
        "from" | "class" | "identifier" | "import" | "dotted_name" | "def" | "if" | "for" | ":" | "," => {
            // simple keywords
            cx.ap.whitespace1(node);
            cx.ap.just_print(node);
            cx.ap.whitespace2(node);
        },
        "module" | "block" | "if_statement" | "expression_statement" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                py_traverse(cx, &child, path);
            }
        },
        "class_definition" => {
            cx.ap.recursive_print_with_red_brackets(node);
            py_class(cx, node, path);
        },
        // TODO
        "import_from_statement" | "function_definition" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                py_traverse(cx, &child, path);
            }
        },
        "assignment" => {
            cx.ap.recursive_print_with_red_brackets(node);
            py_assignment(cx, node);
        }
        // "expression_statement" => {
        //     whitespace1(cx, node);
        //     print!("\x1b[31mexpression[\x1b[0m");
        //     handle_expression(cx, node);
        //     print!("\x1b[31m]\x1b[0m");
        //     whitespace2(cx, node);
        //     return;
        // }
        // "parameters" => handle_argument(cx, node),
        // "assignment" => handle_variable(cx, node),
        // "for_statement" => handle_variable(cx, node),
        _ => {
            // unknown, to discover new syntax, just print
            cx.ap.whitespace1(node);
            print!("\x1b[31m{}[\x1b[0m", node.kind());
            cx.ap.just_print(node);
            print!("\x1b[31m]\x1b[0m");
            cx.ap.whitespace2(node);
        }
    }
}

pub fn parse(code: &str)
{
    let mut sitter = Parser::new();
    sitter.set_language(&language()).unwrap();
    let mut cx = ContextPy {
        ap: ContextAnyParser {
            sitter,
            last_end_byte: 0,
            code,
            defs: IndexMap::new(),
        },
        // assignment[pattern_list[identifier[aaa1],·identifier[aaa2]]·=·expression_list[integer[13],·integer[14]]]
        ass1: Query::new(&language(), "(assignment left: (pattern_list (_) @lhs))").unwrap(),
        // assignment[tuple_pattern[([(]identifier[aaa2],·identifier[aaa3])[)]]·=·expression_list[integer[15],·integer[16]]]
        ass2: Query::new(&language(), "(assignment left: (tuple_pattern (_) @lhs))").unwrap(),
        // assignment[attribute[identifier[self].identifier[also1_age]]:·type[identifier[float]]·=·identifier[age]]
        ass3: Query::new(&language(), "(assignment left: (_) @lhs type: (_) @type)").unwrap(),
        // assignment[attribute[identifier[self].identifier[weight]] =·identifier[weight]]
        ass4: Query::new(&language(), "(assignment left: (_) @lhs)").unwrap(),
        // expression_list[integer[13],·integer[14]]]
        tuple1: Query::new(&language(), "(assignment right: (expression_list (_) @rhs))").unwrap(),
        // tuple[(integer[15],·integer[16])]]
        tuple2: Query::new(&language(), "(assignment right: (tuple (_) @rhs))").unwrap(),
        // integer[12]]
        tuple3: Query::new(&language(), "(assignment right: _ @rhs)").unwrap(),
        // class_definition[class·identifier[Goat]argument_list[(identifier[Animal])]:
        class1: Query::new(&language(), "(class_definition name: (_) superclasses: (argument_list (_) @dfrom))").unwrap(),
    };
    let tree = cx.ap.sitter.parse(code, None).unwrap();
    let path = vec!["file".to_string()];
    py_traverse(&mut cx, &tree.root_node(), &path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_py() {
        let code = include_str!("alt_testsuite/py_goat_library.py");
        parse(code);
    }
}
