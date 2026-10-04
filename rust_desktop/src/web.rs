//! Single place that builds the browser WebView and talks to WebView2 directly.
//!
//! * `build_webview` replaces the three copy-pasted `WebViewBuilder` chains that used to live in `main.rs`.
//! * `com` (Windows only) hooks the real WebView2 network layer for the AdBlock / Privacy shield, manages the
//!   document-start extension script, drives back/forward, DevTools-protocol screenshots and memory level.
//!
//! Commands coming from pages use the `axomai://` scheme. A page that is not ours must not be able to drive the
//! browser (`axomai://exit`, ...), so a command is accepted only when it carries the per-run token that our own
//! overlays embed, or when the current top-level document is one of our internal pages.

use crate::blocklist;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tao::window::Window;
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};

pub enum Initial<'a> {
    Url(&'a str),
    Html(&'a str),
}

#[derive(Clone, Debug)]
pub enum WebEvent {
    LoadStarted(String),
    LoadFinished(String),
    Title(String),
    /// A page asked for a new window (`target=_blank`, `window.open`); we open it in the current view.
    OpenUrl(String),
    /// Result of a screenshot: Ok(path) or Err(message).
    Captured(Result<String, String>),
    /// Text / data read back from the page for a pending request: (kind, question, payload).
    PageData(String, String, String),
}

/// Live counters + switches read by the network hook.
pub struct Shield {
    pub adblock: AtomicBool,
    pub privacy: AtomicBool,
    pub page_blocked: AtomicU32,
    pub total_blocked: AtomicU64,
    pub page_host: Mutex<String>,
}

impl Shield {
    pub fn new() -> Self {
        Shield {
            adblock: AtomicBool::new(true),
            privacy: AtomicBool::new(true),
            page_blocked: AtomicU32::new(0),
            total_blocked: AtomicU64::new(0),
            page_host: Mutex::new(String::new()),
        }
    }
}

#[derive(Clone)]
pub struct WebShared {
    pub nav: Arc<Mutex<Vec<String>>>,
    pub events: Arc<Mutex<Vec<WebEvent>>>,
    pub downloads: Arc<Mutex<Vec<(String, String, bool)>>>,
    pub download_dir: PathBuf,
    /// True while the top-level document is one of our own pages (data:, about:, file under `ui_prefix`).
    pub trusted: Arc<AtomicBool>,
    pub token: Arc<String>,
    pub ui_prefix: Arc<String>,
    pub shield: Arc<Shield>,
}

impl WebShared {
    pub fn new(download_dir: PathBuf, ui_dir: &std::path::Path) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1);
        let mixed = nanos ^ ((std::process::id() as u128) << 64) ^ 0x9E37_79B9_7F4A_7C15_u128.wrapping_mul(nanos | 1);
        WebShared {
            nav: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            downloads: Arc::new(Mutex::new(Vec::new())),
            download_dir,
            trusted: Arc::new(AtomicBool::new(true)),
            token: Arc::new(format!("{:032x}", mixed)),
            ui_prefix: Arc::new(file_url_prefix(ui_dir)),
            shield: Arc::new(Shield::new()),
        }
    }

    pub fn push_event(&self, e: WebEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push(e);
        }
    }

    pub fn drain_events(&self) -> Vec<WebEvent> {
        self.events.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }

    pub fn drain_nav(&self) -> Vec<String> {
        self.nav.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }
}

pub fn file_url_prefix(dir: &std::path::Path) -> String {
    format!("file:///{}", dir.to_string_lossy().replace('\\', "/")).to_ascii_lowercase()
}

/// Is this URL one of the browser's own pages?
pub fn is_internal_url(url: &str, ui_prefix: &str) -> bool {
    let l = url.to_ascii_lowercase();
    l.starts_with("data:") || l.starts_with("about:") || (l.starts_with("file:") && l.starts_with(ui_prefix))
}

/// Decide what the navigation handler does with a URL.
/// Returns (allow_navigation, command_to_enqueue).
pub fn route_navigation(url: &str, token: &str, trusted_now: bool, ui_prefix: &str) -> (bool, Option<String>, Option<bool>) {
    if let Some(rest) = url.strip_prefix("axomai://") {
        let prefix = format!("{}/", token);
        if let Some(cmd) = rest.strip_prefix(&prefix) {
            return (false, Some(format!("axomai://{}", cmd)), None);
        }
        if trusted_now {
            return (false, Some(url.to_string()), None);
        }
        return (false, None, None);
    }
    (true, Some(url.to_string()), Some(is_internal_url(url, ui_prefix)))
}

