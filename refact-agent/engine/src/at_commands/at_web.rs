use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tracing::{info, warn};

use reqwest::Client;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use select::predicate::{Attr, Name};
use html2text::render::text_renderer::{TaggedLine, TextDecorator};
use serde_json::Value;

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AtWeb {
    pub params: Vec<Box<dyn AtParam>>,
}

impl AtWeb {
    pub fn new() -> Self {
        AtWeb {
            params: vec![],
        }
    }
}

#[async_trait]
impl AtCommand for AtWeb {
    fn params(&self) -> &Vec<Box<dyn AtParam>> {
        &self.params
    }

    async fn at_execute(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        cmd: &mut AtCommandMember,
        args: &mut Vec<AtCommandMember>,
    ) -> Result<(Vec<ContextEnum>, String), String> {
        let url = match args.get(0) {
            Some(x) => x.clone(),
            None => {
                cmd.ok = false; cmd.reason = Some("missing URL".to_string());
                args.clear();
                return Err("missing URL".to_string());
            }
        };
        args.truncate(1);

        let preview_cache = {
            let gcx = ccx.lock().await.global_context.clone();
            let gcx_read = gcx.read().await;
            gcx_read.at_commands_preview_cache.clone()
        };
        let text_from_cache = preview_cache.lock().await.get(&format!("@web:{}", url.text));

        let text = match text_from_cache {
            Some(text) => text,
            None => {
                let text = execute_at_web(&url.text, None).await
                    .map_err(|e| format!("Failed to execute @web {}.\nError: {e}", url.text))?;
                preview_cache.lock().await.insert(format!("@web:{}", url.text), text.clone());
                text
            }
        };

        let message = ChatMessage::new(
            "plain_text".to_string(),
            text,
        );

        info!("executed @web {}", url.text);
        Ok((vec![ContextEnum::ChatMessage(message)], format!("[see text downloaded from {} above]", url.text)))
    }

    fn depends_on(&self) -> Vec<String> {
        vec![]
    }
}

const JINA_READER_BASE_URL: &str = "https://r.jina.ai/";
const JINA_TIMEOUT_SECS: u64 = 60;
const FALLBACK_TIMEOUT_SECS: u64 = 10;

pub async fn execute_at_web(url: &str, options: Option<&HashMap<String, Value>>) -> Result<String, String> {
    match fetch_with_jina_reader(url, options).await {
        Ok(text) => {
            info!("successfully fetched {} via Jina Reader", url);
            Ok(text)
        }
        Err(jina_err) => {
            warn!("Jina Reader failed for {}: {}, falling back to simple fetch", url, jina_err);
            match fetch_simple(url).await {
                Ok(text) => {
                    info!("successfully fetched {} via simple fetch (fallback)", url);
                    Ok(text)
                }
                Err(simple_err) => {
                    Err(format!("Both Jina Reader and simple fetch failed.\nJina error: {}\nSimple fetch error: {}", jina_err, simple_err))
                }
            }
        }
    }
}

async fn fetch_with_jina_reader(url: &str, options: Option<&HashMap<String, Value>>) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(JINA_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let jina_url = format!("{}{}", JINA_READER_BASE_URL, url);
    let mut request = client.get(&jina_url).header("User-Agent", "RefactAgent/1.0");

    let mut is_streaming = false;

    if let Some(opts) = options {
        if let Some(Value::String(v)) = opts.get("respond_with") {
            request = request.header("x-respond-with", v.as_str());
        }
        if let Some(Value::String(v)) = opts.get("target_selector") {
            request = request.header("x-target-selector", v.as_str());
        }
        if let Some(Value::String(v)) = opts.get("wait_for_selector") {
            request = request.header("x-wait-for-selector", v.as_str());
        }
        if let Some(Value::Number(n)) = opts.get("timeout") {
            if let Some(t) = n.as_u64() {
                request = request.header("x-timeout", t.to_string());
            }
        }
        if let Some(Value::Bool(true)) = opts.get("no_cache") {
            request = request.header("x-no-cache", "true");
        }
        if let Some(Value::Number(n)) = opts.get("cache_tolerance") {
            if let Some(t) = n.as_u64() {
                request = request.header("x-cache-tolerance", t.to_string());
            }
        }
        if let Some(Value::Bool(true)) = opts.get("with_generated_alt") {
            request = request.header("x-with-generated-alt", "true");
        }
        if let Some(Value::Bool(true)) = opts.get("streaming") {
            request = request.header("Accept", "text/event-stream");
            is_streaming = true;
        }
        if let Some(Value::String(v)) = opts.get("set_cookie") {
            request = request.header("x-set-cookie", v.as_str());
        }
        if let Some(Value::String(v)) = opts.get("proxy_url") {
            request = request.header("x-proxy-url", v.as_str());
        }
    }

    let response = request.send().await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Jina Reader returned status: {}", response.status()));
    }

    let text = if is_streaming {
        parse_streaming_response(response).await?
    } else {
        response.text().await.map_err(|e| e.to_string())?
    };

    if text.trim().is_empty() {
        return Err("Jina Reader returned empty content".to_string());
    }

    Ok(text)
}

