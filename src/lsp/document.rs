use std::any::Any;
use std::collections::HashMap;
use std::fmt::Display;
use futures_util::SinkExt;

use ropey::Rope;
use tower_lsp::jsonrpc::{Error, Result};
use tracing::error;
use tree_sitter::{Node, Parser, Tree};

use crate::lsp::language_id::LanguageId;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};
use crate::lsp::treesitter::ast_config::apex_config::ApexConfig;
use crate::lsp::treesitter::ast_config::bash_config::BashConfig;
use crate::lsp::treesitter::ast_config::c_config::CConfig;
use crate::lsp::treesitter::ast_config::cpp_config::CppConfig;
use crate::lsp::treesitter::ast_config::csharp_config::CSharpConfig;
use crate::lsp::treesitter::ast_config::css_config::CssConfig;
use crate::lsp::treesitter::ast_config::d_config::DConfig;
use crate::lsp::treesitter::ast_config::elm_config::ElmConfig;
use crate::lsp::treesitter::ast_config::go_config::GoConfig;
use crate::lsp::treesitter::ast_config::html_config::HtmlConfig;
use crate::lsp::treesitter::ast_config::java_config::JavaConfig;
use crate::lsp::treesitter::ast_config::js_config::JSConfig;
use crate::lsp::treesitter::ast_config::kotlin_config::KotlinConfig;
use crate::lsp::treesitter::ast_config::lua_config::LuaConfig;
use crate::lsp::treesitter::ast_config::ocaml_config::OcamlConfig;
use crate::lsp::treesitter::ast_config::php_config::PhpConfig;
use crate::lsp::treesitter::ast_config::python_config::PythonConfig;
use crate::lsp::treesitter::ast_config::sql_config::SqlConfig;
use crate::lsp::treesitter::ast_config::ts_config::TSConfig;
use crate::lsp::treesitter::symbol_declaration_struct::SymbolDeclarationStruct;

fn internal_error<E: Display>(err: E) -> Error {
    let err_msg = err.to_string();
    error!(err_msg);
    Error {
        code: tower_lsp::jsonrpc::ErrorCode::InternalError,
        message: err_msg.into(),
        data: None,
    }
}

#[derive(Clone)]
pub struct TypeDeclarationSearchInfo {
    pub node_type: String,
    pub name_node_types: Vec<String>,
}

impl TypeDeclarationSearchInfo {
    pub fn default() -> Self {
        TypeDeclarationSearchInfo {
            node_type: "".to_string(),
            name_node_types: vec![],
        }
    }
    pub fn new(node_type: String, name_node_types: Vec<String>) -> Self {
        TypeDeclarationSearchInfo { node_type, name_node_types }
    }
}

