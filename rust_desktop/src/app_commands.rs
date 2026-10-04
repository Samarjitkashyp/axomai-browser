//! `axomai://<command>` handling and page loading for `App`.
//!
//! Commands come from three places: our popups (through the token-checked message bridge), our own pages
//! (links such as `axomai://settings`) and keyboard shortcuts. They all end up here, so every action has
//! exactly one implementation.

use crate::actions;
use crate::app::App;
use crate::internal_pages;
use crate::overlays;
use crate::pages;
use crate::tabs::TabKind;
use crate::toolbar;
use crate::types::{SearchEngine, CHROME_TOP};
use crate::viewsource;
use crate::web::{self, Initial};

/// Percent-decoding for command arguments produced by `encodeURIComponent` in the popups.
pub fn url_decode(s: &str) -> String {
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 1 && i + 2 <= b.len().saturating_sub(1) {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Map the `<title>` of one of our generated pages to (address, kind).
pub fn internal_page_for_title(t: &str) -> Option<(&'static str, TabKind)> {
    match t {
        "Extensions - Axomai Browser" => Some(("axomai://extensions", TabKind::Extensions)),
        "Settings - Axomai Browser" => Some(("about:settings", TabKind::Settings)),
        "History - Axomai Browser" => Some(("axomai://history", TabKind::Page("history"))),
        "Bookmarks - Axomai Browser" => Some(("axomai://bookmarks", TabKind::Page("bookmarks"))),
        "Downloads - Axomai Browser" => Some(("axomai://downloads", TabKind::Page("downloads"))),
        _ => None,
    }
}

/// `Some(id)` when `s` is `"<prefix><number>"`.
fn tab_arg(cmd: &str, prefix: &str) -> Option<u64> {
    cmd.strip_prefix(prefix)?.parse::<u64>().ok()
}

impl App {
    pub fn ui_file(&self, name: &str) -> String {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or(std::path::Path::new(".")).join("ui");
        format!("file:///{}", ui.join(name).to_string_lossy().replace('\\', "/"))
    }

    fn anchor_for(&self, pick: impl Fn(&toolbar::ToolbarLayout) -> toolbar::Rect) -> f32 {
        let (w, _) = self.window_size();
        actions::anchor_right(w, pick(&toolbar::toolbar_layout(w)).right(), self.scale)
    }

    // ------------------------------------------------------------------ page loading

    /// (Re)load whatever the active tab is supposed to show, creating its web view if it has none yet.
    pub fn load_active_page(&mut self) {
        if self.webview.is_none() && !self.safe {
            // Creating a web view is not safe in this event-loop state; a user event will call us again.
            self.view_pending = true;
            let _ = self.proxy.send_event(());
            self.redraw = true;
            return;
        }
        let idx = self.active;
        let kind = self.tabs[idx].kind;
        let shared = self.tabs[idx].shared.clone();
        let restore = self.setting("restore_session").map(|v| v == "true").unwrap_or(false);
        enum Content {
            Url(String),
            Html(String),
        }
        let content = match kind {
            TabKind::Home => Content::Url(self.ui_file("home.html")),
            TabKind::About => Content::Url(self.ui_file("index.html")),
            TabKind::Extensions => Content::Html(internal_pages::extensions_page_html(&self.extensions)),
            TabKind::Settings => Content::Html(internal_pages::settings_page_html(self.search_engine, restore)),
            TabKind::Page("history") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_history(100).ok()).unwrap_or_default();
                Content::Html(pages::history_page_html(&entries))
            }
            TabKind::Page("bookmarks") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_bookmarks().ok()).unwrap_or_default();
                Content::Html(pages::bookmarks_page_html(&entries))
            }
            TabKind::Page("downloads") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_downloads(100).ok()).unwrap_or_default();
                Content::Html(pages::downloads_page_html(&entries))
            }
            TabKind::Page(_) => Content::Html("<html><body style=\"font-family:sans-serif;padding:32px\">This page is not available yet.</body></html>".into()),
            TabKind::Source => {
                let target = self.tabs[idx].url.strip_prefix("view-source:").unwrap_or("").to_string();
                match &self.tabs[idx].source_html {
                    Some(html) => Content::Html(html.clone()),
                    None => Content::Html(viewsource::loading_page(&target)),
                }
            }
            TabKind::Web => Content::Url(self.tabs[idx].url.clone()),
        };
        let bg = if matches!(kind, TabKind::Web) { (255, 255, 255, 255) } else { (15, 23, 42, 255) };
        if self.webview.is_none() {
            let (w, h) = self.window_size();
            let initial = match &content {
                Content::Url(u) => Initial::Url(u),
                Content::Html(h) => Initial::Html(h),
            };
            self.webview = web::build_webview(&self.window, (w, h), CHROME_TOP, initial, &shared, bg);
            if self.webview.is_some() {
                let doc = self.tabs[idx].doc.clone();
                self.core.apply_extensions(self.webview.as_ref(), &doc, &shared, &self.extensions);
                self.tabs[idx].ext_gen = self.core.ext_generation;
                if self.tabs[idx].muted {
                    if let Some(wv) = &self.webview {
                        web::com::set_muted(wv, true);
                    }
                }
            }
        } else if let Some(wv) = &self.webview {
            let _ = wv.set_background_color(bg);
            match &content {
                Content::Url(u) => {
                    let _ = wv.load_url(u);
                }
                Content::Html(h) => {
                    let _ = wv.load_html(h);
                }
            }
            let _ = wv.set_visible(true);
        }
        if kind == TabKind::Source && self.tabs[idx].source_html.is_none() {
            if let Some(target) = self.tabs[idx].url.strip_prefix("view-source:") {
                viewsource::fetch(target.to_string(), &shared);
            }
        }
        self.redraw = true;
    }

    fn setting(&self, key: &str) -> Option<String> {
        self.storage.as_ref().and_then(|s| s.get_setting(key).ok().flatten())
    }

    /// Navigate the active tab to an ordinary web address.
    pub fn navigate_active(&mut self, url: &str) {
        let idx = self.active;
        let t = &mut self.tabs[idx];
        t.kind = TabKind::Web;
        t.url = url.to_string();
        t.title = crate::blocklist::host_of(url);
        t.favicon = None;
        if let Some(wv) = &self.webview {
            let _ = wv.set_background_color((255, 255, 255, 255));
            let _ = wv.load_url(url);
            let _ = wv.set_visible(true);
            self.redraw = true;
        } else {
            self.load_active_page();
        }
    }

    /// Show one of the browser's own pages in the active tab ("home", "settings", "history", ...).
    pub fn open_internal(&mut self, name: &str) {
        let kind = match name {
            "home" | "newtab" => TabKind::Home,
            "settings" | "themes" => TabKind::Settings,
            "extensions" => TabKind::Extensions,
            "history" => TabKind::Page("history"),
            "bookmarks" => TabKind::Page("bookmarks"),
            "downloads" => TabKind::Page("downloads"),
            "passwords" => TabKind::Page("passwords"),
            "about" => TabKind::About,
            _ => return,
        };
        let idx = self.active;
        let t = &mut self.tabs[idx];
        t.kind = kind;
        t.url = kind.address().unwrap_or_default();
        t.title = kind.default_title().to_string();
        t.favicon = None;
        self.addr_focused = false;
        self.load_active_page();
    }

    /// Open the raw HTML of `url` in a new tab.
    pub fn open_view_source(&mut self, url: &str) {
        let address = format!("view-source:{}", url);
        let at = self.tabs.len();
        self.open_tab_at(&address, at, true);
    }

    // --------------------------------------------------------------------- dispatch

    pub fn process_commands(&mut self) {
        for raw in self.hub.drain_nav() {
            if let Some(cmd) = raw.strip_prefix("axomai://") {
                let cmd = cmd.to_string();
                self.command(&cmd);
            }
        }
    }

    pub fn on_page_data(&mut self, idx: usize, kind: &str, question: &str, payload: &str) {
        let shared = self.shared();
        if kind == "viewsource" {
            // `question` carries the page address, `payload` the finished viewer page.
            // The web view may not exist yet (it is created on a user event), so keep the page on the tab too.
            if let Some(t) = self.tabs.get_mut(idx) {
                t.source_html = Some(payload.to_string());
                t.title = format!("Source: {}", crate::blocklist::host_of(question));
            }
            if let Some(wv) = self.view_of(idx) {
                let _ = wv.set_background_color((15, 23, 42, 255));
                let _ = wv.load_html(payload);
            }
            self.redraw = true;
        } else if kind == "capture-full" {
            self.core.capture_full_finish(self.webview.as_ref(), &shared, payload);
        } else if let Some(k) = kind.strip_prefix("ai-") {
            self.core.ai_respond(k, question, payload, self.webview.as_ref());
        }
    }

    pub fn command(&mut self, cmd: &str) {
        let shared = self.shared();
        // ---- tabs and windows
        if let Some(id) = tab_arg(cmd, "tab-reload/") {
            if let Some(i) = self.idx_of_id(id) {
                if i == self.active {
                    self.reload_active();
                } else if let Some(wv) = self.view_of(i) {
                    web::com::reload(wv);
                }
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-duplicate/") {
            if let Some(i) = self.idx_of_id(id) {
                self.duplicate_tab(i);
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-pin/") {
            if let Some(i) = self.idx_of_id(id) {
                self.toggle_pin(i);
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-mute/") {
            if let Some(i) = self.idx_of_id(id) {
                self.toggle_mute(i);
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-close/") {
            if let Some(i) = self.idx_of_id(id) {
                self.close_tab(i);
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-close-others/") {
            if let Some(i) = self.idx_of_id(id) {
                self.close_others(i);
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-close-right/") {
            if let Some(i) = self.idx_of_id(id) {
                self.close_right_of(i);
            }
            return;
        }
        if let Some(n) = cmd.strip_prefix("tab-index/").and_then(|v| v.parse::<usize>().ok()) {
            self.activate_number(n);
            return;
        }

        match cmd {
            "newtab" => self.new_tab(),
            "closetab" => self.close_tab(self.active),
            "reopen-tab" => self.reopen_closed_tab(),
            "tab-next" => self.activate_relative(1),
            "tab-prev" => self.activate_relative(-1),
            "new-window" => spawn_window(false),
            "new-incognito" => spawn_window(true),
            "focus-url" => self.focus_address(),
            "page-focus" => {
                // Giving the web view focus ourselves (tab switch) echoes back as a focus event; only a real click counts.
                if self.addr_focused && self.addr_focus_at.elapsed() > std::time::Duration::from_millis(600) {
                    self.addr_focused = false;
                    self.addr_selected = false;
                    self.redraw = true;
                }
            }
            "reload" => self.reload_active(),
            "bookmark-toggle" => self.toggle_bookmark_now(),
            "fullscreen" => self.toggle_fullscreen(),
            "print" => {
                if let Some(wv) = &self.webview {
                    let _ = wv.print();
                }
            }
            "find" => {
                if let Some(wv) = &self.webview {
                    let _ = wv.evaluate_script(&overlays::find_popup(self.core.theme));
                }
            }
            "viewsource-current" => {
                let t = &self.tabs[self.active];
                let target = match t.kind {
                    TabKind::Web if t.url.starts_with("http") => Some(t.url.clone()),
                    TabKind::Source => t.url.strip_prefix("view-source:").map(|s| s.to_string()),
                    _ => None,
                };
                match target {
                    Some(u) if t.kind == TabKind::Source => {
                        // Already looking at a source page: refresh it instead of stacking another one.
                        viewsource::fetch(u, &shared);
                    }
                    Some(u) => self.open_view_source(&u),
                    None => {
                        if let Some(wv) = &self.webview {
                            self.core.toast(wv, "View source works on web pages", None, &shared);
                        }
                    }
                }
            }
            "exit" => {
                self.save_session();
                self.exit = true;
            }

            // ---- browser pages
            "home" | "extensions" | "settings" | "themes" | "about" | "history" | "bookmarks" | "downloads" | "passwords" => {
                self.open_internal(cmd)
            }

            // ---- toolbar popups reached from inside other popups
            "shield" => {
                let right = self.anchor_for(|l| l.shield);
                self.core.open_shield(self.webview.as_ref(), &shared, &self.extensions, right);
            }
            "theme-menu" => {
                let right = self.anchor_for(|l| l.theme);
                self.core.open_theme_menu(self.webview.as_ref(), &shared, right);
            }
            "clear-ram" => self.core.free_memory(self.webview.as_ref(), true),
            "open-folder" => self.core.open_capture_folder(),
            "clear-data" => {
                self.core.clear_browsing_data(self.webview.as_ref(), self.storage.as_ref(), &shared);
                self.redraw = true;
            }
            "clear-data-dialog" => self.open_internal("settings"),
            "capture/visible" => self.core.capture_visible(self.webview.as_ref(), &shared),
            "capture/full" => self.core.capture_full_begin(self.webview.as_ref(), &shared),
            "clear-history" => {
                if let Some(s) = &self.storage {
                    let _ = s.clear_history();
                }
                self.open_internal("history");
            }
            "clear-downloads" => {
                if let Some(s) = &self.storage {
                    let _ = s.clear_downloads();
                }
                self.open_internal("downloads");
            }
            "add-bookmark" => self.toggle_bookmark_now(),
            _ => self.command_with_argument(cmd),
        }
    }

    fn command_with_argument(&mut self, cmd: &str) {
        let shared = self.shared();
        if let Some(arg) = cmd.strip_prefix("viewsource/") {
            let target = url_decode(arg);
            if target.starts_with("http://") || target.starts_with("https://") {
                self.open_view_source(&target);
            }
        } else if let Some(arg) = cmd.strip_prefix("ext-toggle/") {
            if let Ok(i) = arg.parse::<usize>() {
                let idx = self.active;
                let doc = self.tabs[idx].doc.clone();
                self.core.toggle_extension(i, self.webview.as_ref(), &doc, &shared, &mut self.extensions, self.storage.as_ref());
                self.tabs[idx].ext_gen = self.core.ext_generation;
                if self.tabs[idx].kind == TabKind::Extensions {
                    self.load_active_page();
                }
                self.redraw = true;
            }
        } else if let Some(arg) = cmd.strip_prefix("ext-run/") {
            if let Ok(i) = arg.parse::<usize>() {
                let right = self.anchor_for(|l| l.extensions);
                self.core.run_extension(i, self.webview.as_ref(), &shared, &self.extensions, right);
            }
        } else if let Some(arg) = cmd.strip_prefix("snip/") {
            self.core.capture_region(arg, self.webview.as_ref(), &shared);
        } else if let Some(arg) = cmd.strip_prefix("theme/") {
            self.core.set_theme(arg, self.storage.as_ref());
            self.redraw = true;
        } else if let Some(arg) = cmd.strip_prefix("profile-name/") {
            self.core.set_profile_name(&url_decode(arg), self.storage.as_ref());
            self.redraw = true;
        } else if let Some(rest) = cmd.strip_prefix("ai/") {
            let (kind, arg) = rest.split_once('/').unwrap_or((rest, ""));
            self.core.ai_request(kind, &url_decode(arg), self.webview.as_ref(), &shared);
        } else if let Some(name) = cmd.strip_prefix("set-engine/") {
            self.search_engine = match name {
                "Bing" => SearchEngine::Bing,
                "Yahoo" => SearchEngine::Yahoo,
                "DuckDuckGo" => SearchEngine::DuckDuckGo,
                _ => SearchEngine::Google,
            };
            if let Some(s) = &self.storage {
                let _ = s.set_setting("search_engine", self.search_engine.name());
            }
            if self.tabs[self.active].kind == TabKind::Settings {
                self.load_active_page();
            }
        } else if let Some(val) = cmd.strip_prefix("set-restore-session/") {
            if let Some(s) = &self.storage {
                let _ = s.set_setting("restore_session", val);
            }
            if self.tabs[self.active].kind == TabKind::Settings {
                self.load_active_page();
            }
        } else if let Some(q) = cmd.strip_prefix("search/") {
            let url = format!("{}{}", self.search_engine.js_search_template(), q);
            self.navigate_active(&url);
        } else if let Some(id) = cmd.strip_prefix("remove-bookmark/").and_then(|v| v.parse::<i64>().ok()) {
            if let Some(s) = &self.storage {
                let _ = s.remove_bookmark(id);
            }
            self.open_internal("bookmarks");
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        self.window.set_fullscreen(if self.fullscreen { Some(tao::window::Fullscreen::Borderless(None)) } else { None });
        self.redraw = true;
    }
}

/// Start another copy of the browser (a new window, optionally in private mode).
fn spawn_window(incognito: bool) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        if incognito {
            cmd.arg("--incognito");
        }
        let _ = cmd.spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_decode_handles_utf8_and_bad_escapes() {
        assert_eq!(url_decode("Asha%20Devi"), "Asha Devi");
        assert_eq!(url_decode("%E0%A6%85%E0%A6%B8%E0%A6%AE"), "\u{985}\u{9b8}\u{9ae}");
        assert_eq!(url_decode("100%"), "100%");
        assert_eq!(url_decode("%zz%4"), "%zz%4");
    }

    #[test]
    fn internal_titles_map_to_pages() {
        assert_eq!(internal_page_for_title("History - Axomai Browser").map(|t| t.1), Some(TabKind::Page("history")));
        assert!(internal_page_for_title("Some Website").is_none());
    }

    #[test]
    fn tab_arguments_parse() {
        assert_eq!(tab_arg("tab-close/42", "tab-close/"), Some(42));
        assert_eq!(tab_arg("tab-close/x", "tab-close/"), None);
        assert_eq!(tab_arg("newtab", "tab-close/"), None);
    }
}
