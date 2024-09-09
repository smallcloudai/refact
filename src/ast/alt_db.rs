use sled::{Db, IVec};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::task;
use crate::ast::alt_minimalistic::{AltIndex, AltState, AltDefinition};
use crate::ast::alt_parse_anything::{parse_anything_and_add_file_path, filesystem_path_to_double_colon_path};
use serde_cbor;

async fn alt_index_init() -> Arc<AMutex<AltIndex>>
{
    let db: Arc<Db> = Arc::new(task::spawn_blocking(|| sled::open("/tmp/my_db.sled").unwrap()).await.unwrap());
    db.clear().unwrap();
    // db.open_tree(b"unprocessed items").unwrap();
    let altindex = AltIndex {
        sleddb: db,
    };
    Arc::new(AMutex::new(altindex))
}

// ## How the database works ##
//
// Database `sled` used here is a key-value storage, everything is stored as keys and values. Try dump_database() below.
//
// All the definitions are serialized under d/ like this:
//   d/alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice
//   AltDefinition { alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice, usages: Link{ up alt_testsuite::cpp_goat_main::CosmicJustice::balance } }
//
// You can look up a shorter path than the full path, by using c/ records:
//   c/main::goat1 ⚡ alt_testsuite::cpp_goat_main::main::goat1
//     ^^^^^^^^^^^ short path that maps to full path
//
// Usages are stored as:
//   u/CosmicJustice::balance ⚡ alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice
//     ^^^^^^^^^^^^^^^^^^^^^^ usage of what? (short path)
//                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path to where the usage is
//
// Read tests below, the show what this index can do!
//

async fn doc_add(altindex: Arc<AMutex<AltIndex>>, cpath: &String, text: &String)
{
    let definitions = parse_anything_and_add_file_path(cpath, text);
    let db = altindex.lock().await.sleddb.clone();
    let mut batch = sled::Batch::default();
    for definition in definitions.values() {
        let serialized = serde_cbor::to_vec(&definition).unwrap();
        let official_path = definition.official_path.join("::");
        let d_key = format!("d/{}", official_path);
        batch.insert(d_key.as_bytes(), serialized);
        let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
        while !path_parts.is_empty() {
            let c_key = format!("c/{} ⚡ {}", path_parts.join("::"), official_path);
            batch.insert(c_key.as_bytes(), b"huu");
            path_parts.remove(0);
        }
        for usage in &definition.usages {
            let mut usage_parts: Vec<&str> = usage.target_for_guesswork.iter().map(|s| s.as_str()).collect();
            if let Some(first) = usage_parts.first() {
                if *first == "?" {
                    usage_parts.remove(0);
                }
            }
            while !usage_parts.is_empty() {
                let u_key = format!("u/{} ⚡ {}", usage_parts.join("::"), official_path);
                batch.insert(u_key.as_bytes(), b"huu");
                usage_parts.remove(0);
            }
        }
    }
    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("doc_add() failed to apply batch: {:?}", e);
    }
}

async fn doc_remove(altindex: Arc<AMutex<AltIndex>>, cpath: &String)
{
    let to_delete_prefix = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d/{}", to_delete_prefix.join("::"));
    let db = altindex.lock().await.sleddb.clone();
    let mut batch = sled::Batch::default();
    let mut iter = db.scan_prefix(d_prefix);
    while let Some(Ok((key, value))) = iter.next() {
        let d_key_b = key.clone();
        if let Ok(definition) = serde_cbor::from_slice::<AltDefinition>(&value) {
            let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
            while !path_parts.is_empty() {
                let c_key = format!("c/{} ⚡ {}", path_parts.join("::"), definition.official_path.join("::"));
                batch.remove(c_key.as_bytes());
                path_parts.remove(0);
            }
            for usage in &definition.usages {
                let mut usage_parts: Vec<&str> = usage.target_for_guesswork.iter().map(|s| s.as_str()).collect();
                if let Some(first) = usage_parts.first() {
                    if *first == "?" {
                        usage_parts.remove(0);
                    }
                }
                while !usage_parts.is_empty() {
                    let u_key = format!("u/{} ⚡ {}", usage_parts.join("::"), definition.official_path.join("::"));
                    batch.remove(u_key.as_bytes());
                    usage_parts.remove(0);
                }
            }
        }
        batch.remove(&d_key_b);
    }
    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("doc_remove() failed to apply batch: {:?}", e);
    }
}

