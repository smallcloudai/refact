use std::any::Any;
use std::sync::Arc;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use serde_json::Value;
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use async_trait::async_trait;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::integrations::sessions::{IntegrationSession, get_session_hashmap_key};
use crate::global_context::GlobalContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::scratchpads::multimodality::MultimodalElement;
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};

use reqwest::Client;
use std::path::PathBuf;
use headless_chrome::{Browser, LaunchOptions, Tab};
use headless_chrome::browser::tab::point::Point;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::protocol::cdp::Emulation;
use serde::{Deserialize, Serialize};
use std::fmt;

use base64::Engine;
use std::io::Cursor;
use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationChrome {
    pub chrome_path: Option<String>,
    pub window_size: Option<Vec<u32>>,
    pub idle_browser_timeout: Option<u32>,
    #[serde(default = "default_headless")]
    pub headless: bool,
}

fn default_headless() -> bool { true }

pub struct ToolChrome {
    integration_chrome: IntegrationChrome,
    supports_clicks: bool,
}

#[derive(Clone, Debug)]
enum DeviceType {
    DESKTOP,
    MOBILE,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceType::DESKTOP => write!(f, "desktop"),
            DeviceType::MOBILE => write!(f, "mobile"),
        }
    }
}

#[derive(Clone)]
pub struct ChromeTab {
    instance: Arc<Tab>,
    device: DeviceType,
    tab_id: String,
    screenshot_scale_factor: f64,
}

impl ChromeTab {
    fn new(instance: Arc<Tab>, device: &DeviceType, tab_id: &String) -> Self {
        Self {
            instance,
            device: device.clone(),
            tab_id: tab_id.clone(),
            screenshot_scale_factor: 1.0,
        }
    }
    pub fn state_string(&self) -> String {
        format!("tab_id `{}` device `{}` uri `{}`", self.tab_id.clone(), self.device, self.instance.get_url())
    }
}

struct ChromeSession {
    browser: Browser,
    tabs: HashMap<String, ChromeTab>,
}

impl ChromeSession {
    fn is_connected(&self) -> bool {
        match self.browser.get_version() {
            Ok(_) => {
                true
            },
            Err(_) => {
                false
            }
        }
    }
}

impl IntegrationSession for ChromeSession
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_expired(&self) -> bool { false }
    fn try_stop(&mut self) -> Box<dyn Future<Output = String> + Send + '_> {
        Box::new(async { "".to_string() })
    }
}

impl ToolChrome {
    pub fn new_from_yaml(v: &serde_yaml::Value, supports_clicks: bool,) -> Result<Self, String> {
        let integration_chrome = serde_yaml::from_value::<IntegrationChrome>(v.clone()).map_err(|e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("{}{}", e.to_string(), location)
        })?;
        Ok(Self {
            integration_chrome,
            supports_clicks,
        })
    }
}

#[async_trait]
impl Tool for ToolChrome {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let (gcx, chat_id) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.global_context.clone(), ccx_lock.chat_id.clone())
        };

        let commands_str = match args.get("commands") {
            Some(Value::String(s)) => s,
            Some(v) => return Err(format!("argument `commands` is not a string: {:?}", v)),
            None => return Err("Missing argument `commands`".to_string())
        };

        let session_hashmap_key = get_session_hashmap_key("chrome", &chat_id);
        let mut tool_log = setup_chrome_session(gcx.clone(), &self.integration_chrome, &session_hashmap_key).await?;

        let command_session = {
            let gcx_locked = gcx.read().await;
            gcx_locked.integration_sessions.get(&session_hashmap_key)
                .ok_or(format!("Error getting chrome session for chat: {}", chat_id))?
                .clone()
        };
        let mut command_session_locked = command_session.lock().await;
        let chrome_session = command_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;

        let mut mutlimodal_els = vec![];
        for command in commands_str.lines().map(|s| s.trim()).collect::<Vec<&str>>() {
            let parsed_command = match parse_single_command(&command.to_string()) {
                Ok(command) => command,
                Err(e) => {
                    tool_log.push(format!("failed to parse command `{}`: {}.", command, e));
                    break
                }
            };
            match parsed_command.execute(chrome_session).await {
                Ok((execute_log, command_multimodal_els)) => {
                    tool_log.extend(execute_log);
                    mutlimodal_els.extend(command_multimodal_els);
                },
                Err(e) => {
                    tool_log.push(format!("failed to execute command `{}`: {}.", command, e));
                    break
                }
            };
        }

        let mut content= vec![];
        content.push(MultimodalElement::new(
            "text".to_string(), tool_log.join("\n")
        )?);
        content.extend(mutlimodal_els);

        let msg = ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::Multimodal(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        });

        Ok((false, vec![msg]))
    }

    fn tool_description(&self) -> ToolDesc {
        let mut supported_commands = vec![
            "open_tab <desktop|mobile> <tab_id>",
            "navigate_to <uri> <tab_id>",
            "screenshot <tab_id>",
            "html <tab_id>",
            "reload <tab_id>",
        ];
        if self.supports_clicks {
            supported_commands.extend(vec![
                "click <x> <y> <tab_id>",
                "insert_text <text> <tab_id>",
            ]);
        }
        let description = format!(
            "One or several commands separated by newline. \
             The <tab_id> is an integer, for example 10, for you to identify the tab later. \
             Supported commands:\n{}", supported_commands.join("\n"));
        ToolDesc {
            name: "chrome".to_string(),
            agentic: true,
            experimental: true,
            description: "A real web browser with graphical interface.".to_string(),
            parameters: vec![ToolParam {
                name: "commands".to_string(),
                param_type: "string".to_string(),
                description,
            }],
            parameters_required: vec!["commands".to_string()],
        }
    }
}

