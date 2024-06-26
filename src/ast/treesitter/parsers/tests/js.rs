#[cfg(test)]
mod tests {
    use std::fs::canonicalize;
    use std::path::PathBuf;

    use crate::ast::treesitter::language_id::LanguageId;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::js::JSParser;
    use crate::ast::treesitter::parsers::tests::{base_declaration_formatter_test, base_parser_test, base_skeletonizer_test};

    const MAIN_JS_CODE: &str = include_str!("cases/js/main.js");
    const MAIN_JS_SYMBOLS: &str = include_str!("cases/js/main.js.json");

    const CAR_JS_CODE: &str = include_str!("cases/js/car.js");
    const CAR_JS_SKELETON: &str = include_str!("cases/js/car.js.skeleton");
    const CAR_JS_DECLS: &str = include_str!("cases/js/car.js.decl_json");

    #[test]
    fn parser_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(JSParser::new().expect("JSParser::new"));
        let path = PathBuf::from("file:///main.js");
        base_parser_test(&mut parser, &path, MAIN_JS_CODE, MAIN_JS_SYMBOLS);
    }

    #[test]
    fn skeletonizer_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(JSParser::new().expect("JSParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/js/car.js");
        assert!(file.exists());

        base_skeletonizer_test(&LanguageId::Java, &mut parser, &file, CAR_JS_CODE, CAR_JS_SKELETON);
    }

    #[test]
    fn declaration_formatter_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(JSParser::new().expect("JSParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/js/car.js");
        assert!(file.exists());
        base_declaration_formatter_test(&LanguageId::Java, &mut parser, &file, CAR_JS_CODE, CAR_JS_DECLS);
    }
}
