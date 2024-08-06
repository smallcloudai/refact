use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use uuid::Uuid;
use crate::ast::ast_index::RequestSymbolType;

use crate::ast::structs::FileReferencesResult;
use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam, vec_context_file_to_context_tools};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::call_validation::{ContextFile, ContextEnum};
use crate::files_correction::canonical_path;


fn results2message(result: &FileReferencesResult) -> Vec<ContextFile> {
    let simplified_symbols: Vec<ContextFile> = result.symbols.iter().map(|x| {
        // let path = format!("{:?}::", result.file_path).to_string();
        ContextFile {
            file_name: x.file_path.to_string_lossy().to_string(),
            file_content: format!("{:?}", x.symbol_type),
            line1: x.full_range.start_point.row + 1,
            line2: x.full_range.end_point.row + 1,
            symbol: vec![Uuid::default()],
            gradient_type: -1,
            usefulness: 100.0,
            is_body_important: false
        }
    }).collect();
    simplified_symbols
}

fn text_on_clip(results: &Vec<ContextFile>, from_tool_call: bool) -> String {
    if !from_tool_call {
        return "".to_string();
    }
    let paths = results.iter().map(|x| x.file_name.clone()).collect::<Vec<_>>();
    if paths.is_empty() {
        return "".to_string();
    }
    return if paths.len() == 1 {
        format!("symbols defined in {}", paths[0])
    } else {
        format!("symbols defined in {} and other", paths[0])
    }
}

pub struct AtAstFileSymbols {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

#[async_trait]
impl AtCommand for AtAstFileSymbols {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let mut cpath = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("no file path".to_string());
                args.clear();
                return Err("no file path".to_string());
            },
        };
        cpath.text = canonical_path(&cpath.text).to_string_lossy().to_string();
        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut cpath).await;
        args.clear();
        args.push(cpath.clone());

        let ccx_lock = ccx.lock().await;
        let ast = ccx_lock.global_context.read().await.ast_module.clone();
        let x = match &ast {
            Some(ast) => {
                let doc = crate::files_in_workspace::Document { path: PathBuf::from(cpath.text), text: None };
                match ast.read().await.get_file_symbols(RequestSymbolType::All, &doc).await {
                    Ok(res) => Ok(results2message(&res)),
                    Err(err) => Err(err)
                }
            }
            None => Err("Ast module is not available".to_string())
        };
        let text = x.clone().map(|x| text_on_clip(&x, false)).unwrap_or("".to_string());
        let context_tools_mb = x.map(|j|vec_context_file_to_context_tools(j));
        context_tools_mb.map(|i|(i, text))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