async fn setup_chrome_session(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &IntegrationChrome,
    session_hashmap_key: &String,
) -> Result<Vec<String>, String> {
    let mut setup_log = vec![];

    let session_entry  = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(session_hashmap_key).cloned()
    };

    if let Some(session) = session_entry {
        let mut session_locked = session.lock().await;
        let chrome_session = session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
        if chrome_session.is_connected() {
            return Ok(setup_log)
        } else {
            setup_log.push("Chrome session is disconnected. Trying to reconnect.".to_string());
            gcx.write().await.integration_sessions.remove(session_hashmap_key);
        }
    }

    let window_size = match args.window_size.as_deref() {
        Some([width, height]) => Some((*width, *height)),
        Some([size]) => Some((*size, *size)),
        _ => None,
    };

    let idle_browser_timeout = args.idle_browser_timeout
        .map(|timeout| Duration::from_secs(timeout as u64))
        .unwrap_or(Duration::from_secs(600));

    let browser = if args.chrome_path.clone().unwrap_or_default().starts_with("ws://") {
        let debug_ws_url: String = args.chrome_path.clone().unwrap();
        setup_log.push("Connect to existing web socket.".to_string());
        Browser::connect_with_timeout(debug_ws_url, idle_browser_timeout).map_err(|e| e.to_string())
    } else {
        let path = args.chrome_path.clone().map(PathBuf::from);
        let launch_options = LaunchOptions {
            path,
            window_size,
            idle_browser_timeout,
            headless: args.headless,
            ..Default::default()
        };
        setup_log.push("Start new chrome process.".to_string());
        Browser::new(launch_options).map_err(|e| e.to_string())
    }?;

    // NOTE: we're not register any tabs because they can be used by another chat
    setup_log.push("No opened tabs.".to_string());

    let command_session: Box<dyn IntegrationSession> = Box::new(ChromeSession { browser, tabs: HashMap::new() });
    gcx.write().await.integration_sessions.insert(
        session_hashmap_key.clone(), Arc::new(AMutex::new(command_session))
    );
    Ok(setup_log)
}

async fn navigate_to(instance: &Arc<Tab>, url: &String) -> Result<(), String> {
    instance.navigate_to(url.as_str()).map_err(|e| e.to_string())?;
    instance.wait_until_navigated().map_err(|e| e.to_string())?;
    Ok(())
}

async fn screenshot_jpeg_base64(
    tab: &mut ChromeTab,
    capture_beyond_viewport: bool,
) -> Result<MultimodalElement, String> {
    let jpeg_base64_data = tab.instance.call_method(Page::CaptureScreenshot {
        format: Some(Page::CaptureScreenshotFormatOption::Jpeg),
        clip: None,
        quality: Some(75),
        from_surface: Some(true),
        capture_beyond_viewport: Some(capture_beyond_viewport),
    }).map_err(|e| e.to_string())?.data;

    let mut data = base64::prelude::BASE64_STANDARD
        .decode(jpeg_base64_data).map_err(|e| e.to_string())?;
    let reader = ImageReader::with_format(Cursor::new(data), ImageFormat::Jpeg);
    let mut image = reader.decode().map_err(|e| e.to_string())?;

    let max_dimension = 800.0;
    let scale_factor = max_dimension / std::cmp::max(image.width(), image.height()) as f32;
    if scale_factor < 1.0 {
        // NOTE: the tool operates on resized image well without a special model notification
        let (nwidth, nheight) = (scale_factor * image.width() as f32, scale_factor * image.height() as f32);
        image = image.resize(nwidth as u32, nheight as u32, FilterType::Lanczos3);
        tab.screenshot_scale_factor = scale_factor as f64;
    }

    data = Vec::new();
    image.write_to(&mut Cursor::new(&mut data), ImageFormat::Jpeg).map_err(|e| e.to_string())?;

    MultimodalElement::new("image/jpeg".to_string(), base64::prelude::BASE64_STANDARD.encode(data))
}

