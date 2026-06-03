pub mod common;
pub mod pc;
pub mod web;

use std::sync::{Arc, Mutex};
use serde::Deserialize;
use schemars::JsonSchema;
use rmcp::{ServerHandler, tool_router, tool_handler, tool};
use rmcp::handler::server::wrapper::Parameters;
use common::is_error;

pub struct D3skServer {
    pub last_error: Arc<Mutex<Option<String>>>,
    pub web_sessions: Arc<web::SessionManager>,
}

impl D3skServer {
    pub fn new() -> Self {
        Self {
            last_error: Arc::new(Mutex::new(None)),
            web_sessions: Arc::new(web::SessionManager::new()),
        }
    }

    pub fn set_error(&self, msg: impl Into<String>) {
        *self.last_error.lock().unwrap() = Some(msg.into());
    }

    pub fn take_error(&self) -> String {
        self.last_error.lock().unwrap().take().unwrap_or_default()
    }

    fn apply(&self, res: common::Res) -> String {
        let (r, e) = res;
        if let Some(msg) = e { self.set_error(msg); }
        r
    }
}

// ── dispatch (for batch) ──────────────────────────────────────────────────────

impl D3skServer {
    async fn dispatch_tool(&self, name: &str, a: &serde_json::Value) -> String {
        let gs  = |k: &str| a.get(k).and_then(|v| v.as_str()).map(String::from);
        let gi  = |k: &str| a.get(k).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let gb  = |k: &str| a.get(k).and_then(|v| v.as_bool());
        let gu  = |k: &str| a.get(k).and_then(|v| v.as_u64());
        let gf  = |k: &str| a.get(k).and_then(|v| v.as_f64()).map(|f| f as f32);
        let garr = |k: &str| -> Vec<String> {
            a.get(k).and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default()
        };
        let sm = &self.web_sessions;

        match name {
            "get_last_error"   => self.take_error(),
            "screenshot"       => self.apply(pc::screenshot(gs("target").unwrap_or_else(||"screen".into()), gs("window_title"), gf("scale"), gu("quality").map(|q|q as u8)).await),
            "mouse_move"       => self.apply(pc::mouse_move(gi("x"), gi("y")).await),
            "mouse_click"      => self.apply(pc::mouse_click(gi("x"), gi("y"), gs("action").unwrap_or_else(||"single".into())).await),
            "mouse_scroll"     => self.apply(pc::mouse_scroll(gi("x"), gi("y"), gi("delta_x"), gi("delta_y"), gs("unit").unwrap_or_else(||"lines".into())).await),
            "mouse_drag"       => self.apply(pc::mouse_drag(gi("from_x"), gi("from_y"), gi("to_x"), gi("to_y"), gu("duration_ms")).await),
            "key_press"        => self.apply(pc::key_press(garr("keys")).await),
            "type_text"        => self.apply(pc::type_text(gs("text").unwrap_or_default(), gu("delay_ms")).await),
            "clipboard_get"    => self.apply(pc::clipboard_get().await),
            "clipboard_set"    => self.apply(pc::clipboard_set(gs("text").unwrap_or_default()).await),
            "file_read"        => self.apply(pc::file_read(gs("path").unwrap_or_default(), gs("encoding")).await),
            "file_write"       => self.apply(pc::file_write(gs("path").unwrap_or_default(), gs("content").unwrap_or_default(), gs("encoding")).await),
            "file_list"        => self.apply(pc::file_list(gs("path").unwrap_or_default(), gb("recursive")).await),
            "file_delete"      => self.apply(pc::file_delete(gs("path").unwrap_or_default()).await),
            "file_move"        => self.apply(pc::file_move(gs("from").unwrap_or_default(), gs("to").unwrap_or_default()).await),
            "file_copy"        => self.apply(pc::file_copy(gs("from").unwrap_or_default(), gs("to").unwrap_or_default()).await),
            "file_exists"      => self.apply(pc::file_exists(gs("path").unwrap_or_default()).await),
            "app_launch"       => self.apply(pc::app_launch(gs("command").unwrap_or_default(), a.get("args").and_then(|v|v.as_array()).map(|arr|arr.iter().filter_map(|v|v.as_str().map(String::from)).collect())).await),
            "app_list"         => self.apply(pc::app_list().await),
            "app_focus"        => self.apply(pc::app_focus(gu("pid").map(|p|p as u32), gs("title")).await),
            "app_close"        => self.apply(pc::app_close(gu("pid").unwrap_or(0) as u32, gb("force")).await),
            "shell"            => self.apply(pc::shell(gs("cmd").unwrap_or_default(), gu("timeout_ms"), gs("cwd")).await),
            "browser_connect"  => self.apply(web::browser_connect(sm, gu("port").map(|p|p as u16), gs("browser")).await),
            "browser_open"     => self.apply(web::browser_open(sm, gs("browser"), gs("profile")).await),
            "browser_close"    => self.apply(web::browser_close(sm, gs("session_id")).await),
            "navigate"         => self.apply(web::navigate(sm, gs("url"), gs("action"), gs("wait"), gs("session_id")).await),
            "get_url"          => self.apply(web::get_url(sm, gs("session_id")).await),
            "get_dom"          => self.apply(web::get_dom(sm, gs("selector"), gu("depth").map(|d|d as u8), gb("interactive_only"), gs("session_id")).await),
            "get_text"         => self.apply(web::get_text(sm, gs("selector").unwrap_or_default(), gs("session_id")).await),
            "get_attr"         => self.apply(web::get_attr(sm, gs("selector").unwrap_or_default(), gs("attr").unwrap_or_default(), gs("session_id")).await),
            "click"            => self.apply(web::click(sm, gs("selector").unwrap_or_default(), gs("session_id")).await),
            "hover"            => self.apply(web::hover(sm, gs("selector").unwrap_or_default(), gs("session_id")).await),
            "type_input"       => self.apply(web::type_input(sm, gs("selector").unwrap_or_default(), gs("text").unwrap_or_default(), gb("clear"), gu("delay_ms"), gs("session_id")).await),
            "web_select"       => self.apply(web::select(sm, gs("selector").unwrap_or_default(), gs("value").unwrap_or_default(), gs("session_id")).await),
            "check"            => self.apply(web::check(sm, gs("selector").unwrap_or_default(), gb("checked").unwrap_or(false), gs("session_id")).await),
            "web_scroll"       => self.apply(web::scroll_page(sm, gs("selector"), a.get("delta_x").and_then(|v|v.as_i64()).map(|v|v as i32), a.get("delta_y").and_then(|v|v.as_i64()).map(|v|v as i32), gs("unit").unwrap_or_else(||"lines".into()), gs("session_id")).await),
            "wait_for"         => self.apply(web::wait_for(sm, gs("selector").unwrap_or_default(), gu("timeout_ms"), gs("state"), gs("session_id")).await),
            "web_screenshot"   => self.apply(web::web_screenshot(sm, gb("full_page"), gs("selector"), gf("scale"), gu("quality").map(|q|q as u8), gs("session_id")).await),
            "evaluate"         => self.apply(web::evaluate(sm, gs("script").unwrap_or_default(), gs("session_id")).await),
            "dialog_handle"    => self.apply(web::dialog_handle(sm, gs("action").unwrap_or_default(), gs("text"), gs("session_id")).await),
            "tab_list"         => self.apply(web::tab_list(sm, gs("session_id")).await),
            "tab_new"          => self.apply(web::tab_new(sm, gs("url"), gs("session_id")).await),
            "tab_switch"       => self.apply(web::tab_switch(sm, gs("id").unwrap_or_default(), gs("session_id")).await),
            "tab_close"        => self.apply(web::tab_close(sm, gs("id").unwrap_or_default(), gs("session_id")).await),
            "cookie_get"       => self.apply(web::cookie_get(sm, gs("url"), gs("name"), gs("session_id")).await),
            "cookie_set"       => self.apply(web::cookie_set(sm, gs("name").unwrap_or_default(), gs("value").unwrap_or_default(), gs("url"), gs("domain"), gs("path"), gs("session_id")).await),
            _ => { self.set_error(format!("unknown tool: {name}")); "E404".to_string() }
        }
    }
}

