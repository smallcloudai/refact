use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Range};

use crate::ast::ast_structs::AstDefinition;


pub struct Thing<'a> {
    pub assigned_rvalue: Option<Node<'a>>,
    pub type_explicit: Option<Node<'a>>,
    pub type_resolved: String,
}

pub struct ContextAnyParser<'a> {
    pub sitter: Parser,
    pub last_end_byte: usize,
    pub code: &'a str,
    pub defs: IndexMap<String, AstDefinition>,
    pub things: IndexMap<String, Thing<'a>>,
    // pub draft: Vec<(String, Thing<'a>)>,
}

impl<'a> ContextAnyParser<'a> {
    pub fn whitespace1(&mut self, node: &Node) {
        if node.start_byte() > self.last_end_byte {
            let whitespace = &self.code[self.last_end_byte..node.start_byte()];
            print!("\x1b[32m{}\x1b[0m", whitespace.replace(" ", "·"));
            self.last_end_byte = node.start_byte();
        }
    }

    pub fn whitespace2(&mut self, node: &Node) {
        self.last_end_byte = node.end_byte();
    }

    pub fn just_print(&mut self, node: &Node) {
        self.whitespace1(node);
        print!("{}", &self.code[node.byte_range()].replace(" ", "·"));
        self.whitespace2(node);
    }

    pub fn recursive_print_with_red_brackets(&mut self, node: &Node) {
        self.whitespace1(node);
        match node.kind() {
            "from" | "class" | "import" | "def" | "if" | "for" | ":" | "," | "=" | "." | "(" | ")" | "[" | "]" | "->" => {
                // keywords
                print!("{}", &self.code[node.byte_range()].replace(" ", "·"));
            },
            _ => {
                print!("\x1b[31m{}[\x1b[0m", node.kind());
                for i in 0..node.child_count() {
                    let child = node.child(i).unwrap();
                    self.recursive_print_with_red_brackets(&child);
                }
                if node.child_count() == 0 {
                    print!("{}", &self.code[node.byte_range()]);
                }
                print!("\x1b[31m]\x1b[0m");
            }
        }
        self.whitespace2(node);
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

pub fn type_call(t: String) -> String
{
    // my_function()      t="!MyRutrnType"  =>  "MyRutrnType"
    if t.starts_with("!") {
        return t[1 ..].to_string();
    }
    return "".to_string();
}

pub fn type_deindex(t: String) -> String
{
    // Used in this scenario: for x in my_list
    // t="[MyType]"  =>  "MyType"
    if t.starts_with("[") && t.ends_with("]") {
        return t[1 .. t.len()-1].to_string();
    }
    return "".to_string();
}

pub fn type_zerolevel_comma_split(t: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut level = 0;
    for c in t.chars() {
        match c {
            '[' => {
                level += 1;
                current.push(c);
            },
            ']' => {
                level -= 1;
                current.push(c);
            },
            ',' if level == 0 => {
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
    // Used in this scenario: _, _ = my_value
    // t="[MyClass1,[int,int],MyClass2]"  =>  n==0 MyClass1  n==1 [int,int]   n==2 MyClass2
    if t.starts_with("[") && t.ends_with("]") {
        let no_square = t[1 .. t.len()-1].to_string();
        let parts = type_zerolevel_comma_split(&no_square);
        if n < parts.len() {
            return parts[n].to_string();
        }
    }
    return "".to_string();
}
