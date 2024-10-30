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
use crate::tools::tools_description::Tool;

use reqwest::Client;
use std::path::PathBuf;
use headless_chrome::{Browser, LaunchOptions, Tab};
use headless_chrome::protocol::cdp::Page;
use serde::{Deserialize, Serialize};


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
}

pub struct ChromeSession {
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

impl ToolChrome {
    pub fn new_from_yaml(v: &serde_yaml::Value) -> Result<Self, String> {
        let integration_chrome = serde_yaml::from_value::<IntegrationChrome>(v.clone()).map_err(|e| {
            let location = e.location().map(|loc| format!(" at line {}, column {}", loc.line(), loc.column())).unwrap_or_default();
            format!("{}{}", e.to_string(), location)
        })?;
        Ok(Self { integration_chrome })
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

        let command = match args.get("command") {
            Some(Value::String(s)) => s,
            Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
            None => return Err("Missing argument `command`".to_string())
        };
        let command = parse_command(command)?;

        let content = interact_with_chrome(
            gcx.clone(),
            &chat_id,
            &self.integration_chrome,
            &command,
        ).await?;

        let msg = ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::Multimodal(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        });

        Ok((false, vec![msg]))
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

    browser.register_missing_tabs();
    let browser_tabs = browser.clone().get_tabs().lock().map_err(|e| e.to_string())?.clone();

    let mut session_tabs = HashMap::new();
    if browser_tabs.is_empty() {
        setup_log.push("No opened tabs.".to_string());
    } else {
        for (tab_id, tab) in browser_tabs.iter().enumerate() {
            setup_log.push(format!("Opened tab {}: {}", tab_id, tab.get_url()));
            session_tabs.insert(tab_id.to_string(), tab.clone());
        }
    }

    let command_session: Box<dyn IntegrationSession> = Box::new(ChromeSession { browser, tabs: session_tabs });
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
pub enum Command {
    // TODO: probably we need connect command
    // if we're tying to operate on non-existing tab (no connection or something like this)
    // we should not auto-open connection again
    NavigateTo(NavigateToArgs),
    Screenshot(ScreenshotArgs),
    Html(HtmlArgs),
    Reload(ReloadArgs),
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
        }

        Ok((tool_log, multimodal_els))
    }
}

#[derive(Debug)]
pub struct NavigateToArgs {
    pub uri: String,
    pub tab_id: String,
}

#[derive(Debug)]
pub struct ScreenshotArgs {
    pub tab_id: String,
}

#[derive(Debug)]
pub struct HtmlArgs {
    pub tab_id: String,
}

#[derive(Debug)]
pub struct ReloadArgs {
    pub tab_id: String,
}

fn parse_command(command: &String) -> Result<Command, String> {
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

    let (execute_log, multimodal_els) = command.execute(chrome_session).await?;

    let mut tool_content = multimodal_els.clone();
    let tool_log = setup_log.iter().chain(execute_log.iter()).map(|s| s.clone()).collect::<Vec<_>>();
    tool_content.push(MultimodalElement::new(
        "text".to_string(), tool_log.join("\n")
    )?);

    Ok(tool_content)
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
