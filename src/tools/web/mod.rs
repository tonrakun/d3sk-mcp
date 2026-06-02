use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::Duration;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use tokio::sync::Mutex;

pub type Res = (String, Option<String>);

fn ok() -> Res { ("ok".to_string(), None) }
fn e500(msg: impl ToString) -> Res { ("E500".to_string(), Some(msg.to_string())) }
fn e400(msg: impl ToString) -> Res { ("E400".to_string(), Some(msg.to_string())) }
fn e404(msg: impl ToString) -> Res { ("E404".to_string(), Some(msg.to_string())) }
fn e408(msg: impl ToString) -> Res { ("E408".to_string(), Some(msg.to_string())) }

// ── session manager ───────────────────────────────────────────────────────────

struct BrowserSession {
    browser: Browser,
    active_page: Option<Page>,
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, Arc<Mutex<BrowserSession>>>>,
    counter: AtomicU32,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            counter: AtomicU32::new(1),
        }
    }

    async fn insert(&self, session: BrowserSession) -> String {
        let id = format!("sess_{}", self.counter.fetch_add(1, Ordering::SeqCst));
        self.sessions.lock().await.insert(id.clone(), Arc::new(Mutex::new(session)));
        id
    }

    async fn get(&self, session_id: Option<&str>) -> Option<Arc<Mutex<BrowserSession>>> {
        let id = session_id.unwrap_or("default");
        self.sessions.lock().await.get(id).cloned()
    }

    async fn remove(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }
}

fn spawn_handler(mut handler: chromiumoxide::Handler) {
    tokio::spawn(async move { while handler.next().await.is_some() {} });
}

// Helper: clone active page (releasing lock) then run body
macro_rules! with_page {
    ($sm:expr, $sid:expr, |$page:ident| $body:expr) => {{
        let arc = match $sm.get($sid.as_deref()).await {
            Some(a) => a,
            None => return e404(format!("session not found: {}", $sid.as_deref().unwrap_or("default"))),
        };
        let $page = {
            let guard = arc.lock().await;
            match guard.active_page.as_ref() {
                Some(p) => p.clone(),
                None => return e404("no active page; call navigate or tab_new first"),
            }
        };
        $body
    }};
}

// ── browser_connect ───────────────────────────────────────────────────────────

pub async fn browser_connect(sm: &SessionManager, port: Option<u16>, _browser: Option<String>) -> Res {
    let url = format!("http://localhost:{}", port.unwrap_or(9222));
    match Browser::connect(url).await {
        Ok((browser, handler)) => {
            spawn_handler(handler);
            let pages = browser.pages().await.unwrap_or_default();
            let active_page = pages.into_iter().next();
            let id = sm.insert(BrowserSession { browser, active_page }).await;
            (id, None)
        }
        Err(e) => e500(e),
    }
}

// ── browser_open ──────────────────────────────────────────────────────────────

pub async fn browser_open(sm: &SessionManager, browser: Option<String>, profile: Option<String>) -> Res {
    let mut cfg = BrowserConfig::builder();

    #[cfg(windows)]
    if let Some(ref name) = browser {
        let exe = match name.as_str() {
            "chrome"  => r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            "brave"   => r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
            "edge"    => r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            _ => return e400(format!("unknown browser: {name}")),
        };
        cfg = cfg.chrome_executable(exe);
    }
    #[cfg(target_os = "macos")]
    if let Some(ref name) = browser {
        let exe = match name.as_str() {
            "chrome"  => "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "brave"   => "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "edge"    => "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            _ => return e400(format!("unknown browser: {name}")),
        };
        cfg = cfg.chrome_executable(exe);
    }

    if let Some(ref p) = profile {
        cfg = cfg.args([format!("--user-data-dir={}", p)]);
    }

    match cfg.build() {
        Ok(config) => match Browser::launch(config).await {
            Ok((b, handler)) => {
                spawn_handler(handler);
                let id = sm.insert(BrowserSession { browser: b, active_page: None }).await;
                (id, None)
            }
            Err(e) => e500(e),
        },
        Err(e) => e500(e),
    }
}

