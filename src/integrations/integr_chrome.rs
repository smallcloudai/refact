use std::any::Any;
use std::sync::Arc;
use std::collections::HashMap;
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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::integrations::integr::{json_schema, Integration};


#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, Default)]
pub struct IntegrationChrome {
    #[schemars(description = "Path to the Chrome binary or WebSocket URL for remote debugging.")]
    pub chrome_path: Option<String>,
    #[schemars(description = "Window width for the Chrome browser.")]
    pub window_width: Option<u32>,
    #[schemars(description = "Window height for the Chrome browser.")]
    pub window_height: Option<u32>,
    #[schemars(description = "Idle timeout for the Chrome browser in seconds.")]
    pub idle_browser_timeout: Option<u32>,
    #[serde(default = "default_headless")]
    pub headless: bool,
}

#[derive(Default)]
pub struct ToolChrome {
    pub integration_chrome: IntegrationChrome,
}

fn default_headless() -> bool { true }

pub struct ToolChrome {
    integration_chrome: IntegrationChrome,
    supports_clicks: bool,
}

struct ChromeSession {
    browser: Browser,
    tabs: HashMap<String, Arc<Tab>>,
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
}

impl Integration for ToolChrome {
    fn name(&self) -> String {
        "chrome".to_string()
    }

    fn update_from_json(&mut self, value: &Value) -> Result<(), String> {
        let integration_github = serde_json::from_value::<IntegrationChrome>(value.clone())
            .map_err(|e|e.to_string())?;
        self.integration_chrome = integration_github;
        Ok(())
    }

    fn from_yaml_validate_to_json(&self, value: &serde_yaml::Value) -> Result<Value, String> {
        let integration_github = serde_yaml::from_value::<IntegrationChrome>(value.clone()).map_err(|e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("{}{}", e.to_string(), location)
        })?;
        serde_json::to_value(&integration_github).map_err(|e| e.to_string())
    }

    fn to_tool(&self) -> Box<dyn Tool + Send> {
        Box::new(ToolChrome {integration_chrome: self.integration_chrome.clone()}) as Box<dyn Tool + Send>
    }

    fn to_json(&self) -> Result<Value, String> {
        serde_json::to_value(&self.integration_chrome).map_err(|e| e.to_string())
    }

    fn to_schema_json(&self) -> Value {
        json_schema::<IntegrationChrome>().unwrap()
    }

    fn default_value(&self) -> String { DEFAULT_CHROME_INTEGRATION_YAML.to_string() }
    fn icon_link(&self) -> String { "https://cdn-icons-png.flaticon.com/512/732/732205.png".to_string() }
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

        let mut content = vec![];
        for command in commands_str.lines().map(|s| s.trim()).collect::<Vec<&str>>() {
            let parsed_command = match parse_single_command(&command.to_string()) {
                Ok(command) => command,
                Err(e) => {
                    content.push(MultimodalElement::new(
                        "text".to_string(),
                        format!("Failed to parse command: {}. Error: {}.", command, e)
                    )?);
                    break
                }
            };
            match interact_with_chrome(
                gcx.clone(),
                &chat_id,
                &self.integration_chrome,
                &parsed_command,
            ).await {
                Ok(command_content) => {
                    content.extend(command_content);
                },
                Err(e) => {
                    content.push(MultimodalElement::new(
                        "text".to_string(),
                        format!("Failed to execute command: {}. Error: {}.", command, e)
                    )?);
                    break
                }
            };
        }

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
        let mut commands_desc = r#"One or several commands separated by newline. The <tab_id> is an integer, for example 10, for you to identify the tab later. Supported commands:
