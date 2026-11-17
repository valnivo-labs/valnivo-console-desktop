//! Give control, built into the app (Kabil 17 Nov: "why do we need a helper for this? shouldn't this be automatic?").
//!
//! A web page may not move the mouse or press keys; a native program on the computer may. The app is that program now: while it
//! runs it answers the Console's chat on http://127.0.0.1:47613 with the same small protocol as desktop-helper/
//! valnivo_screen_helper.py (GET /health, POST /act {kind, nx, ny, sw, sh, ...}), so the page needs no change and the separate helper
//! is only the fallback for a browser. Same rules as the helper:
//!   - it listens on 127.0.0.1 only and answers only the Console's own pages (ORIGINS; anything else is refused);
//!   - the Console grants control per chat and sends one action at a time; the agent never talks to this port;
//!   - the mouse in the top-left corner stops it for a minute (the person takes back control at once);
//!   - text is at most 500 characters per action; only named keys and cmd/ctrl/alt/shift; no Windows-key shortcuts;
//!   - on macOS nothing happens until the person allows the app under Privacy & Security > Accessibility (asked once).
//! If the port is taken (the separate helper already runs) the app leaves it to the helper.
use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use serde_json::{json, Value};
use tauri::AppHandle;

pub const PORT: u16 = 47613;
const ORIGINS: &[&str] = &["https://console.valnivo.eu"];
const CORNER: i32 = 4;
const PAUSE: Duration = Duration::from_secs(60);
const TEXT_MAX: usize = 500;
const BODY_MAX: usize = 8192;

/// One display as the page's capture sees it: physical pixels (pw, ph) and where it is in the input coordinate space (x, y, w, h).
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub pw: u32,
    pub ph: u32,
    pub main: bool,
}

/// The shared display: the one whose pixel size matches the capture (sw x sh), else the only one with the same aspect ratio, else
/// the main one (the helper's pick_display).
pub fn pick_display(ds: &[Display], sw: u32, sh: u32) -> Option<Display> {
    if ds.is_empty() {
        return None;
    }
    if sw > 0 && sh > 0 {
        if let Some(d) = ds.iter().find(|d| (d.pw, d.ph) == (sw, sh) || (d.w.round() as u32, d.h.round() as u32) == (sw, sh)) {
            return Some(d.clone());
        }
        let r = sw as f64 / sh as f64;
        let near: Vec<&Display> = ds.iter().filter(|d| d.h > 0.0 && (d.w / d.h - r).abs() < 0.02).collect();
        if near.len() == 1 {
            return Some(near[0].clone());
        }
    }
    Some(ds.iter().find(|d| d.main).unwrap_or(&ds[0]).clone())
}

/// A key name the agent may press. `cmd` is Meta on macOS and Ctrl on Windows (cmd+c copies on both), like the helper.
pub fn key_of(name: &str) -> Option<Key> {
    Some(match name {
        "return" | "enter" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "delete" | "backspace" => Key::Backspace,
        "forwarddelete" => Key::Delete,
        "escape" | "esc" => Key::Escape,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        s if s.chars().count() == 1 => {
            let c = s.chars().next()?;
            if c.is_ascii_alphanumeric() || "=-[]\\;',./`".contains(c) {
                Key::Unicode(c)
            } else {
                return None;
            }
        }
        _ => return None,
    })
}

pub fn mod_of(name: &str) -> Option<Key> {
    Some(match name {
        "shift" => Key::Shift,
        "ctrl" => Key::Control,
        "alt" => Key::Alt,
        "cmd" => {
            if cfg!(target_os = "macos") {
                Key::Meta
            } else {
                Key::Control
            }
        }
        _ => return None,
    })
}

/// What one action asks for, checked before anything moves (pure: tested without a screen).
#[derive(Debug, PartialEq)]
pub enum Plan {
    Click { x: f64, y: f64, right: bool, double: bool },
    Scroll { x: f64, y: f64, dy: i32 },
    Type(String),
    Key { key: String, mods: Vec<String> },
}

