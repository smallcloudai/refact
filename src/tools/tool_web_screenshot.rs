use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::scratchpads::chat_message::{ChatContent, ChatMessage, ChatMultimodalElement, MultimodalElementImage};
use crate::tools::tools_description::Tool;

use headless_chrome::{Browser, LaunchOptions, Tab};
use headless_chrome::protocol::cdp::Page;
use reqwest::Client;
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

        let attach_html = match args.get("html") {
            Some(Value::Bool(option)) => option.clone(),
            Some(v) => return Err(format!("argument `html` is not boolean: {:?}", v)),
            None => false
        };

        let launch_options = LaunchOptions {
            window_size: Some((1024, 768)),
            ..Default::default()
        };
        let browser = Browser::new(launch_options).map_err(|e| e.to_string())?;
        let tab = browser.new_tab().map_err(|e| e.to_string())?;
        tab.navigate_to(url.as_str()).map_err(|e| e.to_string())?;
        tab.wait_until_navigated().map_err(|e| e.to_string())?;

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(format!("web screenshot results for {}", url)),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        let screenshot_message = screenshot_jpeg_base64(&tab).await?;
        results.push(ContextEnum::ChatMessage(screenshot_message));

        if attach_html {
            let content: String;

            let client = Client::builder()
                .build()
                .map_err(|e| e.to_string())?;

            let response = client.get(url.clone()).send().await.map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                content = format!("unable to fetch url: {}; status: {}", url, response.status());
            } else {
                content = response.text().await.map_err(|e| e.to_string())?;
            }

            results.push(ContextEnum::ChatMessage(ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(content),
                ..Default::default()
            }));
        }

        Ok((false, results))
    }
}

async fn screenshot_jpeg_base64(tab: &Arc<Tab>) -> Result<ChatMessage, String> {
    // let browser = Browser::default().map_err(|e| e.to_string())?;
    // let launch_options = LaunchOptions {
    //     window_size: Some((1024, 768)),
    //     ..Default::default()
    // };
    // let browser = Browser::new(launch_options).map_err(|e| e.to_string())?;
    // let tab = browser.new_tab().map_err(|e| e.to_string())?;
    // tab.navigate_to(url).map_err(|e| e.to_string())?;
    // tab.wait_until_navigated().map_err(|e| e.to_string())?;
    // let current_bounds = tab.get_bounds().map_err(|e| e.to_string())?;
    //
    // info!("current_bounds {} {} {:?}", current_bounds.width, current_bounds.height, current_bounds.state);
    // let jpeg_data = tab.capture_screenshot(
    //     Page::CaptureScreenshotFormatOption::Jpeg,
    //     Some(75),
    //     None,
    //     true).map_err(|e| e.to_string())?;
    // pub fn get_content(&self) -> Result<String>
    // Ok(format!("data:image/jpeg;base64,{}", base64::prelude::BASE64_STANDARD.encode(&jpeg_data)))

    // pub fn capture_screenshot(
    //         &self,
    //         format: Page::CaptureScreenshotFormatOption,
    //         quality: Option<u32>,
    //         clip: Option<Page::Viewport>,
    //         from_surface: bool,
    //     ) -> Result<Vec<u8>> {
    //         let data = self
    //             .call_method(Page::CaptureScreenshot {
    //                 format: Some(format),
    //                 clip,
    //                 quality,
    //                 from_surface: Some(from_surface),
    //                 capture_beyond_viewport: None,
    //             })?
    //             .data;
    //         base64::prelude::BASE64_STANDARD
    //             .decode(data)
    //             .map_err(Into::into)
    //     }

    let jpeg_data = tab.call_method(Page::CaptureScreenshot {
        format: Some(Page::CaptureScreenshotFormatOption::Jpeg),
        clip: None,
        quality: Some(75),
        from_surface: Some(true),
        capture_beyond_viewport: Some(true),
    }).map_err(|e| e.to_string())?.data;

    let screenshot_content = format!("data:image/jpeg;base64,{}", jpeg_data);
    let multimodal_element = ChatMultimodalElement::MultiModalImageURLElement(
        MultimodalElementImage::new(screenshot_content.clone())
    );

    Ok(ChatMessage {
        role: "user".to_string(),  // Image URLs are only allowed for messages with role 'user'
        content: ChatContent::Multimodal(vec![multimodal_element]),
        ..Default::default()
    })
}
