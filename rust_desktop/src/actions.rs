//! What the toolbar buttons and extensions actually do.
//!
//! `Core` owns the browser-wide state that is not tied to a particular page (theme, profile, extension
//! switches, booster timer, back/forward/bookmark state) and exposes one method per user action. `main.rs`
//! only decides *when* to call them, so there is a single code path whether an action comes from a toolbar
//! click, the extensions popup, the Extensions page or the three-dot menu.

use crate::extensions as ex;
use crate::storage::BrowserStorage;
use crate::types::Extension;
use crate::web::{self, WebEvent, WebShared};
use crate::{ai, ext_scripts, overlays, sys, theme};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use wry::WebView;

const BOOSTER_INTERVAL: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub struct Core {
    pub theme: &'static theme::Theme,
    pub profile_name: String,
    /// Bumped whenever an extension switch changes; a tab rebuilds its document-start script when it is behind.
    pub ext_generation: u64,
    pub booster_next: Option<Instant>,
    pub booster_status: String,
    pub can_back: bool,
    pub can_fwd: bool,
    pub bookmarked: bool,
    pub last_capture: Option<String>,
    last_poll: Instant,
}

/// Distance from the right edge of the page area to a toolbar control, in CSS pixels.
pub fn anchor_right(window_w: f32, control_right: f32, scale: f32) -> f32 {
    ((window_w - control_right) / scale.max(0.5) - 4.0).max(6.0)
}

fn default_profile_name() -> String {
    let raw = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return "You".to_string();
    }
    let mut chars = raw.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "You".to_string(),
    }
}

impl Core {
    pub fn new(storage: Option<&BrowserStorage>, exts: &mut [Extension]) -> Self {
        let get = |k: &str| storage.and_then(|s| s.get_setting(k).ok().flatten());
        let theme = theme::by_id(get("theme").as_deref().unwrap_or("tea-garden"));
        let profile_name = get("profile_name").filter(|n| !n.trim().is_empty()).unwrap_or_else(default_profile_name);
        if let Some(bits) = get("ext_enabled") {
            for (i, ch) in bits.chars().enumerate() {
                if let Some(e) = exts.get_mut(i) {
                    e.enabled = ch == '1';
                }
            }
        }
        Core {
            theme,
            profile_name,
            ext_generation: 0,
            booster_next: None,
            booster_status: String::new(),
            can_back: false,
            can_fwd: false,
            bookmarked: false,
            last_capture: None,
            last_poll: Instant::now(),
        }
    }

    fn persist_extensions(&self, storage: Option<&BrowserStorage>, exts: &[Extension]) {
        if let Some(s) = storage {
            let bits: String = exts.iter().map(|e| if e.enabled { '1' } else { '0' }).collect();
            let _ = s.set_setting("ext_enabled", &bits);
        }
    }

    /// Push the current extension switches into the network shield, the document-start script and the engine.
    pub fn apply_extensions(&mut self, wv: Option<&WebView>, doc: &web::com::DocScript, shared: &WebShared, exts: &[Extension]) {
        let adblock = exts[ex::ADBLOCK].enabled;
        let privacy = exts[ex::PRIVACY].enabled;
        shared.shield.adblock.store(adblock, Ordering::SeqCst);
        shared.shield.privacy.store(privacy, Ordering::SeqCst);
        if let Some(wv) = wv {
            doc.set(wv, &ext_scripts::doc_start(adblock, privacy));
            web::com::set_memory_low(wv, exts[ex::BOOSTER].enabled);
        }
        if exts[ex::BOOSTER].enabled {
            if self.booster_next.is_none() {
                self.booster_next = Some(Instant::now() + BOOSTER_INTERVAL);
            }
        } else {
            self.booster_next = None;
        }
    }