fn get_parser(language_id: LanguageId) -> Result<(Parser, AstConfig)> {
    match language_id {
        LanguageId::Apex => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_apex::language())
                .map_err(internal_error)?;
            Ok((parser, ApexConfig::make_ast_config()))
        }
        LanguageId::Bash => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_bash::language())
                .map_err(internal_error)?;
            Ok((parser, BashConfig::make_ast_config()))
        }
        LanguageId::C => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_c::language())
                .map_err(internal_error)?;
            Ok((parser, CConfig::make_ast_config()))
        }
        LanguageId::Cpp => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_cpp::language())
                .map_err(internal_error)?;
            Ok((parser, CppConfig::make_ast_config()))
        }
        LanguageId::CSharp => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_c_sharp::language())
                .map_err(internal_error)?;
            Ok((parser, CSharpConfig::make_ast_config()))
        }
        LanguageId::Css => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_css::language())
                .map_err(internal_error)?;
            Ok((parser, CssConfig::make_ast_config()))
        }
        LanguageId::D => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_d::language())
                .map_err(internal_error)?;
            Ok((parser, DConfig::make_ast_config()))
        }
        LanguageId::Elm => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_elm::language())
                .map_err(internal_error)?;
            Ok((parser, ElmConfig::make_ast_config()))
        }
        // LanguageId::Elixir => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_elixir::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        // LanguageId::Erlang => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_erlang::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        LanguageId::Go => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_go::language())
                .map_err(internal_error)?;
            Ok((parser, GoConfig::make_ast_config()))
        }
        LanguageId::Html => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_html::language())
                .map_err(internal_error)?;
            Ok((parser, HtmlConfig::make_ast_config()))
        }
        LanguageId::Java => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_java::language())
                .map_err(internal_error)?;
            Ok((parser, JavaConfig::make_ast_config()))
        }
        LanguageId::JavaScript | LanguageId::JavaScriptReact => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_javascript::language())
                .map_err(internal_error)?;
            Ok((parser, JSConfig::make_ast_config()))
        }
        LanguageId::Kotlin => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_kotlin::language())
                .map_err(internal_error)?;
            Ok((parser, KotlinConfig::make_ast_config()))
        }
        // LanguageId::Json => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_json::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        LanguageId::Lua => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_lua::language())
                .map_err(internal_error)?;
            Ok((parser, LuaConfig::make_ast_config()))
        }
        LanguageId::Ocaml => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_ocaml::language_ocaml())
                .map_err(internal_error)?;
            Ok((parser, OcamlConfig::make_ast_config()))
        }
        // LanguageId::Markdown => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_md::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        // LanguageId::ObjectiveC => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_objc::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        LanguageId::Php => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_php::language())
                .map_err(internal_error)?;
            Ok((parser, PhpConfig::make_ast_config()))
        }
        LanguageId::Python => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_python::language())
                .map_err(internal_error)?;
            Ok((parser, PythonConfig::make_ast_config()))
        }
        LanguageId::R => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_r::language())
                .map_err(internal_error)?;
            Ok((parser, AstConfig::make_ast_config()))
        }
        LanguageId::Ruby => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_ruby::language())
                .map_err(internal_error)?;
            Ok((parser, AstConfig::make_ast_config()))
        }
        LanguageId::Rust => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_rust::language())
                .map_err(internal_error)?;
            Ok((parser, AstConfig::make_ast_config()))
        }
        LanguageId::Scala => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_scala::language())
                .map_err(internal_error)?;
            Ok((parser, AstConfig::make_ast_config()))
        }
        LanguageId::Sql => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_sql_bigquery::language())
                .map_err(internal_error)?;
            Ok((parser, SqlConfig::make_ast_config()))
        }
        LanguageId::Swift => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_swift::language())
                .map_err(internal_error)?;
            Ok((parser, AstConfig::make_ast_config()))
        }
        // LanguageId::Toml => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_toml::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        LanguageId::TypeScript => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_typescript::language_typescript())
                .map_err(internal_error)?;
            Ok((parser, TSConfig::make_ast_config()))
        }
        LanguageId::TypeScriptReact => {
            let mut parser = Parser::new();
            parser
                .set_language(tree_sitter_typescript::language_tsx())
                .map_err(internal_error)?;
            Ok((parser, TSConfig::make_ast_config()))
        }
        // LanguageId::Vue => {
        //     let mut parser = Parser::new();
        //     parser
        //         .set_language(tree_sitter_vue::language())
        //         .map_err(internal_error)?;
        //     Ok((parser, AstConfig::make_ast_config()))
        // }
        LanguageId::Unknown => Ok((Parser::new(), AstConfig::make_ast_config())),
    }
}

pub struct Document {
    pub(crate) language_id: LanguageId,
    pub(crate) text: Rope,
    pub(crate) path: Rope,
    parser: Parser,
    pub(crate) tree: Option<Tree>,
    pub(crate) config: AstConfig,
}

impl Document {
    pub fn open(language_id: &str, text: &str, path: &str) -> Result<Self> {
        let language_id = language_id.into();
        let (mut parser, config) = get_parser(language_id)?;
        let tree = parser.parse(text, None);
        let mut doc = Document {
            language_id,
            text: Rope::from_str(text),
            path: Rope::from_str(path),
            parser,
            tree,
            config,
        };
        let s = doc.extract_definition_symbols();
        Ok(doc)
    }

    pub(crate) async fn change(&mut self, text: &str) -> Result<()> {
        let rope = Rope::from_str(text);
        self.tree = self.parser.parse(text, Some(&self.tree.clone().unwrap()));
        self.text = rope;
        Ok(())
    }

    fn extract_definition_symbols(&mut self) -> HashMap<String, SymbolDeclarationStruct> {
        let symbols: HashMap<String, SymbolDeclarationStruct> = HashMap::default();
        let cursor = (&self).tree.as_mut().unwrap().walk();

        let mut reached_root = false;
        // let searching_nodes = HashMap::from(
        //     self.config.type_declaration_search_info.clone().iter()
        //         .map(|f| (f.node_type, f)).collect());
        while !reached_root {
            let s = cursor.node().type_id();
            let z = 0;
        //     // cursor.node().type_id()
        //     if !cursor.node().has_error() {
        //         let search_info = searching_nodes
        //         [cursor.node. type ]
        //         let search_info = searching_nodes
        //         [cursor.node. type ]
        //     }
            reached_root = true;
        }

        symbols
    }
    fn search_down(&mut self, node: Node) {}
}