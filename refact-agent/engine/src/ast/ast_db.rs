use std::time::Instant;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::fs;
use std::path::Path;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use tokio::task;
use serde_cbor;
use heed::EnvOpenOptions;
use heed::types::Bytes;
use lazy_static::lazy_static;
use regex::Regex;

// Macro to measure database operation time and log warnings if it exceeds 1 second
macro_rules! measure_db_op {
    ($op_name:expr, $code:expr) => {{
        let start = Instant::now();
        let result = $code;
        let elapsed = start.elapsed();
        if elapsed.as_secs_f64() > 1.0 {
            tracing::warn!("DB operation '{}' took {:.3}s", $op_name, elapsed.as_secs_f64());
        }
        result
    }};
}

use crate::ast::ast_structs::{AstDB, AstDefinition, AstCounters, AstErrorStats};
use crate::ast::ast_parse_anything::{parse_anything_and_add_file_path, filesystem_path_to_double_colon_path};
use crate::fuzzy_search::fuzzy_search;

// ## How the database works ##
//
// Database `sled` used here is a key-value storage, everything is stored as keys and values. Try dump_database() below.
//
// All the definitions are serialized under d| like this:
//   d|alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice
//   AstDefinition { alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice, usages: Link{ up alt_testsuite::cpp_goat_main::CosmicJustice::balance } }
//
// You can look up a shorter path than the full path, by using c| records:
//   c|main::goat1 ⚡ alt_testsuite::cpp_goat_main::main::goat1
//     ^^^^^^^^^^^ short path that maps to full path
//
// Usages are stored as:
//   u|$K2dgAI::alt_testsuite::cpp_goat_main::all_goats_say_hi ⚡ $K2dgAI::alt_testsuite::cpp_goat_main::main (2 bytes)
//     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ file path
//     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ usage of what? (full path)                     ^^^^^^^ uline is serialized in value
//                                                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path to where the usage is
//
// Homeless usages:
//   homeless|printf ⚡ $K2dgAI::alt_testsuite::cpp_goat_main::main (2 bytes)
//            ^^^^^^ something unknown                              ^^^^^^^ uline
//                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path to where the usage is
//
// Resolve todo:
//   resolve-todo|alt_testsuite::cpp_goat_library::Animal::self_review
//                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path the class
//   resolve-cleanup|alt_testsuite::cpp_goat_library::Animal::self_review
//
// Class hierarchy:
//   classes|cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
//           ^^^^^^^^^^^ derived from                               ^^^^^^^^^^ serialized value, klass
//                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path of klass, makes those keys additive
//
// Per doc records:
//   doc-cpath|alt_testsuite::cpp_goat_library 👉 src/ast/alt_testsuite/cpp_goat_library.h
//             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ file_global_path (means path up to the global scope of the file)
//                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ file filesystem path
//
// Other keys:
//   counters|defs: 42
//   counters|usages: 100
//
//
// Read tests below, the show what this index can do!


const A_LOT_OF_PRINTS: bool = false;

macro_rules! debug_print {
    ($($arg:tt)*) => {
        if A_LOT_OF_PRINTS {
            tracing::info!($($arg)*);
        }
    };
}

const MAP_SIZE: usize = 10 * 1024 * 1024 * 1024; // 10GB max database size

pub async fn ast_index_init(ast_permanent: String, ast_max_files: usize, want_perf_report: bool) -> Arc<AstDB>
{
    let db_path = if ast_permanent.is_empty() {
        let temp_dir = std::env::temp_dir().join("refact-lmdb-temp");
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir).unwrap();
        }
        temp_dir.to_str().unwrap().to_string()
    } else {
        // Make sure the directory exists
        let path = Path::new(&ast_permanent);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }
        ast_permanent.clone()
    };

    tracing::info!("starting AST db, ast_permanent={:?}", db_path);
    let env = task::spawn_blocking(move || {
        let mut options = EnvOpenOptions::new();
        options.map_size(MAP_SIZE);
        options.max_dbs(100);
        if want_perf_report {
            tracing::info!("LMDB performance reporting enabled");
        }

        // Try to open the environment, handling the EnvAlreadyOpened error
        let result = unsafe { options.open(&db_path) };
        match result {
            Ok(env) => env,
            Err(heed::Error::Mdb(heed::MdbError::Other(code))) if code == -30784 => {
                // -30784 is the code for MDB_MAP_RESIZED in LMDB, which can happen when another process already opened the env
                tracing::warn!("Environment already opened with a different map size, trying to reopen");
                // Add a small delay to allow other processes to close the environment
                std::thread::sleep(std::time::Duration::from_millis(100));
                unsafe { options.open(&db_path).unwrap() }
            },
            Err(heed::Error::EnvAlreadyOpened) => {
                // If the environment is already opened, try to close and reopen it
                tracing::warn!("Environment already opened, trying with a unique path");
                // Create a unique path by appending a timestamp
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let unique_path = format!("{}-{}", db_path, timestamp);

                // Make sure the directory exists
                let path = Path::new(&unique_path);
                if !path.exists() {
                    fs::create_dir_all(path).unwrap();
                }

                // Try to open with the unique path
                unsafe { options.open(&unique_path).unwrap() }
            },
            Err(e) => {
                tracing::error!("Failed to open LMDB environment: {:?}", e);
                panic!("Failed to open LMDB environment: {:?}", e);
            }
        }
    }).await.unwrap();

    let env = Arc::new(env);

    // Create the main database in a write transaction
    let main_db: heed::Database<heed::types::Str, Bytes> = env
        .write_txn()
        .and_then(|mut txn| {
            let db = env.create_database(&mut txn, Some("main_db"))?;
            txn.commit().map(|_| db)
        })
        .unwrap();

    // Clear the database (equivalent of db.clear())
    let _: Result<(), _> = env.write_txn().and_then(|mut txn| {
        main_db.clear(&mut txn)?;
        txn.commit()
    });

    tracing::info!("/starting AST");
    let ast_index = AstDB {
        env: env,
        main_db,
        batch_counter: AMutex::new(0),
        counters_increase: AMutex::new(HashMap::new()),
        ast_max_files,
    };
    Arc::new(ast_index)
}

pub async fn fetch_counters(ast_index: Arc<AstDB>) -> AstCounters
{
    let env = ast_index.env.clone();
    let db = ast_index.main_db;

    let counter_defs = {
        let txn = env.read_txn().unwrap();
        db.get(&txn, "counters|defs")
            .unwrap()
            .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
            .unwrap_or(0)
    };

    let counter_usages = {
        let txn = env.read_txn().unwrap();
        db.get(&txn, "counters|usages")
            .unwrap()
            .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
            .unwrap_or(0)
    };

    let counter_docs = {
        let txn = env.read_txn().unwrap();
        db.get(&txn, "counters|docs")
            .unwrap()
            .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
            .unwrap_or(0)
    };

    AstCounters {
        counter_defs,
        counter_usages,
        counter_docs,
    }
}