    pub fn toggle_extension(
        &mut self,
        idx: usize,
        wv: Option<&WebView>,
        doc: &web::com::DocScript,
        shared: &WebShared,
        exts: &mut [Extension],
        storage: Option<&BrowserStorage>,
    ) {
        let Some(e) = exts.get_mut(idx) else { return };
        e.enabled = !e.enabled;
        let on = e.enabled;
        self.ext_generation += 1;
        self.persist_extensions(storage, exts);
        self.apply_extensions(wv, doc, shared, exts);
        let Some(wv) = wv else { return };
        let remote_page = !shared.trusted.load(Ordering::SeqCst);
        match idx {
            // Both shields act on requests / scripts that exist before the page is interactive, so apply them
            // with a cache-bypassing reload of the current web page.
            ex::ADBLOCK | ex::PRIVACY if remote_page => web::com::reload_hard(wv),
            ex::READER if !on => {
                let _ = wv.evaluate_script("(function(){var e=document.getElementById('__ax_pop_reader');if(e)e.remove()})()");
            }
            ex::BOOSTER if on => self.free_memory(Some(wv), false),
            _ => {}
        }
    }

    /// Run an extension's main action (what a click on its row does).
    pub fn run_extension(&mut self, idx: usize, wv: Option<&WebView>, shared: &WebShared, exts: &[Extension], right: f32) {
        let Some(wv) = wv else { return };
        let Some(e) = exts.get(idx) else { return };
        if !e.enabled {
            self.toast(wv, &format!("{} is turned off", e.name), None, shared);
            return;
        }
        let remote = !shared.trusted.load(Ordering::SeqCst);
        match idx {
            ex::ADBLOCK | ex::PRIVACY => self.open_shield(Some(wv), shared, exts, right),
            ex::READER => {
                if remote {
                    let _ = wv.evaluate_script(&ext_scripts::reader(self.theme));
                } else {
                    self.toast(wv, "Open an article page to use Reader Mode", None, shared);
                }
            }
            ex::TRANSLATE => {
                if remote {
                    let _ = wv.evaluate_script(&ext_scripts::translate(self.theme));
                } else {
                    self.toast(wv, "Open a web page first, then translate it", None, shared);
                }
            }
            ex::CAPTURE => {
                let _ = wv.evaluate_script(&ext_scripts::capture_menu(self.theme, &shared.token));
            }
            ex::BOOSTER => self.free_memory(Some(wv), true),
            _ => {}
        }
    }

    /// Remove every open popup (extensions list, menus, panels, reader view) from the current page.
    pub fn close_popups(&self, wv: Option<&WebView>) {
        if let Some(wv) = wv {
            let _ = wv.evaluate_script("document.querySelectorAll('[id^=\"__ax_pop_\"]').forEach(function(e){e.remove()})");
        }
    }

    pub fn toast(&self, wv: &WebView, msg: &str, action: Option<(&str, &str, &str)>, _shared: &WebShared) {
        let _ = wv.evaluate_script(&overlays::toast(self.theme, msg, action));
    }

    // ---------------------------------------------------------------- memory

    pub fn free_memory(&mut self, wv: Option<&WebView>, announce: bool) {
        let r = sys::trim_memory();
        self.booster_status = if r.before > 0 {
            format!("Released {:.0} MB of idle memory", r.freed_mb())
        } else {
            "Memory released".to_string()
        };
        if announce {
            if let Some(wv) = wv {
                let msg = if r.before > 0 {
                    format!("⚡ Released {:.0} MB of idle memory", r.freed_mb())
                } else {
                    "⚡ Memory released".to_string()
                };
                let _ = wv.evaluate_script(&overlays::toast(self.theme, &msg, None));
            }
        }
    }

    /// Called every frame; trims memory on a timer while the booster is on.
    pub fn tick(&mut self, exts: &[Extension]) {
        if exts[ex::BOOSTER].enabled {
            if let Some(t) = self.booster_next {
                if Instant::now() >= t {
                    self.booster_next = Some(Instant::now() + BOOSTER_INTERVAL);
                    self.free_memory(None, false);
                }
            }
        }
    }

