use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use arrow::array::ArrayData;
use arrow::buffer::Buffer;
use arrow::compute::concat_batches;
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray, UInt64Array};
use arrow_array::cast::{as_fixed_size_list_array, as_primitive_array, as_string_array};
use arrow_array::types::{Float32Type, UInt64Type};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures_util::TryStreamExt;
use itertools::Itertools;
use lance::dataset::{WriteMode, WriteParams};
use rusqlite::{OpenFlags, params, Result};
use tempfile::{tempdir, TempDir};
use tokio::fs;
use tokio::sync::Mutex as AMutex;
use tokio_rusqlite::Connection;
use tracing::error;
use tracing::info;
use vectordb::database::Database;
use vectordb::table::Table;

use crate::vecdb::structs::{Record, SplitResult};

impl Debug for VecDBHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBHandler: {:?}", self.cache_database.type_id())
    }
}

pub struct VecDBHandler {
    cache_database: Arc<AMutex<Connection>>,
    _data_database_temp_dir: TempDir,
    data_table: Table,
    schema: SchemaRef,
    data_table_hashes: HashSet<String>,
    embedding_size: i32,
    indexed_file_paths: Arc<AMutex<Vec<PathBuf>>>,
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2).map(|(x, y)| x * y).sum();
    let magnitude_vec1: f32 = vec1.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let magnitude_vec2: f32 = vec2.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    dot_product / (magnitude_vec1 * magnitude_vec2)
}

fn cosine_distance(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    1.0 - cosine_similarity(vec1, vec2)
}

const TWO_WEEKS: i32 = 2 * 7 * 24 * 3600;
const ONE_MONTH: i32 = 30 * 24 * 3600;
const MIN_LIKES: i32 = 3;

