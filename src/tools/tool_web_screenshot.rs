use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;
use base64::Engine;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::scratchpads::chat_message::{ChatContent, ChatMessage, ChatMultimodalElement, MultimodalElementImage};
use crate::tools::tools_description::Tool;

use headless_chrome::Browser;
use headless_chrome::protocol::cdp::Page;

use tracing::info;

pub struct ToolWebScreenshot;

#[async_trait]
impl Tool for ToolWebScreenshot {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let url = match args.get("url") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `url` is not a string: {:?}", v)),
            None => return Err("Missing argument `url`".to_string())
        };

        let screenshot = screenshot_jpeg_base64(&url).await?;
        let multimodal_element = ChatMultimodalElement::MultiModalImageURLElement(
            MultimodalElementImage::new(screenshot.clone())
        );

        info!("Made screenshot of {} page: {}", url, screenshot.len());
        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText("Screenshot of {} page".to_string()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "user".to_string(),  // Image URLs are only allowed for messages with role 'user'
            content: ChatContent::Multimodal(vec![multimodal_element]),
            ..Default::default()
        }));

        Ok((false, results))
    }
}

async fn screenshot_jpeg_base64(url: &str) -> Result<String, String> {
    let browser = Browser::default().map_err(|e| e.to_string())?;
    let tab = browser.new_tab().map_err(|e| e.to_string())?;
    tab.navigate_to(url).map_err(|e| e.to_string())?;
    let jpeg_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Jpeg,
        Some(75),
        None,
        true).map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", base64::prelude::BASE64_STANDARD.encode(&jpeg_data)))
}
