#[cfg(test)]
mod tests {
    use std::fs::canonicalize;
    use std::path::PathBuf;

    use crate::ast::treesitter::language_id::LanguageId;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::cpp::CppParser;
    use crate::ast::treesitter::parsers::tests::{base_declaration_formatter_test, base_parser_test, base_skeletonizer_test};

    const MAIN_CPP_CODE: &str = include_str!("cases/cpp/main.cpp");
    const MAIN_CPP_SYMBOLS: &str = include_str!("cases/cpp/main.cpp.json");

    const CIRCLE_CPP_CODE: &str = include_str!("cases/cpp/circle.cpp");
    const CIRCLE_CPP_SKELETON: &str = include_str!("cases/cpp/circle.cpp.skeleton");
    const CIRCLE_CPP_DECLS: &str = include_str!("cases/cpp/circle.cpp.decl_json");

    #[test]
    fn parser_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(CppParser::new().expect("CppParser::new"));
        let path = PathBuf::from("/main.cpp");
        base_parser_test(&mut parser, &path, MAIN_CPP_CODE, MAIN_CPP_SYMBOLS);
    }

    #[test]
    fn skeletonizer_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(CppParser::new().expect("CppParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/cpp/circle.cpp");
        assert!(file.exists());

        base_skeletonizer_test(&LanguageId::Cpp, &mut parser, &file, CIRCLE_CPP_CODE, CIRCLE_CPP_SKELETON);
    }

    #[test]
    fn declaration_formatter_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(CppParser::new().expect("CppParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/cpp/circle.cpp");
        assert!(file.exists());
        base_declaration_formatter_test(&LanguageId::Cpp, &mut parser, &file, CIRCLE_CPP_CODE, CIRCLE_CPP_DECLS);
    }
}