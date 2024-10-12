use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Query, QueryCursor};
use tree_sitter_python::language;

use crate::ast::ast_structs::{AstDefinition, AstUsage};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::parse_common::{ContextAnyParser, Thing, type_deindex_n, type_call, type_zerolevel_comma_split};


pub struct ContextPy<'a> {
    pub ap: ContextAnyParser<'a>,
    pub pass2: Vec<(Node<'a>, Vec<String>)>,
    pub ass1: Query,
    pub ass2: Query,
    pub ass3: Query,
    pub ass4: Query,
    pub class1: Query,
}

fn py_import_save<'a>(cx: &mut ContextPy<'a>, path: &Vec<String>, dotted_from: String, import_what: String, import_as: String)
{
    let save_as = format!("{}::{}", path.join("::"), import_as);
    let mut from_list = dotted_from.split(".").map(|x| { String::from(x.trim()) }).filter(|x| { !x.is_empty() }).collect::<Vec<String>>();
    from_list.push(import_what);
    cx.ap.alias.insert(save_as, from_list.join("::"));
}

fn py_import<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
{
    let mut dotted_from = String::new();
    let mut just_do_it = false;
    let mut from_clause = false;
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let child_text = cx.ap.code[child.byte_range()].to_string();
        match child.kind() {
            "import" => { just_do_it = true; },
            "from" => { from_clause = true; },
            "dotted_name" => {
                if just_do_it {
                    py_import_save(cx, path, dotted_from.clone(), child_text.clone(), child_text.clone());
                } else if from_clause {
                    dotted_from = child_text.clone();
                }
            },
            "aliased_import" => {
                let mut import_what = String::new();
                for i in 0..child.child_count() {
                    let subch = child.child(i).unwrap();
                    let subch_text = cx.ap.code[subch.byte_range()].to_string();
                    // dotted_name[identifier[os]]·as[as]·identifier[ooooos]]
                    match subch.kind() {
                        "dotted_name" => { import_what = subch_text; },
                        "as" => { },
                        "identifier" => { py_import_save(cx, path, dotted_from.clone(), import_what.clone(), subch_text); },
                        _ => {},
                    }
                }
            },
            "," => {},
            _ => {
                println!("\nIMPORT {:?} {:?}", child.kind(), child_text);
            }
        }
    }
}

fn py_is_trivial(potential_usage: &str) -> bool {
    match potential_usage {
        "int" | "float" | "str" | "bool" => true,
        _ => false,
    }
}

fn py_simple_resolve(cx: &mut ContextPy, path: &Vec<String>, look_for: &String) -> Option<String>
{
    match look_for.as_str() {
        "Any" => { return Some("*".to_string()); },
        "print" | "int" | "float" | "str" | "bool" => { return Some(look_for.clone()); },
        _ => {},
    }
    let mut current_path = path.clone();
    while !current_path.is_empty() {
        let mut hypothetical = current_path.clone();
        hypothetical.push(look_for.clone());
        let hypothtical_str = hypothetical.join("::");
        let thing_maybe = cx.ap.things.get(&hypothtical_str);
        if thing_maybe.is_some() {
            return Some(hypothtical_str);
        }
        if let Some(an_alias) = cx.ap.alias.get(&hypothtical_str) {
            return Some(an_alias.clone());
        }
        current_path.pop();
    }
    return None;
}