// ── parameter structs ─────────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)] struct BatchParams          { steps_json: String, on_error: Option<String> }
#[derive(Deserialize, JsonSchema)] struct ScreenshotParams     { target: String, window_title: Option<String>, scale: Option<f32>, quality: Option<u8> }
#[derive(Deserialize, JsonSchema)] struct MouseMoveParams      { x: i32, y: i32 }
#[derive(Deserialize, JsonSchema)] struct MouseClickParams     { x: i32, y: i32, action: String }
#[derive(Deserialize, JsonSchema)] struct MouseScrollParams    { x: i32, y: i32, delta_x: i32, delta_y: i32, unit: String }
#[derive(Deserialize, JsonSchema)] struct MouseDragParams      { from_x: i32, from_y: i32, to_x: i32, to_y: i32, duration_ms: Option<u64> }
#[derive(Deserialize, JsonSchema)] struct KeyPressParams       { keys: Vec<String> }
#[derive(Deserialize, JsonSchema)] struct TypeTextParams       { text: String, delay_ms: Option<u64> }
#[derive(Deserialize, JsonSchema)] struct ClipboardSetParams   { text: String }
#[derive(Deserialize, JsonSchema)] struct FileReadParams       { path: String, encoding: Option<String> }
#[derive(Deserialize, JsonSchema)] struct FileWriteParams      { path: String, content: String, encoding: Option<String> }
#[derive(Deserialize, JsonSchema)] struct FileListParams       { path: String, recursive: Option<bool> }
#[derive(Deserialize, JsonSchema)] struct FileDeleteParams     { path: String }
#[derive(Deserialize, JsonSchema)] struct FileMoveParams       { from: String, to: String }
#[derive(Deserialize, JsonSchema)] struct FileCopyParams       { from: String, to: String }
#[derive(Deserialize, JsonSchema)] struct FileExistsParams     { path: String }
#[derive(Deserialize, JsonSchema)] struct AppLaunchParams      { command: String, args: Option<Vec<String>> }
#[derive(Deserialize, JsonSchema)] struct AppFocusParams       { pid: Option<u32>, title: Option<String> }
#[derive(Deserialize, JsonSchema)] struct AppCloseParams       { pid: u32, force: Option<bool> }
#[derive(Deserialize, JsonSchema)] struct ShellParams          { cmd: String, timeout_ms: Option<u64>, cwd: Option<String> }
#[derive(Deserialize, JsonSchema)] struct BrowserConnectParams { port: Option<u16>, browser: Option<String> }
#[derive(Deserialize, JsonSchema)] struct BrowserOpenParams    { browser: Option<String>, profile: Option<String> }
#[derive(Deserialize, JsonSchema)] struct BrowserCloseParams   { session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct NavigateParams       { url: Option<String>, action: Option<String>, wait: Option<String>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct SessionParam         { session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct GetDomParams         { selector: Option<String>, depth: Option<u8>, interactive_only: Option<bool>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct SelectorParams       { selector: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct GetAttrParams        { selector: String, attr: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct TypeInputParams      { selector: String, text: String, clear: Option<bool>, delay_ms: Option<u64>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct SelectParams         { selector: String, value: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct CheckParams          { selector: String, checked: bool, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct WebScrollParams      { selector: Option<String>, delta_x: Option<i32>, delta_y: Option<i32>, unit: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct WaitForParams        { selector: String, timeout_ms: Option<u64>, state: Option<String>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct WebScreenshotParams  { full_page: Option<bool>, selector: Option<String>, scale: Option<f32>, quality: Option<u8>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct EvaluateParams       { script: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct DialogHandleParams   { action: String, text: Option<String>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct TabIdParams          { id: String, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct TabNewParams         { url: Option<String>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct CookieGetParams      { url: Option<String>, name: Option<String>, session_id: Option<String> }
#[derive(Deserialize, JsonSchema)] struct CookieSetParams      { name: String, value: String, url: Option<String>, domain: Option<String>, path: Option<String>, session_id: Option<String> }

