use std::any::Any;
use itertools::Itertools;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use arrow::array::ArrayData;
use arrow::buffer::Buffer;
use arrow::compute::concat_batches;
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray, UInt64Array};
use arrow_array::cast::{as_fixed_size_list_array, as_primitive_array, as_string_array};
use arrow_array::types::{Float32Type, UInt64Type};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures_util::TryStreamExt;
use lance::dataset::{WriteMode, WriteParams};
use tempfile::{tempdir, TempDir};
use vectordb::database::Database;
use vectordb::table::Table;

use crate::vecdb::vdb_structs::VecdbRecord;


impl Debug for VecDBHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VecDBHandler: {:?}", self.data_table.type_id())
    }
}

pub struct VecDBHandler {
    _data_database_temp_dir: TempDir,
    data_table: Table,
    schema: SchemaRef,
    // data_table_hashes: HashSet<String>,
    embedding_size: i32,
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2).map(|(x, y)| x * y).sum();
    let magnitude_vec1: f32 = vec1.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let magnitude_vec2: f32 = vec2.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    dot_product / (magnitude_vec1 * magnitude_vec2)
}

pub fn cosine_distance(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    1.0 - cosine_similarity(vec1, vec2)
}


impl VecDBHandler {
    pub async fn init(embedding_size: i32) -> Result<VecDBHandler, String> {
        let data_database_temp_dir = match tempdir() {
            Ok(dir) => dir,
            Err(_) => return Err(format!("{:?}", "Error creating temp dir")),
        };
        let data_database_temp_dir_str = match data_database_temp_dir.path().to_str() {
            Some(path) => path,
            None => return Err(format!("{:?}", "Temp directory is not a valid path")),
        };
        let temp_database = match Database::connect(data_database_temp_dir_str).await {
            Ok(db) => db,
            Err(err) => return Err(format!("{:?}", err))
        };

        let vec_trait = Arc::new(Field::new("item", DataType::Float32, true));
        let schema = Arc::new(Schema::new(vec![
            Field::new("vector", DataType::FixedSizeList(vec_trait, embedding_size), true),
            // Field::new("window_text", DataType::Utf8, true),
            // Field::new("window_text_hash", DataType::Utf8, true),
            Field::new("scope", DataType::Utf8, true),
            Field::new("start_line", DataType::UInt64, true),
            Field::new("end_line", DataType::UInt64, true),
        ]));

        let batches_iter = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
        let data_table = match temp_database.create_table("data", batches_iter, Option::from(WriteParams::default())).await {
            Ok(table) => table,
            Err(err) => return Err(format!("{:?}", err))
        };

        Ok(VecDBHandler {
            _data_database_temp_dir: data_database_temp_dir,
            schema,
            data_table,
            // data_table_hashes: HashSet::new(),
            embedding_size,
        })
    }

    pub async fn size(&self) -> Result<usize, String> {
        match self.data_table.count_rows().await {
            Ok(size) => Ok(size),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    pub async fn vecdb_records_add(&mut self, records: &Vec<VecdbRecord>)
    {
        fn make_emb_data(records: &Vec<VecdbRecord>, embedding_size: i32) -> Result<ArrayData, String> {
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
                .build()
            {
                Ok(res) => Ok(res),
                Err(err) => return Err(format!("{:?}", err))
            }
        }

        if records.is_empty() {
            return;
        }

        let vectors: ArrayData = match make_emb_data(&records, self.embedding_size) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("{:?}", err);
                return;
            }
        };
        let scopes: Vec<String> = records.iter().map(|x| x.file_path.to_str().unwrap_or("No filename").to_string()).collect();
        let start_lines: Vec<u64> = records.iter().map(|x| x.start_line).collect();
        let end_lines: Vec<u64> = records.iter().map(|x| x.end_line).collect();
        let data_batches_iter = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                self.schema.clone(),
                vec![
                    Arc::new(FixedSizeListArray::from(vectors.clone())),
                    Arc::new(StringArray::from(scopes.clone())),
                    Arc::new(UInt64Array::from(start_lines.clone())),
                    Arc::new(UInt64Array::from(end_lines.clone())),
                ],
            )],
            self.schema.clone(),
        );

        tracing::info!("vecdb_records_add: adding {} records", records.len());
        if let Err(err) = self.data_table.add(
            data_batches_iter, Option::from(WriteParams {
                mode: WriteMode::Append,
                ..Default::default()
            }),
        ).await {
            tracing::error!("{}", err);
        }
    }

    pub async fn vecdb_records_remove(
        &mut self,
        scopes_to_remove: Vec<String>
    ) {
        let mut delete_queries = Vec::new();

        for chunk in &scopes_to_remove.iter().chunks(100) {
            let paths_to_remove: Vec<&String> = chunk.collect();
            let formatted_scopes: String = paths_to_remove
                .iter()
                .map(|scope| format!("'{}'", scope.replace("'", "''")))
                .join(", ");
            let delete_query = format!("scope IN ({})", formatted_scopes);
            delete_queries.push(delete_query);
        }

        for delete_query in delete_queries {
            tracing::info!("delete: {}", delete_query.as_str());
            match self.data_table.delete(delete_query.as_str()).await {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("Error deleting from vecdb: {:?}", err);
                }
            }
            let cnt = self.data_table.count_deleted_rows().await.unwrap();
            tracing::info!("deleted {} records", cnt);
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

    fn parse_table_iter(
        record_batch: RecordBatch,
        include_embedding: bool,
        embedding_to_compare: Option<&Vec<f32>>,
    ) -> vectordb::error::Result<Vec<VecdbRecord>> {
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

            Ok(VecdbRecord {
                vector: embedding,
                file_path: PathBuf::from(as_string_array(record_batch.column_by_name("scope")
                    .expect("Missing column 'scope'"))
                    .value(idx)
                    .to_string()),
                start_line: as_primitive_array::<UInt64Type>(record_batch.column_by_name("start_line")
                    .expect("Missing column 'start_line'"))
                    .value(idx),
                end_line: as_primitive_array::<UInt64Type>(record_batch.column_by_name("end_line")
                    .expect("Missing column 'end_line'"))
                    .value(idx),
                distance,
                usefulness: 0.0,
            })
        }).collect()
    }

    pub async fn vecdb_search(
        &mut self,
        embedding: &Vec<f32>,
        top_n: usize,
        vecdb_scope_filter_mb: Option<String>,
    ) -> vectordb::error::Result<Vec<VecdbRecord>> {
        let use_prefilter = vecdb_scope_filter_mb.is_some();
        let query = self
            .data_table
            .clone()
            .search(Some(Float32Array::from(embedding.clone())))
            .prefilter(use_prefilter)
            .filter(vecdb_scope_filter_mb)
            .limit(top_n)
            .use_index(true)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        let record_batch = concat_batches(&self.schema, &query)?;
        match VecDBHandler::parse_table_iter(record_batch, false, Some(&embedding)) {
            Ok(records) => {
                let filtered: Vec<VecdbRecord> = records
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
}
