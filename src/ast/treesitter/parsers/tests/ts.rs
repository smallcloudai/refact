#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::tests::{base_test, print};
    use crate::ast::treesitter::parsers::ts::TSParser;


    const MAIN_TS_CODE: &str = include_str!("cases/ts/main.ts");
    const MAIN_TS_SYMBOLS: &str = include_str!("cases/ts/main.ts.json");
    // const MAIN_RS_INDEXES: &str = include_str!("cases/rust/main.rs.indexes.json");
    // const MAIN_RS_USAGES: &str = include_str!("cases/rust/main.rs.usages.json");

    #[test]
    fn test_query_ts_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(TSParser::new().expect("TSParser::new"));
        let path = PathBuf::from("file:///main.ts");
        base_test(&mut parser, &path, MAIN_TS_CODE, MAIN_TS_SYMBOLS);
    }
}