fn py_resolve_dotted_creating_usages(cx: &mut ContextPy, node: Node, path: &Vec<String>, allow_creation: bool) -> Option<AstUsage>
{
    let node_text = cx.ap.code[node.byte_range()].to_string();
    // identifier[Goat]
    // attribute[identifier[my_module].identifier[Animal]]
    match node.kind() {
        "identifier" => {
            if let Some(success) = py_simple_resolve(cx, path, &node_text) {
                let u = AstUsage {
                    targets_for_guesswork: vec![],
                    resolved_as: success.clone(),
                    debug_hint: format!("resolve/id"),
                    uline: node.range().start_point.row,
                };
                if !py_is_trivial(u.resolved_as.as_str()) && !cx.ap.suppress_adding {
                    cx.ap.usages.push((path.join("::"), u.clone()));
                }
                return Some(u);
            }
            if allow_creation {
                return Some(AstUsage {
                    targets_for_guesswork: vec![],
                    resolved_as: format!("{}::{}", path.join("::"), node_text),
                    debug_hint: format!("local_var_create"),
                    uline: node.range().start_point.row,
                });
            }
        },
        "attribute" => {
            let mut path_builder: Vec<String> = vec![];
            let mut found_prev1 = true;
            let mut found_prev2 = true;
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "." => { },
                    "identifier" => {
                        let ident_text = cx.ap.code[child.byte_range()].to_string();
                        if path_builder.is_empty() {  // first
                            if let Some(success) = py_simple_resolve(cx, path, &ident_text) {
                                path_builder = success.split("::").map(String::from).collect::<Vec<String>>();
                            } else {
                                path_builder = vec!["?".to_string(), ident_text.clone()];
                            }
                        } else { // next
                            path_builder.push(ident_text.clone());
                        }
                        // println!("DOTTED_LOOP {:?}", path_builder);
                        if path_builder.starts_with(&vec!["?".to_string()]) { // guesses
                            if !cx.ap.suppress_adding {
                                cx.ap.usages.push((path.join("::"), AstUsage {
                                    targets_for_guesswork: vec![path_builder.join("::")],
                                    resolved_as: "".to_string(),
                                    debug_hint: format!("dotted/guessing"),
                                    uline: node.range().start_point.row,
                                }));
                            }
                        } else if let Some(existing_thing) = cx.ap.things.get(&path_builder.join("::")) { // oh cool, real objects
                            if ident_text != "self"  && !cx.ap.suppress_adding {  // self references are trivial (we don't skip them completely, just reference on `self` itself is skipped)
                                cx.ap.usages.push((path.join("::"), AstUsage {
                                    targets_for_guesswork: vec![],
                                    resolved_as: path_builder.join("::"),
                                    debug_hint: format!("dotted"),
                                    uline: node.range().start_point.row,
                                }));
                            }
                            if existing_thing.thing_kind == 'v' || existing_thing.thing_kind == 'p' {
                                path_builder = existing_thing.type_resolved.split("::").map(|x| { String::from(x) }).collect::<Vec<String>>();
                            }
                        } else {
                            // not a guess, does not exist as a thing, probably usage of something from another module, such as os.system
                            if !allow_creation && !cx.ap.suppress_adding {
                                cx.ap.usages.push((path.join("::"), AstUsage {
                                    targets_for_guesswork: vec![],
                                    resolved_as: path_builder.join("::"),
                                    debug_hint: format!("othermod"),
                                    uline: node.range().start_point.row,
                                }));
                            }
                            found_prev2 = found_prev1;
                            found_prev1 = false;
                        }
                    },
                    _ => {},
                }
            }
            if allow_creation && !found_prev2 {
                return Some(AstUsage {
                    targets_for_guesswork: vec![],
                    resolved_as: path_builder.join("::"),
                    debug_hint: format!("dotted/create"),
                    uline: node.range().start_point.row,
                });
            }
            return Some(AstUsage {
                targets_for_guesswork: vec![path_builder.join("::")],
                resolved_as: "".to_string(),
                debug_hint: format!("ERR/RESOLVE/{}/{}", node.kind(), node_text),
                uline: node.range().start_point.row,
            });
        },
        _ => {}
    }
    None
}

fn py_assignment<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
{
    let mut lhs_tuple: Vec<(Node, Option<Node>)> = Vec::new();
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
            if lhs_lvalue.is_some() {
                lhs_tuple.push((lhs_lvalue.unwrap(), lhs_type));
            }
        }
        if !lhs_tuple.is_empty() {
            break;
        }
    }

    // save
    let right_node = node.child_by_field_name("right");
    let rhs_type = py_type_of_expr_creating_usages(cx, right_node, path);
    for n in 0 .. lhs_tuple.len() {
        let (lhs_lvalue, lvalue_type_node) = lhs_tuple[n];
        let lvalue_type = py_type_generic(cx, lvalue_type_node, path, 0);
        if is_list {
            py_var_add(cx, lhs_lvalue, lvalue_type, type_deindex_n(rhs_type.clone(), n), path);
        } else {
            py_var_add(cx, lhs_lvalue, lvalue_type, rhs_type.clone(), path);
        }
    }
}

fn resolved_type(type_str: &String) -> bool {
    type_str != "?" && !type_str.is_empty() && type_str != "!?"
}

