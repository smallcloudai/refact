use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

use rusqlite::{OpenFlags, params, Result};
use tokio::fs;
use tokio_rusqlite::Connection;
use tracing::info;

use crate::vecdb::structs::{Record, SplitResult};

impl Debug for VecDBCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBCache: {:?}", self.cache_database.type_id())
    }
}

pub struct VecDBCache {
    cache_database: Connection,
    embedding_size: i32,
}

const TWO_WEEKS: i32 = 2 * 7 * 24 * 3600;
const ONE_MONTH: i32 = 30 * 24 * 3600;
const MIN_LIKES: i32 = 3;
const EMB_TABLE_NAME: &str = "embeddings";
const SPLITS_TABLE_NAME: &str = "splits";

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
        DataColumn { name: "time_added".to_string(), type_: "INTEGER".to_string() },
        DataColumn { name: "time_last_used".to_string(), type_: "INTEGER".to_string() },
        DataColumn { name: "model_name".to_string(), type_: "TEXT".to_string() },
        DataColumn { name: "used_counter".to_string(), type_: "INTEGER".to_string() },
    ];
    db.call(move |conn| {
        conn.execute(&format!("ALTER TABLE data RENAME TO {EMB_TABLE_NAME};"), [])?;
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
                window_text_hash TEXT NOT NULL,
                time_added INTEGER NOT NULL,
                time_last_used INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                used_counter INTEGER NOT NULL
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


async fn check_and_recreate_splits_table(db: &Connection) -> tokio_rusqlite::Result<()> {
    db.call(move |conn| {
        conn.execute(&format!(
            "CREATE TABLE IF NOT EXISTS {SPLITS_TABLE_NAME} (
                file_hash TEXT NOT NULL,
                splits TEXT NOT NULL,
                time_added INTEGER NOT NULL,
                time_last_used INTEGER NOT NULL,
                used_counter INTEGER NOT NULL
            )"), [])?;
        conn.execute(&format!(
            "CREATE INDEX IF NOT EXISTS idx_file_hash \
                ON {EMB_TABLE_NAME} (file_hash)"), [],
        )?;
        Ok(())
    }).await
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

        match check_and_recreate_embeddings_table(&cache_database).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }
        match check_and_recreate_splits_table(&cache_database).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }

        Ok(VecDBCache {
            cache_database,
            embedding_size,
        })
    }

    async fn get_records_by_splits(&mut self, splits: &Vec<SplitResult>) -> Result<(Vec<Record>, Vec<SplitResult>), String> {
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
                let time_added_timestamp: i64 = row.get(3)?;
                let time_added = SystemTime::UNIX_EPOCH + Duration::from_secs(time_added_timestamp as u64);
                let time_last_used_timestamp: i64 = row.get(4)?;
                let time_last_used = SystemTime::UNIX_EPOCH + Duration::from_secs(time_last_used_timestamp as u64);
                let model_name: String = row.get(5)?;
                let used_counter: u64 = row.get(6)?;
                Ok((
                    window_text_hash,
                    (vector, window_text, time_added, time_last_used, model_name, used_counter)
                ))
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
                records.push(Record {
                    vector: Some(query_data.0.clone()),
                    window_text: split.window_text.clone(),
                    window_text_hash: split.window_text_hash.clone(),
                    file_path: split.file_path.clone(),
                    start_line: split.start_line,
                    end_line: split.end_line,
                    time_added: query_data.2.clone(),
                    time_last_used: query_data.3.clone(),
                    model_name: query_data.4.clone(),
                    used_counter: query_data.5.clone(),
                    distance: -1.0,
                    usefulness: 0.0,
                })
            } else {
                non_found_splits.push(split.clone());
            }
        }
        Ok((records, non_found_splits))
    }

    async fn insert_records(&mut self, records: Vec<Record>) -> Result<(), String> {
        match self.cache_database.call(|connection| {
            let transaction = connection.transaction()?;
            for record in records {
                let time_added = record.time_added.duration_since(
                    SystemTime::UNIX_EPOCH
                ).unwrap_or(Duration::ZERO)
                    .as_secs();

                let time_last_used = record.time_last_used.duration_since(
                    SystemTime::UNIX_EPOCH
                ).unwrap_or(Duration::ZERO)
                    .as_secs();

                let vector_as_bytes: Vec<u8> = record.vector.expect(
                    "An attempt to push vector-less embeddings to cache DB"
                ).iter()
                    .flat_map(|&num| num.to_ne_bytes())
                    .collect();

                match transaction.execute(&format!(
                    "INSERT INTO {EMB_TABLE_NAME} (vector, window_text, window_text_hash, time_added, \
                    time_last_used, model_name, used_counter) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
                                          rusqlite::params![
                    vector_as_bytes,
                    record.window_text,
                    record.window_text_hash,
                    time_added as i64,
                    time_last_used as i64,
                    record.model_name,
                    record.used_counter,
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

    async fn remove_records(&mut self, file_path: String) -> Result<(), String> {
        match self.cache_database.call(move |connection| {
            match connection.execute(
                &format!("DELETE FROM {EMB_TABLE_NAME} WHERE file_path = ?1"),
                params![file_path],
            ) {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into())
            }
        }).await {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    async fn update_records(&mut self, records: Vec<Record>) -> Result<(), String> {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        match self.cache_database.call(move |connection| {
            let transaction = connection.transaction()?;
            for record in records {
                match transaction.execute(&format!(
                    "UPDATE {EMB_TABLE_NAME} SET time_last_used = ?, used_counter = ?\
                     WHERE window_text_hash = ?"),
                                          params![
                    now,
                    record.used_counter,
                    record.window_text_hash,
                ],
                ) {
                    Ok(_) => {}
                    Err(_) => {
                        continue;
                    }
                };
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

    async fn cleanup_old_records(&mut self) -> Result<(), String> {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        match self.cache_database.call(move |connection| {
            let transaction = connection.transaction()?;

            transaction.execute(
                &format!("DELETE FROM {EMB_TABLE_NAME} \
                WHERE (?1 - time_last_used > ?2) AND (used_counter < ?3)"),
                params![now, TWO_WEEKS, MIN_LIKES],
            )?;

            transaction.execute(
                &format!("DELETE FROM {EMB_TABLE_NAME} WHERE (?1 - time_last_used > ?2)"),
                params![now, ONE_MONTH],
            )?;

            transaction.commit()?;
            Ok({})
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

    pub async fn try_add_from_cache(&mut self, data: Vec<SplitResult>) -> Vec<SplitResult> {
        if data.is_empty() {
            return vec![];
        }

        let (found_records, left_splits) = match self.get_records_by_splits(&data).await {
            Ok(records) => records,
            Err(err) => {
                info!("Error while getting values from cache: {:?}", err);
                return vec![];
            }
        };

        match self.update_records(found_records).await {
            Ok(_) => {}
            Err(err) => info!("Error while adding values from cache: {:?}", err),
        };
        left_splits
    }

    pub async fn update_record_statistic(&mut self, records: Vec<Record>) {
        match self.update_records(records).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from {EMB_TABLE_NAME} table: {:?}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};
    use rusqlite::OpenFlags;
    use tokio_rusqlite::Connection;
    use crate::vecdb::structs::{Record, SplitResult};

    async fn setup_test_db() -> VecDBCache {
        let cache_dir = PathBuf::from("/tmp");
        let model_name = "test_model".to_string();
        let embedding_size = 128;

        VecDBCache::init(&cache_dir, &model_name, embedding_size).await.unwrap()
    }

    #[tokio::test]
    async fn test_init() {
        let cache = setup_test_db().await;
        assert_eq!(cache.embedding_size, 128);
    }

    #[tokio::test]
    async fn test_insert_and_get_records() {
        let mut cache = setup_test_db().await;

        let record = Record {
            vector: Some(vec![0.1, 0.2, 0.3, 0.4]),
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
            time_added: SystemTime::now(),
            time_last_used: SystemTime::now(),
            model_name: "test_model".to_string(),
            used_counter: 1,
            distance: -1.0,
            usefulness: 0.0,
        };

        cache.insert_records(vec![record.clone()]).await.unwrap();

        let splits = vec![SplitResult {
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
        }];

        let (found_records, non_found_splits) = cache.get_records_by_splits(&splits).await.unwrap();
        assert_eq!(found_records.len(), 1);
        assert_eq!(non_found_splits.len(), 0);
        assert_eq!(found_records[0].window_text, "test text");
    }

    #[tokio::test]
    async fn test_remove_records() {
        let mut cache = setup_test_db().await;

        let record = Record {
            vector: Some(vec![0.1, 0.2, 0.3, 0.4]),
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
            time_added: SystemTime::now(),
            time_last_used: SystemTime::now(),
            model_name: "test_model".to_string(),
            used_counter: 1,
            distance: -1.0,
            usefulness: 0.0,
        };

        cache.insert_records(vec![record.clone()]).await.unwrap();
        cache.remove_records("test_file".to_string()).await.unwrap();

        let splits = vec![SplitResult {
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
        }];

        let (found_records, non_found_splits) = cache.get_records_by_splits(&splits).await.unwrap();
        assert_eq!(found_records.len(), 0);
        assert_eq!(non_found_splits.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_old_records() {
        let mut cache = setup_test_db().await;

        let old_time = SystemTime::now() - Duration::from_secs(ONE_MONTH as u64 + 1);

        let record = Record {
            vector: Some(vec![0.1, 0.2, 0.3, 0.4]),
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
            time_added: old_time,
            time_last_used: old_time,
            model_name: "test_model".to_string(),
            used_counter: 1,
            distance: -1.0,
            usefulness: 0.0,
        };

        cache.insert_records(vec![record.clone()]).await.unwrap();
        cache.cleanup_old_records().await.unwrap();

        let splits = vec![SplitResult {
            window_text: "test text".to_string(),
            window_text_hash: "hash1".to_string(),
            file_path: "test_file".to_string(),
            start_line: 1,
            end_line: 2,
        }];

        let (found_records, non_found_splits) = cache.get_records_by_splits(&splits).await.unwrap();
        assert_eq!(found_records.len(), 0);
        assert_eq!(non_found_splits.len(), 1);
    }
}