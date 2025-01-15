use std::os::raw::{c_int, c_void};
use std::sync::Arc;
use std::path::PathBuf;
use tracing::info;

use parking_lot::Mutex as ParkMutex;
use rand::Rng;
use rusqlite::{params, Connection};
use arrow::array::{ArrayData, Float32Array, StringArray, FixedSizeListArray, RecordBatchIterator, RecordBatch};
use arrow::buffer::Buffer;
use arrow_array::cast::{as_fixed_size_list_array, as_primitive_array, as_string_array};
use arrow_array::types::Float32Type;
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures_util::TryStreamExt;
use itertools::Itertools;
use lance::dataset::{WriteMode, WriteParams};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use vectordb::database::Database;
use tempfile::TempDir;
use tokio::sync::{Mutex as AMutex, Notify};
use tokio::time::Instant;
use vectordb::table::Table;

use crate::vecdb::vdb_cache::VecDBCache;
use crate::vecdb::vdb_lance::cosine_distance;
use crate::vecdb::vdb_structs::{MemoRecord, SimpleTextHashVector, VecdbConstants, VecDbStatus};
use crate::ast::chunk_utils::official_text_hashing_function;


pub struct MemoriesDatabase {
    pub conn: Arc<ParkMutex<Connection>>,
    pub vecdb_constants: VecdbConstants,
    pub memories_table: Table,
    pub schema_arc: SchemaRef,
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

extern "C" fn pubsub_trigger_hook(
    user_data: *mut c_void, 
    action: c_int,
    db_name: *const i8,
    table_name: *const i8,
    _: i64,
) {
    let notify = unsafe { &*(user_data as *const Notify) };
    let db_name = unsafe { std::ffi::CStr::from_ptr(db_name).to_str().unwrap_or("unknown") };
    let table_name = unsafe { std::ffi::CStr::from_ptr(table_name).to_str().unwrap_or("unknown") };
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

impl MemoriesDatabase {
    pub async fn init(
        config_dir: &PathBuf,
        // vecdb_cache: Arc<AMutex<VecDBCache>>,
        constants: &VecdbConstants,
        reset_memory: bool,
    ) -> Result<MemoriesDatabase, String> {
        // SQLite database for memories, permanent on disk
        let dbpath = config_dir.join("memories.sqlite");
        let cache_database = Connection::open_with_flags(
            dbpath,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_URI
        ).map_err(|err| format!("Failed to open database: {}", err))?;
        cache_database.busy_timeout(std::time::Duration::from_secs(30)).map_err(|err| format!("Failed to set busy timeout: {}", err))?;
        cache_database.execute_batch("PRAGMA cache_size = 0; PRAGMA shared_cache = OFF;").map_err(|err| format!("Failed to set cache pragmas: {}", err))?;
        let journal_mode: String = cache_database.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0)).map_err(|err| format!("Failed to set journal mode: {}", err))?;
        if journal_mode != "wal" {
            return Err(format!("Failed to set WAL journal mode. Current mode: {}", journal_mode));
        }

        // Arrow database for embeddings, only valid for the current process
        let embedding_temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let embedding_path = embedding_temp_dir.path().to_str().unwrap();
        let schema_arc = Arc::new(Schema::new(vec![
            Field::new("memid", DataType::Utf8, false),
            Field::new("thevec", DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                constants.embedding_size,
            ), false),
        ]));
        let temp_database = Database::connect(embedding_path).await.map_err(|err| format!("Failed to connect to database: {:?}", err))?;
        let batches_iter = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema_arc.clone());
        let memories_table = match temp_database.create_table("memories", batches_iter, Option::from(WriteParams::default())).await {
            Ok(t) => t,
            Err(err) => return Err(format!("{:?}", err))
        };

        let db = MemoriesDatabase {
            conn: Arc::new(ParkMutex::new(cache_database)),
            vecdb_constants: constants.clone(),
            memories_table,
            schema_arc,
            dirty_memids: Vec::new(),
            dirty_everything: true,
            pubsub_notifier: Arc::new(Notify::new())
        };
        db._permdb_create_table(reset_memory)?;
        db._migrate_add_m_origin()?;
        unsafe {
            libsqlite3_sys::sqlite3_update_hook(
                db.conn.lock().handle(), 
                Some(pubsub_trigger_hook),
                Arc::into_raw(db.pubsub_notifier.clone()) as *mut c_void,
            );
        }
        Ok(db)
    }

    fn _migrate_add_m_origin(&self) -> Result<(), String> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("PRAGMA table_info(memories)").map_err(|e| e.to_string())?;
        let column_exists = stmt.query_map([], |row| {
            let column_name: String = row.get(1)?;
            Ok(column_name)
        })
            .map_err(|e| e.to_string())?
            .filter_map(|result| result.ok())
            .any(|column_name| column_name == "m_origin");

        if !column_exists {
            conn.execute("ALTER TABLE memories ADD COLUMN m_origin TEXT NOT NULL DEFAULT 'refact-standard'", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn _permdb_create_table(&self, reset_memory: bool) -> Result<(), String> {
        let conn = self.conn.lock();
        if reset_memory {
            conn.execute("DROP TABLE IF EXISTS memories", []).map_err(|e| e.to_string())?;
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
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pubsub_events (
            pubevent_id INTEGER PRIMARY KEY AUTOINCREMENT,
            pubevent_action TEXT NOT NULL,
            pubevent_memid TEXT NOT NULL,
            pubevent_json TEXT NOT NULL,
            pubevent_ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
            [],
        ).map_err(|e| e.to_string())?;

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
        ).map_err(|e| e.to_string())?;

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
        ).map_err(|e| e.to_string())?;

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
        ).map_err(|e| e.to_string())?;

        // Trigger to delete old events
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_delete_old
            AFTER INSERT ON pubsub_events
            BEGIN
                DELETE FROM pubsub_events WHERE pubevent_ts <= datetime('now', '-15 minutes');
            END;",
            [],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn permdb_add(&self, mem_type: &str, goal: &str, project: &str, payload: &str, m_origin: &str) -> Result<String, String> {
        fn generate_memid() -> String {
            rand::thread_rng()
                .sample_iter(&rand::distributions::Uniform::new(0, 16))
                .take(10)
                .map(|x| format!("{:x}", x))
                .collect()
        }

        let conn = self.conn.lock();
        let memid = generate_memid();
        conn.execute(
            "INSERT INTO memories (memid, m_type, m_goal, m_project, m_payload, m_origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![memid, mem_type, goal, project, payload, m_origin],
        ).map_err(|e| e.to_string())?;
        Ok(memid)
    }

    pub async fn permdb_erase(&mut self, memid: &str) -> Result<usize, String> {
        let affected_rows = {
            let conn = self.conn.lock();
            conn.execute(
                "DELETE FROM memories WHERE memid = ?1",
                params![memid],
            ).map_err(|e| e.to_string())?
        };

        match self.memories_table.delete(&format!("memid IN ('{memid}')")).await {
            Ok(_) => {}
            Err(err) => {
                tracing::error!("Error deleting from vecdb: {:?}", err);
            }
        }

        Ok(affected_rows)
    }

    pub fn permdb_update_used(&self, memid: &str, mstat_correct: i32, mstat_relevant: i32) -> Result<usize, String> {
        let conn = self.conn.lock();
        let affected_rows = conn.execute(
            "UPDATE memories SET mstat_times_used = mstat_times_used + 1, mstat_correct = mstat_correct + ?1, mstat_relevant = mstat_relevant + ?2 WHERE memid = ?3",
            params![mstat_correct, mstat_relevant, memid],
        ).map_err(|e| e.to_string())?;
        Ok(affected_rows)
    }

    #[allow(dead_code)]
    pub fn permdb_print_everything(&self) -> Result<String, String> {
        let mut table_contents = String::new();
        let conn = self.conn.lock();
        let mut stmt = conn.prepare( &format!("SELECT {} FROM memories", fields_ordered()))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, f64>(7)?,
                row.get::<_, i32>(8)?,
            ))
        }).map_err(|e| e.to_string())?;

        for row in rows {
            let (memid, m_type, m_goal, m_project, m_payload, m_origin, mstat_correct, mstat_relevant, mstat_times_used) = row.map_err(|e| e.to_string())?;
            table_contents.push_str(&format!(
                "memid={}, type={}, goal: {:?}, project: {:?}, payload: {:?}, m_origin: {:?}, correct={}, relevant={}, times_used={}\n",
                memid, m_type, m_goal, m_project, m_payload, m_origin, mstat_correct, mstat_relevant, mstat_times_used
            ));
        }
        Ok(table_contents)
    }

    pub async fn permdb_select_all(&self) -> Result<Vec<MemoRecord>, String> {
        let conn = self.conn.lock();
        let query = format!("SELECT {} FROM memories", fields_ordered());

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], map_row_to_memo_record).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub async fn permdb_select_like(&self, query: &String) -> Result<Vec<MemoRecord>, String> {
        let conn = self.conn.lock();

        let query_str = format!(
            "SELECT {} FROM memories WHERE 
            m_type LIKE ? COLLATE NOCASE OR 
            m_goal LIKE ? COLLATE NOCASE OR 
            m_project LIKE ? COLLATE NOCASE OR 
            m_payload LIKE ? COLLATE NOCASE", 
            fields_ordered()
        );

        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(&query_str).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![pattern, pattern, pattern, pattern], map_row_to_memo_record)
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub async fn permdb_fillout_records(&self, input_records: Vec<MemoRecord>) -> Result<Vec<MemoRecord>, String> {
        let t0 = Instant::now();
        let conn = self.conn.lock();

        let memids: Vec<String> = input_records.iter().map(|record| record.memid.clone()).collect();
        let placeholders = memids.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT {} FROM memories WHERE memid IN ({})", fields_ordered(), placeholders);
        let params = rusqlite::params_from_iter(memids.iter());
        let mut statement = conn.prepare(&query).map_err(|e| e.to_string())?;

        let db_records: std::collections::HashMap<String, MemoRecord> = statement
            .query_map(params, map_row_to_memo_record)
            .map_err(|e| e.to_string())?
            .filter_map(|row_result| row_result.ok().map(|record| (record.memid.clone(), record)))
            .collect();

        let result: Vec<MemoRecord> = input_records
            .into_iter()
            .filter_map(|mut record| {
                if let Some(db_record) = db_records.get(&record.memid) {
                    record.m_type = db_record.m_type.clone();
                    record.m_goal = db_record.m_goal.clone();
                    record.m_project = db_record.m_project.clone();
                    record.m_payload = db_record.m_payload.clone();
                    record.m_origin = db_record.m_origin.clone();
                    record.mstat_correct = db_record.mstat_correct;
                    record.mstat_relevant = db_record.mstat_relevant;
                    record.mstat_times_used = db_record.mstat_times_used;
                    Some(record)
                } else {
                    tracing::warn!("permdb_memids2records() not found memid={}", record.memid);
                    None
                }
            }).collect();

        let elapsed_time = t0.elapsed();
        info!("permdb_fillout_records({}) took {:.2}s", memids.len(), elapsed_time.as_secs_f64());
        Ok(result)
    }

    pub async fn permdb_sub_select_all(&self, from_memid: Option<i64>) -> Result<Vec<MemdbSubEvent>, String> {
        let conn = self.conn.lock();
        let query = "
            SELECT pubevent_id, pubevent_action, pubevent_memid, pubevent_json
            FROM pubsub_events
            WHERE pubevent_id > ?1
            ORDER BY pubevent_id ASC
        ";

        let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
        let mut rows = match from_memid {
            Some(id) => stmt.query([id]).map_err(|e| format!("Failed to execute query: {}", e))?,
            None => stmt.query([0]).map_err(|e| format!("Failed to execute query: {}", e))?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().map_err(|e| format!("Failed to fetch row: {}", e))? {
            let pubevent_id: i64 = row.get(0).map_err(|e| format!("Failed to read pubevent_id: {}", e))?;
            let pubevent_action: String = row.get(1).map_err(|e| format!("Failed to read pubevent_action: {}", e))?;
            let pubevent_memid: String = row.get(2).map_err(|e| format!("Failed to read pubevent_action: {}", e))?;
            let pubevent_json: String = row.get(3).map_err(|e| format!("Failed to read pubevent_json: {}", e))?;
            results.push(MemdbSubEvent { pubevent_id, pubevent_action, pubevent_memid, pubevent_json });
        }
        Ok(results)
    }


}

