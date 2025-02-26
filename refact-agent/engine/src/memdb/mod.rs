pub mod db_chore;
pub mod db_cmessage;
pub mod db_cthread;
pub mod db_init;
pub mod db_schema;
pub mod db_structs;
pub mod db_memories;
pub mod db_pubsub;



// TODO agent: move it to another more suitable file
pub fn merge_json(a: &mut serde_json::Value, b: &serde_json::Value) {
    // if let serde_json::Value::Object(_) = b {
    //     tracing::info!("merging json:\n{:#?}", b);
    // }
    match (a, b) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                // yay, it's recursive!
                merge_json(a.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