fn py_var_add(cx: &mut ContextPy, lhs_lvalue: Node, lvalue_type: String, rhs_type: String, path: &Vec<String>)
{
    let lvalue_usage = if let Some(u) = py_resolve_dotted_creating_usages(cx, lhs_lvalue, path, true) {
        u
    } else {
        return; // syntax error or something
    };
    let lvalue_path;
    if lvalue_usage.targets_for_guesswork.is_empty() { // no guessing, exact location
        lvalue_path = lvalue_usage.resolved_as.clone();
    } else {
        // never mind can't create anything, for example a.b.c = 5 if b doesn't exit
        return;
    }
    let mut good_idea_to_write = true;
    let potential_new_type = if !resolved_type(&lvalue_type) || lvalue_type.starts_with("ERR") { rhs_type.clone() } else { lvalue_type.clone() };
    println!("\npy_var_add lvalue_path={} lvalue_type={} <= potential_new_type={} rhs_type={} good_idea_to_write={}", lvalue_path, lvalue_type, potential_new_type, rhs_type, good_idea_to_write);
    if let Some(existing_thing) = cx.ap.things.get(&lvalue_path) {
        good_idea_to_write = !resolved_type(&existing_thing.type_resolved) && resolved_type(&potential_new_type);
        if good_idea_to_write {
            cx.ap.resolved_anything = true;
        }
    }
    if good_idea_to_write {
        cx.ap.things.insert(lvalue_path, Thing {
            thing_kind: 'v',
            type_resolved: potential_new_type,
        });
    }
}

