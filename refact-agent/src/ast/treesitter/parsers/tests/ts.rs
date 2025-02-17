#[cfg(test)]
mod tests {
    use std::fs::canonicalize;
    use std::path::PathBuf;

    use crate::ast::treesitter::language_id::LanguageId;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::tests::{base_declaration_formatter_test, base_parser_test, base_skeletonizer_test};
    use crate::ast::treesitter::parsers::ts::TSParser;

    const MAIN_TS_CODE: &str = include_str!("cases/ts/main.ts");
    const MAIN_TS_SYMBOLS: &str = include_str!("cases/ts/main.ts.json");

    const PERSON_TS_CODE: &str = include_str!("cases/ts/person.ts");
    const PERSON_TS_SKELETON: &str = include_str!("cases/ts/person.ts.skeleton");
    const PERSON_TS_DECLS: &str = include_str!("cases/ts/person.ts.decl_json");

    #[test]
    fn parser_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(TSParser::new().expect("TSParser::new"));
        let path = PathBuf::from("file:///main.ts");
        base_parser_test(&mut parser, &path, MAIN_TS_CODE, MAIN_TS_SYMBOLS);
    }

    #[test]
    fn skeletonizer_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(TSParser::new().expect("TSParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/ts/person.ts");
        assert!(file.exists());

        base_skeletonizer_test(&LanguageId::Java, &mut parser, &file, PERSON_TS_CODE, PERSON_TS_SKELETON);
    }

    #[test]
    fn declaration_formatter_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(TSParser::new().expect("TSParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/ts/person.ts");
        assert!(file.exists());
        base_declaration_formatter_test(&LanguageId::Java, &mut parser, &file, PERSON_TS_CODE, PERSON_TS_DECLS);
    }
}