fn _increase_counter_commit(env: &heed::Env, db: &heed::Database<heed::types::Str, Bytes>, counter_key: &str, adjustment: i32) {
    if adjustment == 0 {
        return;
    }

    let result = env.write_txn().and_then(|mut txn| {
        // Get the current value
        let current = db.get(&txn, counter_key)
            .unwrap_or(None)
            .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
            .unwrap_or(0);

        // Calculate the new value
        let new_value = current + adjustment;

        // Store the new value
        db.put(&mut txn, counter_key, &serde_cbor::to_vec(&new_value).unwrap())?;

        // Commit the transaction
        txn.commit()
    });

    if let Err(e) = result {
        tracing::error!("failed to update and fetch counter: {:?}", e);
    }
}

async fn _increase_counter(ast_index: Arc<AstDB>, counter_key: &str, adjustment: i32) {
    if adjustment == 0 {
        return;
    }
    let mut counters_increase = ast_index.counters_increase.lock().await;
    let counter = counters_increase.entry(counter_key.to_string()).or_insert(0);
    *counter += adjustment;
}

// In LMDB, we use a transaction-based approach instead of batches
pub async fn flush_db_changes(
    ast_db: Arc<AstDB>,
    threshold: usize,   // if zero, flush everything including counters
) -> Arc<AMutex<HashMap<String, Vec<u8>>>> {
    measure_db_op!("flush_db_changes", {
        let mut batch_counter = ast_db.batch_counter.lock().await;

        if *batch_counter >= threshold {
            let env = ast_db.env.clone();
            let db = ast_db.main_db;

            // Reset batch counter
            let was_counter = *batch_counter;
            *batch_counter = 0;

            // Get the counters to process
            let mut counters_increase = ast_db.counters_increase.lock().await;
            let counters_to_process = if threshold == 0 {
                std::mem::replace(&mut *counters_increase, HashMap::new())
            } else {
                HashMap::new()
            };
            drop(counters_increase);

            // Process counter updates
            if was_counter > 0 || !counters_to_process.is_empty() {
                measure_db_op!("flush_db_changes.process_counters", {
                    for (counter_key, adjustment) in counters_to_process {
                        _increase_counter_commit(&env, &db, &counter_key, adjustment);
                    }
                });
            }

            // Return a dummy Arc Mutex to maintain compatibility with existing code
            return Arc::new(AMutex::new(HashMap::new()));
        }

        *batch_counter += 1;
        Arc::new(AMutex::new(HashMap::new()))
    })
}

pub async fn doc_add(
    ast_index: Arc<AstDB>,
    cpath: &String,
    text: &String,
    errors: &mut AstErrorStats,
) -> Result<(Vec<Arc<AstDefinition>>, String), String>
{
    measure_db_op!("doc_add", {
        let file_global_path = filesystem_path_to_double_colon_path(cpath);
        let (defs, language) = parse_anything_and_add_file_path(&cpath, text, errors)?;   // errors mostly "no such parser" here
        let env = ast_index.env.clone();
        let db = ast_index.main_db;

        // Start a write transaction
        let result = env.write_txn().and_then(|mut txn| {
            let mut added_defs: i32 = 0;
            let mut added_usages: i32 = 0;
            let mut unresolved_usages: i32 = 0;

            for definition in defs.iter() {
                assert!(definition.cpath == *cpath);
                let serialized = serde_cbor::to_vec(&definition).unwrap();
                let official_path = definition.official_path.join("::");
                let d_key = format!("d|{}", official_path);
                debug_print!("writing {}", d_key);
                db.put(&mut txn, &d_key, &serialized)?;

                let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
                while !path_parts.is_empty() {
                    let c_key = format!("c|{} ⚡ {}", path_parts.join("::"), official_path);
                    db.put(&mut txn, &c_key, b"")?;
                    path_parts.remove(0);
                }

                for usage in &definition.usages {
                    if !usage.resolved_as.is_empty() {
                        let u_key = format!("u|{} ⚡ {}", usage.resolved_as, official_path);
                        db.put(&mut txn, &u_key, &serde_cbor::to_vec(&usage.uline).unwrap())?;
                    } else if usage.targets_for_guesswork.len() == 1 && !usage.targets_for_guesswork[0].starts_with("?::") {
                        let homeless_key = format!("homeless|{} ⚡ {}", usage.targets_for_guesswork[0], official_path);
                        db.put(&mut txn, &homeless_key, &serde_cbor::to_vec(&usage.uline).unwrap())?;
                        debug_print!("        homeless {}", homeless_key);
                        continue;
                    } else {
                        unresolved_usages += 1;
                    }
                    added_usages += 1;
                }

                // this_is_a_class: cpp🔎CosmicGoat, derived_from: "cpp🔎Goat" "cpp🔎CosmicJustice"
                for from in &definition.this_class_derived_from {
                    let t_key = format!("classes|{} ⚡ {}", from, official_path);
                    db.put(&mut txn, &t_key, definition.this_is_a_class.as_bytes())?;
                }
                added_defs += 1;
            }

            if unresolved_usages > 0 {
                let resolve_todo_key = format!("resolve-todo|{}", file_global_path.join("::"));
                db.put(&mut txn, &resolve_todo_key, cpath.as_bytes())?;
            }

            let doc_key = format!("doc-cpath|{}", file_global_path.join("::"));
            if db.get(&txn, &doc_key)?.is_none() {
                // Increment docs counter
                let counter_key = "counters|docs";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current + 1)).unwrap())?;

                db.put(&mut txn, &doc_key, cpath.as_bytes())?;
            }

            // Update counters
            if added_defs > 0 {
                let counter_key = "counters|defs";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current + added_defs)).unwrap())?;
            }

            if added_usages > 0 {
                let counter_key = "counters|usages";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current + added_usages)).unwrap())?;
            }

            // Commit the transaction
            txn.commit()
        });

        if let Err(e) = result {
            return Err(format!("Failed to add document: {:?}", e));
        }

        Ok((defs.into_iter().map(Arc::new).collect(), language))
    })
}

