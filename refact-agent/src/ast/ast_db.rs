use std::time::Instant;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use tokio::task;
use serde_cbor;
use sled::Db;
use lazy_static::lazy_static;
use regex::Regex;

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

const CACHE_CAPACITY_BYTES: u64 = 256 * 1024 * 1024;  // 256M cache

pub async fn ast_index_init(ast_permanent: String, ast_max_files: usize, want_perf_report: bool) -> Arc<AMutex<AstDB>>
{
    let mut config = sled::Config::default()
        .cache_capacity(CACHE_CAPACITY_BYTES)
        .use_compression(false)
        .print_profile_on_drop(want_perf_report)
        .mode(sled::Mode::HighThroughput)
        .flush_every_ms(Some(5000));

    if ast_permanent.is_empty() {
        config = config.temporary(true).create_new(true);
    } else {
        config = config.path(ast_permanent.clone());
    }

    tracing::info!("starting AST db, ast_permanent={:?}", ast_permanent);
    let db: Arc<Db> = Arc::new(task::spawn_blocking(
        move || config.open().unwrap()
    ).await.unwrap());
    db.clear().unwrap();
    tracing::info!("/starting AST");
    let ast_index = AstDB {
        sleddb: db,
        sledbatch: Arc::new(AMutex::new(sled::Batch::default())),
        batch_counter: 0,
        counters_increase: HashMap::new(),
        ast_max_files,
    };
    Arc::new(AMutex::new(ast_index))
}

pub async fn fetch_counters(ast_index: Arc<AMutex<AstDB>>) -> AstCounters
{
    let db = ast_index.lock().await.sleddb.clone();
    let counter_defs = db.get(b"counters|defs").unwrap().map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0);
    let counter_usages = db.get(b"counters|usages").unwrap().map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0);
    let counter_docs = db.get(b"counters|docs").unwrap().map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0);
    AstCounters {
        counter_defs,
        counter_usages,
        counter_docs,
    }
}

fn _increase_counter_commit(db: &sled::Db, counter_key: &[u8], adjustment: i32) {
    if adjustment == 0 {
        return;
    }
    match db.update_and_fetch(counter_key, |counter| {
        let counter = counter.map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0) + adjustment;
        Some(serde_cbor::to_vec(&counter).unwrap())
    }) {
        Ok(_) => {},
        Err(e) => tracing::error!("failed to update and fetch counter: {:?}", e),
    }
}

async fn _increase_counter(ast_index: Arc<AMutex<AstDB>>, counter_key: &str, adjustment: i32) {
    if adjustment == 0 {
        return;
    }
    let mut ast_index_locked = ast_index.lock().await;
    let counter = ast_index_locked.counters_increase.entry(counter_key.to_string()).or_insert(0);
    *counter += adjustment;
}

pub async fn flush_sled_batch(
    ast_db: Arc<AMutex<AstDB>>,
    threshold: usize,   // if zero, flush everything including counters
) -> Arc<AMutex<sled::Batch>> {
    let mut ast_index_locked = ast_db.lock().await;
    if ast_index_locked.batch_counter >= threshold {
        let sleddb = ast_index_locked.sleddb.clone();
        let batch_arc = std::mem::replace(&mut ast_index_locked.sledbatch, Arc::new(AMutex::new(sled::Batch::default())));
        let was_counter = std::mem::replace(&mut ast_index_locked.batch_counter, 0);
        let counters_increase = std::mem::replace(&mut ast_index_locked.counters_increase, HashMap::new());
        drop(ast_index_locked);
        if was_counter > 0 {
            // tracing::info!("flushing {} sled batches", was_counter);
            let mut batch = batch_arc.lock().await;
            let batch_to_apply = std::mem::replace(&mut *batch, sled::Batch::default());
            if let Err(e) = sleddb.apply_batch(batch_to_apply) {
                tracing::error!("failed to apply batch: {:?}", e);
            }
        }
        for (counter_key, adjustment) in counters_increase {
            _increase_counter_commit(&sleddb, counter_key.as_bytes(), adjustment);
        }
        let ast_index_locked2 = ast_db.lock().await;
        return ast_index_locked2.sledbatch.clone();
    }
    ast_index_locked.batch_counter += 1;
    ast_index_locked.sledbatch.clone()
}