pub fn build_webview(
    window: &Window,
    size_px: (f32, f32),
    top: f32,
    initial: Initial,
    shared: &WebShared,
    bg: (u8, u8, u8, u8),
) -> Option<WebView> {
    let nav_shared = shared.clone();
    let title_shared = shared.clone();
    let load_shared = shared.clone();
    let window_shared = shared.clone();
    let dl_dir = shared.download_dir.clone();
    let dl_sig = shared.downloads.clone();
    let dl_started = shared.clone();

    let builder = WebViewBuilder::new();
    let builder = match initial {
        Initial::Url(u) => builder.with_url(u),
        Initial::Html(h) => builder.with_html(h),
    };
    let wv = builder
        .with_devtools(true)
        .with_background_color(bg)
        .with_bounds(Rect {
            position: wry::dpi::PhysicalPosition::new(crate::types::SIDEBAR_W as i32, top as i32).into(),
            size: wry::dpi::PhysicalSize::new(
                (size_px.0 - crate::types::SIDEBAR_W).max(1.0) as u32,
                (size_px.1 - top).max(1.0) as u32,
            )
            .into(),
        })
        .with_navigation_handler(move |url: String| {
            let trusted_now = nav_shared.trusted.load(Ordering::SeqCst);
            let (allow, cmd, new_trust) = route_navigation(&url, &nav_shared.token, trusted_now, &nav_shared.ui_prefix);
            if let Some(t) = new_trust {
                nav_shared.trusted.store(t, Ordering::SeqCst);
                if url.starts_with("http://") || url.starts_with("https://") {
                    if let Ok(mut h) = nav_shared.shield.page_host.lock() {
                        *h = blocklist::host_of(&url);
                    }
                    nav_shared.shield.page_blocked.store(0, Ordering::SeqCst);
                }
            }
            if let Some(c) = cmd {
                if let Ok(mut q) = nav_shared.nav.lock() {
                    q.push(c);
                }
            }
            allow
        })
        .with_new_window_req_handler(move |url: String| {
            // `target=_blank` and `window.open` open a new tab; the context-menu "View page source" asks for `view-source:`.
            if url.starts_with("http://") || url.starts_with("https://") {
                window_shared.push_event(WebEvent::OpenUrl(url));
            } else if let Some(target) = url.strip_prefix("view-source:") {
                if let Ok(mut q) = window_shared.nav.lock() {
                    q.push(format!("axomai://viewsource/{}", crate::viewsource::encode(target)));
                }
            }
            false
        })
        .with_document_title_changed_handler(move |t: String| title_shared.push_event(WebEvent::Title(t)))
        .with_on_page_load_handler(move |ev, url| match ev {
            PageLoadEvent::Started => load_shared.push_event(WebEvent::LoadStarted(url)),
            PageLoadEvent::Finished => load_shared.push_event(WebEvent::LoadFinished(url)),
        })
        .with_download_started_handler(move |url, path| {
            let fname = url.rsplit('/').next().unwrap_or("download").split('?').next().unwrap_or("download");
            let fname = if fname.is_empty() { "download" } else { fname };
            *path = dl_dir.join(fname);
            // The navigation became a download, so no page will ever report that it finished loading.
            dl_started.push_event(WebEvent::LoadFinished(url));
            true
        })
        .with_download_completed_handler(move |url, path, success| {
            let fname = path
                .as_ref()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .unwrap_or_default();
            if let Ok(mut sig) = dl_sig.lock() {
                sig.push((url, fname, success));
            }
        })
        .build_as_child(window)
        .ok()?;

    #[cfg(windows)]
    {
        com::install_network_shield(&wv, shared.shield.clone());
        com::install_message_bridge(&wv, shared.clone());
        com::install_accelerators(&wv, shared.clone());
        com::install_context_menu(&wv, shared.clone());
    }
    Some(wv)
}

#[cfg(windows)]
pub mod com {
    use super::*;
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{
        take_pwstr, AddScriptToExecuteOnDocumentCreatedCompletedHandler, CallDevToolsProtocolMethodCompletedHandler,
        AcceleratorKeyPressedEventHandler, ContextMenuRequestedEventHandler, CustomItemSelectedEventHandler, WebMessageReceivedEventHandler, WebResourceRequestedEventHandler,
    };
    use windows::core::{w, Interface, HSTRING, PWSTR};
    use windows::Win32::Foundation::BOOL;
    use wry::WebViewExtWindows;

