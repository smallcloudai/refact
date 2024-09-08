use sled::{Db, IVec};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use tokio::task;
use crate::ast::alt_minimalistic::{AltIndex, AltState, AltDefinition};
use crate::ast::alt_parse_anything::parse_anything_and_add_file_path;
use serde_cbor;

async fn alt_index_init() -> Arc<AMutex<AltIndex>> {
    let db: Arc<AMutex<Db>> = Arc::new(AMutex::new(task::spawn_blocking(|| sled::open("/tmp/my_db.sled").unwrap()).await.unwrap()));
    {
        let db_locked = db.lock().await;
        db_locked.clear().unwrap();
    }
    let index = AltIndex {
        sleddb: db,
    };
    Arc::new(AMutex::new(index))
}

async fn doc_add(index: Arc<AMutex<AltIndex>>, cpath: &String, text: &String) {
    let definitions = parse_anything_and_add_file_path(cpath, text);
    let db = index.lock().await.sleddb.clone();
    let db_locked = db.lock().await;
    for (guid, definition) in definitions {
        let serialized = serde_cbor::to_vec(&definition).unwrap();
        db_locked.insert(guid.as_bytes(), serialized).unwrap();
    }
}

async fn doc_remove(index: Arc<AMutex<AltIndex>>, cpath: &String) {
}

async fn connect_everything(index: Arc<AMutex<AltIndex>>)
{
}

async fn dump_database(index: Arc<AMutex<AltIndex>>) {
    let db = index.lock().await.sleddb.clone();
    let db_locked = db.lock().await;

    let iter = db_locked.iter();
    for item in iter {
        let (key, value) = item.unwrap();
        let guid = Uuid::from_slice(&key).unwrap();
        match serde_cbor::from_slice::<AltDefinition>(&value) {
            Ok(definition) => println!("{} {:?}", guid, definition),
            Err(e) => println!("Failed to deserialize value for GUID {}: {:?}", guid, e),
        }
    }
}

// async fn doc_symbols(index: Arc<AMutex<AltState>>, cpath: &String) -> Vec<Arc<AltDefinition>>
// {
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    #[tokio::test]
    async fn test_alt_db() {
        let index = alt_index_init().await;

        let cpp_library_path = "src/ast/alt_testsuite/cpp_goat_library.h";
        let cpp_library_text = read_file(cpp_library_path);
        doc_add(index.clone(), &cpp_library_path.to_string(), &cpp_library_text).await;

        let cpp_main_path = "src/ast/alt_testsuite/cpp_goat_main.cpp";
        let cpp_main_text = read_file(cpp_main_path);
        doc_add(index.clone(), &cpp_main_path.to_string(), &cpp_main_text).await;

        connect_everything(index.clone()).await;

        dump_database(index.clone()).await;
    }
}