    // ----------------------------------------------------------- page state

    /// Refresh back/forward/bookmark state a few times per second. Returns true if anything changed.
    pub fn poll(&mut self, wv: Option<&WebView>, storage: Option<&BrowserStorage>, address: &str) -> bool {
        if self.last_poll.elapsed() < POLL_INTERVAL {
            return false;
        }
        self.last_poll = Instant::now();
        let (back, fwd) = wv.map(web::com::can_go).unwrap_or((false, false));
        let bookmarked = storage.map(|s| s.is_bookmarked(address).unwrap_or(false)).unwrap_or(false);
        let changed = back != self.can_back || fwd != self.can_fwd || bookmarked != self.bookmarked;
        self.can_back = back;
        self.can_fwd = fwd;
        self.bookmarked = bookmarked;
        changed
    }

    pub fn toggle_bookmark(&mut self, address: &str, title: &str, storage: Option<&BrowserStorage>) -> Option<bool> {
        let s = storage?;
        if !(address.starts_with("http://") || address.starts_with("https://")) {
            return None;
        }
        let now = if s.is_bookmarked(address).unwrap_or(false) {
            let _ = s.remove_bookmark_by_url(address);
            false
        } else {
            let _ = s.add_bookmark(address, title, "Unsorted");
            true
        };
        self.bookmarked = now;
        Some(now)
    }

    // --------------------------------------------------------------- popups

    pub fn ext_items(&self, shared: &WebShared, exts: &[Extension]) -> Vec<overlays::ExtItem> {
        exts.iter()
            .enumerate()
            .map(|(i, e)| {
                let status = match i {
                    ex::ADBLOCK => format!(
                        "{} blocked on this page",
                        shared.shield.page_blocked.load(Ordering::Relaxed)
                    ),
                    ex::READER => "Clutter-free reading".to_string(),
                    ex::TRANSLATE => "12 languages".to_string(),
                    ex::PRIVACY => "Trackers & fingerprinting".to_string(),
                    ex::CAPTURE => "Visible · Full page · Select".to_string(),
                    _ => {
                        if self.booster_status.is_empty() {
                            "Low-memory mode".to_string()
                        } else {
                            self.booster_status.clone()
                        }
                    }
                };
                overlays::ExtItem { idx: i, emoji: ex::EMOJI[i], name: e.name, status, enabled: e.enabled }
            })
            .collect()
    }

    pub fn open_extensions(&self, wv: Option<&WebView>, shared: &WebShared, exts: &[Extension], right: f32) {
        if let Some(wv) = wv {
            let items = self.ext_items(shared, exts);
            let _ = wv.evaluate_script(&overlays::extensions_popup(self.theme, &shared.token, right, &items));
        }
    }

    pub fn open_shield(&self, wv: Option<&WebView>, shared: &WebShared, exts: &[Extension], right: f32) {
        if let Some(wv) = wv {
            let host = shared.shield.page_host.lock().map(|h| h.clone()).unwrap_or_default();
            let v = overlays::ShieldView {
                host: if host.is_empty() { "this page".into() } else { host },
                page_blocked: shared.shield.page_blocked.load(Ordering::Relaxed),
                total_blocked: shared.shield.total_blocked.load(Ordering::Relaxed),
                adblock: exts[ex::ADBLOCK].enabled,
                privacy: exts[ex::PRIVACY].enabled,
            };
            let _ = wv.evaluate_script(&overlays::shield_popup(self.theme, &shared.token, right, &v));
        }
    }

    pub fn open_profile(&self, wv: Option<&WebView>, shared: &WebShared, storage: Option<&BrowserStorage>, right: f32) {
        if let Some(wv) = wv {
            let count = |t: &str| storage.map(|s| s.count(t)).unwrap_or(0);
            let v = overlays::ProfileView {
                name: self.profile_name.clone(),
                bookmarks: count("bookmarks"),
                history: count("history"),
                downloads: count("downloads"),
                blocked_total: shared.shield.total_blocked.load(Ordering::Relaxed),
            };
            let _ = wv.evaluate_script(&overlays::profile_popup(self.theme, &shared.token, right, &v));
        }
    }

