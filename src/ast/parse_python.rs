use indexmap::IndexMap;
use tree_sitter::{Node, Parser};
use tree_sitter_python::language;

use crate::ast::ast_structs::{AstDefinition, AstUsage, AstErrorStats};
use crate::ast::treesitter::structs::SymbolType;
use crate::ast::parse_common::{ContextAnyParser, Thing, any_child_of_type, type_deindex, type_deindex_n, type_call, type_zerolevel_comma_split};

const DEBUG: bool = false;

// more python todo:
// - comments
// - type aliases
// - star imports


pub struct ContextPy {
    pub ap: ContextAnyParser,
}

fn debug_helper(cx: &ContextPy, args: std::fmt::Arguments) {
    cx.ap.indented_println(args);
}

macro_rules! debug {
    ($cx:expr, $($arg:tt)*) => {
        if DEBUG {
            debug_helper($cx, format_args!($($arg)*));
        }
    }
}

fn type_problems(type_str: &String) -> usize {
    let empty_very_bad = 1000000 * (type_str == "?" || type_str.is_empty()) as usize;
    let question_marks = type_str.matches('?').count() * 1000;
    let errors = type_str.matches("ERR").count();
    question_marks + errors + empty_very_bad
}

fn py_trivial(potential_usage: &str) -> Option<String> {
    match potential_usage {
        "?::int" | "int" => Some("int".to_string()),
        "?::float" | "float" => Some("float".to_string()),
        "?::bool" | "bool" => Some("bool".to_string()),
        "?::str" | "str" => Some("str".to_string()),
        "Any" => { Some("*".to_string()) },
        "__name__" => { Some("str".to_string()) },
        "range" => { Some("![int]".to_string()) },
        // "print" => { Some("!void".to_string()) },
        _ => None,
    }
}

fn py_simple_resolve(cx: &mut ContextPy, path: &Vec<String>, look_for: &String, uline: usize) -> AstUsage
{
    if let Some(t) = py_trivial(look_for) {
        return AstUsage {
            resolved_as: t,
            targets_for_guesswork: vec![],
            debug_hint: format!("trivial"),
            uline,
        };
    }
    let mut current_path = path.clone();
    while !current_path.is_empty() {
        let mut hypothetical = current_path.clone();
        hypothetical.push(look_for.clone());
        let hypothtical_str = hypothetical.join("::");
        let thing_maybe = cx.ap.things.get(&hypothtical_str);
        if thing_maybe.is_some() {
            return AstUsage {
                resolved_as: hypothtical_str,
                targets_for_guesswork: vec![],
                debug_hint: format!("go_up"),
                uline,
            };
        }
        if let Some(an_alias) = cx.ap.alias.get(&hypothtical_str) {
            return AstUsage {
                resolved_as: an_alias.clone(),
                targets_for_guesswork: vec![],
                debug_hint: format!("alias"),
                uline,
            };
        }
        current_path.pop();
    }
    return AstUsage {
        resolved_as: "".to_string(),
        targets_for_guesswork: vec![format!("?::{}", look_for)],
        debug_hint: format!("go_up_fail"),
        uline,
    };
}

fn py_add_a_thing<'a>(cx: &mut ContextPy, thing_path: &String, thing_kind: char, type_new: String, node: &Node<'a>) -> (bool, String)
{
    if let Some(thing_exists) = cx.ap.things.get(thing_path) {
        if thing_exists.thing_kind != thing_kind {
            let msg = cx.ap.error_report(node, format!("py_add_a_thing both {:?} and {:?} exist", thing_exists.thing_kind, thing_kind));
            debug!(cx, "{}", msg);
            return (false, type_new.clone());
        }
        let good_idea_to_write = type_problems(&thing_exists.type_resolved) > type_problems(&type_new);
        if good_idea_to_write {
            debug!(cx, "TYPE UPDATE {thing_kind} {thing_path} TYPE {} problems={:?} => {} problems={:?}", thing_exists.type_resolved, type_problems(&thing_exists.type_resolved), type_new, type_problems(&type_new));
            cx.ap.resolved_anything = true;
        } else {
            return (false, thing_exists.type_resolved.clone());
        }
    } else {
        debug!(cx, "ADD {thing_kind} {thing_path} {}", type_new);
    }
    cx.ap.things.insert(thing_path.clone(), Thing {
        tline: node.range().start_point.row,
        public: py_is_public(cx, thing_path),
        thing_kind,
        type_resolved: type_new.clone(),
    });
    return (true, type_new);
}