pub fn plan(a: &Value, displays: &[Display]) -> Result<Plan, String> {
    let kind = a.get("kind").and_then(Value::as_str).unwrap_or("");
    match kind {
        "click" | "scroll" => {
            let nx = a.get("nx").and_then(Value::as_f64).unwrap_or(-1.0);
            let ny = a.get("ny").and_then(Value::as_f64).unwrap_or(-1.0);
            if !(0.0..=1.0).contains(&nx) || !(0.0..=1.0).contains(&ny) {
                return Err("bad position".into());
            }
            let sw = a.get("sw").and_then(Value::as_u64).unwrap_or(0) as u32;
            let sh = a.get("sh").and_then(Value::as_u64).unwrap_or(0) as u32;
            let d = pick_display(displays, sw, sh).ok_or("no display")?;
            let (mut x, mut y) = (d.x + nx * d.w, d.y + ny * d.h);
            if x <= CORNER as f64 && y <= CORNER as f64 {
                x = CORNER as f64 + 1.0; // never into the stop corner itself
                y = CORNER as f64 + 1.0;
            }
            if kind == "click" {
                Ok(Plan::Click { x, y, right: a.get("button").and_then(Value::as_str) == Some("right"), double: a.get("double") == Some(&Value::Bool(true)) })
            } else {
                let dy = a.get("dy").and_then(Value::as_i64).unwrap_or(0);
                if dy == 0 || dy.abs() > 50 {
                    return Err("bad scroll".into());
                }
                Ok(Plan::Scroll { x, y, dy: dy as i32 })
            }
        }
        "type" => {
            let t = a.get("text").and_then(Value::as_str).unwrap_or("");
            if t.is_empty() || t.chars().count() > TEXT_MAX {
                return Err("bad text".into());
            }
            Ok(Plan::Type(t.to_string()))
        }
        "key" => {
            let k = a.get("key").and_then(Value::as_str).unwrap_or("").to_lowercase();
            if key_of(&k).is_none() {
                return Err("unknown key".into());
            }
            let mods: Vec<String> = match a.get("mods") {
                None | Some(Value::Null) => vec![],
                Some(Value::Array(v)) => v.iter().map(|m| m.as_str().unwrap_or("").to_string()).collect(),
                _ => return Err("bad mods".into()),
            };
            if mods.iter().any(|m| mod_of(m).is_none()) {
                return Err("bad mods".into());
            }
            Ok(Plan::Key { key: k, mods })
        }
        _ => Err("unknown action".into()),
    }
}

/// Is this request from the Console's own page? The exact origin, never a prefix.
pub fn origin_ok(origin: &str) -> Option<String> {
    let o = origin.trim_end_matches('/');
    ORIGINS.iter().find(|x| **x == o).map(|x| x.to_string())
}

// ---------------------------------------------------------------- the computer (macOS / Windows)
#[cfg(target_os = "macos")]
fn trusted(prompt: bool) -> bool {
    if prompt {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
    } else {
        macos_accessibility_client::accessibility::application_is_trusted()
    }
}
#[cfg(not(target_os = "macos"))]
fn trusted(_prompt: bool) -> bool {
    true // Windows has no Accessibility switch (it cannot reach windows run as administrator)
}

/// The displays in the input coordinate space: points on macOS (Core Graphics), physical pixels on Windows (the app is DPI aware).
fn displays(app: &AppHandle) -> Vec<Display> {
    let main = app.primary_monitor().ok().flatten().map(|m| (m.position().x, m.position().y));
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let (p, s, f) = (m.position(), m.size(), m.scale_factor());
            let k = if cfg!(target_os = "macos") { f } else { 1.0 };
            Display { x: p.x as f64 / k, y: p.y as f64 / k, w: s.width as f64 / k, h: s.height as f64 / k, pw: s.width, ph: s.height,
                      main: main == Some((p.x, p.y)) }
        })
        .collect()
}