async fn inner_html(url: String) -> Result<String, String> {
    let client = Client::builder()
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url.clone()).send().await.map_err(|e| e.to_string())?;
    if response.status().is_success() {
        let html = response.text().await.map_err(|e| e.to_string())?;
        Ok(html)
    } else {
        Err(format!("status: {}", response.status()))
    }
}

async fn click_on_point(tab: &ChromeTab, point: &Point) -> Result<(), String> {
    let mapped_point = Point {
        x: point.x / tab.screenshot_scale_factor,
        y: point.y / tab.screenshot_scale_factor,
    };
    tab.instance.click_point(mapped_point).map_err(|e| e.to_string())?;
    tab.instance.wait_until_navigated().map_err(|e| e.to_string())?;
    Ok(())
}

async fn session_open_tab(
    chrome_session: &mut ChromeSession,
    tab_id: &String,
    device: &DeviceType,
) -> Result<String, String> {
    match chrome_session.tabs.get(tab_id) {
        Some(tab) => {
            Err(format!("Tab is already opened: {}\n", tab.state_string()))
        },
        None => {
            let instance = chrome_session.browser.new_tab().map_err(|e| e.to_string())?;
            match device {
                DeviceType::MOBILE => {
                    instance.call_method(Emulation::SetDeviceMetricsOverride {
                        width: 375,
                        height: 812,
                        device_scale_factor: 0.0,
                        mobile: true,
                        scale: None,
                        screen_width: None,
                        screen_height: None,
                        position_x: None,
                        position_y: None,
                        dont_set_visible_size: None,
                        screen_orientation: None,
                        viewport: None,
                        display_feature: None,
                    }).map_err(|e| e.to_string())?;
                },
                DeviceType::DESKTOP => {
                    instance.call_method(Emulation::ClearDeviceMetricsOverride(None)).map_err(|e| e.to_string())?;
                }
            }
            let tab = ChromeTab::new(instance, device, tab_id);
            chrome_session.tabs.insert(tab.tab_id.clone(), tab.clone());
            Ok(format!("opened a new tab: {}\n", tab.state_string()))
        }
    }
}

async fn session_get_tab_mut<'a>(
    chrome_session: &'a mut ChromeSession,
    tab_id: &String,
) -> Result<&'a mut ChromeTab, String> {
    match chrome_session.tabs.get_mut(tab_id) {
        Some(tab) => Ok(tab),
        None => Err(format!("tab_id {} is not opened", tab_id)),
    }
}

#[derive(Debug)]
enum Command {
    OpenTab(OpenTabArgs),
    NavigateTo(NavigateToArgs),
    Screenshot(ScreenshotArgs),
    Html(HtmlArgs),
    Reload(ReloadArgs),
    Click(ClickArgs),
    InsertText(InsertTextArgs),
}

impl Command {
    pub async fn execute(
        &self,
        chrome_session: &mut ChromeSession,
    ) -> Result<(Vec<String>, Vec<MultimodalElement>), String> {
        let mut tool_log = vec![];
        let mut multimodal_els = vec![];

        match self {
            Command::OpenTab(args) => {
                let log = session_open_tab(chrome_session, &args.tab_id, &args.device).await?;
                tool_log.push(log);
            },
            Command::NavigateTo(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match navigate_to(&tab.instance, &args.uri).await {
                    Ok(_) => format!("navigate_to successful: {}", tab.state_string()),
                    Err(e) => format!("navigate_to `{}` failed: {}. If you're trying to open a local file, add a file:// prefix.", args.uri, e.to_string()),
                };
                tool_log.push(log);
            },
            Command::Screenshot(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match screenshot_jpeg_base64(tab, false).await {
                    Ok(multimodal_el) => {
                        multimodal_els.push(multimodal_el);
                        format!("made a screenshot of {}", tab.state_string())
                    },
                    Err(e) => format!("screenshot failed for {}: {}", tab.state_string(), e.to_string()),
                };
                tool_log.push(log);
            },
            Command::Html(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match inner_html(tab.instance.get_url()).await {
                    Ok(html) => format!("innerHtml of {}:\n\n{}", tab.state_string(), html),
                    Err(e) => format!("can't fetch innerHtml of {}: {}", tab.state_string(), e.to_string()),
                };
                tool_log.push(log);
            },
            Command::Reload(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match tab.instance.reload(false, None) {
                    Ok(_) => format!("reload of {} successful", tab.state_string()),
                    Err(e) => format!("reload of {} failed: {}", tab.state_string(), e.to_string()),
                };
                tool_log.push(log);
            },
            Command::Click(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match click_on_point(&tab, &args.point).await {
                    Ok(_) => format!("clicked on `{} {}` at {}", args.point.x, args.point.y, tab.state_string()),
                    Err(e) => format!("clicked on `{} {}` failed at {}: {}", args.point.x, args.point.y, tab.state_string(), e.to_string()),
                };
                tool_log.push(log);
            },
            Command::InsertText(args) => {
                let tab = session_get_tab_mut(chrome_session, &args.tab_id).await?;
                let log = match tab.instance.type_str(args.text.as_str()) {
                    Ok(_) => format!("insert_text `{}` to {}", args.text, tab.state_string()),
                    Err(e) => format!("insert_text failed to {}: {}", tab.state_string(), e.to_string()),
                };
                tool_log.push(log);
            },
        }

        Ok((tool_log, multimodal_els))
    }
}