fn py_is_public(cx: &ContextPy, path_str: &String) -> bool {
    let path: Vec<String> = path_str.split("::").map(String::from).collect();
    // if let Some(name) = path.last() {
    //     if name.starts_with('_') {
    //         return false;
    //     }
    // }
    for i in 1 .. path.len() {
        let parent_path = path[0 .. i].join("::");
        if let Some(parent_thing) = cx.ap.things.get(&parent_path) {
            match parent_thing.thing_kind {
                's' => { return parent_thing.public; },
                'f' => { return false; },
                _ => { },
            }
        }
    }
    true
}

fn py_import_save<'a>(cx: &mut ContextPy, path: &Vec<String>, dotted_from: String, import_what: String, import_as: String)
{
    let save_as = format!("{}::{}", path.join("::"), import_as);
    let mut p = dotted_from.split(".").map(|x| { String::from(x.trim()) }).filter(|x| { !x.is_empty() }).collect::<Vec<String>>();
    p.push(import_what);
    p.insert(0, "?".to_string());
    cx.ap.alias.insert(save_as, p.join("::"));
}

fn py_import<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>)
{
    let mut dotted_from = String::new();
    let mut just_do_it = false;
    let mut from_clause = false;
    for i in 0 .. node.child_count() {
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
                    match subch.kind() {
                        "dotted_name" => { import_what = subch_text; },
                        "as" => { },
                        "identifier" => { py_import_save(cx, path, dotted_from.clone(), import_what.clone(), subch_text); },
                        _ => {
                            let msg = cx.ap.error_report(&child, format!("aliased_import syntax"));
                            debug!(cx, "{}", msg);
                        },
                    }
                }
            },
            "," => {},
            _ => {
                let msg = cx.ap.error_report(&child, format!("import syntax"));
                debug!(cx, "{}", msg);
            }
        }
    }
}

fn py_resolve_dotted_creating_usages<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>, allow_creation: bool) -> Option<AstUsage>
{
    let node_text = cx.ap.code[node.byte_range()].to_string();
    // debug!(cx, "DOTTED {}", cx.ap.recursive_print_with_red_brackets(&node));
    match node.kind() {
        "identifier" => {
            let u = py_simple_resolve(cx, path, &node_text, node.range().start_point.row);
            // debug!(cx, "DOTTED GO_UP {:?}", u);
            if u.resolved_as.is_empty() && allow_creation {
                return Some(AstUsage {
                    targets_for_guesswork: vec![],
                    resolved_as: format!("{}::{}", path.join("::"), node_text),
                    debug_hint: format!("local_var_create"),
                    uline: node.range().start_point.row,
                });
            }
            if !u.resolved_as.ends_with("::self") && !u.debug_hint.ends_with("trivial") {
                cx.ap.usages.push((path.join("::"), u.clone()));
            }
            return Some(u);
        },
        "attribute" => {
            let object = node.child_by_field_name("object").unwrap();
            let attrib = node.child_by_field_name("attribute").unwrap();
            let object_type = py_type_of_expr_creating_usages(cx, Some(object), path);
            let attrib_text = cx.ap.code[attrib.byte_range()].to_string();
            let attrib_path = format!("{}::{}", object_type, attrib_text);
            let mut u = AstUsage {
                targets_for_guesswork: vec![],
                resolved_as: attrib_path.clone(),
                debug_hint: format!("attr"),
                uline: attrib.range().start_point.row,
            };
            // debug!(cx, "DOTTED_ATTR {:?}", u);
            if let Some(_existing_attr) = cx.ap.things.get(&attrib_path) {
                cx.ap.usages.push((path.join("::"), u.clone()));
                return Some(u);
            }
            if let Some(_existing_object) = cx.ap.things.get(&object_type) {
                if allow_creation {
                    u.debug_hint = format!("attr_create");
                    return Some(u);
                }
            }
            u.resolved_as = "".to_string();
            u.targets_for_guesswork.push(format!("?::{}", attrib_text));
            cx.ap.usages.push((path.join("::"), u.clone()));
            return Some(u);
        },
        _ => {
            let msg = cx.ap.error_report(node, format!("py_resolve_dotted_creating_usages syntax"));
            debug!(cx, "{}", msg);
        }
    }
    None
}

