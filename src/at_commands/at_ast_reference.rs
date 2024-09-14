use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::call_validation::{ContextFile, ContextEnum};
use crate::at_commands::execute_at::{AtCommandMember, correct_at_arg};
use crate::at_commands::at_ast_definition::AtParamSymbolPathQuery;


pub struct AtAstReference {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtAstReference {
    pub fn new() -> Self {
        AtAstReference {
            params: vec![
                Arc::new(AMutex::new(AtParamSymbolPathQuery::new()))
            ],
        }
    }
}


#[async_trait]
impl AtCommand for AtAstReference {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let mut arg_symbol = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false;
                cmd.reason = Some("no symbol path".to_string());
                args.clear();
                return Err("no symbol path".to_string());
            },
        };

        correct_at_arg(ccx.clone(), self.params[0].clone(), &mut arg_symbol).await;
        args.clear();
        args.push(arg_symbol.clone());

        let gcx = ccx.lock().await.global_context.clone();
        let ast_service_opt = gcx.read().await.ast_service.clone();

        if let Some(ast_service) = ast_service_opt {
            let ast_index = ast_service.lock().await.ast_index.clone();
            let defs = crate::ast::ast_db::definitions(ast_index.clone(), arg_symbol.text.as_str()).await;
            let mut all_results = vec![];
            let mut messages = vec![];

            const USAGES_LIMIT: usize = 20;

            if let Some(def) = defs.get(0) {
                let usages: Vec<(Arc<crate::ast::ast_minimalistic::AstDefinition>, usize)> = crate::ast::ast_db::usages(ast_index.clone(), def.path(), 100).await;
                let usage_count = usages.len();

                let text = format!(
                    "symbol `{}` has {} usages",
                    arg_symbol.text,
                    usage_count
                );
                messages.push(text);

                for (usedin, uline) in usages.iter().take(USAGES_LIMIT) {
                    all_results.push(ContextFile {
                        file_name: usedin.cpath.clone(),
                        file_content: "".to_string(),
                        line1: *uline,
                        line2: *uline,
                        symbols: vec![usedin.path()],
                        gradient_type: -1,
                        usefulness: 100.0,
                        is_body_important: false
                    });
                }
                if usage_count > USAGES_LIMIT {
                    messages.push(format!("...and {} more usages", usage_count - USAGES_LIMIT));
                }
            } else {
                messages.push("No definitions found for the symbol".to_string());
            }

            Ok((all_results.into_iter().map(|x| ContextEnum::ContextFile(x)).collect::<Vec<ContextEnum>>(), messages.join("\n")))
        } else {
            Err("attempt to use @references with no ast turned on".to_string())
        }
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}
