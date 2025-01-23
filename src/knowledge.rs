use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::sync::Arc;
use itertools::Itertools;
use tracing::info;

use rand::Rng;
use reqwest::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AMutex, Notify};
use tokio::time::Instant;
use tokio_rusqlite::Connection;

use zerocopy::IntoBytes;
use crate::ast::chunk_utils::official_text_hashing_function;
use crate::vecdb::vdb_sqlite::VecDBSqlite;
use crate::vecdb::vdb_structs::{MemoRecord, SimpleTextHashVector, VecDbStatus, VecdbConstants};


pub struct MemoriesDatabase {
    pub conn: Arc<AMutex<Connection>>,
    pub vecdb_constants: VecdbConstants,
    pub dirty_memids: Vec<String>,
    pub dirty_everything: bool,
    pub pubsub_notifier: Arc<Notify>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemdbSubEvent {
    pub pubevent_id: i64,
    pub pubevent_action: String,
    pub pubevent_memid: String,
    pub pubevent_json: String,
}

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
    "memid,m_type,m_goal,m_project,m_payload,m_origin,mstat_correct,mstat_relevant,mstat_times_used".to_string()
}

async fn setup_db(conn: &Connection, pubsub_notifier: Arc<Notify>) -> Result<(), String> {
    extern "C" fn pubsub_trigger_hook(
        user_data: *mut c_void,
        action: c_int,
        db_name: *const std::os::raw::c_char,
        table_name: *const std::os::raw::c_char,
        _: i64,
    ) {
        let notify = unsafe { &*(user_data as *const Notify) };
        // Use c_char which is platform dependent (i8 or u8)
        let db_name = unsafe { std::ffi::CStr::from_ptr(db_name as *const std::os::raw::c_char).to_str().unwrap_or("unknown") };
        let table_name = unsafe { std::ffi::CStr::from_ptr(table_name as *const std::os::raw::c_char).to_str().unwrap_or("unknown") };
        let operation = match action {
            18 => "INSERT",
            9 => "DELETE",
            23 => "UPDATE",
            _ => "UNKNOWN",
        };
        if db_name != "main" && table_name != "pubsub_events" {
            return;
        }
        info!("memdb pubsub {} action triggered", operation);
        notify.notify_one();
    }
    conn.call(move |conn| {
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.execute_batch("PRAGMA cache_size = 0; PRAGMA shared_cache = OFF;")?;
        let _: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        unsafe {
            libsqlite3_sys::sqlite3_update_hook(
                conn.handle(),
                Some(pubsub_trigger_hook),
                Arc::into_raw(pubsub_notifier.clone()) as *mut c_void,
            );
        }
        Ok(())
    }).await.map_err(|err| err.to_string())
}