impl VecDBHandler {
    pub async fn init(cache_dir: &PathBuf, model_name: &String, embedding_size: i32) -> Result<VecDBHandler, String> {
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
        let data_database_temp_dir = match tempdir() {
            Ok(dir) => dir,
            Err(_) => return Err(format!("{:?}", "Error creating temp dir")),
        };
        let data_database_temp_dir_str = match data_database_temp_dir.path().to_str() {
            Some(path) => path,
            None => return Err(format!("{:?}", "Temp directory is not a valid path")),
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
            Ok(db) => Arc::new(AMutex::new(db)),
            Err(err) => return Err(format!("{:?}", err))
        };
        let temp_database = match Database::connect(data_database_temp_dir_str).await {
            Ok(db) => db,
            Err(err) => return Err(format!("{:?}", err))
        };

        let vec_trait = Arc::new(Field::new("item", DataType::Float32, true));
        let schema = Arc::new(Schema::new(vec![
            Field::new("vector", DataType::FixedSizeList(vec_trait, embedding_size), true),
            Field::new("window_text", DataType::Utf8, true),
            Field::new("window_text_hash", DataType::Utf8, true),
            Field::new("file_path", DataType::Utf8, true),
            Field::new("start_line", DataType::UInt64, true),
            Field::new("end_line", DataType::UInt64, true),
            Field::new("time_added", DataType::UInt64, true),
            Field::new("time_last_used", DataType::UInt64, true),
            Field::new("model_name", DataType::Utf8, true),
            Field::new("used_counter", DataType::UInt64, true),
        ]));
        match cache_database.lock().await.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS data (
                        vector BLOB,
                        window_text TEXT NOT NULL,
                        window_text_hash TEXT NOT NULL,
                        time_added INTEGER NOT NULL,
                        time_last_used INTEGER NOT NULL,
                        model_name TEXT NOT NULL,
                        used_counter INTEGER NOT NULL
                    )", [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_window_text_hash ON data (window_text_hash)",
                [],
            )?;
            Ok(())
        }).await {
            Ok(_) => {}
            Err(err) => return Err(format!("{:?}", err))
        }

        let batches_iter = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
        let data_table = match temp_database.create_table("data", batches_iter, Option::from(WriteParams::default())).await {
            Ok(table) => table,
            Err(err) => return Err(format!("{:?}", err))
        };

        Ok(VecDBHandler {
            cache_database,
            _data_database_temp_dir: data_database_temp_dir,
            schema,
            data_table,
            data_table_hashes: HashSet::new(),
            embedding_size,
            indexed_file_paths: Arc::new(AMutex::new(vec![])),
        })
    }

    async fn checkout(&mut self) {
        match self.data_table.checkout_latest().await {
            Ok(table) => { self.data_table = table }
            Err(err) => error!("Error while checking out the data table: {:?}", err)
        }
        match self.cache_database.lock().await.call(|connection| {
            connection.cache_flush()?;
            Ok({})
        }).await {
            Ok(_) => {}
            Err(err) => error!("Error while flushing cache: {:?}", err)
        }
    }

    async fn get_records_from_cache(&mut self, splits: &Vec<SplitResult>) -> Result<(Vec<Record>, Vec<String>), String> {
        let mut hashes_by_split = splits
            .iter()
            .map(|x| (x.window_text_hash.clone(), x.clone()))
            .collect::<HashMap<String, SplitResult>>();
        let placeholders: String = splits.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!("SELECT * FROM data WHERE window_text_hash IN ({})", placeholders);

        let hashes_by_split_clone = hashes_by_split.clone();
        let records = match self.cache_database.lock().await.call(move |connection| {
            let mut statement = connection.prepare(&query)?;
            let params = rusqlite::params_from_iter(hashes_by_split_clone.keys());
            let records = statement.query_map(params, |row| {
                let vector_blob: Vec<u8> = row.get(0)?;
                let vector: Vec<f32> = vector_blob
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();

                let window_text_hash: String = row.get(2)?;
                let time_added_timestamp: i64 = row.get(3)?;
                let time_added = SystemTime::UNIX_EPOCH + Duration::from_secs(time_added_timestamp as u64);

                let time_last_used_timestamp: i64 = row.get(4)?;
                let time_last_used = SystemTime::UNIX_EPOCH + Duration::from_secs(time_last_used_timestamp as u64);

                let split = hashes_by_split_clone
                    .get(&window_text_hash)
                    .expect("An attempt to get split from cache DB");
                Ok(Record {
                    vector: Some(vector),
                    window_text: row.get(1)?,
                    window_text_hash: window_text_hash,
                    file_path: split.file_path.clone(),
                    start_line: split.start_line,
                    end_line: split.end_line,
                    time_added: time_added,
                    time_last_used: time_last_used,
                    model_name: row.get(5)?,
                    used_counter: row.get(6)?,
                    distance: -1.0,
                    usefulness: 0.0,
                })
            })?
                .filter_map(|row| row.ok())
                .collect::<Vec<Record>>();
            Ok(records)
        }).await {
            Ok(records) => records,
            Err(err) => return Err(format!("{:?}", err))
        };

        for r in &records {
            hashes_by_split.remove(&r.window_text_hash);
        }
        Ok((records, hashes_by_split.iter().map(|(hash, _)| hash.clone()).collect()))
    }

    async fn insert_records_to_cache(&mut self, records: Vec<Record>) -> Result<(), String> {
        match self.cache_database.lock().await.call(|connection| {
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
                    "An attempt to push vector-less data to cache DB"
                ).iter()
                    .flat_map(|&num| num.to_ne_bytes())
                    .collect();

                match transaction.execute(
                    "INSERT INTO data (vector, window_text, window_text_hash, time_added, \
                    time_last_used, model_name, used_counter) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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

    #[allow(unused)]
    async fn remove_records_from_cache(&mut self, file_path: String) -> Result<(), String> {
        match self.cache_database.lock().await.call(move |connection| {
            match connection.execute(
                "DELETE FROM data WHERE file_path = ?1",
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

    async fn update_cache_records(&mut self, records: Vec<Record>) -> Result<(), String> {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        match self.cache_database.lock().await.call(move |connection| {
            let transaction = connection.transaction()?;
            for record in records {
                match transaction.execute(
                    "UPDATE data SET time_last_used = ?, used_counter = ? WHERE window_text_hash = ?",
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

    async fn delete_old_records_from_cache(&mut self) -> Result<(), String> {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        match self.cache_database.lock().await.call(move |connection| {
            let transaction = connection.transaction()?;

            transaction.execute(
                "DELETE FROM data WHERE (?1 - time_last_used > ?2) AND (used_counter < ?3)",
                params![now, TWO_WEEKS, MIN_LIKES],
            )?;

            transaction.execute(
                "DELETE FROM data WHERE (?1 - time_last_used > ?2)",
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
        match self.data_table.count_rows().await {
            Ok(size) => Ok(size),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    pub async fn cache_size(&self) -> Result<usize, String> {
        self.cache_database.lock().await.call(move |connection| {
            let mut stmt = connection.prepare("SELECT COUNT(*) FROM data")?;
            let count: usize = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }).await
            .map_err(|e| {
                e.to_string()
            })
    }

    pub async fn select_all_file_paths(&self) -> Vec<PathBuf> {
        let mut file_paths: HashSet<PathBuf> = HashSet::new();
        let records: Vec<RecordBatch> = self.data_table
            .filter(format!("file_path in (select file_path from data)"))
            .execute()
            .await.unwrap()
            .try_collect::<Vec<_>>()
            .await.unwrap();

        for rec_batch in records {
            for record in VecDBHandler::parse_table_iter(rec_batch, false, None).unwrap() {
                file_paths.insert(record.file_path.clone());
            }
        }
        return file_paths.into_iter().collect();
    }

    pub async fn update_indexed_file_paths(&mut self) {
        let res = self.select_all_file_paths().await;
        self.indexed_file_paths = Arc::new(AMutex::new(res));
    }

    // pub async fn get_indexed_file_paths(&self) -> Arc<AMutex<Vec<PathBuf>>> {
    //     return self.indexed_file_paths.clone();
    // }

    pub async fn try_add_from_cache(&mut self, data: Vec<SplitResult>) -> Vec<SplitResult> {
        if data.is_empty() {
            return vec![];
        }

        let (found_records, left_hashes) = match self.get_records_from_cache(&data).await {
            Ok(records) => records,
            Err(err) => {
                info!("Error while getting values from cache: {:?}", err);
                return vec![];
            }
        };
        let left_results: Vec<SplitResult> =
            data.into_iter().filter(|x| left_hashes.contains(&x.window_text_hash)).collect();

        match self.add_or_update(found_records, false).await {
            Ok(_) => {}
            Err(err) => info!("Error while adding values from cache: {:?}", err),
        };
        left_results
    }

    pub async fn add_or_update(&mut self, records: Vec<Record>, add_to_cache: bool) -> Result<(), String> {
        fn make_emb_data(records: &Vec<Record>, embedding_size: i32) -> Result<ArrayData, String> {
            let vec_trait = Arc::new(Field::new("item", DataType::Float32, true));
            let mut emb_builder: Vec<f32> = vec![];

            for record in records {
                emb_builder.append(&mut record.vector.clone().expect("No embedding is provided"));
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
                .build() {
                Ok(res) => Ok(res),
                Err(err) => return Err(format!("{:?}", err))
            }
        }

        if records.is_empty() {
            return Ok(());
        }

        let vectors: ArrayData = match make_emb_data(&records, self.embedding_size) {
            Ok(res) => res,
            Err(err) => return Err(format!("{:?}", err))
        };
        let window_texts: Vec<String> = records.iter().map(|x| x.window_text.clone()).collect();
        let window_text_hashes: Vec<String> = records.iter().map(|x| x.window_text_hash.clone()).collect();
        let file_paths: Vec<String> = records.iter().map(|x| x.file_path.to_str().unwrap_or("No filename").to_string()).collect();
        let start_lines: Vec<u64> = records.iter().map(|x| x.start_line).collect();
        let end_lines: Vec<u64> = records.iter().map(|x| x.end_line).collect();
        let time_adds: Vec<u64> = records.iter().map(|x| x.time_added.duration_since(std::time::UNIX_EPOCH).unwrap_or(
            Duration::from_secs(0)
        ).as_secs()).collect();
        let time_last_used: Vec<u64> = records.iter().map(|x| x.time_last_used.duration_since(std::time::UNIX_EPOCH).unwrap_or(
            Duration::from_secs(0)
        ).as_secs()).collect();
        let model_names: Vec<String> = records.iter().map(|x| x.model_name.clone()).collect();
        let used_counters: Vec<u64> = records.iter().map(|x| x.used_counter).collect();
        let data_batches_iter = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                self.schema.clone(),
                vec![
                    Arc::new(FixedSizeListArray::from(vectors.clone())),
                    Arc::new(StringArray::from(window_texts.clone())),
                    Arc::new(StringArray::from(window_text_hashes.clone())),
                    Arc::new(StringArray::from(file_paths.clone())),
                    Arc::new(UInt64Array::from(start_lines.clone())),
                    Arc::new(UInt64Array::from(end_lines.clone())),
                    Arc::new(UInt64Array::from(time_adds.clone())),
                    Arc::new(UInt64Array::from(time_last_used.clone())),
                    Arc::new(StringArray::from(model_names.clone())),
                    Arc::new(UInt64Array::from(used_counters.clone())),
                ],
            )],
            self.schema.clone(),
        );
        RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                self.schema.clone(),
                vec![
                    Arc::new(FixedSizeListArray::from(vectors)),
                    Arc::new(StringArray::from(window_texts)),
                    Arc::new(StringArray::from(window_text_hashes.clone())),
                    Arc::new(StringArray::from(file_paths)),
                    Arc::new(UInt64Array::from(start_lines)),
                    Arc::new(UInt64Array::from(end_lines)),
                    Arc::new(UInt64Array::from(time_adds)),
                    Arc::new(UInt64Array::from(time_last_used)),
                    Arc::new(StringArray::from(model_names)),
                    Arc::new(UInt64Array::from(used_counters)),
                ],
            )],
            self.schema.clone(),
        );

        if add_to_cache {
            match self.insert_records_to_cache(records).await {
                Ok(_) => {}
                Err(err) => return Err(format!("{:?}", err))
            };
        }

        let data_res = self.data_table.add(
            data_batches_iter, Option::from(WriteParams {
                mode: WriteMode::Append,
                ..Default::default()
            }),
        );
        self.data_table_hashes.extend(window_text_hashes);
        match data_res.await {
            Ok(_) => Ok(()),
            Err(err) => return Err(format!("{:?}", err))
        }
    }

    pub async fn remove(&mut self, file_path: &PathBuf) {
        let file_path_str = match file_path.to_str() {
            None => {
                info!("File path is not a string");
                return;
            }
            Some(res) => res
        };

        match self.remove_records_from_cache(file_path_str.to_string()).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from cache table: {:?}", err);
            }
        }
        // valerii: In documentation I found no way to preprocess strings to prevent SQL injections
        match self.data_table.delete(
            format!("(file_path = \"{}\")", file_path_str).as_str()  // TODO: Prevent a possible sql injection here
        ).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from data table: {:?}", err);
            }
        }
    }

    // pub async fn create_index(&mut self) -> vectordb::error::Result<()> {
    //     let size = self.size().await.unwrap_or(0);
    //     if size == 0 {
    //         return Ok(());
    //     }
    //     self.data_table.create_index(
    //         IvfPQIndexBuilder::default()
    //             .column("vector".to_owned())
    //             .index_name("index".to_owned())
    //             .metric_type(MetricType::Cosine)
    //             .ivf_params(IvfBuildParams {
    //                 num_partitions: min(size, 512),
    //                 ..IvfBuildParams::default()
    //             })
    //             .replace(true)
    //     ).await
    // }

    pub fn contains(&self, hash: &str) -> bool {
        self.data_table_hashes.contains(hash)
    }

    fn parse_table_iter(
        record_batch: RecordBatch,
        include_embedding: bool,
        embedding_to_compare: Option<&Vec<f32>>,
    ) -> vectordb::error::Result<Vec<Record>> {
        (0..record_batch.num_rows()).map(|idx| {
            let gathered_vec = as_primitive_array::<Float32Type>(
                &as_fixed_size_list_array(record_batch.column_by_name("vector").unwrap())
                    .iter()
                    .map(|x| x.unwrap())
                    .collect::<Vec<_>>()[idx]
            )
                .iter()
                .map(|x| x.unwrap()).collect();
            let distance = match embedding_to_compare {
                None => { -1.0 }
                Some(embedding) => { cosine_distance(&embedding, &gathered_vec) }
            };
            let embedding = match include_embedding {
                true => Some(gathered_vec),
                false => None
            };

            Ok(Record {
                vector: embedding,
                window_text: as_string_array(record_batch.column_by_name("window_text")
                    .expect("Missing column 'window_text'"))
                    .value(idx)
                    .to_string(),
                window_text_hash: as_string_array(record_batch.column_by_name("window_text_hash")
                    .expect("Missing column 'window_text_hash'"))
                    .value(idx)
                    .to_string(),
                file_path: PathBuf::from(as_string_array(record_batch.column_by_name("file_path")
                    .expect("Missing column 'file_path'"))
                    .value(idx)
                    .to_string()),
                start_line: as_primitive_array::<UInt64Type>(record_batch.column_by_name("start_line")
                    .expect("Missing column 'start_line'"))
                    .value(idx),
                end_line: as_primitive_array::<UInt64Type>(record_batch.column_by_name("end_line")
                    .expect("Missing column 'end_line'"))
                    .value(idx),
                time_added: std::time::UNIX_EPOCH + Duration::from_secs(
                    as_primitive_array::<UInt64Type>(
                        record_batch.column_by_name("time_added")
                            .expect("Missing column 'time_added'"))
                        .value(idx)
                ),
                time_last_used: std::time::UNIX_EPOCH + Duration::from_secs(
                    as_primitive_array::<UInt64Type>(
                        record_batch.column_by_name("time_last_used")
                            .expect("Missing column 'time_last_used'"))
                        .value(idx)
                ),
                model_name: as_string_array(record_batch.column_by_name("model_name")
                    .expect("Missing column 'model_name'"))
                    .value(idx)
                    .to_string(),
                used_counter: as_primitive_array::<UInt64Type>(record_batch.column_by_name("used_counter")
                    .expect("Missing column 'used_counter'"))
                    .value(idx),
                distance,
                usefulness: 0.0,
            })
        }).collect()
    }

    pub async fn search(
        &mut self,
        embedding: Vec<f32>,
        top_n: usize
    ) -> vectordb::error::Result<Vec<Record>> {
        let query = self
            .data_table
            .clone()
            .search(Some(Float32Array::from(embedding.clone())))
            .limit(top_n)
            .use_index(true)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        let record_batch = concat_batches(&self.schema, &query)?;
        match VecDBHandler::parse_table_iter(record_batch, false, Some(&embedding)) {
            Ok(records) => {
                let filtered: Vec<Record> = records
                    .into_iter()
                    .dedup()
                    .sorted_unstable_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .collect();
                Ok(filtered)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn update_record_statistic(&mut self, records: Vec<Record>) {
        match self.update_cache_records(records).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from data table: {:?}", err);
            }
        }
    }

    pub async fn cleanup_old_records(&mut self) -> Result<(), String> {
        info!("VECDB: Cleaning up old records");

        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        let q = format!("{} - time_last_used > {TWO_WEEKS} AND used_counter < {MIN_LIKES}", now.as_secs());
        self.data_table.delete(&*q).await.expect("could not delete old records");

        let q = format!("{} - time_last_used > {ONE_MONTH}", now.as_secs());
        self.data_table.delete(&*q).await.expect("could not delete old records");

        self.delete_old_records_from_cache().await.expect("could not delete old records");
        self.checkout().await;
        Ok(())
    }
}
