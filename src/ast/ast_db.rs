use std::time::Instant;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use indexmap::IndexMap;
use tokio::sync::Mutex as AMutex;
use tokio::task;
use serde_cbor;
use sled::Db;

use crate::ast::ast_minimalistic::{AstDB, AstDefinition, AstCounters, ErrorStats};
use crate::ast::ast_parse_anything::{parse_anything_and_add_file_path, filesystem_path_to_double_colon_path};


pub async fn ast_index_init() -> Arc<AMutex<AstDB>>
{
    let db: Arc<Db> = Arc::new(task::spawn_blocking(|| sled::open("/tmp/my_db.sled").unwrap()).await.unwrap());
    db.clear().unwrap();
    // db.open_tree(b"unprocessed items").unwrap();
    let ast_index = AstDB {
        sleddb: db,
    };
    Arc::new(AMutex::new(ast_index))
}

// ## How the database works ##
//
// Database `sled` used here is a key-value storage, everything is stored as keys and values. Try dump_database() below.
//
// All the definitions are serialized under d/ like this:
//   d/alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice
//   AstDefinition { alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice, usages: Link{ up alt_testsuite::cpp_goat_main::CosmicJustice::balance } }
//
// You can look up a shorter path than the full path, by using c/ records:
//   c/main::goat1 ⚡ alt_testsuite::cpp_goat_main::main::goat1
//     ^^^^^^^^^^^ short path that maps to full path
//
// Usages are stored as:
//   u/CosmicJustice::balance ⚡ alt_testsuite::cpp_goat_main::CosmicJustice::CosmicJustice
//     ^^^^^^^^^^^^^^^^^^^^^^ usage of what? (short path)
//                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ full path to where the usage is
//
// Read tests below, the show what this index can do!
//

pub async fn fetch_counters(ast_index: Arc<AMutex<AstDB>>) -> AstCounters
{
    let db = ast_index.lock().await.sleddb.clone();
    let counter_defs = db.get(b"counters#defs").unwrap().map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0);
    let counter_usages = db.get(b"counters#usages").unwrap().map(|v| serde_cbor::from_slice::<i32>(&v).unwrap()).unwrap_or(0);
    AstCounters {
        counter_defs,
        counter_usages,
    }
}

