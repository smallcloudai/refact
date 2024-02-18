use std::iter::Iterator;
use std::string::ToString;

use similar::DiffableStr;
use structopt::lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCapture};
use tree_sitter_typescript::language_tsx;

use crate::ast::treesitter::parsers::{internal_error, LanguageParser, ParserError};
use crate::ast::treesitter::parsers::ts::TypescriptParser;
use crate::ast::treesitter::parsers::utils::get_function_name;
use crate::ast::treesitter::structs::{UsageSymbolInfo, VariableInfo};

const TYPESCRIPTX_PARSER_QUERY_FIND_VARIABLES: &str = "([
    (lexical_declaration 
        (variable_declarator name: (identifier) @variable_name 
                             type: (type_annotation)? @variable_type
                             value: [
                                (new_expression constructor: (identifier) @variable_constructor_name 
                                                type_arguments: (type_arguments)? @variable_type)
                             ]?
        )
    )
    (variable_declaration 
        (variable_declarator name: (identifier) @variable_name 
                             type: (type_annotation)? @variable_type
                             value: [
                                (new_expression constructor: (identifier) @variable_constructor_name 
                                                type_arguments: (type_arguments)? @variable_type)
                             ]?
        )
    )
]) @variable";

const TYPESCRIPTX_PARSER_QUERY_FIND_CALLS: &str = r#"((call_expression function: [
(identifier) @call_name
(member_expression property: (property_identifier) @call_name)
]) @call)"#;

const TYPESCRIPTX_PARSER_QUERY_FIND_STATICS: &str = r#"(comment) @comment
(string_fragment) @string_literal"#;

lazy_static! {
    static ref TYPESCRIPTX_PARSER_QUERY_FIND_ALL: String = format!("{}\n{}\n{}", 
        TYPESCRIPTX_PARSER_QUERY_FIND_VARIABLES, TYPESCRIPTX_PARSER_QUERY_FIND_CALLS, TYPESCRIPTX_PARSER_QUERY_FIND_STATICS);
}

pub(crate) struct TypescriptxParser {
    inner: TypescriptParser,
    pub parser: Parser,
}

impl TypescriptxParser {
    pub fn new() -> Result<TypescriptxParser, ParserError> {
        let inner = TypescriptParser::new()?;
        let mut parser = Parser::new();
        parser
            .set_language(language_tsx())
            .map_err(internal_error)?;
        Ok(TypescriptxParser { inner, parser })
    }
}

impl LanguageParser for TypescriptxParser {
    fn get_parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    fn get_parser_query(&self) -> &String {
        self.inner.get_parser_query()
    }

    fn get_parser_query_find_all(&self) -> &String {
        &TYPESCRIPTX_PARSER_QUERY_FIND_ALL
    }

    fn get_namespace(&self, mut parent: Option<Node>, text: &str) -> Vec<String> {
        self.inner.get_namespace(parent, text)
    }

    fn get_enum_name_and_all_values(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        self.inner.get_enum_name_and_all_values(parent, text)
    }

    fn get_function_name_and_scope(&self, parent: Node, text: &str) -> (String, Vec<String>) {
        (get_function_name(parent, text), vec![])
    }

    fn get_variable_name(&self, parent: Node, text: &str) -> String {
        self.inner.get_variable_name(parent, text)
    }

    fn get_variable(&mut self, captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
        self.inner.get_variable(captures, query, code)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::LanguageParser;
    use crate::ast::treesitter::parsers::tsx::TypescriptxParser;

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

// Type Assertion doesn't work in XML extension
// let cid: any = 1;
// let customerId = <number>cid;

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
import React, { useState } from 'react';

class Point {
    constructor(public x: number, public y: number) {}

    euclideanDistance(other: Point): number {
        let dx = other.x - this.x;
        let dy = other.y - this.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}

interface UserProps {
  name: string;
  initialAge: number;
}

const UserComponent: React.FC<UserProps> = ({ name, initialAge }) => {
  const [age, setAge] = useState(initialAge);

  const handleBirthday = () => {
    setAge(age + 1);
  };

  return (
    <div>
      <p>Hello, {name}!</p>
      <p>You are {age} years old.</p>
      <button onClick={handleBirthday}>Celebrate Birthday</button>
    </div>
  );
};

interface ListProps<T> {
  items: T[];
  itemRenderer: (item: T) => JSX.Element;
}

const ListComponent = <T,>({ items, itemRenderer }: ListProps<T>) => {
  return (
    <div>
      {items.map(itemRenderer)}
    </div>
  );
};

interface IProps<T> {
  items: T[];
}

class MyGenericComponent<T> extends React.Component<IProps<T>, {}> {
  render() {
    const { items } = this.props;
    return (
      <div>
        {items.map((item, index) => (
          <div key={index}>{item.toString()}</div>
        ))}
      </div>
    );
  }
}

export default MyGenericComponent;
export default ListComponent;
export default UserComponent;
"#;

    #[test]
    fn test_query_typescriptx_function() {
        let mut parser = TypescriptxParser::new().expect("TypescriptXParser::new");
        let path = PathBuf::from("test.tsx");
        let indexes = parser.parse_declarations(TEST_CODE, &path).unwrap();
        let zxc = parser.parse_usages(TEST_CODE);
        // assert_eq!(indexes.len(), 1);
        // assert_eq!(indexes.get("function").unwrap().name, "foo");
    }
}
