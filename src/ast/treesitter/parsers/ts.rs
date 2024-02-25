use std::collections::HashSet;
use std::iter::Iterator;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture, Range};
use tree_sitter_typescript::language_typescript;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::utils::{get_function_name, try_to_find_matches};
use crate::ast::treesitter::structs::VariableInfo;

const TYPESCRIPT_PARSER_QUERY_GLOBAL_VARIABLE: &str = "(program [
(lexical_declaration) @global_variable
(ambient_declaration (lexical_declaration)) @global_variable
(variable_declaration) @global_variable
(ambient_declaration (variable_declaration)) @global_variable
])";
const TYPESCRIPT_PARSER_QUERY_FUNCTION: &str = "((function_declaration name: (identifier)) @function)
((method_definition name: (property_identifier)) @function)";
const TYPESCRIPT_PARSER_QUERY_CLASS: &str = "((enum_declaration name: (identifier)) @enum)
((type_alias_declaration name: (type_identifier)) @class)
((interface_declaration name: (type_identifier)) @class)
((class_declaration name: (type_identifier)) @class)";
const TYPESCRIPT_PARSER_QUERY_CALL_FUNCTION: &str = "";
const TYPESCRIPT_PARSER_QUERY_IMPORT_STATEMENT: &str = "";
const TYPESCRIPT_PARSER_QUERY_IMPORT_FROM_STATEMENT: &str = "";
const TYPESCRIPT_PARSER_QUERY_CLASS_METHOD: &str = "";

const TYPESCRIPT_PARSER_QUERY_FIND_VARIABLES: &str = "([
    (lexical_declaration 
        (variable_declarator name: (identifier) @variable_name 
                             type: (type_annotation)? @variable_type
                             value: [
                                (type_assertion (type_arguments) @variable_type)
                                (new_expression constructor: (identifier) @variable_constructor_name 
                                                type_arguments: (type_arguments)? @variable_type)
                             ]?
        )
    )
    (variable_declaration 
        (variable_declarator name: (identifier) @variable_name 
                             type: (type_annotation)? @variable_type
                             value: [
                                (type_assertion (type_arguments) @variable_type)
                                (new_expression constructor: (identifier) @variable_constructor_name 
                                                type_arguments: (type_arguments)? @variable_type)
                             ]?
        )
    )
]) @variable";

const TYPESCRIPT_PARSER_QUERY_FIND_CALLS: &str = r#"((call_expression function: [
(identifier) @call_name
(member_expression property: (property_identifier) @call_name)
]) @call)"#;

const TYPESCRIPT_PARSER_QUERY_FIND_STATICS: &str = r#"(comment) @comment
(string_fragment) @string_literal"#;


const TYPESCRIPT_ALL_TYPES_QUERY: &str = "(predefined_type) @type (type_identifier) @type";
const TYPESCRIPT_NAME_OF_VAR_QUERY: &str = "(variable_declarator name: (identifier) @name)";

lazy_static! {
    static ref TYPESCRIPT_PARSER_QUERY: String = {
        let mut m = Vec::new();
        m.push(TYPESCRIPT_PARSER_QUERY_GLOBAL_VARIABLE);
        m.push(TYPESCRIPT_PARSER_QUERY_FUNCTION);
        m.push(TYPESCRIPT_PARSER_QUERY_CLASS);
        m.push(TYPESCRIPT_PARSER_QUERY_CALL_FUNCTION);
        m.push(TYPESCRIPT_PARSER_QUERY_IMPORT_STATEMENT);
        m.push(TYPESCRIPT_PARSER_QUERY_IMPORT_FROM_STATEMENT);
        m.push(TYPESCRIPT_PARSER_QUERY_CLASS_METHOD);
        m.join("\n")
    };
    
    static ref TYPESCRIPT_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        TYPESCRIPT_PARSER_QUERY_FIND_VARIABLES, TYPESCRIPT_PARSER_QUERY_FIND_CALLS, TYPESCRIPT_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct TypescriptParser {
    pub parser: Parser
}

impl TypescriptParser {
    pub fn new() -> Result<TypescriptParser, ParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(language_typescript())
            .map_err(internal_error)?;
        Ok(TypescriptParser { parser })
    }
}