    fn core(wv: &WebView) -> Option<ICoreWebView2> {
        unsafe { wv.controller().CoreWebView2().ok() }
    }

    /// Block ad / tracker requests at the WebView2 network layer (nothing is downloaded).
    pub fn install_network_shield(wv: &WebView, shield: Arc<Shield>) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let _ = core.AddWebResourceRequestedFilter(w!("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
            let Ok(env) = core.cast::<ICoreWebView2_2>().and_then(|c| c.Environment()) else { return };
            let handler = WebResourceRequestedEventHandler::create(Box::new(move |_sender, args| {
                let Some(args) = args else { return Ok(()) };
                let request = args.Request()?;
                let mut uri = PWSTR::null();
                request.Uri(&mut uri)?;
                let url = take_pwstr(uri);
                let page_host = shield.page_host.lock().map(|h| h.clone()).unwrap_or_default();
                if let Some(kind) = blocklist::classify(&url, &page_host) {
                    let enabled = match kind {
                        blocklist::Kind::Ad => shield.adblock.load(Ordering::Relaxed),
                        blocklist::Kind::Tracker => shield.privacy.load(Ordering::Relaxed),
                    };
                    if enabled {
                        shield.page_blocked.fetch_add(1, Ordering::Relaxed);
                        shield.total_blocked.fetch_add(1, Ordering::Relaxed);
                        let response = env.CreateWebResourceResponse(None, 403, w!("Blocked by Axomai Shield"), w!("Content-Type: text/plain"))?;
                        args.SetResponse(&response)?;
                    }
                }
                Ok(())
            }));
            let mut token = std::mem::zeroed();
            let _ = core.add_WebResourceRequested(&handler, &mut token);
        }
    }

    /// Receive `chrome.webview.postMessage('<token>/<command>')` from our popups.
    ///
    /// This deliberately bypasses wry's `with_ipc_handler`: wry turns the sender's URL into an `http::Uri` and
    /// unwraps it, which panics (and aborts the process) for `file:///E:/...` and `data:` pages, i.e. our own
    /// home page and generated pages. Messages without the per-run token (anything sent by a web page) are dropped.
    pub fn install_message_bridge(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_sender, args| {
            let Some(args) = args else { return Ok(()) };
            let mut raw = PWSTR::null();
            let body = unsafe {
                if args.TryGetWebMessageAsString(&mut raw).is_err() {
                    return Ok(());
                }
                take_pwstr(raw)
            };
            if let Some(cmd) = body.strip_prefix(&format!("{}/", shared.token)) {
                if let Ok(mut q) = shared.nav.lock() {
                    q.push(format!("axomai://{}", cmd));
                }
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = core.add_WebMessageReceived(&handler, &mut token);
        }
    }

    /// WebView2's own right-click menu has no "View page source", so add one just above "Inspect".
    pub fn install_context_menu(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(menu_core) = core.cast::<ICoreWebView2_11>() else { return };
            let Ok(env) = core.cast::<ICoreWebView2_2>().and_then(|c| c.Environment()) else { return };
            let Ok(env9) = env.cast::<ICoreWebView2Environment9>() else { return };
            let handler = ContextMenuRequestedEventHandler::create(Box::new(move |_sender, args| {
                let Some(args) = args else { return Ok(()) };
                let items = args.MenuItems()?;
                let mut count = 0u32;
                items.Count(&mut count)?;
                let item = env9.CreateContextMenuItem(w!("View page source"), None, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_COMMAND)?;
                let click_shared = shared.clone();
                let selected = CustomItemSelectedEventHandler::create(Box::new(move |_s, _a| {
                    if let Ok(mut q) = click_shared.nav.lock() {
                        q.push(String::from("axomai://viewsource-current"));
                    }
                    Ok(())
                }));
                let mut token = std::mem::zeroed();
                item.add_CustomItemSelected(&selected, &mut token)?;
                // Above the last entry (Inspect); if the menu is tiny, just append.
                items.InsertValueAtIndex(count.saturating_sub(1), &item)?;
                Ok(())
            }));
            let mut token = std::mem::zeroed();
            let _ = menu_core.add_ContextMenuRequested(&handler, &mut token);
        }
    }