pub async fn doc_remove(ast_index: Arc<AstDB>, cpath: &String)
{
    measure_db_op!("doc_remove", {
        let file_global_path = filesystem_path_to_double_colon_path(cpath);
        let d_prefix = format!("d|{}::", file_global_path.join("::"));
        let env = ast_index.env.clone();
        let db = ast_index.main_db;

        // First, collect all keys to delete
        let mut keys_to_delete = Vec::new();
        let mut definitions_to_process = Vec::new();

        // Read transaction to collect data
        let _ = env.read_txn().and_then(|txn| {
            // Create a cursor to iterate through keys with the prefix
            let mut cursor = db.prefix_iter(&txn, &d_prefix)?;

            while let Some(result) = cursor.next() {
                if let Ok((key, value)) = result {
                    keys_to_delete.push(key.to_string());
                    if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(value) {
                        definitions_to_process.push(definition);
                    }
                }
            }

            Ok(())
        });

        // Now process the collected data in a write transaction
        let result = env.write_txn().and_then(|mut txn| {
            let mut deleted_defs: i32 = 0;
            let mut deleted_usages: i32 = 0;

            for definition in definitions_to_process {
                let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
                let official_path = definition.official_path.join("::");

                // Remove c| keys
                while !path_parts.is_empty() {
                    let c_key = format!("c|{} ⚡ {}", path_parts.join("::"), official_path);
                    db.delete(&mut txn, &c_key)?;
                    path_parts.remove(0);
                }

                // Remove usage keys
                for usage in &definition.usages {
                    if !usage.resolved_as.is_empty() {
                        let u_key = format!("u|{} ⚡ {}", usage.resolved_as, official_path);
                        db.delete(&mut txn, &u_key)?;
                    } else if usage.targets_for_guesswork.len() == 1 && !usage.targets_for_guesswork[0].starts_with("?::") {
                        let homeless_key = format!("homeless|{} ⚡ {}", usage.targets_for_guesswork[0], official_path);
                        db.delete(&mut txn, &homeless_key)?;
                        debug_print!("        homeless {}", homeless_key);
                        continue;
                    }
                    deleted_usages += 1;
                }

                // Remove class hierarchy keys
                for from in &definition.this_class_derived_from {
                    let t_key = format!("classes|{} ⚡ {}", from, official_path);
                    db.delete(&mut txn, &t_key)?;
                }

                // Handle cleanup keys
                let cleanup_key = format!("resolve-cleanup|{}", definition.official_path.join("::"));
                if let Some(cleanup_value) = db.get(&txn, &cleanup_key)? {
                    if let Ok(all_saved_ulinks) = serde_cbor::from_slice::<Vec<String>>(cleanup_value) {
                        for ulink in all_saved_ulinks {
                            db.delete(&mut txn, &ulink)?;
                        }
                    } else {
                        tracing::error!("failed to deserialize cleanup_value for key: {}", cleanup_key);
                    }
                    db.delete(&mut txn, &cleanup_key)?;
                }

                deleted_defs += 1;
            }

            // Delete all collected d| keys
            for key in keys_to_delete {
                debug_print!("removing {}", key);
                db.delete(&mut txn, &key)?;
            }

            // Delete doc_resolved key
            let doc_resolved_key = format!("doc-resolved|{}", file_global_path.join("::"));
            db.delete(&mut txn, &doc_resolved_key)?;

            // Delete doc_cpath key and update counter
            let doc_key = format!("doc-cpath|{}", file_global_path.join("::"));
            if db.get(&txn, &doc_key)?.is_some() {
                // Decrement docs counter
                let counter_key = "counters|docs";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current - 1)).unwrap())?;

                db.delete(&mut txn, &doc_key)?;
            }

            // Update counters
            if deleted_defs > 0 {
                let counter_key = "counters|defs";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current - deleted_defs)).unwrap())?;
            }

            if deleted_usages > 0 {
                let counter_key = "counters|usages";
                let current = db.get(&txn, counter_key)?
                    .map(|v| serde_cbor::from_slice::<i32>(v).unwrap())
                    .unwrap_or(0);
                db.put(&mut txn, counter_key, &serde_cbor::to_vec(&(current - deleted_usages)).unwrap())?;
            }

            // Commit the transaction
            txn.commit()
        });

        if let Err(e) = result {
            tracing::error!("Failed to remove document: {:?}", e);
        }
    })
}

pub async fn doc_defs(ast_index: Arc<AstDB>, cpath: &String, info: bool) -> Vec<Arc<AstDefinition>>
{
    let env = ast_index.env.clone();
    let db = ast_index.main_db;
    doc_def_internal(env, db, cpath, info).await
}

pub async fn doc_def_internal(env: Arc<heed::Env>, db: heed::Database<heed::types::Str, Bytes>, cpath: &String, info: bool) -> Vec<Arc<AstDefinition>>
{
    measure_db_op!("doc_def_internal", {
        let to_search_prefix = filesystem_path_to_double_colon_path(cpath);
        let d_prefix = format!("d|{}::", to_search_prefix.join("::"));
        let mut defs = Vec::new();
        let t0 = tokio::time::Instant::now();

        if info { tracing::info!("Starting prefix scan for {}", d_prefix); }

        // Use a read transaction to get all matching keys
        let values = measure_db_op!("doc_def_internal.spawn_blocking", {
            tokio::task::spawn_blocking(move || {
                let mut collected = Vec::new();
                let mut seg_count = 0;

                // Create a read transaction
                if let Ok(txn) = env.read_txn() {
                    // Create a cursor to iterate through keys with the prefix
                    if let Ok(mut cursor) = db.prefix_iter(&txn, &d_prefix) {
                        while let Some(Ok((_, value))) = cursor.next() {
                            seg_count += 1;
                            if info { tracing::info!("segment {} processed", seg_count); }
                            collected.push(value.to_vec());
                        }
                    }
                }

                (collected, seg_count)
            }).await.unwrap()
        });

        let (values, seg_count) = values;
        if info {
            tracing::info!("got {} segments", seg_count);
        }

        // Process the collected values
        for value in values {
            if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(&value) {
                defs.push(Arc::new(definition));
            }
        }

        if info { tracing::info!("db look took {}s", t0.elapsed().as_secs_f64()); }
        defs
    })
}

pub async fn doc_usages(ast_index: Arc<AstDB>, cpath: &String) -> Vec<(usize, String)>
{
    let definitions = doc_defs(ast_index.clone(), cpath, false).await;
    let env = ast_index.env.clone();
    let db = ast_index.main_db;
    let mut usages = Vec::new();

    // Simple usages
    for def in definitions {
        for usage in &def.usages {
            if !usage.resolved_as.is_empty() {
                usages.push((usage.uline, usage.resolved_as.clone()));
            }
        }
    }

    // Scan for usages that needed resolving
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let doc_resolved_key = format!("doc-resolved|{}", file_global_path.join("::"));

    // Use a read transaction to get the resolved usages
    if let Ok(txn) = env.read_txn() {
        if let Ok(Some(resolved_usages)) = db.get(&txn, &doc_resolved_key) {
            if let Ok(resolved_usages_vec) = serde_cbor::from_slice::<Vec<(usize, String)>>(resolved_usages) {
                usages.extend(resolved_usages_vec);
            }
        }
    }

    usages
}

