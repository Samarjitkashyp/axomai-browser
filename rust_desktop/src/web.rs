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

/// A file download, identified by a process-wide id.
#[derive(Clone, Debug)]
pub enum DlEvent {
    Started { id: u64, url: String, path: PathBuf, total: i64 },
    Progress { id: u64, received: i64, total: i64 },
    Paused { id: u64, paused: bool },
    Done { id: u64 },
    Failed { id: u64, reason: String },
}

/// `dir/name`, or `dir/name (1).ext`, `(2)` ... when that file already exists.
pub fn unique_path(dir: &std::path::Path, name: &str) -> PathBuf {
    let name = name.trim();
    let name = if name.is_empty() { "download" } else { name };
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = std::path::Path::new(name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| name.to_string());
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (1..10_000).map(|n| dir.join(format!("{} ({}){}", stem, n, ext))).find(|c| !c.exists()).unwrap_or(first)
}

/// Things a web view reports. Every event is tagged with the id of the tab (web view) that produced it.
#[derive(Clone, Debug)]
pub enum WebEvent {
    LoadStarted(String),
    LoadFinished(String),
    Title(String),
    /// Top-level navigation to an ordinary web address (not an `axomai://` command).
    Navigating(String),
    /// A page asked for a new window (`target=_blank`, `window.open`); it opens in a new tab.
    OpenUrl(String),
    /// Result of a screenshot: Ok(path) or Err(message).
    Captured(Result<String, String>),
    /// Text / data read back from the page for a pending request: (kind, question, payload).
    PageData(String, String, String),
    /// New favicon image (PNG/JPEG bytes) for the page.
    Favicon(Vec<u8>),
    /// The page started / stopped playing audio.
    Audio(bool),
    /// Progress of a file download started by this tab.
    Download(DlEvent),
    /// A JSON message posted by a web page: (address of the sending document, raw JSON).
    PageMsg(String, String),
    /// A page wants a permission: (request id, address of the page, permission name).
    PermissionAsk { id: u64, uri: String, name: String },
    /// HTTPS-only mode cancelled this http:// navigation; load this https:// address instead.
    Upgrade(String),
    /// The address belongs to a blocked site; show the block page instead.
    SiteBlocked(String),
    /// The https:// address we upgraded to did not load; this is the original http:// address.
    UpgradeFailed(String),
    /// The page zoom changed (factor, 1.0 = 100%).
    Zoom(f64),
    /// A page entered (true) or left (false) its own full-screen mode.
    HtmlFullscreen(bool),
}

