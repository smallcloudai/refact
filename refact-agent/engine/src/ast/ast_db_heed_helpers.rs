use std::sync::Arc;
use indexmap::IndexMap;
use regex::Regex;
use lazy_static::lazy_static;
use heed;
use heed::types::Bytes;

use crate::ast::ast_structs::AstDefinition;
use crate::ast::ast_db::ConnectUsageContext;

lazy_static! {
    static ref MAGNIFYING_GLASS_RE: Regex = Regex::new(r"(\w+)🔎(\w+)").unwrap();
}

pub async fn connect_usages_helper(
    env: Arc<heed::Env>,
    db: heed::Database<heed::types::Str, Bytes>,
    ucx: &mut ConnectUsageContext,
    definition: &AstDefinition
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
    
    // Start a write transaction for batch operations
    let txn_result = env.write_txn();
    if let Err(e) = txn_result {
        tracing::error!("Failed to create write transaction: {:?}", e);
        return result;
    }
    
    let mut txn = txn_result.unwrap();
    
    for (uindex, usage) in definition.usages.iter().enumerate() {
        tracing::debug!("    resolving {}.usage[{}] == {:?}", official_path, uindex, usage);
        if !usage.resolved_as.is_empty() {
            ucx.usages_connected += 1;
            continue;
        }
        for to_resolve_unstripped in &usage.targets_for_guesswork {
            if !to_resolve_unstripped.starts_with("?::") {
                tracing::debug!("    homeless {}", to_resolve_unstripped);
                ucx.usages_homeless += 1;
                continue;
            }
            let to_resolve = to_resolve_unstripped.strip_prefix("?::").unwrap();
            tracing::debug!("    to resolve {}.usage[{}] guessing {}", official_path, uindex, to_resolve);

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
                tracing::debug!("        scanning {}", c_prefix);
                
                // Create a cursor to iterate through keys with the prefix
                if let Ok(mut cursor) = db.prefix_iter(&txn, &c_prefix) {
                    while let Some(Ok((key, _))) = cursor.next() {
                        let key_string = key.to_string();
                        let parts: Vec<&str> = key_string.split(" ⚡ ").collect();
                        if parts.len() == 2 {
                            if parts[0] == c_prefix {
                                let resolved_target = parts[1].trim();
                                found.push(resolved_target.to_string());
                            }
                        }
                    }
                }
                
                if found.len() > 0 {
                    break;
                }
            }
            tracing::debug!("        found {:?}", found);

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
            
            // Add the key-value pair to the database
            if let Err(e) = db.put(&mut txn, &u_key, &serde_cbor::to_vec(&usage.uline).unwrap()) {
                tracing::error!("Failed to put key-value pair: {:?}", e);
                continue;
            }
            
            tracing::debug!("        add {:?} <= {}", u_key, usage.uline);
            all_saved_ulinks.push(u_key);
            result.push((usage.uline, single_thing_found));
            ucx.usages_connected += 1;
            break;  // the next thing from targets_for_guesswork is a worse query, keep this one and exit
        }
    } // for usages
    
    // Save cleanup key
    let cleanup_key = format!("resolve-cleanup|{}", definition.official_path.join("::"));
    let cleanup_value = serde_cbor::to_vec(&all_saved_ulinks).unwrap();
    
    if let Err(e) = db.put(&mut txn, &cleanup_key, &cleanup_value) {
        tracing::error!("Failed to put cleanup key-value pair: {:?}", e);
    }
    
    // Commit the transaction
    if let Err(e) = txn.commit() {
        tracing::error!("Failed to commit transaction: {:?}", e);
    }
    
    result
}

pub async fn derived_from(env: &heed::Env, db: heed::Database<heed::types::Str, Bytes>) -> IndexMap<String, Vec<String>> {
    // Data example:
    // classes/cpp🔎Animal ⚡ alt_testsuite::cpp_goat_library::Goat 👉 "cpp🔎Goat"
    let mut derived_map: IndexMap<String, Vec<String>> = IndexMap::new();
    let t_prefix = "classes|";
    
    // Create a read transaction
    if let Ok(txn) = env.read_txn() {
        // Create a cursor to iterate through keys with the prefix
        if let Ok(mut cursor) = db.prefix_iter(&txn, t_prefix) {
            while let Some(Ok((key, value))) = cursor.next() {
                let key_string = key.to_string();
                let value_string = std::str::from_utf8(value).unwrap_or_default().to_string();
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
        }
    }
    
    // Have perfectly good [child, [parent1, parent2, ..]]
    // derived_map {"cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat"]}
    // Now we need to post-process this into [child, [parent1, parent_of_parent1, parent2, parent_of_parent2, ...]]
    fn build_all_derived_from(
        klass: &str,
        derived_map: &IndexMap<String, Vec<String>>,
        all_derived_from: &mut IndexMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
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
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        build_all_derived_from(klass, &derived_map, &mut all_derived_from, &mut visited);
    }
    // now have all_derived_from {"cpp🔎CosmicGoat": ["cpp🔎CosmicJustice", "cpp🔎Goat", "cpp🔎Animal"], "cpp🔎CosmicJustice": [], "cpp🔎Goat": ["cpp🔎Animal"], "cpp🔎Animal": []}
    all_derived_from
}
