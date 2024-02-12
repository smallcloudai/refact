use std::iter::Iterator;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture, Range};
use tree_sitter_python::language;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::{UsageSymbolInfo, VariableInfo};

const PYTHON_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(expression_statement (assignment left: (identifier)) @global_variable)";
const PYTHON_PARSER_QUERY_FUNCTION: &str = "((function_definition name: (identifier)) @function)";
const PYTHON_PARSER_QUERY_CLASS: &str = "((class_definition name: (identifier)) @class)";
const PYTHON_PARSER_QUERY_CALL_FUNCTION: &str = "";
const PYTHON_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const PYTHON_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const PYTHON_PARSER_QUERY_CLASS_METHOD: &str = "";

const PYTHON_PARSER_QUERY_FIND_VARIABLES: &str = r#"(expression_statement 
(assignment left: (identifier) @variable_left type: (_)? @variable_type right: (_) @variable_right) @variable)"#;

const PYTHON_PARSER_QUERY_FIND_CALLS: &str = r#"
((call function: [
(identifier) @call_name
(attribute attribute: (identifier) @call_name)
]) @call)"#;

const PYTHON_PARSER_QUERY_FIND_STATICS: &str = r#"(
([
(comment) @comment
(string) @string_literal
])
)"#;

lazy_static! {
    static ref PYTHON_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(PYTHON_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(PYTHON_PARSER_QUERY_FUNCTION);
        m.push(PYTHON_PARSER_QUERY_CLASS);
        m.push(PYTHON_PARSER_QUERY_CALL_FUNCTION);
        m.push(PYTHON_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(PYTHON_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(PYTHON_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
    
    static ref PYTHON_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        PYTHON_PARSER_QUERY_FIND_VARIABLES, PYTHON_PARSER_QUERY_FIND_CALLS, PYTHON_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct PythonParser {
    pub parser: Parser,
}

impl PythonParser {
    pub fn new() -> Result<PythonParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(PythonParser { parser })
    }
}

impl LanguageParser for PythonParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        &PYTHON_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &PYTHON_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
            match parent.unwrap().kind() {
                "class_definition" => {
                    let children_len = parent.unwrap().child_count();
                    for i in 0..children_len {
                        if let Some(child) = parent.unwrap().child(i) {
                            if child.kind() == "identifier" {
                                namespaces.push(text.slice(child.byte_range()).to_string());
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
            parent = parent.unwrap().parent();
        }
        namespaces.reverse();
        namespaces
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        (get_function_name(parent, text), vec![])
    }

    fn get_variable_name(&self, parent: Node, text: &str) -> String {
        for i in 0..parent.child_count() {
            if let Some(child) = parent.child(i) {
                let kind = child.kind();
                match kind {
                    "identifier" => {
                        let name = text.slice(child.byte_range());
                        return name.to_string();
                    }
                    _ => {}
                }
            }
        }
        return "".to_string();
    }

    fn get_variable(&self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        let mut var = VariableInfo {
            name: "".to_string(),
            range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            type_name: None,
        };
        for capture in captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            match capture_name.as_str() {
                "variable" => {
                    var.range = capture.node.range()
                }
                "variable_left" => {
                    let text = code.slice(capture.node.byte_range());
                    var.name = text.to_string();
                }
                "variable_type" => {
                    let text = code.slice(capture.node.byte_range());
                    var.type_name = Some(text.to_string());
                }
                "variable_right" => {
                    if var.type_name.is_some() {
                        continue;
                    }
                    match capture.node.kind() {
                        "string" => {
                            var.type_name = Some("str".to_string());
                        }
                        "integer" => {
                            var.type_name = Some("int".to_string());
                        }
                        "false" | "true" => {
                            var.type_name = Some("bool".to_string());
                        }
                        "float" => {
                            var.type_name = Some("float".to_string());
                        }
                        // "call" => {
                        //     let node = capture.node;
                        //     for i in 0..node.child_count() {
                        //         if let Some(child) = node.child(i) {
                        //             let kind = child.kind();
                        //             match kind {
                        //                 "identifier" => {
                        //                     let text = code.slice(child.byte_range());
                        //                     var.type_name = Some(text.to_string());
                        //                 }
                        //                 _ => {}
                        //             }
                        //         }
                        //     }
                        // }
                        &_ => {}
                    }
                }
                &_ => {}
            }
        }
        if var.name.is_empty() {
            return None;
        }

        Some(var)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::LanguageParser;
    use crate::ast::treesitter::parsers::python::PythonParser;

    const TEST_CODE: &str =
        r#"import numpy as np
  
global_var = "pip"
bar = true

@dataclass
class C:
    a: int       # 'a' has no default value
    b: int = 0   # assign a default value for 'b'
    def __init__(self):
        self.a = 23

class BabyClass:
    def __init__(self):
        self.xyi = 2
class AdultClass:
    def __init__(self):
        self.xyi = 2
        self.zxc = False
        self.zxcq = "asd"
        self.zxcw = 0.1
        self.qwe = BabyClass()
        
    class NestedClass:
        def __init__(self):
            self.c = 2

zxc = BabyClass()

def baz(asd, zxc):
    pass

@tits
def foo():
    if bar:
        baz(asd, zxc)
"#;

    #[test]
    fn test_query_python_function() {
        let mut parser = PythonParser::new().expect("PythonParser::new");
        let path = PathBuf::from("test.py");
        let indexes = parser.parse_declarations(TEST_CODE, &path).unwrap();
        let zxc = parser.parse_usages(TEST_CODE);
        // assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}
