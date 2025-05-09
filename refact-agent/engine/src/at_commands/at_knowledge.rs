use std::sync::Arc;
use std::collections::HashSet;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, ContextFile};
use crate::vecdb::vdb_highlev::memories_search;

/// @knowledge-load command - loads knowledge entries by search key or memory ID
pub struct AtLoadKnowledge {
    params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtLoadKnowledge {
    pub fn new() -> Self {
        AtLoadKnowledge {
            params: vec![],
        }
    }
}

#[async_trait]
impl AtCommand for AtLoadKnowledge {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        _cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        if args.is_empty() {
            return Err("Usage: @knowledge-load <search_key>".to_string());
        }

        let search_key = args[0].text.clone();
        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };

        let mem_top_n = 5;
        let memories = memories_search(gcx.clone(), &search_key, mem_top_n).await?;
        let mut seen_memids = HashSet::new();
        let unique_memories: Vec<_> = memories.results.into_iter()
            .filter(|m| seen_memids.insert(m.iknow_id.clone()))
            .collect();
        if unique_memories.is_empty() {
            return Err(format!("No knowledge entries found for: {}", search_key));
        }
        let mut results = Vec::new();
        for memory in unique_memories {
            let mut content = String::new();
            content.push_str(&format!("🗃️{}\n", memory.iknow_id));
            content.push_str(&memory.iknow_memory);
            results.push(ContextEnum::ContextFile(ContextFile {
                file_name: format!("{}", memory.iknow_id),
                file_content: content,
                line1: 1,
                line2: 1,
                symbols: Vec::new(),
                gradient_type: -1,
                usefulness: 0.0
            }));
        }

        let count = results.len();
        Ok((results, format!("Loaded {} knowledge entries", count)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