async fn doc_symbols(altindex: Arc<AMutex<AltState>>, cpath: &String) -> Vec<Arc<AltDefinition>>
{
    let to_search_prefix = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d/{}", to_search_prefix.join("::"));
    let db = altindex.lock().await.alt_index.lock().await.sleddb.clone();
    let mut definitions = Vec::new();
    let mut iter = db.scan_prefix(d_prefix);
    while let Some(Ok((_, value))) = iter.next() {
        if let Ok(definition) = serde_cbor::from_slice::<AltDefinition>(&value) {
            definitions.push(Arc::new(definition));
        }
    }
    definitions
}

async fn connect_everything(altindex: Arc<AMutex<AltIndex>>)
{
}

async fn dump_database(altindex: Arc<AMutex<AltIndex>>)
{
    let db = altindex.lock().await.sleddb.clone();
    println!("\nsled has {} reconds", db.len());
    let iter = db.iter();
    for item in iter {
        let (key, value) = item.unwrap();
        let key_string = String::from_utf8(key.to_vec()).unwrap(); // Convert key to String
        if key_string.starts_with("d/") { // Check if the key is a d_key
            match serde_cbor::from_slice::<AltDefinition>(&value) {
                Ok(definition) => println!("{}\n{:?}", key_string, definition),
                Err(e) => println!("Failed to deserialize value at {}: {:?}", key_string, e),
            }
        }
        if key_string.starts_with("c/") {
            println!("{}", key_string);
        }
        if key_string.starts_with("u/") {
            println!("{}", key_string);
        }
    }
}

// pub async fn usages(altindex: Arc<AMutex<AltIndex>>, double_colon_path: String) -> Vec<String>
// {
// }

pub async fn definitions(altindex: Arc<AMutex<AltIndex>>, double_colon_path: &str) -> Vec<Arc<AltDefinition>>
{
    let db = altindex.lock().await.sleddb.clone();
    let c_prefix = format!("c/{}", double_colon_path);
    let mut definitions = Vec::new();
    println!("definitions(c_prefix={:?})", c_prefix);
    let mut iter = db.scan_prefix(&c_prefix);
    while let Some(Ok((key, _))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        if key_string.contains(" ⚡ ") {
            let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
            if parts.len() == 2 && parts[0] == c_prefix {
                let full_path = parts[1].trim();
                let d_key = format!("d/{}", full_path);
                if let Ok(Some(d_value)) = db.get(d_key.as_bytes()) {
                    match serde_cbor::from_slice::<AltDefinition>(&d_value) {
                        Ok(definition) => definitions.push(Arc::new(definition)),
                        Err(e) => println!("Failed to deserialize value for {}: {:?}", d_key, e),
                    }
                }
            } else {
                tracing::error!("usage record has more than two ⚡ key was: {}", key_string);
            }
        } else {
            tracing::error!("usage record doesn't have ⚡ key was: {}", key_string);
        }
    }
    definitions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    #[tokio::test]
    async fn test_alt_db() {
        let altindex = alt_index_init().await;

        let cpp_library_path = "src/ast/alt_testsuite/cpp_goat_library.h";
        let cpp_library_text = read_file(cpp_library_path);
        doc_add(altindex.clone(), &cpp_library_path.to_string(), &cpp_library_text).await;

        let cpp_main_path = "src/ast/alt_testsuite/cpp_goat_main.cpp";
        let cpp_main_text = read_file(cpp_main_path);
        doc_add(altindex.clone(), &cpp_main_path.to_string(), &cpp_main_text).await;

        connect_everything(altindex.clone()).await;

        dump_database(altindex.clone()).await;
        let goat_def = definitions(altindex.clone(), "Goat").await;
        let mut definitions_str = String::new();
        for def in goat_def {
            definitions_str.push_str(&format!("{:?}\n", def));
        }
        println!("Definitions:\n{}", definitions_str);

        // doc_remove(altindex.clone(), &cpp_library_path.to_string()).await;
        // doc_remove(altindex.clone(), &cpp_main_path.to_string()).await;

        // dump_database(altindex.clone()).await;
    }
}
