use rusqlite::{params, OpenFlags, Result};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::info;
use zerocopy::AsBytes;

use crate::vecdb::vdb_structs::{SimpleTextHashVector, SplitResult, VecdbRecord};


impl Debug for VecDBSqlite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBSqlite: {:?}", self.conn.type_id())
    }
}

pub struct VecDBSqlite {
    conn: Connection,
}


#[derive(Debug, PartialEq)]
struct DataColumn {
    name: String,
    type_: String,
}

async fn get_db_path(cache_dir: &PathBuf, model_name: &String, embedding_size: i32) -> Result<String, String> {
    let old_path = cache_dir
        .join("refact_vecdb_cache")
        .join(format!("model_{}_esize_{}.sqlite",
                      model_name.replace("/", "_"),
                      embedding_size
        ));
    let new_path = cache_dir
        .join(format!("vecdb_model_{}_esize_{}.sqlite",
                      model_name.replace("/", "_"),
                      embedding_size
        ));
    if old_path.exists() && !new_path.exists() {
        match fs::rename(&old_path, &new_path).await {
            Ok(_) => {
                Ok(new_path.to_string_lossy().to_string())
            }
            Err(e) => Err(format!("{:?}", e))
        }
    } else {
        Ok(new_path.to_string_lossy().to_string())
    }
}

async fn migrate_202406(conn: &Connection) -> tokio_rusqlite::Result<()> {
    let expected_schema = vec![
        DataColumn { name: "vector".to_string(), type_: "BLOB".to_string() },
        DataColumn { name: "window_text".to_string(), type_: "TEXT".to_string() },
        DataColumn { name: "window_text_hash".to_string(), type_: "TEXT".to_string() },
    ];
    conn.call(move |conn| {
        match conn.execute(&format!("ALTER TABLE data RENAME TO embeddings;"), []) {
            _ => {}
        };
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(embeddings);"))?;
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
            conn.execute(&format!("DROP TABLE IF EXISTS embeddings"), [])?;
            conn.execute(&format!(
                "CREATE TABLE embeddings (
                    vector BLOB,
                    window_text TEXT NOT NULL,
                    window_text_hash TEXT NOT NULL
                )"), [])?;
            conn.execute(&format!(
                "CREATE INDEX IF NOT EXISTS idx_window_text_hash \
                ON embeddings (window_text_hash)"),
                         [],
            )?;
        }
        Ok(())
    }).await
}


async fn migrate_202501(conn: &Connection, embedding_size: i32) -> tokio_rusqlite::Result<()> {
    conn.call(move |conn| {
        match conn.execute("ALTER TABLE embeddings RENAME TO embeddings_cache;", []) {
            _ => {}
        };
        match conn.execute(&format!(
            "CREATE VIRTUAL TABLE embeddings using vec0(
              embedding float[{embedding_size}] distance_metric=cosine,
              scope text partition key,
              +start_line integer
              +end_line integer
            );"), []) {
            _ => {}
        };
        Ok(())
    }).await
}