fn _lance_fetch_all_records_measuring_distance(
    record_batch: RecordBatch,
    include_embedding: bool,
    embedding_to_compare: Option<&Vec<f32>>,
) -> vectordb::error::Result<Vec<MemoRecord>> {
    (0..record_batch.num_rows()).map(|idx| {
        let gathered_vec = as_primitive_array::<Float32Type>(
            &as_fixed_size_list_array(record_batch.column_by_name("thevec").unwrap())
                .iter()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()[idx]
        )
            .iter()
            .map(|x| x.unwrap()).collect::<Vec<_>>();
        let distance = match embedding_to_compare {
            None => { -1.0 }
            Some(embedding) => {
                // info!("cosine_distance, embd\n{:?}\nv\n{:?}\n", embedding, gathered_vec);
                cosine_distance(&embedding, &gathered_vec)
            }
        };
        let embedding = match include_embedding {
            true => Some(gathered_vec),
            false => None
        };

        Ok(MemoRecord {
            memid: as_string_array(record_batch.column_by_name("memid")
                .expect("Missing column 'memid'"))
                .value(idx)
                .to_string(),
            thevec: embedding,
            distance,
            ..Default::default()
        })
    }).collect()   // collect() here operates on Result<> and returns Result<Vec<>>, a feature of rust
}