pub async fn doc_add(
    ast_index: Arc<AMutex<AstDB>>,
    cpath: &String,
    text: &String,
    errors: &mut AstErrorStats,
) -> Result<(Vec<Arc<AstDefinition>>, String), String>
{
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let (defs, language) = parse_anything_and_add_file_path(&cpath, text, errors)?;   // errors mostly "no such parser" here
    let db = ast_index.lock().await.sleddb.clone();
    let batch_arc = flush_sled_batch(ast_index.clone(), 1000).await;
    let mut batch = batch_arc.lock().await;
    let mut added_defs: i32 = 0;
    let mut added_usages: i32 = 0;
    let mut unresolved_usages: i32 = 0;
    for definition in defs.iter() {
        assert!(definition.cpath == *cpath);
        let serialized = serde_cbor::to_vec(&definition).unwrap();
        let official_path = definition.official_path.join("::");
        let d_key = format!("d|{}", official_path);
        debug_print!("writing {}", d_key);
        batch.insert(d_key.as_bytes(), serialized);
        let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
        while !path_parts.is_empty() {
            let c_key = format!("c|{} ⚡ {}", path_parts.join("::"), official_path);
            batch.insert(c_key.as_bytes(), b"");
            path_parts.remove(0);
        }
        for usage in &definition.usages {
            if !usage.resolved_as.is_empty() {
                let u_key = format!("u|{} ⚡ {}", usage.resolved_as, official_path);
                batch.insert(u_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
            } else if usage.targets_for_guesswork.len() == 1 && !usage.targets_for_guesswork[0].starts_with("?::") {
                let homeless_key = format!("homeless|{} ⚡ {}", usage.targets_for_guesswork[0], official_path);
                batch.insert(homeless_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
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
            batch.insert(t_key.as_bytes(), definition.this_is_a_class.as_bytes());
        }
        added_defs += 1;
    }
    if unresolved_usages > 0 {
        let resolve_todo_key = format!("resolve-todo|{}", file_global_path.join("::"));
        batch.insert(resolve_todo_key.as_bytes(), cpath.as_bytes());
    }
    let doc_key = format!("doc-cpath|{}", file_global_path.join("::"));
    if db.get(doc_key.as_bytes()).unwrap().is_none() {
        _increase_counter(ast_index.clone(), "counters|docs", 1).await;
        db.insert(doc_key.as_bytes(), cpath.as_bytes()).unwrap();
    }
    _increase_counter(ast_index.clone(), "counters|defs", added_defs).await;
    _increase_counter(ast_index.clone(), "counters|usages", added_usages).await;

    Ok((defs.into_iter().map(Arc::new).collect(), language))
}

pub async fn doc_remove(ast_index: Arc<AMutex<AstDB>>, cpath: &String)
{
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d|{}::", file_global_path.join("::"));
    let db = ast_index.lock().await.sleddb.clone();
    let batch_arc = flush_sled_batch(ast_index.clone(), 1000).await;
    let mut batch = batch_arc.lock().await;
    let mut iter = db.scan_prefix(d_prefix);
    let mut deleted_defs: i32 = 0;
    let mut deleted_usages: i32 = 0;
    while let Some(Ok((key, value))) = iter.next() {
        let d_key = key.clone();
        if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(&value) {
            let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
            let official_path = definition.official_path.join("::");
            while !path_parts.is_empty() {
                let c_key = format!("c|{} ⚡ {}", path_parts.join("::"), official_path);
                batch.remove(c_key.as_bytes());
                path_parts.remove(0);
            }
            for usage in &definition.usages {
                if !usage.resolved_as.is_empty() {
                    let u_key = format!("u|{} ⚡ {}", usage.resolved_as, official_path);
                    batch.remove(u_key.as_bytes());
                } else if usage.targets_for_guesswork.len() == 1 && !usage.targets_for_guesswork[0].starts_with("?::") {
                    let homeless_key = format!("homeless|{} ⚡ {}", usage.targets_for_guesswork[0], official_path);
                    batch.remove(homeless_key.as_bytes());
                    debug_print!("        homeless {}", homeless_key);
                    continue;
                }
                deleted_usages += 1;
            }
            for from in &definition.this_class_derived_from {
                let t_key = format!("classes|{} ⚡ {}", from, official_path);
                batch.remove(t_key.as_bytes());
            }
            let cleanup_key = format!("resolve-cleanup|{}", definition.official_path.join("::"));
            if let Ok(Some(cleanup_value)) = db.get(cleanup_key.as_bytes()) {
                if let Ok(all_saved_ulinks) = serde_cbor::from_slice::<Vec<String>>(&cleanup_value) {
                    for ulink in all_saved_ulinks {
                        batch.remove(ulink.as_bytes());
                    }
                } else {
                    tracing::error!("failed to deserialize cleanup_value for key: {}", cleanup_key);
                }
                batch.remove(cleanup_key.as_bytes());
            }
            deleted_defs += 1;
        }
        debug_print!("removing {}", String::from_utf8_lossy(&d_key));
        batch.remove(&d_key);
    }
    let doc_resolved_key = format!("doc-resolved|{}", file_global_path.join("::"));
    batch.remove(doc_resolved_key.as_bytes());
    let doc_key = format!("doc-cpath|{}", file_global_path.join("::"));
    if db.get(doc_key.as_bytes()).unwrap().is_some() {
        _increase_counter(ast_index.clone(), "counters|docs", -1).await;
        db.remove(doc_key.as_bytes()).unwrap();
    }
    _increase_counter(ast_index.clone(), "counters|defs", -deleted_defs).await;
    _increase_counter(ast_index.clone(), "counters|usages", -deleted_usages).await;
}

pub async fn doc_defs(ast_index: Arc<AMutex<AstDB>>, cpath: &String) -> Vec<Arc<AstDefinition>>
{
    let to_search_prefix = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d|{}::", to_search_prefix.join("::"));
    let db = ast_index.lock().await.sleddb.clone();
    let mut defs = Vec::new();
    let mut iter = db.scan_prefix(d_prefix);
    while let Some(Ok((_, value))) = iter.next() {
        if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(&value) {
            defs.push(Arc::new(definition));
        }
    }
    defs
}

pub async fn doc_usages(ast_index: Arc<AMutex<AstDB>>, cpath: &String) -> Vec<(usize, String)>
{
    let definitions = doc_defs(ast_index.clone(), cpath).await;
    let db = ast_index.lock().await.sleddb.clone();
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
    if let Ok(Some(resolved_usages)) = db.get(doc_resolved_key.as_bytes()) {
        let resolved_usages_vec = serde_cbor::from_slice::<Vec<(usize, String)>>(&resolved_usages).unwrap();
        usages.extend(resolved_usages_vec);
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

pub async fn connect_usages(ast_index: Arc<AMutex<AstDB>>, ucx: &mut ConnectUsageContext) -> bool
{
    let db = ast_index.lock().await.sleddb.clone();
    let mut iter = db.scan_prefix("resolve-todo|").take(1);

    if let Some(Ok((todo_key, todo_value))) = iter.next() {
        let todo_key_string = String::from_utf8(todo_key.to_vec()).unwrap();
        let global_file_path = todo_key_string.strip_prefix("resolve-todo|").unwrap();
        let cpath = String::from_utf8(todo_value.to_vec()).unwrap();
        debug_print!("resolving {}", cpath);

        // delete immediately, to make sure connect_usages() does not continue forever, even if there are errors and stuff
        if let Err(e) = db.remove(&todo_key) {
            tracing::error!("connect_usages() failed to remove resolve-todo key: {:?}", e);
        }

        let definitions = doc_defs(ast_index.clone(), &cpath.to_string()).await;
        let batch_arc = flush_sled_batch(ast_index.clone(), 1000).await;
        let mut batch = batch_arc.lock().await;

        let mut resolved_usages: Vec<(usize, String)> = vec![];
        for def in definitions {
            let tmp = _connect_usages_helper(&db, ucx, &def, &mut batch).await;
            resolved_usages.extend(tmp);
        }
        batch.insert(
            format!("doc-resolved|{}", global_file_path).as_bytes(),
            serde_cbor::to_vec(&resolved_usages).unwrap().as_slice(),
        );
        return true;
    }

    false
}

pub async fn connect_usages_look_if_full_reset_needed(ast_index: Arc<AMutex<AstDB>>) -> ConnectUsageContext
{
    flush_sled_batch(ast_index.clone(), 0).await;
    let db = ast_index.lock().await.sleddb.clone();
    let class_hierarchy_key = b"class-hierarchy|";
    let existing_hierarchy: IndexMap<String, Vec<String>> = match db.get(class_hierarchy_key) {
        Ok(Some(value)) => serde_cbor::from_slice(&value).unwrap_or_default(),
        _ => IndexMap::new(),
    };

    let new_derived_from_map = _derived_from(&db).await;
    let mut batch = sled::Batch::default();

    if existing_hierarchy.is_empty() {
        let serialized_hierarchy = serde_cbor::to_vec(&new_derived_from_map).unwrap();
        batch.insert(class_hierarchy_key, serialized_hierarchy.as_slice());
        // first run, do nothing because all the definitions are already in the todo list

    } else if new_derived_from_map != existing_hierarchy {
        // XXX first branch is not covered by tests (simple enough to work and not break?)
        tracing::info!(" * * * class hierarchy changed {} classes => {} classes, all usages need to be reconnected * * *", existing_hierarchy.len(), new_derived_from_map.len());
        let serialized_hierarchy = serde_cbor::to_vec(&new_derived_from_map).unwrap();
        batch.insert(class_hierarchy_key, serialized_hierarchy.as_slice());

        let mut iter = db.scan_prefix("doc-cpath|");
        let mut cnt = 0;
        while let Some(Ok((key, value))) = iter.next() {
            let key_string = String::from_utf8(key.to_vec()).unwrap();
            if let Some(file_global_path) = key_string.strip_prefix("doc-cpath|") {
                let cpath = String::from_utf8(value.to_vec()).unwrap();
                let resolve_todo_key = format!("resolve-todo|{}", file_global_path);
                batch.insert(resolve_todo_key.as_bytes(), cpath.as_bytes());
                cnt += 1;
            }
        }
        tracing::info!("added {} items to resolve-todo", cnt);
    }

    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("connect_usages_look_if_full_reset_needed() failed to apply batch: {:?}", e);
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
}

lazy_static! {
    static ref MAGNIFYING_GLASS_RE: Regex = Regex::new(r"(\w+)🔎(\w+)").unwrap();
}

async fn _connect_usages_helper(
    db: &sled::Db,
    ucx: &mut ConnectUsageContext,
    definition: &AstDefinition,
    batch: &mut sled::Batch
) -> Vec<(usize, String)> {
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
    let official_path = definition.official_path.join("::");
    let mut result = Vec::<(usize, String)>::new();
    let mut all_saved_ulinks = Vec::<String>::new();
    for (uindex, usage) in definition.usages.iter().enumerate() {
        debug_print!("    resolving {}.usage[{}] == {:?}", official_path, uindex, usage);
        if !usage.resolved_as.is_empty() {
            ucx.usages_connected += 1;
            continue;
        }
        for to_resolve_unstripped in &usage.targets_for_guesswork {
            if !to_resolve_unstripped.starts_with("?::") {
                debug_print!("    homeless {}", to_resolve_unstripped);
                ucx.usages_homeless += 1;
                continue;
            }
            let to_resolve = to_resolve_unstripped.strip_prefix("?::").unwrap();
            // println!("to_resolve_unstripped {:?}", to_resolve_unstripped);
            debug_print!("    to resolve {}.usage[{}] guessing {}", official_path, uindex, to_resolve);

            // Extract all LANGUAGE🔎CLASS from to_resolve
            let mut magnifying_glass_pairs = Vec::new();
            let mut template = to_resolve.to_string();
            for (i, cap) in MAGNIFYING_GLASS_RE.captures_iter(to_resolve).enumerate() {
                let language = cap.get(1).unwrap().as_str().to_string();
                let klass = cap.get(2).unwrap().as_str().to_string();
                let placeholder = format!("%%PAIR{}%%", i);
                template = template.replacen(&format!("{}🔎{}", language, klass), &placeholder, 1);
                magnifying_glass_pairs.push((language, klass));
            }
            let mut variants = Vec::<String>::new();
            if magnifying_glass_pairs.len() == 0 {
                variants.push(to_resolve.to_string());
            } else {
                let substitutions_of_each_pair: Vec<Vec<String>> = magnifying_glass_pairs.iter().map(|(language, klass)| {
                    let mut substitutions = ucx.derived_from_map.get(format!("{}🔎{}", language, klass).as_str()).cloned().unwrap_or_else(|| vec![]);
                    substitutions.insert(0, klass.clone());
                    substitutions.iter().map(|s| s.strip_prefix(&format!("{}🔎", language)).unwrap_or(s).to_string()).collect()
                }).collect();

                fn generate_combinations(substitutions: &[Vec<String>], index: usize, current: Vec<String>) -> Vec<Vec<String>> {
                    if index == substitutions.len() {
                        return vec![current];
                    }
                    let mut result = Vec::new();
                    for substitution in &substitutions[index] {
                        let mut new_current = current.clone();
                        new_current.push(substitution.clone());
                        result.extend(generate_combinations(substitutions, index + 1, new_current));
                    }
                    result
                }
                let intermediate_results = generate_combinations(&substitutions_of_each_pair, 0, Vec::new());
                // Transform each something::LANGUAGE🔎CLASS::something into something::class::something
                for intermediate_result in intermediate_results {
                    let mut variant = template.clone();
                    for (i, substitution) in intermediate_result.iter().enumerate() {
                        let placeholder = format!("%%PAIR{}%%", i);
                        variant = variant.replacen(&placeholder, substitution, 1);
                    }
                    variants.push(variant);
                }
                // ?::cpp🔎Goat::self_review magnifying_glass_pairs [("cpp", "Goat")]
                //   substitutions_of_each_pair [["Goat", "Animal"]]
                //   intermediate_results [["Goat"], ["Animal"]]
                //   variants possible ["Goat::self_review", "Animal::self_review"]
            }

            let mut found = Vec::new();
            for v in variants {
                let c_prefix = format!("c|{}", v);
                debug_print!("        scanning {}", c_prefix);
                // println!("    c_prefix {:?} because v={:?}", c_prefix, v);
                let mut c_iter = db.scan_prefix(&c_prefix);
                while let Some(Ok((c_key, _))) = c_iter.next() {
                    let c_key_string = String::from_utf8(c_key.to_vec()).unwrap();
                    let parts: Vec<&str> = c_key_string.split(" ⚡ ").collect();
                    if parts.len() == 2 {
                        if parts[0] == c_prefix {
                            let resolved_target = parts[1].trim();
                            found.push(resolved_target.to_string());
                        }
                    }
                }
                if found.len() > 0 {
                    break;
                }
            }
            debug_print!("        found {:?}", found);

            if found.len() == 0 {
                ucx.usages_not_found += 1;
                continue;
            }
            if found.len() > 1 {
                ucx.errstats.add_error(definition.cpath.clone(), usage.uline, &format!("usage `{}` is ambiguous, can mean: {:?}", to_resolve, found));
                ucx.usages_ambiguous += 1;
                found.truncate(1);
            }
            let single_thing_found = found.into_iter().next().unwrap();
            let u_key = format!("u|{} ⚡ {}", single_thing_found, official_path);
            batch.insert(u_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
            debug_print!("        add {:?} <= {}", u_key, usage.uline);
            all_saved_ulinks.push(u_key);
            result.push((usage.uline, single_thing_found));
            ucx.usages_connected += 1;
            break;  // the next thing from targets_for_guesswork is a worse query, keep this one and exit
        }
    } // for usages
    let cleanup_key = format!("resolve-cleanup|{}", definition.official_path.join("::"));
    let cleanup_value = serde_cbor::to_vec(&all_saved_ulinks).unwrap();
    batch.insert(cleanup_key.as_bytes(), cleanup_value.as_slice());
    result
}

async fn _derived_from(db: &sled::Db) -> IndexMap<String, Vec<String>>
{
    // Data example:
    // classes/cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
    let mut derived_map: IndexMap<String, Vec<String>> = IndexMap::new();
    let t_prefix = "classes|";
    let mut iter = db.scan_prefix(t_prefix);
    while let Some(Ok((key, value))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        let value_string = String::from_utf8(value.to_vec()).unwrap();
        let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
        if parts.len() == 2 {
            let parent = parts[0].trim().strip_prefix(t_prefix).unwrap_or(parts[0].trim()).to_string();
            let child = value_string.trim().to_string();
            let entry = derived_map.entry(child).or_insert_with(Vec::new);
            if !entry.contains(&parent) {
                entry.push(parent);
            }
        } else {
            tracing::warn!("bad key {}", key_string);
        }
    }
    // Have perfectly good [child, [parent1, parent2, ..]]
    // derived_map {"cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat"]}
    // Now we need to post-process this into [child, [parent1, parent_of_parent1, parent2, parent_of_parent2, ...]]
    fn build_all_derived_from(
        klass: &str,
        derived_map: &IndexMap<String, Vec<String>>,
        all_derived_from: &mut IndexMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
    ) -> Vec<String> {
        if visited.contains(klass) {
            return all_derived_from.get(klass).cloned().unwrap_or_default();
        }
        visited.insert(klass.to_string());
        let mut all_parents = Vec::new();
        if let Some(parents) = derived_map.get(klass) {
            for parent in parents {
                all_parents.push(parent.clone());
                let ancestors = build_all_derived_from(parent, derived_map, all_derived_from, visited);
                for ancestor in ancestors {
                    if !all_parents.contains(&ancestor) {
                        all_parents.push(ancestor);
                    }
                }
            }
        }
        all_derived_from.insert(klass.to_string(), all_parents.clone());
        all_parents
    }
    let mut all_derived_from: IndexMap<String, Vec<String>> = IndexMap::new();
    for klass in derived_map.keys() {
        let mut visited: HashSet<String> = HashSet::new();
        build_all_derived_from(klass, &derived_map, &mut all_derived_from, &mut visited);
    }
    // now have all_derived_from {"cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat", "cpp🔎Animal"], "cpp🔎CosmicJustice": [], "cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎Animal": []}
    all_derived_from
}

pub async fn usages(ast_index: Arc<AMutex<AstDB>>, full_official_path: String, limit_n: usize) -> Vec<(Arc<AstDefinition>, usize)>
{
    // The best way to get full_official_path is to call definitions() first
    let db = ast_index.lock().await.sleddb.clone();
    let mut usages = Vec::new();
    let u_prefix1 = format!("u|{} ", full_official_path); // this one has space
    let u_prefix2 = format!("u|{}", full_official_path);
    let mut iter = db.scan_prefix(&u_prefix1);
    while let Some(Ok((u_key, u_value))) = iter.next() {
        if usages.len() >= limit_n {
            break;
        }
        let key_string = String::from_utf8(u_key.to_vec()).unwrap();
        let uline: usize = serde_cbor::from_slice(&u_value).unwrap_or(0); // Assuming `uline` is stored in the value
        let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
        if parts.len() == 2 && parts[0] == u_prefix2 {
            let full_path = parts[1].trim();
            let d_key = format!("d|{}", full_path);
            if let Ok(Some(d_value)) = db.get(d_key.as_bytes()) {
                match serde_cbor::from_slice::<AstDefinition>(&d_value) {
                    Ok(definition) => {
                        usages.push((Arc::new(definition), uline));
                    },
                    Err(e) => println!("Failed to deserialize value for {}: {:?}", d_key, e),
                }
            }
        } else if parts.len() != 2  {
            tracing::error!("usage record has more than two ⚡ key was: {}", key_string);
        }
    }
    usages
}

pub async fn definitions(ast_index: Arc<AMutex<AstDB>>, double_colon_path: &str) -> Vec<Arc<AstDefinition>>
{
    let db = ast_index.lock().await.sleddb.clone();
    let c_prefix1 = format!("c|{} ", double_colon_path); // has space
    let c_prefix2 = format!("c|{}", double_colon_path);
    let mut path_groups: HashMap<usize, Vec<String>> = HashMap::new();
    // println!("definitions(c_prefix={:?})", c_prefix);
    let mut iter = db.scan_prefix(&c_prefix1);
    while let Some(Ok((key, _))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
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
    let min_colon_count = path_groups.keys().min().cloned().unwrap_or(usize::MAX);
    let mut defs = Vec::new();
    if let Some(paths) = path_groups.get(&min_colon_count) {
        for full_path in paths {
            let d_key = format!("d|{}", full_path);
            if let Ok(Some(d_value)) = db.get(d_key.as_bytes()) {
                match serde_cbor::from_slice::<AstDefinition>(&d_value) {
                    Ok(definition) => defs.push(Arc::new(definition)),
                    Err(e) => println!("Failed to deserialize value for {}: {:?}", d_key, e),
                }
            }
        }
    }
    defs
}

#[allow(dead_code)]
pub async fn type_hierarchy(ast_index: Arc<AMutex<AstDB>>, language: String, subtree_of: String) -> String
{
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
    let db = ast_index.lock().await.sleddb.clone();
    let t_prefix = format!("classes|{}", language);
    let mut iter = db.scan_prefix(&t_prefix);
    let mut hierarchy_map: IndexMap<String, Vec<String>> = IndexMap::new();

    while let Some(Ok((key, value))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        let value_string = String::from_utf8(value.to_vec()).unwrap();
        if key_string.contains(" ⚡ ") {
            let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
            if parts.len() == 2 {
                let parent = parts[0].trim().strip_prefix("classes|").unwrap_or(parts[0].trim()).to_string();
                let child = value_string.trim().to_string();
                hierarchy_map.entry(parent).or_insert_with(Vec::new).push(child);
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
}

pub async fn definition_paths_fuzzy(ast_index: Arc<AMutex<AstDB>>, pattern: &str, top_n: usize, max_candidates_to_consider: usize) -> Vec<String> {
    let db = ast_index.lock().await.sleddb.clone();
    let mut candidates = HashSet::new();
    let mut patterns_to_try = Vec::new();

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

    for pat in patterns_to_try {
        let c_prefix = format!("c|{}", pat);
        let mut iter = db.scan_prefix(&c_prefix);
        while let Some(Ok((key, _))) = iter.next() {
            let key_string = String::from_utf8(key.to_vec()).unwrap();
            if let Some((_, dest)) = key_string.split_once(" ⚡ ") {
                candidates.insert(dest.to_string());
            }
            if candidates.len() >= max_candidates_to_consider {
                break;
            }
        }
        if candidates.len() >= max_candidates_to_consider {
            break;
        }
    }

    let results = fuzzy_search(&pattern.to_string(), candidates, top_n, &[':']);

    results.into_iter()
        .map(|result| {
            if let Some(pos) = result.find("::") {
                result[pos + 2..].to_string()
            } else {
                result
            }
        })
        .collect()
}

#[allow(dead_code)]
pub async fn dump_database(ast_index: Arc<AMutex<AstDB>>) -> usize
{
    let db = ast_index.lock().await.sleddb.clone();
    println!("\nsled has {} records", db.len());
    let iter = db.iter();
    for item in iter {
        let (key, value) = item.unwrap();
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        if key_string.starts_with("d|") {
            match serde_cbor::from_slice::<AstDefinition>(&value) {
                Ok(definition) => println!("{} 👉 {:.50}", key_string, format!("{:?}", definition)),
                Err(e) => println!("Failed to deserialize value at {}: {:?}", key_string, e),
            }
        } else if key_string.starts_with("classes|") {
            let value_string = String::from_utf8(value.to_vec()).unwrap();
            println!("{} 👉 {:?}", key_string, value_string);
        } else if key_string.starts_with("counters|") {
            let counter_value: i32 = serde_cbor::from_slice(&value).unwrap();
            println!("{}: {}", key_string, counter_value);
        } else if value.len() > 0 {
            println!("{} ({} bytes)", key_string, value.len());
        } else {
            println!("{}", key_string);
        }
    }
    println!("dump_database over");
    db.len()
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
        ast_index: Arc<AMutex<AstDB>>,
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

        flush_sled_batch(ast_index.clone(), 0).await;
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
        flush_sled_batch(ast_index.clone(), 0).await;

        let dblen = dump_database(ast_index.clone()).await;
        let counters = fetch_counters(ast_index.clone()).await;
        assert_eq!(counters.counter_defs, 0);
        assert_eq!(counters.counter_usages, 0);
        assert_eq!(counters.counter_docs, 0);
        assert_eq!(dblen, 3 + 1); // 3 counters and 1 class hierarchy

        let db = ast_index.lock().await.sleddb.clone();
        drop(ast_index);
        assert!(Arc::strong_count(&db) == 1);
        println!("db.flush");
        let x = db.flush().unwrap();
        println!("db.flush returned {}, drop", x);
        drop(db);
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