navigate_to <uri> <tab_id>
screenshot <tab_id>
html <tab_id>
reload <tab_id>
device <desktop|mobile> <tab_id>"#.to_string();
        if self.supports_clicks {
            commands_desc = format!("{}\nclick <x> <y> <tab_id>\ninsert_text <text> <tab_id>\n", commands_desc);
        }
        ToolDesc {
            name: "chrome".to_string(),
            agentic: true,
            experimental: true,
            description: "A real web browser with graphical interface.".to_string(),
            parameters: vec![ToolParam {
                name: "commands".to_string(),
                param_type: "string".to_string(),
                description: commands_desc,
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
    if !is_chrome_session_active(&session_hashmap_key, gcx.clone()).await {
        let mut is_connection = false;
        if let Some(chrome_path) = args.chrome_path.clone() {
            is_connection = chrome_path.starts_with("ws://");
        }

        let window_size = if args.window_width.is_some() && args.window_height.is_some() {
            Some((args.window_width.unwrap(), args.window_height.unwrap()))
        } else if args.window_width.is_some() {
            Some((args.window_width.unwrap(), args.window_width.unwrap()))
        } else {
            None
        };

        let mut idle_browser_timeout = Duration::from_secs(600);
        if let Some(timeout) = args.idle_browser_timeout.clone() {
            idle_browser_timeout = Duration::from_secs(timeout as u64);
        }
    }

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

async fn navigate_to(tab: &Arc<Tab>, url: &String) -> Result<String, String> {
    tab.navigate_to(url.as_str()).map_err(|e| e.to_string())?;
    tab.wait_until_navigated().map_err(|e| e.to_string())?;
    Ok(format!("Chrome tab navigated to {}", tab.get_url()))
}

async fn click_on_point(tab: &Arc<Tab>, point: &Point) -> Result<String, String> {
    tab.click_point(point.clone()).map_err(|e| e.to_string())?;
    tab.wait_until_navigated().map_err(|e| e.to_string())?;
    Ok(format!("clicked on `{} {}`", point.x, point.y))
}

async fn insert_text(tab: &Arc<Tab>, text: &String) -> Result<String, String> {
    tab.type_str(text.as_str()).map_err(|e| e.to_string())?;
    Ok(format!("inserted text `{}`", text.clone()))
}

async fn session_open_tab(
    chrome_session: &mut ChromeSession,
    tab_name: &String,
) -> Result<(Arc<Tab>, String), String> {
    match chrome_session.tabs.get(tab_name) {
        Some(tab) => {
            Ok((tab.clone(), format!("Using opened tab {}\n", tab_name.clone())))
        },
        None => {
            let tab = chrome_session.browser.new_tab().map_err(|e| e.to_string())?;
            chrome_session.tabs.insert(tab_name.clone(), tab.clone());
            Ok((tab, format!("Opened new tab {}\n", tab_name.clone())))
        }
    }
}

#[derive(Debug)]
enum Command {
    // TODO: probably we need connect command
    // if we're tying to operate on non-existing tab (no connection or something like this)
    // we should not auto-open connection again
    NavigateTo(NavigateToArgs),
    Screenshot(ScreenshotArgs),
    Html(HtmlArgs),
    Reload(ReloadArgs),
    Device(DeviceArgs),
    Click(ClickArgs),
    InsertText(InsertTextArgs),
}

impl Command {
    pub async fn execute(
        &self,
        chrome_session: &mut ChromeSession
    ) -> Result<(Vec<String>, Vec<MultimodalElement>), String> {
        let mut tool_log = vec![];
        let mut multimodal_els = vec![];

        match self {
            Command::NavigateTo(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                let content = navigate_to(&tab, &args.uri).await.map_err(
                    |e| format!("Can't navigate_to `{}` on tab `{}`: {}. If you're trying to open a local file, add a file:// prefix.", args.uri, args.tab_id, e)
                )?;
                tool_log.push(content);
            },
            Command::Screenshot(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                let screenshot = screenshot_jpeg_base64(&tab, false).await?;
                tool_log.push(format!("Made a screenshot of {}", tab.get_url()));
                multimodal_els.push(screenshot);
            },
            Command::Html(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                let client = Client::builder()
                    .build()
                    .map_err(|e| e.to_string())?;
                let url = tab.get_url();
                let response = client.get(url.clone()).send().await.map_err(|e| e.to_string())?;
                if !response.status().is_success() {
                    tool_log.push(format!("Unable to fetch url: {}; status: {}", url, response.status()));
                } else {
                    tool_log.push(response.text().await.map_err(|e| e.to_string())?);
                }
            },
            Command::Reload(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                tab.reload(false, None).map_err(|e| e.to_string())?;
                tool_log.push(format!("Page `{}` on tab `{}` reloaded", tab.get_url(), args.tab_id));
            },
            Command::Device(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                match args.device {
                    DeviceType::MOBILE => {
                        tab.call_method(Emulation::SetDeviceMetricsOverride {
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
                        tool_log.push(format!("Tab `{}` set to mobile view", args.tab_id));
                    },
                    DeviceType::DESKTOP => {
                        tab.call_method(Emulation::ClearDeviceMetricsOverride(None)).map_err(|e| e.to_string())?;
                        tool_log.push(format!("Tab `{}` set to desktop view", args.tab_id));
                    }
                }
            },
            Command::Click(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                let content = click_on_point(&tab, &args.point).await?;
                tool_log.push(content);
            },
            Command::InsertText(args) => {
                let (tab, open_tab_log) = session_open_tab(chrome_session, &args.tab_id).await?;
                tool_log.push(open_tab_log);
                let content = insert_text(&tab, &args.text).await?;
                tool_log.push(content);
            },
        }

        Ok((tool_log, multimodal_els))
    }
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


#[derive(Debug)]
enum DeviceType {
    DESKTOP,
    MOBILE,
}

#[derive(Debug)]
struct DeviceArgs {
    device: DeviceType,
    tab_id: String,
}

fn parse_single_command(command: &String) -> Result<Command, String> {
    let args = shell_words::split(&command).map_err(|e| e.to_string())?;
    if args.is_empty() {
        return Err("Command is empty".to_string());
    }

    let (command_name, parsed_args) = (args[0].clone(), args[1..].to_vec());

    match command_name.as_str() {
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
        "device" => {
            if parsed_args.len() < 2 {
                return Err(format!("`device` requires 2 arguments: `desktop|mobile` and `tab_id`. Provided: {:?}", parsed_args));
            }
            Ok(Command::Device(DeviceArgs {
                device: match parsed_args[0].as_str() {
                    "desktop" => DeviceType::DESKTOP,
                    "mobile" => DeviceType::MOBILE,
                    _ => return Err(format!("Unknown device type: {}. Should be either `desktop` or `mobile`.", parsed_args[0]))
                },
                tab_id: parsed_args[1].clone(),
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

async fn interact_with_chrome(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &String,
    integration_chrome: &IntegrationChrome,
    command: &Command,
) -> Result<Vec<MultimodalElement>, String> {
    let session_hashmap_key = get_session_hashmap_key("chrome", &chat_id);
    let setup_log = setup_chrome_session(gcx.clone(), &integration_chrome, &session_hashmap_key).await?;

    let command_session = {
        let gcx_locked = gcx.read().await;
        gcx_locked.integration_sessions.get(&session_hashmap_key)
            .ok_or(format!("Error getting chrome session for chat: {}", chat_id))?
            .clone()
    };
    let mut command_session_locked = command_session.lock().await;
    let chrome_session = command_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;

    let (execute_log, mut multimodal_els) = command.execute(chrome_session).await?;

    let tool_log = setup_log.iter().chain(execute_log.iter()).map(|s| s.clone()).collect::<Vec<_>>();
    multimodal_els.push(MultimodalElement::new(
        "text".to_string(), tool_log.join("\n")
    )?);

    Ok(multimodal_els)
}

async fn screenshot_jpeg_base64(tab: &Arc<Tab>, capture_beyond_viewport: bool) -> Result<MultimodalElement, String> {
    let jpeg_data = tab.call_method(Page::CaptureScreenshot {
        format: Some(Page::CaptureScreenshotFormatOption::Jpeg),
        clip: None,
        quality: Some(75),
        from_surface: Some(true),
        capture_beyond_viewport: Some(capture_beyond_viewport),
    }).map_err(|e| e.to_string())?.data;

    MultimodalElement::new("image/jpeg".to_string(), jpeg_data)
}

const DEFAULT_CHROME_INTEGRATION_YAML: &str = r#"
# Chrome integration

# This can be path to your chrome binary. You can install with "npx @puppeteer/browsers install chrome@stable", read
# more here https://developer.chrome.com/blog/chrome-for-testing/?utm_source=Fibery&utm_medium=iframely
#chrome_path: "/Users/me/my_path/chrome/mac_arm-130.0.6723.69/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
# Or you can give it ws:// path, read more here https://developer.chrome.com/docs/devtools/remote-debugging/local-server/
# In that case start chrome with --remote-debugging-port
# chrome_path: "ws://127.0.0.1:6006/"
# window_width: 1024
# window_height: 768
# idle_browser_timeout: 600
"#;