pub async fn lance_search(
    memdb: Arc<AMutex<MemoriesDatabase>>,
    embedding: &Vec<f32>,
    top_n: usize,
) -> vectordb::error::Result<Vec<MemoRecord>> {
    let (my_memories_table, my_schema_arc) = {
        let memdb_locked = memdb.lock().await;
        (memdb_locked.memories_table.clone(), memdb_locked.schema_arc.clone())
    };
    let query = my_memories_table
        .clone()
        .search(Some(Float32Array::from(embedding.clone())))
        .column("thevec")
        .limit(top_n)
        .use_index(true)
        .execute().await?
        .try_collect::<Vec<_>>().await?;
    let record_batch = arrow::compute::concat_batches(&my_schema_arc, &query)?;

    match _lance_fetch_all_records_measuring_distance(record_batch, false, Some(&embedding)) {
        Ok(records) => {
            let sorted = records.into_iter().sorted_unstable_by(|a, b|a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal)).collect::<Vec<_>>();
            Ok(sorted)
        },
        Err(e) => Err(e),
    }
}

async fn recall_dirty_memories_and_mark_them_not_dirty(
    memdb: Arc<AMutex<MemoriesDatabase>>,
) -> Result<(Vec<String>, Vec<SimpleTextHashVector>), String> {
    let mut memids: Vec<String> = Vec::new();
    let mut todo: Vec<SimpleTextHashVector> = Vec::new();
    let mut memdb_locked = memdb.lock().await;
    let rows: Vec<(String, String)> = {
        let conn = memdb_locked.conn.lock();
        if memdb_locked.dirty_everything {
            let mut stmt = conn.prepare("SELECT memid, m_goal FROM memories")
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;
            let x = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })
                .map_err(|e| format!("Failed to query memories: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to collect rows: {}", e))?;
            x
        } else if !memdb_locked.dirty_memids.is_empty() {
            let placeholders = (0..memdb_locked.dirty_memids.len())
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let query = format!("SELECT memid, m_goal FROM memories WHERE memid IN ({})", placeholders);
            let mut stmt = conn.prepare(&query)
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;
            let x = stmt.query_map(rusqlite::params_from_iter(memdb_locked.dirty_memids.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })
                .map_err(|e| format!("Failed to query memories: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to collect rows: {}", e))?;
            x
        } else {
            Vec::new()
        }
    };
    for (memid, m_goal) in rows {
        let window_text_hash = official_text_hashing_function(&m_goal);
        let simple_text_hash_vector = SimpleTextHashVector {
            window_text: m_goal,
            window_text_hash,
            vector: None,
        };
        memids.push(memid);
        todo.push(simple_text_hash_vector);
    }
    memdb_locked.dirty_memids.clear();
    memdb_locked.dirty_everything = false;
    Ok((memids, todo))
}