fn py_lhs_tuple<'a>(cx: &mut ContextPy, left: &Node<'a>, type_node: Option<Node<'a>>, path: &Vec<String>) -> (Vec<(Node<'a>, String)>, bool)
{
    let mut lhs_tuple: Vec<(Node, String)> = Vec::new();
    let mut is_list = false;
    match left.kind() {
        "pattern_list" | "tuple_pattern" => {
            is_list = true;
            for j in 0 .. left.child_count() {
                let child = left.child(j).unwrap();
                match child.kind() {
                    "identifier" | "attribute" => {
                        lhs_tuple.push((child, "?".to_string()));
                    },
                    "," | "(" | ")" => { },
                    _ => {
                        let msg = cx.ap.error_report(&child, format!("py_lhs_tuple list syntax"));
                        debug!(cx, "{}", msg);
                    }
                }
            }
        },
        "identifier" | "attribute" => {
            lhs_tuple.push((*left, py_type_generic(cx, type_node, path, 0)));
        },
        _ => {
            let msg = cx.ap.error_report(left, format!("py_lhs_tuple syntax"));
            debug!(cx, "{}", msg);
        },
    }
    (lhs_tuple, is_list)
}

fn py_assignment<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>, is_for_loop: bool)
{
    let left_node = node.child_by_field_name("left");
    let right_node = node.child_by_field_name("right");
    let mut rhs_type = py_type_of_expr_creating_usages(cx, right_node, path);
    if is_for_loop {
        rhs_type = type_deindex(rhs_type);
    }
    if left_node.is_none() {
        return;
    }
    let (lhs_tuple, is_list) = py_lhs_tuple(cx, &left_node.unwrap(), node.child_by_field_name("type"), path);
    for n in 0 .. lhs_tuple.len() {
        let (lhs_lvalue, lvalue_type) = &lhs_tuple[n];
        if is_list {
            py_var_add(cx, lhs_lvalue, lvalue_type.clone(), type_deindex_n(rhs_type.clone(), n), path);
        } else {
            py_var_add(cx, lhs_lvalue, lvalue_type.clone(), rhs_type.clone(), path);
        }
    }
}

fn py_var_add<'a>(cx: &mut ContextPy, lhs_lvalue: &Node<'a>, lvalue_type: String, rhs_type: String, path: &Vec<String>)
{
    let lvalue_usage = if let Some(u) = py_resolve_dotted_creating_usages(cx, lhs_lvalue, path, true) {
        u
    } else {
        let msg = cx.ap.error_report(lhs_lvalue, format!("py_var_add cannot form lvalue"));
        debug!(cx, "{}", msg);
        return;
    };
    let lvalue_path;
    if lvalue_usage.targets_for_guesswork.is_empty() { // no guessing, exact location
        lvalue_path = lvalue_usage.resolved_as.clone();
    } else {
        // typical for creating things in a different file, or for example a.b.c = 5 when b doesn't exit
        let msg = cx.ap.error_report(lhs_lvalue, format!("py_var_add cannot create"));
        debug!(cx, "{}", msg);
        return;
    }
    let potential_new_type = if type_problems(&lvalue_type) > type_problems(&rhs_type) { rhs_type.clone() } else { lvalue_type.clone() };
    let (upd, best_return_type) = py_add_a_thing(cx, &lvalue_path, 'v', potential_new_type, lhs_lvalue);
    // let (upd2, best_return_type) = py_add_a_thing(cx, &func_path_str, 'f', format!("!{}", ret_type), node);
    if upd {
        let path: Vec<String> = lvalue_path.split("::").map(String::from).collect();
        cx.ap.defs.insert(lvalue_path.clone(), AstDefinition {
            official_path: path,
            symbol_type: SymbolType::VariableDefinition,
            usages: vec![],
            resolved_type: best_return_type,
            this_is_a_class: "".to_string(),
            this_class_derived_from: vec![],
            cpath: "".to_string(),
            decl_line1: lhs_lvalue.range().start_point.row + 1,
            decl_line2: lhs_lvalue.range().end_point.row + 1,
            body_line1: 0,
            body_line2: 0,
        });
    }
}

