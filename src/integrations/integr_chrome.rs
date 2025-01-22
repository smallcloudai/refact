use std::any::Any;
use std::sync::{Arc, Mutex};
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
use crate::postprocessing::pp_command_output::{CmdlineOutputFilter, output_mini_postprocessing};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam};
use crate::integrations::integr_abstract::{IntegrationTrait, IntegrationCommon, IntegrationConfirmation};
use crate::integrations::docker::docker_container_manager::get_container_name;

use tokio::time::sleep;
use chrono::DateTime;
use std::path::PathBuf;
use headless_chrome::{Browser, Element, LaunchOptions, Tab as HeadlessTab};
use headless_chrome::browser::tab::point::Point;
use headless_chrome::browser::tab::ModifierKey;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::protocol::cdp::Emulation;
use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::protocol::cdp::DOM::Enable as DOMEnable;
use headless_chrome::protocol::cdp::CSS::Enable as CSSEnable;
use serde::{Deserialize, Serialize};

use base64::Engine;
use std::io::Cursor;
use headless_chrome::protocol::cdp::Runtime::RemoteObject;
use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};


#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SettingsChrome {
    pub chrome_path: String,
    #[serde(default)]
    pub idle_browser_timeout: String,
    #[serde(default)]
    pub headless: String,
    // desktop
    #[serde(default)]
    pub window_width: String,
    #[serde(default)]
    pub window_height: String,
    #[serde(default)]
    pub scale_factor: String,
    #[serde(default)]
    // mobile
    pub mobile_window_width: String,
    #[serde(default)]
    pub mobile_window_height: String,
    #[serde(default)]
    pub mobile_scale_factor: String,
    // tablet
    #[serde(default)]
    pub tablet_window_width: String,
    #[serde(default)]
    pub tablet_window_height: String,
    #[serde(default)]
    pub tablet_scale_factor: String,
}

#[derive(Default)]
pub struct ToolChrome {
    pub common: IntegrationCommon,
    pub settings_chrome: SettingsChrome,
    pub supports_clicks: bool,
    pub config_path: String,
}

#[derive(Clone, Debug)]
enum DeviceType {
    DESKTOP,
    MOBILE,
    TABLET,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeviceType::DESKTOP => write!(f, "desktop"),
            DeviceType::MOBILE => write!(f, "mobile"),
            DeviceType::TABLET => write!(f, "tablet"),
        }
    }
}

const MAX_CACHED_LOG_LINES: usize = 1000;

#[derive(Clone)]
pub struct ChromeTab {
    headless_tab: Arc<HeadlessTab>,
    device: DeviceType,
    tab_id: String,
    screenshot_scale_factor: f64,
    tab_log: Arc<Mutex<Vec<String>>>,
}

