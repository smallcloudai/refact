use std::io::Cursor;
use image::ImageReader;
use regex::Regex;
use serde_json::Value;
use tokenizers::Tokenizer;

use crate::postprocessing::pp_context_files::RESERVE_FOR_QUESTION_AND_FOLLOWUP;


pub struct HasRagResults {
    pub was_sent: bool,
    pub in_json: Vec<Value>,
}

impl HasRagResults {
    pub fn new() -> Self {
        HasRagResults {
            was_sent: false,
            in_json: vec![],
        }
    }
}

impl HasRagResults {
    pub fn push_in_json(&mut self, value: Value) {
        self.in_json.push(value);
    }

    pub fn response_streaming(&mut self) -> Result<Vec<Value>, String> {
        if self.was_sent == true || self.in_json.is_empty() {
            return Ok(vec![]);
        }
        self.was_sent = true;
        Ok(self.in_json.clone())
    }
}

pub fn count_tokens(
    tokenizer: &Tokenizer,
    text: &str,
) -> usize {
    match tokenizer.encode(text, false) {
        Ok(tokens) => tokens.len(),
        Err(_) => 0,
    }
}

pub fn parse_image_b64_from_image_url_openai(image_url: &str) -> Option<(String, String, String)> {
    let re = Regex::new(r"data:(image/(png|jpeg|jpg|webp|gif));base64,([A-Za-z0-9+/=]+)").unwrap();
    re.captures(image_url).and_then(|captures| {
        let image_type = captures.get(1)?.as_str().to_string();
        let encoding = "base64".to_string();
        let value = captures.get(3)?.as_str().to_string();
        Some((image_type, encoding, value))
    })
}

pub fn max_tokens_for_rag_chat(n_ctx: usize, maxgen: usize) -> usize {
    (n_ctx/2).saturating_sub(maxgen).saturating_sub(RESERVE_FOR_QUESTION_AND_FOLLOWUP)
}

fn calculate_image_tokens_by_dimensions_openai(mut width: u32, mut height: u32) -> i32 {
    // as per https://platform.openai.com/docs/guides/vision
    const SMALL_CHUNK_SIZE: u32 = 512;
    const COST_PER_SMALL_CHUNK: i32 = 170;
    const BIG_CHUNK_SIZE: u32 = 2048;
    const CONST_COST: i32 = 85;

    let shrink_factor = (width.max(height) as f64) / (BIG_CHUNK_SIZE as f64);
    if shrink_factor > 1.0 {
        width = (width as f64 / shrink_factor) as u32;
        height = (height as f64 / shrink_factor) as u32;
    }

    let width_chunks = (width as f64 / SMALL_CHUNK_SIZE as f64).ceil() as u32;
    let height_chunks = (height as f64 / SMALL_CHUNK_SIZE as f64).ceil() as u32;
    let small_chunks_needed = width_chunks * height_chunks;

    small_chunks_needed as i32 * COST_PER_SMALL_CHUNK + CONST_COST
}

pub fn image_reader_from_b64string(image_b64: &str) -> Result<ImageReader<Cursor<Vec<u8>>>, String> {
    #[allow(deprecated)]
    let image_bytes = base64::decode(image_b64).map_err(|_| "base64 decode failed".to_string())?;
    let cursor = Cursor::new(image_bytes);
    let reader = ImageReader::new(cursor).with_guessed_format().map_err(|e| e.to_string())?;
    Ok(reader)
}

// for detail = high. all images w detail = low cost 85 tokens (independent of image size)
pub fn calculate_image_tokens_openai(image_string: &String, detail: &str) -> Result<i32, String> {
    let reader = image_reader_from_b64string(&image_string).map_err(|_| "Failed to read image".to_string())?;
    let (width, height) = reader.into_dimensions().map_err(|_| "Failed to get dimensions".to_string())?;

    match detail {
        "high" => Ok(calculate_image_tokens_by_dimensions_openai(width, height)),
        "low" => Ok(85),
        _ => Err("detail must be one of high or low".to_string()),
    }
}

// cargo test scratchpads::scratchpad_utils
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_image_tokens_by_dimensions_openai() {
        let width = 1024;
        let height = 1024;
        let expected_tokens = 765;
        let tokens = calculate_image_tokens_by_dimensions_openai(width, height);
        assert_eq!(tokens, expected_tokens, "Expected {} tokens, but got {}", expected_tokens, tokens);
    }

    #[test]
    fn test_parse_from_image_url_openai() {
        let image_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA";
        let expected_image_type = "image/png".to_string();
        let expected_encoding = "base64".to_string();
        let expected_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAUA".to_string();
        assert_eq!(
            parse_image_b64_from_image_url_openai(image_url),
            Some((expected_image_type, expected_encoding, expected_base64))
        );

        let invalid_image_url = "data:image/png;base64,";
        assert_eq!(parse_image_b64_from_image_url_openai(invalid_image_url), None);

        let non_matching_url = "https://example.com/image.png";
        assert_eq!(parse_image_b64_from_image_url_openai(non_matching_url), None);
    }
}
