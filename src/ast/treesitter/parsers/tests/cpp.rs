#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::cpp::CppParser;
    use crate::ast::treesitter::parsers::tests::base_test;

    const MAIN_CPP_CODE: &str = include_str!("cases/cpp/main.cpp");
    const MAIN_CPP_SYMBOLS: &str = include_str!("cases/cpp/main.cpp.json");

    #[test]
    fn test_query_cpp_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(CppParser::new().expect("CppParser::new"));
        let path = PathBuf::from("/main.cpp");
        base_test(&mut parser, &path, MAIN_CPP_CODE, MAIN_CPP_SYMBOLS);
    }
}