async fn parse_streaming_response(response: reqwest::Response) -> Result<String, String> {
    let text = response.text().await.map_err(|e| e.to_string())?;
    let mut last_content = String::new();

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                    last_content = content.to_string();
                }
            } else if !data.trim().is_empty() {
                last_content = data.to_string();
            }
        }
    }

    if last_content.is_empty() {
        Ok(text)
    } else {
        Ok(last_content)
    }
}

#[derive(Clone, Copy)]
struct CustomTextConversion;

impl TextDecorator for CustomTextConversion {
    type Annotation = ();

    fn decorate_link_start(&mut self, _url: &str) -> (String, Self::Annotation) {
        ("[".to_string(), ())
    }

    fn decorate_link_end(&mut self) -> String {
        "]".to_string()
    }

    fn decorate_em_start(&self) -> (String, Self::Annotation) {
        ("*".to_string(), ())
    }

    fn decorate_em_end(&self) -> String {
        "*".to_string()
    }

    fn decorate_strong_start(&self) -> (String, Self::Annotation) {
        ("**".to_string(), ())
    }

    fn decorate_strong_end(&self) -> String {
        "**".to_string()
    }

    fn decorate_strikeout_start(&self) -> (String, Self::Annotation) {
        ("".to_string(), ())
    }

    fn decorate_strikeout_end(&self) -> String {
        "".to_string()
    }

    fn decorate_code_start(&self) -> (String, Self::Annotation) {
        ("`".to_string(), ())
    }

    fn decorate_code_end(&self) -> String {
        "`".to_string()
    }

    fn decorate_preformat_first(&self) -> Self::Annotation {}
    fn decorate_preformat_cont(&self) -> Self::Annotation {}

    fn decorate_image(&mut self, _src: &str, title: &str) -> (String, Self::Annotation) {
        (format!("[{}]", title), ())
    }

    fn header_prefix(&self, level: usize) -> String {
        "#".repeat(level) + " "
    }

    fn quote_prefix(&self) -> String {
        "> ".to_string()
    }

    fn unordered_item_prefix(&self) -> String {
        "* ".to_string()
    }

    fn ordered_item_prefix(&self, i: i64) -> String {
        format!("{}. ", i)
    }

    fn make_subblock_decorator(&self) -> Self {
        *self
    }

    fn finalise(&mut self, _: Vec<String>) -> Vec<TaggedLine<()>> {
        vec![]
    }
}

fn find_content(html: String) -> String {
    let document = select::document::Document::from(html.as_str());
    let content_ids = vec![
        "content",
        "I_content",
        "main-content",
        "main_content",
        "CONTENT",
    ];
    for id in content_ids {
        if let Some(node) = document.find(Attr("id", id)).next() {
            return node.html();
        }
    }
    if let Some(node) = document.find(Name("article")).next() {
        return node.html();
    }
    if let Some(node) = document.find(Name("main")).next() {
        return node.html();
    }
    html
}

async fn fetch_html(url: &str, timeout: Duration) -> Result<String, String> {
    let client = Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Connection", "keep-alive")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Cache-Control", "max-age=0")
        .header("DNT", "1")
        .header("Referer", "https://www.google.com/")
        .send().await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("unable to fetch url: {}; status: {}", url, response.status()));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

async fn fetch_simple(url: &str) -> Result<String, String> {
    let html = fetch_html(url, Duration::from_secs(FALLBACK_TIMEOUT_SECS)).await?;
    let html = find_content(html);

    let text = html2text::config::with_decorator(CustomTextConversion)
        .string_from_read(&html.as_bytes()[..], 200)
        .map_err(|_| "Unable to convert html to text".to_string())?;

    Ok(text)
}


#[cfg(test)]
mod tests {
    use tracing::warn;
    use super::*;

    #[tokio::test]
    async fn test_execute_at_web_jina() {
        let url = "https://doc.rust-lang.org/book/ch03-04-comments.html";
        match execute_at_web(url, None).await {
            Ok(text) => info!("test executed successfully (length: {} chars):\n\n{}", text.len(), &text[..text.len().min(500)]),
            Err(e) => warn!("test failed with error: {e}"),
        }
    }

    #[tokio::test]
    async fn test_jina_pdf_reading() {
        let url = "https://www.w3.org/WAI/WCAG21/Techniques/pdf/PDF1.pdf";
        match execute_at_web(url, None).await {
            Ok(text) => info!("PDF test executed successfully (length: {} chars)", text.len()),
            Err(e) => warn!("PDF test failed with error: {e}"),
        }
    }

    #[tokio::test]
    async fn test_jina_with_options() {
        let url = "https://doc.rust-lang.org/book/ch03-04-comments.html";
        let mut options = HashMap::new();
        options.insert("target_selector".to_string(), Value::String("main".to_string()));
        match execute_at_web(url, Some(&options)).await {
            Ok(text) => info!("options test executed successfully (length: {} chars)", text.len()),
            Err(e) => warn!("options test failed with error: {e}"),
        }
    }
}