// ── tool definitions ──────────────────────────────────────────────────────────

#[tool_router]
impl D3skServer {
    #[tool(description = "Get detail message of the last error")]
    async fn get_last_error(&self) -> String { self.take_error() }

    #[tool(description = "Execute multiple tools in sequence. steps_json: JSON array of [{tool,...args}]. on_error: 'stop'|'continue'")]
    async fn batch(&self, Parameters(p): Parameters<BatchParams>) -> String {
        let steps: Vec<serde_json::Value> = match serde_json::from_str(&p.steps_json) {
            Ok(v) => v,
            Err(e) => { self.set_error(e.to_string()); return "E400".to_string(); }
        };
        let on_error = p.on_error.as_deref().unwrap_or("stop");
        let mut results: Vec<String> = Vec::new();
        for step in &steps {
            let name = step.get("tool").and_then(|v| v.as_str()).unwrap_or("");
            let result = self.dispatch_tool(name, step).await;
            let failed = is_error(&result);
            results.push(result);
            if on_error == "stop" && failed { break; }
        }
        serde_json::to_string(&results).unwrap_or_default()
    }

    // ── screenshot ───────────────────────────────────────────────────────────

    #[tool(description = "Take screenshot. target: 'screen'|'window'. scale: 0.1-2.0. Returns '<base64>,<w>,<h>'")]
    async fn screenshot(&self, Parameters(p): Parameters<ScreenshotParams>) -> String {
        self.apply(pc::screenshot(p.target, p.window_title, p.scale, p.quality).await)
    }

