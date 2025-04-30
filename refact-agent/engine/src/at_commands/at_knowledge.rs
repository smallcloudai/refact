use std::sync::Arc;
use std::collections::HashSet;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ContextEnum, ContextFile};
use crate::vecdb::vdb_highlev::{memories_search, memories_select_all};
use crate::vecdb::vdb_structs::MemoRecord;

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
            return Err("Usage: @knowledge-load <search_key_or_memid>".to_string());
        }

        let search_key_or_memid = args[0].text.clone();
        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };

        // TODO: memories_select_all -> memories_select_by_memid (after we merge choredb + memdb combination)
        let vec_db = gcx.read().await.vec_db.clone();
        let all_memories = memories_select_all(vec_db.clone()).await?;
        let memory_by_id = all_memories.iter()
            .find(|m| m.memid == search_key_or_memid);
        if let Some(memory) = memory_by_id {
            let mut result = String::new();
            result.push_str(&format!("🗃️{}\n", memory.memid));
            result.push_str(&memory.m_payload);
            return Ok((vec![ContextEnum::ContextFile(ContextFile {
                file_name: format!("knowledge/{}.md", memory.memid),
                file_content: result,
                line1: 1,
                line2: 1,
                symbols: Vec::new(),
                gradient_type: -1,
                usefulness: 0.0
            })], "Knowledge entry loaded".to_string()));
        }
        
        // If not a memory ID, treat as a search key
        let mem_top_n = 5;
        let memories = memories_search(gcx.clone(), &search_key_or_memid, mem_top_n).await?;
        let mut seen_memids = HashSet::new();
        let unique_memories: Vec<_> = memories.results.into_iter()
            .filter(|m| seen_memids.insert(m.memid.clone()))
            .collect();
        if unique_memories.is_empty() {
            return Err(format!("No knowledge entries found for: {}", search_key_or_memid));
        }
        let mut results = Vec::new();
        for memory in unique_memories {
            let mut content = String::new();
            content.push_str(&format!("🗃️{}\n", memory.memid));
            content.push_str(&memory.m_payload);
            results.push(ContextEnum::ContextFile(ContextFile {
                file_name: format!("knowledge/{}.md", memory.memid),
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

/// @knowledge-load-last command - loads the most recent knowledge entries
pub struct AtLoadLastKnowledge {
    params: Vec<Arc<AMutex<dyn AtParam>>>,
}

impl AtLoadLastKnowledge {
    pub fn new() -> Self {
        AtLoadLastKnowledge {
            params: vec![],
        }
    }
}

#[async_trait]
impl AtCommand for AtLoadLastKnowledge {
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        _cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let count = if !args.is_empty() {
            args[0].text.parse::<usize>().unwrap_or(5)
        } else {
            5 // Default to 5 entries
        };

        let gcx = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.global_context.clone()
        };

        let vec_db = gcx.read().await.vec_db.clone();
        let all_memories = memories_select_all(vec_db.clone()).await?;
        
        // Sort by memory ID (assuming newer entries have higher IDs)
        // This is a simplification - in a real system you might want to sort by timestamp
        let mut sorted_memories: Vec<MemoRecord> = all_memories.clone();
        sorted_memories.sort_by(|a, b| b.memid.cmp(&a.memid));
        
        // Take only the requested number of entries
        let recent_memories = sorted_memories.into_iter().take(count).collect::<Vec<_>>();
        
        if recent_memories.is_empty() {
            return Err("No knowledge entries found".to_string());
        }

        let mut results = Vec::new();
        for memory in recent_memories {
            let mut content = String::new();
            content.push_str(&format!("🗃️{}\n", memory.memid));
            content.push_str(&memory.m_payload);
            
            results.push(ContextEnum::ContextFile(ContextFile {
                file_name: format!("knowledge/{}.md", memory.memid),
                file_content: content,
                line1: 1,
                line2: 1,
                symbols: Vec::new(),
                gradient_type: -1,
                usefulness: 0.0
            }));
        }

        let count = results.len();
        Ok((results, format!("Loaded {} most recent knowledge entries", count)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}