fn py_type_generic<'a>(cx: &mut ContextPy, node: Option<Node<'a>>, path: &Vec<String>, level: usize) -> String {
    if node.is_none() {
        return format!("?")
    }
    // type[generic_type[identifier[List]type_parameter[[type[identifier[Goat]]]]]]]
    // type[generic_type[identifier[List]type_parameter[[type[generic_type[identifier[Optional]type_parameter[[type[identifier[Goat]]]]]]]]
    let node = node.unwrap();
    match node.kind() {
        "type" => { py_type_generic(cx, node.child(0), path, level+1) },
        "identifier" | "attribute" => {
            if let Some(a_type) = py_resolve_dotted_creating_usages(cx, &node, path, false) {
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
                // debug!(cx, "{}GENERIC_LOOP {:?} {:?}", spaces, child.kind(), child_text);
                match (child.kind(), child_text.as_str()) {
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
            // debug!(cx, "{}=> TODO {}", spaces, result);
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
            let msg = cx.ap.error_report(&node, format!("py_type_generic syntax"));
            debug!(cx, "{}", msg);
            format!("UNK/{:?}/{}", node.kind(), cx.ap.code[node.byte_range()].to_string())
        }
    }
}

fn py_string<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>) -> String
{
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        // debug!(cx, "  string child[{}] {}", i, cx.ap.recursive_print_with_red_brackets(&child));
        match child.kind() {
            "interpolation" => {
                let _ = py_type_of_expr_creating_usages(cx, child.child_by_field_name("expression"), path);
            },
            _ => { },
        }
    }
    "str".to_string()
}

fn py_type_of_expr_creating_usages<'a>(cx: &mut ContextPy, node: Option<Node<'a>>, path: &Vec<String>) -> String
{
    if node.is_none() {
        return "".to_string();
    }
    let node = node.unwrap();
    let node_text = cx.ap.code[node.byte_range()].to_string();
    debug!(cx, "EXPR {}", node_text);
    cx.ap.reclevel += 1;
    let type_of = match node.kind() {
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
        "comparison_operator" => {
            for i in 0 .. node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "is" | "is not" | ">" | "<" | "<=" | "==" | "!=" | ">=" | "%" => { continue; }
                    _ => {}
                }
                py_type_of_expr_creating_usages(cx, Some(child), path);
            }
            "bool".to_string()
        },
        "binary_operator" => {
            let left_type = py_type_of_expr_creating_usages(cx, node.child_by_field_name("left"), path);
            let _right_type = py_type_of_expr_creating_usages(cx, node.child_by_field_name("right"), path);
            let _op =  cx.ap.code[node.child_by_field_name("operator").unwrap().byte_range()].to_string();
            left_type
        },
        "integer" => { "int".to_string() },
        "float" => { "float".to_string() },
        "string" => { py_string(cx, &node, path) },
        "false" => { "bool".to_string() },
        "true" => { "bool".to_string() },
        "none" => { "void".to_string() },
        "call" => {
            let fname = node.child_by_field_name("function").unwrap();
            let ftype = py_type_of_expr_creating_usages(cx, Some(fname), path);
            let arg_types = py_type_of_expr_creating_usages(cx, node.child_by_field_name("arguments"), path);
            let ret_type = type_call(ftype.clone(), arg_types.clone());
            ret_type
        },
        "identifier" | "dotted_name" | "attribute" => {
            let dotted_type = if let Some(u) = py_resolve_dotted_creating_usages(cx, &node, path, false) {
                if u.resolved_as.starts_with("!") {  // trivial function, like "range" that has type ![int]
                    u.resolved_as
                } else if !u.resolved_as.is_empty() {
                    if let Some(resolved_thing) = cx.ap.things.get(&u.resolved_as) {
                        resolved_thing.type_resolved.clone()
                    } else {
                        format!("?::{}", u.resolved_as)
                    }
                } else {
                    // assert!(u.targets_for_guesswork.len() > 0);
                    // u.targets_for_guesswork[0].clone()
                    format!("ERR/FUNC_NOT_FOUND/{}", u.targets_for_guesswork[0])
                }
            } else {
                format!("ERR/DOTTED_NOT_FOUND/{}", node_text)
            };
            dotted_type
        },
        "subscript" => {
            let typeof_value = py_type_of_expr_creating_usages(cx, node.child_by_field_name("value"), path);
            py_type_of_expr_creating_usages(cx, node.child_by_field_name("subscript"), path);
            type_deindex(typeof_value)
        },
        "list_comprehension" => {
            let mut path_anon = path.clone();
            path_anon.push("<listcomp>".to_string());
            if let Some(for_clause) = any_child_of_type(node, "for_in_clause") {
                py_assignment(cx, &for_clause, &path_anon, true);
                // XXX two let Some combined?
                let body = node.child_by_field_name("body");
                let body_type = py_type_of_expr_creating_usages(cx, body, &path_anon);
                format!("[{}]", body_type)
            } else {
                format!("ERR/EXPR/list_comprehension/no_for")
            }
        },
        "keyword_argument" => { format!("void") },
        _ => {
            let msg = cx.ap.error_report(&node, format!("py_type_of_expr syntax"));
            debug!(cx, "{}", msg);
            format!("ERR/EXPR/{:?}/{}", node.kind(), node_text)
        }
    };
    cx.ap.reclevel -= 1;
    debug!(cx, "/EXPR type={}", type_of);
    type_of
}