    // ── mouse ────────────────────────────────────────────────────────────────

    #[tool(description = "Move mouse to absolute position")]
    async fn mouse_move(&self, Parameters(p): Parameters<MouseMoveParams>) -> String {
        self.apply(pc::mouse_move(p.x, p.y).await)
    }

    #[tool(description = "Click mouse. action: 'single'|'double'|'right'|'middle'")]
    async fn mouse_click(&self, Parameters(p): Parameters<MouseClickParams>) -> String {
        self.apply(pc::mouse_click(p.x, p.y, p.action).await)
    }

    #[tool(description = "Scroll mouse at position. unit: 'px'|'lines'")]
    async fn mouse_scroll(&self, Parameters(p): Parameters<MouseScrollParams>) -> String {
        self.apply(pc::mouse_scroll(p.x, p.y, p.delta_x, p.delta_y, p.unit).await)
    }

    #[tool(description = "Drag mouse from one position to another. duration_ms: total drag time in ms (default 100)")]
    async fn mouse_drag(&self, Parameters(p): Parameters<MouseDragParams>) -> String {
        self.apply(pc::mouse_drag(p.from_x, p.from_y, p.to_x, p.to_y, p.duration_ms).await)
    }

    // ── keyboard ─────────────────────────────────────────────────────────────

    #[tool(description = "Press key combination. keys: e.g. ['ctrl','c']")]
    async fn key_press(&self, Parameters(p): Parameters<KeyPressParams>) -> String {
        self.apply(pc::key_press(p.keys).await)
    }

    #[tool(description = "Type text via keyboard. delay_ms: per-character delay in ms")]
    async fn type_text(&self, Parameters(p): Parameters<TypeTextParams>) -> String {
        self.apply(pc::type_text(p.text, p.delay_ms).await)
    }

    // ── clipboard ─────────────────────────────────────────────────────────────

    #[tool(description = "Get clipboard text")]
    async fn clipboard_get(&self) -> String { self.apply(pc::clipboard_get().await) }

    #[tool(description = "Set clipboard text")]
    async fn clipboard_set(&self, Parameters(p): Parameters<ClipboardSetParams>) -> String {
        self.apply(pc::clipboard_set(p.text).await)
    }

    // ── file ──────────────────────────────────────────────────────────────────

    #[tool(description = "Read file. encoding: 'utf8'|'base64'")]
    async fn file_read(&self, Parameters(p): Parameters<FileReadParams>) -> String {
        self.apply(pc::file_read(p.path, p.encoding).await)
    }

    #[tool(description = "Write file. encoding: 'utf8'|'base64'")]
    async fn file_write(&self, Parameters(p): Parameters<FileWriteParams>) -> String {
        self.apply(pc::file_write(p.path, p.content, p.encoding).await)
    }

    #[tool(description = "List directory. Returns JSON [{name,is_dir,size}]")]
    async fn file_list(&self, Parameters(p): Parameters<FileListParams>) -> String {
        self.apply(pc::file_list(p.path, p.recursive).await)
    }

    #[tool(description = "Delete file or directory")]
    async fn file_delete(&self, Parameters(p): Parameters<FileDeleteParams>) -> String {
        self.apply(pc::file_delete(p.path).await)
    }

    #[tool(description = "Move or rename file")]
    async fn file_move(&self, Parameters(p): Parameters<FileMoveParams>) -> String {
        self.apply(pc::file_move(p.from, p.to).await)
    }

    #[tool(description = "Copy file")]
    async fn file_copy(&self, Parameters(p): Parameters<FileCopyParams>) -> String {
        self.apply(pc::file_copy(p.from, p.to).await)
    }

    #[tool(description = "Check if file or directory exists. Returns 'true' or 'false'")]
    async fn file_exists(&self, Parameters(p): Parameters<FileExistsParams>) -> String {
        self.apply(pc::file_exists(p.path).await)
    }

    // ── app ───────────────────────────────────────────────────────────────────

