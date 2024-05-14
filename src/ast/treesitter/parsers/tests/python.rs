#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::python::PythonParser;
    use crate::ast::treesitter::parsers::tests::base_test;

    const MAIN_PY_CODE: &str = include_str!("cases/python/main.py");
    const MAIN_PY_SYMBOLS: &str = include_str!("cases/python/main.py.json");

    #[test]
    fn test_query_py_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(PythonParser::new().expect("PythonParser::new"));
        let path = PathBuf::from("file:///main.py");
        base_test(&mut parser, &path, MAIN_PY_CODE, MAIN_PY_SYMBOLS);
    }
}
