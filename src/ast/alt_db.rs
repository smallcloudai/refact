use sled::{Db, IVec};
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use crate::ast::alt_minimalistic::{AltIndex, AltState, AltDefinition};


async fn alt_index_init()
{
    let db: Db = sled::open("my_db").unwrap();
    // index: Arc<AMutex<AltIndex>>
}

async fn doc_add(index: Arc<AMutex<AltIndex>>, cpath: &String, text: &String)
{
}

async fn doc_remove(index: Arc<AMutex<AltIndex>>, cpath: &String)
{
}

// async fn doc_symbols(index: Arc<AMutex<AltState>>, cpath: &String) -> Vec<Arc<AltDefinition>>
// {
// }

// async fn doc_symbols(index: Arc<AMutex<AltState>>, cpath: &String) -> Vec<Arc<AltDefinition>>
// {
// }
