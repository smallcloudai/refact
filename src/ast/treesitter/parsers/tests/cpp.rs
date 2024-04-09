#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::cpp::CppParser;

    const MAIN_CPP_CODE: &str = include_str!("cases/cpp/main.cpp");
    // const MAIN_CPP_INDEXES: &str = include_str!("cases/cpp/main.cpp.indexes.json");
    // const MAIN_CPP_USAGES: &str = include_str!("cases/cpp/main.cpp.usages.json");

    #[test]
    fn test_query_cpp_function() {
        let mut parser = Box::new(CppParser::new().expect("CppParser::new"));
        let path = PathBuf::from("/main.cpp");
        let asd = parser.parse(MAIN_CPP_CODE, &path);
        let asd = parser.parse(MAIN_CPP_CODE, &path);
    }
}