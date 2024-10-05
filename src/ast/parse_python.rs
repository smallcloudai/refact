use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Query, QueryCursor};
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

fn py_simple_resolve(cx: &mut ContextPy, node: Node, path: &Vec<String>, look_for: &String) -> Option<String>
{
    match look_for.as_str() {
        "Any" => { return Some("*".to_string()); },
        "int" | "float" | "str" | "bool" => { return Some(look_for.clone()); },
        _ => {},
    }
    let mut current_path = path.clone();
    while !current_path.is_empty() {
        let mut hypothetical = current_path.clone();
        hypothetical.push(look_for.clone());
        let thing_maybe = cx.ap.things.get(&hypothetical.join("::"));
        if thing_maybe.is_some() {
            return Some(hypothetical.join("::"));
        }
        current_path.pop();
    }
    return None;
}

fn py_resolve_type_creating_usages(cx: &mut ContextPy, node: Node, path: &Vec<String>) -> String
{
    let node_text = cx.ap.code[node.byte_range()].to_string();
    // cx.ap.recursive_print_with_red_brackets(&node);
    // identifier[Goat]
    // attribute[identifier[my_module].identifier[Animal]]
    match node.kind() {
        "identifier" => {
            if let Some(success) = py_simple_resolve(cx, node, path, &node_text) {
                // create usage
                return success;
            }
        },
        "attribute" => {
            let mut path_builder: Vec<String> = vec![];
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "." => { },
                    "identifier" => {
                        let ident_text = cx.ap.code[child.byte_range()].to_string();
                        if path_builder.is_empty() {  // first
                            if let Some(success) = py_simple_resolve(cx, node, path, &ident_text) {
                                path_builder = success.split("::").map(String::from).collect::<Vec<String>>();
                            } else {
                                return format!("ERR/NOTFOUND/{}", ident_text);
                            }
                        } else { // next
                            path_builder.push(ident_text);
                        }
                    },
                    _ => {},
                }
            }
            // create usage
            return path_builder.join("::");
        },
        _ => {}
    }
    return format!("ERR/RESOLVE/{}/{}", node.kind(), node_text);
}

// fn py_expression<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
// {
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

    let right_node = node.child_by_field_name("right");
    let rhs = py_type_of_expr(cx, right_node, path);

    // println!();
    for i in 0 .. lhs_tuple.len() {
        let (lhs_lvalue, lhs_explicit_type_node) = lhs_tuple[i];
        let lhs_explicit_type_str = py_type_explicit(cx, lhs_explicit_type_node, path, 0);
        println!("is_list={} LVALUE[{:?}] {} = {}", is_list, lhs_lvalue, lhs_explicit_type_str, rhs);
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
    match node.kind() {
        "type" => { py_type_explicit(cx, node.child(0), path, level+1) },
        "identifier" | "attribute" => { py_resolve_type_creating_usages(cx, node, path) },
        "list" => { format!("CALLABLE_ARGLIST") },
        "generic_type" => {
            let mut inside_type = String::new();
            let mut todo = "";
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                let child_text = cx.ap.code[child.byte_range()].to_string();
                // println!("{}GENERIC_LOOP {:?} {:?}", spaces, child.kind(), child_text);
                match (child.kind(), child_text.as_str()) {
                    // ("identifier", "Any") => todo = "give_up",
                    ("identifier", "List") => todo = "List",
                    ("identifier", "Set") => todo = "Set",
                    ("identifier", "Dict") => todo = "Dict",
                    ("identifier", "Tuple") => todo = "Tuple",
                    ("identifier", "Callable") => todo = "Callable",
                    ("identifier", "Optional") => todo = "Optional",
                    ("identifier", _) | ("attribute", _) => inside_type = format!("ERR/ID/{}", child_text),
                    // ("identifier", _) => { inside_type = py_resolve_type_creating_usages(cx, child, path); },
                    ("type_parameter", _) => inside_type = py_type_explicit(cx, Some(child), path, level+1),
                    (_, _) => inside_type = format!("ERR/GENERIC/{:?}", child.kind()),
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


fn py_type_of_expr(cx: &mut ContextPy, node: Option<Node>, path: &Vec<String>) -> String
{
    if node.is_none() {
        return "".to_string();
    }
    let node = node.unwrap();
    match node.kind() {
        "expression_list" => {
            let mut elements = vec![];
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "(" | "," |")" => { continue; }
                    _ => {}
                }
                elements.push(py_type_of_expr(cx, Some(child), path));
            }
            format!("({})", elements.join(","))
        },
        "tuple" => {
            let mut elements = vec![];
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "(" | "," |")" => { continue; }
                    _ => {}
                }
                elements.push(py_type_of_expr(cx, Some(child), path));
            }
            format!("({})", elements.join(","))
        },
        "integer" => { "int".to_string() },
        "float" => { "float".to_string() },
        // call
        "identifier" => {
            cx.ap.code[node.byte_range()].to_string()
        },
        _ => {
            format!("UNKNOWN[{:?}{}]", node.kind(), cx.ap.code[node.byte_range()].to_string())
        }
    }
}


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
    if params_node.is_none() {
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

    // All types in type annotations must be already visible in python
    let params = params_node.unwrap();
    for i in 0..params.child_count() {
        let param_node = params.child(i).unwrap();
        let mut param_name = "".to_string();
        let mut type_resolved = "".to_string();
        match param_node.kind() {
            "identifier" => {
                param_name = cx.ap.code[param_node.byte_range()].to_string();
                if param_name == "self" {
                    type_resolved = path.join("::");
                }
            },
            "typed_parameter" => {
                if let Some(param_name_node) = param_node.child(0) {
                    param_name = cx.ap.code[param_name_node.byte_range()].to_string();
                }
                type_resolved = py_type_explicit(cx, param_node.child_by_field_name("type"), &func_path, 0);
            },
            // "list_splat_pattern" for *args
            // "dictionary_splat_pattern" for **kwargs
            _ => {
                continue;
            }
        }
        if param_name.is_empty() {
            // XXX make error
            continue;
        }
        let param_path = [func_path.clone(), vec![param_name.clone()]].concat();
        cx.ap.things.insert(param_path.join("::"), Thing {
            assigned_rvalue: None,
            type_explicit: None,
            type_resolved,
        });
    }

    cx.pass2.push( (body.unwrap(), func_path.clone()) );
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
            // cx.ap.just_print(node);
            cx.ap.recursive_print_with_red_brackets(node);
            py_assignment(cx, node, path);
        }
        // "expression_statement" => {
        //     print!("\nexpression_statement\n");
        //     cx.ap.recursive_print_with_red_brackets(node);
        //     print!("\n/expression_statement\n");
            // x1.x2.x3
            //  -> usage of x1
            //  -> usage of x2
            //  -> usage of x3
            // return type(x3)
            // x4.f()
            //  -> usage of x4
            //  -> usage of f
            // return type of f()
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_py_goat() {
        let code = include_str!("alt_testsuite/py_torture.py");
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
}
