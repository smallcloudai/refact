use indexmap::IndexMap;
use tree_sitter::{Node, Parser, Point, Range, Query, QueryCursor};
use tree_sitter_python::language;

use crate::ast::ast_structs::{AstDefinition, AstUsage};
use crate::ast::treesitter::structs::SymbolType;


pub struct ContextAnyParser<'a> {
    pub sitter: Parser,
    pub last_end_byte: usize,
    pub code: &'a str,
    pub defs: IndexMap<String, AstDefinition>,
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
            "from" | "class" | "import" | "def" | "if" | "for" | ":" | "," | "=" | "." | "(" | ")" => {
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