// ── navigate ──────────────────────────────────────────────────────────────────

pub async fn navigate(sm: &SessionManager, url: String, wait: Option<String>, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let mut sess = arc.lock().await;

    let page = match sess.active_page.take() {
        Some(p) => p,
        None => match sess.browser.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => return e500(e),
        },
    };

    if let Err(e) = page.goto(&url).await {
        sess.active_page = Some(page);
        return e500(e);
    }

    let wait_res = match wait.as_deref().unwrap_or("load") {
        "networkidle" => page.wait_for_navigation().await,
        _ => Ok(&page),
    };
    if let Err(e) = wait_res {
        sess.active_page = Some(page);
        return e500(e);
    }

    sess.active_page = Some(page);
    ok()
}

// ── get_url ───────────────────────────────────────────────────────────────────

pub async fn get_url(sm: &SessionManager, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        let url = match page.evaluate("location.href").await {
            Ok(v) => v.into_value::<String>().unwrap_or_default(),
            Err(e) => return e500(e),
        };
        let title = match page.evaluate("document.title").await {
            Ok(v) => v.into_value::<String>().unwrap_or_default(),
            Err(e) => return e500(e),
        };
        (format!("{},{}", url, title), None)
    })
}

// ── get_dom ───────────────────────────────────────────────────────────────────

pub async fn get_dom(sm: &SessionManager, selector: Option<String>, _depth: Option<u8>, interactive_only: Option<bool>, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        if interactive_only.unwrap_or(false) {
            let js = r#"Array.from(document.querySelectorAll('button,input,select,a,textarea')).map(el=>{const tag=el.tagName.toLowerCase();let sel=tag;if(el.id)sel='#'+el.id;else if(el.getAttribute('name'))sel=tag+'[name="'+el.getAttribute('name')+'"]';else if(el.className)sel=tag+'.'+el.className.toString().trim().split(/\s+/).join('.');const text=(el.textContent||el.value||el.placeholder||'').trim().replace(/\s+/g,' ').slice(0,100);return tag+','+sel+','+text;}).join('\n')"#;
            match page.evaluate(js).await {
                Ok(v) => return (v.into_value::<String>().unwrap_or_default(), None),
                Err(e) => return e500(e),
            }
        }

        if let Some(sel) = selector {
            let js = format!("document.querySelector({:?})?.outerHTML??''", sel);
            match page.evaluate(js).await {
                Ok(v) => return (v.into_value::<String>().unwrap_or_default(), None),
                Err(e) => return e500(e),
            }
        }

        match page.content().await {
            Ok(html) => (html, None),
            Err(e) => e500(e),
        }
    })
}

// ── click ─────────────────────────────────────────────────────────────────────

pub async fn click(sm: &SessionManager, selector: String, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        match page.find_element(&selector).await {
            Ok(el) => match el.click().await { Ok(_) => ok(), Err(e) => e500(e) },
            Err(_) => e404(format!("element not found: {selector}")),
        }
    })
}

// ── hover ─────────────────────────────────────────────────────────────────────

pub async fn hover(sm: &SessionManager, selector: String, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        match page.find_element(&selector).await {
            Ok(el) => match el.hover().await { Ok(_) => ok(), Err(e) => e500(e) },
            Err(_) => e404(format!("element not found: {selector}")),
        }
    })
}

// ── type ──────────────────────────────────────────────────────────────────────

pub async fn type_input(sm: &SessionManager, selector: String, text: String, clear: Option<bool>, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        match page.find_element(&selector).await {
            Ok(el) => {
                if clear.unwrap_or(false) {
                    let js = format!("document.querySelector({:?}).value=''", selector);
                    let _ = page.evaluate(js).await;
                }
                match el.type_str(&text).await { Ok(_) => ok(), Err(e) => e500(e) }
            }
            Err(_) => e404(format!("element not found: {selector}")),
        }
    })
}

// ── select ────────────────────────────────────────────────────────────────────