/// Live counters + switches read by the network hook. The switches and the lifetime total are shared by every
/// tab; the per-page numbers belong to one tab.
pub struct Shield {
    pub adblock: Arc<AtomicBool>,
    pub privacy: Arc<AtomicBool>,
    pub total_blocked: Arc<AtomicU64>,
    pub page_blocked: AtomicU32,
    pub page_host: Mutex<String>,
    /// Switches shared by every tab.
    pub https_only: Arc<AtomicBool>,
    pub gpc: Arc<AtomicBool>,
    /// Hosts the user chose to open over plain http for this session, and https addresses we upgraded to.
    pub https_exempt: Arc<Mutex<std::collections::HashSet<String>>>,
    pub upgraded: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Sites the user blocked; opening one shows the block page instead.
    pub blocked_sites: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl Shield {
    pub fn new() -> Self {
        Shield {
            adblock: Arc::new(AtomicBool::new(true)),
            privacy: Arc::new(AtomicBool::new(true)),
            total_blocked: Arc::new(AtomicU64::new(0)),
            page_blocked: AtomicU32::new(0),
            page_host: Mutex::new(String::new()),
            https_only: Arc::new(AtomicBool::new(false)),
            gpc: Arc::new(AtomicBool::new(true)),
            https_exempt: Default::default(),
            upgraded: Default::default(),
            blocked_sites: Default::default(),
        }
    }

    /// A shield for another tab: same switches and total, fresh per-page numbers.
    pub fn for_tab(&self) -> Shield {
        Shield {
            adblock: self.adblock.clone(),
            privacy: self.privacy.clone(),
            total_blocked: self.total_blocked.clone(),
            page_blocked: AtomicU32::new(0),
            page_host: Mutex::new(String::new()),
            https_only: self.https_only.clone(),
            gpc: self.gpc.clone(),
            https_exempt: self.https_exempt.clone(),
            upgraded: self.upgraded.clone(),
            blocked_sites: self.blocked_sites.clone(),
        }
    }
}

#[derive(Clone)]
pub struct WebShared {
    /// `axomai://` commands from popups and our own pages (shared by all tabs).
    pub nav: Arc<Mutex<Vec<String>>>,
    pub events: Arc<Mutex<Vec<(u64, WebEvent)>>>,
    /// Where downloads are saved; the Settings page can change it while the browser runs.
    pub download_dir: Arc<Mutex<PathBuf>>,
    /// True while this tab's top-level document is one of our own pages (data:, about:, file under `ui_prefix`).
    pub trusted: Arc<AtomicBool>,
    pub token: Arc<String>,
    pub ui_prefix: Arc<String>,
    pub shield: Arc<Shield>,
    pub tab_id: u64,
    pub private: bool,
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
            download_dir: Arc::new(Mutex::new(download_dir)),
            trusted: Arc::new(AtomicBool::new(true)),
            token: Arc::new(format!("{:032x}", mixed)),
            ui_prefix: Arc::new(file_url_prefix(ui_dir)),
            shield: Arc::new(Shield::new()),
            tab_id: 0,
            private: false,
        }
    }

    /// The handle one tab's web view uses: shared queues and switches, but its own trust flag and page counters.
    pub fn for_tab(&self, tab_id: u64, private: bool) -> WebShared {
        WebShared {
            trusted: Arc::new(AtomicBool::new(true)),
            shield: Arc::new(self.shield.for_tab()),
            tab_id,
            private,
            ..self.clone()
        }
    }

    pub fn download_dir(&self) -> PathBuf {
        self.download_dir.lock().map(|d| d.clone()).unwrap_or_default()
    }

    pub fn set_download_dir(&self, dir: PathBuf) {
        if let Ok(mut d) = self.download_dir.lock() {
            *d = dir;
        }
    }

    pub fn push_event(&self, e: WebEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push((self.tab_id, e));
        }
    }

    pub fn drain_events(&self) -> Vec<(u64, WebEvent)> {
        self.events.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }

    pub fn drain_nav(&self) -> Vec<String> {
        self.nav.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }

    pub fn push_command(&self, cmd: &str) {
        if let Ok(mut q) = self.nav.lock() {
            q.push(format!("axomai://{}", cmd));
        }
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

/// An address in the form used to compare "the page we asked for" with "the page that failed".
pub fn norm_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
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

    let builder = WebViewBuilder::new();
    let builder = match initial {
        Initial::Url(u) => builder.with_url(u),
        Initial::Html(h) => builder.with_html(h),
    };
    // Chrome extensions: every unpacked extension in the browser's folder is loaded when a view starts.
    #[cfg(windows)]
    let builder = {
        use wry::WebViewBuilderExtWindows;
        if shared.private {
            builder
        } else {
            let root = crate::chrome_ext::enabled_root();
            let _ = std::fs::create_dir_all(&root);
            builder.with_browser_extensions_enabled(true).with_extensions_path(root)
        }
    };
    // The proxy the browser started with (the engine reads it once, so every view must use the same one).
    let builder = match crate::proxy::startup() {
        Some(p) => {
            let ep = wry::ProxyEndpoint { host: p.host.clone(), port: p.port.clone() };
            builder.with_proxy_config(if p.socks { wry::ProxyConfig::Socks5(ep) } else { wry::ProxyConfig::Http(ep) })
        }
        None => builder,
    };
    let wv = builder
        .with_devtools(true)
        .with_incognito(shared.private)
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
            if nav_shared.shield.blocked_sites.lock().map_or(false, |b| !b.is_empty() && crate::siterules::matches_any(&b, &url)) {
                nav_shared.push_event(WebEvent::SiteBlocked(url));
                return false;
            }
            if nav_shared.shield.https_only.load(Ordering::Relaxed) && url.get(..7).map_or(false, |p| p.eq_ignore_ascii_case("http://")) {
                let exempt = nav_shared.shield.https_exempt.lock().map(|e| e.clone()).unwrap_or_default();
                if let Some(https) = crate::permissions::upgrade_target(&url, &exempt) {
                    if let Ok(mut up) = nav_shared.shield.upgraded.lock() {
                        up.insert(norm_url(&https));
                    }
                    nav_shared.push_event(WebEvent::Upgrade(https));
                    return false;
                }
            }
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
                if c.starts_with("axomai://") {
                    if let Ok(mut q) = nav_shared.nav.lock() {
                        q.push(c);
                    }
                } else {
                    nav_shared.push_event(WebEvent::Navigating(c));
                }
            }
            allow
        })
        .with_new_window_req_handler(move |url: String| {
            // `target=_blank` and `window.open` open a new tab; the context-menu "View page source" asks for `view-source:`.
            if url.starts_with("http://") || url.starts_with("https://") {
                window_shared.push_event(WebEvent::OpenUrl(url));
            } else if let Some(target) = url.strip_prefix("view-source:") {
                window_shared.push_command(&format!("viewsource/{}", crate::viewsource::encode(target)));
            }
            false
        })
        .with_document_title_changed_handler(move |t: String| title_shared.push_event(WebEvent::Title(t)))
        .with_on_page_load_handler(move |ev, url| match ev {
            PageLoadEvent::Started => load_shared.push_event(WebEvent::LoadStarted(url)),
            PageLoadEvent::Finished => load_shared.push_event(WebEvent::LoadFinished(url)),
        })
        .build_as_child(window)
        .ok()?;

    #[cfg(windows)]
    {
        com::install_network_shield(&wv, shared.shield.clone());
        com::install_message_bridge(&wv, shared.clone());
        com::install_accelerators(&wv, shared.clone());
        com::install_context_menu(&wv, shared.clone());
        com::install_favicon(&wv, shared.clone());
        com::install_audio(&wv, shared.clone());
        com::install_focus(&wv, shared.clone());
        com::install_downloads(&wv, shared.clone());
        com::disable_builtin_autofill(&wv);
        com::install_permissions(&wv, shared.clone());
        com::install_nav_failure(&wv, shared.clone());
        com::install_zoom_and_fullscreen(&wv, shared.clone());
    }
    Some(wv)
}

