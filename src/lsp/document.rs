use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Display;
use std::sync::Arc;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub struct AstContext {
    pub(crate) tree: Arc<Tree>,
    pub(crate) definition_symbols: HashMap<usize, SymbolDeclarationStruct>,
    // pub(crate) positions_map_rows: HashMap<usize, Vec<Arc<Node<'a>>>>,
    pub(crate) all_symbols: HashSet<String>,
}

impl AstContext {
    pub fn new(tree: Arc<Tree>, config: AstConfig, rope_text: Rope, rope_path: &Rope) -> Self {
        let definition_symbols = extract_definition_symbols(tree.clone(), config.clone(),
                                                            rope_text.clone(), &rope_path.clone());
        // let positions_map_rows = extract_positions_map(tree.clone());
        let all_symbols = extract_all_symbols(tree.clone(), rope_text.clone(), config.clone());
        return AstContext {
            tree,
            definition_symbols,
            all_symbols,
        }
    }
}


pub struct Document {
    pub(crate) language_id: LanguageId,
    pub(crate) text: Rope,
    pub(crate) path: Rope,
    parser: Parser,
    pub(crate) ast_config: AstConfig,
    pub(crate) ast_context: Option<AstContext>
}

fn search_down<'a>(node: &'a Node<'a>, node_types_: &'a Vec<String>) -> Option<Node<'a>> {
    let node_types = HashSet::from_iter(node_types_.clone());
    let mut result: Vec<(Option<Node>, i32)> = vec![];

    fn _helper<'a>(node: Node<'a>, 
                   current_depth: i32, 
                   node_types: HashSet<String>, 
                   result: &mut Vec<(Option<Node<'a>>, i32)>) {
        for idx in 0..node.child_count() {
            let child = node.child(idx);
            let ch = child.unwrap();
            let type_name = ch.kind().to_string();
            if node_types.contains(&type_name) {
                result.push((child.clone(), current_depth));
            } else {
                let child = child.unwrap();
                _helper(child.clone(), current_depth + 1, node_types.clone(), result);
            }
        }
    }
    _helper(node.clone(), 1, node_types.clone(), &mut result);
    return if result.len() == 0 {
        None
    } else {
        let mut m = result[0];
        for r in result {
            if r.1 < m.1 {
                m = r;
            }
        }
        m.0
    };
}

fn search_namespace(mut node: Option<Node>, namespace_search_info: Option<TypeDeclarationSearchInfo>, text: Rope) -> Vec<String> {
    if namespace_search_info.is_none() {
        return vec![];
    }
    let namespace_search_info = namespace_search_info.unwrap();
    let mut names: Vec<String> = vec![];
    while node.is_some() {
        let real_node = node.unwrap();
        if real_node.kind().to_string() == namespace_search_info.node_type {
            let name_node = search_down(&real_node, &namespace_search_info.name_node_types);
            if name_node.is_some() {
                let name_node = name_node.unwrap();
                let name = text.slice(name_node.start_byte()..name_node.end_byte());
                names.push(name.to_string());
            }
        }
        node = real_node.parent();
    }
    names.reverse();
    names
}

fn get_parent_ids(mut node: Option<Node>, ids: HashSet<usize>) -> Vec<usize> {
    let mut parent_ids = vec![];
    while node.is_some() {
        let real_node = node.unwrap();
        if ids.contains(&real_node.id()) {
            parent_ids.push(real_node.id());
        }
        node = real_node.parent();
    }
    parent_ids.reverse();
    parent_ids
}