pub struct ConnectUsageContext {
    pub derived_from_map: IndexMap<String, Vec<String>>,
    pub errstats: AstErrorStats,
    pub usages_homeless: usize,
    pub usages_connected: usize,
    pub usages_not_found: usize,
    pub usages_ambiguous: usize,
    pub t0: Instant,
}

pub async fn connect_usages(ast_index: Arc<AstDB>, ucx: &mut ConnectUsageContext) -> bool
{
    measure_db_op!("connect_usages", {
        let env = ast_index.env.clone();
        let db = ast_index.main_db;

        // First, find a resolve-todo key
        let mut todo_key_to_process = None;
        let mut todo_value_to_process = None;

        // Use a read transaction to find a resolve-todo key
        if let Ok(txn) = env.read_txn() {
            if let Ok(mut cursor) = db.prefix_iter(&txn, "resolve-todo|") {
                if let Some(Ok((key, value))) = cursor.next() {
                    todo_key_to_process = Some(key.to_string());
                    todo_value_to_process = Some(value.to_vec());
                }
            }
        }

        if let (Some(todo_key_string), Some(todo_value)) = (todo_key_to_process, todo_value_to_process) {
            let global_file_path = todo_key_string.strip_prefix("resolve-todo|").unwrap();
            let cpath = String::from_utf8(todo_value).unwrap();
            debug_print!("resolving {}", cpath);

            // Delete the todo key immediately
            let delete_result = env.write_txn().and_then(|mut txn| {
                db.delete(&mut txn, &todo_key_string)?;
                txn.commit()
            });

            if let Err(e) = delete_result {
                tracing::error!("connect_usages() failed to remove resolve-todo key: {:?}", e);
            }

            // Process the definitions
            let definitions = doc_defs(ast_index.clone(), &cpath.to_string(), false).await;

            // Start a write transaction for the batch operations
            let mut resolved_usages: Vec<(usize, String)> = vec![];

            for def in definitions {
                let tmp = _connect_usages_helper(env.clone(), db, ucx, &def).await;
                resolved_usages.extend(tmp);
            }

            // Write the resolved usages
            let write_result = env.write_txn().and_then(|mut txn| {
                let doc_resolved_key = format!("doc-resolved|{}", global_file_path);
                db.put(&mut txn, &doc_resolved_key, &serde_cbor::to_vec(&resolved_usages).unwrap())?;
                txn.commit()
            });

            if let Err(e) = write_result {
                tracing::error!("connect_usages() failed to write resolved usages: {:?}", e);
            }

            return true;
        }

        false
    })
}

pub async fn connect_usages_look_if_full_reset_needed(ast_index: Arc<AstDB>) -> ConnectUsageContext
{
    measure_db_op!("connect_usages_look_if_full_reset_needed", {
        // Ensure all pending changes are committed
        flush_db_changes(ast_index.clone(), 0).await;

        let env = ast_index.env.clone();
        let db = ast_index.main_db;
        let class_hierarchy_key = "class-hierarchy|";

        // Get existing hierarchy
        let existing_hierarchy: IndexMap<String, Vec<String>> = measure_db_op!("connect_usages_look_if_full_reset_needed.get", {
            let txn_result = env.read_txn();
            if let Ok(txn) = txn_result {
                if let Ok(Some(value)) = db.get(&txn, class_hierarchy_key) {
                    serde_cbor::from_slice(value).unwrap_or_default()
                } else {
                    IndexMap::new()
                }
            } else {
                IndexMap::new()
            }
        });

        // Get new derived from map
        let new_derived_from_map = _derived_from(&env, db).await;

        // Start a write transaction for batch operations
        let result = env.write_txn().and_then(|mut txn| {
            if existing_hierarchy.is_empty() {
                // First run, store the hierarchy
                let serialized_hierarchy = serde_cbor::to_vec(&new_derived_from_map).unwrap();
                db.put(&mut txn, class_hierarchy_key, &serialized_hierarchy)?;
                // First run, do nothing because all the definitions are already in the todo list
            } else if new_derived_from_map != existing_hierarchy {
                // Class hierarchy changed, update it
                tracing::info!(" * * * class hierarchy changed {} classes => {} classes, all usages need to be reconnected * * *",
                    existing_hierarchy.len(), new_derived_from_map.len());

                let serialized_hierarchy = serde_cbor::to_vec(&new_derived_from_map).unwrap();
                db.put(&mut txn, class_hierarchy_key, &serialized_hierarchy)?;

                // Add all documents to resolve-todo
                let mut items_to_add = Vec::new();
                let mut cnt = 0;

                // Use a separate scope to ensure cursor is dropped before we modify txn
                {
                    let mut cursor = db.prefix_iter(&txn, "doc-cpath|")?;

                    // Collect items to add to resolve-todo
                    while let Some(Ok((key, value))) = cursor.next() {
                        if let Some(file_global_path) = key.strip_prefix("doc-cpath|") {
                            let cpath = std::str::from_utf8(value).unwrap_or_default();
                            let resolve_todo_key = format!("resolve-todo|{}", file_global_path);
                            items_to_add.push((resolve_todo_key, cpath.to_string()));
                            cnt += 1;
                        }
                    }
                } // cursor is dropped here, freeing the immutable borrow

                // Add collected items to resolve-todo
                for (key, value) in items_to_add {
                    db.put(&mut txn, &key, value.as_bytes())?;
                }

                tracing::info!("added {} items to resolve-todo", cnt);
            }

            // Commit the transaction
            txn.commit()
        });

        if let Err(e) = result {
            tracing::error!("connect_usages_look_if_full_reset_needed() failed: {:?}", e);
        }

        ConnectUsageContext {
            derived_from_map: new_derived_from_map,
            errstats: AstErrorStats::default(),
            usages_homeless: 0,
            usages_connected: 0,
            usages_not_found: 0,
            usages_ambiguous: 0,
            t0: Instant::now(),
        }
    })
}

lazy_static! {
    static ref MAGNIFYING_GLASS_RE: Regex = Regex::new(r"(\w+)🔎(\w+)").unwrap();
}

