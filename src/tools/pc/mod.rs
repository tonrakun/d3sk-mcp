use std::io::Cursor;
use std::fs;
use std::path::Path;
use std::process::Command;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use enigo::{Enigo, Mouse, Keyboard, Settings, Direction, Button, Coordinate, Key, Axis};
use xcap::{Monitor, Window};
use image::{DynamicImage, codecs::jpeg::JpegEncoder};
use arboard::Clipboard;
use serde::Serialize;
use super::common::{Res, ok, e400, e404, e500};

fn new_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| e.to_string())
}

// ── screenshot ──────────────────────────────────────────────────────────────

pub async fn screenshot(
    target: String,
    window_title: Option<String>,
    scale: Option<f32>,
    quality: Option<u8>,
) -> Res {
    let scale = scale.unwrap_or(1.0).clamp(0.1, 2.0);
    let quality = quality.unwrap_or(80).clamp(1, 100);

    let raw = match target.as_str() {
        "screen" => {
            let monitors = match Monitor::all() { Ok(m) => m, Err(e) => return e500(e) };
            if monitors.is_empty() { return e404("no monitors found"); }
            match monitors[0].capture_image() { Ok(img) => img, Err(e) => return e500(e) }
        }
        "window" => {
            let title = match window_title { Some(t) => t, None => return e400("window_title required") };
            let windows = match Window::all() { Ok(w) => w, Err(e) => return e500(e) };
            let win = windows.iter().find(|w| w.title().unwrap_or_default().contains(title.as_str()));
            match win {
                Some(w) => match w.capture_image() { Ok(img) => img, Err(e) => return e500(e) },
                None => return e404(format!("window not found: {title}")),
            }
        }
        _ => return e400("target must be 'screen' or 'window'"),
    };

    let orig_w = raw.width();
    let orig_h = raw.height();
    let dyn_img = DynamicImage::ImageRgba8(raw);

    let dyn_img = if (scale - 1.0).abs() > 0.001 {
        let nw = (orig_w as f32 * scale) as u32;
        let nh = (orig_h as f32 * scale) as u32;
        dyn_img.resize(nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        dyn_img
    };
    let fw = dyn_img.width();
    let fh = dyn_img.height();

    let mut buf = Cursor::new(Vec::<u8>::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    if let Err(e) = dyn_img.into_rgb8().write_with_encoder(encoder) {
        return e500(e);
    }
    (format!("{},{},{}", B64.encode(buf.into_inner()), fw, fh), None)
}

// ── mouse ───────────────────────────────────────────────────────────────────

pub async fn mouse_move(x: i32, y: i32) -> Res {
    let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
    match eg.move_mouse(x, y, Coordinate::Abs) { Ok(_) => ok(), Err(e) => e500(e) }
}

pub async fn mouse_click(x: i32, y: i32, action: String) -> Res {
    tokio::task::spawn_blocking(move || {
        let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
        if let Err(e) = eg.move_mouse(x, y, Coordinate::Abs) { return e500(e); }
        match action.as_str() {
            "single" => match eg.button(Button::Left, Direction::Click) { Ok(_) => ok(), Err(e) => e500(e) },
            "double" => {
                if let Err(e) = eg.button(Button::Left, Direction::Click) { return e500(e); }
                std::thread::sleep(std::time::Duration::from_millis(50));
                match eg.button(Button::Left, Direction::Click) { Ok(_) => ok(), Err(e) => e500(e) }
            }
            "right"  => match eg.button(Button::Right,  Direction::Click) { Ok(_) => ok(), Err(e) => e500(e) },
            "middle" => match eg.button(Button::Middle, Direction::Click) { Ok(_) => ok(), Err(e) => e500(e) },
            _ => e400("action must be single/double/right/middle"),
        }
    }).await.unwrap_or_else(|e| e500(e.to_string()))
}

pub async fn mouse_scroll(x: i32, y: i32, delta_x: i32, delta_y: i32, unit: String) -> Res {
    let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
    if let Err(e) = eg.move_mouse(x, y, Coordinate::Abs) { return e500(e); }
    let (dx, dy) = if unit == "px" { (delta_x / 40, delta_y / 40) } else { (delta_x, delta_y) };
    if dy != 0 && let Err(e) = eg.scroll(dy, Axis::Vertical)   { return e500(e); }
    if dx != 0 && let Err(e) = eg.scroll(dx, Axis::Horizontal) { return e500(e); }
    ok()
}

pub async fn mouse_drag(from_x: i32, from_y: i32, to_x: i32, to_y: i32, duration_ms: Option<u64>) -> Res {
    tokio::task::spawn_blocking(move || {
        let half = std::time::Duration::from_millis(duration_ms.unwrap_or(100) / 2);
        let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
        if let Err(e) = eg.move_mouse(from_x, from_y, Coordinate::Abs) { return e500(e); }
        if let Err(e) = eg.button(Button::Left, Direction::Press) { return e500(e); }
        std::thread::sleep(half);
        if let Err(e) = eg.move_mouse(to_x, to_y, Coordinate::Abs) {
            let _ = eg.button(Button::Left, Direction::Release);
            return e500(e);
        }
        std::thread::sleep(half);
        match eg.button(Button::Left, Direction::Release) { Ok(_) => ok(), Err(e) => e500(e) }
    }).await.unwrap_or_else(|e| e500(e.to_string()))
}

// ── keyboard ─────────────────────────────────────────────────────────────────

fn parse_key(name: &str) -> Option<Key> {
    match name.to_lowercase().as_str() {
        "ctrl" | "control"           => Some(Key::Control),
        "alt"                        => Some(Key::Alt),
        "shift"                      => Some(Key::Shift),
        "meta"|"super"|"win"|"cmd"   => Some(Key::Meta),
        "enter" | "return"           => Some(Key::Return),
        "esc" | "escape"             => Some(Key::Escape),
        "tab"                        => Some(Key::Tab),
        "space"                      => Some(Key::Space),
        "backspace"                  => Some(Key::Backspace),
        "delete"                     => Some(Key::Delete),
        "home"                       => Some(Key::Home),
        "end"                        => Some(Key::End),
        "pageup"                     => Some(Key::PageUp),
        "pagedown"                   => Some(Key::PageDown),
        "up"                         => Some(Key::UpArrow),
        "down"                       => Some(Key::DownArrow),
        "left"                       => Some(Key::LeftArrow),
        "right"                      => Some(Key::RightArrow),
        "f1"  => Some(Key::F1),  "f2"  => Some(Key::F2),
        "f3"  => Some(Key::F3),  "f4"  => Some(Key::F4),
        "f5"  => Some(Key::F5),  "f6"  => Some(Key::F6),
        "f7"  => Some(Key::F7),  "f8"  => Some(Key::F8),
        "f9"  => Some(Key::F9),  "f10" => Some(Key::F10),
        "f11" => Some(Key::F11), "f12" => Some(Key::F12),
        c if c.chars().count() == 1 => Some(Key::Unicode(c.chars().next().unwrap())),
        _ => None,
    }
}

pub async fn key_press(keys: Vec<String>) -> Res {
    let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
    let parsed: Vec<Key> = keys.iter().filter_map(|k| parse_key(k)).collect();
    if parsed.len() != keys.len() {
        return e400(format!("unknown key in: {:?}", keys));
    }
    for &k in &parsed            { if let Err(e) = eg.key(k, Direction::Press)   { return e500(e); } }
    for &k in parsed.iter().rev() { if let Err(e) = eg.key(k, Direction::Release) { return e500(e); } }
    ok()
}

pub async fn type_text(text: String, delay_ms: Option<u64>) -> Res {
    tokio::task::spawn_blocking(move || {
        if let Some(delay) = delay_ms {
            let dur = std::time::Duration::from_millis(delay);
            for ch in text.chars() {
                let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
                if let Err(e) = eg.text(&ch.to_string()) { return e500(e); }
                drop(eg);
                std::thread::sleep(dur);
            }
            ok()
        } else {
            let mut eg = match new_enigo() { Ok(e) => e, Err(e) => return e500(e) };
            match eg.text(&text) { Ok(_) => ok(), Err(e) => e500(e) }
        }
    }).await.unwrap_or_else(|e| e500(e.to_string()))
}

// ── clipboard ─────────────────────────────────────────────────────────────────

pub async fn clipboard_get() -> Res {
    match Clipboard::new() {
        Ok(mut cb) => match cb.get_text() { Ok(t) => (t, None), Err(e) => e500(e) },
        Err(e) => e500(e),
    }
}

pub async fn clipboard_set(text: String) -> Res {
    match Clipboard::new() {
        Ok(mut cb) => match cb.set_text(text) { Ok(_) => ok(), Err(e) => e500(e) },
        Err(e) => e500(e),
    }
}

// ── file ──────────────────────────────────────────────────────────────────────

pub async fn file_read(path: String, encoding: Option<String>) -> Res {
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return e404(format!("{path}: not found")),
        Err(e) => return e500(e),
    };
    match encoding.as_deref().unwrap_or("utf8") {
        "base64" => (B64.encode(&bytes), None),
        _ => match String::from_utf8(bytes) { Ok(s) => (s, None), Err(e) => e500(e) },
    }
}

pub async fn file_write(path: String, content: String, encoding: Option<String>) -> Res {
    let bytes: Vec<u8> = match encoding.as_deref().unwrap_or("utf8") {
        "base64" => match B64.decode(content.trim()) { Ok(b) => b, Err(e) => return e400(e) },
        _ => content.into_bytes(),
    };
    if let Some(parent) = Path::new(&path).parent() && !parent.as_os_str().is_empty()
        && let Err(e) = fs::create_dir_all(parent) { return e500(e); }
    match fs::write(&path, &bytes) { Ok(_) => ok(), Err(e) => e500(e) }
}

#[derive(Serialize)]
struct FileEntry { name: String, is_dir: bool, size: u64 }

pub async fn file_list(path: String, recursive: Option<bool>) -> Res {
    match collect_entries(&path, recursive.unwrap_or(false)) {
        Ok(list) => match serde_json::to_string(&list) { Ok(s) => (s, None), Err(e) => e500(e) },
        Err(r) => r,
    }
}

fn collect_entries(path: &str, recursive: bool) -> Result<Vec<FileEntry>, Res> {
    let rd = fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { e404(format!("{path}: not found")) } else { e500(e) }
    })?;
    let mut list = Vec::new();
    for entry in rd.flatten() {
        let meta = entry.metadata().map_err(e500)?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        list.push(FileEntry { name: name.clone(), is_dir, size: meta.len() });
        if recursive && is_dir {
            let sub = format!("{}/{}", path, name);
            list.extend(collect_entries(&sub, true)?);
        }
    }
    Ok(list)
}

