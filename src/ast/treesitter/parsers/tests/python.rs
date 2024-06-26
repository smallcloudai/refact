#[cfg(test)]
mod tests {
    use std::fs::canonicalize;
    use std::path::PathBuf;

    use crate::ast::treesitter::language_id::LanguageId;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::python::PythonParser;
    use crate::ast::treesitter::parsers::tests::{base_declaration_formatter_test, base_parser_test, base_skeletonizer_test};

    const MAIN_PY_CODE: &str = include_str!("cases/python/main.py");
    const CALCULATOR_PY_CODE: &str = include_str!("cases/python/calculator.py");
    const CALCULATOR_PY_SKELETON: &str = include_str!("cases/python/calculator.py.skeleton");
    const CALCULATOR_PY_DECLS: &str = include_str!("cases/python/calculator.py.decl_json");
    const MAIN_PY_SYMBOLS: &str = include_str!("cases/python/main.py.json");

    #[test]
    fn parser_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(PythonParser::new().expect("PythonParser::new"));
        let path = PathBuf::from("file:///main.py");
        base_parser_test(&mut parser, &path, MAIN_PY_CODE, MAIN_PY_SYMBOLS);
    }

    #[test]
    fn skeletonizer_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(PythonParser::new().expect("PythonParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/python/calculator.py");
        assert!(file.exists());

        base_skeletonizer_test(&LanguageId::Python, &mut parser, &file, CALCULATOR_PY_CODE, CALCULATOR_PY_SKELETON);
    }

    #[test]
    fn declaration_formatter_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(PythonParser::new().expect("PythonParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/python/calculator.py");
        assert!(file.exists());
        base_declaration_formatter_test(&LanguageId::Python, &mut parser, &file, CALCULATOR_PY_CODE, CALCULATOR_PY_DECLS);
    }
}
