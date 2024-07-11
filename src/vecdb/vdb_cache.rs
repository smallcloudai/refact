use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;

use hashbrown::HashSet;
use rusqlite::{OpenFlags, params, Result};
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::info;

use crate::vecdb::vdb_structs::{VecdbRecord, SplitResult};

impl Debug for VecDBCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBCache: {:?}", self.cache_database.type_id())
    }
}

pub struct VecDBCache {
    cache_database: Connection,
    cached_window_text_hashes: HashSet<String>,
}

const EMB_TABLE_NAME: &str = "embeddings";

#[derive(Debug, PartialEq)]
struct DataColumn {
    name: String,
    type_: String,
}


async fn check_and_recreate_embeddings_table(db: &Connection) -> tokio_rusqlite::Result<()> {
    let expected_schema = vec![
        DataColumn { name: "vector".to_string(), type_: "BLOB".to_string() },
        DataColumn { name: "window_text".to_string(), type_: "TEXT".to_string() },
        DataColumn { name: "window_text_hash".to_string(), type_: "TEXT".to_string() },
    ];
    db.call(move |conn| {
        match conn.execute(&format!("ALTER TABLE data RENAME TO {EMB_TABLE_NAME};"), []) {
            _ => {}
        };
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({EMB_TABLE_NAME});"))?;
        let schema_iter = stmt.query_map([], |row| {
            Ok(DataColumn {
                name: row.get(1)?,
                type_: row.get(2)?,
            })
        })?;
        let mut schema = Vec::new();
        for column in schema_iter {
            schema.push(column?);
        }
        if schema != expected_schema {
            if schema.len() > 0 {
                info!("vector cache database has invalid schema, recreating the database");
            }
            conn.execute(&format!("DROP TABLE IF EXISTS {EMB_TABLE_NAME}"), [])?;
            conn.execute(&format!(
                "CREATE TABLE {EMB_TABLE_NAME} (
                vector BLOB,
                window_text TEXT NOT NULL,
                window_text_hash TEXT NOT NULL
            )"), [])?;
            conn.execute(&format!(
                "CREATE INDEX IF NOT EXISTS idx_window_text_hash \
                ON {EMB_TABLE_NAME} (window_text_hash)"),
                         [],
            )?;
        }
        Ok(())
    }).await
}

async fn select_window_text_hashes(db: &Connection) -> HashSet<String> {
    let query = format!("SELECT window_text_hash FROM {EMB_TABLE_NAME}");
    let result = db.call(move |connection| {
        let mut statement = connection.prepare(&query)?;
        let mut rows = statement.query([])?;
        let mut hashes = HashSet::new();
        while let Some(row) = rows.next()? {
            let hash: String = row.get(0)?;
            hashes.insert(hash);
        }
        Ok(hashes)
    }).await;

    result.unwrap_or_else(|err| {
        info!("Error while selecting window_text_hashes: {:?}", err);
        HashSet::new()
    })
}

impl VecDBCache {
    pub async fn init(cache_dir: &PathBuf, model_name: &String, embedding_size: i32) -> Result<VecDBCache, String> {
        let cache_dir_str = match cache_dir.join("refact_vecdb_cache")
            .join(format!("model_{}_esize_{}.sqlite",
                          model_name.replace("/", "_"),
                          embedding_size
            )).to_str() {
            Some(dir) => dir.to_string(),
            None => {
                return Err(format!("{:?}", "Cache directory is not a valid path"));
            }
        };
        if !cache_dir.join("refact_vecdb_cache").exists() {
            match fs::create_dir_all(cache_dir.join("refact_vecdb_cache")).await {
                Ok(_) => {}
                Err(e) => return Err(format!("{:?}", e)),
            }
        }
        let cache_database = match Connection::open_with_flags(
            cache_dir_str, OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI).await {
            Ok(db) => db,
            Err(err) => return Err(format!("{:?}", err))
        };
        let _ = cache_database.call(move |conn| {
            Ok(conn.execute("PRAGMA journal_mode=WAL", params![])?)
        }).await;
        match check_and_recreate_embeddings_table(&cache_database).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }

        info!("building window_text_hashes index");
        let cached_window_text_hashes = select_window_text_hashes(&cache_database).await;
        info!("building window_text_hashes complete");

        Ok(VecDBCache { cache_database, cached_window_text_hashes })
    }

    pub fn contains(&self, window_text_hash: &String) -> bool {
        self.cached_window_text_hashes.contains(window_text_hash)
    }

    pub async fn get_records_by_splits(&mut self, splits: &Vec<SplitResult>) -> Result<(Vec<VecdbRecord>, Vec<SplitResult>), String> {
        let placeholders: String = splits.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT * FROM {EMB_TABLE_NAME} WHERE window_text_hash IN ({placeholders})");
        let splits_clone = splits.clone();
        let found_hashes = match self.cache_database.call(move |connection| {
            let mut statement = connection.prepare(&query)?;
            let params = rusqlite::params_from_iter(splits_clone.iter().map(|x| &x.window_text_hash));
            let x = match statement.query_map(params, |row| {
                let vector_blob: Vec<u8> = row.get(0)?;
                let vector: Vec<f32> = vector_blob
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();
                let window_text: String = row.get(1)?;
                let window_text_hash: String = row.get(2)?;
                Ok((window_text_hash, (vector, window_text)))
            }) {
                Ok(mapped_rows) => {
                    Ok(mapped_rows.filter_map(|r| r.ok()).collect::<HashMap<_, _>>())
                }
                Err(e) => {
                    Err(tokio_rusqlite::Error::Rusqlite(e))
                }
            };
            x
        }).await {
            Ok(records) => records,
            Err(err) => return Err(format!("{:?}", err))
        };
        let mut records = vec![];
        let mut non_found_splits = vec![];
        for split in splits.iter() {
            if let Some(query_data) = found_hashes.get(&split.window_text_hash) {
                records.push(VecdbRecord {
                    vector: Some(query_data.0.clone()),
                    window_text: split.window_text.clone(),
                    window_text_hash: split.window_text_hash.clone(),
                    file_path: split.file_path.clone(),
                    start_line: split.start_line,
                    end_line: split.end_line,
                    distance: -1.0,
                    usefulness: 0.0,
                })
            } else {
                non_found_splits.push(split.clone());
            }
        }
        Ok((records, non_found_splits))
    }

    pub async fn insert_records(&mut self, records: Vec<VecdbRecord>) -> Result<(), String> {
        match self.cache_database.call(|connection| {
            let transaction = connection.transaction()?;
            for record in records {
                let vector_as_bytes: Vec<u8> = record.vector.expect(
                    "An attempt to push vector-less embeddings to cache DB"
                ).iter()
                    .flat_map(|&num| num.to_ne_bytes())
                    .collect();

                match transaction.execute(&format!(
                    "INSERT INTO {EMB_TABLE_NAME} (vector, window_text, window_text_hash) VALUES (?1, ?2, ?3)"),
                                          rusqlite::params![
                    vector_as_bytes,
                    record.window_text,
                    record.window_text_hash,
                ],
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        info!("Error while inserting record to cache: {:?}", err);
                        continue;
                    }
                }
            }
            match transaction.commit() {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into())
            }
        }).await {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    pub async fn size(&self) -> Result<usize, String> {
        self.cache_database.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("SELECT COUNT(*) FROM {EMB_TABLE_NAME}")
            )?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await
            .map_err(|e| {
                e.to_string()
            })
    }
}
