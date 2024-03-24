use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AMutex;

use crate::ast::structs::AstQuerySearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamSymbolPathQuery;
use crate::call_validation::{ChatMessage, ContextFile};
use tracing::{info, error};
use crate::ast::ast_index::RequestSymbolType;
use crate::ast::ast_module::AstModule;
use crate::files_in_workspace::DocumentInfo;
use crate::ast::treesitter::ast_instance_structs::SymbolInformation;


async fn oleg_check_out_file_map(
    astmod: &AstModule,
    result: &AstQuerySearchResult
) {
    let mut files_affected: Vec<DocumentInfo> = vec![];
    for res in &result.search_results {
        let file_path: String = res.symbol_declaration.get_path_str();
        files_affected.push(DocumentInfo::from_pathbuf(&std::path::PathBuf::from(file_path)).unwrap());
    }
    files_affected.sort();
    files_affected.dedup();
    for file_info in files_affected {
        let filename = file_info.uri.to_file_path().unwrap_or_default().to_str().unwrap_or_default().to_string();  // sync with SymbolInformation::get_path_str
        let file_markup: crate::ast::structs::FileASTMarkup = match astmod.file_markup(&file_info).await {
            Ok(x) => x,
            Err(e) => {
                error!("file_markup error {:?}", e);
                continue;
            }
        };
        let lines_cnt = file_markup.file_content.lines().count();
        let mut color: Vec<String> = vec!["".to_string(); lines_cnt];
        let mut useful: Vec<f64> = vec![0.0; lines_cnt];

        let mut colorize = |symb: &SymbolInformation, spath: &String|
        {
            if symb.declaration_range.end_byte != 0 {
                // full_range Range { start_byte: 696, end_byte: 1563, start_point: Point { row: 23, column: 4 }, end_point: Point { row: 47, column: 5 } }
                // declaration_range Range { start_byte: 696, end_byte: 842, start_point: Point { row: 23, column: 4 }, end_point: Point { row: 27, column: 42 } }
                // definition_range Range { start_byte: 843, end_byte: 1563, start_point: Point { row: 27, column: 43 }, end_point: Point { row: 47, column: 5 } }
                for line_num in symb.declaration_range.start_point.row..symb.declaration_range.end_point.row+1 {
                    if !color[line_num].is_empty() {
                        continue;
                    }
                    color[line_num] = format!("decl {}", spath);
                    useful[line_num] = 85.0;
                }
                for line_num in symb.definition_range.start_point.row..symb.definition_range.end_point.row+1 {
                    if !color[line_num].is_empty() {
                        continue;
                    }
                    color[line_num] = format!("def  {}", spath);
                    useful[line_num] = 75.0;
                }
            } else {
                for line_num in symb.full_range.start_point.row..symb.full_range.end_point.row+1 {
                    if !color[line_num].is_empty() {
                        continue;
                    }
                    color[line_num] = format!("full {}", spath);
                    useful[line_num] = 95.0;
                }
            }
        };

        fn path_of_guid(guid: &String, file_markup: &crate::ast::structs::FileASTMarkup) -> String
        {
            match file_markup.guid2symbol.get(guid) {
                Some(x) => {
                    let pname = if !x.name.is_empty() { x.name.clone() } else { x.guid[..8].to_string() };
                    let pp = path_of_guid(&x.parent_guid, &file_markup);
                    return format!("{}::{}", pp, pname);
                },
                None => {
                    info!("parent_guid {} not found, maybe outside of this file", guid);
                    return format!("UNK");
                }
            };
        }

        info!("markup {}", filename);
        for res in &result.search_results {
            if res.symbol_declaration.get_path_str() != filename {
                continue;
            }
            let mut guid = res.symbol_declaration.guid.clone();
            while !guid.is_empty() {
                let spath = path_of_guid(&guid, &file_markup);
                let symbol: &SymbolInformation = match file_markup.guid2symbol.get(&guid) {
                    Some(x) => x,
                    None => { break; }
                };
                colorize(&symbol, &spath);
                guid = symbol.parent_guid.clone();
            }
        }
        let mut prev_n = 0 as usize;
        let mut result = String::new();
        for (line_n, line) in file_markup.file_content.lines().enumerate() {
            if useful[line_n] == 0.0 {
                continue;
            }
            if line_n > prev_n + 1 {
                result += "...\n";
                // info!("{:05} line {:?}", line_n, line);
            }
            result += &format!("{:05} {:>5.1} {}\n", line_n, useful[line_n], color[line_n]);
            prev_n = line_n;
        }
        info!("result {}\n{}", filename, result);
    }
}


async fn results2message(result: &AstQuerySearchResult) -> ChatMessage {
    // info!("results2message {:?}", result);
    let mut symbols = vec![];
    for res in &result.search_results {
        let file_path: String = res.symbol_declaration.get_path_str();
        let content = res.symbol_declaration.get_content().await.unwrap_or("".to_string());
        symbols.push(ContextFile {
            file_name: file_path,
            file_content: content,
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            usefulness: 100.0 * res.sim_to_query
        });
    }
    ChatMessage {
        role: "context_file".to_string(),
        content: json!(symbols).to_string(),
    }
}

pub struct AtAstDefinition {
    pub name: String,
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstDefinition {
    pub fn new() -> Self {
        AtAstDefinition {
            name: "@definition".to_string(),
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolPathQuery::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtAstDefinition {
    fn name(&self) -> &String {
        &self.name
    }
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }
    async fn execute(&self, _query: &String, args: &Vec<String>, _top_n: usize, context: &AtCommandsContext) -> Result<ChatMessage, String> {
        let can_execute = self.can_execute(args, context).await;
        if !can_execute {
            return Err("incorrect arguments".to_string());
        }
        info!("execute @definition {:?}", args);
        let symbol_path = match args.get(0) {
            Some(x) => x,
            None => return Err("no symbol path".to_string()),
        };
        let binding = context.global_context.read().await;
        let x = match *binding.ast_module.lock().await {
            Some(ref ast) => {
                match ast.search_by_name(symbol_path.clone(), RequestSymbolType::Declaration).await {
                    Ok(res) => {
                        // TMP
                        oleg_check_out_file_map(ast, &res).await;
                        // TMP
                        Ok(results2message(&res).await)
                    },
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        }; x
    }
}
