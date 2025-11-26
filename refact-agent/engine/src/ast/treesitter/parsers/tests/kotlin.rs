use std::path::PathBuf;

use crate::ast::treesitter::language_id::LanguageId;
use crate::ast::treesitter::parsers::kotlin::KotlinParser;
use crate::ast::treesitter::parsers::tests::{base_parser_test, base_skeletonizer_test, base_declaration_formatter_test};

#[test]
fn test_kotlin_main() {
    let parser = KotlinParser::new().unwrap();
    let mut boxed_parser: Box<dyn crate::ast::treesitter::parsers::AstLanguageParser> = Box::new(parser);
    let path = PathBuf::from("main.kt");
    let code = include_str!("cases/kotlin/main.kt");
    let symbols_str = include_str!("cases/kotlin/main.kt.json");
    base_parser_test(&mut boxed_parser, &path, code, symbols_str);
}

#[test]
fn test_kotlin_person() {
    let parser = KotlinParser::new().unwrap();
    let mut boxed_parser: Box<dyn crate::ast::treesitter::parsers::AstLanguageParser> = Box::new(parser);
    let path = PathBuf::from("person.kt");
    let code = include_str!("cases/kotlin/person.kt");
    let symbols_str = include_str!("cases/kotlin/person.kt.json");
    base_parser_test(&mut boxed_parser, &path, code, symbols_str);
}

#[test]
fn test_kotlin_skeletonizer() {
    let parser = KotlinParser::new().unwrap();
    let mut boxed_parser: Box<dyn crate::ast::treesitter::parsers::AstLanguageParser> = Box::new(parser);
    let path = PathBuf::from("person.kt");
    let code = include_str!("cases/kotlin/person.kt");
    let skeleton_ref_str = include_str!("cases/kotlin/person.kt.skeleton");
    base_skeletonizer_test(&LanguageId::Kotlin, &mut boxed_parser, &path, code, skeleton_ref_str);
}

#[test]
fn test_kotlin_declaration_formatter() {
    let parser = KotlinParser::new().unwrap();
    let mut boxed_parser: Box<dyn crate::ast::treesitter::parsers::AstLanguageParser> = Box::new(parser);
    let path = PathBuf::from("person.kt");
    let code = include_str!("cases/kotlin/person.kt");
    let decls_ref_str = include_str!("cases/kotlin/person.kt.decl_json");
    base_declaration_formatter_test(&LanguageId::Kotlin, &mut boxed_parser, &path, code, decls_ref_str);
}

#[test]
fn test_kotlin_lambda_properties() {
    let parser = KotlinParser::new().unwrap();
    let mut boxed_parser: Box<dyn crate::ast::treesitter::parsers::AstLanguageParser> = Box::new(parser);
    let path = PathBuf::from("lambda_test.kt");
    let code = r#"
class TestClass {
    val realtimeCleaner: () -> String = {
        "test"
    }
    
    val regularProperty: String = "test"
    
    companion object {
        private val logger: String = "test"
    }
}
"#;
    let symbols = boxed_parser.parse(code, &path);
    
    println!("Total symbols found: {}", symbols.len());
    
    for (i, symbol) in symbols.iter().enumerate() {
        let sym = symbol.read();
        println!("Symbol {}: {} - '{}'", i, sym.symbol_type(), sym.name());
        
        if let Some(prop) = sym.as_any().downcast_ref::<crate::ast::treesitter::ast_instance_structs::ClassFieldDeclaration>() {
            println!("  -> Property type: {:?}", prop.type_);
            if let Some(inference) = &prop.type_.inference_info {
                println!("  -> Inference info: {}", inference);
            }
        }
    }
    
    assert!(symbols.len() > 0, "Expected some symbols to be parsed");
}
