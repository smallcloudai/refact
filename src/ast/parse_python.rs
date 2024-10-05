use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Query, QueryCursor};
// use tree_sitter::{Point, Range};
use tree_sitter_python::language;

use crate::ast::ast_structs::AstDefinition;
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::parse_common::{ContextAnyParser, Thing, type_deindex_n, type_zerolevel_comma_split};


pub struct ContextPy<'a> {
    pub ap: ContextAnyParser<'a>,
    pub pass2: Vec<(Node<'a>, Vec<String>)>,
    pub ass1: Query,
    pub ass2: Query,
    pub ass3: Query,
    pub ass4: Query,
    pub class1: Query,
}

// fn generate_usage(cx: &mut ContextPy, node: &Node, debug_note: &str, lhs_target: &str, rhs_target: &String) {
//     // print!("\x1b[34m<usage {} lhs_type={:?} rhs_type={:?}>\x1b[0m", debug_note, lhs_target, rhs_target);
// }

// fn py_type(cx: &ContextPy, node: &Node) -> Option<String> {
//     return Some(format!("TYPE[{}]", &cx.ap.code[node.byte_range()]));
// }

// fn py_lvalue(cx: &ContextPy, node: &Node) -> Option<String> {
//     return Some(format!("LVALUE[{}]", &cx.ap.code[node.byte_range()]));
// }

// fn py_rvalue(cx: &ContextPy, node: &Node) -> Option<String> {
//     return Some(format!("VAL[{}]", &cx.ap.code[node.byte_range()]));
// }

fn py_assignment<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
{
    let mut lhs_tuple: Vec<(Option<Node>, Option<Node>)> = Vec::new();
    let mut is_list = false;
    for query in [&cx.ass1, &cx.ass2, &cx.ass3, &cx.ass4] {
        let mut query_cursor = QueryCursor::new();
        for m in query_cursor.matches(&query, *node, cx.ap.code.as_bytes()) {
            let mut lhs_lvalue = None;
            let mut lhs_type = None;
            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                if capture_name == "lhs_inlist" {
                    lhs_lvalue = Some(capture.node);
                    is_list = true;
                } else if capture_name == "lhs" {
                    lhs_lvalue = Some(capture.node);
                } else if capture_name == "lhs_type" {
                    lhs_type = Some(capture.node);
                }
            }
            lhs_tuple.push((lhs_lvalue, lhs_type));
        }
        if !lhs_tuple.is_empty() {
            break;
        }
    }

    // let right_node = node.child_by_field_name("right");
    // let rhs_type = if let Some(x) = right_node { py_type_of_expr(cx, &x, path) } else { "".to_string() };

    println!();
    for i in 0 .. lhs_tuple.len() {
        let (lhs_lvalue, lhs_explicit_type_node) = lhs_tuple[i];
        let lhs_explicit_type_str = py_type_explicit(cx, lhs_explicit_type_node, path, 0);
        println!("is_list={} LVALUE[{:?}] {} = ?", is_list, lhs_lvalue, lhs_explicit_type_str);
        let var_path = [path.clone(), vec!["hui".to_string()]].concat();
        if is_list {
            cx.ap.things.insert(var_path.join("::"), Thing {
                assigned_rvalue: None,
                type_explicit: None,
                type_resolved: type_deindex_n(lhs_explicit_type_str, i)
            });
        } else {
            cx.ap.things.insert(var_path.join("::"), Thing {
                assigned_rvalue: None,
                type_explicit: None,
                type_resolved: lhs_explicit_type_str,
            });
        }
    }
}