impl VecDBSqlite {
    pub async fn init(cache_dir: &PathBuf, model_name: &String, embedding_size: i32) -> Result<VecDBSqlite, String> {
        let db_path = get_db_path(cache_dir, model_name, embedding_size).await?;
        let conn = match Connection::open_with_flags(
            db_path, OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI).await {
            Ok(db) => db,
            Err(err) => return Err(format!("{:?}", err))
        };
        let _ = conn.call(move |conn| {
            Ok(conn.execute("PRAGMA journal_mode=WAL", params![])?)
        }).await;
        match migrate_202406(&conn).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }
        match migrate_202501(&conn, embedding_size).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }
        info!("vecdb initialized");
        Ok(VecDBSqlite { conn })
    }

    pub async fn process_simple_hash_text_vector(
        &mut self,
        v: &mut Vec<SimpleTextHashVector>,
    ) -> Result<(), String> {
        let placeholders: String = v.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT vector, window_text_hash FROM embeddings_cache WHERE window_text_hash IN ({placeholders})");
        let vclone = v.clone();
        let found_vectors = match self.conn.call(move |connection| {
            let mut statement = connection.prepare(&query)?;
            let params = rusqlite::params_from_iter(vclone.iter().map(|x| &x.window_text_hash));
            let result = statement.query_map(params, |row| {
                let vector_blob: Vec<u8> = row.get(0)?;
                let window_text_hash: String = row.get(1)?;
                let vector: Vec<f32> = vector_blob
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();
                Ok((window_text_hash, vector))
            })?;
            Ok(result.filter_map(|r| r.ok()).collect::<HashMap<_, _>>())
        }).await {
            Ok(vectors) => vectors,
            Err(err) => return Err(format!("Error querying database: {:?}", err))
        };
        for save in v.iter_mut() {
            if let Some(vector) = found_vectors.get(&save.window_text_hash) {
                save.vector = Some(vector.clone());
            }
        }
        Ok(())
    }

    pub async fn fetch_vectors_from_cache(&mut self, splits: &Vec<SplitResult>) -> Result<Vec<Option<Vec<f32>>>, String> {
        let placeholders: String = splits.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT * FROM embeddings_cache WHERE window_text_hash IN ({placeholders})");
        let splits_clone = splits.clone();
        let found_hashes = match self.conn.call(move |connection| {
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
        let mut records: Vec<Option<Vec<f32>>> = vec![];
        for split in splits.iter() {
            if let Some(query_data) = found_hashes.get(&split.window_text_hash) {
                records.push(Some(query_data.0.clone()));
            } else {
                records.push(None);
            }
        }
        Ok(records)
    }

    pub async fn cache_add_new_records(&mut self, records: Vec<SimpleTextHashVector>) -> Result<(), String> {
        self.conn.call(|connection| {
            let transaction = connection.transaction()?;
            for record in records {
                let vector_as_bytes: Vec<u8> = match record.vector {
                    Some(vector) => vector.iter()
                        .flat_map(|&num| num.to_ne_bytes())
                        .collect(),
                    None => {
                        tracing::error!("Skipping record with no vector: {:?}", record.window_text_hash);
                        continue;
                    }
                };

                match transaction.execute(&format!(
                    "INSERT INTO embeddings_cache (vector, window_text, window_text_hash) VALUES (?1, ?2, ?3)"),
                                          rusqlite::params![
                        vector_as_bytes,
                        record.window_text,
                        record.window_text_hash,
                    ],
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Error while inserting record to cache: {:?}", err);
                        continue;
                    }
                }
            }
            match transaction.commit() {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into())
            }
        }).await.map_err(|e| e.to_string())
    }

    pub async fn cache_size(&self) -> Result<usize, String> {
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("SELECT COUNT(1) FROM embeddings_cache")
            )?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn size(&self) -> Result<usize, String> {
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("SELECT COUNT(1) FROM embeddings")
            )?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn vecdb_records_add(&mut self, records: &Vec<VecdbRecord>) -> Result<(), String> {
        let records_owned = records.clone();
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                "INSERT INTO embeddings(embedding, scope, start_line, end_line) VALUES (?, ?, ?, ?)"
            )?;
            for item in records_owned.iter() {
                stmt.execute(rusqlite::params![
                    item.vector.clone().expect("No embedding is provided").as_bytes(), 
                    item.file_path.to_string_lossy().to_string(),
                    item.start_line,
                    item.end_line
                ])?;
            }
            Ok(())
        }).await.map_err(|e| e.to_string())
    }

    pub async fn vecdb_search(
        &mut self,
        embedding: &Vec<f32>,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
    ) -> Result<Vec<VecdbRecord>, String> {
        let scope_condition = vecdb_scope_filter_mb
            .clone()
            .map(|_| format!("AND scope = ?"))
            .unwrap_or_else(String::new);
        let embedding_owned = embedding.clone();
        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(&format!(
                r#"
                SELECT
                    scope,
                    start_line,
                    end_line,
                    embedding,
                    distance
                FROM embeddings
                WHERE embedding MATCH ?
                    AND k = ?
                    {}
                ORDER BY distance
                "#,
                scope_condition
            ))?;

            let embedding_bytes = embedding_owned.as_bytes();
            let params = match &vecdb_scope_filter_mb {
                Some(scope) => rusqlite::params![&embedding_bytes, top_n, scope.clone()],
                None => rusqlite::params![&embedding_bytes, top_n],
            };
            
            let rows = stmt.query_map(
                params,
                |row| {
                    let vector_blob: Vec<u8> = row.get(3)?;
                    let vector: Vec<f32> = vector_blob
                        .chunks_exact(4)
                        .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                        .collect();
                    Ok(VecdbRecord {
                        vector: Some(vector),
                        file_path: PathBuf::from(row.get::<_, String>(0)?),
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        distance: row.get(4)?,
                        usefulness: 0.0,
                    })
                },
            )?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }

            Ok(results)
        }).await.map_err(|e| e.to_string())
    }

    pub async fn vecdb_records_remove(
        &mut self,
        scopes_to_remove: Vec<String>,
    ) -> Result<(), String> {
        if scopes_to_remove.is_empty() {
            return Ok(());
        }

        let placeholders: String = scopes_to_remove.iter()
            .map(|_| "?")
            .collect::<Vec<&str>>()
            .join(",");

        self.conn.call(move |connection| {
            let mut stmt = connection.prepare(
                &format!("DELETE FROM embeddings WHERE scope IN ({})", placeholders)
            )?;

            stmt.execute(rusqlite::params_from_iter(scopes_to_remove.iter()))?;
            Ok(())
        }).await.map_err(|e| e.to_string())
    }
}
