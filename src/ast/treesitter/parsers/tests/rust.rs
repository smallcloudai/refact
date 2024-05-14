#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::rust::RustParser;
    use crate::ast::treesitter::parsers::tests::{base_test, print};

    const MAIN_RS_CODE: &str = include_str!("cases/rust/main.rs");
    const MAIN_RS_SYMBOLS: &str = include_str!("cases/rust/main.rs.json");

    #[test]
    fn test_query_rust_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(RustParser::new().expect("RustParser::new"));
        let path = PathBuf::from("file:///main.rs");
        base_test(&mut parser, &path, MAIN_RS_CODE, MAIN_RS_SYMBOLS);
    }
}
