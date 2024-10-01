use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Point, Range, Query, QueryCursor};
use crate::ast::ast_structs::{AstDefinition, AstUsage};
use tree_sitter_python::language;


pub struct ContextPy<'a> {
    pub sitter: Parser,
    pub last_end_byte: usize,
    pub code: &'a str,
    pub defs: IndexMap<String, AstDefinition>,
    pub ass1: Query,
    pub ass2: Query,
    pub ass3: Query,
    pub ass4: Query,
    pub tuple1: Query,
    pub tuple2: Query,
    pub tuple3: Query,
}

fn whitespace1(cx: &mut ContextPy, node: &Node) {
    if node.start_byte() > cx.last_end_byte {
        let whitespace = &cx.code[cx.last_end_byte..node.start_byte()];
        print!("\x1b[32m{}\x1b[0m", whitespace.replace(" ", "·"));
        cx.last_end_byte = node.start_byte();
    }
}

fn whitespace2(cx: &mut ContextPy, node: &Node) {
    cx.last_end_byte = node.end_byte();
}

fn just_print(cx: &mut ContextPy, node: &Node) {
    whitespace1(cx, node);
    print!("{}", &cx.code[node.byte_range()].replace(" ", "·"));
    whitespace2(cx, node);
}

fn recursive_print_with_red_brackets(cx: &mut ContextPy, node: &Node) {
    whitespace1(cx, node);
    match node.kind() {
        "from" | "class" | "import" | "def" | "if" | "for" | ":" | "," | "=" | "." | "(" | ")" => {
            // keywords
            print!("{}", &cx.code[node.byte_range()].replace(" ", "·"));
        },
        _ => {
            print!("\x1b[31m{}[\x1b[0m", node.kind());
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                recursive_print_with_red_brackets(cx, &child);
            }
            if node.child_count() == 0 {
                print!("{}", &cx.code[node.byte_range()]);
            }
            print!("\x1b[31m]\x1b[0m");
        }
    }
    whitespace2(cx, node);
}

fn generate_usage(cx: &mut ContextPy, node: &Node, debug_note: &str, lhs_target: &str, rhs_target: &String) {
    // print!("\x1b[34m<usage {} lhs_type={:?} rhs_type={:?}>\x1b[0m", debug_note, lhs_target, rhs_target);
}

fn py_typeof(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("TYPE[{}]", &cx.code[node.byte_range()]));
}

fn py_lvalue(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("LVALUE[{}]", &cx.code[node.byte_range()]));
}

fn py_rvalue(cx: &ContextPy, node: &Node) -> Option<String> {
    return Some(format!("VAL[{}]", &cx.code[node.byte_range()]));
}

fn py_assignment(cx: &mut ContextPy, node: &Node) {
    let mut lhs_tuple: Vec<(String, Option<String>)> = Vec::new();
    for query in [&cx.ass1, &cx.ass2, &cx.ass3, &cx.ass4] {
        let mut query_cursor = QueryCursor::new();
        for m in query_cursor.matches(&query, *node, cx.code.as_bytes()) {
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
        for m in query_cursor.matches(&query, *node, cx.code.as_bytes()) {
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
    println!("{:?}", lhs_tuple);
    println!("{:?}", rhs_tuple);
    for i in 0 .. (lhs_tuple.len().min(rhs_tuple.len())) {
        let (lhs_text, lhs_type) = &lhs_tuple[i];
        let rhs_text = &rhs_tuple[i];
        println!("{}: {:?} = {}", lhs_text, lhs_type, rhs_text);
    }
}

fn py_class(cx: &mut ContextPy, node: &Node, path: &Vec<String>)
{
    // let ass1 = r#"(assignment left: (pattern_list (_) @lhs))"#;
    // let ass2 = r#"(assignment left: (tuple_pattern (_) @lhs))"#;
    // let ass3 = r#"(assignment left: (_) @lhs type: (_) @type)"#;
    // let ass4 = r#"(assignment left: (_) @lhs)"#;

    // let mut lhs_tuple: Vec<(String, Option<String>)> = Vec::new();
    // let queries = [ass1, ass2, ass3, ass4];
    // for query_str in &queries {
    //     let query = Query::new(&language(), query_str).unwrap();
    //     let mut query_cursor = QueryCursor::new();
    //     for m in query_cursor.matches(&query, *node, cx.code.as_bytes()) {


}

fn py_traverse(cx: &mut ContextPy, node: &Node, path: &Vec<String>) {
    match node.kind() {
        "from" | "class" | "identifier" | "import" | "dotted_name" | "def" | "if" | "for" | ":" | "," => {
            // simple keywords
            whitespace1(cx, node);
            just_print(cx, node);
            whitespace2(cx, node);
        },
        "module" | "block" | "if_statement" | "expression_statement" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                py_traverse(cx, &child, path);
            }
        },
        "class_definition" => {
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
            recursive_print_with_red_brackets(cx, node);
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
            whitespace1(cx, node);
            print!("\x1b[31m{}[\x1b[0m", node.kind());
            just_print(cx, node);
            print!("\x1b[31m]\x1b[0m");
            whitespace2(cx, node);
        }
    }
}

pub fn parse(code: &str) {
    let mut sitter = Parser::new();
    sitter.set_language(&language()).unwrap();
    let mut cx = ContextPy {
        sitter,
        last_end_byte: 0,
        code,
        defs: IndexMap::new(),
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
    };
    let tree = cx.sitter.parse(code, None).unwrap();
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
