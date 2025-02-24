#[cfg(test)]
mod tests {
    use std::fs::canonicalize;
    use std::path::PathBuf;

    use crate::ast::treesitter::language_id::LanguageId;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::rust::RustParser;
    use crate::ast::treesitter::parsers::tests::{base_declaration_formatter_test, base_parser_test, base_skeletonizer_test};

    const MAIN_RS_CODE: &str = include_str!("cases/rust/main.rs");
    const MAIN_RS_SYMBOLS: &str = include_str!("cases/rust/main.rs.json");

    const POINT_RS_CODE: &str = include_str!("cases/rust/point.rs");
    const POINT_RS_DECLS: &str = include_str!("cases/rust/point.rs.decl_json");
    const POINT_RS_SKELETON: &str = include_str!("cases/rust/point.rs.skeleton");

    #[test]
    fn parser_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(RustParser::new().expect("RustParser::new"));
        let path = PathBuf::from("file:///main.rs");
        base_parser_test(&mut parser, &path, MAIN_RS_CODE, MAIN_RS_SYMBOLS);
    }

    #[test]
    fn skeletonizer_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(RustParser::new().expect("RustParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/rust/point.rs");
        assert!(file.exists());

        base_skeletonizer_test(&LanguageId::Rust, &mut parser, &file, POINT_RS_CODE, POINT_RS_SKELETON);
    }

    #[test]
    fn declaration_formatter_test() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(RustParser::new().expect("RustParser::new"));
        let file = canonicalize(PathBuf::from(file!())).unwrap().parent().unwrap().join("cases/rust/point.rs");
        assert!(file.exists());
        base_declaration_formatter_test(&LanguageId::Rust, &mut parser, &file, POINT_RS_CODE, POINT_RS_DECLS);
    }
}
