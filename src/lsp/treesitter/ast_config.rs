use crate::lsp::document::TypeDeclarationSearchInfo;

pub mod apex_config;
pub mod bash_config;
pub mod c_config;
pub mod cpp_config;
pub mod css_config;
pub mod csharp_config;
pub mod d_config;
pub mod elm_config;
pub mod go_config;
pub mod html_config;
pub mod java_config;
pub mod js_config;
pub mod kotlin_config;
pub mod lua_config;
pub mod ocaml_config;
pub mod php_config;
pub mod python_config;
pub mod r_config;
pub mod ruby_config;
pub mod rust_config;
pub mod scala_config;
pub mod sql_config;
pub mod swift_config;
pub mod ts_config;

pub struct AstConfig {
    pub type_declaration_search_info: Vec<TypeDeclarationSearchInfo>,
    pub namespace_search_info: Option<TypeDeclarationSearchInfo>,
    pub keywords: Vec<String>,
    pub keywords_types: Vec<String>,
}

impl AstConfig {
    pub fn default() -> Self {
        Self {
            type_declaration_search_info: vec![],
            keywords: vec![],
            namespace_search_info: None,
            keywords_types: vec![],
        }
    }
}

pub trait Language {
    fn make_ast_config() -> AstConfig;
}

impl Language for AstConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig::default()
    }
}