pub async fn file_delete(path: String) -> Res {
    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return e404(path),
        Err(e) => return e500(e),
    };
    let res = if meta.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) };
    match res { Ok(_) => ok(), Err(e) => e500(e) }
}

pub async fn file_move(from: String, to: String) -> Res {
    match fs::rename(&from, &to) {
        Ok(_) => ok(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => e404(format!("{from}: not found")),
        Err(e) => e500(e),
    }
}

pub async fn file_copy(from: String, to: String) -> Res {
    match fs::copy(&from, &to) {
        Ok(_) => ok(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => e404(format!("{from}: not found")),
        Err(e) => e500(e),
    }
}

pub async fn file_exists(path: String) -> Res {
    (Path::new(&path).exists().to_string(), None)
}

// ── app ───────────────────────────────────────────────────────────────────────

pub async fn app_launch(command: String, args: Option<Vec<String>>) -> Res {
    let mut cmd = Command::new(&command);
    if let Some(a) = args { cmd.args(&a); }
    match cmd.spawn() {
        Ok(child) => (child.id().to_string(), None),
        Err(e) => e500(e),
    }
}

pub async fn app_list() -> Res {
    #[cfg(windows)]
    let result = {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command",
                r#"ConvertTo-Json -InputObject @(Get-Process | Where-Object {$_.MainWindowTitle -ne ''} | Select-Object @{N='pid';E={$_.Id}},@{N='name';E={$_.ProcessName}},@{N='title';E={$_.MainWindowTitle}}) -Compress"#])
            .output();
        match out {
            Ok(o) => (String::from_utf8_lossy(&o.stdout).trim().to_string(), None),
            Err(e) => e500(e),
        }
    };
    #[cfg(not(windows))]
    let result = {
        let out = Command::new("ps").args(["-eo", "pid=,comm="]).output();
        match out {
            Ok(o) => {
                let entries: Vec<serde_json::Value> = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|line| {
                        let mut it = line.trim().splitn(2, ' ');
                        let pid: u32 = it.next()?.trim().parse().ok()?;
                        let name = it.next().unwrap_or("").trim().to_string();
                        Some(serde_json::json!({"pid": pid, "name": name, "title": ""}))
                    })
                    .collect();
                match serde_json::to_string(&entries) {
                    Ok(s) => (s, None),
                    Err(e) => e500(e),
                }
            }
            Err(e) => e500(e),
        }
    };
    result
}

pub async fn app_focus(pid: Option<u32>, title: Option<String>) -> Res {
    if pid.is_none() && title.is_none() { return e400("pid or title required"); }

    #[cfg(windows)]
    let result = {
        let selector = if let Some(p) = pid {
            format!("Get-Process -Id {p} -EA SilentlyContinue")
        } else {
            format!("Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{}*' }} | Select-Object -First 1", title.as_deref().unwrap_or(""))
        };
        let script = format!(r#"
$p = {selector}
if (!$p) {{ Write-Output 'E404'; exit }}
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class W32 {{ [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); }}'
[W32]::ShowWindow($p.MainWindowHandle, 9) | Out-Null
[W32]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
Write-Output 'ok'
"#);
        let out = Command::new("powershell").args(["-NoProfile", "-Command", &script]).output();
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s == "E404" { e404("window not found") } else { (s, None) }
            }
            Err(e) => e500(e),
        }
    };
    #[cfg(not(windows))]
    let result = {
        let arg = pid.map(|p| p.to_string()).unwrap_or_else(|| title.unwrap_or_default());
        let out = Command::new("wmctrl").args(["-ia", &arg]).output();
        match out {
            Ok(o) if o.status.success() => ok(),
            Ok(_)  => e404("window not found"),
            Err(e) => e500(e),
        }
    };
    result
}