    pub fn open_site_info(&self, wv: Option<&WebView>, shared: &WebShared, right: f32, address: &str) {
        if let Some(wv) = wv {
            let sec = crate::toolbar::Security::from_address(address);
            let host = match sec {
                crate::toolbar::Security::Internal => "axomai".to_string(),
                _ => crate::blocklist::host_of(address),
            };
            let blocked = shared.shield.page_blocked.load(Ordering::Relaxed);
            let _ = wv.evaluate_script(&overlays::site_info_popup(self.theme, &shared.token, right, sec, &host, blocked));
        }
    }

    pub fn open_theme_menu(&self, wv: Option<&WebView>, shared: &WebShared, right: f32) {
        if let Some(wv) = wv {
            let _ = wv.evaluate_script(&overlays::theme_popup(self.theme, &shared.token, right));
        }
    }

    pub fn open_menu(&self, wv: Option<&WebView>, shared: &WebShared, right: f32) {
        if let Some(wv) = wv {
            let _ = wv.evaluate_script(&overlays::menu_popup(self.theme, &shared.token, right, MENU));
        }
    }

    pub fn open_ai(&self, wv: Option<&WebView>, shared: &WebShared, right: f32, title: &str) {
        if let Some(wv) = wv {
            let host = shared.shield.page_host.lock().map(|h| h.clone()).unwrap_or_default();
            let host = if host.is_empty() { "this page".to_string() } else { host };
            let _ = wv.evaluate_script(&overlays::ai_popup(self.theme, &shared.token, right, &host, title));
        }
    }

