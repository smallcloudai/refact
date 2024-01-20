use std::any::Any;
use std::collections::HashSet;
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
use lance::dataset::{WriteMode, WriteParams};
use log::info;
use tempfile::{tempdir, TempDir};
use tokio::sync::Mutex as AMutex;
use tracing::error;
use vectordb::database::Database;
use vectordb::table::Table;

use crate::vecdb::structs::{Record, SplitResult};


impl Debug for VecDBHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBHandler: {:?}", self.cache_database.type_id())
    }
}

pub struct VecDBHandler {
    cache_database: Database,
    _data_database_temp_dir: TempDir,
    cache_table: Table,
    data_table: Table,
    schema: SchemaRef,
    data_table_hashes: HashSet<String>,
    embedding_size: i32,
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
            .join(format!("model_{}_esize_{}",
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

        let cache_database = match Database::connect(cache_dir_str.as_str()).await {
            Ok(db) => db,
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
        let cache_table = match cache_database.open_table("data").await {
            Ok(table) => { table }
            Err(_) => {
                let batches_iter = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
                match cache_database.create_table("data", batches_iter, Option::from(WriteParams::default())).await {
                    Ok(table) => table,
                    Err(err) => return Err(format!("{:?}", err))
                }
            }
        };
        let batches_iter = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
        let data_table = match temp_database.create_table("data", batches_iter, Option::from(WriteParams::default())).await {
            Ok(table) => table,
            Err(err) => return Err(format!("{:?}", err))
        };

        Ok(VecDBHandler {
            cache_database,
            _data_database_temp_dir: data_database_temp_dir,
            schema,
            cache_table,
            data_table,
            data_table_hashes: HashSet::new(),
            embedding_size,
        })
    }

    async fn checkout(&mut self) {
        match self.data_table.checkout_latest().await {
            Ok(table) => { self.data_table = table }
            Err(err) => error!("Error while checking out data table: {:?}", err)
        }
        match self.cache_table.checkout_latest().await {
            Ok(table) => { self.cache_table = table }
            Err(err) => error!("Error while checking out data table: {:?}", err)
        }
    }

    pub async fn size(&self) -> Result<usize, String> {
        match self.data_table.count_rows().await {
            Ok(size) => Ok(size),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    pub async fn cache_size(&self) -> Result<usize, String> {
        match self.cache_table.count_rows().await {
            Ok(size) => Ok(size),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    async fn get_records(&mut self, table: Table, _hashes: Vec<String>) -> (Vec<Record>, Vec<String>) {
        let mut hashes: HashSet<String> = HashSet::from_iter(_hashes);
        let q = hashes.iter().map(|x| format!("'{}'", x)).collect::<Vec<String>>().join(", ");
        let records = table
            .filter(format!("window_text_hash in ({})", q))
            .execute()
            .await.unwrap()
            .try_collect::<Vec<_>>()
            .await.unwrap();
        let record_batch = concat_batches(&self.schema, &records).unwrap();
        let records = VecDBHandler::parse_table_iter(record_batch, true, None).unwrap();
        for r in &records {
            hashes.remove(&r.window_text_hash);
        }
        (records, hashes.into_iter().collect())
    }

    pub async fn _get_records_from_data(&mut self, hashes: Vec<String>) -> (Vec<Record>, Vec<String>) {
        self.get_records(self.data_table.clone(), hashes).await
    }
    pub async fn get_records_from_cache(&mut self, hashes: Vec<String>) -> (Vec<Record>, Vec<String>) {
        self.get_records(self.cache_table.clone(), hashes).await
    }

    async fn get_record(&mut self, table: Table, hash: String) -> vectordb::error::Result<Record> {
        let records = table
            .filter(format!("window_text_hash == '{}'", hash))
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        let record_batch = concat_batches(&self.schema, &records)?;
        let records = VecDBHandler::parse_table_iter(record_batch, true, None)?;
        match records.get(0) {
            Some(x) => Ok(x.clone()),
            None => Err(vectordb::error::Error::Lance { message: format!("No record found for hash: {}", hash) })
        }
    }

    pub async fn _get_record_from_data(&mut self, hash: String) -> vectordb::error::Result<Record> {
        self.get_record(self.data_table.clone(), hash).await
    }
    pub async fn _get_record_from_cache(&mut self, hash: String) -> vectordb::error::Result<Record> {
        self.get_record(self.cache_table.clone(), hash).await
    }

    pub async fn try_add_from_cache(&mut self, data: Vec<SplitResult>) -> Vec<SplitResult> {
        if data.is_empty() {
            return vec![];
        }

        let hashes = data.iter().map(|x| x.window_text_hash.clone()).collect();
        let (found_records, left_hashes) = self.get_records_from_cache(hashes).await;
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
        let cache_batches_iter = RecordBatchIterator::new(
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
            let cache_res = self.cache_table.add(
                cache_batches_iter, Option::from(WriteParams {
                    mode: WriteMode::Append,
                    ..Default::default()
                }),
            );
            match cache_res.await {
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

        // valerii: In documentation I found no way to preprocess strings to prevent SQL injections
        match self.cache_table.delete(
            format!("(file_path = \"{}\")", file_path_str).as_str()  // TODO: Prevent a possible sql injection here
        ).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from cache: {:?}", err);
            }
        }
        match self.data_table.delete(
            format!("(file_path = \"{}\")", file_path_str).as_str()  // TODO: Prevent a possible sql injection here
        ).await {
            Ok(_) => {}
            Err(err) => {
                info!("Error while deleting from cache: {:?}", err);
            }
        }
    }

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
            })
        }).collect()
    }

    pub async fn search(&mut self, embedding: Vec<f32>, top_n: usize) -> vectordb::error::Result<Vec<Record>> {
        let query = self.data_table.clone()
            .search(Some(Float32Array::from(embedding.clone())))
            .limit(top_n)
            .use_index(true)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        let record_batch = concat_batches(&self.schema, &query)?;
        VecDBHandler::parse_table_iter(record_batch, false, Some(&embedding))
    }

    pub async fn update_record_statistic(&mut self, records: Vec<Record>) {
        let now = SystemTime::now();
        for record in records {
            for mut table in vec![self.data_table.clone(), self.cache_table.clone()] {
                let _ = table.update(Some(format!("window_text_hash == '{}'", record.window_text_hash.clone()).as_str()),
                                     vec![
                                         ("used_counter", &(&record.used_counter + 1).to_string()),
                                         ("time_last_used", &*now.elapsed().unwrap().as_secs().to_string()),
                                     ]).await.unwrap();
            }
            self.checkout().await;
        }
    }
    pub async fn cleanup_old_records(&mut self) -> Result<(), String> {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        let q = format!("{} - time_last_used > {TWO_WEEKS} AND used_counter < {MIN_LIKES}", now.as_secs());
        self.cache_table.delete(&*q).await.expect("could not delete old records");
        self.data_table.delete(&*q).await.expect("could not delete old records");
        self.checkout().await;

        let q = format!("{} - time_last_used > {ONE_MONTH}", now.as_secs());
        self.cache_table.delete(&*q).await.expect("could not delete old records");
        self.data_table.delete(&*q).await.expect("could not delete old records");
        self.checkout().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::tempdir;
    use tokio;

    use super::*;

    #[tokio::test]
    async fn test_init() {
        let temp_dir = tempdir().unwrap();
        let embedding_size = 2;
        let mut handler = VecDBHandler::init(
            temp_dir.path().to_path_buf(),
            embedding_size,
        ).await;
        assert_eq!(handler.size().await, 0);
    }

    #[tokio::test]
    async fn test_add_or_update() {
        let temp_dir = tempdir().unwrap();
        let embedding_size = 2;
        let mut handler = VecDBHandler::init(
            temp_dir.path().to_path_buf(),
            embedding_size,
        ).await;
        let expected_size = 1;

        // Prepare a sample record
        let records = vec![
            Record {
                vector: Some(vec![1.0, 2.0]), // Example values
                window_text: "sample text".to_string(),
                window_text_hash: "hash1".to_string(),
                file_path: PathBuf::from("/path/to/file"),
                start_line: 1,
                end_line: 2,
                time_added: SystemTime::now(),
                time_last_used: SystemTime::now(),
                model_name: "model1".to_string(),
                used_counter: 0,
                distance: 1.0,
            },
        ];

        // Call add_or_update
        handler.add_or_update(records, true).await.unwrap();

        // Validate the records
        assert_eq!(handler.size().await, expected_size);
    }

    #[tokio::test]
    async fn test_search() {
        let temp_dir = tempdir().unwrap();
        let embedding_size = 4;
        let mut handler = VecDBHandler::init(
            temp_dir.path().to_path_buf(),
            embedding_size,
        ).await;
        let top_n = 1;

        let time_added = SystemTime::now();
        let records = vec![
            Record {
                vector: Some(vec![1.0, 2.0, 3.0, 4.0]),
                window_text: "test text".to_string(),
                window_text_hash: "hash2".to_string(),
                file_path: PathBuf::from("/path/to/another/file"),
                start_line: 3,
                end_line: 4,
                time_added: time_added,
                time_last_used: time_added,
                model_name: "model2".to_string(),
                used_counter: 0,
                distance: 1.0,
            },
        ];
        handler.add_or_update(records, true).await.unwrap();

        let query_embedding = vec![1.0, 2.0, 3.0, 4.0];
        let results = handler.search(query_embedding, top_n).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].window_text, "test text");
        assert_eq!(results[0].window_text_hash, "hash2");
        assert_eq!(results[0].file_path, PathBuf::from("/path/to/another/file"));
        assert_eq!(results[0].start_line, 3);
        assert_eq!(results[0].end_line, 4);
        assert_eq!(results[0].model_name, "model2");
        assert_eq!(results[0].distance, 1.0);
    }
}
