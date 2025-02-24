use crate::call_validation::CodeCompletionPost;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::collections::HashMap;

use ropey::Rope;
// use tracing::info;

const CACHE_ENTRIES: usize = 500;
const CACHE_KEY_CHARS: usize = 5000;  // max memory CACHE_KEY_CHARS * CACHE_ENTRIES = 2500000 = 2.5M


// aggregate this struct in scratchpad to save cache
#[derive(Debug, Clone)]
pub struct CompletionSaveToCache {
    pub cache_arc: Arc<StdRwLock<CompletionCache>>,
    pub cache_key: (String, String),
    pub completion0_text: String,
    pub completion0_finish_reason: String,
    pub completion0_snippet_telemetry_id: Option<u64>,
    pub model: String,
}

impl CompletionSaveToCache {
    pub fn new(
        cache_arc: Arc<StdRwLock<CompletionCache>>,
        post: &CodeCompletionPost
    ) -> Self {
        CompletionSaveToCache {
            cache_arc: cache_arc.clone(),
            cache_key: cache_key_from_post(post),
            completion0_text: String::new(),
            completion0_finish_reason: String::new(),
            completion0_snippet_telemetry_id: None,
            model: post.model.clone(),
        }
    }
}


#[derive(Debug)]
pub struct CompletionCache {
    pub map: HashMap<(String, String), serde_json::Value>,
    pub in_added_order: Vec<(String, String)>,
}

impl CompletionCache {
    pub fn new(
    ) -> Self {
        Self { map: HashMap::new(), in_added_order: Vec::new() }
    }
}

pub fn cache_get(
    cache: Arc<StdRwLock<CompletionCache>>,
    key: (String, String),
) -> Option<serde_json::Value> {
    let cache_locked = cache.write().unwrap();
    if let Some(value) = cache_locked.map.get(&key) {
        return Some(value.clone());
    }
    None
}

pub fn cache_put(
    cache: Arc<StdRwLock<CompletionCache>>,
    new_key: (String, String),
    value: serde_json::Value,
) {
    let mut cache_locked = cache.write().unwrap();
    while cache_locked.in_added_order.len() > CACHE_ENTRIES {
        let old_key = cache_locked.in_added_order.remove(0);
        cache_locked.map.remove(&old_key);
    }
    // info!("cache put: {:?} = {:?}", new_key, value);
    let mut new_key_copy = new_key.clone();
    let k0_chars = new_key_copy.0.chars();
    if k0_chars.clone().count() > CACHE_KEY_CHARS {
        new_key_copy.0 = k0_chars.clone().skip(k0_chars.count() - CACHE_KEY_CHARS).collect();
    }
    cache_locked.map.entry(new_key_copy.clone()).or_insert(value);
    cache_locked.in_added_order.push(new_key_copy.clone());
}

pub fn cache_key_from_post(
    post: &CodeCompletionPost,
) -> (String, String) {
    // Change this function only together with the function below, it fills the cache ahead of cursor
    // directly manupulating the cache key.
    let text_maybe = post.inputs.sources.get(&post.inputs.cursor.file);
    if let None = text_maybe {
        // Don't handle it there, validation should have caught it
        return (format!("dummy1-{}:{}", post.inputs.cursor.line, post.inputs.cursor.character), "".to_string());
    }
    let rope = Rope::from_str(text_maybe.unwrap());
    let cursor_line_maybe = rope.get_line(post.inputs.cursor.line as usize);
    if let None = cursor_line_maybe {
        return (format!("dummy2-{}:{}", post.inputs.cursor.line, post.inputs.cursor.character), "".to_string());
    }
    let mut cursor_line = cursor_line_maybe.unwrap();
    let cpos = post.inputs.cursor.character as usize;
    if cpos < cursor_line.len_chars() {
        cursor_line = cursor_line.slice(..cpos);
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
        // info!("cache key line prev: {:?}", line);
        let line_str = line.to_string();
        linesvec.push(line_str.replace("\r", ""));
        bytes += line.len_chars();
        if bytes > CACHE_KEY_CHARS {
            break;
        }
    }
    linesvec.reverse();
    let mut key = "".to_string();
    key.push_str(&linesvec.join(""));
    key.push_str(&cursor_line.to_string());
    let chars = key.chars();

    if chars.clone().count() > CACHE_KEY_CHARS {
        key = chars.skip(key.len() - CACHE_KEY_CHARS).collect();
    }
    return (key, cache_part2_from_post(post));
}


pub fn cache_part2_from_post(post: &CodeCompletionPost) -> String {
    if post.inputs.multiline { "multiline".to_string() } else { "singleline".to_string() }
}


impl Drop for CompletionSaveToCache {
    fn drop(&mut self) {
        // flush to cache on destruction
        if self.completion0_finish_reason.is_empty() { // error happened, no nothing happened (prompt only request)
            return;
        }
        let mut believe_chars = self.completion0_text.len();
        if self.completion0_finish_reason == "length" {
            // Model stopped because of max tokens, there is a continuation, so it's good for cache in the beginning, but don't believe it to the end.
            // For example CODECODECODECOMPLETION| with empty completion is obviously junk as cache.
            // And it's not junk for "stop", it actually saves one model call after accepting each completion.
            believe_chars = believe_chars.checked_sub(10).unwrap_or(0);
        } else {
            believe_chars += 1;
        }
        for char_num in 0..believe_chars {
            let code_completion_ahead: String = self.completion0_text.chars().skip(char_num).collect();
            let cache_key_ahead: (String, String) = (
                self.cache_key.0.clone() + &self.completion0_text.chars().take(char_num).collect::<String>(),
                self.cache_key.1.clone()
            );
            cache_put(self.cache_arc.clone(), cache_key_ahead, serde_json::json!(
                {
                    "choices": [{
                        "index": 0,
                        "code_completion": code_completion_ahead,
                        "finish_reason": self.completion0_finish_reason,
                    }],
                    "model": self.model,
                    "cached": true,
                    "snippet_telemetry_id": self.completion0_snippet_telemetry_id,
                }
            ));
        }
    }
}
