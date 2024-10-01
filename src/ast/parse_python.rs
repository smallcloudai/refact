use tree_sitter::{Node, Parser, Point, Range, Query, QueryCursor};
use tree_sitter_python::language;

pub struct ContextPy {
    pub sitter: Parser,
    pub last_end_byte: usize,
}

// fn handle_class(cx: &mut ContextPy, node: &Node, code: &str) {
//     let class_name = node.child_by_field_name("name").map(|n| code[n.byte_range()].to_string());
//     println!("Class: {:?}", class_name);
// }

// fn handle_argument(cx: &mut ContextPy, node: &Node, code: &str) {
//     for i in 0..node.child_count() {
//         let child = node.child(i).unwrap();
//         if child.kind() == "identifier" {
//             let argument_name = code[child.byte_range()].to_string();
//             println!("    Argument: {:?}", argument_name);
//         }
//     }
// }
// fn handle_variable(cx: &mut ContextPy, node: &Node, code: &str) {
//     let variable_name = node.child_by_field_name("name").map(|n| code[n.byte_range()].to_string());
//     println!("Variable: {:?}", variable_name);
// }

fn handle_function(cx: &mut ContextPy, node: &Node, code: &str) {
    let function_name = node.child_by_field_name("name").map(|n| code[n.byte_range()].to_string());
    print!("\x1b[34m[function={:?}]\x1b[0m", function_name);
    print!("{}", &code[node.byte_range()]);
}

fn whitespace1(cx: &mut ContextPy, node: &Node, code: &str) {
    if node.start_byte() > cx.last_end_byte {
        let whitespace = &code[cx.last_end_byte..node.start_byte()];
        print!("\x1b[32m{}\x1b[0m", whitespace.replace(" ", "·"));
        cx.last_end_byte = node.start_byte();
    }
}

fn whitespace2(cx: &mut ContextPy, node: &Node, code: &str) {
    cx.last_end_byte = node.end_byte();
}

fn just_print(cx: &mut ContextPy, node: &Node, code: &str) {
    whitespace1(cx, node, code);
    print!("{}", &code[node.byte_range()].replace(" ", "·"));
    whitespace2(cx, node, code);
}

fn py_traverse(cx: &mut ContextPy, node: &Node, code: &str) {
    match node.kind() {
        "module" | "block" => {
            // fall through, means loop children
        },
        "import_from_statement" => {},
        "class_definition" => {},
        "function_definition" => {
            whitespace1(cx, node, code);
            handle_function(cx, node, code);
            whitespace2(cx, node, code);
            return;
        }
        "from" | "class" | "identifier" | "import" | "dotted_name" | ":" | "," => {
            // simple keywords
            whitespace1(cx, node, code);
            just_print(cx, node, code);
            whitespace2(cx, node, code);
            return;
        },
        // "parameters" => handle_argument(cx, node, code),
        // "assignment" => handle_variable(cx, node, code),
        // "for_statement" => handle_variable(cx, node, code),
        _ => {
            whitespace1(cx, node, code);
            print!("\x1b[31m{}[\x1b[0m", node.kind());
            just_print(cx, node, code);
            print!("\x1b[31m]\x1b[0m");
            whitespace2(cx, node, code);
            return;
        }
    }

    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        py_traverse(cx, &child, code);
    }
}

// lambda
// decorated_definition
// as_pattern
// call
// for_statement
// comment | string
// ERROR

// if let Some(type_annotation) = get_type_annotation(node, code) {
//     println!("    Type annotation: {}", type_annotation);
// }
fn get_type_annotation(node: &Node, code: &str) -> Option<String> {
    if node.kind() == "type" {
        return Some(code[node.byte_range()].to_string());
    }
    None
}

pub fn parse(code: &str) {
    let mut sitter = Parser::new();
    sitter.set_language(&tree_sitter_python::language()).unwrap();
    let mut cx = ContextPy {
        sitter,
        last_end_byte: 0,
    };
    let tree = cx.sitter.parse(code, None).unwrap();
    println!("cc\n{:?}\ndd", tree);
    println!("hello world!\n\n");

    py_traverse(&mut cx, &tree.root_node(), code);

    println!("\n\nhello world 2\n\n");
    let query_source = "(function_definition name: (identifier) @function_name)";
    let query = Query::new(&tree_sitter_python::language(), query_source).expect("Error compiling query");
    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), code.as_bytes());
    for m in matches {
        for capture in m.captures {
            let node = capture.node;
            let function_name = node.utf8_text(code.as_bytes()).unwrap();
            println!("Found function: {}", function_name);
        }
    }
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