pub async fn select(sm: &SessionManager, selector: String, value: String, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        let js = format!(
            "(function(){{const el=document.querySelector({:?});if(!el)return 'E404';el.value={:?};el.dispatchEvent(new Event('change',{{bubbles:true}}));return 'ok';}})()",
            selector, value
        );
        match page.evaluate(js).await {
            Ok(v) => {
                let s = v.into_value::<String>().unwrap_or_default();
                if s == "E404" { e404(format!("element not found: {selector}")) } else { ok() }
            }
            Err(e) => e500(e),
        }
    })
}

// ── check ─────────────────────────────────────────────────────────────────────

pub async fn check(sm: &SessionManager, selector: String, checked: bool, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        let js = format!(
            "(function(){{const el=document.querySelector({:?});if(!el)return 'E404';el.checked={};el.dispatchEvent(new Event('change',{{bubbles:true}}));return 'ok';}})()",
            selector, checked
        );
        match page.evaluate(js).await {
            Ok(v) => {
                let s = v.into_value::<String>().unwrap_or_default();
                if s == "E404" { e404(format!("element not found: {selector}")) } else { ok() }
            }
            Err(e) => e500(e),
        }
    })
}

// ── scroll (web) ──────────────────────────────────────────────────────────────

pub async fn scroll_page(sm: &SessionManager, selector: Option<String>, delta_x: Option<i32>, delta_y: Option<i32>, unit: String, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        let (dx, dy) = (delta_x.unwrap_or(0), delta_y.unwrap_or(0));
        let (sdx, sdy) = if unit == "lines" { (dx * 40, dy * 40) } else { (dx, dy) };
        let js = match selector {
            Some(ref sel) => format!(
                "(function(){{const el=document.querySelector({:?});if(el)el.scrollBy({sdx},{sdy});return el?'ok':'E404';}})()",
                sel
            ),
            None => format!("window.scrollBy({sdx},{sdy});'ok'"),
        };
        match page.evaluate(js).await {
            Ok(v) => {
                let s = v.into_value::<String>().unwrap_or_default();
                if s == "E404" { e404("element not found") } else { ok() }
            }
            Err(e) => e500(e),
        }
    })
}

// ── wait_for ──────────────────────────────────────────────────────────────────

pub async fn wait_for(sm: &SessionManager, selector: String, timeout_ms: Option<u64>, state: Option<String>, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(30_000));
    let start = std::time::Instant::now();
    let hidden = state.as_deref() == Some("hidden");

    loop {
        let page = {
            let guard = arc.lock().await;
            guard.active_page.as_ref().map(|p| p.clone())
        };
        if let Some(page) = page {
            let js = format!(
                "(function(){{const el=document.querySelector({:?});if(!el)return 'missing';const vis=el.offsetParent!==null||el.getBoundingClientRect().width>0;return vis?'visible':'hidden';}})()",
                selector
            );
            if let Ok(v) = page.evaluate(js).await {
                let s = v.into_value::<String>().unwrap_or_default();
                let done = if hidden { s == "hidden" || s == "missing" }
                           else     { s == "visible" };
                if done { return ok(); }
            }
        }
        if start.elapsed() >= timeout { return e408("wait_for timed out"); }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

// ── web_screenshot ────────────────────────────────────────────────────────────

pub async fn web_screenshot(sm: &SessionManager, full_page: Option<bool>, _selector: Option<String>, _scale: Option<f32>, quality: Option<u8>, session_id: Option<String>) -> Res {
    use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as B64;

    with_page!(sm, session_id, |page| {
        let q = quality.unwrap_or(80) as i64;
        let params = chromiumoxide::page::ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(q)
            .full_page(full_page.unwrap_or(false))
            .build();

        match page.screenshot(params).await {
            Ok(bytes) => {
                match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        let (w, h) = (img.width(), img.height());
                        (format!("{},{},{}", B64.encode(&bytes), w, h), None)
                    }
                    Err(_) => {
                        (format!("{},0,0", B64.encode(&bytes)), None)
                    }
                }
            }
            Err(e) => e500(e),
        }
    })
}

