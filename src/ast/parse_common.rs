// use std::ascii::Char;
use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Range};

use crate::ast::ast_structs::{AstDefinition, AstUsage, AstErrorStats};


#[derive(Debug)]
pub struct Thing {
    #[allow(dead_code)]
    pub tline: usize,  // only needed for printing in this file
    pub public: bool,
    pub thing_kind: char,
    pub type_resolved: String,
}

pub struct ContextAnyParser {
    pub sitter: Parser,
    // pub code: &'a str,
    pub code: String,
    pub errs: AstErrorStats,
    pub reclevel: usize,
    pub resolved_anything: bool,
    pub defs: IndexMap<String, AstDefinition>,
    pub things: IndexMap<String, Thing>,
    pub usages: Vec<(String, AstUsage)>,
    // Aliases:
    // from hello.world import MyClass       ->   (file::MyClass, hello::world::MyClass)
    // from hello.world import MyClass as A  ->   (file::A, hello::world::MyClass)
    pub alias: IndexMap<String, String>,
    // TODO
    // star imports are bad for resolving stuff
    // from hello.world import *             ->   any unresolved usage of MyClass becomes ?::hello::world::MyClass, ?::MyClass
    #[allow(dead_code)]
    pub star_imports: Vec<String>,
}

impl ContextAnyParser {
    pub fn error_report(&mut self, node: &Node, msg: String) -> String {
        let line = node.range().start_point.row + 1;
        let mut node_text = self.code[node.byte_range()].to_string().replace("\n", "\\n");
        if node_text.len() > 50 {
            node_text = node_text.chars().take(50).collect();
            node_text.push_str("...");
        }
        self.errs.add_error(
            "".to_string(),
            line,
            format!("{msg}: {:?} in {node_text}", node.kind()).as_str());
        return format!("line {}: {msg} {}", line, self.recursive_print_with_red_brackets(node));
    }

    pub fn recursive_print_with_red_brackets(&self, node: &Node) -> String {
        self._recursive_print_with_red_brackets_helper(node, 0)
    }

    fn _recursive_print_with_red_brackets_helper(&self, node: &Node, rec: usize) -> String {
        let mut result = String::new();
        let color_code = if rec >= 1 { "\x1b[90m" } else { "\x1b[31m" };
        match node.kind() {
            "from" | "class" | "import" | "def" | "if" | "for" | ":" | "," | "=" | "." | "(" | ")" | "[" | "]" | "->" => {
                result.push_str(&self.code[node.byte_range()]);
            },
            _ => {
                result.push_str(&format!("{}{}[\x1b[0m", color_code, node.kind()));
                for i in 0..node.child_count() {
                    let child = node.child(i).unwrap();
                    let field_name = node.field_name_for_child(i as u32).unwrap_or("");
                    if field_name != "" && rec == 0 {
                        result.push_str(&format!("\x1b[35mfield_name={:?} \x1b[0m", field_name));
                    } else if rec == 0 {
                        result.push_str(&format!("\x1b[35mnaf\x1b[0m"));
                    }
                    result.push_str(&self._recursive_print_with_red_brackets_helper(&child, rec + 1));
                }
                if node.child_count() == 0 {
                    result.push_str(&self.code[node.byte_range()]);
                }
                result.push_str(&format!("{}]\x1b[0m", color_code));
            }
        }
        result
    }

    pub fn indent(&self) -> String {
        return " ".repeat(self.reclevel*4);
    }

    pub fn indented_println(&self, args: std::fmt::Arguments) {
        println!("{}{}", self.indent(), args);
    }

    #[allow(dead_code)]
    pub fn dump(&self) {
        println!("\n  -- things -- ");
        for (key, thing) in self.things.iter() {
            println!("{:<40} {} {:<40} {}", key, thing.thing_kind, thing.type_resolved, if thing.public { "pub" } else { "" } );
        }
        println!("  -- /things --\n");

        println!("\n  -- usages -- ");
        for (uat, u) in self.usages.iter() {
            println!("{:<40} {:03} {:?}", uat, u.uline + 1, u);
        }
        println!("  -- /usages -- ");

        println!("\n  -- defs -- ");
        for (key, def) in self.defs.iter() {
            println!("{:<40} {:?}", key, def);
        }
        println!("  -- /defs -- ");

        println!("\n  -- alias -- ");
        for (key, dest) in self.alias.iter() {
            println!("{:<40} -> {:?}", key, dest);
        }
        println!("  -- /alias -- ");

        println!("\n  -- star imports -- ");
        for star in self.star_imports.iter() {
            println!("{:<40}", star);
        }
        println!("  -- /star -- ");
    }