    /// Keyboard shortcuts while the page has focus. WebView2 keeps keyboard focus once you click inside a page, so
    /// without this Ctrl+T / Ctrl+W / Ctrl+L / Ctrl+U / Ctrl+D / Ctrl+H / Ctrl+J would only work from the toolbar.
    /// F12 and Ctrl+Shift+I (DevTools) are left to WebView2, which handles them itself.
    pub fn install_accelerators(wv: &WebView, shared: WebShared) {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
        let controller = wv.controller();
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_sender, args| {
            let Some(args) = args else { return Ok(()) };
            unsafe {
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND(0);
                args.KeyEventKind(&mut kind)?;
                // Only plain key-down / system-key-down events; ignore key-up so a shortcut fires once.
                if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
                    return Ok(());
                }
                let mut vk = 0u32;
                args.VirtualKey(&mut vk)?;
                let down = |code: i32| (GetKeyState(code) as u16 & 0x8000) != 0;
                let (ctrl, shift, alt) = (down(0x11), down(0x10), down(0x12));
                if !ctrl || shift || alt {
                    return Ok(());
                }
                let command = match vk {
                    0x55 => "viewsource-current", // U
                    0x54 => "newtab",             // T
                    0x57 => "closetab",           // W
                    0x4C => "focus-url",          // L
                    0x48 => "history",            // H
                    0x4A => "downloads",          // J
                    0x44 => "bookmark-toggle",    // D
                    _ => return Ok(()),
                };
                if let Ok(mut q) = shared.nav.lock() {
                    q.push(format!("axomai://{}", command));
                }
                args.SetHandled(true)?;
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = controller.add_AcceleratorKeyPressed(&handler, &mut token);
        }
    }

    /// Script that WebView2 runs at the start of every document; replaceable at runtime.
    #[derive(Clone, Default)]
    pub struct DocScript {
        state: Arc<Mutex<(u64, Option<String>)>>,
    }