fn py_class<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>)
{
    let mut derived_from = vec![];
    let mut class_name = "".to_string();
    let mut body = None;
    let mut body_line1 = usize::MAX;
    let mut body_line2 = 0;
    for i in 0 .. node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "class" | ":" => continue,
            "identifier" => class_name = cx.ap.code[child.byte_range()].to_string(),
            "block" => {
                body_line1 = body_line1.min(child.range().start_point.row + 1);
                body_line2 = body_line2.max(child.range().end_point.row + 1);
                body = Some(child);
                break;
            },
            "argument_list" => {
                for j in 0 .. child.child_count() {
                    let arg = child.child(j).unwrap();
                    match arg.kind() {
                        "identifier" | "attribute" => {
                            if let Some(a_type) = py_resolve_dotted_creating_usages(cx, &arg, path, false) {
                                if !a_type.resolved_as.is_empty() {
                                    // XXX losing information, we have resolved usage, turning it into approx 🔎-link
                                    let after_last_colon_colon = a_type.resolved_as.split("::").last().unwrap().to_string();
                                    derived_from.push(format!("py🔎{}", after_last_colon_colon));
                                } else {
                                    // could be better than a guess, too
                                    assert!(!a_type.targets_for_guesswork.is_empty());
                                    let after_last_colon_colon = a_type.targets_for_guesswork.first().unwrap().split("::").last().unwrap().to_string();
                                    derived_from.push(format!("py🔎{}", after_last_colon_colon));
                                }
                            }
                        },
                        "," | "(" | ")" => continue,
                        _ => {
                            let msg = cx.ap.error_report(&arg, format!("py_class dfrom syntax"));
                            debug!(cx, "{}", msg);
                        }
                    }
                }
            },
            _ => {
                let msg = cx.ap.error_report(&child, format!("py_class syntax"));
                debug!(cx, "{}", msg);
            }
        }
    }

    if class_name == "" {
        let msg = cx.ap.error_report(node, format!("py_class nameless class"));
        debug!(cx, "{}", msg);
        return;
    }
    if body.is_none() {
        let msg = cx.ap.error_report(node, format!("py_class bodyless class"));
        debug!(cx, "{}", msg);
        return;
    }

    let class_path = [path.clone(), vec![class_name.clone()]].concat();
    let class_path_str = class_path.join("::");
    cx.ap.defs.insert(class_path_str.clone(), AstDefinition {
        official_path: class_path.clone(),
        symbol_type: SymbolType::StructDeclaration,
        usages: vec![],
        resolved_type: format!("!{}", class_path.join("::")),
        this_is_a_class: format!("py🔎{}", class_name),
        this_class_derived_from: derived_from,
        cpath: "".to_string(),
        decl_line1: node.range().start_point.row + 1,
        decl_line2: (node.range().start_point.row + 1).max(body_line1 - 1),
        body_line1,
        body_line2,
    });

    cx.ap.things.insert(class_path_str.clone(), Thing {
        tline: node.range().start_point.row,
        public: py_is_public(cx, &class_path_str),
        thing_kind: 's',
        type_resolved: format!("!{}", class_path_str),   // this is about constructor in python, name of the class() is used as constructor, return type is the class
    });

    py_body(cx, &body.unwrap(), &class_path);
    // debug!(cx, "\nCLASS {:?}", cx.ap.defs.get(&class_path.join("::")).unwrap());
}