pub async fn vectorize_dirty_memories(
    memdb: Arc<AMutex<MemoriesDatabase>>,
    vecdb_cache: Arc<AMutex<VecDBCache>>,
    _status: Arc<AMutex<VecDbStatus>>,
    client: Arc<AMutex<Client>>,
    api_key: &String,
    #[allow(non_snake_case)]
    B: usize,
) -> Result<(), String> {
    let (memids, mut todo) = recall_dirty_memories_and_mark_them_not_dirty(memdb.clone()).await?;
    if memids.is_empty() {
        return Ok(());
    }

    {
        let mut cache_locked = vecdb_cache.lock().await;
        cache_locked.process_simple_hash_text_vector(&mut todo).await.map_err(|e| format!("Failed to get vectors from cache: {}", e))?
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
        let mut cache_locked = vecdb_cache.lock().await;
        let temp_vec: Vec<SimpleTextHashVector> = to_vectorize.iter().map(|x| (**x).clone()).collect();
        cache_locked.cache_add_new_records(temp_vec).await.map_err(|e| format!("Failed to update cache: {}", e))?;
    }

    // Save to lance
    fn make_emb_data(records: &Vec<SimpleTextHashVector>, embedding_size: i32) -> Result<ArrayData, String> {
        let vec_trait = Arc::new(Field::new("item", DataType::Float32, true));
        let mut emb_builder: Vec<f32> = vec![];
        for record in records {
            assert!(record.vector.is_some());
            assert_eq!(record.vector.as_ref().unwrap().len(), embedding_size as usize);
            emb_builder.append(&mut record.vector.clone().expect("No embedding"));
        }
        let emb_data_res = ArrayData::builder(DataType::Float32)
            .add_buffer(Buffer::from_vec(emb_builder))
            .len(records.len() * embedding_size as usize)
            .build();
        let emb_data = match emb_data_res {
            Ok(res) => res,
            Err(err) => { return Err(format!("{:?}", err)); }
        };
        match ArrayData::builder(DataType::FixedSizeList(vec_trait.clone(), embedding_size))
            .len(records.len())
            .add_child_data(emb_data.clone())
            .build()
        {
            Ok(res) => Ok(res),
            Err(err) => return Err(format!("{:?}", err))
        }
    }

    let vectors: ArrayData = match make_emb_data(&todo, my_constants.embedding_size) {
        Ok(res) => res,
        Err(err) => return Err(format!("{:?}", err))
    };

    let my_schema_arc = memdb.lock().await.schema_arc.clone();
    let data_batches_iter = RecordBatchIterator::new(
        vec![RecordBatch::try_new(
            my_schema_arc.clone(),
            vec![
                Arc::new(StringArray::from(memids)),
                Arc::new(FixedSizeListArray::from(vectors.clone())),
            ],
        )],
        my_schema_arc.clone(),
    );
    let data_res = {
        let mut memdb_locked = memdb.lock().await;
        memdb_locked.memories_table.add(
            data_batches_iter,
            Some(WriteParams {
                mode: WriteMode::Append,
                ..Default::default()
            }),
        ).await
    };
    info!("updated {} memories in the database:\n{:?}", todo.len(), data_res);

    Ok(())
}
