#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::tests::base_test;
    use crate::ast::treesitter::parsers::ts::TSParser;

    const MAIN_TS_CODE: &str = include_str!("cases/ts/main.ts");
    const MAIN_TS_SYMBOLS: &str = include_str!("cases/ts/main.ts.json");
    
    #[test]
    fn test_query_ts_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(TSParser::new().expect("TSParser::new"));
        let path = PathBuf::from("file:///main.ts");
        base_test(&mut parser, &path, MAIN_TS_CODE, MAIN_TS_SYMBOLS);
    }
}