async fn _connect_usages_helper(
    env: Arc<heed::Env>,
    db: heed::Database<heed::types::Str, Bytes>,
    ucx: &mut ConnectUsageContext,
    definition: &AstDefinition
) -> Vec<(usize, String)> {
    // Use the implementation from ast_db_heed_helpers.rs
    crate::ast::ast_db_heed_helpers::connect_usages_helper(env, db, ucx, definition).await
}

// Data example:
    // (1) c/Animal::self_review ⚡ alt_testsuite::cpp_goat_library::Animal::self_review
    // (2) c/cpp_goat_library::Animal::self_review ⚡ alt_testsuite::cpp_goat_library::Animal::self_review
    // (3) c/self_review ⚡ alt_testsuite::cpp_goat_library::Animal::self_review
    // (4) d/alt_testsuite::cpp_goat_library::Animal::self_review
    //   AstDefinition { alt_testsuite::cpp_goat_library::Animal::self_review, usages: U{ up file::Animal::age } }
    // (5) d/alt_testsuite::cpp_goat_library::Goat::jump_around
    //   AstDefinition { alt_testsuite::cpp_goat_library::Goat::jump_around, usages: U{ n2p ?::cpp🔎Goat::self_review ?::self_review } U{ n2p ?::cpp🔎Goat::age ?::age } U{ up file::Goat::weight } }
    //
    // Example of usage to resolve:
    // U{ n2p ?::cpp🔎Goat::self_review ?::self_review }
    // first, try ?::cpp🔎Goat::self_review, according to type hierarchy Goat is derived from Animal, therefore full list to try:
    //   Goat::self_review
    //   Animal::self_review -- matches (1)
    //   self_review -- matches (3)
    //
    // The longer the matched path, the more reliable it is. The `targets_for_guesswork` field is constructed in such a way that it starts
    // with longer paths.
    //
    // Usage data:
    //   u/file::Animal::age ⚡ alt_testsuite::cpp_goat_library::Animal::self_review
    // means `age` was used in self_review(). This all goes to the key, value contains a line number.
    //
    // Saved data by this function:
    //   u/RESOLVED ⚡ official_path        -- value has line number uline
    //   resolve-cleanup/official_path     -- value contains all the "u|RESOLVED ⚡ official_path" in a list
    //
//     let official_path = definition.official_path.join("::");
//     let mut result = Vec::<(usize, String)>::new();
//     let mut all_saved_ulinks = Vec::<String>::new();
//     for (uindex, usage) in definition.usages.iter().enumerate() {
//         debug_print!("    resolving {}.usage[{}] == {:?}", official_path, uindex, usage);
//         if !usage.resolved_as.is_empty() {
//             ucx.usages_connected += 1;
//             continue;
//         }
//         for to_resolve_unstripped in &usage.targets_for_guesswork {
//             if !to_resolve_unstripped.starts_with("?::") {
//                 debug_print!("    homeless {}", to_resolve_unstripped);
//                 ucx.usages_homeless += 1;
//                 continue;
//             }
//             let to_resolve = to_resolve_unstripped.strip_prefix("?::").unwrap();
//             // println!("to_resolve_unstripped {:?}", to_resolve_unstripped);
//             debug_print!("    to resolve {}.usage[{}] guessing {}", official_path, uindex, to_resolve);

//             // Extract all LANGUAGE🔎CLASS from to_resolve
//             let mut magnifying_glass_pairs = Vec::new();
//             let mut template = to_resolve.to_string();
//             for (i, cap) in MAGNIFYING_GLASS_RE.captures_iter(to_resolve).enumerate() {
//                 let language = cap.get(1).unwrap().as_str().to_string();
//                 let klass = cap.get(2).unwrap().as_str().to_string();
//                 let placeholder = format!("%%PAIR{}%%", i);
//                 template = template.replacen(&format!("{}🔎{}", language, klass), &placeholder, 1);
//                 magnifying_glass_pairs.push((language, klass));
//             }
//             let mut variants = Vec::<String>::new();
//             if magnifying_glass_pairs.len() == 0 {
//                 variants.push(to_resolve.to_string());
//             } else {
//                 let substitutions_of_each_pair: Vec<Vec<String>> = magnifying_glass_pairs.iter().map(|(language, klass)| {
//                     let mut substitutions = ucx.derived_from_map.get(format!("{}🔎{}", language, klass).as_str()).cloned().unwrap_or_else(|| vec![]);
//                     substitutions.insert(0, klass.clone());
//                     substitutions.iter().map(|s| s.strip_prefix(&format!("{}🔎", language)).unwrap_or(s).to_string()).collect()
//                 }).collect();

//                 fn generate_combinations(substitutions: &[Vec<String>], index: usize, current: Vec<String>) -> Vec<Vec<String>> {
//                     if index == substitutions.len() {
//                         return vec![current];
//                     }
//                     let mut result = Vec::new();
//                     for substitution in &substitutions[index] {
//                         let mut new_current = current.clone();
//                         new_current.push(substitution.clone());
//                         result.extend(generate_combinations(substitutions, index + 1, new_current));
//                     }
//                     result
//                 }
//                 let intermediate_results = generate_combinations(&substitutions_of_each_pair, 0, Vec::new());
//                 // Transform each something::LANGUAGE🔎CLASS::something into something::class::something
//                 for intermediate_result in intermediate_results {
//                     let mut variant = template.clone();
//                     for (i, substitution) in intermediate_result.iter().enumerate() {
//                         let placeholder = format!("%%PAIR{}%%", i);
//                         variant = variant.replacen(&placeholder, substitution, 1);
//                     }
//                     variants.push(variant);
//                 }
//                 // ?::cpp🔎Goat::self_review magnifying_glass_pairs [("cpp", "Goat")]
//                 //   substitutions_of_each_pair [["Goat", "Animal"]]
//                 //   intermediate_results [["Goat"], ["Animal"]]
//                 //   variants possible ["Goat::self_review", "Animal::self_review"]
//             }

//             let mut found = Vec::new();
//             for v in variants {
//                 let c_prefix = format!("c|{}", v);
//                 debug_print!("        scanning {}", c_prefix);
//                 // println!("    c_prefix {:?} because v={:?}", c_prefix, v);
//                 let mut c_iter = db.scan_prefix(&c_prefix);
//                 while let Some(Ok((c_key, _))) = c_iter.next() {
//                     let c_key_string = String::from_utf8(c_key.to_vec()).unwrap();
//                     let parts: Vec<&str> = c_key_string.split(" ⚡ ").collect();
//                     if parts.len() == 2 {
//                         if parts[0] == c_prefix {
//                             let resolved_target = parts[1].trim();
//                             found.push(resolved_target.to_string());
//                         }
//                     }
//                 }
//                 if found.len() > 0 {
//                     break;
//                 }
//             }
//             debug_print!("        found {:?}", found);

