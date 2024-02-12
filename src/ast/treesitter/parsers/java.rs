use std::collections::HashMap;
use std::iter::Iterator;
use std::path::PathBuf;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture, Range, Tree};
use tree_sitter_java::language;
use crate::ast::treesitter::language_id::LanguageId;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::{get_call, get_function_name, get_static};
use crate::ast::treesitter::structs::{SymbolDeclarationStruct, SymbolInfo, SymbolType, UsageSymbolInfo, VariableInfo};

const JAVA_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(program (local_variable_declaration (_)) @global_variable)";
const JAVA_PARSER_QUERY_FUNCTION: &str = "";
const JAVA_PARSER_QUERY_CLASS: &str = "((interface_declaration (_)) @interface)
((class_declaration (_)) @class)
((enum_declaration (_)) @enum)";
const JAVA_PARSER_QUERY_CALL_FUNCTION: &str = "((method_declaration (_)) @function)";
const JAVA_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const JAVA_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const JAVA_PARSER_QUERY_CLASS_METHOD: &str = "";

const JAVA_PARSER_QUERY_FIND_VARIABLES: &str = r#"
((local_variable_declaration type: [
(array_type element: (_) @variable_type)
(generic_type . (type_identifier) @variable_type)
(floating_point_type) @variable_type
(void_type) @variable_type
(integral_type) @variable_type
(boolean_type) @variable_type
(type_identifier) @variable_type
] 
declarator: (variable_declarator name: (identifier) @variable_name)) @variable)"#;

const JAVA_PARSER_QUERY_FIND_CALLS: &str = r#"(expression_statement (method_invocation name: (identifier) @call_name) @call)"#;

const JAVA_PARSER_QUERY_FIND_STATICS: &str = r#"(
([
(line_comment) @comment
(block_comment) @comment
(string_literal) @string_literal
])
)"#;

