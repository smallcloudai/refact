#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::cpp::CppParser;
    use crate::ast::treesitter::parsers::tests::test_query_function;

    const MAIN_CPP_CODE: &str = include_str!("cases/cpp/main.cpp");
    const MAIN_CPP_INDEXES: &str = include_str!("cases/cpp/main.cpp.indexes.json");
    const MAIN_CPP_USAGES: &str = include_str!("cases/cpp/main.cpp.usages.json");

    #[test]
    fn test_query_cpp_function() {
        let parser = Box::new(CppParser::new().expect("CppParser::new"));
        let path = PathBuf::from("main.cpp");
        test_query_function(parser, &path, MAIN_CPP_CODE, 
                            serde_json::from_str(MAIN_CPP_INDEXES).unwrap(), 
                            serde_json::from_str(MAIN_CPP_USAGES).unwrap());
    }
}