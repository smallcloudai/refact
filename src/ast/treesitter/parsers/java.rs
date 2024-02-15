use std::iter::Iterator;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser};
use tree_sitter_java::language;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::UsageSymbolInfo;

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

impl LanguageParser for JavaParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        &JAVA_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &JAVA_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
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

    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
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