//             if found.len() == 0 {
//                 ucx.usages_not_found += 1;
//                 continue;
//             }
//             if found.len() > 1 {
//                 ucx.errstats.add_error(definition.cpath.clone(), usage.uline, &format!("usage `{}` is ambiguous, can mean: {:?}", to_resolve, found));
//                 ucx.usages_ambiguous += 1;
//                 found.truncate(1);
//             }
//             let single_thing_found = found.into_iter().next().unwrap();
//             let u_key = format!("u|{} ⚡ {}", single_thing_found, official_path);
//             batch.insert(u_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
//             debug_print!("        add {:?} <= {}", u_key, usage.uline);
//             all_saved_ulinks.push(u_key);
//             result.push((usage.uline, single_thing_found));
//             ucx.usages_connected += 1;
//             break;  // the next thing from targets_for_guesswork is a worse query, keep this one and exit
//         }
//     } // for usages
//     let cleanup_key = format!("resolve-cleanup|{}", definition.official_path.join("::"));
//     let cleanup_value = serde_cbor::to_vec(&all_saved_ulinks).unwrap();
//     batch.insert(cleanup_key.as_bytes(), cleanup_value.as_slice());
//     result
// }

async fn _derived_from(env: &heed::Env, db: heed::Database<heed::types::Str, Bytes>) -> IndexMap<String, Vec<String>>
{
    // Use the implementation from ast_db_heed_helpers.rs
    crate::ast::ast_db_heed_helpers::derived_from(env, db).await
}
        // Data example:
        // classes/cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
    //     let mut derived_map: IndexMap<String, Vec<String>> = IndexMap::new();
    //     let t_prefix = "classes|";

    //     // Create a read transaction
    //     if let Ok(txn) = env.read_txn() {
    //         // Create a cursor to iterate through keys with the prefix
    //         if let Ok(mut cursor) = db.prefix_iter(&txn, t_prefix) {
    //             while let Some(Ok((key, value))) = cursor.next() {
    //                 let key_string = key.to_string();
    //                 let value_string = std::str::from_utf8(value).unwrap_or_default().to_string();
    //                 let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
    //                 if parts.len() == 2 {
    //                     let parent = parts[0].trim().strip_prefix(t_prefix).unwrap_or(parts[0].trim()).to_string();
    //                     let child = value_string.trim().to_string();
    //                     let entry = derived_map.entry(child).or_insert_with(Vec::new);
    //                     if !entry.contains(&parent) {
    //                         entry.push(parent);
    //                     }
    //                 } else {
    //                     tracing::warn!("bad key {}", key_string);
    //                 }
    //             }
    //         }
    //     }
    // // Have perfectly good [child, [parent1, parent2, ..]]
    // // derived_map {"cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat"]}
    // // Now we need to post-process this into [child, [parent1, parent_of_parent1, parent2, parent_of_parent2, ...]]
    // fn build_all_derived_from(
    //     klass: &str,
    //     derived_map: &IndexMap<String, Vec<String>>,
    //     all_derived_from: &mut IndexMap<String, Vec<String>>,
    //     visited: &mut HashSet<String>,
    // ) -> Vec<String> {
    //     if visited.contains(klass) {
    //         return all_derived_from.get(klass).cloned().unwrap_or_default();
    //     }
    //     visited.insert(klass.to_string());
    //     let mut all_parents = Vec::new();
    //     if let Some(parents) = derived_map.get(klass) {
    //         for parent in parents {
    //             all_parents.push(parent.clone());
    //             let ancestors = build_all_derived_from(parent, derived_map, all_derived_from, visited);
    //             for ancestor in ancestors {
    //                 if !all_parents.contains(&ancestor) {
    //                     all_parents.push(ancestor);
    //                 }
    //             }
    //         }
    //     }
    //     all_derived_from.insert(klass.to_string(), all_parents.clone());
    //     all_parents
    // }
    // let mut all_derived_from: IndexMap<String, Vec<String>> = IndexMap::new();
    // for klass in derived_map.keys() {
    //     let mut visited: HashSet<String> = HashSet::new();
    //     build_all_derived_from(klass, &derived_map, &mut all_derived_from, &mut visited);
    // }
    // // now have all_derived_from {"cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat", "cpp🔎Animal"], "cpp🔎CosmicJustice": [], "cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎Animal": []}
    // all_derived_from
    // })
// }

pub async fn usages(ast_index: Arc<AstDB>, full_official_path: String, limit_n: usize) -> Vec<(Arc<AstDefinition>, usize)>
{
    measure_db_op!("usages", {
        // The best way to get full_official_path is to call definitions() first
        let env = ast_index.env.clone();
        let db = ast_index.main_db;
        let mut usages = Vec::new();
        let u_prefix1 = format!("u|{} ", full_official_path); // this one has space
        let u_prefix2 = format!("u|{}", full_official_path);

        // Create a read transaction
        if let Ok(txn) = env.read_txn() {
            // Create a cursor to iterate through keys with the prefix
            if let Ok(mut cursor) = db.prefix_iter(&txn, &u_prefix1) {
                while let Some(Ok((key, value))) = cursor.next() {
                    if usages.len() >= limit_n {
                        break;
                    }

                    let key_string = key.to_string();
                    let uline: usize = serde_cbor::from_slice(value).unwrap_or(0); // Assuming `uline` is stored in the value
                    let parts: Vec<&str> = key_string.split(" ⚡ ").collect();

                    if parts.len() == 2 && parts[0] == u_prefix2 {
                        let full_path = parts[1].trim();
                        let d_key = format!("d|{}", full_path);

                        if let Ok(Some(d_value)) = db.get(&txn, &d_key) {
                            match serde_cbor::from_slice::<AstDefinition>(d_value) {
                                Ok(definition) => {
                                    usages.push((Arc::new(definition), uline));
                                },
                                Err(e) => println!("Failed to deserialize value for {}: {:?}", d_key, e),
                            }
                        }
                    } else if parts.len() != 2 {
                        tracing::error!("usage record has more than two ⚡ key was: {}", key_string);
                    }
                }
            }
        }

        usages
    })
}