fn _increase_counter(db: &sled::Db, counter_key: &[u8], adjustment: i32) {
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

pub async fn doc_add(
    ast_index: Arc<AMutex<AstDB>>,
    cpath: &String,
    text: &String,
    errors: &mut ErrorStats,
) -> Result<(Vec<Arc<AstDefinition>>, String), String>
{
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let (defs, language) = parse_anything_and_add_file_path(&cpath, text, errors)?;
    let db = ast_index.lock().await.sleddb.clone();
    let mut batch = sled::Batch::default();
    let mut added_defs: i32 = 0;
    let mut added_usages: i32 = 0;
    for definition in defs.values() {
        let serialized = serde_cbor::to_vec(&definition).unwrap();
        let official_path = definition.official_path.join("::");
        let d_key = format!("d#{}", official_path);
        batch.insert(d_key.as_bytes(), serialized);
        let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
        while !path_parts.is_empty() {
            let c_key = format!("c#{} ⚡ {}", path_parts.join("::"), official_path);
            batch.insert(c_key.as_bytes(), b"");
            path_parts.remove(0);
        }
        let mut unresolved_usages: i32 = 0;
        for usage in &definition.usages {
            if !usage.resolved_as.is_empty() {
                let u_key = format!("u#{} ⚡ {}", usage.resolved_as, official_path);
                batch.insert(u_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
            } else {
                unresolved_usages += 1;
            }
            added_usages += 1;
        }
        // this_is_a_class: cpp🔎CosmicGoat, derived_from: "cpp🔎Goat" "cpp🔎CosmicJustice"
        for from in &definition.this_class_derived_from {
            let t_key = format!("classes#{} ⚡ {}", from, official_path);
            batch.insert(t_key.as_bytes(), definition.this_is_a_class.as_bytes());
        }
        if unresolved_usages > 0 {
            let resolve_todo_key = format!("resolve-todo#{}", official_path);
            batch.insert(resolve_todo_key.as_bytes(), b"");
        }
        added_defs += 1;
    }

    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("doc_add() failed to apply batch: {:?}", e);
    }

    let doc_key = format!("doc#{}", file_global_path.join("::"));
    if db.get(doc_key.as_bytes()).unwrap().is_none() {
        _increase_counter(&db, b"counters#docs", 1);
        db.insert(doc_key.as_bytes(), cpath.as_bytes()).unwrap();
    }
    _increase_counter(&db, b"counters#defs", added_defs);
    _increase_counter(&db, b"counters#usages", added_usages);

    Ok((defs.into_values().map(Arc::new).collect(), language))
}

pub async fn doc_remove(ast_index: Arc<AMutex<AstDB>>, cpath: &String)
{
    let file_global_path = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d#{}::", file_global_path.join("::"));
    let db = ast_index.lock().await.sleddb.clone();
    let mut batch = sled::Batch::default();
    let mut iter = db.scan_prefix(d_prefix);
    let mut deleted_defs: i32 = 0;
    let mut deleted_usages: i32 = 0;
    while let Some(Ok((key, value))) = iter.next() {
        let d_key_b = key.clone();
        if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(&value) {
            let mut path_parts: Vec<&str> = definition.official_path.iter().map(|s| s.as_str()).collect();
            let official_path = definition.official_path.join("::");
            while !path_parts.is_empty() {
                let c_key = format!("c#{} ⚡ {}", path_parts.join("::"), official_path);
                batch.remove(c_key.as_bytes());
                path_parts.remove(0);
            }
            for usage in &definition.usages {
                if !usage.resolved_as.is_empty() {
                    let u_key = format!("u#{} ⚡ {}", usage.resolved_as, official_path);
                    batch.remove(u_key.as_bytes());
                }
                deleted_usages += 1;
            }
            for from in &definition.this_class_derived_from {
                let t_key = format!("classes#{} ⚡ {}", from, official_path);
                batch.remove(t_key.as_bytes());
            }
            let cleanup_key = format!("resolve-cleanup#{}", definition.official_path.join("::"));
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
        batch.remove(&d_key_b);
    }
    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("doc_remove() failed to apply batch: {:?}", e);
    }
    let doc_key = format!("doc#{}", file_global_path.join("::"));
    if db.get(doc_key.as_bytes()).unwrap().is_some() {
        _increase_counter(&db, b"counters#docs", -1);
        db.remove(doc_key.as_bytes()).unwrap();
    }
    _increase_counter(&db, b"counters#defs", -deleted_defs);
    _increase_counter(&db, b"counters#usages", -deleted_usages);
}

pub async fn doc_symbols(ast_index: Arc<AMutex<AstDB>>, cpath: &String) -> Vec<Arc<AstDefinition>>
{
    let to_search_prefix = filesystem_path_to_double_colon_path(cpath);
    let d_prefix = format!("d#{}::", to_search_prefix.join("::"));
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

pub struct ConnectUsageContext {
    pub derived_from_map: IndexMap<String, Vec<String>>,
    pub errstats: ErrorStats,
    pub usages_connected: usize,
    pub usages_not_found: usize,
    pub usages_ambiguous: usize,
    pub t0: Instant,
}

pub async fn connect_usages(ast_index: Arc<AMutex<AstDB>>, ucx: &mut ConnectUsageContext) -> bool
{
    let db = ast_index.lock().await.sleddb.clone();
    let mut iter = db.scan_prefix("resolve-todo#").take(1);

    if let Some(Ok((todo_key, _))) = iter.next() {
        let key_string = String::from_utf8(todo_key.to_vec()).unwrap();
        let def_official_path = key_string.strip_prefix("resolve-todo#").unwrap();

        // delete immediately, to make sure connect_usages() does not continue forever, even if there are errors and stuff
        if let Err(e) = db.remove(&todo_key) {
            tracing::error!("connect_usages() failed to remove resolve-todo key: {:?}", e);
        }

        let d_key = format!("d#{}", def_official_path);
        if let Ok(Some(value)) = db.get(&d_key) {
            let mut batch = sled::Batch::default();

            if let Ok(definition) = serde_cbor::from_slice::<AstDefinition>(&value) {
                _connect_usages_helper(&db, ucx, &definition, &mut batch).await;
            }

            if let Err(e) = db.apply_batch(batch) {
                tracing::error!("connect_usages() failed to apply batch: {:?}", e);
            }
        } else {
            tracing::error!("connect_usages() failed to get d#{}", d_key);
        }

        return true;
    }

    false
}

pub async fn connect_usages_look_if_full_reset_needed(ast_index: Arc<AMutex<AstDB>>) -> ConnectUsageContext
{
    let db = ast_index.lock().await.sleddb.clone();
    let class_hierarchy_key = b"class-hierarchy#";
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
        tracing::info!(" * * * class hierarchy changed, all usages need to be reconnected * * *");
        let serialized_hierarchy = serde_cbor::to_vec(&new_derived_from_map).unwrap();
        batch.insert(class_hierarchy_key, serialized_hierarchy.as_slice());

        let mut iter = db.scan_prefix("d#");
        let mut cnt = 0;
        while let Some(Ok((key, _))) = iter.next() {
            let key_string = String::from_utf8(key.to_vec()).unwrap();
            let resolve_todo_key = format!("resolve-todo#{}", key_string.strip_prefix("d#").unwrap());
            batch.insert(resolve_todo_key.as_bytes(), b"");
            cnt += 1;
        }
        tracing::info!("added {} items to resolve-todo.", cnt);
    }

    if let Err(e) = db.apply_batch(batch) {
        tracing::error!("connect_usages_look_if_full_reset_needed() failed to apply batch: {:?}", e);
    }

    ConnectUsageContext {
        derived_from_map: new_derived_from_map,
        errstats: ErrorStats::default(),
        usages_connected: 0,
        usages_not_found: 0,
        usages_ambiguous: 0,
        t0: Instant::now(),
    }
}

async fn _connect_usages_helper(
    db: &sled::Db,
    ucx: &mut ConnectUsageContext,
    definition: &AstDefinition,
    batch: &mut sled::Batch
) {
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
    // means `age` was used in self_review(). Only key is set, value doesn't matter.
    //
    // Saved data by this function:
    //   u/RESOLVED ⚡ official_path        -- value doesn't matter
    //   resolve-cleanup/official_path     -- value contains all the "u#RESOLVED ⚡ official_path" in a list
    //
    let official_path = definition.official_path.join("::");
    let magnifying_glass_re = regex::Regex::new(r"(\w+)🔎(\w+)").unwrap();
    let mut all_saved_ulinks = Vec::<String>::new();
    for usage in &definition.usages {
        if !usage.resolved_as.is_empty() {
            ucx.usages_connected += 1;
            continue;
        }
        for to_resolve_unstripped in &usage.targets_for_guesswork {
            assert!(to_resolve_unstripped.starts_with("?::"), "Target does not start with '?::': {}", to_resolve_unstripped);
            let to_resolve = to_resolve_unstripped.strip_prefix("?::").unwrap();
            // println!("to_resolve_unstripped {:?}", to_resolve_unstripped);

            // Extract all LANGUAGE🔎CLASS from to_resolve
            let mut magnifying_glass_pairs = Vec::new();
            let mut template = to_resolve.to_string();
            for (i, cap) in magnifying_glass_re.captures_iter(to_resolve).enumerate() {
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
                let c_prefix = format!("c#{}", v);
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
            // println!("   found {:?}", found);
            if found.len() == 0 {
                ucx.usages_not_found += 1;
                continue;
            }
            if found.len() > 1 {
                ucx.errstats.add_error(definition.cpath.clone(), usage.uline, &format!("link {} is ambiguous, can mean: {:?}", to_resolve, found));
                ucx.usages_ambiguous += 1;
                found.truncate(1);
            }
            let single_thing_found = found.into_iter().next().unwrap();
            let u_key = format!("u#{} ⚡ {}", single_thing_found, official_path);
            batch.insert(u_key.as_bytes(), serde_cbor::to_vec(&usage.uline).unwrap());
            all_saved_ulinks.push(u_key);
            tracing::info!("resolved {} to {} line {}", single_thing_found, official_path, usage.uline);
            ucx.usages_connected += 1;
            break;  // the next thing from targets_for_guesswork is a worse query, keep this one and exit
        }
    } // for usages
    let cleanup_key = format!("resolve-cleanup#{}", definition.official_path.join("::"));
    let cleanup_value = serde_cbor::to_vec(&all_saved_ulinks).unwrap();
    batch.insert(cleanup_key.as_bytes(), cleanup_value.as_slice());
}

async fn _derived_from(db: &sled::Db) -> IndexMap<String, Vec<String>> {
    // Data example:
    // classes/cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
    let mut derived_map: IndexMap<String, Vec<String>> = IndexMap::new();
    let t_prefix = "classes#";
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
    let u_prefix1 = format!("u#{} ", full_official_path); // this one has space
    let u_prefix2 = format!("u#{}", full_official_path);
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
            let d_key = format!("d#{}", full_path);
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
    let c_prefix1 = format!("c#{} ", double_colon_path); // has space
    let c_prefix2 = format!("c#{}", double_colon_path);
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
            let d_key = format!("d#{}", full_path);
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
    let t_prefix = format!("classes#{}", language);
    let mut iter = db.scan_prefix(&t_prefix);
    let mut hierarchy_map: IndexMap<String, Vec<String>> = IndexMap::new();

    while let Some(Ok((key, value))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        let value_string = String::from_utf8(value.to_vec()).unwrap();
        if key_string.contains(" ⚡ ") {
            let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
            if parts.len() == 2 {
                let parent = parts[0].trim().strip_prefix("classes#").unwrap_or(parts[0].trim()).to_string();
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

pub async fn definition_paths_fuzzy(ast_index: Arc<AMutex<AstDB>>, pattern: &str) -> Vec<String>
{
    let db = ast_index.lock().await.sleddb.clone();
    let c_prefix = format!("c#{}", pattern);
    let mut iter = db.scan_prefix(c_prefix);
    let mut found: IndexMap<String, Vec<String>> = IndexMap::new();

    while let Some(Ok((key, _))) = iter.next() {
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        if let Some((cmatch, dest)) = key_string.split_once(" ⚡ ") {
            let cmatch_stripped = cmatch.strip_prefix("c#").unwrap();
            found.entry(cmatch_stripped.to_string()).or_default().push(dest.to_string());
        }
        if found.len() >= 100 {
            break;
        }
    }

    let mut unique_found = Vec::new();
    let mut ambiguity = false;
    for (mat, destinations) in &found {
        unique_found.push(mat.clone());
        if destinations.len() > 1 {
            ambiguity = true;
            break;
        }
    }

    if ambiguity {
        unique_found.clear();
        let colons_pattern_already_has = pattern.matches("::").count();
        let cut_colons_at = colons_pattern_already_has + 2;
        // Dest always has pattern somewhere in the middle aaaa::bbbb::{pattern}cc
        for destinations in found.values() {
            for dest in destinations {
                let parts: Vec<&str> = dest.split("::").collect();
                if parts.len() >= cut_colons_at {
                    let more_colons_match = parts[parts.len() - cut_colons_at..].join("::");
                    unique_found.push(more_colons_match);
                }
            }
        }
    }

    unique_found.into_iter().collect()
}

pub async fn dump_database(ast_index: Arc<AMutex<AstDB>>)
{
    let db = ast_index.lock().await.sleddb.clone();
    println!("\nsled has {} records", db.len());
    let iter = db.iter();
    for item in iter {
        let (key, value) = item.unwrap();
        let key_string = String::from_utf8(key.to_vec()).unwrap();
        if key_string.starts_with("d#") {
            match serde_cbor::from_slice::<AstDefinition>(&value) {
                Ok(definition) => println!("{}\n{:?}", key_string, definition),
                Err(e) => println!("Failed to deserialize value at {}: {:?}", key_string, e),
            }
        } else if key_string.starts_with("classes#") {
            let value_string = String::from_utf8(value.to_vec()).unwrap();
            println!("{} 👉 {:?}", key_string, value_string);
        } else if key_string.starts_with("counters#") {
            let counter_value: i32 = serde_cbor::from_slice(&value).unwrap();
            println!("{}: {}", key_string, counter_value);
        } else if value.len() > 0 {
            println!("{} ({} bytes)", key_string, value.len());
        } else {
            println!("{}", key_string);
        }
    }
    println!("dump_database over");
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

    #[tokio::test]
    async fn test_ast_db() {
        init_tracing();
        let ast_index = ast_index_init().await;
        let mut errstats: ErrorStats = ErrorStats::default();

        let cpp_library_path = "src/ast/alt_testsuite/cpp_goat_library.h";
        let cpp_library_text = read_file(cpp_library_path);
        doc_add(ast_index.clone(), &cpp_library_path.to_string(), &cpp_library_text, &mut errstats).await.unwrap();

        let cpp_main_path = "src/ast/alt_testsuite/cpp_goat_main.cpp";
        let cpp_main_text = read_file(cpp_main_path);
        doc_add(ast_index.clone(), &cpp_main_path.to_string(), &cpp_main_text, &mut errstats).await.unwrap();

        for error in errstats.errors {
            println!("(E) {}:{} {}", error.err_cpath, error.err_line, error.err_message);
        }

        println!("Type hierachy:\n{}", type_hierarchy(ast_index.clone(), "cpp".to_string(), "".to_string()).await);
        println!("Type hierachy subtree_of=Animal:\n{}", type_hierarchy(ast_index.clone(), "cpp".to_string(), "cpp🔎Animal".to_string()).await);

        let mut ucx: ConnectUsageContext = connect_usages_look_if_full_reset_needed(ast_index.clone()).await;
        loop {
            let did_anything = connect_usages(ast_index.clone(), &mut ucx).await;
            if !did_anything {
                break;
            }
        }

        dump_database(ast_index.clone()).await;

        // Goat::Goat() is a C++ constructor
        let goat_def = definitions(ast_index.clone(), "Goat::Goat").await;
        let mut goat_def_str = String::new();
        for def in goat_def.iter() {
            goat_def_str.push_str(&format!("{:?}\n", def));
        }
        println!("goat_def_str:\n{}", goat_def_str);
        assert!(goat_def.len() == 1);

        let animalage_defs = definitions(ast_index.clone(), "Animal::age").await;
        let animalage_def0 = animalage_defs.first().unwrap();
        let animalage_usage = usages(ast_index.clone(), animalage_def0.path(), 100).await;
        let mut animalage_usage_str = String::new();
        for (used_at_def, used_at_uline) in animalage_usage.iter() {
            animalage_usage_str.push_str(&format!("{:}:{}\n", used_at_def.cpath, used_at_uline));
        }
        println!("animalage_usage_str:\n{}", animalage_usage_str);
        assert!(animalage_usage.len() == 5);

        doc_remove(ast_index.clone(), &cpp_library_path.to_string()).await;
        doc_remove(ast_index.clone(), &cpp_main_path.to_string()).await;

        dump_database(ast_index.clone()).await;
    }
}
