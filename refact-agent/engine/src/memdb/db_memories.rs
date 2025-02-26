use itertools::Itertools;
use std::sync::Arc;
use tracing::info;

use parking_lot::Mutex as ParkMutex;
use rand::Rng;
use reqwest::Client;
use rusqlite::params;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tokio::time::Instant;

use crate::ast::chunk_utils::official_text_hashing_function;
use crate::caps::get_custom_embedding_api_key;
use crate::fetch_embedding;
use crate::global_context::GlobalContext;
use crate::memdb::db_structs::MemDB;
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{
    MemoRecord, MemoSearchResult, SimpleTextHashVector, VecDbStatus, VecdbConstants,
};
use crate::vecdb::vectorizer_service::{vectorizer_enqueue_dirty_memory, FileVectorizerService};
use zerocopy::IntoBytes;


fn map_row_to_memo_record(row: &rusqlite::Row) -> rusqlite::Result<MemoRecord> {
    Ok(MemoRecord {
        memid: row.get(0)?,
        thevec: None,
        distance: 2.0,
        m_type: row.get(1)?,
        m_goal: row.get(2)?,
        m_project: row.get(3)?,
        m_payload: row.get(4)?,
        m_origin: row.get(5)?,
        mstat_correct: row.get(6)?,
        mstat_relevant: row.get(7)?,
        mstat_times_used: row.get(8)?,
    })
}

fn fields_ordered() -> String {
    "memid,m_type,m_goal,m_project,m_payload,m_origin,mstat_correct,mstat_relevant,mstat_times_used"
        .to_string()
}

pub async fn memories_add(
    mdb: Arc<ParkMutex<MemDB>>,
    vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    mem_type: &str,
    goal: &str,
    project: &str,
    payload: &str,
    m_origin: &str,
) -> rusqlite::Result<String, String> {
    fn generate_memid() -> String {
        rand::thread_rng()
            .sample_iter(&rand::distributions::Uniform::new(0, 16))
            .take(10)
            .map(|x| format!("{:x}", x))
            .collect()
    }
    let memid = generate_memid();
    let lite = mdb.lock().lite.clone();
    lite.lock().execute(
        "INSERT INTO memories (memid, m_type, m_goal, m_project, m_payload, m_origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![memid, mem_type, goal, project, payload, m_origin],
    ).map_err(|e| e.to_string())?;
    mdb.lock().dirty_memids.push(memid.clone());
    vectorizer_enqueue_dirty_memory(vectorizer_service).await;
    Ok(memid)
}

pub async fn memories_erase(
    mdb: Arc<ParkMutex<MemDB>>,
    memid: &str,
) -> rusqlite::Result<usize, String> {
    let lite = mdb.lock().lite.clone();
    let removed_cnt = lite
        .lock()
        .execute("DELETE FROM memories WHERE memid = ?1", params![memid])
        .map_err(|e| e.to_string())?;
    Ok(removed_cnt)
}

pub async fn memories_select_all(
    mdb: Arc<ParkMutex<MemDB>>,
) -> rusqlite::Result<Vec<MemoRecord>, String> {
    let lite = mdb.lock().lite.clone();
    let query = format!("SELECT {} FROM memories", fields_ordered());
    let lite_locked = lite.lock();
    let mut stmt = lite_locked.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_row_to_memo_record)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?)
}

pub async fn memories_select_like(
    mdb: Arc<ParkMutex<MemDB>>,
    query: &String,
) -> rusqlite::Result<Vec<MemoRecord>, String> {
    let lite = mdb.lock().lite.clone();
    let query_str = format!(
        "SELECT {} FROM memories WHERE 
            m_type LIKE ? COLLATE NOCASE OR 
            m_goal LIKE ? COLLATE NOCASE OR 
            m_project LIKE ? COLLATE NOCASE OR 
            m_payload LIKE ? COLLATE NOCASE",
        fields_ordered()
    );
    let pattern = format!("%{}%", query);
    let lite_locked = lite.lock();
    let mut stmt = lite_locked.prepare(&query_str).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![pattern, pattern, pattern, pattern],
            map_row_to_memo_record,
        )
        .map_err(|e| e.to_string())?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?)
}

