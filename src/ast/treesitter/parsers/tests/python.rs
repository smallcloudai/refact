#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use url::Url;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::python::PythonParser;

    const MAIN_PY_CODE: &str = include_str!("cases/python/main.py");
    // const MAIN_RS_INDEXES: &str = include_str!("cases/python/main.py.indexes.json");
    // const MAIN_RS_USAGES: &str = include_str!("cases/python/main.py.usages.json");

    #[test]
    fn test_query_rust_function() {
        let mut parser = Box::new(PythonParser::new().expect("PythonParser::new"));
        let path = PathBuf::from("file:///main.py");
        let asd = parser.parse(MAIN_PY_CODE, &path);

        // test_query_function(parser, &path, MAIN_RS_CODE,
        //                     serde_json::from_str(MAIN_RS_INDEXES).unwrap(),
        //                     serde_json::from_str(MAIN_RS_USAGES).unwrap());
        // let usages_json = serde_json::to_string_pretty(&usages).unwrap();

        // // Open a file and write the JSON string to it
        // let mut file = File::create("cases/rust/main.rs.usages.json").unwrap();
        // file.write_all(usages_json.as_bytes()).unwrap();
        //
        // let indexes_json = serde_json::to_string_pretty(&indexes).unwrap();
        //
        // // Open a file and write the JSON string to it
        // let mut file = File::create("cases/rust/main.rs.indexes.json").unwrap();
        // file.write_all(indexes_json.as_bytes()).unwrap();
    }
}