fn extract_definition_symbols(tree: Arc<Tree>, config: AstConfig, text: Rope, path: &Rope)
                              -> HashMap<usize, SymbolDeclarationStruct> {
    let mut symbols: HashMap<usize, SymbolDeclarationStruct> = HashMap::default();
    let mut cursor = tree.walk();
    let q = r#"(function_definition name: (identifier) @function.def)"#;
    let query = tree_sitter::Query::new(tree_sitter_python::language(), q).unwrap();
    let mut qcursor = tree_sitter::QueryCursor::new();
    let zxc = text.to_string();
    for mat in qcursor.matches(&query, tree.root_node(), text.to_string().as_bytes()) {
        for capture in mat.captures {
            let start = capture.node.start_position();
            let end = capture.node.end_position();
            let capture_name = &query.capture_names()[capture.index as usize];
            let z = &query.capture_names()[capture.index as usize];;
        }
    }

    let mut reached_root = false;
    let searching_nodes: HashMap<String, TypeDeclarationSearchInfo> =
        HashMap::from_iter(config.type_declaration_search_info.clone().iter()
            .map(|f| (f.clone().node_type, f.clone())).collect::<Vec<_>>());
    while !reached_root {
        let cursor_node = cursor.node();
        if searching_nodes.contains_key(&cursor_node.kind().to_string()) && !(&cursor_node.has_error()) {
            let type_name = cursor_node.kind().to_string();
            let search_info = searching_nodes.get(&type_name.clone()).unwrap();
            let name_node = search_down(&cursor_node, &search_info.name_node_types);
            if name_node.is_some() {
                let node = name_node.unwrap();
                let name = text.slice(node.start_byte()..node.end_byte()).to_string();
                let namespace = search_namespace(name_node, config.namespace_search_info.clone(), text.clone());
                let parent_ids = get_parent_ids(cursor_node.parent(), HashSet::from_iter(symbols.keys().cloned()));
                symbols.insert(cursor_node.id(), SymbolDeclarationStruct {
                    id: cursor_node.id(),
                    node_type: type_name,
                    name,
                    content: text.slice(cursor_node.start_byte()..cursor_node.end_byte()).to_string(),
                    start_point: cursor_node.start_position(),
                    end_point: cursor_node.end_position(),
                    path: path.to_string(),
                    parent_ids: Some(parent_ids),
                    namespaces_name: Some(namespace),
                });
            }
        }

        if cursor.goto_first_child() {
            continue;
        }

        if cursor.goto_next_sibling() {
            continue;
        }
        let mut retracing = true;
        while retracing {
            if !cursor.goto_parent() {
                retracing = false;
                reached_root = true;
            }
            if cursor.goto_next_sibling() {
                retracing = false;
            }
        }
    }

    symbols
}



const DIGITS: &str = "0123456789";
const PUNCTUATION: &str = r#"#!$%&'"()*+,-./:;<=>?@[\]^_`{|}~"#;


fn get_nodes_nearby<'a>(
    row_idx: i32,
    row_max_distance: i32,
    positions_map_rows: &'a HashMap<usize, Vec<Node<'a>>>,
    text: &'a Rope,
    config: AstConfig,
    filter_num_and_punctuation: bool,
    filter_reserved_words: bool,
    filter_keywords: bool,
    filter_empty: bool,
    filter_by_text_duplicate: bool,
) -> Vec<Node<'a>> {
    let mut nodes: Vec<Node<'a>> = vec![];

    let min_row_idx = max(0, row_idx - row_max_distance);
    let max_row_idx = min(row_idx + row_max_distance, positions_map_rows.len() as i32);
    for idx in min_row_idx..max_row_idx {
        let pos_vec = positions_map_rows.get(&(idx as usize));
        if pos_vec.is_some() {
            let pos_vec = pos_vec.unwrap();
            nodes.append(pos_vec.clone().as_mut());
        }
        nodes = nodes.iter().filter(|node| {
            let binding = text.to_string();
            let text = node.utf8_text(binding.as_ref());
            let mut res = true;
            if text.is_err() {
                return false;
            }

            let text = text.unwrap();
            if filter_num_and_punctuation {
                res &= !text.chars().all(|c| {
                    char::is_numeric(c) && PUNCTUATION.contains(c)
                });
            }
            if filter_reserved_words {
                res &= (node.kind() != text);
            }
            if filter_keywords {
                res &= config.keywords.contains(&text.to_string());
            }
            if filter_empty {
                res &= text.len() != 0;
            }
            res
        }).cloned().collect();
        if filter_by_text_duplicate {
            let mut texts: HashSet<String> = Default::default();
            let mut filtered_nodes: Vec<Node> = vec![];
            for node in nodes {
                if let Ok(text) = node.utf8_text(text.to_string().as_ref()) {
                    if texts.contains(&text.to_string()) {
                        continue;
                    }
                    texts.insert(text.to_string());
                    filtered_nodes.push(node);
                }
            }
            nodes = filtered_nodes;
        }
    }
    nodes
}