    pub fn open_qr(&self, wv: Option<&WebView>, shared: &WebShared, right: f32, url: &str) {
        let Some(wv) = wv else { return };
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            self.toast(wv, "Open a web page to get its QR code", None, shared);
            return;
        }
        match qrcode::QrCode::new(url.as_bytes()) {
            Ok(code) => {
                let size = code.width();
                let modules: Vec<bool> = code.to_colors().iter().map(|c| *c == qrcode::Color::Dark).collect();
                let _ = wv.evaluate_script(&overlays::qr_popup(self.theme, &shared.token, right, url, size, &modules));
            }
            Err(_) => self.toast(wv, "This address is too long to fit in a QR code", None, shared),
        }
    }

    pub fn set_theme(&mut self, id: &str, storage: Option<&BrowserStorage>) {
        self.theme = theme::by_id(id);
        if let Some(s) = storage {
            let _ = s.set_setting("theme", self.theme.id);
        }
    }

    pub fn set_profile_name(&mut self, name: &str, storage: Option<&BrowserStorage>) {
        let name: String = name.trim().chars().take(32).collect();
        if name.is_empty() {
            return;
        }
        self.profile_name = name;
        if let Some(s) = storage {
            let _ = s.set_setting("profile_name", &self.profile_name);
        }
    }

    pub fn avatar_letter(&self) -> String {
        self.profile_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "A".into())
    }

    pub fn clear_browsing_data(&mut self, wv: Option<&WebView>, storage: Option<&BrowserStorage>, shared: &WebShared) {
        if let Some(s) = storage {
            let _ = s.clear_history();
            let _ = s.clear_downloads();
        }
        if let Some(wv) = wv {
            let _ = wv.clear_all_browsing_data();
            self.toast(wv, "Browsing data cleared", None, shared);
        }
        self.bookmarked = storage.map(|_| false).unwrap_or(false);
    }

    // ------------------------------------------------------------- capture

    fn capture_path(&mut self) -> std::path::PathBuf {
        let p = sys::screenshots_dir().join(format!("axomai-{}.png", sys::timestamp()));
        self.last_capture = Some(p.to_string_lossy().to_string());
        p
    }

    pub fn capture_visible(&mut self, wv: Option<&WebView>, shared: &WebShared) {
        if let Some(wv) = wv {
            let out = self.capture_path();
            web::com::capture_png(wv, r#"{"format":"png"}"#, out, shared);
        }
    }

    /// Ask the page for its full size; the result arrives as `PageData("capture-full", ..)`.
    pub fn capture_full_begin(&self, wv: Option<&WebView>, shared: &WebShared) {
        let Some(wv) = wv else { return };
        let s = shared.clone();
        let _ = wv.evaluate_script_with_callback(
            "JSON.stringify([Math.max(document.documentElement.scrollWidth,document.body?document.body.scrollWidth:0),Math.max(document.documentElement.scrollHeight,document.body?document.body.scrollHeight:0)])",
            move |res| s.push_event(WebEvent::PageData("capture-full".into(), String::new(), res)),
        );
    }

    pub fn capture_full_finish(&mut self, wv: Option<&WebView>, shared: &WebShared, raw: &str) {
        let Some(wv) = wv else { return };
        // `raw` is the JSON-encoded string returned by the script, i.e. "\"[1280,5400]\"".
        let dims: Vec<f64> = serde_json::from_str::<String>(raw)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<f64>>(&s).ok())
            .unwrap_or_default();
        if dims.len() != 2 || dims[0] < 1.0 || dims[1] < 1.0 {
            self.toast(wv, "Could not measure the page for a full capture", None, shared);
            return;
        }
        let (w, h) = (dims[0].min(16_384.0), dims[1].min(16_384.0));
        let params = format!(
            r#"{{"format":"png","captureBeyondViewport":true,"fromSurface":true,"clip":{{"x":0,"y":0,"width":{},"height":{},"scale":1}}}}"#,
            w, h
        );
        let out = self.capture_path();
        web::com::capture_png(wv, &params, out, shared);
    }

    pub fn capture_region(&mut self, args: &str, wv: Option<&WebView>, shared: &WebShared) {
        let Some(wv) = wv else { return };
        let n: Vec<f64> = args.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        if n.len() != 5 || n[2] < 1.0 || n[3] < 1.0 {
            return;
        }
        let scale = n[4].clamp(1.0, 4.0);
        let params = format!(
            r#"{{"format":"png","captureBeyondViewport":true,"fromSurface":true,"clip":{{"x":{},"y":{},"width":{},"height":{},"scale":{}}}}}"#,
            n[0].max(0.0), n[1].max(0.0), n[2], n[3], scale
        );
        let out = self.capture_path();
        web::com::capture_png(wv, &params, out, shared);
    }

    pub fn on_captured(&self, result: &Result<String, String>, wv: Option<&WebView>, shared: &WebShared) {
        let Some(wv) = wv else { return };
        match result {
            Ok(path) => {
                let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                self.toast(wv, &format!("📸 Saved {}", name), Some(("Show in folder", &shared.token, "open-folder")), shared);
            }
            Err(e) => self.toast(wv, &format!("Screenshot failed: {}", e), None, shared),
        }
    }

    pub fn open_capture_folder(&self) {
        #[cfg(windows)]
        {
            let mut cmd = std::process::Command::new("explorer");
            match &self.last_capture {
                Some(p) if std::path::Path::new(p).exists() => {
                    cmd.arg(format!("/select,{}", p));
                }
                _ => {
                    let dir = sys::screenshots_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    cmd.arg(dir);
                }
            }
            let _ = cmd.spawn();
        }
    }

    // ------------------------------------------------------------------ AI

    pub fn ai_request(&self, kind: &str, question: &str, wv: Option<&WebView>, shared: &WebShared) {
        let Some(wv) = wv else { return };
        let (s, k, q) = (shared.clone(), kind.to_string(), question.to_string());
        let _ = wv.evaluate_script_with_callback(
            "(function(){var t=(document.body&&document.body.innerText)||'';return t.slice(0,150000)})()",
            move |res| s.push_event(WebEvent::PageData(format!("ai-{}", k), q.clone(), res)),
        );
    }

    pub fn ai_respond(&self, kind: &str, question: &str, raw: &str, wv: Option<&WebView>) {
        let Some(wv) = wv else { return };
        let text = serde_json::from_str::<String>(raw).unwrap_or_default();
        let js = match kind {
            "summarize" => {
                let s = ai::summarize(&text, 4);
                if s.is_empty() {
                    overlays::ai_result("Summary", &["There is not enough readable text on this page to summarise.".into()], &[])
                } else {
                    overlays::ai_result("Summary", &s, &[])
                }
            }
            "keywords" => {
                let k = ai::keywords(&text, 10);
                if k.is_empty() {
                    overlays::ai_result("Key topics", &["No repeated topics found on this page.".into()], &[])
                } else {
                    overlays::ai_result("Key topics", &[], &k)
                }
            }
            "stats" => {
                let st = ai::stats(&text);
                overlays::ai_result(
                    "Reading stats",
                    &[format!("{} words · {} sentences · about {} min to read", st.words, st.sentences, st.minutes)],
                    &[],
                )
            }
            "ask" => {
                let a = ai::answer(&text, question, 3);
                if a.is_empty() {
                    overlays::ai_result("Answer", &["I could not find anything about that on this page.".into()], &[])
                } else {
                    overlays::ai_result("Best matches on this page", &a, &[])
                }
            }
            _ => return,
        };
        let _ = wv.evaluate_script(&js);
    }
}

