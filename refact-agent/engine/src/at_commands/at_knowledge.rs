use std::sync::Arc;
use std::collections::HashSet;
use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::memories::memories_search;

pub struct AtLoadKnowledge {
    params: Vec<Box<dyn AtParam>>,
}

impl AtLoadKnowledge {
    pub fn new() -> Self {
        AtLoadKnowledge { params: vec![] }
    }
}

#[async_trait]
impl AtCommand for AtLoadKnowledge {
    fn params(&self) -> &Vec<Box<dyn AtParam>> {
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

        let search_key = args.iter().map(|x| x.text.clone()).join(" ");
        let gcx = ccx.lock().await.global_context.clone();

        let memories = memories_search(gcx, &search_key, 5, 0).await?;
        let mut seen_memids = HashSet::new();
        let unique_memories: Vec<_> = memories.into_iter()
            .filter(|m| seen_memids.insert(m.memid.clone()))
            .collect();

        let results = unique_memories.iter().map(|m| {
            let mut result = String::new();
            if let Some(path) = &m.file_path {
                result.push_str(&format!("📄 {}", path.display()));
                if let Some((start, end)) = m.line_range {
                    result.push_str(&format!(":{}-{}", start, end));
                }
                result.push('\n');
            }
            if let Some(title) = &m.title {
                result.push_str(&format!("📌 {}\n", title));
            }
            result.push_str(&m.content);
            result.push_str("\n\n");
            result
        }).collect::<String>();

        let context = ContextEnum::ChatMessage(ChatMessage::new("plain_text".to_string(), results));
        Ok((vec![context], "".to_string()))
    }

    fn depends_on(&self) -> Vec<String> {
        vec![]
    }
}