fn py_type_explicit(cx: &mut ContextPy, node: Option<Node>, path: &Vec<String>, level: usize) -> String {
    if node.is_none() {
        return format!("NO_EXPLICIT_TYPE")
    }
    // type[generic_type[identifier[List]type_parameter[[type[identifier[Goat]]]]]]]
    // type[generic_type[identifier[List]type_parameter[[type[generic_type[identifier[Optional]type_parameter[[type[identifier[Goat]]]]]]]]
    let node = node.unwrap();
    // let node_text = cx.ap.code[node.byte_range()].to_string();
    // let spaces = "    ".repeat(level);
    // println!("{}TYPE_EXPLICIT {:?} {:?}", spaces, node.kind(), node_text);
    match node.kind() {
        "type" => {
            py_type_explicit(cx, node.child(0), path, level+1)
        }
        "identifier" | "attribute" => {
            let look_for = cx.ap.code[node.byte_range()].to_string();
            if look_for == "Any" {
                return "".to_string();
            }
            format!("ATTR/{}", cx.ap.code[node.byte_range()].to_string())
            // get_type_of_identifier_or_attribute(cx, &node)
        }
        "list" => {
            format!("CALLABLE_ARGLIST")
        }
        "generic_type" => {
            let mut inside_type = String::new();
            let mut todo = "";
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                let child_text = cx.ap.code[child.byte_range()].to_string();
                // println!("{}GENERIC_LOOP {:?} {:?}", spaces, child.kind(), child_text);
                match (child.kind(), child_text.as_str()) {
                    ("identifier", "Any") => todo = "give_up",
                    ("identifier", "List") => todo = "List",
                    ("identifier", "Set") => todo = "Set",
                    ("identifier", "Dict") => todo = "Dict",
                    ("identifier", "Tuple") => todo = "Tuple",
                    ("identifier", "Callable") => todo = "Callable",
                    ("identifier", "Optional") => todo = "Optional",
                    ("identifier", _) => inside_type = format!("ID/{}", child_text),
                    ("type_parameter", _) => inside_type = py_type_explicit(cx, Some(child), path, level+1),
                    (_, _) => inside_type = format!(" HMM/{:?}", child.kind()),
                }
            }
            let result = match todo {
                "give_up" => format!(""),
                "List" => format!("[{}]", inside_type),
                "Set" => format!("[{}]", inside_type),
                "Tuple" => format!("({})", inside_type),
                "Optional" => format!("{}", inside_type),
                "Callable" => {
                    if let Some(return_type_only) = inside_type.strip_prefix("CALLABLE_ARGLIST,") {
                        format!("!{}", return_type_only)
                    } else {
                        format!("!")
                    }
                },
                "Dict" => {
                    let split = type_zerolevel_comma_split(inside_type.as_str());
                    if split.len() == 2 {
                        format!("[{}]", split[1])
                    } else {
                        format!("BADDICT[{}]", inside_type)
                    }
                },
                _ => format!("NOTHING_TODO/{}", inside_type)
            };
            // println!("{}=> TODO {}", spaces, result);
            result
        }
        "type_parameter" => {
            // type_parameter[ "[" "type" "," "type" "]" ]
            let mut comma_sep_types = String::new();
            for i in 0 .. node.child_count() {
                let child = node.child(i).unwrap();
                // let child_text = cx.ap.code[child.byte_range()].to_string();
                // println!("{}TYPE_PARAMETER_LOOP {:?} {:?}", spaces, child.kind(), child_text);
                comma_sep_types.push_str(match child.kind() {
                    "[" | "]" => "".to_string(),
                    "type" | "identifier" => py_type_explicit(cx, Some(child), path, level+1),
                    "," => ",".to_string(),
                    _ => format!("SOMETHING/{:?}/{}", child.kind(), cx.ap.code[child.byte_range()].to_string())
                }.as_str());
            }
            comma_sep_types
        }
        _ => {
            format!("UNK/{:?}/{}", node.kind(), cx.ap.code[node.byte_range()].to_string())
        }
    }
}

// fn py_type_of_expr(cx: &mut ContextPy, node: &Node, path: &Vec<String>) -> String
// {
//     match node.kind() {
//         "expression_list" => {
//             let mut elements = vec![];
//             for i in 0..node.child_count() {
//                 let child = node.child(i).unwrap();
//                 elements.push(py_type_of_expr(cx, &child, path));
//             }
//             format!("[{}]", elements.join(","))
//         },
//         "tuple" => {
//             let mut elements = vec![];
//             for i in 0..node.child_count() {
//                 let child = node.child(i).unwrap();
//                 elements.push(py_type_of_expr(cx, &child, path));
//             }
//             format!("({})", elements.join(","))
//         },
//         "identifier" => {
//             cx.ap.code[node.byte_range()].to_string()
//         },
//         _ => {
//             format!("UNKNOWN[{}]", cx.ap.code[node.byte_range()].to_string())
//         }
//     }
// }

