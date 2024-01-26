use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct PythonConfig;

impl Language for PythonConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("class_definition".to_string(), vec!["identifier".to_string()]),
                TypeDeclarationSearchInfo::new("function_definition".to_string(), vec!["identifier".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "False", "def", "if", "raise", "None", "del", "import", "return", "True", "elif", "in",
                "try", "and", "else", "is", "while", "as", "except", "lambda", "with", "assert", "finally",
                "nonlocal", "yield", "break", "for", "not", "class", "from", "or", "continue", "global", "pass",
                "__init__", "__str__", "__repr__", "__len__", "__getitem__", "__setitem__", "__delitem__",
                "__del__", "__iter__", "__reversed__", "__cmp__", "__lt__", "__gt__", "__le__", "__ge__", "__all__",
                "__format__", "__sizeof__", "__str__", "__repr__", "__hash__", "__cmp__", "__lt__", "__gt__",
                "__call__", "Dict", "List", "Tuple", "Set", "Dict", "String", "Bytes", "Bytes", "self", "str", "dict",
                "int", "float", "str", "bool", "None", "bytes", "bytes"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}
// (#eq? @function.name "%s")
// ((function_definition name: (identifier) @function.name body: (block) @function.body) @function.declare)
const PARSER_QUERY_PYTHON_FUNCTION: &str = r#"

(call function: (identifier) @function.used arguments: (argument_list) @function.used_args )

"#;

#[cfg(test)]
mod tests {
    use similar::DiffableStr;
    use tree_sitter::{Point, Range};
    use crate::lsp::treesitter::ast_config::python_config::PARSER_QUERY_PYTHON_FUNCTION;

    const TEST_CODE: &str = 
r#"def foo():
    if bar:
        def baz():
            return 2
        baz(sd)
"#;

    #[test]
    fn test_query_python_function() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_python::language()).unwrap();
        let tree = parser.parse(TEST_CODE, None).unwrap();;
        let query = tree_sitter::Query::new(tree_sitter_python::language(), PARSER_QUERY_PYTHON_FUNCTION).unwrap();
        let mut qcursor = tree_sitter::QueryCursor::new();
        let mut matches = qcursor.matches(&query, tree.root_node(), TEST_CODE.as_bytes());
        let captures = matches.next().unwrap().captures;
        // let captures = matches.next().unwrap().captures;
        // assert_eq!(captures.len(), 3);
        {
            let capture = captures[0];
            let capture_name = &query.capture_names()[capture.index as usize];
            let text = TEST_CODE.slice(capture.node.byte_range());
            // assert_eq!(capture.node.range(), Range {
            //     start_byte: 0,
            //     end_byte: 36,
            //     start_point: Point { row: 0, column: 0 },
            //     end_point: Point { row: 2, column: 13 },
            // });
            // assert_eq!(text, "def foo():\n    if bar:\n        baz()");
        }
        {
            let capture = captures[1];
            let capture_name = &query.capture_names()[capture.index as usize];
            let text = TEST_CODE.slice(capture.node.byte_range());
            assert_eq!(capture.node.range(), Range {
                start_byte: 4,
                end_byte: 7,
                start_point: Point { row: 0, column: 4 },
                end_point: Point { row: 0, column: 7 },
            });
            assert_eq!(text, "foo");
        }
        {
            let capture = captures[2];
            let capture_name = &query.capture_names()[capture.index as usize];
            let text = TEST_CODE.slice(capture.node.byte_range());
            assert_eq!(capture.node.range(), Range {
                start_byte: 15,
                end_byte: 36,
                start_point: Point { row: 1, column: 4 },
                end_point: Point { row: 2, column: 13 },
            });
            assert_eq!(text, "if bar:\n        baz()");
        }
    }
    
}

