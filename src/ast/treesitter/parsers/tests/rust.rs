#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::ast::treesitter::parsers::AstLanguageParser;
    use crate::ast::treesitter::parsers::rust::RustParser;

    const MAIN_RS_CODE: &str = include_str!("cases/rust/main.rs");
    // const MAIN_RS_INDEXES: &str = include_str!("cases/rust/main.rs.indexes.json");
    // const MAIN_RS_USAGES: &str = include_str!("cases/rust/main.rs.usages.json");

    #[test]
    fn test_query_rust_function() {
        let mut parser = Box::new(RustParser::new().expect("RustParser::new"));
        let asd = parser.parse(MAIN_RS_CODE, &PathBuf::from("main.rs"));
        // let indexes_json: HashMap<String, SymbolDeclarationStruct> = serde_json::from_str(MAIN_RS_INDEXES).unwrap();

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