    #[tool(description = "Launch application. Returns PID")]
    async fn app_launch(&self, Parameters(p): Parameters<AppLaunchParams>) -> String {
        self.apply(pc::app_launch(p.command, p.args).await)
    }

    #[tool(description = "List running windowed apps. Returns JSON [{pid,name,title}]")]
    async fn app_list(&self) -> String { self.apply(pc::app_list().await) }

    #[tool(description = "Bring window to front. Provide pid or title")]
    async fn app_focus(&self, Parameters(p): Parameters<AppFocusParams>) -> String {
        self.apply(pc::app_focus(p.pid, p.title).await)
    }

    #[tool(description = "Close app by PID. force: kill immediately")]
    async fn app_close(&self, Parameters(p): Parameters<AppCloseParams>) -> String {
        self.apply(pc::app_close(p.pid, p.force).await)
    }

    // ── shell ─────────────────────────────────────────────────────────────────

    #[tool(description = "Execute shell command. cwd: working directory. timeout_ms default 30000. Returns JSON {stdout,stderr,exit_code}")]
    async fn shell(&self, Parameters(p): Parameters<ShellParams>) -> String {
        self.apply(pc::shell(p.cmd, p.timeout_ms, p.cwd).await)
    }

    // ── web: browser ──────────────────────────────────────────────────────────

    #[tool(description = "Connect to running browser via CDP. port default 9222. Returns session_id")]
    async fn browser_connect(&self, Parameters(p): Parameters<BrowserConnectParams>) -> String {
        self.apply(web::browser_connect(&self.web_sessions, p.port, p.browser).await)
    }

    #[tool(description = "Launch new browser. browser: 'chrome'|'brave'|'edge'. Returns session_id")]
    async fn browser_open(&self, Parameters(p): Parameters<BrowserOpenParams>) -> String {
        self.apply(web::browser_open(&self.web_sessions, p.browser, p.profile).await)
    }

    #[tool(description = "Close browser session. For browser_open sessions, terminates the browser process")]
    async fn browser_close(&self, Parameters(p): Parameters<BrowserCloseParams>) -> String {
        self.apply(web::browser_close(&self.web_sessions, p.session_id).await)
    }

    // ── web: navigation ───────────────────────────────────────────────────────

    #[tool(description = "Navigate to URL or perform history action (back/forward/reload). wait: 'load'|'domcontentloaded'|'networkidle'")]
    async fn navigate(&self, Parameters(p): Parameters<NavigateParams>) -> String {
        self.apply(web::navigate(&self.web_sessions, p.url, p.action, p.wait, p.session_id).await)
    }

    #[tool(description = "Get current URL and title. Returns JSON {url,title}")]
    async fn get_url(&self, Parameters(p): Parameters<SessionParam>) -> String {
        self.apply(web::get_url(&self.web_sessions, p.session_id).await)
    }

    // ── web: DOM ──────────────────────────────────────────────────────────────

    #[tool(description = "Get DOM. depth: max nesting depth. interactive_only: returns JSON [{tag,selector,text}] for interactive elements")]
    async fn get_dom(&self, Parameters(p): Parameters<GetDomParams>) -> String {
        self.apply(web::get_dom(&self.web_sessions, p.selector, p.depth, p.interactive_only, p.session_id).await)
    }

    #[tool(description = "Get text content of an element by CSS selector. Lighter than get_dom")]
    async fn get_text(&self, Parameters(p): Parameters<SelectorParams>) -> String {
        self.apply(web::get_text(&self.web_sessions, p.selector, p.session_id).await)
    }

    #[tool(description = "Get attribute value of an element. attr: attribute name (e.g. 'href', 'src', 'value')")]
    async fn get_attr(&self, Parameters(p): Parameters<GetAttrParams>) -> String {
        self.apply(web::get_attr(&self.web_sessions, p.selector, p.attr, p.session_id).await)
    }

    // ── web: elements ─────────────────────────────────────────────────────────

    #[tool(description = "Click element by CSS selector")]
    async fn click(&self, Parameters(p): Parameters<SelectorParams>) -> String {
        self.apply(web::click(&self.web_sessions, p.selector, p.session_id).await)
    }

    #[tool(description = "Hover over element by CSS selector")]
    async fn hover(&self, Parameters(p): Parameters<SelectorParams>) -> String {
        self.apply(web::hover(&self.web_sessions, p.selector, p.session_id).await)
    }