    #[allow(dead_code)]
    pub fn annotate_code(&self, comment: &str) -> String {
        let mut r = String::new();
        let lines: Vec<&str> = self.code.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            r.push_str(line);
            let mut usages_on_line = Vec::new();
            for (_, usage) in &self.usages {
                if usage.uline == i {
                    usages_on_line.push(format!("{:?}", usage));
                }
            }
            let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
            for err in &self.errs.errors {
                if err.err_line == i + 1 {
                    r.push_str(format!("\n{indent}{comment} ERROR {}", err.err_message).as_str());
                }
            }
            for (key, thing) in &self.things {
                if thing.tline == i {
                    let mut key_last = key.split("::").last().unwrap_or("").to_string();
                    if thing.thing_kind == 'f' {
                        key_last += "()";
                    }
                    r.push_str(format!("\n{indent}{comment} {} {} {}", thing.thing_kind, key_last, thing.type_resolved).as_str());
                }
            }
            if !usages_on_line.is_empty() {
                r.push_str(format!("\n{}{} {}", indent, comment, usages_on_line.join(" ")).as_str());
            }
            r.push('\n');
        }
        r
    }

    pub fn export_defs(&mut self, cpath: &str) -> Vec<AstDefinition> {  // self.defs becomes empty after this operation
        for (def_key, def) in &mut self.defs {
            let def_offpath = def.official_path.join("::");
            assert!(*def_key == def_offpath || format!("{}::<toplevel>", *def_key) == def_offpath);
            def.usages.clear();
            def.cpath = cpath.to_string();
        }
        for (usage_at, usage) in &self.usages {
            // println!("usage_at {} {:?} usage.resolved_as={:?}", usage_at, usage, usage.resolved_as);
            assert!(usage.resolved_as.is_empty() || usage.resolved_as.starts_with("root::") || usage.resolved_as.starts_with("?::"));
            let mut atv = usage_at.split("::").collect::<Vec<&str>>();
            let mut found_home = false;
            while !atv.is_empty() {
                let k = atv.join("::");
                if let Some(existing_def) = self.defs.get_mut(&k) {
                    existing_def.usages.push(usage.clone());
                    found_home = true;
                    break;
                }
                atv.pop();
            }
            if !found_home {
                self.errs.add_error(
                    "".to_string(),
                    usage.uline + 1,
                    format!("cannot find parent for {}", usage_at).as_str()
                );
            }
        }
        let mut new_defs = IndexMap::new();
        std::mem::swap(&mut self.defs, &mut new_defs);
        new_defs.into_values().collect()
    }
}

pub fn line12mid_from_ranges(full_range: &Range, body_range: &Range) -> (usize, usize, usize)
{
    let line1: usize = full_range.start_point.row;
    let mut line_mid: usize = full_range.end_point.row;
    let line2: usize = full_range.end_point.row;
    if body_range.start_byte > 0 {
        line_mid = body_range.start_point.row;
        assert!(line_mid >= line1);
        assert!(line_mid <= line2);
    }
    (line1, line2, line_mid)
}


// -----------------------------------------------------------

// pub fn any_child_of_type_recursive<'a>(node: Node<'a>, of_type: &str) -> Option<Node<'a>>
// {
//     if node.kind() == of_type {
//         return Some(node);
//     }
//     for i in 0 .. node.child_count() {
//         if let Some(found) = any_child_of_type_recursive(node.child(i).unwrap(), of_type) {
//             return Some(found);
//         }
//     }
//     None
// }

pub fn any_child_of_type<'a>(node: Node<'a>, of_type: &str) -> Option<Node<'a>>
{
    for i in 0 .. node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == of_type {
            return Some(child);
        }
    }
    None
}

pub fn type_call(t: String, _arg_types: String) -> String
{
    if t.starts_with("ERR/") {
        return t;
    }
    // my_function()      t="!MyReturnType"  =>  "MyReturnType"
    if t.starts_with("!") {
        return t[1 ..].to_string();
    }
    return "?".to_string();
}

pub fn type_deindex(t: String) -> String
{
    if t.starts_with("ERR/") {
        return t;
    }
    // Used in this scenario: for x in my_list
    // t="[MyType]"  =>  "MyType"
    if t.starts_with("[") && t.ends_with("]") {
        return t[1 .. t.len()-1].to_string();
    }
    // can't do anything for ()
    return "".to_string();
}

pub fn type_zerolevel_comma_split(t: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut level_brackets1 = 0;
    let mut level_brackets2 = 0;
    for c in t.chars() {
        match c {
            '[' => {
                level_brackets1 += 1;
                current.push(c);
            },
            ']' => {
                level_brackets1 -= 1;
                current.push(c);
            },
            '(' => {
                level_brackets2 += 1;
                current.push(c);
            },
            ')' => {
                level_brackets2 -= 1;
                current.push(c);
            },
            ',' if level_brackets1 == 0 && level_brackets2 == 0 => {
                parts.push(current.to_string());
                current = String::new();
            },
            _ => {
                current.push(c);
            }
        }
    }
    parts.push(current.to_string());
    parts
}

pub fn type_deindex_n(t: String, n: usize) -> String
{
    if t.starts_with("ERR/") {
        return t;
    }
    // Used in this scenario: _, _ = my_value
    // t="[MyClass1,[int,int],MyClass2]"  =>  n==0 MyClass1  n==1 [int,int]   n==2 MyClass2
    if t.starts_with("(") && t.ends_with(")") {
        let no_square = t[1 .. t.len()-1].to_string();
        let parts = type_zerolevel_comma_split(&no_square);
        if n < parts.len() {
            return parts[n].to_string();
        }
    }
    return "".to_string();
}