fn py_class<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
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

    let mut class_name = "".to_string();
    let mut body = None;
    let mut body_line1 = usize::MAX;
    let mut body_line2 = 0;
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "identifier" => class_name = cx.ap.code[child.byte_range()].to_string(),
            "block" => {
                body_line1 = body_line1.min(child.range().start_point.row + 1);
                body_line2 = body_line2.max(child.range().end_point.row + 1);
                body = Some(child);
                break;
            },
            _ => {}
        }
        cx.ap.just_print(&child);
    }

    if class_name == "" {
        return;
    }
    if body.is_none() {
        return;
    }

    let class_path = [path.clone(), vec![class_name.clone()]].concat();
    cx.ap.defs.insert(class_path.join("::"), AstDefinition {
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

    cx.ap.things.insert(class_path.join("::"), Thing {
        assigned_rvalue: None,
        type_explicit: None,
        type_resolved: format!("!{}", class_path.join("::")),   // this is about constructor in python, name of the class() is used as constructor, return type is the class
    });

    py_traverse(cx, &body.unwrap(), &class_path);
    // println!("\nCLASS {:?}", cx.ap.defs.get(&class_path.join("::")).unwrap());
}

fn py_function<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>) {
    // No need for 2-nd pass, all types in type annotations must be already visible
    // function_definition[def·identifier[jump_around]parameters[(identifier[self])]·->[->]·type[identifier[Animal]]
    // function_definition[def·identifier[jump_around]parameters[(typed_parameter[identifier[v1]:·type[identifier[Goat]]]
    let mut body_line1 = usize::MAX;
    let mut body_line2 = 0;
    let mut func_name = "".to_string();
    let mut params_node = None;
    let mut body = None;
    let mut returns = None;
    for i in 0 .. node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "identifier" => func_name = cx.ap.code[child.byte_range()].to_string(),
            "block" => {
                body_line1 = body_line1.min(child.range().start_point.row + 1);
                body_line2 = body_line2.max(child.range().end_point.row + 1);
                body = Some(child);
                break;
            },
            "parameters" => params_node = Some(child),
            "type" => returns = Some(child),
            "def" | "->" | ":" => {},
            _ => {
                println!("\nFUNCTION STRANGE NODE {:?}", child.kind());
            }
        }
    }
    if func_name == "" {
        // XXX make error
        return;
    }
    if body.is_none() {
        // XXX make error
        return;
    }

    let mut func_path = path.clone();
    func_path.push(func_name.clone());

    cx.ap.defs.insert(func_path.join("::"), AstDefinition {
        official_path: func_path.clone(),
        symbol_type: SymbolType::FunctionDeclaration,
        usages: vec![],
        this_is_a_class: "".to_string(),
        this_class_derived_from: vec![],
        cpath: "".to_string(),
        decl_line1: node.range().start_point.row + 1,
        decl_line2: (node.range().start_point.row + 1).max(body_line1 - 1),
        body_line1,
        body_line2,
    });

    let returns_type = py_type_explicit(cx, returns, path, 0);

    cx.ap.things.insert(func_path.join("::"), Thing {
        assigned_rvalue: None,
        type_explicit: None,
        type_resolved: returns_type,
    });

    println!("\nPARAMS");
    cx.ap.recursive_print_with_red_brackets(&params_node.unwrap());
    println!("\n/PARAMS");
    // println!("\nRETURNS");
    // println!("{:?}", returns);
    // println!("/RETURNS");
    cx.pass2.push( (body.unwrap(), func_path.clone()) );
    // println!("\nFUNCTION {:?}", cx.ap.defs.get(&func_path.join("::")).unwrap());
}


