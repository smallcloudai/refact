use std::sync::Arc;
use tracing::info;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tree_sitter::Point;

use crate::ast::structs::AstCursorSearchResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::at_file::{AtParamFilePath, RangeKind, colon_lines_range_from_arg};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::call_validation::{ContextFile, ContextEnum};


pub async fn results2message(result: &AstCursorSearchResult) -> Vec<ContextFile> {
    // info!("results2message {:?}", result);
    let mut fvec = vec![];
    for res in &result.bucket_declarations {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: vec![res.symbol_declaration.guid.clone()],
            gradient_type: -1,
            usefulness: res.usefulness,
            is_body_important: false
        });
    }
    for res in &result.bucket_usage_of_same_stuff {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: vec![res.symbol_declaration.guid.clone()],
            gradient_type: -1,
            usefulness: res.usefulness,
            is_body_important: true
        });
    }
    for res in &result.bucket_high_overlap {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: vec![res.symbol_declaration.guid.clone()],
            gradient_type: -1,
            usefulness: res.usefulness,
            is_body_important: true
        });
    }
    for res in &result.bucket_imports {
        let file_name = res.symbol_declaration.file_path.to_string_lossy().to_string();
        fvec.push(ContextFile {
            file_name,
            file_content: res.content.clone(),
            line1: res.symbol_declaration.full_range.start_point.row + 1,
            line2: res.symbol_declaration.full_range.end_point.row + 1,
            symbol: vec![res.symbol_declaration.guid.clone()],
            gradient_type: -1,
            usefulness: res.usefulness,
            is_body_important: false
        });
    }
    fvec
}

pub fn text_on_clip(results: &Vec<ContextFile>, from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    let paths = results.iter().map(|x| x.file_name.clone()).collect::<Vec<_>>();
    if paths.is_empty() {
        return "".to_string();
    }
    return if paths.len() == 1 {
        format!("found symbols from  {}", paths[0])
    } else {
        format!("found symbols from {} and other", paths[0])
    }
}

pub struct AtAstLookupSymbols {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstLookupSymbols {
    pub fn new() -> Self {
        AtAstLookupSymbols {
            params: vec![
                Arc::new(AMutex::new(AtParamFilePath::new()))
            ],
        }
    }
}

pub async fn execute_at_ast_lookup_symbols(ccx: Arc<AMutex<AtCommandsContext>>, file_path: &String, row_idx: usize) -> Result<Vec<ContextFile>, String> {
    let cpath = crate::files_correction::canonical_path(&file_path);
    let gcx = ccx.lock().await.global_context.clone();
    let ast = gcx.read().await.ast_module.clone();
    let x = match &ast {
        Some(ast) => {
            let mut doc = crate::files_in_workspace::Document { path: cpath.clone(), text: None };
            let file_text = crate::files_in_workspace::get_file_text_from_memory_or_disk(gcx.clone(), &cpath).await?; // FIXME
            doc.update_text(&file_text);
            match ast.read().await.symbols_near_cursor_to_buckets(
                &doc, &file_text, Point { row: row_idx, column: 0 }, 15,  3
            ).await {
                Ok(res) => Ok(results2message(&res).await),
                Err(err) => Err(err)
            }
        }
        None => Err("Ast module is not available".to_string())
    };
    x
}

#[async_trait]
impl AtCommand for AtAstLookupSymbols {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>
    ) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @lookup_symbols_at {:?}", args);

        let mut file_path = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("file path is missing".to_string());
                args.clear();
                return Err("no file path".to_string());
            }
        };
        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut file_path).await;
        args.clear();
        args.push(file_path.clone());

        let row_idx = match colon_lines_range_from_arg(&mut file_path.text) {
            Some(x) => {
                if x.kind == RangeKind::GradToCursorTwoSided {
                    x.line1
                } else {
                    return Err("line number is not a valid".to_string());
                }
            },
            None => return Err("line number is not a valid".to_string()),
        };
        let results = execute_at_ast_lookup_symbols(ccx.clone(), &file_path.text, row_idx).await?;
        let text = text_on_clip(&results, false);
        Ok((vec_context_file_to_context_tools(results), text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