async fn migrate_202501(conn: &Connection, embedding_size: i32, reset_memory: bool) -> rusqlite::Result<(), String> {
    conn.call(move |conn| {
        if reset_memory {
            conn.execute("DROP TABLE IF EXISTS memories", [])?;
        }
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                memid TEXT PRIMARY KEY,
                m_type TEXT NOT NULL,
                m_goal TEXT NOT NULL,
                m_project TEXT NOT NULL,
                m_payload TEXT NOT NULL,
                m_origin TEXT NOT NULL,
                mstat_correct REAL NOT NULL DEFAULT 0,
                mstat_relevant REAL NOT NULL DEFAULT 0,
                mstat_times_used INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pubsub_events (
                pubevent_id INTEGER PRIMARY KEY AUTOINCREMENT,
                pubevent_action TEXT NOT NULL,
                pubevent_memid TEXT NOT NULL,
                pubevent_json TEXT NOT NULL,
                pubevent_ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Trigger for INSERT actions
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_on_insert
            AFTER INSERT ON memories
            BEGIN
                INSERT INTO pubsub_events (pubevent_action, pubevent_memid, pubevent_json)
                VALUES ('INSERT', NEW.memid, json_object(
                    'memid', NEW.memid,
                    'm_type', NEW.m_type,
                    'm_goal', NEW.m_goal,
                    'm_project', NEW.m_project,
                    'm_payload', NEW.m_payload,
                    'm_origin', NEW.m_origin,
                    'mstat_correct', NEW.mstat_correct,
                    'mstat_relevant', NEW.mstat_relevant,
                    'mstat_times_used', NEW.mstat_times_used
                ));
            END;",
            [],
        )?;

        // Trigger for UPDATE actions
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_on_update
            AFTER UPDATE ON memories
            BEGIN
                INSERT INTO pubsub_events (pubevent_action, pubevent_memid, pubevent_json)
                VALUES ('UPDATE', NEW.memid, json_object(
                    'memid', NEW.memid,
                    'm_type', NEW.m_type,
                    'm_goal', NEW.m_goal,
                    'm_project', NEW.m_project,
                    'm_payload', NEW.m_payload,
                    'm_origin', NEW.m_origin,
                    'mstat_correct', NEW.mstat_correct,
                    'mstat_relevant', NEW.mstat_relevant,
                    'mstat_times_used', NEW.mstat_times_used
                ));
            END;",
            [],
        )?;

        // Trigger for DELETE actions
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_on_delete
            AFTER DELETE ON memories
            BEGIN
                INSERT INTO pubsub_events (pubevent_action, pubevent_memid, pubevent_json)
                VALUES ('DELETE', OLD.memid, json_object(
                    'memid', OLD.memid,
                    'm_type', OLD.m_type,
                    'm_goal', OLD.m_goal,
                    'm_project', OLD.m_project,
                    'm_payload', OLD.m_payload,
                    'm_origin', OLD.m_origin,
                    'mstat_correct', OLD.mstat_correct,
                    'mstat_relevant', OLD.mstat_relevant,
                    'mstat_times_used', OLD.mstat_times_used
                ));
            END;",
            [],
        )?;

        // Trigger to delete old events
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_delete_old
            AFTER INSERT ON pubsub_events
            BEGIN
                DELETE FROM pubsub_events WHERE pubevent_ts <= datetime('now', '-15 minutes');
            END;",
            [],
        )?;

        // Embeddings
        conn.execute("DROP TABLE IF EXISTS embeddings", [])?;
        conn.execute(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS embeddings using vec0(
              embedding float[{embedding_size}] distance_metric=cosine,
              +memid text
            );"),
                     [],
        )?;

        // Trigger to delete linked memids
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS embeddings_delete_old
            AFTER DELETE ON memories
            BEGIN
                DELETE FROM embeddings WHERE memid = OLD.memid;
            END;",
            [],
        )?;

        Ok(())
    }).await.map_err(|e| e.to_string())
}

impl MemoriesDatabase {
    pub async fn init(
        config_dir: &PathBuf,
        constants: &VecdbConstants,
        reset_memory: bool,
    ) -> rusqlite::Result<MemoriesDatabase, String> {
        let dbpath = config_dir.join("memories.sqlite");
        let pubsub_notifier = Arc::new(Notify::new());
        let conn = Connection::open_with_flags(
            dbpath,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        ).await.map_err(|err| format!("Failed to open database: {}", err))?;
        setup_db(&conn, pubsub_notifier.clone()).await?;
        migrate_202501(&conn, constants.embedding_size, reset_memory).await?;

        let db = MemoriesDatabase {
            conn: Arc::new(AMutex::new(conn)),
            vecdb_constants: constants.clone(),
            dirty_memids: Vec::new(),
            dirty_everything: true,
            pubsub_notifier,
        };

        Ok(db)
    }

    pub async fn permdb_add(
        &self,
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

        let conn = self.conn.lock().await;
        let memid = generate_memid();
        let memid_owned = memid.clone();
        let mem_type_owned = mem_type.to_string();
        let goal_owned = goal.to_string();
        let project_owned = project.to_string();
        let payload_owned = payload.to_string();
        let m_origin_owned = m_origin.to_string();
        conn.call(move |conn| {
            conn.execute(
                "INSERT INTO memories (memid, m_type, m_goal, m_project, m_payload, m_origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![memid_owned, mem_type_owned, goal_owned, project_owned, payload_owned, m_origin_owned],
            )?;
            Ok(())
        }).await.map_err(|e| e.to_string())?;
        Ok(memid)
    }

