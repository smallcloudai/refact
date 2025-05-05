use std::collections::HashMap;
use std::sync::Arc;
use tokenizers::Tokenizer;
use crate::call_validation::ChatMessage;

pub struct TokenCountCache {
    cache: HashMap<String, i32>,
    hits: usize,
    misses: usize,
}

impl TokenCountCache {
    pub fn new() -> Self {
        TokenCountCache {
            cache: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }
    
    fn cache_key(msg: &ChatMessage) -> String {
        // Use role and content as the key
        // This is sufficient because we only care about the tokenization of the text
        format!("{}:{}", msg.role, msg.content.content_text_only())
    }
    
    pub fn get_token_count(
        &mut self,
        msg: &ChatMessage,
        tokenizer: Option<Arc<Tokenizer>>,
        extra_tokens_per_message: i32,
    ) -> Result<i32, String> {
        let key = Self::cache_key(msg);
        
        if let Some(&count) = self.cache.get(&key) {
            // Cache hit
            self.hits += 1;
            return Ok(count);
        }
        
        // Cache miss - compute the token count
        self.misses += 1;
        let content_tokens = msg.content.count_tokens(tokenizer, &None)?;
        let total_tokens = extra_tokens_per_message + content_tokens;
        
        // Cache the result
        self.cache.insert(key, total_tokens);
        
        Ok(total_tokens)
    }
    
    pub fn invalidate(&mut self, msg: &ChatMessage) {
        let key = Self::cache_key(msg);
        self.cache.remove(&key);
    }
    
    pub fn stats(&self) -> (usize, usize, f32) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        };
        (self.hits, self.misses, hit_rate)
    }
}