fn py_function<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>) {
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
                let msg = cx.ap.error_report(&child, format!("py_function syntax"));
                debug!(cx, "{}", msg);
            }
        }
    }
    if func_name == "" {
        let msg = cx.ap.error_report(node, format!("py_function no name"));
        debug!(cx, "{}", msg);
        return;
    }
    if body.is_none() {
        let msg = cx.ap.error_report(node, format!("py_function no body"));
        debug!(cx, "{}", msg);
        return;
    }
    if params_node.is_none() {
        let msg = cx.ap.error_report(node, format!("py_function no params"));
        debug!(cx, "{}", msg);
        return;
    }

    let mut func_path = path.clone();
    func_path.push(func_name.clone());
    let func_path_str = func_path.join("::");

    let returns_type = py_type_generic(cx, returns, path, 0);

    let upd1;
    (upd1, _) = py_add_a_thing(cx, &func_path_str, 'f', returns_type, node);

    // All types in param type annotations must be already visible in python
    let params = params_node.unwrap();
    for i in 0..params.child_count() {
        let param_node = params.child(i).unwrap();
        let mut param_name = "".to_string();
        let mut type_resolved = "?".to_string();
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
            "," | "(" | ")" => continue,
            // "list_splat_pattern" for *args
            // "dictionary_splat_pattern" for **kwargs
            _ => {
                let msg = cx.ap.error_report(&param_node, format!("py_function parameter syntax"));
                debug!(cx, "{}", msg);
                continue;
            }
        }
        if param_name.is_empty() {
            let msg = cx.ap.error_report(&param_node, format!("py_function nameless param"));
            debug!(cx, "{}", msg);
            continue;
        }
        let param_path = [func_path.clone(), vec![param_name.clone()]].concat();
        cx.ap.things.insert(param_path.join("::"), Thing {
            tline: param_node.range().start_point.row,
            public: false,
            thing_kind: 'p',
            type_resolved,
        });
    }

    let ret_type = py_body(cx, &body.unwrap(), &func_path);
    let (upd2, best_return_type) = py_add_a_thing(cx, &func_path_str, 'f', format!("!{}", ret_type), node);
    if upd1 || upd2 {
        cx.ap.defs.insert(func_path_str, AstDefinition {
            official_path: func_path.clone(),
            symbol_type: SymbolType::FunctionDeclaration,
            usages: vec![],
            resolved_type: best_return_type,
            this_is_a_class: "".to_string(),
            this_class_derived_from: vec![],
            cpath: "".to_string(),
            decl_line1: node.range().start_point.row + 1,
            decl_line2: (node.range().start_point.row + 1).max(body_line1 - 1),
            body_line1,
            body_line2,
        });
    }
}