#[cfg(windows)]
pub mod com {
    use super::*;
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{
        take_pwstr, AddScriptToExecuteOnDocumentCreatedCompletedHandler, CallDevToolsProtocolMethodCompletedHandler,
        AcceleratorKeyPressedEventHandler, ContextMenuRequestedEventHandler, CustomItemSelectedEventHandler, FaviconChangedEventHandler,
        GetFaviconCompletedHandler, FocusChangedEventHandler, ClearBrowsingDataCompletedHandler, IsDocumentPlayingAudioChangedEventHandler, TrySuspendCompletedHandler, WebMessageReceivedEventHandler,
        WebResourceRequestedEventHandler, BytesReceivedChangedEventHandler, DownloadStartingEventHandler, StateChangedEventHandler,
        NavigationCompletedEventHandler, PermissionRequestedEventHandler, ZoomFactorChangedEventHandler,
        ContainsFullScreenElementChangedEventHandler, PrintToPdfCompletedHandler,
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
                if shield.gpc.load(Ordering::Relaxed) {
                    if let Ok(headers) = request.Headers() {
                        let _ = headers.SetHeader(w!("Sec-GPC"), w!("1"));
                    }
                }
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
                    // Not a string: a structured message from a page (the password manager). The sender's address
                    // comes from the web view, never from the message itself.
                    let mut json = PWSTR::null();
                    let mut source = PWSTR::null();
                    if args.WebMessageAsJson(&mut json).is_ok() && args.Source(&mut source).is_ok() {
                        let (json, source) = (take_pwstr(json), take_pwstr(source));
                        if json.len() <= 16 * 1024 {
                            shared.push_event(WebEvent::PageMsg(source, json));
                        }
                    }
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
                let command: String = if alt {
                    match (vk, ctrl) {
                        (0x25, false) => "back".into(),      // Alt+Left
                        (0x27, false) => "forward".into(),   // Alt+Right
                        (0x24, false) => "home".into(),      // Alt+Home
                        (0x44, false) => "focus-url".into(), // Alt+D
                        _ => return Ok(()),
                    }
                } else { match (vk, ctrl, shift) {
                    (0xBB, true, _) | (0x6B, true, _) => "zoom-in".into(),    // Ctrl + / Ctrl =
                    (0xBD, true, false) | (0x6D, true, false) => "zoom-out".into(), // Ctrl -
                    (0x30, true, false) | (0x60, true, false) => "zoom-reset".into(), // Ctrl 0
                    (0x74, false, false) | (0x52, true, false) => "reload".into(),   // F5 / Ctrl+R
                    (0x74, true, _) | (0x52, true, true) => "reload-hard".into(),    // Ctrl+F5 / Ctrl+Shift+R
                    (0x4B, true, false) | (0x45, true, false) | (0x75, false, false) => "focus-url".into(), // Ctrl+K / Ctrl+E / F6
                    (0x42, true, true) => "bookmark-bar-toggle".into(), // Ctrl+Shift+B
                    (0x7A, false, false) => "fullscreen".into(), // F11
                    (0x55, true, false) => "viewsource-current".into(), // U
                    (0x54, true, false) => "newtab".into(),             // T
                    (0x54, true, true) => "reopen-tab".into(),          // Shift+T
                    (0x57, true, false) => "closetab".into(),           // W
                    (0x4C, true, false) => "focus-url".into(),          // L
                    (0x48, true, false) => "history".into(),            // H
                    (0x4A, true, false) => "downloads".into(),          // J
                    (0x44, true, false) => "bookmark-toggle".into(),    // D
                    (0x46, true, false) => "find".into(),               // F
                    (0x50, true, false) => "print".into(),              // P
                    (0x4E, true, false) => "new-window".into(),         // N
                    (0x41, true, true) => "tab-search".into(),          // Ctrl+Shift+A
                    (0x4E, true, true) => "new-incognito".into(),       // Shift+N
                    (0x4F, true, true) => "bookmarks".into(),           // Shift+O
                    (0x2E, true, true) => "clear-data-dialog".into(),   // Shift+Delete
                    (0x09, true, false) | (0x22, true, false) => "tab-next".into(), // Tab / PageDown
                    (0x09, true, true) | (0x21, true, false) => "tab-prev".into(),  // Shift+Tab / PageUp
                    (0x31..=0x39, true, false) => format!("tab-index/{}", vk - 0x30),
                    _ => return Ok(()),
                } };
                shared.push_command(&command);
                args.SetHandled(true)?;
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = controller.add_AcceleratorKeyPressed(&handler, &mut token);
        }
    }

    /// The page took keyboard focus (the user clicked into it): the address bar must stop capturing keys.
    pub fn install_focus(wv: &WebView, shared: WebShared) {
        let controller = wv.controller();
        let handler = FocusChangedEventHandler::create(Box::new(move |_sender, _args| {
            shared.push_command(&format!("page-focus/{}", shared.tab_id));
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = controller.add_GotFocus(&handler, &mut token);
        }
    }

    thread_local! {
        /// Download operations that are still running, by download id (WebView2 objects stay on the UI thread).
        static OPS: std::cell::RefCell<std::collections::HashMap<u64, ICoreWebView2DownloadOperation>> = Default::default();
    }
    static NEXT_DOWNLOAD: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    /// "pause", "resume" or "cancel" for a running download.
    pub fn download_control(id: u64, action: &str) {
        OPS.with(|ops| {
            if let Some(op) = ops.borrow().get(&id) {
                unsafe {
                    let _ = match action {
                        "pause" => op.Pause(),
                        "resume" => op.Resume(),
                        _ => op.Cancel(),
                    };
                }
            }
        });
    }

    thread_local! {
        /// Permission questions that are waiting for the user (WebView2 objects stay on the UI thread).
        static PERMS: std::cell::RefCell<std::collections::HashMap<u64, (ICoreWebView2PermissionRequestedEventArgs, ICoreWebView2Deferral)>> = Default::default();
    }
    static NEXT_PERM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    fn permission_name(kind: COREWEBVIEW2_PERMISSION_KIND) -> &'static str {
        let table: [(COREWEBVIEW2_PERMISSION_KIND, &str); 12] = [
            (COREWEBVIEW2_PERMISSION_KIND_CAMERA, "camera"),
            (COREWEBVIEW2_PERMISSION_KIND_MICROPHONE, "microphone"),
            (COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION, "location"),
            (COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS, "notifications"),
            (COREWEBVIEW2_PERMISSION_KIND_OTHER_SENSORS, "sensors"),
            (COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, "clipboard"),
            (COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS, "downloads"),
            (COREWEBVIEW2_PERMISSION_KIND_FILE_READ_WRITE, "files"),
            (COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY, "autoplay"),
            (COREWEBVIEW2_PERMISSION_KIND_LOCAL_FONTS, "fonts"),
            (COREWEBVIEW2_PERMISSION_KIND_MIDI_SYSTEM_EXCLUSIVE_MESSAGES, "midi"),
            (COREWEBVIEW2_PERMISSION_KIND_WINDOW_MANAGEMENT, "windows"),
        ];
        table.iter().find(|(k, _)| *k == kind).map(|(_, n)| *n).unwrap_or("other")
    }

    /// Finish a permission question: the page gets its answer.
    pub fn permission_answer(id: u64, allow: bool) {
        PERMS.with(|m| {
            if let Some((args, deferral)) = m.borrow_mut().remove(&id) {
                unsafe {
                    let _ = args.SetState(if allow { COREWEBVIEW2_PERMISSION_STATE_ALLOW } else { COREWEBVIEW2_PERMISSION_STATE_DENY });
                    let _ = deferral.Complete();
                }
            }
        });
    }

    /// Every permission request is held (deferred) and reported; the browser decides, usually after asking the user.
    pub fn install_permissions(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        let handler = PermissionRequestedEventHandler::create(Box::new(move |_sender, args| {
            let Some(args) = args else { return Ok(()) };
            unsafe {
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                let mut kind = COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION;
                args.PermissionKind(&mut kind)?;
                let deferral = args.GetDeferral()?;
                let id = NEXT_PERM.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                PERMS.with(|m| m.borrow_mut().insert(id, (args.clone(), deferral)));
                shared.push_event(WebEvent::PermissionAsk { id, uri, name: permission_name(kind).to_string() });
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = core.add_PermissionRequested(&handler, &mut token);
        }
    }

    /// Report a failed load of an address we upgraded to https, so the user can be offered the http original.
    pub fn install_nav_failure(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        let handler = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
            let (Some(sender), Some(args)) = (sender, args) else { return Ok(()) };
            unsafe {
                let mut ok = BOOL(0);
                args.IsSuccess(&mut ok)?;
                if ok.as_bool() {
                    return Ok(());
                }
                let mut status = COREWEBVIEW2_WEB_ERROR_STATUS_UNKNOWN;
                args.WebErrorStatus(&mut status)?;
                if status == COREWEBVIEW2_WEB_ERROR_STATUS_OPERATION_CANCELED {
                    return Ok(());
                }
                let mut src = PWSTR::null();
                sender.Source(&mut src)?;
                let url = take_pwstr(src);
                let was_upgrade = shared.shield.upgraded.lock().map(|mut up| up.remove(&norm_url(&url))).unwrap_or(false);
                if was_upgrade && url.len() > 5 {
                    shared.push_event(WebEvent::UpgradeFailed(format!("http{}", &url[5..])));
                }
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = core.add_NavigationCompleted(&handler, &mut token);
        }
    }

    pub fn set_zoom(wv: &WebView, factor: f64) {
        unsafe {
            let _ = wv.controller().SetZoomFactor(factor);
        }
    }

    /// Report zoom changes (wheel, keys, our own) and a page's own full-screen requests.
    pub fn install_zoom_and_fullscreen(wv: &WebView, shared: WebShared) {
        let controller = wv.controller();
        let zoom_shared = shared.clone();
        let zoom = ZoomFactorChangedEventHandler::create(Box::new(move |sender, _| {
            if let Some(c) = sender {
                let mut z = 1.0f64;
                unsafe {
                    if c.ZoomFactor(&mut z).is_ok() {
                        zoom_shared.push_event(WebEvent::Zoom(z));
                    }
                }
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = controller.add_ZoomFactorChanged(&zoom, &mut token);
        }
        let Some(core) = core(wv) else { return };
        let fs = ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
            if let Some(c) = sender {
                let mut on = BOOL(0);
                unsafe {
                    if c.ContainsFullScreenElement(&mut on).is_ok() {
                        shared.push_event(WebEvent::HtmlFullscreen(on.as_bool()));
                    }
                }
            }
            Ok(())
        }));
        unsafe {
            let mut token = std::mem::zeroed();
            let _ = core.add_ContainsFullScreenElementChanged(&fs, &mut token);
        }
    }

    /// Write the page to a PDF file; the result comes back as page data `("pdf", path, "1" | "0")`.
    pub fn print_to_pdf(wv: &WebView, path: &std::path::Path, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(c7) = core.cast::<ICoreWebView2_7>() else { return };
            let shown = path.to_string_lossy().to_string();
            let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
                let good = result.is_ok() && ok;
                shared.push_event(WebEvent::PageData("pdf".into(), shown.clone(), if good { "1".into() } else { "0".into() }));
                Ok(())
            }));
            let _ = c7.PrintToPdf(&HSTRING::from(path.to_string_lossy().as_ref()), None, &handler);
        }
    }

    /// Tracking prevention level of the whole profile: "basic", "balanced" or "strict".
    pub fn set_tracking_level(wv: &WebView, level: &str) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(c13) = core.cast::<ICoreWebView2_13>() else { return };
            let Ok(profile) = c13.Profile() else { return };
            let Ok(p3) = profile.cast::<ICoreWebView2Profile3>() else { return };
            let l = match level {
                "basic" => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BASIC,
                "strict" => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT,
                _ => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED,
            };
            let _ = p3.SetPreferredTrackingPreventionLevel(l);
        }
    }

    /// Axomai has its own password manager (encrypted, per-site); WebView2's built-in "Saved info" would keep a
    /// second, unrelated copy of what was typed into forms.
    pub fn disable_builtin_autofill(wv: &WebView) {
        let Some(core) = core(wv) else { return };
        unsafe {
            if let Ok(settings) = core.Settings() {
                if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
                    let _ = s4.SetIsGeneralAutofillEnabled(false);
                    let _ = s4.SetIsPasswordAutosaveEnabled(false);
                }
            }
        }
    }

    /// Takes over every download of this web view: saves into the download folder under a free name, hides the
    /// browser's own download flyout, and reports progress so the Downloads page can show it live.
    pub fn install_downloads(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(core4) = core.cast::<ICoreWebView2_4>() else { return };
            let handler = DownloadStartingEventHandler::create(Box::new(move |_sender, args| {
                let Some(args) = args else { return Ok(()) };
                let op = args.DownloadOperation()?;
                let mut s = PWSTR::null();
                op.Uri(&mut s)?;
                let url = take_pwstr(s);
                let mut s = PWSTR::null();
                args.ResultFilePath(&mut s)?;
                let suggested = take_pwstr(s);
                let name = std::path::Path::new(&suggested).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let path = unique_path(&shared.download_dir(), &name);
                let wide = HSTRING::from(path.to_string_lossy().as_ref());
                args.SetResultFilePath(&wide)?;
                args.SetHandled(true)?;
                let mut total = 0i64;
                let _ = op.TotalBytesToReceive(&mut total);
                let id = NEXT_DOWNLOAD.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                OPS.with(|ops| ops.borrow_mut().insert(id, op.clone()));
                // The navigation became a download, so no page will ever report that it finished loading.
                shared.push_event(WebEvent::LoadFinished(url.clone()));
                shared.push_event(WebEvent::Download(DlEvent::Started { id, url, path, total: total.max(0) }));

                let progress = shared.clone();
                let on_bytes = BytesReceivedChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(op) = sender {
                        let (mut got, mut all) = (0i64, 0i64);
                        let _ = op.BytesReceived(&mut got);
                        let _ = op.TotalBytesToReceive(&mut all);
                        progress.push_event(WebEvent::Download(DlEvent::Progress { id, received: got, total: all.max(0) }));
                    }
                    Ok(())
                }));
                let state_events = shared.clone();
                let on_state = StateChangedEventHandler::create(Box::new(move |sender, _| {
                    let Some(op) = sender else { return Ok(()) };
                    let mut state = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
                    let _ = op.State(&mut state);
                    let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NONE;
                    let _ = op.InterruptReason(&mut reason);
                    if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
                        OPS.with(|ops| ops.borrow_mut().remove(&id));
                        state_events.push_event(WebEvent::Download(DlEvent::Done { id }));
                    } else if state == COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED {
                        if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED {
                            state_events.push_event(WebEvent::Download(DlEvent::Paused { id, paused: true }));
                        } else {
                            OPS.with(|ops| ops.borrow_mut().remove(&id));
                            let why = if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED { "canceled".to_string() } else { format!("error {}", reason.0) };
                            state_events.push_event(WebEvent::Download(DlEvent::Failed { id, reason: why }));
                        }
                    } else {
                        state_events.push_event(WebEvent::Download(DlEvent::Paused { id, paused: false }));
                    }
                    Ok(())
                }));
                let mut token = std::mem::zeroed();
                let _ = op.add_BytesReceivedChanged(&on_bytes, &mut token);
                let _ = op.add_StateChanged(&on_state, &mut token);
                Ok(())
            }));
            let mut token = std::mem::zeroed();
            let _ = core4.add_DownloadStarting(&handler, &mut token);
        }
    }

    /// Favicon changes: fetch the PNG for the new icon and report it.
    pub fn install_favicon(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(core15) = core.cast::<ICoreWebView2_15>() else { return };
            let handler = FaviconChangedEventHandler::create(Box::new(move |sender, _args| {
                let Some(sender) = sender else { return Ok(()) };
                let Ok(c15) = sender.cast::<ICoreWebView2_15>() else { return Ok(()) };
                let events = shared.clone();
                let done = GetFaviconCompletedHandler::create(Box::new(move |res, stream| {
                    if res.is_err() {
                        return Ok(());
                    }
                    if let Some(stream) = stream {
                        let mut bytes: Vec<u8> = Vec::new();
                        let mut buf = vec![0u8; 16 * 1024];
                        loop {
                            let mut read = 0u32;
                            let hr = stream.Read(buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len() as u32, Some(&mut read));
                            if hr.is_err() || read == 0 {
                                break;
                            }
                            bytes.extend_from_slice(&buf[..read as usize]);
                            if bytes.len() > 2 * 1024 * 1024 {
                                break;
                            }
                        }
                        if !bytes.is_empty() {
                            events.push_event(WebEvent::Favicon(bytes));
                        }
                    }
                    Ok(())
                }));
                let _ = c15.GetFavicon(COREWEBVIEW2_FAVICON_IMAGE_FORMAT_PNG, &done);
                Ok(())
            }));
            let mut token = std::mem::zeroed();
            let _ = core15.add_FaviconChanged(&handler, &mut token);
        }
    }

    /// "This tab is playing audio" indicator.
    pub fn install_audio(wv: &WebView, shared: WebShared) {
        let Some(core) = core(wv) else { return };
        unsafe {
            let Ok(core8) = core.cast::<ICoreWebView2_8>() else { return };
            let handler = IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |sender, _args| {
                if let Some(sender) = sender {
                    if let Ok(c8) = sender.cast::<ICoreWebView2_8>() {
                        let mut playing = BOOL(0);
                        if c8.IsDocumentPlayingAudio(&mut playing).is_ok() {
                            shared.push_event(WebEvent::Audio(playing.as_bool()));
                        }
                    }
                }
                Ok(())
            }));
            let mut token = std::mem::zeroed();
            let _ = core8.add_IsDocumentPlayingAudioChanged(&handler, &mut token);
        }
    }

    /// Clear cookies / site data and / or the HTTP cache. `since_secs` limits it to the last N seconds.
    pub fn clear_browsing_data(wv: &WebView, cookies: bool, cache: bool, since_secs: Option<u64>) {
        let Some(core) = core(wv) else { return };
        let mut kinds = 0i32;
        if cookies {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES.0 | COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE.0;
        }
        if cache {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE.0;
        }
        if kinds == 0 {
            return;
        }
        unsafe {
            let Ok(profile) = core.cast::<ICoreWebView2_13>().and_then(|c| c.Profile()) else { return };
            let Ok(profile2) = profile.cast::<ICoreWebView2Profile2>() else { return };
            let handler = ClearBrowsingDataCompletedHandler::create(Box::new(|_hr| Ok(())));
            let kinds = COREWEBVIEW2_BROWSING_DATA_KINDS(kinds);
            match since_secs {
                None => {
                    let _ = profile2.ClearBrowsingData(kinds, &handler);
                }
                Some(s) => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
                    let _ = profile2.ClearBrowsingDataInTimeRange(kinds, now - s as f64, now + 60.0, &handler);
                }
            }
        }
    }

    pub fn set_muted(wv: &WebView, muted: bool) {
        if let Some(core) = core(wv) {
            unsafe {
                if let Ok(c8) = core.cast::<ICoreWebView2_8>() {
                    let _ = c8.SetIsMuted(muted);
                }
            }
        }
    }

    /// Release the memory of a hidden tab while keeping its page state (scroll, forms, history).
    pub fn suspend(wv: &WebView) {
        if let Some(core) = core(wv) {
            unsafe {
                if let Ok(c3) = core.cast::<ICoreWebView2_3>() {
                    let handler = TrySuspendCompletedHandler::create(Box::new(|_hr, _ok| Ok(())));
                    let _ = c3.TrySuspend(&handler);
                }
            }
        }
    }

    pub fn resume(wv: &WebView) {
        if let Some(core) = core(wv) {
            unsafe {
                if let Ok(c3) = core.cast::<ICoreWebView2_3>() {
                    let _ = c3.Resume();
                }
            }
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

    /// Open the DevTools window (wry only offers this in debug builds, so ask the engine directly).
    pub fn open_devtools(wv: &WebView) {
        if let Some(core) = core(wv) {
            unsafe {
                let _ = core.OpenDevToolsWindow();
            }
        }
    }

    /// Fire-and-forget DevTools-protocol call.
    pub fn cdp(wv: &WebView, method: &str, params: &str) {
        let Some(core) = core(wv) else { return };
        let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_res, _json| Ok(())));
        unsafe {
            let _ = core.CallDevToolsProtocolMethod(&HSTRING::from(method), &HSTRING::from(params), &handler);
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

    pub fn download_control(_id: u64, _action: &str) {}

    pub fn disable_builtin_autofill(_wv: &WebView) {}
    pub fn permission_answer(_id: u64, _allow: bool) {}
    pub fn set_zoom(_wv: &WebView, _factor: f64) {}
    pub fn print_to_pdf(_wv: &WebView, _path: &std::path::Path, _shared: WebShared) {}
    pub fn set_tracking_level(_wv: &WebView, _level: &str) {}

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
    pub fn clear_browsing_data(wv: &WebView, cookies: bool, cache: bool, _since: Option<u64>) {
        if cookies || cache {
            let _ = wv.clear_all_browsing_data();
        }
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