lazy_static! {
    static ref JAVA_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(JAVA_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(JAVA_PARSER_QUERY_FUNCTION);
        m.push(JAVA_PARSER_QUERY_CLASS);
        m.push(JAVA_PARSER_QUERY_CALL_FUNCTION);
        m.push(JAVA_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(JAVA_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(JAVA_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
    
    static ref JAVA_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        JAVA_PARSER_QUERY_FIND_VARIABLES, JAVA_PARSER_QUERY_FIND_CALLS, JAVA_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct JavaParser {
    pub parser: Parser,
}

impl JavaParser {
    pub fn new() -> Result<JavaParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(JavaParser { parser })
    }
}

fn get_namespace(mut parent: Option<Node>, text: &str) -> Vec<String> {
    let mut namespaces: Vec<String> = vec![];
    while parent.is_some() {
        match parent.unwrap().kind() {
            "class_declaration" => {
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

fn get_variable_name(parent: Node, text: &str) -> String {
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

fn get_variable(captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
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
            "variable_name" => {
                let text = code.slice(capture.node.byte_range());
                var.name = text.to_string();
            }
            "variable_type" => {
                let text = code.slice(capture.node.byte_range());
                var.type_name = Some(text.to_string());
            }
            &_ => {}
        }
    }
    if var.name.is_empty() {
        return None;
    }

    Some(var)
}

fn get_enum_name_and_all_values(parent: Node, text: &str) -> (String, Vec<String>) {
    let mut name: String = Default::default();
    let mut values: Vec<String> = vec![];
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            let kind = child.kind();
            match kind {
                "identifier" => {
                    name = text.slice(child.byte_range()).to_string();
                }
                "enum_body" => {
                    for i in 0..child.child_count() {
                        if let Some(child) = child.child(i) {
                            let kind = child.kind();
                            match kind {
                                "enum_constant" => {
                                    for i in 0..child.child_count() {
                                        if let Some(child) = child.child(i) {
                                            let kind = child.kind();
                                            match kind {
                                                "identifier" => {
                                                    let text = text.slice(child.byte_range());
                                                    values.push(text.to_string());
                                                    break;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    (name, values)
}


impl LanguageParser for JavaParser {
    fn parse_declarations(&mut self, code: &str, path: &PathBuf) -> Result<HashMap<String, SymbolDeclarationStruct>, String> {
        let mut indexes: HashMap<String, SymbolDeclarationStruct> = Default::default();
        let tree: Tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(tree_sitter_java::language(), &**JAVA_PARSER_QUERY).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "class" | "struct" => {
                        let range = capture.node.range();
                        let namespaces = get_namespace(Some(capture.node), code);
                        let class_name = namespaces.last().unwrap().clone();
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name: class_name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::Class,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                       });
                    }
                    "enum" => {
                        let range = capture.node.range();
                        let mut namespaces = get_namespace(Some(capture.node), code);
                        let (enum_name, values) = get_enum_name_and_all_values(capture.node, code);
                        namespaces.push(enum_name);
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        values.iter().for_each(|value| {
                            let key = format!("{}::{}", key, value);
                            indexes.insert(key.clone(),
                                           SymbolDeclarationStruct {
                                               name: value.clone(),
                                               definition_info: SymbolInfo { path: path.clone(), range },
                                               children: vec![],
                                               symbol_type: SymbolType::Enum,
                                               meta_path: key,
                                               language: LanguageId::from(capture.node.language()),
                                           });
                        });
                    }
                    "function" => {
                        let range = capture.node.range();
                        let mut namespaces = get_namespace(Some(capture.node), code);
                        let name = get_function_name(capture.node.clone(), code);
                        namespaces.push(name.clone());
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::Function,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                       });
                    }
                    "global_variable" => {
                        let range = capture.node.range();
                        let mut namespaces = get_namespace(Some(capture.node), code);
                        let name = get_variable_name(capture.node, code);
                        let mut key = path.to_str().unwrap().to_string();
                        namespaces.push(name.clone());
                        namespaces.iter().for_each(|ns| {
                            key += format!("::{}", ns).as_str();
                        });
                        indexes.insert(key.clone(),
                                       SymbolDeclarationStruct {
                                           name,
                                           definition_info: SymbolInfo { path: path.clone(), range },
                                           children: vec![],
                                           symbol_type: SymbolType::GlobalVar,
                                           meta_path: key,
                                           language: LanguageId::from(capture.node.language()),
                                       });
                    }
                    &_ => {}
                }
            }
        }
        Ok(indexes)
    }
    fn parse_usages(&mut self, code: &str) -> Result<Vec<Box<dyn UsageSymbolInfo>>, String> {
        let tree: Tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Err("Parse error".to_string()),
        };
        let mut usages: Vec<Box<dyn UsageSymbolInfo>> = vec![];
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(language(), &**JAVA_PARSER_QUERY_FIND_ALL).unwrap();
        let matches = qcursor.matches(&query, tree.root_node(), code.as_bytes());
        for match_ in matches {
            match match_.pattern_index {
                0 => {
                    if let Some(var) = get_variable(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                1 => {
                    if let Some(var) = get_call(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                2 => {
                    if let Some(var) = get_static(match_.captures, &query, code) {
                        usages.push(Box::new(var));
                    }
                }
                _ => {}
            }
        }
        Ok(usages)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::ast::treesitter::parsers::java::JavaParser;
    use crate::ast::treesitter::parsers::LanguageParser;

    const TEST_CODE: &str =
        r#"interface Animal {
  public void animalSound(); // interface method (does not have a body)
  public void run(); // interface method (does not have a body)
}

//Java Program to illustrate how to define a class and fields  
//Defining a Student class.  
class Student{  
 //defining fields  
 int id;//field or data member or instance variable  
 String name;  
 //creating main method inside the Student class  
 public static void pip(String args[]){  
  //Creating an object or instance  
  Student s1=new Student();//creating an object of Student  
  //Printing values of the object  
  System.out.println(s1.id);//accessing member through reference variable  
  System.out.println(s1.name);  
 }  
}  

int as =2 ;
Poo<s> as =2 ;
float asd = 2;
Poo qwe = 2;

public class Main {
  enum Level {
    LOW,
    MEDIUM,
    HIGH
  }

  public static void main(String[] args) {
    String[] cars = {"Volvo", "BMW", "Ford", "Mazda"};
    System.out.println(cars.length);
  }
}
"#;

    #[test]
    fn test_query_java_function() {
        let mut parser = JavaParser::new().expect("JavaParser::new");
        let path = PathBuf::from("test.java");
        let indexes = parser.parse_declarations(TEST_CODE, &path).unwrap();
        let zxc = parser.parse_usages(TEST_CODE);
        // assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}
