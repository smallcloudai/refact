use std::collections::HashMap;
use serde_json::Value;
use tracing::info;
use indexmap::IndexMap;

use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum};
use crate::vecdb::vdb_highlev::{memories_search, ongoing_find};


pub struct AttGetKnowledge;


#[async_trait]
impl Tool for AttGetKnowledge {
    async fn tool_execute(&mut self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        info!("run @get-knowledge {:?}", args);
        let im_going_to_do = match args.get("im_going_to_do") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => { return Err(format!("argument `im_going_to_do` is not a string: {:?}", v)) },
            None => { return Err("argument `im_going_to_do` is missing".to_string()) }
        };

        let vec_db = ccx.global_context.read().await.vec_db.clone();
        let memories: crate::vecdb::vdb_structs::MemoSearchResult = memories_search(vec_db.clone(), &im_going_to_do, ccx.top_n).await?;

        // TODO: verify it's valid json in payload when accepting the mem into db
        let memories_json = memories.results.iter().map(|m| {
            let payload: serde_json::Value = serde_json::from_str(&m.m_payload).unwrap_or(Value::Null);
            serde_json::json!({
                "memid": m.memid.clone(),
                "content": payload,
            })
        }).collect::<Vec<Value>>();
        let memories_str = serde_json::to_string_pretty(&memories_json).unwrap();

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: memories_str,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        let ongoing_maybe: Option<crate::vecdb::vdb_structs::OngoingWork> = ongoing_find(vec_db.clone(), im_going_to_do.clone()).await?;
        if let Some(ongoing) = ongoing_maybe {
            let mut toplevel = IndexMap::new();
            toplevel.insert("PROGRESS".to_string(), serde_json::Value::Object(ongoing.ongoing_progress.into_iter().collect()));
            let action_sequences: Vec<Value> = ongoing.ongoing_action_sequences
                .into_iter()
                .map(|map| serde_json::Value::Object(map.into_iter().collect()))
                .collect();
            toplevel.insert("TRIED_ACTION_SEQUENCES".to_string(), serde_json::Value::Array(action_sequences));
            let output_value: serde_json::Value = indexmap_to_json_value(
                ongoing.ongoing_output
                    .into_iter()
                    .map(|(k, v)| (k, indexmap_to_json_value(v)))
                    .collect()
            );
            toplevel.insert("OUTPUT".to_string(), output_value);
            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "user".to_string(),
                content: format!("💿 An ongoing session with this goal is found, it's your attempt {}. Here is the summary of your progress. Read it and follow the system prompt, especially pay attention to strategy choice:\n\n{}",
                    ongoing.ongoing_attempt_n + 1,
                    serde_json::to_string_pretty(&toplevel).unwrap()
                ),
                tool_calls: None,
                tool_call_id: String::new(),
                ..Default::default()
            }));
        } else {
            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "user".to_string(),
                content: format!("💿 There is no ongoing session with this goal. A new empty ongoing session is created, this is your attempt 1."),
                tool_calls: None,
                tool_call_id: String::new(),
                ..Default::default()
            }));
        }

        Ok(results)
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec!["vecdb".to_string()]
    }
}

fn indexmap_to_json_value(map: IndexMap<String, serde_json::Value>) -> Value {
    Value::Object(serde_json::Map::from_iter(
        map.into_iter().map(|(k, v)| {
            (k, match v {
                Value::Object(o) => indexmap_to_json_value(IndexMap::from_iter(o)),
                _ => v,
            })
        })
    ))
}


// pub struct AttSaveKnowledge;
// #[async_trait]
// impl Tool for AttSaveKnowledge {
//     async fn execute(&self, ccx: &mut AtCommandsContext, tool_call_id: &String, args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
//         info!("run @save-knowledge {:?}", args);
//         let memory_topic = match args.get("memory_topic") {
//             Some(Value::String(s)) => s,
//             _ => return Err("argument `memory_topic` is missing or not a string".to_string()),
//         };
//         let memory_text = match args.get("memory_text") {
//             Some(Value::String(s)) => s,
//             _ => return Err("argument `memory_text` is missing or not a string".to_string()),
//         };
//         let memory_type = match args.get("memory_type") {
//             Some(Value::String(s)) => s,
//             _ => return Err("argument `memory_type` is missing or not a string".to_string()),
//         };
//         if !["consequence", "reflection", "familiarity", "relationship"].contains(&memory_type.as_str()) {
//             return Err(format!("Invalid memory_type: {}. Must be one of: consequence, reflection, familiarity, relationship", memory_type));
//         }
//         let memdb = {
//             let vec_db = ccx.global_context.read().await.vec_db.clone();
//             let vec_db_guard = vec_db.lock().await;
//             let vec_db_ref = vec_db_guard.as_ref().ok_or("vecdb is not available".to_string())?;
//             vec_db_ref.memdb.clone()
//         };
//         let _memid = memdb.lock().await.permdb_add(memory_type, memory_topic, "current_project", memory_text)?;
//         let mut results = vec![];
//         results.push(ContextEnum::ChatMessage(ChatMessage {
//             role: "tool".to_string(),
//             content: format!("Model will remember it:\n{memory_text}"),
//             tool_calls: None,
//             tool_call_id: tool_call_id.clone(),
//         }));
//         Ok(results)
//     }
//     fn depends_on(&self) -> Vec<String> {
//         vec!["vecdb".to_string()]
//     }
// }