    impl DocScript {
        pub fn set(&self, wv: &WebView, js: &str) {
            let Some(core) = core(wv) else { return };
            let generation = {
                let Ok(mut st) = self.state.lock() else { return };
                if let Some(old) = st.1.take() {
                    unsafe {
                        let _ = core.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(old));
                    }
                }
                st.0 += 1;
                st.0
            };
            if js.trim().is_empty() {
                return;
            }
            let state = self.state.clone();
            let core_for_cb = core.clone();
            let handler = AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(move |res, id| {
                if res.is_ok() {
                    if let Ok(mut st) = state.lock() {
                        if st.0 == generation {
                            st.1 = Some(id);
                        } else {
                            unsafe {
                                let _ = core_for_cb.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
                            }
                        }
                    }
                }
                Ok(())
            }));
            unsafe {
                let _ = core.AddScriptToExecuteOnDocumentCreated(&HSTRING::from(js), &handler);
            }
        }
    }

    pub fn can_go(wv: &WebView) -> (bool, bool) {
        let Some(core) = core(wv) else { return (false, false) };
        unsafe {
            let (mut b, mut f) = (BOOL(0), BOOL(0));
            let _ = core.CanGoBack(&mut b);
            let _ = core.CanGoForward(&mut f);
            (b.as_bool(), f.as_bool())
        }
    }

    pub fn go_back(wv: &WebView) {
        if let Some(c) = core(wv) {
            unsafe {
                let _ = c.GoBack();
            }
        }
    }

    pub fn go_forward(wv: &WebView) {
        if let Some(c) = core(wv) {
            unsafe {
                let _ = c.GoForward();
            }
        }
    }

    pub fn reload(wv: &WebView) {
        if let Some(c) = core(wv) {
            unsafe {
                let _ = c.Reload();
            }
        }
    }

    /// Reload ignoring the HTTP cache, so resources cached before a shield was switched on are fetched (and blocked) again.
    pub fn reload_hard(wv: &WebView) {
        let Some(core) = core(wv) else { return };
        let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_res, _json| Ok(())));
        unsafe {
            let _ = core.CallDevToolsProtocolMethod(&HSTRING::from("Page.reload"), &HSTRING::from(r#"{"ignoreCache":true}"#), &handler);
        }
    }

    pub fn set_memory_low(wv: &WebView, low: bool) {
        use wry::MemoryUsageLevel;
        let _ = wv.set_memory_usage_level(if low { MemoryUsageLevel::Low } else { MemoryUsageLevel::Normal });
    }

    /// Screenshot through the DevTools protocol. `params` is the JSON for `Page.captureScreenshot`.
    /// The PNG is written to `out` and the result is reported through `shared.events`.
    pub fn capture_png(wv: &WebView, params: &str, out: PathBuf, shared: &WebShared) {
        let Some(core) = core(wv) else {
            shared.push_event(WebEvent::Captured(Err("WebView is not ready".into())));
            return;
        };
        let events = shared.clone();
        let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |res, json| {
            let result = (|| -> Result<String, String> {
                res.map_err(|e| e.to_string())?;
                let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                let b64 = v.get("data").and_then(|d| d.as_str()).ok_or("no image data returned")?;
                let bytes = crate::sys::base64_decode(b64).ok_or("invalid image data")?;
                if let Some(dir) = out.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
                Ok(out.to_string_lossy().to_string())
            })();
            events.push_event(WebEvent::Captured(result));
            Ok(())
        }));
        unsafe {
            let _ = core.CallDevToolsProtocolMethod(&HSTRING::from("Page.captureScreenshot"), &HSTRING::from(params), &handler);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UI: &str = "file:///e:/axomai-browser/ui";

    #[test]
    fn untrusted_page_cannot_issue_commands() {
        let (allow, cmd, trust) = route_navigation("axomai://exit", "tok", false, UI);
        assert!(!allow);
        assert!(cmd.is_none());
        assert!(trust.is_none());
    }

    #[test]
    fn token_unlocks_command_from_remote_overlay() {
        let (allow, cmd, _) = route_navigation("axomai://tok/ext-toggle/2", "tok", false, UI);
        assert!(!allow);
        assert_eq!(cmd.as_deref(), Some("axomai://ext-toggle/2"));
    }

    #[test]
    fn internal_page_can_issue_plain_commands() {
        let (allow, cmd, _) = route_navigation("axomai://settings", "tok", true, UI);
        assert!(!allow);
        assert_eq!(cmd.as_deref(), Some("axomai://settings"));
    }

    #[test]
    fn navigation_updates_trust() {
        let (_, _, t) = route_navigation("https://example.com/", "tok", true, UI);
        assert_eq!(t, Some(false));
        let (_, _, t) = route_navigation("file:///E:/axomai-browser/ui/home.html", "tok", false, UI);
        assert_eq!(t, Some(true));
        let (_, _, t) = route_navigation("data:text/html;charset=utf-8,%3Cp%3E", "tok", false, UI);
        assert_eq!(t, Some(true));
        let (_, _, t) = route_navigation("file:///C:/Users/x/secret.html", "tok", true, UI);
        assert_eq!(t, Some(false));
    }
}

/// Non-Windows builds: the WebView2-only features are unavailable, so these are inert.
#[cfg(not(windows))]
pub mod com {
    use super::*;

    #[derive(Clone, Default)]
    pub struct DocScript;

    impl DocScript {
        pub fn set(&self, _wv: &WebView, _js: &str) {}
    }

    pub fn can_go(_wv: &WebView) -> (bool, bool) {
        (false, false)
    }
    pub fn go_back(wv: &WebView) {
        let _ = wv.evaluate_script("history.back()");
    }
    pub fn go_forward(wv: &WebView) {
        let _ = wv.evaluate_script("history.forward()");
    }
    pub fn reload(wv: &WebView) {
        let _ = wv.evaluate_script("location.reload()");
    }
    pub fn reload_hard(wv: &WebView) {
        let _ = wv.evaluate_script("location.reload(true)");
    }
    pub fn set_memory_low(_wv: &WebView, _low: bool) {}
    pub fn capture_png(_wv: &WebView, _params: &str, _out: PathBuf, shared: &WebShared) {
        shared.push_event(WebEvent::Captured(Err("Screenshots need the Windows WebView2 runtime".into())));
    }
}

#[cfg(test)]
mod wry_quirks {
    /// Regression guard: wry's ipc handler unwraps `Uri` parsing of the sender URL. These pages fail to parse,
    /// so `with_ipc_handler` must never be used here (see `com::install_message_bridge`).
    #[test]
    fn our_own_pages_are_not_valid_http_uris() {
        assert!("file:///E:/axomai-browser/ui/home.html".parse::<wry::http::Uri>().is_err());
        assert!("data:text/html;charset=utf-8,<html>".parse::<wry::http::Uri>().is_err());
    }
}