pub async fn memories_update(
    mdb: Arc<ParkMutex<MemDB>>,
    vectorizer_service: Arc<AMutex<FileVectorizerService>>,
    memid: &str,
    m_type: &str,
    m_goal: &str,
    m_project: &str,
    m_payload: &str,
    m_origin: &str,
) -> rusqlite::Result<usize, String> {
    let lite = mdb.lock().lite.clone();
    let updated_cnt = lite
        .lock()
        .execute(
            "UPDATE memories SET 
                   m_type=?1,
                   m_goal=?2,
                   m_project=?3,
                   m_payload=?4,
                   m_origin=?5
                 WHERE memid = ?6",
            params![m_type, m_goal, m_project, m_payload, m_origin, memid],
        )
        .map_err(|e| e.to_string())?;
    mdb.lock().dirty_memids.push(memid.to_string());
    vectorizer_enqueue_dirty_memory(vectorizer_service).await;
    Ok(updated_cnt)
}

pub async fn memories_update_used(
    mdb: Arc<ParkMutex<MemDB>>,
    memid: &str,
    mstat_correct: i32,
    mstat_relevant: i32,
) -> rusqlite::Result<usize, String> {
    let lite = mdb.lock().lite.clone();
    let updated_cnt = lite
        .lock()
        .execute(
            "UPDATE memories SET 
               mstat_times_used = mstat_times_used + 1, 
               mstat_correct = mstat_correct + ?1, 
               mstat_relevant = mstat_relevant + ?2 
             WHERE memid = ?3",
            params![mstat_correct, mstat_relevant, memid],
        )
        .map_err(|e| e.to_string())?;
    Ok(updated_cnt)
}

pub async fn memories_search(
    gcx: Arc<ARwLock<GlobalContext>>,
    query: &String,
    top_n: usize,
) -> rusqlite::Result<MemoSearchResult, String> {
    fn calculate_score(distance: f32, _times_used: i32) -> f32 {
        distance
        // distance - (times_used as f32) * 0.01
    }

    let api_key = get_custom_embedding_api_key(gcx.clone()).await;
    if let Err(err) = api_key {
        return Err(err.message);
    }
    let (lite, vecdb_emb_client, constants) = {
        let gcx_locked = gcx.read().await;
        let vecdb_locked = gcx_locked.vec_db.lock().await;
        let memdb = gcx_locked.memdb.clone().expect("memdb not initialized");
        let constants = memdb.lock().vecdb_constants.clone();
        let vecdb_emb_client = vecdb_locked
            .as_ref()
            .ok_or("VecDb is not initialized")?
            .vecdb_emb_client
            .clone();
        let x = (memdb.lock().lite.clone(), vecdb_emb_client, constants);
        x
    };

    let t0 = std::time::Instant::now();
    let embedding = fetch_embedding::get_embedding_with_retry(
        vecdb_emb_client,
        &constants.endpoint_embeddings_style,
        &constants.embedding_model,
        &constants.endpoint_embeddings_template,
        vec![query.clone()],
        &api_key.unwrap(),
        5,
    )
    .await?;
    if embedding.is_empty() {
        return Err("memdb_search: empty embedding".to_string());
    }
    info!(
        "search query {:?}, it took {:.3}s to vectorize the query",
        query,
        t0.elapsed().as_secs_f64()
    );

    let mut results = {
        let lite_locked = lite.lock();
        let t0 = Instant::now();
        let fields = fields_ordered()
            .split(',')
            .map(|x| format!("memories.{x}"))
            .join(",");
        let query = format!(
            "WITH knn_matches AS (
                SELECT memid, distance
                FROM embeddings
                WHERE embedding MATCH ?1
                    AND k = ?2
            )
            SELECT {fields},knn_matches.distance
            FROM knn_matches
            LEFT JOIN memories ON memories.memid = knn_matches.memid
            ORDER BY knn_matches.distance"
        );
        let mut stmt = lite_locked.prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![embedding[0].as_bytes(), top_n as i64], |row| {
                let mut record = map_row_to_memo_record(row)?;
                record.distance = row.get(9)?; // change it if `fields_ordered()` changes
                Ok(record)
            })
            .map_err(|e| e.to_string())?;
        let elapsed_time = t0.elapsed();
        info!(
            "search_similar_records({}) took {:.2}s",
            top_n,
            elapsed_time.as_secs_f64()
        );
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    results.sort_by(|a, b| {
        let score_a = calculate_score(a.distance, a.mstat_times_used);
        let score_b = calculate_score(b.distance, b.mstat_times_used);
        score_a
            .partial_cmp(&score_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(MemoSearchResult {
        query_text: query.clone(),
        results,
    })
}

async fn recall_dirty_memories_and_mark_them_not_dirty(
    mdb: Arc<ParkMutex<MemDB>>,
) -> rusqlite::Result<(Vec<String>, Vec<SimpleTextHashVector>), String> {
    let (query, params) = {
        let memdb_locked = mdb.lock();
        if memdb_locked.dirty_everything {
            ("SELECT memid, m_goal FROM memories".to_string(), Vec::new())
        } else if !memdb_locked.dirty_memids.is_empty() {
            let placeholders = (0..memdb_locked.dirty_memids.len())
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let query = format!(
                "SELECT memid, m_goal FROM memories WHERE memid IN ({})",
                placeholders
            );
            let params: Vec<String> = memdb_locked.dirty_memids.iter().cloned().collect();
            (query, params)
        } else {
            return Ok((Vec::new(), Vec::new()));
        }
    };

    let (memids, todo) = {
        let memdb_locked = mdb.lock();
        let lite_locked = memdb_locked.lite.lock();
        let mut stmt = lite_locked.prepare(&query).map_err(|e| e.to_string())?;
        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<(String, String)> {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        };
        let param_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_refs), map_fn)
            .map_err(|e| e.to_string())?;
        let results = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut memids: Vec<String> = Vec::with_capacity(results.len());
        let mut todo: Vec<SimpleTextHashVector> = Vec::with_capacity(results.len());
        for (memid, m_goal) in results {
            let window_text_hash = official_text_hashing_function(&m_goal);
            memids.push(memid);
            todo.push(SimpleTextHashVector {
                window_text: m_goal,
                window_text_hash,
                vector: None,
            });
        }
        Ok::<(Vec<String>, Vec<SimpleTextHashVector>), String>((memids, todo))
    }?;

    {
        let mut memdb_locked = mdb.lock();
        memdb_locked.dirty_memids.clear();
        memdb_locked.dirty_everything = false;
    }

    Ok((memids, todo))
}