#[derive(Debug)]
struct OpenTabArgs {
    device: DeviceType,
    tab_id: String,
}

#[derive(Debug)]
struct NavigateToArgs {
    uri: String,
    tab_id: String,
}

#[derive(Debug)]
struct ScreenshotArgs {
    tab_id: String,
}

#[derive(Debug)]
struct HtmlArgs {
    tab_id: String,
}

#[derive(Debug)]
struct ReloadArgs {
    tab_id: String,
}

#[derive(Debug)]
struct ClickArgs {
    point: Point,
    tab_id: String,
}

#[derive(Debug)]
struct InsertTextArgs {
    text: String,
    tab_id: String,
}

fn parse_single_command(command: &String) -> Result<Command, String> {
    let args = shell_words::split(&command).map_err(|e| e.to_string())?;
    if args.is_empty() {
        return Err("Command is empty".to_string());
    }

    let (command_name, parsed_args) = (args[0].clone(), args[1..].to_vec());

    match command_name.as_str() {
        "open_tab" => {
            if parsed_args.len() < 2 {
                return Err(format!("`open_tab` requires 2 arguments: `<device|mobile>` and `tab_id`. Provided: {:?}", parsed_args));
            }
            let device = match parsed_args[0].as_str() {
                "desktop" => DeviceType::DESKTOP,
                "mobile" => DeviceType::MOBILE,
                _ => return Err(format!("unknown device type: {}. Should be either `desktop` or `mobile`.", parsed_args[0]))
            };
            Ok(Command::OpenTab(OpenTabArgs {
                device,
                tab_id: parsed_args[1].clone(),
            }))
        },
        "navigate_to" => {
            if parsed_args.len() < 2 {
                return Err(format!("`navigate_to` requires 2 arguments: `uri` and `tab_id`. Provided: {:?}", parsed_args));
            }
            Ok(Command::NavigateTo(NavigateToArgs {
                uri: parsed_args[0].clone(),
                tab_id: parsed_args[1].clone(),
            }))
        },
        "screenshot" => {
            if parsed_args.len() < 1 {
                return Err(format!("`screenshot` requires 1 argument: `tab_id`. Provided: {:?}", parsed_args));
            }
            Ok(Command::Screenshot(ScreenshotArgs {
                tab_id: parsed_args[0].clone(),
            }))
        },
        "html" => {
            if parsed_args.len() < 1 {
                return Err(format!("`html` requires 1 argument: `tab_id`. Provided: {:?}", parsed_args));
            }
            Ok(Command::Html(HtmlArgs {
                tab_id: parsed_args[0].clone(),
            }))
        },
        "reload" => {
            if parsed_args.len() < 1 {
                return Err(format!("`reload` requires 1 argument: `tab_id`. Provided: {:?}", parsed_args));
            }
            Ok(Command::Reload(ReloadArgs {
                tab_id: parsed_args[0].clone(),
            }))
        },
        "click" => {
            match parsed_args.as_slice() {
                [x_str, y_str, tab_id] => {
                    let x = x_str.parse::<f64>().map_err(|e| format!("Failed to parse x: {}", e))?;
                    let y = y_str.parse::<f64>().map_err(|e| format!("Failed to parse y: {}", e))?;
                    let point = Point { x, y };
                    Ok(Command::Click(ClickArgs {
                        point,
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments 'x', 'y', 'tab_id'".to_string())
                }
            }
        },
        "insert_text" => {
            match parsed_args.as_slice() {
                [text, tab_id] => {
                    Ok(Command::InsertText(InsertTextArgs {
                        text: text.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments 'text', 'tab_id'".to_string())
                }
            }
        },
        _ => Err(format!("Unknown command: {:?}.", command_name)),
    }
}