fn py_body<'a>(cx: &mut ContextPy, node: &Node<'a>, path: &Vec<String>) -> String
{
    let mut ret_type = "void".to_string();  // if there's no return clause, then it's None aka void
    debug!(cx, "{}", node.kind());
    cx.ap.reclevel += 1;
    match node.kind() {
        "import_statement" | "import_from_statement" => py_import(cx, node, path),
        "if" | "else" | "elif" => { },
        "module" | "block" | "expression_statement" | "else_clause" | "if_statement" | "elif_clause" => {
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                match child.kind() {
                    "if" | "elif" | "else" | ":" | "integer" | "float" | "string" | "false" | "true" => { continue; }
                    "return_statement" => { ret_type = py_type_of_expr_creating_usages(cx, child.child(1), path); }
                    _ => { let _ = py_body(cx, &child, path); }
                }
            }
        },
        "class_definition" => py_class(cx, node, path),  // calls py_body recursively
        "function_definition" => py_function(cx, node, path),  // calls py_body recursively
        "assignment" => py_assignment(cx, node, path, false),
        "for_statement" => py_assignment(cx, node, path, true),
        "call" | "comparison_operator" => { py_type_of_expr_creating_usages(cx, Some(node.clone()), path); }
        _ => {
            let msg = cx.ap.error_report(node, format!("py_body no body"));
            debug!(cx, "{}", msg);
        }
    }
    cx.ap.reclevel -= 1;
    debug!(cx, "/{} func_returns={:?}", node.kind(), ret_type);
    return ret_type;
}

fn py_make_cx(code: &str) -> ContextPy
{
    let mut sitter = Parser::new();
    sitter.set_language(&language()).unwrap();
    let cx = ContextPy {
        ap: ContextAnyParser {
            sitter,
            reclevel: 0,
            code: code.to_string(),
            errs: AstErrorStats::default(),
            resolved_anything: false,
            defs: IndexMap::new(),
            things: IndexMap::new(),
            usages: vec![],
            alias: IndexMap::new(),
            star_imports: vec![],
        },
    };
    cx
}

pub fn py_parse(code: &str) -> ContextPy
{
    let mut cx = py_make_cx(code);
    let tree = cx.ap.sitter.parse(code, None).unwrap();
    let path = vec!["root".to_string()];
    let mut pass_n = 1;
    loop {
        debug!(&cx, "\n\x1b[31mPASS {}\x1b[0m", pass_n);
        cx.ap.resolved_anything = false;
        py_body(&mut cx, &tree.root_node(), &path);
        if !cx.ap.resolved_anything {
            break;
        }
        cx.ap.usages.clear();
        cx.ap.errs = AstErrorStats::default();
        pass_n += 1;
    }
    cx.ap.defs.insert("root".to_string(), AstDefinition {
        official_path: vec!["root".to_string(), "<toplevel>".to_string()],
        symbol_type: SymbolType::Module,
        usages: vec![],
        resolved_type: "".to_string(),
        this_is_a_class: "".to_string(),
        this_class_derived_from: vec![],
        cpath: "".to_string(),
        decl_line1: 1,
        decl_line2: cx.ap.code.lines().count(),
        body_line1: 0,
        body_line2: 0,
    });
    return cx;
}


#[cfg(test)]
mod tests {
    use super::*;

    fn py_parse4test(code: &str) -> String
    {
        let mut cx = py_parse(code);
        cx.ap.dump();
        let _ = cx.ap.export_defs("test");
        cx.ap.annotate_code("#")
    }

    #[test]
    fn test_parse_py_tort1() {
        let code = include_str!("alt_testsuite/py_torture1_attr.py");
        let annotated = py_parse4test(code);
        std::fs::write("src/ast/alt_testsuite/py_torture1_attr_annotated.py", annotated).expect("Unable to write file");
    }

    #[test]
    fn test_parse_py_tort2() {
        let code = include_str!("alt_testsuite/py_torture2_resolving.py");
        let annotated = py_parse4test(code);
        std::fs::write("src/ast/alt_testsuite/py_torture2_resolving_annotated.py", annotated).expect("Unable to write file");
    }

    #[test]
    fn test_parse_py_goat_library() {
        let code = include_str!("alt_testsuite/py_goat_library.py");
        let annotated = py_parse4test(code);
        std::fs::write("src/ast/alt_testsuite/py_goat_library_annotated.py", annotated).expect("Unable to write file");
    }

    #[test]
    fn test_parse_py_goat_main() {
        let code = include_str!("alt_testsuite/py_goat_main.py");
        let annotated = py_parse4test(code);
        std::fs::write("src/ast/alt_testsuite/py_goat_main_annotated.py", annotated).expect("Unable to write file");
    }
}
