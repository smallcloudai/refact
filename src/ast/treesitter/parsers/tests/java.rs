#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::java::JavaParser;
    use crate::ast::treesitter::parsers::tests::base_test;

    const MAIN_JAVA_CODE: &str = include_str!("cases/java/main.java");
    const MAIN_JAVA_SYMBOLS: &str = include_str!("cases/java/main.java.json");

    #[test]
    fn test_query_java_function() {
        let mut parser: Box<dyn AstLanguageParser> = Box::new(JavaParser::new().expect("JavaParser::new"));
        let path = PathBuf::from("file:///main.java");
        base_test(&mut parser, &path, MAIN_JAVA_CODE, MAIN_JAVA_SYMBOLS);
    }
}