    pub async fn permdb_erase(&mut self, memid: &str) -> rusqlite::Result<usize, String> {
        let conn = self.conn.lock().await;
        let memid_owned = memid.to_string();
        conn.call(move |conn| {
            let count: usize = conn.execute(
                "DELETE FROM memories WHERE memid = ?1",
                params![memid_owned],
            )?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn permdb_update(
        &mut self,
        memid: &str,
        mem_type: &str,
        goal: &str,
        project: &str,
        payload: &str,
        m_origin: &str,
    ) -> rusqlite::Result<usize, String> {
        let conn = self.conn.lock().await;
        let mem_type_owned = mem_type.to_string();
        let goal_owned = goal.to_string();
        let project_owned = project.to_string();
        let payload_owned = payload.to_string();
        let m_origin_owned = m_origin.to_string();
        let memid_owned = memid.to_string();
        conn.call(move |conn| {
            let count: usize = conn.execute(
                "UPDATE memories SET 
                       m_type=?1,
                       m_goal=?2,
                       m_project=?3,
                       m_payload=?4,
                       m_origin=?5
                     WHERE memid = ?6",
                params![mem_type_owned, goal_owned, project_owned, payload_owned, m_origin_owned, memid_owned],
            )?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }


    pub async fn permdb_update_used(&self, memid: &str, mstat_correct: i32, mstat_relevant: i32) -> rusqlite::Result<usize, String> {
        let conn = self.conn.lock().await;
        let memid_owned = memid.to_string();        
        conn.call(move |conn| {
            let count: usize = conn.execute(
                "UPDATE memories SET 
                       mstat_times_used = mstat_times_used + 1, 
                       mstat_correct = mstat_correct + ?1, 
                       mstat_relevant = mstat_relevant + ?2 
                     WHERE memid = ?3",
                params![mstat_correct, mstat_relevant, memid_owned],
            )?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn permdb_select_all(&self) -> rusqlite::Result<Vec<MemoRecord>, String> {
        let conn = self.conn.lock().await;
        let query = format!("SELECT {} FROM memories", fields_ordered());
        conn.call(move |conn| {
            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], map_row_to_memo_record)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn permdb_select_like(&self, query: &String) -> rusqlite::Result<Vec<MemoRecord>, String> {
        let conn = self.conn.lock().await;
        let query_str = format!(
            "SELECT {} FROM memories WHERE 
                m_type LIKE ? COLLATE NOCASE OR 
                m_goal LIKE ? COLLATE NOCASE OR 
                m_project LIKE ? COLLATE NOCASE OR 
                m_payload LIKE ? COLLATE NOCASE",
            fields_ordered()
        );
        let pattern = format!("%{}%", query);

        conn.call(move |conn| {
            let mut stmt = conn.prepare(&query_str)?;
            let rows = stmt.query_map(params![pattern, pattern, pattern, pattern], map_row_to_memo_record)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn search_similar_records(&self, embedding: &Vec<f32>, top_n: usize) -> rusqlite::Result<Vec<MemoRecord>, String> {
        let t0 = Instant::now();
        let conn = self.conn.lock().await;

        let embedding_owned = embedding.clone();
        conn.call(move |conn| {
            let fields = fields_ordered().split(',').map(|x| format!("memories.{x}")).join(",");
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
            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(params![embedding_owned.as_bytes(), top_n as i64], |row| {
                let mut record = map_row_to_memo_record(row)?;
                record.distance = row.get(9)?;  // change it if `fields_ordered()` changes
                Ok(record)
            })?;
            let results = rows.collect::<Result<Vec<_>, _>>()?;

            let elapsed_time = t0.elapsed();
            info!("search_similar_records({}) took {:.2}s", top_n, elapsed_time.as_secs_f64());

            Ok(results)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn permdb_sub_select_all(&self, from_memid: Option<i64>) -> rusqlite::Result<Vec<MemdbSubEvent>, String> {
        let conn = self.conn.lock().await;
        let query = "
            SELECT pubevent_id, pubevent_action, pubevent_memid, pubevent_json
            FROM pubsub_events
            WHERE pubevent_id > ?1
            ORDER BY pubevent_id ASC
        ";
        let from_id = from_memid.unwrap_or(0);
        conn.call(move |conn| {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map([from_id], |row| {
                Ok(MemdbSubEvent {
                    pubevent_id: row.get(0)?,
                    pubevent_action: row.get(1)?,
                    pubevent_memid: row.get(2)?,
                    pubevent_json: row.get(3)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        }).await.map_err(|e| e.to_string())
    }
}

async fn recall_dirty_memories_and_mark_them_not_dirty(
    memdb: Arc<AMutex<MemoriesDatabase>>,
) -> rusqlite::Result<(Vec<String>, Vec<SimpleTextHashVector>), String> {
    let (query, params) = {
        let memdb_locked = memdb.lock().await;
        if memdb_locked.dirty_everything {
            ("SELECT memid, m_goal FROM memories".to_string(), Vec::new())
        } else if !memdb_locked.dirty_memids.is_empty() {
            let placeholders = (0..memdb_locked.dirty_memids.len())
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let query = format!("SELECT memid, m_goal FROM memories WHERE memid IN ({})", placeholders);
            let params: Vec<String> = memdb_locked.dirty_memids.iter().cloned().collect();
            (query, params)
        } else {
            return Ok((Vec::new(), Vec::new()));
        }
    };

    let (memids, todo) = {
        let memdb_locked = memdb.lock().await;
        let conn = memdb_locked.conn.lock().await;
        conn.call(move |conn| {
            let mut stmt = conn.prepare(&query)?;

            let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<(String, String)> {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            };
            let param_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
            let rows = stmt.query_map(rusqlite::params_from_iter(param_refs), map_fn)?;

            let results = rows.collect::<Result<Vec<_>, _>>()?;
            let mut memids = Vec::with_capacity(results.len());
            let mut todo = Vec::with_capacity(results.len());

            for (memid, m_goal) in results {
                let window_text_hash = official_text_hashing_function(&m_goal);
                memids.push(memid);
                todo.push(SimpleTextHashVector {
                    window_text: m_goal,
                    window_text_hash,
                    vector: None,
                });
            }

            Ok((memids, todo))
        }).await.map_err(|e| e.to_string())?
    };

    {
        let mut memdb_locked = memdb.lock().await;
        memdb_locked.dirty_memids.clear();
        memdb_locked.dirty_everything = false;
    }

    Ok((memids, todo))
}

pub async fn vectorize_dirty_memories(
    memdb: Arc<AMutex<MemoriesDatabase>>,
    vecdb_handler: Arc<AMutex<VecDBSqlite>>,
    _status: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<Client>>,
    api_key: &String,
    #[allow(non_snake_case)]
    B: usize,
) -> rusqlite::Result<(), String> {
    let (memids, mut todo) = recall_dirty_memories_and_mark_them_not_dirty(memdb.clone()).await?;
    if memids.is_empty() {
        return Ok(());
    }

    {
        let mut handler_locked = vecdb_handler.lock().await;
        handler_locked.process_simple_hash_text_vector(&mut todo).await.map_err(|e| format!("Failed to get vectors from cache: {}", e))?
        // this makes todo[].vector appear for records that exist in cache
    }

    let todo_len = todo.len();
    let mut to_vectorize = todo.iter_mut().filter(|x| x.vector.is_none()).collect::<Vec<&mut SimpleTextHashVector>>();
    info!("{} memories total, {} to vectorize", todo_len, to_vectorize.len());
    let my_constants: VecdbConstants = memdb.lock().await.vecdb_constants.clone();
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
        ).await?;
        for (chunk_save, x) in chunk.iter_mut().zip(embedding_mb.iter()) {
            chunk_save.vector = Some(x.clone());  // <-- this will make the rest of todo[].vector appear
        }
    }

    {
        let mut handler_locked = vecdb_handler.lock().await;
        let temp_vec: Vec<SimpleTextHashVector> = to_vectorize.iter().map(|x| (**x).clone()).collect();
        handler_locked.cache_add_new_records(temp_vec).await.map_err(|e| format!("Failed to update cache: {}", e))?;
    }

    let conn = memdb.lock().await.conn.clone();
    conn.lock().await.call(move |connection| {
        let mut stmt = connection.prepare(
            "INSERT INTO embeddings(embedding, memid) VALUES (?, ?)"
        )?;
        for (item, memid) in todo.into_iter().zip(memids) {
            stmt.execute(rusqlite::params![
                    item.vector.clone().expect("No embedding is provided").as_bytes(), 
                    memid
                ])?;
        }
        Ok(())
    }).await.map_err(|e| e.to_string())?;
    Ok(())
}