pub const MENU: &[overlays::MenuEntry] = &[
    overlays::MenuEntry { emoji: "➕", label: "New Tab", cmd: "newtab", danger: false },
    overlays::MenuEntry { emoji: "🏠", label: "Home Page", cmd: "home", danger: false },
    overlays::MenuEntry { emoji: "", label: "-", cmd: "", danger: false },
    overlays::MenuEntry { emoji: "⭐", label: "Bookmarks", cmd: "bookmarks", danger: false },
    overlays::MenuEntry { emoji: "🕒", label: "History", cmd: "history", danger: false },
    overlays::MenuEntry { emoji: "⬇️", label: "Downloads", cmd: "downloads", danger: false },
    overlays::MenuEntry { emoji: "🧩", label: "Extensions", cmd: "extensions", danger: false },
    overlays::MenuEntry { emoji: "", label: "-", cmd: "", danger: false },
    overlays::MenuEntry { emoji: "🎨", label: "Heritage Themes", cmd: "theme-menu", danger: false },
    overlays::MenuEntry { emoji: "🧹", label: "Clear RAM & Cache", cmd: "clear-ram", danger: false },
    overlays::MenuEntry { emoji: "⚙️", label: "Settings", cmd: "settings", danger: false },
    overlays::MenuEntry { emoji: "ℹ️", label: "About Axomai", cmd: "about", danger: false },
    overlays::MenuEntry { emoji: "", label: "-", cmd: "", danger: false },
    overlays::MenuEntry { emoji: "🚪", label: "Exit Axomai Browser", cmd: "exit", danger: true },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_is_clamped_and_scaled() {
        assert_eq!(anchor_right(1200.0, 1190.0, 1.0), 6.0);
        assert!((anchor_right(1200.0, 1000.0, 1.0) - 196.0).abs() < 0.01);
        assert!((anchor_right(1500.0, 1250.0, 1.25) - 196.0).abs() < 0.01);
    }

    #[test]
    fn default_name_is_never_empty() {
        assert!(!default_profile_name().is_empty());
    }

    #[test]
    fn extension_state_round_trips_through_bits() {
        let mut exts = crate::extensions::create_extensions();
        let mut core = Core::new(None, &mut exts);
        assert!(exts.iter().all(|e| e.enabled));
        core.persist_extensions(None, &exts);
        assert_eq!(core.theme.id, "tea-garden");
    }
}