pub async fn definitions(ast_index: Arc<AstDB>, double_colon_path: &str) -> Vec<Arc<AstDefinition>>
{
    measure_db_op!("definitions", {
        let env = ast_index.env.clone();
        let db = ast_index.main_db;
        let c_prefix1 = format!("c|{} ", double_colon_path); // has space
        let c_prefix2 = format!("c|{}", double_colon_path);
        let mut path_groups: HashMap<usize, Vec<String>> = HashMap::new();

        // Create a read transaction
        if let Ok(txn) = env.read_txn() {
            // Create a cursor to iterate through keys with the prefix
            if let Ok(mut cursor) = db.prefix_iter(&txn, &c_prefix1) {
                while let Some(Ok((key, _))) = cursor.next() {
                    let key_string = key.to_string();
                    if key_string.contains(" ⚡ ") {
                        let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
                        if parts.len() == 2 && parts[0] == c_prefix2 {
                            let full_path = parts[1].trim().to_string();
                            let colon_count = full_path.matches("::").count();
                            path_groups.entry(colon_count).or_insert_with(Vec::new).push(full_path);
                        } else if parts.len() != 2 {
                            tracing::error!("c-record has more than two ⚡ key was: {}", key_string);
                        }
                    } else {
                        tracing::error!("c-record doesn't have ⚡ key: {}", key_string);
                    }
                }
            }

            // Process the collected paths
            let min_colon_count = path_groups.keys().min().cloned().unwrap_or(usize::MAX);
            let mut defs = Vec::new();

            if let Some(paths) = path_groups.get(&min_colon_count) {
                for full_path in paths {
                    let d_key = format!("d|{}", full_path);
                    if let Ok(Some(d_value)) = db.get(&txn, &d_key) {
                        match serde_cbor::from_slice::<AstDefinition>(d_value) {
                            Ok(definition) => defs.push(Arc::new(definition)),
                            Err(e) => println!("Failed to deserialize value for {}: {:?}", d_key, e),
                        }
                    }
                }
            }

            return defs;
        }

        // Return empty vector if transaction failed
        Vec::new()
    })
}

#[allow(dead_code)]
pub async fn type_hierarchy(ast_index: Arc<AstDB>, language: String, subtree_of: String) -> String
{
    measure_db_op!("type_hierarchy", {
        // Data example:
        // classes/cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
        // classes/cpp🔎CosmicJustice ⚡ alt_testsuite::cpp_goat_main::CosmicGoat 👉 "cpp🔎CosmicGoat"
        // classes/cpp🔎Goat ⚡ alt_testsuite::cpp_goat_main::CosmicGoat 👉 "cpp🔎CosmicGoat"
    //
    // Output for that data:
    // type_hierarchy("cpp", "")
    // Animal
    //    Goat
    //       CosmicGoat
    // CosmicJustice
    //    CosmicGoat
    //
    // Output for that data:
    // type_hierarchy("cpp", "CosmicJustice")
    // CosmicJustice
    //    CosmicGoat
    //
    let env = ast_index.env.clone();
    let db = ast_index.main_db;
    let t_prefix = format!("classes|{}", language);
    let mut hierarchy_map: IndexMap<String, Vec<String>> = IndexMap::new();

    // Create a read transaction
    if let Ok(txn) = env.read_txn() {
        // Create a cursor to iterate through keys with the prefix
        if let Ok(mut cursor) = db.prefix_iter(&txn, &t_prefix) {
            while let Some(Ok((key, value))) = cursor.next() {
                let key_string = key.to_string();
                let value_string = std::str::from_utf8(value).unwrap_or_default().to_string();
                if key_string.contains(" ⚡ ") {
                    let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
                    if parts.len() == 2 {
                        let parent = parts[0].trim().strip_prefix("classes|").unwrap_or(parts[0].trim()).to_string();
                        let child = value_string.trim().to_string();
                        hierarchy_map.entry(parent).or_insert_with(Vec::new).push(child);
                    }
                }
            }
        }
    }

    fn build_hierarchy(hierarchy_map: &IndexMap<String, Vec<String>>, node: &str, indent: usize, language: &str) -> String {
        let prefix = format!("{}🔎", language);
        let node_stripped = node.strip_prefix(&prefix).unwrap_or(node);
        let mut result = format!("{:indent$}{}\n", "", node_stripped, indent = indent);
        if let Some(children) = hierarchy_map.get(node) {
            for child in children {
                result.push_str(&build_hierarchy(hierarchy_map, child, indent + 2, language));
            }
        }
        result
    }

    let mut result = String::new();
    if subtree_of.is_empty() {
        for root in hierarchy_map.keys() {
            if !hierarchy_map.values().any(|children| children.contains(root)) {
                result.push_str(&build_hierarchy(&hierarchy_map, root, 0, &language));
            }
        }
    } else {
        result.push_str(&build_hierarchy(&hierarchy_map, &subtree_of, 0, &language));
    }

    result
    })
}

pub async fn definition_paths_fuzzy(ast_index: Arc<AstDB>, pattern: &str, top_n: usize, max_candidates_to_consider: usize) -> Vec<String> {
    measure_db_op!("definition_paths_fuzzy", {
        let env = ast_index.env.clone();
        let db = ast_index.main_db;
        let mut candidates = HashSet::new();
        let mut patterns_to_try = Vec::new();

        // Prepare patterns to try
        let parts: Vec<&str> = pattern.split("::").collect();
        for i in 0..parts.len() {
            patterns_to_try.push(parts[i..].join("::"));
        }

        if let Some(symbol_name_part) = parts.last() {
            let mut symbol_name = symbol_name_part.to_string();
            while !symbol_name.is_empty() {
                patterns_to_try.push(symbol_name.clone());
                let _ = symbol_name.split_off(symbol_name.len() / 2);
            }
        }

        // Create a read transaction
        if let Ok(txn) = env.read_txn() {
            for pat in patterns_to_try {
                let c_prefix = format!("c|{}", pat);

                // Create a cursor to iterate through keys with the prefix
                if let Ok(mut cursor) = db.prefix_iter(&txn, &c_prefix) {
                    while let Some(Ok((key, _))) = cursor.next() {
                        let key_string = key.to_string();
                        if let Some((_, dest)) = key_string.split_once(" ⚡ ") {
                            candidates.insert(dest.to_string());
                        }
                        if candidates.len() >= max_candidates_to_consider {
                            break;
                        }
                    }
                }

                if candidates.len() >= max_candidates_to_consider {
                    break;
                }
            }
        }

        // Perform fuzzy search on collected candidates
        let results = fuzzy_search(&pattern.to_string(), candidates, top_n, &[':']);

        // Process and return results
        results.into_iter()
            .map(|result| {
                if let Some(pos) = result.find("::") {
                    result[pos + 2..].to_string()
                } else {
                    result
                }
            })
            .collect()
    })
}