impl ChromeTab {
    fn new(headless_tab: Arc<HeadlessTab>, device: &DeviceType, tab_id: &String) -> Self {
        Self {
            headless_tab,
            device: device.clone(),
            tab_id: tab_id.clone(),
            screenshot_scale_factor: 1.0,
            tab_log: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn state_string(&self) -> String {
        format!("tab_id `{}` device `{}` uri `{}`", self.tab_id.clone(), self.device, self.headless_tab.get_url())
    }
}

struct ChromeSession {
    browser: Browser,
    tabs: HashMap<String, Arc<AMutex<ChromeTab>>>,
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

#[async_trait]
impl IntegrationTrait for ToolChrome {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), String> {
        match serde_json::from_value::<SettingsChrome>(value.clone()) {
            Ok(settings_chrome) => self.settings_chrome = settings_chrome,
            Err(e) => {
                tracing::error!("Failed to apply settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        match serde_json::from_value::<IntegrationCommon>(value.clone()) {
            Ok(x) => self.common = x,
            Err(e) => {
                tracing::error!("Failed to apply common settings: {}\n{:?}", e, value);
                return Err(e.to_string());
            }
        }
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_chrome).unwrap()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolChrome {
            common: self.common.clone(),
            settings_chrome: self.settings_chrome.clone(),
            supports_clicks: false,
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str
    {
        CHROME_INTEGRATION_SCHEMA
    }
}

#[async_trait]
impl Tool for ToolChrome {
    fn as_any(&self) -> &dyn std::any::Any { self }

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
        let mut tool_log = setup_chrome_session(gcx.clone(), &self.settings_chrome, &session_hashmap_key).await?;

        let command_session = {
            let gcx_locked = gcx.read().await;
            gcx_locked.integration_sessions.get(&session_hashmap_key)
                .ok_or(format!("Error getting chrome session for chat: {}", chat_id))?
                .clone()
        };

        let mut mutlimodal_els = vec![];
        for command in commands_str.lines().map(|s| s.trim()).collect::<Vec<&str>>() {
            let parsed_command = match parse_single_command(&command.to_string()) {
                Ok(command) => command,
                Err(e) => {
                    tool_log.push(format!("Failed to parse command `{}`: {}.", command, e));
                    break
                }
            };
            match chrome_command_exec(&parsed_command, command_session.clone(), &self.settings_chrome, gcx.clone(), &chat_id).await {
                Ok((execute_log, command_multimodal_els)) => {
                    tool_log.extend(execute_log);
                    mutlimodal_els.extend(command_multimodal_els);
                },
                Err(e) => {
                    tool_log.push(format!("Failed to execute command `{}`: {}.", command, e));
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
            "open_tab <tab_id> <desktop|mobile|tablet>",
            "navigate_to <tab_id> <uri>",
            "scroll_to <tab_id> <element_selector>",
            "screenshot <tab_id>",
            "html <tab_id> <element_selector>",
            "reload <tab_id>",
            "press_key <tab_id> <KeyName> [<Alt|Ctrl|Meta|Shift>,...]",
            "type_text_at <tab_id> <text>",
            "tab_log <tab_id>",
            "eval <tab_id> <expression>",
            "styles <tab_id> <element_selector> <property_filter>",
            "wait_for <tab_id> <1-5>",
            "click_at_element <tab_id> <element_selector>",
        ];
        if self.supports_clicks {
            supported_commands.extend(vec![
                "click_at_point <tab_id> <x> <y>",
            ]);
        }
        let description = format!(
            "One or several commands separated by newline. \
             The <tab_id> is an integer, for example 10, for you to identify the tab later. \
             Most of web pages are dynamic. If you see that it's still loading try again with wait_for command. \
             Supported commands:\n{}", supported_commands.join("\n"));
        ToolDesc {
            name: "chrome".to_string(),
            agentic: true,
            experimental: false,
            description: "A real web browser with graphical interface.".to_string(),
            parameters: vec![ToolParam {
                name: "commands".to_string(),
                param_type: "string".to_string(),
                description,
            }],
            parameters_required: vec!["commands".to_string()],
        }
    }


    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

async fn setup_chrome_session(
    gcx: Arc<ARwLock<GlobalContext>>,
    args: &SettingsChrome,
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

    let window_size = match (args.window_width.parse::<u32>(), args.window_height.parse::<u32>()) {
        (Ok(width), Ok(height)) => Some((width, height)),
        _ => None,
    };

    let idle_browser_timeout = args.idle_browser_timeout
        .parse::<u64>()
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600));

    let browser = if args.chrome_path.clone().starts_with("ws://") {
        let debug_ws_url: String = args.chrome_path.clone();
        setup_log.push("Connect to existing web socket.".to_string());
        Browser::connect_with_timeout(debug_ws_url, idle_browser_timeout).map_err(|e| e.to_string())
    } else if let Some (container_address) = args.chrome_path.strip_prefix("container://") {
        setup_log.push("Connect to chrome from container.".to_string());
        let response = reqwest::get(&format!("http://{container_address}/json")).await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Response from {} resulted in status code: {}", args.chrome_path, response.status().as_u16()));
        }
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let ws_url_returned = json[0]["webSocketDebuggerUrl"].as_str()
            .ok_or_else(|| "webSocketDebuggerUrl not found in the response JSON".to_string())?;
        setup_log.push("Extracted webSocketDebuggerUrl from HTTP response.".to_string());

        let mut ws_url_parts: Vec<&str> = ws_url_returned.split('/').collect();
        if ws_url_parts.len() > 2 {
            ws_url_parts[2] = container_address;
        }
        let ws_url = ws_url_parts.join("/");
        Browser::connect_with_timeout(ws_url, idle_browser_timeout).map_err(|e| e.to_string())
    } else {
        let mut path: Option<PathBuf> = None;
        if !args.chrome_path.is_empty() {
            path = Some(PathBuf::from(args.chrome_path.clone()));
        }
        let launch_options = LaunchOptions {
            path,
            window_size,
            idle_browser_timeout,
            headless: args.headless.parse::<bool>().unwrap_or(true),
            ..Default::default()
        };

        setup_log.push("Started new chrome process.".to_string());
        Browser::new(launch_options).map_err(|e| e.to_string())
    }?;

    // NOTE: we're not register any tabs because they can be used by another chat
    setup_log.push("No opened tabs at this moment.".to_string());

    let command_session: Box<dyn IntegrationSession> = Box::new(ChromeSession { browser, tabs: HashMap::new() });
    gcx.write().await.integration_sessions.insert(
        session_hashmap_key.clone(), Arc::new(AMutex::new(command_session))
    );
    Ok(setup_log)
}

async fn screenshot_jpeg_base64(
    tab: Arc<AMutex<ChromeTab>>,
    capture_beyond_viewport: bool,
) -> Result<MultimodalElement, String> {
    let jpeg_base64_data = {
        let tab_lock = tab.lock().await;
        tab_lock.headless_tab.call_method(Page::CaptureScreenshot {
            format: Some(Page::CaptureScreenshotFormatOption::Jpeg),
            clip: None,
            quality: Some(75),
            from_surface: Some(true),
            capture_beyond_viewport: Some(capture_beyond_viewport),
        }).map_err(|e| e.to_string())?.data
    };

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
        // NOTE: we should store screenshot_scale_factor for every resized screenshot, not for a tab!
        let mut tab_lock = tab.lock().await;
        tab_lock.screenshot_scale_factor = scale_factor as f64;
    }

    data = Vec::new();
    image.write_to(&mut Cursor::new(&mut data), ImageFormat::Jpeg).map_err(|e| e.to_string())?;

    MultimodalElement::new("image/jpeg".to_string(), base64::prelude::BASE64_STANDARD.encode(data))
}

fn get_inner_html(
    element: &Element,
) -> Result<String, String> {
    let func = r"
    function() {
        function wrap_html(text, depth) {
            return '  '.repeat(depth) + text + '\n';
        }

        function budget_html(el, max_depth, symbols_budget) {
            let innerHtml = '';
            let elements = [el]
            for (let depth = 0; depth < max_depth; depth++) {
                let expanded_html = '';
                let expanded_elements = [];
                elements.forEach(el => {
                    if (typeof el === 'string') {
                        expanded_html += el;
                        expanded_elements.push(el);
                    } else {
                        if (el.innerHTML.length > 0) {
                            let tagHtml = el.outerHTML.split(el.innerHTML);
                            const tag_open = wrap_html(tagHtml[0], depth);
                            expanded_html += tag_open;
                            expanded_elements.push(tag_open);
                            const children = Array.from(el.children);
                            if (children.length > 0) {
                                expanded_html += wrap_html('...', depth + 1)
                                Array.from(el.children).forEach(child => {
                                    expanded_elements.push(child);
                                });
                            } else if (el.innerText.length > 0) {
                                const tag_text = wrap_html(el.innerText, depth + 1);
                                expanded_html += tag_text;
                                expanded_elements.push(tag_text);
                            }
                            if (tagHtml.length > 1) {
                                const tag_close = wrap_html(tagHtml[1], depth);
                                expanded_html += tag_close
                                expanded_elements.push(tag_close);
                            }
                        } else {
                            const tag = wrap_html(el.outerHTML, depth);
                            expanded_html += tag;
                            expanded_elements.push(tag);
                        }
                    }
                });
                if (expanded_html.length > symbols_budget) {
                    break;
                }
                if (expanded_html.length === innerHtml.length) {
                    break;
                }
                innerHtml = expanded_html;
                elements = expanded_elements;
            }
            return innerHtml;
        }
        return budget_html(this, 100, 3000);
    }";
    let result = element.call_js_fn(func, vec![], false).map_err(|e| e.to_string())?;
    Ok(result.value.unwrap().to_string())
}

fn format_remote_object(
    remote_object: &RemoteObject,
) -> String {
    let mut result = vec![];
    if let Some(subtype) = remote_object.subtype.clone() {
        result.push(format!("subtype {:?}", subtype));
    }
    if let Some(class_name) = remote_object.class_name.clone() {
        result.push(format!("class_name {:?}", class_name));
    }
    if let Some(value) = remote_object.value.clone() {
        result.push(format!("value {:?}", value));
    }
    if let Some(unserializable_value) = remote_object.unserializable_value.clone() {
        result.push(format!("unserializable_value {:?}", unserializable_value));
    }
    if let Some(description) = remote_object.description.clone() {
        result.push(format!("description {:?}", description));
    }
    if let Some(preview) = remote_object.preview.clone() {
        result.push(format!("preview {:?}", preview));
    }
    if let Some(custom_preview) = remote_object.custom_preview.clone() {
        result.push(format!("custom_preview {:?}", custom_preview));
    }
    format!("result: {}", result.join(", "))
}

fn set_device_metrics_method(
    width: u32,
    height: u32,
    device_scale_factor: f64,
    mobile: bool,
) -> Emulation::SetDeviceMetricsOverride {
    Emulation::SetDeviceMetricsOverride {
        width, height, device_scale_factor, mobile,
        scale: None, screen_width: None, screen_height: None,
        position_x: None, position_y: None, dont_set_visible_size: None,
        screen_orientation: None, viewport: None, display_feature: None,
    }
}

async fn session_open_tab(
    chrome_session: &mut ChromeSession,
    tab_id: &String,
    device: &DeviceType,
    settings_chrome: &SettingsChrome,
) -> Result<String, String> {
    match chrome_session.tabs.get(tab_id) {
        Some(tab) => {
            let tab_lock = tab.lock().await;
            Err(format!("Tab is already opened: {}\n", tab_lock.state_string()))
        },
        None => {
            let headless_tab = chrome_session.browser.new_tab().map_err(|e| e.to_string())?;
            let method = match device {
                DeviceType::DESKTOP => {
                    let (width, height) = match (settings_chrome.window_width.parse::<u32>(), settings_chrome.window_height.parse::<u32>()) {
                        (Ok(width), Ok(height)) => (width, height),
                        _ => (800, 600),
                    };
                    let scale_factor = match settings_chrome.scale_factor.parse::<f64>() {
                        Ok(scale_factor) => scale_factor,
                        _ => 0.0,
                    };
                    set_device_metrics_method(width, height, scale_factor, false)
                },
                DeviceType::MOBILE => {
                    let (width, height) = match (settings_chrome.mobile_window_width.parse::<u32>(), settings_chrome.mobile_window_height.parse::<u32>()) {
                        (Ok(width), Ok(height)) => (width, height),
                        _ => (400, 800),
                    };
                    let scale_factor = match settings_chrome.mobile_scale_factor.parse::<f64>() {
                        Ok(scale_factor) => scale_factor,
                        _ => 0.0,
                    };
                    set_device_metrics_method(width, height, scale_factor, true)
                },
                DeviceType::TABLET => {
                    let (width, height) = match (settings_chrome.tablet_window_width.parse::<u32>(), settings_chrome.tablet_window_height.parse::<u32>()) {
                        (Ok(width), Ok(height)) => (width, height),
                        _ => (600, 800),
                    };
                    let scale_factor = match settings_chrome.tablet_scale_factor.parse::<f64>() {
                        Ok(scale_factor) => scale_factor,
                        _ => 0.0,
                    };
                    set_device_metrics_method(width, height, scale_factor, true)
                },
            };
            headless_tab.call_method(method).map_err(|e| e.to_string())?;
            let tab = Arc::new(AMutex::new(ChromeTab::new(headless_tab, device, tab_id)));
            let tab_lock = tab.lock().await;
            let tab_log = Arc::clone(&tab_lock.tab_log);
            tab_lock.headless_tab.enable_log().map_err(|e| e.to_string())?;
            tab_lock.headless_tab.add_event_listener(Arc::new(move |event: &Event| {
                if let Event::LogEntryAdded(e) = event {
                    let formatted_ts = {
                        let dt = DateTime::from_timestamp(e.params.entry.timestamp as i64, 0).unwrap();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    };
                    let mut tab_log_lock = tab_log.lock().unwrap();
                    tab_log_lock.push(format!("{} [{:?}]: {}", formatted_ts, e.params.entry.level, e.params.entry.text));
                    if tab_log_lock.len() > MAX_CACHED_LOG_LINES {
                        tab_log_lock.remove(0);
                    }
                }
            })).map_err(|e| e.to_string())?;
            chrome_session.tabs.insert(tab_id.clone(), tab.clone());
            Ok(format!("Opened a new tab: {}\n", tab_lock.state_string()))
        }
    }
}

async fn session_get_tab_arc(
    chrome_session: &ChromeSession,
    tab_id: &String,
) -> Result<Arc<AMutex<ChromeTab>>, String> {
    match chrome_session.tabs.get(tab_id) {
        Some(tab) => Ok(tab.clone()),
        None => Err(format!("tab_id {} is not opened", tab_id)),
    }
}

#[derive(Debug)]
enum Command {
    OpenTab(OpenTabArgs),
    NavigateTo(NavigateToArgs),
    ScrollTo(TabElementArgs),
    Screenshot(TabArgs),
    Html(TabElementArgs),
    Reload(TabArgs),
    ClickAtPoint(ClickAtPointArgs),
    ClickAtElement(TabElementArgs),
    TypeTextAt(TypeTextAtArgs),
    PressKey(PressKeyArgs),
    TabLog(TabArgs),
    Eval(EvalArgs),
    Styles(StylesArgs),
    WaitFor(WaitForArgs),
}

async fn chrome_command_exec(
    cmd: &Command,
    chrome_session: Arc<AMutex<Box<dyn IntegrationSession>>>,
    settings_chrome: &SettingsChrome,
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_id: &str,
) -> Result<(Vec<String>, Vec<MultimodalElement>), String> {
    let mut tool_log = vec![];
    let mut multimodal_els = vec![];

    match cmd {
        Command::OpenTab(args) => {
            let log = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_open_tab(chrome_session, &args.tab_id, &args.device, &settings_chrome).await?
            };
            tool_log.push(log);
        },
        Command::NavigateTo(args) => {
            let tab: Arc<AMutex<ChromeTab>> = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let mut url = args.uri.clone();
            if settings_chrome.chrome_path.starts_with("container://") {
                let is_inside_container = gcx.read().await.cmdline.inside_container;
                if is_inside_container {
                    url = replace_host_with_container_if_needed(&url, chat_id);
                }
            }
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    tab_lock.headless_tab.navigate_to(&url).map_err(|e| e.to_string())?;
                    tab_lock.headless_tab.wait_until_navigated().map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                } {
                    Ok(_) => {
                        format!("navigate_to successful: {}", tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("navigate_to `{}` failed: {}. If you're trying to open a local file, add a file:// prefix.", args.uri, e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::ScrollTo(args) => {
            let tab: Arc<AMutex<ChromeTab>> = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    let element = tab_lock.headless_tab.find_element(&args.selector).map_err(|e| e.to_string())?;
                    element.scroll_into_view().map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                } {
                    Ok(_) => {
                        format!("scroll_to `{}` successful: {}.", args.selector, tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("scroll_to `{}` failed: {}.", args.selector, e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::Screenshot(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                // NOTE: this operation is not atomic, unfortunately
                match screenshot_jpeg_base64(tab.clone(), false).await {
                    Ok(multimodal_el) => {
                        multimodal_els.push(multimodal_el);
                        let tab_lock = tab.lock().await;
                        format!("Made a screenshot of {}", tab_lock.state_string())
                    },
                    Err(e) => {
                        let tab_lock = tab.lock().await;
                        format!("Screenshot failed for {}: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::Html(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    let elements = tab_lock.headless_tab.find_elements(&args.selector).map_err(|e| e.to_string())?;
                    if elements.len() == 0 {
                        Err("No elements found".to_string())
                    } else {
                        let mut elements_log = vec![];
                        let first_element = elements.first().unwrap();
                        elements_log.push(get_inner_html(first_element)?);
                        if elements.len() > 2 {
                            elements_log.push(format!("\n\nShown html for first of {} elements", elements.len()));
                        }
                        Ok::<String, String>(elements_log.join("\n"))
                    }
                } {
                    Ok(html) => {
                        format!("html of `{}`:\n\n{}", args.selector, html)
                    },
                    Err(e) => {
                        format!("can't fetch html of `{}`: {}", args.selector, e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::Reload(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                let chrome_tab = tab_lock.headless_tab.clone();
                match chrome_tab.reload(false, None) {
                    Ok(_) => {
                        format!("reload of {} successful", tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("reload of {} failed: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::ClickAtPoint(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    let mapped_point = Point {
                        x: args.point.x / tab_lock.screenshot_scale_factor,
                        y: args.point.y / tab_lock.screenshot_scale_factor,
                    };
                    tab_lock.headless_tab.click_point(mapped_point).map_err(|e| e.to_string())?;
                    tab_lock.headless_tab.wait_until_navigated().map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                } {
                    Ok(_) => {
                        format!("clicked `{} {}` at {}", args.point.x, args.point.y, tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("clicked `{} {}` failed at {}: {}", args.point.x, args.point.y, tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::ClickAtElement(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    let element = tab_lock.headless_tab.find_element(&args.selector).map_err(|e| e.to_string())?;
                    element.click().map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                } {
                    Ok(_) => {
                        format!("clicked `{}` at {}", args.selector, tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("click at element `{}` failed at {}: {}", args.selector, tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::TypeTextAt(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match tab_lock.headless_tab.type_str(args.text.as_str()) {
                    Ok(_) => {
                        format!("type `{}` at {}", args.text, tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("type text failed at {}: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::PressKey(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    tab_lock.headless_tab.press_key_with_modifiers(
                        args.key.as_str(), args.key_modifiers.as_deref())
                        .map_err(|e| e.to_string())?;
                    tab_lock.headless_tab.wait_until_navigated().map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                } {
                    Ok(_) => {
                        format!("press_key at {}", tab_lock.state_string())
                    },
                    Err(e) => {
                        format!("press_key failed at {}: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::TabLog(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let tab_log = {
                let tab_lock = tab.lock().await;
                let mut tab_log_lock = tab_lock.tab_log.lock().unwrap();
                let tab_log = tab_log_lock.join("\n");
                tab_log_lock.clear();
                tab_log
            };
            // let filter = CmdlineOutputFilter::default();
            let filter = CmdlineOutputFilter {
                limit_lines: 100,
                limit_chars: 10000,
                valuable_top_or_bottom: "top".to_string(),
                grep: "".to_string(),
                grep_context_lines: 0,
                remove_from_output: "".to_string(),
            };
            let filtered_log = output_mini_postprocessing(&filter, tab_log.as_str());
            tool_log.push(filtered_log.clone());
        },
        Command::Eval(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match tab_lock.headless_tab.evaluate(args.expression.as_str(), false) {
                    Ok(remote_object) => {
                        format_remote_object(&remote_object)
                    },
                    Err(e) => {
                        format!("eval failed at {}: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::Styles(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                match {
                    tab_lock.headless_tab.call_method(DOMEnable(None)).map_err(|e| e.to_string())?;
                    tab_lock.headless_tab.call_method(CSSEnable(None)).map_err(|e| e.to_string())?;
                    let element = tab_lock.headless_tab.find_element(&args.selector).map_err(|e| e.to_string())?;
                    let computed_styles = element.get_computed_styles().map_err(|e| e.to_string())?;
                    let mut styles_filtered = computed_styles.iter()
                        .filter(|s| s.name.contains(args.property_filter.as_str()))
                        .map(|s| format!("{}: {}", s.name, s.value))
                        .collect::<Vec<String>>();
                    let max_lines_output = 30;
                    if styles_filtered.len() > max_lines_output {
                        let skipped_message = format!("Skipped {} properties. Specify filter if you need to see more.", styles_filtered.len() - max_lines_output);
                        styles_filtered = styles_filtered[..max_lines_output].to_vec();
                        styles_filtered.push(skipped_message)
                    }
                    if styles_filtered.is_empty() {
                        styles_filtered.push("No properties for given filter.".to_string());
                    }
                    Ok::<String, String>(styles_filtered.join("\n"))
                } {
                    Ok(styles_str) => {
                        format!("Style properties for element `{}` at {}:\n{}", args.selector, tab_lock.state_string(), styles_str)
                    },
                    Err(e) => {
                        format!("Styles get failed at {}: {}", tab_lock.state_string(), e.to_string())
                    },
                }
            };
            tool_log.push(log);
        },
        Command::WaitFor(args) => {
            let tab = {
                let mut chrome_session_locked = chrome_session.lock().await;
                let chrome_session = chrome_session_locked.as_any_mut().downcast_mut::<ChromeSession>().ok_or("Failed to downcast to ChromeSession")?;
                session_get_tab_arc(chrome_session, &args.tab_id).await?
            };
            let log = {
                let tab_lock = tab.lock().await;
                if args.seconds < 1.0 && args.seconds > 5.0 {
                    return Err(format!("wait_for at {} failed: `seconds` should be integer in interval [1, 5]", tab_lock.state_string()))
                }
                sleep(Duration::from_secs(3)).await;
                format!("wait_for {} seconds at {} successful.", args.seconds, tab_lock.state_string())
            };
            tool_log.push(log);
        },
    }

    Ok((tool_log, multimodal_els))
}

#[derive(Debug)]
struct TabArgs {
    tab_id: String,
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
struct ClickAtPointArgs {
    point: Point,
    tab_id: String,
}

#[derive(Debug)]
struct TypeTextAtArgs {
    text: String,
    tab_id: String,
}

#[derive(Debug)]
struct PressKeyArgs {
    key: String,
    key_modifiers: Option<Vec<ModifierKey>>,
    tab_id: String,
}

#[derive(Debug)]
struct EvalArgs {
    tab_id: String,
    expression: String,
}

#[derive(Debug)]
struct TabElementArgs {
    tab_id: String,
    selector: String,
}

#[derive(Debug)]
struct StylesArgs {
    tab_id: String,
    selector: String,
    property_filter: String,
}

#[derive(Debug)]
struct WaitForArgs {
    tab_id: String,
    seconds: f64,
}

fn parse_single_command(command: &String) -> Result<Command, String> {
    let args = shell_words::split(&command).map_err(|e| e.to_string())?;
    if args.is_empty() {
        return Err("Command is empty".to_string());
    }

    let (command_name, parsed_args) = (args[0].clone(), args[1..].to_vec());

    match command_name.as_str() {
        "open_tab" => {
            match parsed_args.as_slice() {
                [tab_id, device_str] => {
                    let device = match device_str.as_str() {
                        "desktop" => DeviceType::DESKTOP,
                        "mobile" => DeviceType::MOBILE,
                        "tablet" => DeviceType::TABLET,
                        _ => return Err(format!("unknown device type: {}. Should be `desktop`, `mobile` or `tablet`.", parsed_args[0]))
                    };
                    Ok(Command::OpenTab(OpenTabArgs {
                        device: device.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `<device|mobile|tablet>`".to_string())
                }
            }
        },
        "navigate_to" => {
            match parsed_args.as_slice() {
                [tab_id, uri] => {
                    Ok(Command::NavigateTo(NavigateToArgs {
                        uri: uri.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `uri`".to_string())
                }
            }
        },
        "scroll_to" => {
            match parsed_args.as_slice() {
                [tab_id, selector] => {
                    Ok(Command::ScrollTo(TabElementArgs {
                        selector: selector.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `selector`".to_string())
                }
            }
        },
        "screenshot" => {
            match parsed_args.as_slice() {
                [tab_id] => {
                    Ok(Command::Screenshot(TabArgs {
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`".to_string())
                }
            }
        },
        "html" => {
            match parsed_args.as_slice() {
                [tab_id, selector] => {
                    Ok(Command::Html(TabElementArgs {
                        selector: selector.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `selector`".to_string())
                }
            }
        },
        "reload" => {
            match parsed_args.as_slice() {
                [tab_id] => {
                    Ok(Command::Reload(TabArgs {
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`".to_string())
                }
            }
        },
        "click_at_point" => {
            match parsed_args.as_slice() {
                [tab_id, x_str, y_str] => {
                    let x = x_str.parse::<f64>().map_err(|e| format!("Failed to parse x: {}", e))?;
                    let y = y_str.parse::<f64>().map_err(|e| format!("Failed to parse y: {}", e))?;
                    let point = Point { x, y };
                    Ok(Command::ClickAtPoint(ClickAtPointArgs {
                        point,
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `x`, 'y`".to_string())
                }
            }
        },
        "click_at_element" => {
            match parsed_args.as_slice() {
                [tab_id, selector] => {
                    Ok(Command::ClickAtElement(TabElementArgs {
                        selector: selector.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `selector`".to_string())
                }
            }
        },
        "type_text_at" => {
            match parsed_args.as_slice() {
                [tab_id, text] => {
                    Ok(Command::TypeTextAt(TypeTextAtArgs {
                        text: text.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `text`".to_string())
                }
            }
        },
        "press_key" => {
            match parsed_args.as_slice() {
                [tab_id, key] => {
                    Ok(Command::PressKey(PressKeyArgs {
                        key: key.clone(),
                        key_modifiers: None,
                        tab_id: tab_id.clone(),
                    }))
                },
                [tab_id, key, key_modifiers] => {
                    let modifiers: Result<Vec<ModifierKey>, String> = key_modifiers.split(',')
                        .map(|modifier_str| match modifier_str.trim() {
                            "Alt" => Ok(ModifierKey::Alt),
                            "Ctrl" => Ok(ModifierKey::Ctrl),
                            "Meta" => Ok(ModifierKey::Meta),
                            "Shift" => Ok(ModifierKey::Shift),
                            _ => Err(format!("Unknown key modifier: {}", modifier_str)),
                        })
                        .collect();

                    match modifiers {
                        Ok(modifiers) => Ok(Command::PressKey(PressKeyArgs {
                            key: key.clone(),
                            key_modifiers: Some(modifiers),
                            tab_id: tab_id.clone(),
                        })),
                        Err(e) => Err(e),
                    }
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `key`".to_string())
                }
            }
        },
        "tab_log" => {
            match parsed_args.as_slice() {
                [tab_id] => {
                    Ok(Command::TabLog(TabArgs {
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`".to_string())
                }
            }
        },
        "eval" => {
            match parsed_args.as_slice() {
                [tab_id, expression] => {
                    Ok(Command::Eval(EvalArgs {
                        expression: expression.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `expression`.".to_string())
                }
            }
        },
        "styles" => {
            match parsed_args.as_slice() {
                [tab_id, selector, property_filter] => {
                    Ok(Command::Styles(StylesArgs {
                        selector: selector.clone(),
                        tab_id: tab_id.clone(),
                        property_filter: property_filter.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `selector`.".to_string())
                }
            }
        },
        "wait_for" => {
            match parsed_args.as_slice() {
                [tab_id, seconds_str] => {
                    let seconds = seconds_str.parse::<f64>().map_err(|e| format!("Failed to parse seconds: {}", e))?;
                    Ok(Command::WaitFor(WaitForArgs {
                        seconds: seconds.clone(),
                        tab_id: tab_id.clone(),
                    }))
                },
                _ => {
                    Err("Missing one or several arguments `tab_id`, `seconds`.".to_string())
                }
            }
        },
        _ => Err(format!("Unknown command: {:?}.", command_name)),
    }
}

fn replace_host_with_container_if_needed(url: &str, chat_id: &str) -> String {
    if let Ok(mut parsed_url) = url::Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            if host == "127.0.0.1" || host == "0.0.0.0" || host == "localhost" {
                parsed_url.set_host(Some(&get_container_name(chat_id))).unwrap();
                return parsed_url.to_string();
            }
        }
    }
    url.to_string()
}


const CHROME_INTEGRATION_SCHEMA: &str = r#"
fields:
  chrome_path:
    f_type: string_long
    f_desc: "Path to Google Chrome, Chromium or Edge binary. If empty, it searches for binary in your system"
  idle_browser_timeout:
    f_type: string_short
    f_desc: "Idle timeout for the browser in seconds."
    f_extra: true
  headless:
    f_type: string_short
    f_desc: "Run Chrome in headless mode."
    f_default: "true"
    f_extra: true
  window_width:
    f_type: string_short
    f_desc: "Width of the browser window."
    f_extra: true
  window_height:
    f_type: string_short
    f_desc: "Height of the browser window."
    f_extra: true
  scale_factor:
    f_type: string_short
    f_desc: "Scale factor of the browser window."
    f_extra: true
  mobile_window_width:
    f_type: string_short
    f_desc: "Width of the browser window in mobile mode."
    f_extra: true
  mobile_window_height:
    f_type: string_short
    f_desc: "Height of the browser window in mobile mode."
    f_extra: true
  mobile_scale_factor:
    f_type: string_short
    f_desc: "Scale factor of the browser window in mobile mode."
    f_extra: true
  tablet_window_width:
    f_type: string_short
    f_desc: "Width of the browser window in tablet mode."
    f_extra: true
  tablet_window_height:
    f_type: string_short
    f_desc: "Height of the browser window in tablet mode."
    f_extra: true
  tablet_scale_factor:
    f_type: string_short
    f_desc: "Scale factor of the browser window in tablet mode."
    f_extra: true
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  not_applicable: true
  ask_user_default: []
  deny_default: []
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The chrome tool should be visible now. To test the tool, navigate to a website like https://example.com/ take a screenshot, and express happiness if it works. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
  - sl_label: "Help me install Chrome for Testing"
    sl_chat:
      - role: "user"
        content: |
          🔧 Help the user to install Chrome for Testing using npm, once that is done rewrite the current config file %CURRENT_CONFIG% to use chrome_path to use it.
docker:
  filter_label: ""
  filter_image: "standalone-chrome"
  new_container_default:
    image: "selenium/standalone-chrome:latest"
    environment: {}
  smartlinks:
    - sl_label: "Add Chrome Container"
      sl_chat:
        - role: "user"
          content: |
            🔧 Your job is to create a chrome container, using the image and environment from new_container_default section in the current config file: %CURRENT_CONFIG%. Follow the system prompt.
  smartlinks_for_each_container:
    - sl_label: "Use for integration"
      sl_chat:
        - role: "user"
          content: |
            🔧 Your job is to modify chrome config in the current file to connect through websockets to the container, use docker tool to inspect the container if needed. Current config file: %CURRENT_CONFIG%.
"#;