impl LanguageParser for TypescriptParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        &TYPESCRIPT_PARSER_QUERY
    }

    fn get_parser_query_find_all(&self) -> &String {
        &TYPESCRIPT_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        let name_id = parent.unwrap().language().field_id_for_name("name").unwrap();
        let mut namespaces: Vec<String> = vec![];
        while parent.is_some() {
            match parent.unwrap().kind() {
                "class_declaration" | "enum_declaration" | "interface_declaration" | "type_alias_declaration" => {
                    if let Some(child) = parent.unwrap().child_by_field_id(name_id) {
                        if child.kind() == "type_identifier" {
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

    fn get_variable(&mut self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        let mut types: HashSet<String> = Default::default();

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
                    if let Some(parent) = capture.node.parent() {
                        if parent.kind() == "ambient_declaration" {
                            var.range = parent.range();
                        }
                    }
                }
                "variable_name" => {
                    let text = code.slice(capture.node.byte_range());
                    var.name = text.to_string();
                }
                "variable_constructor_name" => {
                    let text = code.slice(capture.node.byte_range());
                    types.insert(text.to_string());
                }
                "variable_type" => {
                    let local_types = try_to_find_matches(&mut self.parser, TYPESCRIPT_ALL_TYPES_QUERY, &capture.node, code);
                    types.extend(local_types);
                }
                &_ => {}
            }
        }


        if var.name.is_empty() {
            return None;
        }
        if !types.is_empty() {
            var.type_names = types.iter().map(|s| s.to_string()).collect();
        }

        Some(var)
    }

    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        let mut enum_values: Vec<String> = vec![];
        let mut name = "";
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(parent.language(),
                               "(enum_declaration name: (identifier) @enum_name body: (_ (property_identifier) @enum_value))").unwrap();
        let matches = qcursor.matches(&query, parent, text.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match capture_name.as_str() {
                    "enum_name" => {
                        name = text.slice(capture.node.byte_range());
                    }
                    "enum_value" => {
                        let text = text.slice(capture.node.byte_range());
                        enum_values.push(text.to_string());
                    }
                    &_ => {}
                }
            }
        }
        (name.to_string(), enum_values)
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        (get_function_name(parent, text), vec![])
    }

    fn get_variable_name(&self, parent: Node, text: &str) -> String {
        let mut qcursor = tree_sitter::QueryCursor::new();
        let query = Query::new(parent.language(), TYPESCRIPT_NAME_OF_VAR_QUERY).unwrap();
        let matches = qcursor.matches(&query, parent, text.as_bytes());
        for match_ in matches {
            for capture in match_.captures {
                return text.slice(capture.node.byte_range()).to_string();
            }
        }
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::LanguageParser;
    use crate::ast::treesitter::parsers::ts::TypescriptParser;

    const TEST_CODE: &str =
        r#"
// Basic Types
let id: number = 1;
let company: string = 'My Company';
let isPublished: boolean = true;
let x: any = "Hello";
type InOrOut<T> = T extends `fade${infer R}` ? R : never;
let ids: number[] = [1, 2, 3];
let arr: any[] = [1, true, 'Hello'];
const PI: number = 3.14;
var asd: wqe<dfg> = 12;

` 
  This is a multiline string.
  In TypeScript, we use backticks.
  It makes the code more readable.
`

// Tuple
let person: [number, string, boolean] = [1, 'John', true];

// Tuple Array
let employee: [number, string][] = [
  [1, 'John'],
  [2, 'Jane'],
  [3, 'Joe'],
];

// Union
let pid: string | number = 22;
// Enum
enum Direction1 {
  Up,
  Down,
  Left,
  Right,
}

// Objects
type User = {
  id: number;
  name: string;
};

const user: User = {
  id: 1,
  name: 'John',
};

// Type Assertion
let cid: any = 1;
let customerId = <number>cid;

// Functions
function addNum(x: number, y: number): number {
	var s = 2;
  return x + y;
}

class Point {
    constructor(public x: number, public y: number) {}

    euclideanDistance(other: Point): number {
        let dx = other.x - this.x;
        let dy = other.y - this.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}

// Interfaces
interface UserInterface {
  readonly id: number;
  name: string;
  age?: number;
}

const user1: UserInterface = {
  id: 1,
  name: 'John',
};

// Classes
class Person {
  id: number;
  name: string;

  constructor(id: number, name: string) {
    this.id = id;
    this.name = name;
  }
}

const john = new Person(1, 'John');

class GenericNumber<T> {
    zeroValue: T;
    add: (x: T, y: T) => T;
}

let myGenericNumber = new GenericNumber<number>();
myGenericNumber.zeroValue = 0;
myGenericNumber.add = function(x, y) { return x + y; };

console.log(myGenericNumber.add(3, 4)); // Outputs: 7

let stringNumeric = new GenericNumber();
stringNumeric.zeroValue = "";
stringNumeric.add = function(x, y) { return x + y; };

console.log(stringNumeric.add(stringNumeric.zeroValue, "test")); // Outputs: test

// Generics
function getArray<T>(items : T[] ) : T[] {
    return new Array<T>().concat(items);
}

let numArray = getArray<number>([1, 2, 3, 4]);
let strArray = getArray<string>(['John', 'Jane', 'Joe']);

console.log(numArray);
console.console.log(strArray);

// Generic with constraints
interface Lengthy {
    length: number;
}

function countAndDescribe<T extends Lengthy>(element: T): [T, string] {
    let descriptionText = 'Got no value.';
    if (element.length === 1) {
        descriptionText = 'Got 1 value.';
    } else if (element.length > 1) {
        descriptionText = 'Got ' + element.length + ' values.';
    }
    return [element, descriptionText];
}

console.log(countAndDescribe('Hello there'));

declare var jQuery: (selector: string) => any;
let asd = 122;
"#;

    #[test]
    fn test_query_typescript_function() {
        let mut parser = TypescriptParser::new().expect("TypescriptParser::new");
        let path = PathBuf::from("test.ts");
        let indexes = parser.parse_declarations(TEST_CODE, &path).unwrap();
        let zxc = parser.parse_usages(TEST_CODE, true);
        // assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}
