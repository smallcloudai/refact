use crate::call_validation::CodeCompletionInputs;
use crate::call_validation::CodeCompletionPost;
use crate::call_validation::SamplingParameters;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::collections::HashMap;

// use ropey::RopeSlice;
use ropey::Rope;
use tracing::info;

const CACHE_ENTRIES: usize = 500;
const CACHE_KEY_CHARS: usize = 5000;
// max memory CACHE_KEY_CHARS * CACHE_ENTRIES = 2500000 = 2.5M


#[derive(Debug)]
pub struct CompletionCache {
    pub map: HashMap<String, serde_json::Value>,
    pub in_added_order: Vec<String>,
}

pub fn cache_key(
    post: &CodeCompletionPost,
) -> String {
    let text_maybe = post.inputs.sources.get(&post.inputs.cursor.file);
    if let None = text_maybe {
        // Don't handle it there, validation should have caught it
        return format!("dummy1-{}:{}", post.inputs.cursor.line, post.inputs.cursor.character);
    }
    let rope = Rope::from_str(text_maybe.unwrap());
    let cursor_line_maybe = rope.get_line(post.inputs.cursor.line as usize);
    if let None = cursor_line_maybe {
        return format!("dummy2-{}:{}", post.inputs.cursor.line, post.inputs.cursor.character);
    }
    let mut cursor_line = cursor_line_maybe.unwrap().to_string();
    let cpos = post.inputs.cursor.character as usize;
    if cpos < cursor_line.len() {
        cursor_line = cursor_line[..cpos].to_string();
    }
    let mut before_iter = rope.lines_at(post.inputs.cursor.line as usize).reversed();
    let mut linesvec = Vec::<String>::new();
    let mut bytes = 0;
    loop {
        let line_maybe = before_iter.next();
        if let None = line_maybe {
            break;
        }
        let line = line_maybe.unwrap();
        info!("cache key line prev: {:?}", line);
        let line_str = line.to_string();
        bytes += line_str.len();
        linesvec.push(line_str.replace("\r", ""));
        if bytes > CACHE_KEY_CHARS {
            break;
        }
    }
    linesvec.reverse();
    let mut key = format!("/{}/{}/", &post.model, &post.scratchpad);
    key.push_str(&linesvec.join(""));
    key.push_str(&cursor_line.to_string());
    if key.len() > CACHE_KEY_CHARS {
        key = key[..CACHE_KEY_CHARS].to_string();
    }
    return key;
}

pub fn cache_get(
    cache: Arc<StdRwLock<CompletionCache>>,
    key: String,
) -> Option<serde_json::Value> {
    let cache_locked = cache.write().unwrap();
    if let Some(value) = cache_locked.map.get(&key) {
        return Some(value.clone());
    }
    None
}

pub fn cache_put(
    cache: Arc<StdRwLock<CompletionCache>>,
    new_key: String,
    value: serde_json::Value,
) {
    let mut cache_locked = cache.write().unwrap();
    // let mut map = &mut cache_locked.map;
    // if let Some(value) = map.get(&new_key) {
    //     map.remove(&new_key).unwrap();
    //     map.insert(new_key.clone(), value.clone());
    // }
    while cache_locked.in_added_order.len() > CACHE_ENTRIES {
        let old_key = cache_locked.in_added_order.remove(0);
        cache_locked.map.remove(&old_key).unwrap();  // will crash if not present, works as an assert
    }
    info!("cache put: {:?} = {:?}", new_key, value);
    let mut new_key_copy = new_key.clone();
    if new_key_copy.len() > CACHE_KEY_CHARS {
        new_key_copy = new_key_copy[..CACHE_KEY_CHARS].to_string();
    }
    cache_locked.map.entry(new_key_copy.clone()).or_insert(value);
    cache_locked.in_added_order.push(new_key_copy.clone());
}

impl CompletionCache {
    pub fn new(
    ) -> Self {
        Self {
            map: HashMap::new(),
            in_added_order: Vec::new(),
        }
    }
}