fn py_traverse<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
{
    match node.kind() {
        "from" | "class" | "identifier" | "import" | "dotted_name" | "def" | "if" | "for" | ":" | "," => {
            // simple keywords
            cx.ap.just_print(node);
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
        "function_definition" => {
            py_function(cx, node, path);
        },
        "import_from_statement" => {
            cx.ap.recursive_print_with_red_brackets(node);
            return;
        },
        "assignment" => {
            cx.ap.just_print(node);
            // cx.ap.recursive_print_with_red_brackets(node);
            py_assignment(cx, node, path);
        }
        // "expression_statement" => {
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

pub fn py_make_cx(code: &str) -> ContextPy
{
    let mut sitter = Parser::new();
    sitter.set_language(&language()).unwrap();
    let cx = ContextPy {
        ap: ContextAnyParser {
            sitter,
            last_end_byte: 0,
            code,
            defs: IndexMap::new(),
            things: IndexMap::new(),
        },
        pass2: vec![],
        // assignment[pattern_list[identifier[aaa1],·identifier[aaa2]]·=·expression_list[integer[13],·integer[14]]]
        ass1: Query::new(&language(), "(assignment left: (pattern_list (_) @lhs_inlist))").unwrap(),
        // assignment[tuple_pattern[([(]identifier[aaa2],·identifier[aaa3])[)]]·=·expression_list[integer[15],·integer[16]]]
        ass2: Query::new(&language(), "(assignment left: (tuple_pattern (_) @lhs_inlist))").unwrap(),
        // assignment[attribute[identifier[self].identifier[also1_age]]:·type[identifier[float]]·=·identifier[age]]
        ass3: Query::new(&language(), "(assignment left: (_) @lhs type: (_) @lhs_type)").unwrap(),
        // assignment[attribute[identifier[self].identifier[weight]] =·identifier[weight]]
        ass4: Query::new(&language(), "(assignment left: (_) @lhs)").unwrap(),

        // class_definition[class·identifier[Goat]argument_list[(identifier[Animal])]:
        class1: Query::new(&language(), "(class_definition name: (_) superclasses: (argument_list (_) @dfrom))").unwrap(),
        // function_definition[def·identifier[jump_around]parameters[(identifier[self])]·->[->]·type[identifier[Animal]]
        // function_definition[def·identifier[jump_around]parameters[(typed_parameter[identifier[v1]:·type[identifier[Goat]]]
    };
    cx
}

#[allow(dead_code)]
pub fn parse(code: &str)
{
    let mut cx = py_make_cx(code);
    let tree = cx.ap.sitter.parse(code, None).unwrap();
    let path = vec!["file".to_string()];
    py_traverse(&mut cx, &tree.root_node(), &path);

    println!("\n  -- things -- ");
    for (key, thing) in cx.ap.things.iter() {
        println!("{:<40} assigned_rvalue={:?}, type_explicit={:?}, type_resolved={:?}", key, thing.assigned_rvalue, thing.type_explicit, thing.type_resolved);
    }
    println!("  -- /things --\n");

    println!("\n  -- defs -- ");
    for (key, def) in cx.ap.defs.iter() {
        println!("{:<40} {:?}", key, def);
    }
    println!("\n  -- /defs -- ");
}

        // // expression_list[integer[13],·integer[14]]]
        // tuple1: Query::new(&language(), "(assignment right: (expression_list (_) @rhs))").unwrap(),
        // // tuple[(integer[15],·integer[16])]]
        // tuple2: Query::new(&language(), "(assignment right: (tuple (_) @rhs))").unwrap(),
        // // integer[12]]
        // tuple3: Query::new(&language(), "(assignment right: _ @rhs)").unwrap(),


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_py_goat() {
        let code = include_str!("alt_testsuite/py_goat_library.py");
        parse(code);
    }

    fn tree_any_node_of_type<'a>(node: Node<'a>, of_type: &str) -> Option<Node<'a>>
    {
        if node.kind() == of_type {
            return Some(node);
        }
        for i in 0..node.child_count() {
            if let Some(found) = tree_any_node_of_type(node.child(i).unwrap(), of_type) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn test_parse_py_explicit_types() {
        let examples = vec![
            ("x: Tuple[MyClass1, Optional[MyClass2]]", "EXPECTED_RESULT_1"),
            ("x: List[my_module.MyClass3]", "EXPECTED_RESULT_2"),
            ("x: Set[Tuple[int, float]]", "EXPECTED_RESULT_3"),
            ("x: Callable[[int, str], Tuple[my_module.MyClass4, int]]", "EXPECTED_RESULT_4"),
            ("x: Callable[[int, str], float]", "EXPECTED_RESULT_5"),
            ("x: Dict[str, my_module.MyClass4]", "EXPECTED_RESULT_6"),
            ("x: Dict[str, Any]", "EXPECTED_RESULT_7"),
        ];

        for (code, _expected) in examples {
            let mut cx = py_make_cx(code);
            let tree = cx.ap.sitter.parse(code, None).unwrap();
            let path = vec!["dummy".to_string()];
            let type_node = tree_any_node_of_type(tree.root_node(), "type");
            cx.ap.recursive_print_with_red_brackets(&type_node.unwrap());
            println!();
            let type_str = py_type_explicit(&mut cx, type_node, &path, 0);
            println!("{} => {}", code, type_str);
            println!();
            // assert_eq!(type_str, expected);
        }
    }
}