pub async fn app_close(pid: u32, force: Option<bool>) -> Res {
    let force = force.unwrap_or(false);
    #[cfg(windows)]
    let result = {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string()]);
        if force { cmd.arg("/F"); }
        match cmd.output() {
            Ok(o) if o.status.success() => ok(),
            Ok(o) => e404(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => e500(e),
        }
    };
    #[cfg(not(windows))]
    let result = {
        let sig = if force { "-9" } else { "-15" };
        match Command::new("kill").args([sig, &pid.to_string()]).output() {
            Ok(o) if o.status.success() => ok(),
            Ok(o) => e404(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => e500(e),
        }
    };
    result
}

// ── shell ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ShellResult { stdout: String, stderr: String, exit_code: i32 }

pub async fn shell(cmd: String, timeout_ms: Option<u64>, cwd: Option<String>) -> Res {
    let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30_000));

    #[cfg(windows)]
    let mut builder = {
        let mut c = tokio::process::Command::new("powershell");
        c.args(["-NoProfile", "-Command", &cmd]);
        c
    };
    #[cfg(not(windows))]
    let mut builder = {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", &cmd]);
        c
    };

    if let Some(ref dir) = cwd {
        builder.current_dir(dir);
    }
    builder.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());

    let child = match builder.spawn() {
        Ok(c) => c,
        Err(e) => return e500(e),
    };

    let handle = tokio::spawn(async move { child.wait_with_output().await });

    match tokio::time::timeout(timeout, handle).await {
        Ok(Ok(Ok(out))) => {
            let r = ShellResult {
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                exit_code: out.status.code().unwrap_or(-1),
            };
            match serde_json::to_string(&r) { Ok(s) => (s, None), Err(e) => e500(e) }
        }
        Ok(Ok(Err(e)))  => e500(e),
        Ok(Err(e))      => e500(e),
        Err(_)          => ("E408".to_string(), Some("command timed out".to_string())),
    }
}
