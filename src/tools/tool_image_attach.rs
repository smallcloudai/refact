use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum};
use crate::scratchpads::multimodality::MultimodalElement;

use resvg::usvg;
use resvg::tiny_skia;


pub struct ToolImageAttach;


#[async_trait]
impl Tool for ToolImageAttach {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let paths = match args.get("paths") {
            Some(Value::String(s)) => {
                let paths = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
                paths
            },
            Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
            None => return Err("Missing argument `paths`".to_string())
        };

        let mut tool_messages = vec!["Attach images log".to_string()];
        let mut image_messages = vec![];
        for path in paths.iter() {
            let message = load_image(path).await;
            if let Err(e) = message {
                tool_messages.push(e);
            } else {
                tool_messages.push(format!("Successfully attached {}", path.clone()));
                image_messages.push(message.unwrap());
            }
        }

        let tool_message = ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(tool_messages.join("\n")),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        });

        let mut results = vec![tool_message];
        results.extend(image_messages);

        Ok((false, results))
    }
}

async fn load_image(path: &String) -> Result<ContextEnum, String> {
    let extension = path.split(".").last().unwrap().to_string();

    let m_type: String;
    let data;
    if extension == "png" {
        m_type = "image/png".to_string();
        data = std::fs::read(path).map_err(|_| format!("{} read failed", path))?;
    } else if extension == "jpeg" || extension == "jpg" {
        m_type = "image/jpeg".to_string();
        data = std::fs::read(path).map_err(|_| format!("{} read failed", path))?;
    } else if extension == "svg" {
        m_type = "image/png".to_string();
        let tree = {
            let mut opt = usvg::Options::default();
            opt.resources_dir = std::fs::canonicalize(&path)
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));
            opt.fontdb_mut().load_system_fonts();

            let svg_data = std::fs::read(&path).unwrap();
            usvg::Tree::from_data(&svg_data, &opt).unwrap()
        };
        let pixmap_size = tree.size().to_int_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        data = pixmap.encode_png().map_err(|_| format!("{} encode_png failed", path))?;
    } else {
        return Err(format!("Unsupported image format (extension): {}", extension));
    }
    #[allow(deprecated)]
    let m_content = base64::encode(&data);

    let image = MultimodalElement::new(
        m_type,
        m_content,
    ).map_err(|e| e.to_string())?;

    let text = MultimodalElement::new(
        "text".to_string(),
        path.clone(),
    ).map_err(|e| e.to_string())?;

    Ok(ContextEnum::ChatMessage(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::Multimodal(vec![text, image]),
        ..Default::default()
    }))
}