// ── evaluate ──────────────────────────────────────────────────────────────────

pub async fn evaluate(sm: &SessionManager, script: String, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        match page.evaluate(script).await {
            Ok(v) => {
                let result = match v.into_value::<serde_json::Value>() {
                    Ok(val) => match val {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    },
                    Err(_) => String::new(),
                };
                (result, None)
            }
            Err(e) => e500(e),
        }
    })
}

// ── tab_list ──────────────────────────────────────────────────────────────────

pub async fn tab_list(sm: &SessionManager, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let sess = arc.lock().await;
    let pages = match sess.browser.pages().await {
        Ok(p) => p,
        Err(e) => return e500(e),
    };

    let mut lines = Vec::new();
    for page in &pages {
        let id = page.target_id().inner().to_string();
        let url = page.evaluate("location.href").await
            .ok().and_then(|v| v.into_value::<String>().ok()).unwrap_or_default();
        let title = page.evaluate("document.title").await
            .ok().and_then(|v| v.into_value::<String>().ok()).unwrap_or_default();
        lines.push(format!("{id},{url},{title}"));
    }
    (lines.join("\n"), None)
}

// ── tab_new ───────────────────────────────────────────────────────────────────

pub async fn tab_new(sm: &SessionManager, url: Option<String>, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let mut sess = arc.lock().await;
    let target_url = url.as_deref().unwrap_or("about:blank");
    match sess.browser.new_page(target_url).await {
        Ok(page) => {
            let id = page.target_id().inner().to_string();
            sess.active_page = Some(page);
            (id, None)
        }
        Err(e) => e500(e),
    }
}

// ── tab_switch ────────────────────────────────────────────────────────────────

pub async fn tab_switch(sm: &SessionManager, id: String, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let mut sess = arc.lock().await;
    let pages = match sess.browser.pages().await {
        Ok(p) => p,
        Err(e) => return e500(e),
    };
    let page = pages.into_iter().find(|p| p.target_id().inner() == id.as_str());
    match page {
        Some(p) => { sess.active_page = Some(p); ok() }
        None    => e404(format!("tab not found: {id}")),
    }
}

// ── tab_close ─────────────────────────────────────────────────────────────────

pub async fn tab_close(sm: &SessionManager, id: String, session_id: Option<String>) -> Res {
    let arc = match sm.get(session_id.as_deref()).await {
        Some(a) => a,
        None => return e404("session not found"),
    };
    let mut sess = arc.lock().await;
    let pages = match sess.browser.pages().await {
        Ok(p) => p,
        Err(e) => return e500(e),
    };
    let found = pages.iter().find(|p| p.target_id().inner() == id.as_str());
    match found {
        None => return e404(format!("tab not found: {id}")),
        Some(p) => {
            if let Err(e) = p.clone().close().await { return e500(e); }
        }
    }
    if sess.active_page.as_ref().map(|p| p.target_id().inner().to_string()).as_deref() == Some(id.as_str()) {
        let remaining = sess.browser.pages().await.unwrap_or_default();
        sess.active_page = remaining.into_iter().next();
    }
    ok()
}

// ── cookie_get ────────────────────────────────────────────────────────────────

pub async fn cookie_get(sm: &SessionManager, url: Option<String>, name: Option<String>, session_id: Option<String>) -> Res {
    with_page!(sm, session_id, |page| {
        let cookies = match page.get_cookies().await {
            Ok(c) => c,
            Err(e) => return e500(e),
        };
        let filtered: Vec<String> = cookies.iter()
            .filter(|c| {
                let url_ok = url.as_ref().map(|u| u.contains(c.domain.as_str())).unwrap_or(true);
                let name_ok = name.as_ref().map(|n| c.name == *n).unwrap_or(true);
                url_ok && name_ok
            })
            .map(|c| format!("{},{},{}", c.name, c.value, c.domain))
            .collect();
        (filtered.join("\n"), None)
    })
}