fn py_type_generic(cx: &mut ContextPy, node: Option<Node>, path: &Vec<String>, level: usize) -> String {
    if node.is_none() {
        return format!("?")
    }
    // type[generic_type[identifier[List]type_parameter[[type[identifier[Goat]]]]]]]
    // type[generic_type[identifier[List]type_parameter[[type[generic_type[identifier[Optional]type_parameter[[type[identifier[Goat]]]]]]]]
    let node = node.unwrap();
    match node.kind() {
        "type" => { py_type_generic(cx, node.child(0), path, level+1) },
        "identifier" | "attribute" => {
            if let Some(a_type) = py_resolve_dotted_creating_usages(cx, node, path, false) {
                if !a_type.resolved_as.is_empty() {
                    return a_type.resolved_as;
                } else if !a_type.targets_for_guesswork.is_empty() {
                    return a_type.targets_for_guesswork.first().unwrap().clone();
                }
            }
            format!("UNK/id/{}", cx.ap.code[node.byte_range()].to_string())
        },
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
                    ("type_parameter", _) => inside_type = py_type_generic(cx, Some(child), path, level+1),
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
                    "type" | "identifier" => py_type_generic(cx, Some(child), path, level+1),
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

fn py_type_of_expr_creating_usages(cx: &mut ContextPy, node: Option<Node>, path: &Vec<String>) -> String
{
    if node.is_none() {
        return "".to_string();
    }
    let node = node.unwrap();
    match node.kind() {
        "expression_list" | "argument_list" => {
            let mut elements = vec![];
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "(" | "," |")" => { continue; }
                    _ => {}
                }
                elements.push(py_type_of_expr_creating_usages(cx, Some(child), path));
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
                elements.push(py_type_of_expr_creating_usages(cx, Some(child), path));
            }
            format!("({})", elements.join(","))
        },
        "integer" => { "int".to_string() },
        "float" => { "float".to_string() },
        "string" => { "str".to_string() },
        "false" => { "bool".to_string() },
        "true" => { "bool".to_string() },
        "call" => {
            let fname = node.child_by_field_name("function");
            if fname.is_none() {
                return format!("ERR/CALL/NAMELESS")
            }
            let ftype = if let Some(u) = py_resolve_dotted_creating_usages(cx, fname.unwrap(), path, false) {
                if !u.resolved_as.is_empty() {
                    if let Some(resolved_thing) = cx.ap.things.get(&u.resolved_as) {
                        resolved_thing.type_resolved.clone()
                    } else {
                        format!("ERR/NOT_A_THING/{}", u.resolved_as.clone())
                    }
                } else {
                    "?".to_string()  // something outside of this file :/
                }
            } else {
                format!("ERR/FUNC_NOT_FOUND/{}", cx.ap.code[fname.unwrap().byte_range()].to_string())
            };
            let arg_types = py_type_of_expr_creating_usages(cx, node.child_by_field_name("arguments"), path);
            let ret_type = type_call(ftype.clone(), arg_types.clone());
            println!("\nCALL ftype={:?} arg_types={:?} => ret_type={:?}", ftype, arg_types, ret_type);
            ret_type
        },
        "identifier" | "dotted_name" | "attribute" => {
            let dotted_type = if let Some(u) = py_resolve_dotted_creating_usages(cx, node, path, false) {
                if let Some(resolved_thing) = cx.ap.things.get(&u.resolved_as) {
                    resolved_thing.type_resolved.clone()
                } else {
                    format!("ERR/NOT_A_THING/{}", u.resolved_as.clone())
                }
            } else {
                format!("ERR/DOTTED_NOT_FOUND/{}", cx.ap.code[node.byte_range()].to_string())
            };
            dotted_type
        },
        _ => {
            format!("ERR/EXPR/{:?}/{}", node.kind(), cx.ap.code[node.byte_range()].to_string())
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
        thing_kind: 's',
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

    let returns_type = py_type_generic(cx, returns, path, 0);

    cx.ap.things.insert(func_path.join("::"), Thing {
        thing_kind: 'f',
        type_resolved: format!("!{}", returns_type),
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
                type_resolved = py_type_generic(cx, param_node.child_by_field_name("type"), &func_path, 0);
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
            thing_kind: 'p',
            type_resolved,
        });
    }

    cx.pass2.push( (body.unwrap(), func_path.clone()) );
}


fn py_traverse<'a>(cx: &mut ContextPy<'a>, node: &Node<'a>, path: &Vec<String>)
{
    match node.kind() {
        "if" | "else" | ":" | "integer" | "float" | "string" | "false" | "true" => {
            cx.ap.just_print(node);
        },
        "module" | "block" | "if_statement" | "expression_statement" | "else_clause" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                py_traverse(cx, &child, path);
            }
        },
        "class_definition" => {
            py_class(cx, node, path);  // class recursively calls py_traverse
        },
        "function_definition" => {
            py_function(cx, node, path);  // function adds body to pass2, this calls py_traverse later
        },
        "assignment" => {
            py_assignment(cx, node, path);
        }
        "import_statement" | "import_from_statement" => { py_import(cx, node, path); }
        // "for_statement" => handle_variable(cx, node),
        "call" => {
            py_type_of_expr_creating_usages(cx, Some(node.clone()), path);
        }
        "return_statement" => {
            let ret_type = py_type_of_expr_creating_usages(cx, node.child(1), path);
            let func_path = path.join("::");
            if let Some(func_exists) = cx.ap.things.get(&func_path) {
                let good_idea_to_write = !resolved_type(&func_exists.type_resolved) && resolved_type(&ret_type) && func_exists.thing_kind == 'f';
                if good_idea_to_write {
                    println!("\nUPDATE RETURN TYPE {:?} for {}", ret_type, path.join("::"));
                    cx.ap.things.insert(func_path, Thing {
                        thing_kind: 'f',
                        type_resolved: ret_type,
                    });
                    cx.ap.resolved_anything = true;
                }
            }
        }
        _ => {
            // unknown, to discover new syntax, just print
            cx.ap.whitespace1(node);
            print!("\x1b[35m{}[\x1b[0m", node.kind());
            cx.ap.just_print(node);
            print!("\x1b[35m]\x1b[0m\n");
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
            suppress_adding: false,
            resolved_anything: false,
            defs: IndexMap::new(),
            things: IndexMap::new(),
            usages: vec![],
            alias: IndexMap::new(),
            star_imports: vec![],
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
pub fn parse(code: &str) -> String
{
    let mut cx = py_make_cx(code);
    let tree = cx.ap.sitter.parse(code, None).unwrap();
    let path = vec!["file".to_string()];
    py_traverse(&mut cx, &tree.root_node(), &path);
    while let Some((body, func_path)) = cx.pass2.pop() {
        py_traverse(&mut cx, &body, &func_path);
    }
    cx.ap.dump();
    cx.ap.annotate_code("#")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_py_goat() {
        let code = include_str!("alt_testsuite/py_torture.py");
        let annotated = parse(code);
        std::fs::write("src/ast/alt_testsuite/py_torture_annotated.py", annotated).expect("Unable to write file");
    }
}