#[allow(dead_code)]
pub async fn dump_database(ast_index: Arc<AstDB>) -> usize
{
    measure_db_op!("dump_database", {
        let env = ast_index.env.clone();
        let db = ast_index.main_db;

        // Count records
        let mut count = 0;

        // Create a read transaction
        if let Ok(txn) = env.read_txn() {
            println!("\nDumping database records");

            // Create a cursor to iterate through all keys
            if let Ok(mut cursor) = db.iter(&txn) {
                while let Some(Ok((key, value))) = cursor.next() {
                    count += 1;
                    let key_string = key.to_string();

                    if key_string.starts_with("d|") {
                        match serde_cbor::from_slice::<AstDefinition>(value) {
                            Ok(definition) => println!("{} 👉 {:.50}", key_string, format!("{:?}", definition)),
                            Err(e) => println!("Failed to deserialize value at {}: {:?}", key_string, e),
                        }
                    } else if key_string.starts_with("classes|") {
                        let value_string = std::str::from_utf8(value).unwrap_or_default();
                        println!("{} 👉 {:?}", key_string, value_string);
                    } else if key_string.starts_with("counters|") {
                        if let Ok(counter_value) = serde_cbor::from_slice::<i32>(value) {
                            println!("{}: {}", key_string, counter_value);
                        } else {
                            println!("{}: <invalid counter value>", key_string);
                        }
                    } else if !value.is_empty() {
                        println!("{} ({} bytes)", key_string, value.len());
                    } else {
                        println!("{}", key_string);
                    }
                }
            }
        }

        println!("dump_database over, {} records", count);
        count
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_subscriber;
    use std::io::stderr;
    use tracing_subscriber::fmt::format;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_writer(stderr)
            .with_max_level(tracing::Level::INFO)
            .event_format(format::Format::default())
            .try_init();
    }

    fn read_file(file_path: &str) -> String {
        fs::read_to_string(file_path).expect("Unable to read file")
    }

    async fn run_ast_db_test(
        ast_index: Arc<AstDB>,
        library_file_path: &str,
        main_file_path: &str,
        goat_location: &str,
        language: &str,
        animal_age_location: &str,
    ) {
        let mut errstats: AstErrorStats = AstErrorStats::default();

        let library_text = read_file(library_file_path);
        let main_text = read_file(main_file_path);
        doc_add(ast_index.clone(), &library_file_path.to_string(), &library_text, &mut errstats).await.unwrap();
        doc_add(ast_index.clone(), &main_file_path.to_string(), &main_text, &mut errstats).await.unwrap();

        for error in errstats.errors {
            println!("(E) {}:{} {}", error.err_cpath, error.err_line, error.err_message);
        }

        let mut ucx: ConnectUsageContext = connect_usages_look_if_full_reset_needed(ast_index.clone()).await;
        loop {
            let did_anything = connect_usages(ast_index.clone(), &mut ucx).await;
            if !did_anything {
                break;
            }
        }

        flush_db_changes(ast_index.clone(), 0).await;
        dump_database(ast_index.clone()).await;

        let hierarchy = type_hierarchy(ast_index.clone(), language.to_string(), "".to_string()).await;
        println!("Type hierarchy:\n{}", hierarchy);
        let expected_hierarchy = "Animal\n  Goat\n    CosmicGoat\nCosmicJustice\n  CosmicGoat\n";
        assert_eq!(
            hierarchy, expected_hierarchy,
            "Type hierarchy does not match expected output"
        );
        println!(
            "Type hierachy subtree_of=Animal:\n{}",
            type_hierarchy(ast_index.clone(), language.to_string(), format!("{}🔎Animal", language)).await
        );

        // Goat::Goat() is a C++ constructor
        let goat_def = definitions(ast_index.clone(), goat_location).await;
        let mut goat_def_str = String::new();
        for def in goat_def.iter() {
            goat_def_str.push_str(&format!("{:?}\n", def));
        }
        println!("goat_def_str:\n{}", goat_def_str);
        assert!(goat_def.len() == 1);

        let animalage_defs = definitions(ast_index.clone(), animal_age_location).await;
        let animalage_def0 = animalage_defs.first().unwrap();
        let animalage_usage = usages(ast_index.clone(), animalage_def0.path(), 100).await;
        let mut animalage_usage_str = String::new();
        for (used_at_def, used_at_uline) in animalage_usage.iter() {
            animalage_usage_str.push_str(&format!("{:}:{}\n", used_at_def.cpath, used_at_uline));
        }
        println!("animalage_usage_str:\n{}", animalage_usage_str);
        assert!(animalage_usage.len() == 5);

        let goat_defs = definitions(ast_index.clone(), format!("{}_goat_library::Goat", language).as_str()).await;
        let goat_def0 = goat_defs.first().unwrap();
        let goat_usage = usages(ast_index.clone(), goat_def0.path(), 100).await;
        let mut goat_usage_str = String::new();
        for (used_at_def, used_at_uline) in goat_usage.iter() {
            goat_usage_str.push_str(&format!("{:}:{}\n", used_at_def.cpath, used_at_uline));
        }
        println!("goat_usage:\n{}", goat_usage_str);
        assert!(goat_usage.len() == 1 || goat_usage.len() == 2);  // derived from generates usages (new style: py) or not (old style)

        doc_remove(ast_index.clone(), &library_file_path.to_string()).await;
        doc_remove(ast_index.clone(), &main_file_path.to_string()).await;
        flush_db_changes(ast_index.clone(), 0).await;

        let dblen = dump_database(ast_index.clone()).await;
        let counters = fetch_counters(ast_index.clone()).await;
        assert_eq!(counters.counter_defs, 0);
        assert_eq!(counters.counter_usages, 0);
        assert_eq!(counters.counter_docs, 0);
        assert_eq!(dblen, 3 + 1); // 3 counters and 1 class hierarchy

        let env = ast_index.env.clone();
        drop(ast_index);
        // No need to flush with LMDB as transactions are automatically committed
        println!("dropping env");
        drop(env);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_ast_db_cpp() {
        init_tracing();
        let ast_index = ast_index_init("".to_string(), 10, false).await;
        run_ast_db_test(
            ast_index,
            "src/ast/alt_testsuite/cpp_goat_library.h",
            "src/ast/alt_testsuite/cpp_goat_main.cpp",
            "Goat::Goat",
            "cpp",
            "Animal::age",
        ).await;
    }

    #[tokio::test]
    async fn test_ast_db_py() {
        init_tracing();
        let ast_index = ast_index_init("".to_string(), 10, false).await;
        run_ast_db_test(
            ast_index,
            "src/ast/alt_testsuite/py_goat_library.py",
            "src/ast/alt_testsuite/py_goat_main.py",
            "Goat::__init__",
            "py",
            "Animal::age",
        ).await;
    }
}
