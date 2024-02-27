use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture, Range};
use tree_sitter_javascript::language;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::VariableInfo;

const JAVASCRIPT_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(program [
(lexical_declaration) @global_variable
(variable_declaration) @global_variable
])";
const JAVASCRIPT_PARSER_QUERY_FUNCTION: &str = "(function_declaration) @function
(method_definition name: (property_identifier)) @function
(generator_function_declaration) @function";

const JAVASCRIPT_PARSER_QUERY_CLASS: &str = "(class_declaration name: (identifier)) @class";
const JAVASCRIPT_PARSER_QUERY_CALL_FUNCTION: &str = "";
const JAVASCRIPT_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const JAVASCRIPT_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const JAVASCRIPT_PARSER_QUERY_CLASS_METHOD: &str = "";

const JAVASCRIPT_PARSER_QUERY_FIND_VARIABLES: &str = "([
    (lexical_declaration 
        (variable_declarator name: (identifier) @variable_name )
    )
    (variable_declaration 
        (variable_declarator name: (identifier) @variable_name)
    )
]) @variable";

const JAVASCRIPT_PARSER_QUERY_FIND_CALLS: &str = r#"((call_expression function: [
(identifier) @call_name
(member_expression property: (property_identifier) @call_name)
]) @call)"#;

const JAVASCRIPT_PARSER_QUERY_FIND_STATICS: &str = r#"(comment) @comment
(string_fragment) @string_literal"#;

const JAVASCRIPT_NAME_OF_VAR_QUERY: &str = "(variable_declarator name: (identifier) @name)";

lazy_static! {
    static ref JAVASCRIPT_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(JAVASCRIPT_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(JAVASCRIPT_PARSER_QUERY_FUNCTION);
        m.push(JAVASCRIPT_PARSER_QUERY_CLASS);
        m.push(JAVASCRIPT_PARSER_QUERY_CALL_FUNCTION);
        m.push(JAVASCRIPT_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(JAVASCRIPT_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(JAVASCRIPT_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
    
    static ref JAVASCRIPT_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        JAVASCRIPT_PARSER_QUERY_FIND_VARIABLES, JAVASCRIPT_PARSER_QUERY_FIND_CALLS, JAVASCRIPT_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct JavascriptParser {
    pub parser: Parser,
}

impl JavascriptParser {
    pub fn new() -> Result<JavascriptParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .map_err(internal_error)?;
        Ok(JavascriptParser { parser })
    }
}

impl LanguageParser for JavascriptParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        &JAVASCRIPT_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &JAVASCRIPT_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        let name_id = parent.unwrap().language().field_id_for_name("name").unwrap();
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
            match parent.unwrap().kind() {
                "class_declaration" => {
                    if let Some(child) = parent.unwrap().child_by_field_id(name_id) {
                        if child.kind() == "identifier" {
                            namespaces.push(text.slice(child.byte_range()).to_string());
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
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(parent.language(), JAVASCRIPT_NAME_OF_VAR_QUERY).unwrap();
        let matches = qcursor.matches(&query, parent, text.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                return text.slice(capture.node.byte_range()).to_string();
            }
        }
        "".to_string()
    }

    fn get_variable(&mut self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        let mut var = VariableInfo {
            name: "".to_string(),
            range: Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Default::default(),
                end_point: Default::default(),
            },
            type_names: vec![],
            meta_path: None,
        };
        for capture in captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            match capture_name.as_str() {
                "variable" => {
                    var.range = capture.node.range();
                }
                "variable_name" => {
                    let text = code.slice(capture.node.byte_range());
                    var.name = text.to_string();
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

    use crate::ast::treesitter::parsers::js::JavascriptParser;
    use crate::ast::treesitter::parsers::LanguageParser;

    const TEST_CODE: &str =
        r#"
// Variables
var name = "John Doe";
let age = 30;
const pi = 3.14;

// Function
function greet(name) {
    return "Hello, " + name;
}

const Color = {
    RED: 'Red',
    BLUE: 'Blue',
    GREEN: 'Green'
};

console.log(Color.RED); // Output: Red

console.log(greet(name)); // Output: Hello, John Doe

// Object
let person = {
    firstName: "John",
    lastName: "Doe",
    fullName: function() {
        return this.firstName + " " + this.lastName;
    }
}

console.log(person.fullName()); // Output: John Doe
let asd = person;

// Array
let fruits = ["apple", "banana", "cherry"];
fruits.forEach(function(item, index, array) {
    console.log(item, index); 
});

// Conditional
if (age > 18) {
    console.log("You are an adult.");
} else {
    console.log("You are a minor.");
}

// Loop
for (let i = 0; i < 5; i++) {
    console.log(i);
}

// Event
document.getElementById("myButton").addEventListener("click", function() {
    alert("Button clicked!");
});

// Define a class
class Person {
    constructor(firstName, lastName) {
        this.firstName = firstName;
        this.lastName = lastName;
    }

    // Method
    fullName() {
        return this.firstName + " " + this.lastName;
    }
}

// Create an instance of the class
let john = new Person("John", "Doe");

console.log(john.fullName()); // Output: John Doe

// Inheritance
class Employee extends Person {
    constructor(firstName, lastName, position) {
        super(firstName, lastName); // Call the parent constructor
        this.position = position;
    }

    // Override method
    fullName() {
        return super.fullName() + ", " + this.position;
    }
}

let jane = new Employee("Jane", "Doe", "Engineer");

console.log(jane.fullName()); // Output: Jane Doe, Engineer

// Function Declaration (or Function Statement)
function add(a, b) {
    return a + b;
}
add(2,3);
console.log(add(1, 2)); // Outputs: 3

// Function Expression
let multiply = function(a, b) {
    return a * b;
}
console.log(multiply(2, 3)); // Outputs: 6

// Arrow Function
let subtract = (a, b) => {
    return a - b;
}
console.log(subtract(5, 2)); // Outputs: 3

// Immediately Invoked Function Expression (IIFE)
(function() {
    console.log('This is an IIFE');
})(); // Outputs: This is an IIFE

// Constructor Function
function Person(name, age) {
    this.name = name;
    this.age = age;
}

let john = new Person('John', 30);
console.log(john); // Outputs: Person { name: 'John', age: 30 }

// Generator Function
function* idGenerator() {
    let id = 0;
    while (true) {
        yield id++;
    }
}

var gen = idGenerator();
console.log(gen.next().value); // Outputs: 0
console.log(gen.next().value); // Outputs: 1

import React from 'react';

class HelloWorld extends React.Component {
    render() {
        return (
            <div>
                <h1>Hello, World!</h1>
                <p>Welcome to React.</p>
            </div>
        );
    }
}

export default HelloWorld;
"#;

    #[test]
    fn test_query_javascript_function() {
        let mut parser = JavascriptParser::new().expect("JavascriptParser::new");
        let path = PathBuf::from("test.js");
        let indexes = parser.parse_declarations(TEST_CODE, &path).unwrap();
        let zxc = parser.parse_usages(TEST_CODE, true);
        // assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}
