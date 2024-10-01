use tree_sitter::{Node, Parser, Point, Range, Query, QueryCursor};

pub struct ContextPy<'a> {
    pub sitter: Parser,
    pub last_end_byte: usize,
    pub code: &'a str,
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

fn recursive_print(cx: &mut ContextPy, node: &Node) {
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
                recursive_print(cx, &child);
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
    print!("\x1b[34m<usage {} lhs_type={:?} rhs_type={:?}>\x1b[0m", debug_note, lhs_target, rhs_target);
}

fn py_typeof(cx: &mut ContextPy, node: &Node) -> Option<String> {
    return Some(format!("TYPE[{}]", &cx.code[node.byte_range()]));
}

fn py_lvalue(cx: &mut ContextPy, node: &Node) -> Option<String> {
    return Some(format!("LVALUE[{}]", &cx.code[node.byte_range()]));
}

fn handle_assignment(cx: &mut ContextPy, node: &Node) {
    // assignment[identifier[my_int1]·=·integer[10]]
    // assignment[identifier[my_int2]:·type[identifier[int]]·=·integer[11]]
    // assignment[identifier[my_int3]:·type[generic_type[identifier[Optional]type_parameter[[[[]type[identifier[int]]][]]]]]·=·integer[12]]
    // assignment[attribute[identifier[self].identifier[weight]]
    // assignment[attribute[identifier[self].identifier[also1_age]]:·type[identifier[float]]·=·identifier[age]]
    // assignment[pattern_list[identifier[aaa1],·identifier[aaa2]]·=·expression_list[integer[13],·integer[14]]]
    // assignment[tuple_pattern[([(]identifier[aaa2],·identifier[aaa3])[)]]·=·expression_list[integer[15],·integer[16]]]
    // assignment[pattern_list[identifier[aaa5],·tuple_pattern[([(]identifier[aaa6],·identifier[aaa7])[)]]]·=·expression_list[integer[17],·tuple[([(]integer[18],·integer[19])[)]]]]

    let ass1 = r#"(assignment left: (pattern_list (_) @lhs))"#;
    let ass2 = r#"(assignment left: (tuple_pattern (_) @lhs))"#;
    let ass3 = r#"(assignment left: (identifier) @lhs type: (_) @type)"#;
    let ass4 = r#"(assignment left: (identifier) @lhs)"#;
    let ass5 = r#"(assignment left: (attribute) @lhs type: (_) @type)"#;
    let ass6 = r#"(assignment left: (attribute) @lhs)"#;

    let mut lhs_tuple: Vec<(String, Option<String>)> = Vec::new();
    let queries = [ass1, ass2, ass3, ass4, ass5, ass6];
    for query_str in &queries {
        let query = Query::new(&tree_sitter_python::language(), query_str).unwrap();
        let mut query_cursor = QueryCursor::new();
        let matches = query_cursor.matches(&query, *node, cx.code.as_bytes());
        for m in matches {
            let mut lhs_text = None;
            let mut lhs_type = None;
            for capture in m.captures {
                let captured_node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];
                if capture_name == "lhs" {
                    lhs_text = py_lvalue(cx, &captured_node);
                } else if capture_name == "type" {
                    lhs_type = py_typeof(cx, &captured_node);
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

    println!();
    for (lhs_text, lhs_type) in &lhs_tuple {
        println!("{}: {:?} = VAL", lhs_text, lhs_type);
    }

    // // Generate usage information
    // if lhs_tuple.len() == rhs_tuple.len() {
    //     for ((lhs_text, lhs_type), (rhs_text, rhs_type)) in lhs_tuple.iter().zip(rhs_tuple.iter()) {
    //         generate_usage(cx, node, "assignment", lhs_text, rhs_type);
    //     }
    // } else {
    //     print!("mismatched lhs and rhs lengths");
    // }
}

fn typeof_expression(cx: &mut ContextPy, node_expr: &Node) -> String {
    whitespace1(cx, node_expr);
    print!("\x1b[31mexpression[\x1b[0m");
    just_print(cx, node_expr);
    print!("\x1b[31m]\x1b[0m");
    whitespace2(cx, node_expr);
    return "hello_type".to_string();
}

fn py_traverse(cx: &mut ContextPy, node: &Node) {
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
                py_traverse(cx, &child);
            }
        },
        // TODO
        "import_from_statement" | "class_definition" | "function_definition" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                py_traverse(cx, &child);
            }
        },
        "assignment" => {
            recursive_print(cx, node);
            // whitespace1(cx, node);
            // print!("\x1b[31massignment[\x1b[0m");
            handle_assignment(cx, node);
            // print!("\x1b[31m]\x1b[0m");
            // whitespace2(cx, node);
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
    sitter.set_language(&tree_sitter_python::language()).unwrap();
    let mut cx = ContextPy {
        sitter,
        last_end_byte: 0,
        code,
    };
    let tree = cx.sitter.parse(code, None).unwrap();
    py_traverse(&mut cx, &tree.root_node());
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