struct Guard {
    paused_until: Option<Instant>,
}

/// Moves the pointer. On Windows with SetCursorPos in physical pixels (the app is per-monitor DPI aware), which places it right on
/// any monitor of the virtual desktop and at any scaling; elsewhere through enigo (Core Graphics points on macOS).
fn move_to(enigo: &mut Enigo, x: f64, y: f64) -> Result<(), String> {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn SetCursorPos(x: i32, y: i32) -> i32;
        }
        let _ = enigo;
        if unsafe { SetCursorPos(x.round() as i32, y.round() as i32) } == 0 {
            return Err("Windows refused to move the pointer".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        enigo.move_mouse(x.round() as i32, y.round() as i32, Coordinate::Abs).map_err(|x| format!("input: {x:?}"))
    }
}

fn run(enigo: &mut Enigo, p: &Plan) -> Result<(), String> {
    let e = |r: Result<(), enigo::InputError>| r.map_err(|x| format!("input: {x:?}"));
    match p {
        Plan::Click { x, y, right, double } => {
            move_to(enigo, *x, *y)?;
            std::thread::sleep(Duration::from_millis(30));
            let b = if *right { Button::Right } else { Button::Left };
            for _ in 0..(if *double { 2 } else { 1 }) {
                e(enigo.button(b, Direction::Click))?;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Plan::Scroll { x, y, dy } => {
            move_to(enigo, *x, *y)?;
            std::thread::sleep(Duration::from_millis(30));
            e(enigo.scroll(*dy, Axis::Vertical))?;
        }
        Plan::Type(t) => e(enigo.text(t))?,
        Plan::Key { key, mods } => {
            let ms: Vec<Key> = mods.iter().filter_map(|m| mod_of(m)).collect();
            for m in &ms {
                e(enigo.key(*m, Direction::Press))?;
            }
            let r = enigo.key(key_of(key).ok_or("unknown key")?, Direction::Click);
            for m in ms.iter().rev() {
                let _ = enigo.key(*m, Direction::Release);
            }
            e(r)?;
        }
    }
    Ok(())
}

fn respond(req: tiny_http::Request, code: u16, body: Value, origin: Option<&str>, preflight: bool) {
    let mut r = tiny_http::Response::from_string(if preflight { String::new() } else { body.to_string() }).with_status_code(code);
    let h = |k: &str, v: &str| tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("header");
    r.add_header(h("content-type", "application/json"));
    r.add_header(h("cache-control", "no-store"));
    if let Some(o) = origin {
        r.add_header(h("Access-Control-Allow-Origin", o));
        r.add_header(h("Vary", "Origin"));
        if preflight {
            r.add_header(h("Access-Control-Allow-Methods", "GET, POST"));
            r.add_header(h("Access-Control-Allow-Headers", "content-type"));
            r.add_header(h("Access-Control-Allow-Private-Network", "true")); // Chrome / WebView2: a public page asking a local address
            r.add_header(h("Access-Control-Max-Age", "600"));
        }
    }
    let _ = req.respond(r);
}

/// Starts the control service in its own thread. Returns false when the port is taken (the separate helper runs).
pub fn start(app: AppHandle) -> bool {
    let port = std::env::var("VALNIVO_HELPER_PORT").ok().and_then(|v| v.parse::<u16>().ok()).unwrap_or(PORT);   // tests; the page uses 47613
    serve(port, move || displays(&app))
}

/// The service itself, with the screens from `screens` (the app's monitors; fixed ones in tests).
pub fn serve(port: u16, screens: impl Fn() -> Vec<Display> + Send + 'static) -> bool {
    let server = match tiny_http::Server::http(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    std::thread::spawn(move || {
        let guard = Mutex::new(Guard { paused_until: None });
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => Some(e),
            Err(_) => None,
        };
        let mut asked = false;
        for mut req in server.incoming_requests() {
            let origin = req.headers().iter().find(|h| h.field.equiv("Origin")).map(|h| h.value.as_str().to_string()).unwrap_or_default();
            let ok_origin = origin_ok(&origin);
            let (method, path) = (req.method().clone(), req.url().to_string());
            if method == tiny_http::Method::Options {
                match ok_origin {
                    Some(o) => respond(req, 204, json!({}), Some(&o), true),
                    None => respond(req, 403, json!({"e": "origin"}), None, false),
                }
                continue;
            }
            let Some(o) = ok_origin else {
                respond(req, 403, json!({"e": "origin"}), None, false);
                continue;
            };
            match (method, path.as_str()) {
                (tiny_http::Method::Get, "/health") => {
                    // the first look from the page asks macOS for Accessibility once (the person sees the system prompt)
                    let t = trusted(!asked);
                    asked = true;
                    let os = if cfg!(target_os = "macos") { "macos" } else { "windows" };
                    respond(req, 200, json!({"ok": 1, "accessibility": t, "displays": screens().len(), "v": 1, "os": os, "app": 1}), Some(&o), false);
                }
                (tiny_http::Method::Post, "/act") => {
                    let mut body = String::new();
                    let n = req.body_length().unwrap_or(0);
                    if n == 0 || n > BODY_MAX || req.as_reader().take(BODY_MAX as u64).read_to_string(&mut body).is_err() {
                        respond(req, 400, json!({"e": "body"}), Some(&o), false);
                        continue;
                    }
                    let Ok(a) = serde_json::from_str::<Value>(&body) else {
                        respond(req, 400, json!({"e": "json"}), Some(&o), false);
                        continue;
                    };
                    let res = (|| -> Result<(), String> {
                        let en = enigo.as_mut().ok_or("input is not available on this computer")?;
                        let mut g = guard.lock().map_err(|_| "busy")?;
                        let (mx, my) = en.location().unwrap_or((100, 100));
                        if mx <= CORNER && my <= CORNER {
                            g.paused_until = Some(Instant::now() + PAUSE);
                            return Err("stopped: the person pushed the mouse into the top-left corner".into());
                        }
                        if g.paused_until.is_some_and(|t| Instant::now() < t) {
                            return Err("paused for a minute after the person stopped it".into());
                        }
                        if !trusted(false) {
                            return Err("this computer has not allowed Valnivo Crew Accessibility (System Settings > Privacy & Security > Accessibility)".into());
                        }
                        let p = plan(&a, &screens())?;
                        run(en, &p)
                    })();
                    match res {
                        Ok(()) => respond(req, 200, json!({"ok": true}), Some(&o), false),
                        Err(why) => respond(req, 200, json!({"ok": false, "why": why}), Some(&o), false),
                    }
                }
                _ => respond(req, 404, json!({"e": "nf"}), Some(&o), false),
            }
        }
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http(port: u16, method: &str, path: &str, origin: &str, body: &str) -> (u16, String, String) {
        use std::io::Write;
        let mut c = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {origin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        c.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        let code = out.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        let (head, b) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
        (code, head.to_lowercase(), b.to_string())
    }

    #[test]
    fn the_service_answers_only_the_console() {
        let port = 47_690;
        assert!(serve(port, || vec![Display { x: 0.0, y: 0.0, w: 100.0, h: 100.0, pw: 100, ph: 100, main: true }]));
        assert!(!serve(port, Vec::new), "a taken port is left to whoever has it (the separate helper)");
        let (c, _, b) = http(port, "GET", "/health", "https://console.valnivo.eu", "");
        assert_eq!(c, 200);
        assert!(b.contains("\"app\":1") && b.contains("\"displays\":1"), "{b}");
        assert_eq!(http(port, "GET", "/health", "https://evil.example", "").0, 403, "another site is refused");
        let (pc, ph, _) = http(port, "OPTIONS", "/act", "https://console.valnivo.eu", "");
        assert_eq!(pc, 204);
        assert!(ph.contains("access-control-allow-private-network: true"), "Chrome's private network preflight");
        let (ac, _, ab) = http(port, "POST", "/act", "https://console.valnivo.eu", "{\"kind\":\"launch\"}");
        assert_eq!(ac, 200);
        assert!(ab.contains("\"ok\":false"), "an unknown action is refused, nothing moves: {ab}");
        assert_eq!(http(port, "POST", "/act", "https://console.valnivo.eu", "not json").0, 400);
    }

    fn d(x: f64, y: f64, w: f64, h: f64, pw: u32, ph: u32, main: bool) -> Display {
        Display { x, y, w, h, pw, ph, main }
    }

    #[test]
    fn origin_is_exact() {
        assert_eq!(origin_ok("https://console.valnivo.eu/").as_deref(), Some("https://console.valnivo.eu"));
        assert_eq!(origin_ok("https://console.valnivo.eu.evil.com"), None);
        assert_eq!(origin_ok("http://console.valnivo.eu"), None);
        assert_eq!(origin_ok(""), None);
    }

    #[test]
    fn picks_the_shared_screen() {
        let ds = [d(0.0, 0.0, 1512.0, 982.0, 3024, 1964, true), d(1512.0, 0.0, 2560.0, 1440.0, 2560, 1440, false)];
        assert_eq!(pick_display(&ds, 2560, 1440).unwrap().x, 1512.0, "by pixel size");
        assert!(pick_display(&ds, 3024, 1964).unwrap().main);
        assert!(pick_display(&ds, 0, 0).unwrap().main, "unknown size: the main one");
        assert!(pick_display(&[], 1, 1).is_none());
    }

    #[test]
    fn plans_a_click_on_the_right_screen_and_never_in_the_corner() {
        let ds = [d(0.0, 0.0, 1000.0, 500.0, 2000, 1000, true), d(1000.0, 0.0, 1920.0, 1080.0, 1920, 1080, false)];
        let p = plan(&json!({"kind": "click", "nx": 0.5, "ny": 0.5, "sw": 1920, "sh": 1080}), &ds).unwrap();
        assert_eq!(p, Plan::Click { x: 1960.0, y: 540.0, right: false, double: false });
        let c = plan(&json!({"kind": "click", "nx": 0.0, "ny": 0.0, "sw": 2000, "sh": 1000}), &ds).unwrap();
        assert_eq!(c, Plan::Click { x: 5.0, y: 5.0, right: false, double: false });
        assert!(plan(&json!({"kind": "click", "nx": 1.5, "ny": 0.2}), &ds).is_err());
    }

    #[test]
    fn checks_scroll_text_and_keys() {
        let ds = [d(0.0, 0.0, 100.0, 100.0, 100, 100, true)];
        assert!(plan(&json!({"kind": "scroll", "nx": 0.5, "ny": 0.5, "dy": 0}), &ds).is_err());
        assert!(plan(&json!({"kind": "scroll", "nx": 0.5, "ny": 0.5, "dy": 99}), &ds).is_err());
        assert_eq!(plan(&json!({"kind": "type", "text": "héllo"}), &ds).unwrap(), Plan::Type("héllo".into()));
        assert!(plan(&json!({"kind": "type", "text": "x".repeat(501)}), &ds).is_err());
        assert_eq!(plan(&json!({"kind": "key", "key": "C", "mods": ["cmd"]}), &ds).unwrap(), Plan::Key { key: "c".into(), mods: vec!["cmd".into()] });
        assert!(plan(&json!({"kind": "key", "key": "f13"}), &ds).is_err(), "only named keys");
        assert!(plan(&json!({"kind": "key", "key": "a", "mods": ["win"]}), &ds).is_err(), "no Windows-key shortcuts");
        assert!(plan(&json!({"kind": "launch"}), &ds).is_err());
    }
}