pub async fn vectorize_dirty_memories(
    mdb: Arc<ParkMutex<MemDB>>,
    vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    _status: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<Client>>,
    api_key: &String,
    #[allow(non_snake_case)] B: usize,
) -> rusqlite::Result<(), String> {
    let (memids, mut todo) = recall_dirty_memories_and_mark_them_not_dirty(mdb.clone()).await?;
    if memids.is_empty() {
        return Ok(());
    }

    {
        let mut handler_locked = vecdb_handler.lock().await;
        handler_locked
            .process_simple_hash_text_vector(&mut todo)
            .await
            .map_err(|e| format!("Failed to get vectors from cache: {}", e))?
        // this makes todo[].vector appear for records that exist in cache
    }

    let todo_len = todo.len();
    let mut to_vectorize = todo
        .iter_mut()
        .filter(|x| x.vector.is_none())
        .collect::<Vec<&mut SimpleTextHashVector>>();
    info!(
        "{} memories total, {} to vectorize",
        todo_len,
        to_vectorize.len()
    );
    let my_constants: VecdbConstants = mdb.lock().vecdb_constants.clone();
    for chunk in to_vectorize.chunks_mut(B) {
        let texts: Vec<String> = chunk.iter().map(|x| x.window_text.clone()).collect();
        let embedding_mb = crate::fetch_embedding::get_embedding_with_retry(
            client.clone(),
            &my_constants.endpoint_embeddings_style,
            &my_constants.embedding_model,
            &my_constants.endpoint_embeddings_template,
            texts,
            api_key,
            1,
        )
        .await?;
        for (chunk_save, x) in chunk.iter_mut().zip(embedding_mb.iter()) {
            chunk_save.vector = Some(x.clone()); // <-- this will make the rest of todo[].vector appear
        }
    }

    {
        let mut handler_locked = vecdb_handler.lock().await;
        let temp_vec: Vec<SimpleTextHashVector> =
            to_vectorize.iter().map(|x| (**x).clone()).collect();
        handler_locked
            .cache_add_new_records(temp_vec)
            .await
            .map_err(|e| format!("Failed to update cache: {}", e))?;
    }

    let lite = mdb.lock().lite.clone();
    let lite_locked = lite.lock();
    let mut stmt = lite_locked
        .prepare("INSERT INTO embeddings(embedding, memid) VALUES (?, ?)")
        .map_err(|e| e.to_string())?;
    for (item, memid) in todo.into_iter().zip(memids) {
        let vector_bytes: Vec<u8> = item.vector
            .clone()
            .expect("No embedding is provided")
            .iter()
            .flat_map(|&num| num.to_ne_bytes())
            .collect();
            
        stmt.execute(rusqlite::params![
            vector_bytes,
            memid
        ])
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
