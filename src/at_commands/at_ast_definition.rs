use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tracing::info;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::at_params::AtParamSymbolPathQuery;
use crate::call_validation::{ContextFile, ContextEnum};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};


pub struct AtAstDefinition {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstDefinition {
    pub fn new() -> Self {
        AtAstDefinition {
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolPathQuery::new()))
            ],
        }
    }
}

#[async_trait]
impl AtCommand for AtAstDefinition {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        info!("execute @definition {:?}", args);
        let mut symbol = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false;
                cmd.reason = Some("parameter is missing".to_string());
                args.clear();
                return Err("parameter `symbol` is missing".to_string());
            },
        };

        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut symbol).await;
        args.clear();
        args.push(symbol.clone());

        let gcx = ccx.lock().await.global_context.clone();
        let ast_service_opt = gcx.read().await.ast_service.clone();
        if let Some(ast_service) = ast_service_opt {
            let alt_index = ast_service.lock().await.alt_index;
            let defs = crate::ast::alt_db::definitions(alt_index, symbol.text.as_str()).await;
            let file_paths = defs.iter().map(|x| x.cpath.clone()).collect::<Vec<_>>();
            let text = if let Some(path0) = file_paths.get(0) {
                let path = PathBuf::from(path0);
                let file_name = path.file_name().unwrap_or(OsStr::new(path0)).to_string_lossy();
                if file_paths.len() > 1 {
                    format!("`{}` (defined in {} and other files)", &symbol.text, file_name)
                } else {
                    format!("`{}` (defined in {})", &symbol.text, file_name)
                }
            } else {
                format!("`{}` (definition not found in the AST tree)", &symbol.text)
            };
            let mut result = vec![];
            for res in &defs {
                let file_name = res.cpath.clone();
                let content = res.get_content_from_file().await.unwrap_or("".to_string());
                result.push(ContextFile {
                    file_name,
                    file_content: content,
                    line1: res.full_range.start_point.row + 1,
                    line2: res.full_range.end_point.row + 1,
                    symbols: vec![res.path()],
                    gradient_type: -1,
                    usefulness: 100.0,
                    is_body_important: false
                });
            }
            Ok((result.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>(), text))
        } else {
            Err("attempt to use @definition with no ast turned on".to_string())
        }
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