    #[tool(description = "Type text into element. clear: clear first. delay_ms: per-character delay in ms")]
    async fn type_input(&self, Parameters(p): Parameters<TypeInputParams>) -> String {
        self.apply(web::type_input(&self.web_sessions, p.selector, p.text, p.clear, p.delay_ms, p.session_id).await)
    }

    #[tool(description = "Select option in <select> by value")]
    async fn web_select(&self, Parameters(p): Parameters<SelectParams>) -> String {
        self.apply(web::select(&self.web_sessions, p.selector, p.value, p.session_id).await)
    }

    #[tool(description = "Set checkbox/radio checked state")]
    async fn check(&self, Parameters(p): Parameters<CheckParams>) -> String {
        self.apply(web::check(&self.web_sessions, p.selector, p.checked, p.session_id).await)
    }

    #[tool(description = "Scroll page or element. unit: 'px'|'lines'")]
    async fn web_scroll(&self, Parameters(p): Parameters<WebScrollParams>) -> String {
        self.apply(web::scroll_page(&self.web_sessions, p.selector, p.delta_x, p.delta_y, p.unit, p.session_id).await)
    }

    // ── web: wait ─────────────────────────────────────────────────────────────

    #[tool(description = "Wait for element state. state: 'visible'|'hidden'|'attached'|'detached'. timeout_ms default 30000")]
    async fn wait_for(&self, Parameters(p): Parameters<WaitForParams>) -> String {
        self.apply(web::wait_for(&self.web_sessions, p.selector, p.timeout_ms, p.state, p.session_id).await)
    }

    // ── web: screenshot ───────────────────────────────────────────────────────

    #[tool(description = "Browser screenshot. selector: capture element only. scale: 0.1-2.0. Returns '<base64>,<w>,<h>'")]
    async fn web_screenshot(&self, Parameters(p): Parameters<WebScreenshotParams>) -> String {
        self.apply(web::web_screenshot(&self.web_sessions, p.full_page, p.selector, p.scale, p.quality, p.session_id).await)
    }

    // ── web: JavaScript ───────────────────────────────────────────────────────

    #[tool(description = "Evaluate JavaScript in page. Returns result as string")]
    async fn evaluate(&self, Parameters(p): Parameters<EvaluateParams>) -> String {
        self.apply(web::evaluate(&self.web_sessions, p.script, p.session_id).await)
    }

    // ── web: dialog ───────────────────────────────────────────────────────────

    #[tool(description = "Handle alert/confirm/prompt dialog. action: 'accept'|'dismiss'. text: prompt input value")]
    async fn dialog_handle(&self, Parameters(p): Parameters<DialogHandleParams>) -> String {
        self.apply(web::dialog_handle(&self.web_sessions, p.action, p.text, p.session_id).await)
    }

    // ── web: tabs ─────────────────────────────────────────────────────────────

    #[tool(description = "List open tabs. Returns JSON [{id,url,title}]")]
    async fn tab_list(&self, Parameters(p): Parameters<SessionParam>) -> String {
        self.apply(web::tab_list(&self.web_sessions, p.session_id).await)
    }

    #[tool(description = "Open new tab. Returns tab id")]
    async fn tab_new(&self, Parameters(p): Parameters<TabNewParams>) -> String {
        self.apply(web::tab_new(&self.web_sessions, p.url, p.session_id).await)
    }

    #[tool(description = "Switch active tab by id")]
    async fn tab_switch(&self, Parameters(p): Parameters<TabIdParams>) -> String {
        self.apply(web::tab_switch(&self.web_sessions, p.id, p.session_id).await)
    }

    #[tool(description = "Close tab by id")]
    async fn tab_close(&self, Parameters(p): Parameters<TabIdParams>) -> String {
        self.apply(web::tab_close(&self.web_sessions, p.id, p.session_id).await)
    }

    // ── web: cookie ───────────────────────────────────────────────────────────

    #[tool(description = "Get cookies. Returns JSON [{name,value,domain}]")]
    async fn cookie_get(&self, Parameters(p): Parameters<CookieGetParams>) -> String {
        self.apply(web::cookie_get(&self.web_sessions, p.url, p.name, p.session_id).await)
    }

    #[tool(description = "Set a cookie. url or domain required to scope the cookie")]
    async fn cookie_set(&self, Parameters(p): Parameters<CookieSetParams>) -> String {
        self.apply(web::cookie_set(&self.web_sessions, p.name, p.value, p.url, p.domain, p.path, p.session_id).await)
    }
}

#[tool_handler]
impl ServerHandler for D3skServer {}
