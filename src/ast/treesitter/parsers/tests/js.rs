#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::js::JSParser;
    use crate::ast::treesitter::parsers::tests::base_test;

    const MAIN_JS_CODE: &str = include_str!("cases/js/main.js");
    const MAIN_JS_SYMBOLS: &str = include_str!("cases/js/main.js.json");

    #[test]
    fn test_query_js_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(JSParser::new().expect("JSParser::new"));
        let path = PathBuf::from("file:///main.js");
        base_test(&mut parser, &path, MAIN_JS_CODE, MAIN_JS_SYMBOLS);
    }
}