fn extract_all_symbols(tree: Arc<Tree>, text: Rope, config: AstConfig)
                       -> HashSet<String> {
    // extract_positions_map start
    let mut cursor = tree.walk();
    let mut positions_map: HashMap<usize, Vec<Node>> = Default::default();
    let mut reached_root = false;
    while !reached_root {
        let node = cursor.node();
        if node.child_count() == 0 {
            positions_map.entry(node.start_position().row).or_default().push(node);
        }

        if cursor.goto_first_child() {
            continue;
        }

        if cursor.goto_next_sibling() {
            continue;
        }
        let mut retracing = true;
        while retracing {
            if !cursor.goto_parent() {
                retracing = false;
                reached_root = true;
            }
            if cursor.goto_next_sibling() {
                retracing = false;
            }
        }
    }
    // extract_positions_map finish
    let nodes = get_nodes_nearby(
        0,
        positions_map.len() as i32,
        &positions_map,
        &text,
        config,
        true,
        true,
        true,
        false,
        true);
    let mut res: HashSet<String> = Default::default();
    for n in nodes {
        if let Ok(text) = n.utf8_text(text.to_string().as_ref()) {
            res.insert(text.to_string());
        }
    }
    res
}

// fn extract_positions_map<'a>(tree: Arc<Tree>) -> HashMap<usize, Vec<Arc<Node<'a>>>> {
//     let mut cursor = tree.walk();
//     let mut positions_map: HashMap<usize, Vec<Arc<Node>>> = Default::default();
//     let mut reached_root = false;
//     while !reached_root {
//         let node = Arc::new(cursor.node());
//         if node.child_count() == 0 {
//             positions_map.entry(node.start_position().row).or_default().push(node);
//         }
//     
//         if cursor.goto_first_child() {
//             continue;
//         }
//     
//         if cursor.goto_next_sibling() {
//             continue;
//         }
//         let mut retracing = true;
//         while retracing {
//             if !cursor.goto_parent() {
//                 retracing = false;
//                 reached_root = true;
//             }
//             if cursor.goto_next_sibling() {
//                 retracing = false;
//             }
//         }
//     }
//     positions_map
// }

impl Document {
    pub fn open(language_id: &str, text: &str, path: &str) -> Result<Self> {
        let language_id = language_id.into();
        let (mut parser, config) = get_parser(language_id)?;
        let tree = parser.parse(text, None);
        let rope_text = Rope::from_str(text);
        let rope_path = Rope::from_str(path);
        match tree {
            None => {
                Ok(Document {
                    language_id,
                    text: rope_text,
                    path: rope_path,
                    parser,
                    ast_config: config.clone(),
                    ast_context: None,
                })
            }
            Some(tr) => {
                Ok(Document {
                    language_id,
                    text: rope_text.clone(),
                    path: rope_path.clone(),
                    parser,
                    ast_config: config.clone(),
                    ast_context: Some(AstContext::new(Arc::new(tr), config.clone(), rope_text.clone(), &rope_path)),
                })
            }
        }
    }

    pub(crate) async fn change(&mut self, text: &str) -> Result<()> {
        let rope = Rope::from_str(text);
        self.text = rope.clone();
        if let Some(ast_context) = self.ast_context.as_mut() {
            if let Some(tree) = self.parser.parse(text, Option::from(ast_context.tree.as_ref())) {
                ast_context.tree = Arc::new(tree);
            }
        }
        Ok(())
    }
}