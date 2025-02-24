use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use reqwest::Client;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use select::predicate::{Attr, Name};
use html2text::render::text_renderer::{TaggedLine, TextDecorator};

use crate::at_commands::at_commands::{AtCommand, AtCommandsContext, AtParam};
use crate::at_commands::execute_at::AtCommandMember;
use crate::call_validation::{ChatMessage, ContextEnum};


pub struct AtWeb {
    pub params: Vec<Arc<AMutex<dyn AtParam>>>,
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
    fn params(&self) -> &Vec<Arc<AMutex<dyn AtParam>>> {
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
                let text = execute_at_web(&url.text).await.map_err(|e|format!("Failed to execute @web {}.\nError: {e}", url.text))?;
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

pub async fn execute_at_web(url: &str) -> Result<String, String>{
    let html = fetch_html(url, Duration::from_secs(5)).await?;
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
    async fn test_execute_at_web() {
        let url = "https://doc.rust-lang.org/book/ch03-04-comments.html";
        match execute_at_web(url).await {
            Ok(text) => info!("Test executed successfully:\n\n{text}"),
            Err(e) => warn!("Test failed with error: {e}"),
        }
    }
}
