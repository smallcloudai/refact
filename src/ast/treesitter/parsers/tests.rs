use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::treesitter::parsers::LanguageParser;
use crate::ast::treesitter::structs::{SymbolDeclarationStruct, UsageSymbolInfo};

mod cpp;
mod rust;

pub(crate) fn test_query_function(mut parser: Box<dyn LanguageParser>,
                                  path: &PathBuf,
                                  code: &str,
                                  ref_indexes: HashMap<String, SymbolDeclarationStruct>,
                                  ref_usages: Vec<Box<dyn UsageSymbolInfo>>) {
    let indexes = parser.parse_declarations(code, &path).unwrap();
    let usages = parser.parse_usages(code).unwrap();

    assert_eq!(indexes, ref_indexes);
    assert_eq!(usages, ref_usages);
}
