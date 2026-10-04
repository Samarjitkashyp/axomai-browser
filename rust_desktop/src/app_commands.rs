//! `axomai://<command>` handling and page loading for `App`.
//!
//! Commands come from three places: our popups (through the token-checked message bridge), our own pages
//! (links such as `axomai://settings`) and keyboard shortcuts. They all end up here, so every action has
//! exactly one implementation.

use crate::actions;
use crate::app::App;
use crate::settings_page::{self, SettingsView};
use crate::ui_shell::PageCtx;
use crate::overlays;
use crate::pages;
use crate::tabs::TabKind;
use crate::toolbar;
use crate::types::SearchEngine;
use crate::viewsource;
use crate::web::{self, Initial, WebShared};

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
    let name = t.strip_suffix(" - Axomai Browser")?;
    Some(match name {
        "Extensions" => ("axomai://extensions", TabKind::Extensions),
        "Settings" => ("about:settings", TabKind::Settings),
        "About" => ("axomai://about", TabKind::About),
        "History" => ("axomai://history", TabKind::Page("history")),
        "Bookmarks" => ("axomai://bookmarks", TabKind::Page("bookmarks")),
        "Downloads" => ("axomai://downloads", TabKind::Page("downloads")),
        "Passwords" => ("axomai://passwords", TabKind::Page("passwords")),
        "Permissions" => ("axomai://permissions", TabKind::Page("permissions")),
        "Reading list" => ("axomai://readinglist", TabKind::Page("readinglist")),
        "Notes" => ("axomai://notes", TabKind::Page("notes")),
        _ => return None,
    })
}

/// `Some(id)` when `s` is `"<prefix><number>"`.
fn tab_arg(cmd: &str, prefix: &str) -> Option<u64> {
    cmd.strip_prefix(prefix)?.parse::<u64>().ok()
}

impl App {
    pub fn ui_file(&self, name: &str) -> String {
        let ui = crate::sys::ui_dir();
        format!("file:///{}", ui.join(name).to_string_lossy().replace('\\', "/"))
    }

    fn anchor_for(&self, pick: impl Fn(&toolbar::ToolbarLayout) -> toolbar::Rect) -> f32 {
        let (w, _) = self.window_size();
        actions::anchor_right(w, pick(&toolbar::toolbar_layout(w)).right(), 1.0)
    }

    // ------------------------------------------------------------------ page loading

    /// The New Tab page, told which theme, weather city and Shield total to show.
    pub fn home_page_url(&self) -> String {
        let blocked = self.hub.shield.total_blocked.load(std::sync::atomic::Ordering::Relaxed);
        let on = self.hub.shield.adblock.load(std::sync::atomic::Ordering::Relaxed) || self.hub.shield.privacy.load(std::sync::atomic::Ordering::Relaxed);
        format!(
            "{}?theme={}&city={}&shield={}&shieldon={}",
            self.ui_file("home.html"),
            self.core.theme.id,
            self.settings.weather_city,
            blocked,
            on as u8
        )
    }

    pub fn page_ctx<'a>(&'a self, shared: &'a WebShared) -> PageCtx<'a> {
        PageCtx { theme: self.core.theme, token: &shared.token, lang: &self.settings.language }
    }

    /// HTML of one of Axomai's generated pages.
    pub fn internal_html(&self, kind: TabKind, shared: &WebShared) -> String {
        let ctx = self.page_ctx(shared);
        match kind {
            TabKind::Extensions => pages::extensions_page(&ctx, &self.extensions),
            TabKind::About => pages::about_page(
                &ctx,
                &pages::AboutInfo {
                    version: env!("CARGO_PKG_VERSION"),
                    webview: wry::webview_version().unwrap_or_else(|_| "unknown".to_string()),
                    data_dir: crate::storage::data_dir().to_string_lossy().to_string(),
                },
            ),
            TabKind::Settings => {
                let download_dir = if self.settings.download_dir.is_empty() {
                    shared.download_dir().to_string_lossy().to_string()
                } else {
                    self.settings.download_dir.clone()
                };
                settings_page::settings_page(
                    &ctx,
                    &SettingsView {
                        settings: &self.settings,
                        engine: self.search_engine,
                        extensions: &self.extensions,
                        theme_id: self.core.theme.id,
                        download_dir,
                        version: env!("CARGO_PKG_VERSION"),
                        is_default: crate::launch::is_default(),
                        site_rules: &self.storage.as_ref().map(|s| s.site_rules()).unwrap_or_default(),
                    },
                )
            }
            TabKind::Page("history") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_history(1000).ok()).unwrap_or_default();
                let urls: Vec<&str> = entries.iter().map(|e| e.url.as_str()).collect();
                let icons = self.icon_map(&urls);
                pages::history_page(&ctx, &entries, &icons)
            }
            TabKind::Page("bookmarks") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_bookmarks().ok()).unwrap_or_default();
                let urls: Vec<&str> = entries.iter().map(|e| e.url.as_str()).collect();
                let icons = self.icon_map(&urls);
                pages::bookmarks_page(&ctx, &entries, &icons)
            }
            TabKind::Page("readinglist") => {
                let items = self.storage.as_ref().map(|s| s.reading_items()).unwrap_or_default();
                let urls: Vec<&str> = items.iter().map(|i| i.url.as_str()).collect();
                let icons = self.icon_map(&urls);
                pages::reading_list_page(&ctx, &items, &icons)
            }
            TabKind::Page("notes") => pages::notes_page(&ctx, &self.storage.as_ref().map(|s| s.notes_all()).unwrap_or_default()),
            TabKind::Page("passwords") => {
                let (logins, never) = self.storage.as_ref().map(|s| (s.list_passwords(), s.pw_never_list())).unwrap_or_default();
                pages::passwords_page(&ctx, &logins, &never)
            }
            TabKind::Page("permissions") => {
                let rows = self.storage.as_ref().map(|s| s.list_permissions()).unwrap_or_default();
                pages::permissions_page(&ctx, &rows)
            }
            TabKind::Page("downloads") => {
                let entries = self.storage.as_ref().and_then(|s| s.get_downloads(200).ok()).unwrap_or_default();
                pages::downloads_page(&ctx, &entries, &self.live_downloads())
            }
            _ => crate::ui_shell::page(&ctx, "", "Axomai", "<p class=\"sub\">This page is not available yet.</p>", ""),
        }
    }

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
        enum Content {
            Url(String),
            Html(String),
        }
        let content = match kind {
            TabKind::Home => Content::Url(self.home_page_url()),
            TabKind::Extensions | TabKind::Settings | TabKind::About | TabKind::Page(_) => Content::Html(self.internal_html(kind, &shared)),
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
            let (w, h) = (w * self.scale.max(0.5), h * self.scale.max(0.5));
            // A page that is loaded straight at creation can start before the document-start scripts (ad blocking,
            // Global Privacy Control, password manager) are registered. So the view starts blank and the address is
            // loaded once the scripts are in place.
            const BLANK: &str = "<!doctype html><meta charset=\"utf-8\"><title></title>";
            let initial = match &content {
                Content::Url(_) => Initial::Html(BLANK),
                Content::Html(h) => Initial::Html(h),
            };
            self.webview = web::build_webview(&self.window, (w, h), self.chrome_top() * self.scale.max(0.5), initial, &shared, bg);
            if self.webview.is_some() {
                self.fit_active_view();
                self.apply_privacy_settings();
                let doc = self.tabs[idx].doc.clone();
                self.core.apply_extensions(self.webview.as_ref(), &doc, &shared, &self.extensions);
                self.tabs[idx].ext_gen = self.core.ext_generation;
                if let (Content::Url(u), Some(wv)) = (&content, &self.webview) {
                    let _ = wv.load_url(u);
                }
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

    /// Navigate the active tab to an ordinary web address.
    pub fn navigate_active(&mut self, url: &str) {
        let idx = self.active;
        let t = &mut self.tabs[idx];
        t.prev_nav = Some((t.kind, t.url.clone(), t.title.clone()));
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
            "permissions" => TabKind::Page("permissions"),
            "readinglist" => TabKind::Page("readinglist"),
            "notes" => TabKind::Page("notes"),
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
        } else if kind == "pdf" {
            self.on_pdf_done(question, payload == "1");
        } else if kind == "bookmark-file" {
            self.import_bookmark_file(payload);
        } else if kind == "download-target" {
            if let Ok(id) = question.parse::<u64>() {
                self.on_download_target(id, payload);
            }
        } else if kind == "download-dir" {
            self.apply_setting("download_dir", payload);
        } else if kind == "capture-full" {
            self.core.capture_full_finish(self.webview.as_ref(), &shared, payload);
        } else if kind == "site-icon" {
            self.site_icon_arrived(question, payload);
        } else if kind == "news" {
            self.news_arrived(idx, question, payload);
        } else if let Some(k) = kind.strip_prefix("ai-") {
            self.core.ai_respond(k, question, payload, self.webview.as_ref());
        }
    }

    pub fn command(&mut self, cmd: &str) {
        let shared = self.shared();
        // ---- tabs and windows
        if cmd == "tab-search" {
            self.open_tab_search();
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-go/") {
            self.tab_search_go(id);
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-group-prompt/") {
            self.open_group_prompt(id);
            return;
        }
        if let Some(rest) = cmd.strip_prefix("tab-group-new/") {
            let mut p = rest.splitn(3, '/');
            if let (Some(id), Some(color), Some(name)) = (p.next().and_then(|v| v.parse::<u64>().ok()), p.next().and_then(|v| v.parse::<usize>().ok()), p.next()) {
                self.tab_group_new(id, color, &url_decode(name));
            }
            return;
        }
        if let Some(rest) = cmd.strip_prefix("tab-group-add/") {
            if let Some((id, gid)) = rest.split_once('/') {
                if let (Ok(id), Ok(gid)) = (id.parse::<u64>(), gid.parse::<u32>()) {
                    self.tab_group_add(id, gid);
                }
            }
            return;
        }
        if let Some(id) = tab_arg(cmd, "tab-ungroup/") {
            self.tab_ungroup(id);
            return;
        }
        if let Some(id) = tab_arg(cmd, "page-focus/") {
            self.on_pane_focus(id);
            self.command("page-focus");
            return;
        }
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
            "split-view" => self.toggle_split(),
            "reading-add" => self.reading_add(),
            "site-mute" => self.site_mute_current(),
            "site-block-current" => self.site_block_current(),
            "note-new" => self.note_new(),
            "page-focus" => {
                // Giving the web view focus ourselves (tab switch) echoes back as a focus event; only a real click counts.
                if self.addr_focused && self.addr_focus_at.elapsed() > std::time::Duration::from_millis(600) {
                    self.addr_focused = false;
                    self.addr_selected = false;
                    self.sugg_hide_at = Some(std::time::Instant::now());
                    self.redraw = true;
                }
            }
            "reload" => self.reload_active(),
            "reload-hard" => self.hard_reload(),
            "zoom-in" => self.zoom_command("in"),
            "zoom-out" => self.zoom_command("out"),
            "zoom-reset" => self.zoom_command("reset"),
            "save-pdf" => self.save_pdf(),
            "bookmark-bar-toggle" => self.toggle_bookmark_bar(),
            "back" => {
                if let Some(wv) = &self.webview {
                    web::com::go_back(wv);
                }
            }
            "forward" => {
                if let Some(wv) = &self.webview {
                    web::com::go_forward(wv);
                }
            }
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
            "home" | "extensions" | "settings" | "themes" | "about" | "history" | "bookmarks" | "downloads" | "passwords" | "permissions" | "readinglist" | "notes" => {
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
            "settings-reset" => self.reset_settings(),
            "https-back" => self.https_back(),
            "default-browser" => {
                let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let ok = crate::launch::register(&exe);
                if let Some(wv) = &self.webview {
                    let msg = if ok { "Choose Axomai Browser in the list that opened" } else { "Could not register Axomai with Windows" };
                    self.core.toast(wv, msg, None, &shared);
                }
                if ok {
                    crate::launch::open_default_apps_settings();
                }
                self.refresh_internal_page();
            }
            "open-ai" => {
                let right = self.anchor_for(|l| l.ai);
                let title = self.tabs[self.active].title.clone();
                self.core.open_ai(self.webview.as_ref(), &shared, right, &title);
            }
            "bm-import-chrome" => self.import_chrome_bookmarks(),
            "bm-import-file" => self.pick_bookmark_file(),
            "bm-export" => self.export_bookmarks(),
            "pick-download-dir" => self.pick_download_dir(),
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
            self.refresh_internal_page();
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
        } else if let Some(rest) = cmd.strip_prefix("set/") {
            let (key, val) = rest.split_once('/').unwrap_or((rest, ""));
            self.apply_setting(key, &url_decode(val));
        } else if let Some(rest) = cmd.strip_prefix("clear-data-range/") {
            let (range, flags) = rest.split_once('/').unwrap_or((rest, ""));
            self.clear_data_range(range, flags);
        } else if let Some(id) = cmd.strip_prefix("history-delete/").and_then(|v| v.parse::<i64>().ok()) {
            if let Some(s) = &self.storage {
                let _ = s.delete_history(id);
            }
            self.load_active_page();
        } else if let Some(rest) = cmd.strip_prefix("bm-move/") {
            let (id, folder) = rest.split_once('/').unwrap_or((rest, ""));
            self.bookmark_move(id.parse().unwrap_or(-1), &url_decode(folder));
        } else if let Some(rest) = cmd.strip_prefix("bm-rename/") {
            let (id, title) = rest.split_once('/').unwrap_or((rest, ""));
            self.bookmark_rename(id.parse().unwrap_or(-1), &url_decode(title));
        } else if let Some(folder) = cmd.strip_prefix("bm-folder-delete/") {
            if let Some(s) = &self.storage {
                let _ = s.delete_folder(&url_decode(folder), crate::bookmarks_io::DEFAULT_FOLDER);
            }
            self.bookmarks_changed();
        } else if let Some((action, arg)) = cmd.strip_prefix("perm-").and_then(|r| r.split_once('/')) {
            self.permission_command(action, &url_decode(arg));
        } else if let Some((action, id)) = cmd.strip_prefix("reading-").and_then(|r| r.rsplit_once('/')).and_then(|(a, i)| i.parse::<i64>().ok().map(|i| (a, i))) {
            self.reading_command(action, id);
        } else if let Some((action, host)) = ["block", "unblock", "mute", "unmute"].iter().find_map(|a| cmd.strip_prefix(&format!("site-{}/", a)).map(|h| (*a, h))) {
            let host = url_decode(host);
            self.site_rule_command(action, &host);
        } else if let Some(id) = cmd.strip_prefix("note-delete/").and_then(|v| v.parse::<i64>().ok()) {
            self.note_command("delete", id);
        } else if let Some(cat) = cmd.strip_prefix("news/") {
            self.news_request(cat);
        } else if let Some(url) = cmd.strip_prefix("qr-for/") {
            let right = self.anchor_for(|l| l.qr);
            self.core.open_qr(self.webview.as_ref(), &shared, right, &url_decode(url));
        } else if let Some(url) = cmd.strip_prefix("https-continue/") {
            self.https_continue(&url_decode(url));
        } else if let Some((action, arg)) = cmd.strip_prefix("pw-").and_then(|r| r.split_once('/')) {
            self.password_command(action, &url_decode(arg));
        } else if let Some((action, id)) = cmd.strip_prefix("dl-").and_then(|r| r.split_once('/')).and_then(|(a, i)| i.parse::<i64>().ok().map(|i| (a, i))) {
            self.download_command(action, id);
        } else if let Some(row) = cmd.strip_prefix("suggest/").and_then(|v| v.parse::<usize>().ok()) {
            self.open_suggestion(row);
        } else if let Some(q) = cmd.strip_prefix("search/") {
            let url = format!("{}{}", self.search_engine.js_search_template(), q);
            self.navigate_active(&url);
        } else if let Some(id) = cmd.strip_prefix("remove-bookmark/").and_then(|v| v.parse::<i64>().ok()) {
            if let Some(s) = &self.storage {
                let _ = s.remove_bookmark(id);
            }
            self.bookmarks_changed();
        }
    }

}

impl App {
    /// Re-render the active tab if it is one of Axomai's own pages (after a theme / language / data change).
    pub fn refresh_internal_page(&mut self) {
        if matches!(self.tabs[self.active].kind, TabKind::Home | TabKind::Extensions | TabKind::Settings | TabKind::About | TabKind::Page(_)) {
            self.load_active_page();
        }
    }

    fn notify_settings_page(&self, ok: bool) {
        if matches!(self.tabs[self.active].kind, TabKind::Settings) {
            if let Some(wv) = &self.webview {
                let _ = wv.evaluate_script(&format!("window.__saved&&__saved({})", ok));
            }
        }
    }

    /// One `set/<key>/<value>` message from the Settings page.
    pub fn apply_setting(&mut self, key: &str, value: &str) {
        let ok = if key == "search_engine" {
            self.search_engine = match value {
                "Bing" => SearchEngine::Bing,
                "Yahoo" => SearchEngine::Yahoo,
                "DuckDuckGo" => SearchEngine::DuckDuckGo,
                "Google" => SearchEngine::Google,
                _ => {
                    self.notify_settings_page(false);
                    return;
                }
            };
            if let Some(s) = &self.storage {
                let _ = s.set_setting("search_engine", self.search_engine.name());
            }
            true
        } else {
            self.settings.set(self.storage.as_ref(), key, value)
        };
        if ok {
            match key {
                "download_dir" => {
                    let dir = if self.settings.download_dir.is_empty() {
                        default_download_dir()
                    } else {
                        std::path::PathBuf::from(&self.settings.download_dir)
                    };
                    let _ = std::fs::create_dir_all(&dir);
                    self.hub.set_download_dir(dir);
                    self.refresh_internal_page();
                }
                "language" | "weather_city" => self.refresh_internal_page(),
                "https_only" | "tracking" => self.apply_privacy_settings(),
                "gpc" => {
                    self.apply_privacy_settings();
                    self.core.set_gpc(self.settings.gpc);
                    self.sync_extensions_for_active();
                }
                "dark_sites" | "cookie_banners" | "youtube_ads" | "print_clean" => {
                    self.core.set_tweaks(self.settings.tweaks());
                    self.sync_extensions_for_active();
                }
                "password_manager" => {
                    self.core.set_passwords(self.settings.password_manager);
                    self.sync_extensions_for_active();
                }
                "bookmark_bar" => {
                    self.refresh_bar();
                    self.fit_active_view();
                }
                _ => {}
            }
            self.redraw = true;
        }
        self.notify_settings_page(ok);
    }

    pub fn reset_settings(&mut self) {
        if let Some(s) = &self.storage {
            for key in [
                "startup", "restore_session", "home_url", "password_manager", "weather_city", "sleep_minutes", "download_dir", "ask_download", "bookmark_bar",
                "https_only", "tracking", "gpc", "dark_sites", "cookie_banners", "youtube_ads", "print_clean", "language", "search_engine", "theme", "ext_enabled",
            ] {
                let _ = s.delete_setting(key);
            }
        }
        self.settings = crate::settings::Settings::default();
        self.core.set_passwords(true);
        self.core.set_gpc(true);
        self.core.set_tweaks(self.settings.tweaks());
        self.apply_privacy_settings();
        self.search_engine = SearchEngine::Google;
        self.core.set_theme("tea-garden", None);
        // Switch every extension back on through the normal path so the web views follow.
        let off: Vec<usize> = self.extensions.iter().enumerate().filter(|(_, e)| !e.enabled).map(|(i, _)| i).collect();
        for i in off {
            self.command(&format!("ext-toggle/{}", i));
        }
        self.hub.set_download_dir(default_download_dir());
        self.refresh_internal_page();
        self.redraw = true;
    }

    /// Ask for a folder on a helper thread (the native dialog runs its own message loop, which must not run inside
    /// the event handler); the answer comes back as a `download-dir` event.
    pub fn pick_download_dir(&mut self) {
        let shared = self.shared();
        let start = shared.download_dir();
        std::thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new().set_title("Choose where downloads are saved").set_directory(start).pick_folder() {
                shared.push_event(web::WebEvent::PageData("download-dir".into(), String::new(), p.to_string_lossy().to_string()));
            }
        });
    }

    /// "Clear browsing data" from the Settings page. `flags`: h = history, d = download list, k = cookies, c = cache.
    pub fn clear_data_range(&mut self, range: &str, flags: &str) {
        let seconds: Option<i64> = match range {
            "hour" => Some(3600),
            "day" => Some(86_400),
            "week" => Some(7 * 86_400),
            "month" => Some(28 * 86_400),
            "all" => None,
            _ => return,
        };
        if flags.contains('h') {
            if let Some(s) = &self.storage {
                let _ = s.clear_history_since(seconds);
            }
        }
        if flags.contains('d') {
            if let Some(s) = &self.storage {
                let _ = s.clear_downloads_since(seconds);
            }
        }
        let (cookies, cache) = (flags.contains('k'), flags.contains('c'));
        if cookies || cache {
            if let Some(wv) = &self.webview {
                web::com::clear_browsing_data(wv, cookies, cache, seconds.map(|s| s as u64));
            }
        }
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            self.core.toast(wv, "Browsing data cleared", None, &shared);
        }
        self.refresh_internal_page();
    }
}

impl App {
    /// Push the privacy switches to every tab (shared flags) and the tracking level to the web engine profile.
    pub fn apply_privacy_settings(&self) {
        let shield = &self.hub.shield;
        shield.https_only.store(self.settings.https_only, std::sync::atomic::Ordering::SeqCst);
        shield.gpc.store(self.settings.gpc, std::sync::atomic::Ordering::SeqCst);
        if let Some(wv) = &self.webview {
            web::com::set_tracking_level(wv, self.settings.tracking.key());
        }
    }

    /// HTTPS-only mode cancelled an http:// navigation in tab `idx`; go to the https:// address instead.
    pub fn on_https_upgrade(&mut self, idx: usize, https: &str) {
        let t = &mut self.tabs[idx];
        t.kind = TabKind::Web;
        t.url = https.to_string();
        t.title = crate::blocklist::host_of(https);
        if let Some(wv) = self.view_of(idx) {
            let _ = wv.load_url(https);
        }
        self.redraw = true;
    }

    /// The https:// version did not load: show a warning page that offers the http:// original.
    pub fn on_https_failed(&mut self, idx: usize, http_url: &str) {
        let shared = self.tabs[idx].shared.clone();
        let host = crate::blocklist::host_of(http_url);
        let html = pages::https_warning_page(&self.page_ctx(&shared), &host, http_url);
        let t = &mut self.tabs[idx];
        t.url = http_url.to_string();
        t.title = "Connection not secure".to_string();
        // The engine shows its own error page right after reporting the failure; ours replaces it a moment later.
        self.https_warn.push((self.tabs[idx].id, html, std::time::Instant::now()));
        self.redraw = true;
    }

    /// Put the warning page in place once the engine's error page has appeared.
    pub fn show_https_warnings(&mut self) {
        if self.https_warn.iter().all(|(_, _, at)| at.elapsed() < std::time::Duration::from_millis(350)) {
            return;
        }
        let due: Vec<(u64, String, std::time::Instant)> = std::mem::take(&mut self.https_warn);
        for (tab_id, html, at) in due {
            if at.elapsed() < std::time::Duration::from_millis(350) {
                self.https_warn.push((tab_id, html, at));
            } else if let Some(idx) = self.idx_of_id(tab_id) {
                if let Some(wv) = self.view_of(idx) {
                    let _ = wv.load_html(&html);
                }
                // The address bar must not keep claiming the https:// address that failed.
                let t = &mut self.tabs[idx];
                if let Some(rest) = t.url.strip_prefix("https") {
                    t.url = format!("http{}", rest);
                }
            }
        }
    }

    /// "Continue to the site (not secure)": remember the host for this session and open the http:// address.
    pub fn https_continue(&mut self, http_url: &str) {
        if !http_url.starts_with("http://") {
            return;
        }
        if let Ok(mut ex) = self.hub.shield.https_exempt.lock() {
            ex.insert(crate::blocklist::host_of(http_url).to_ascii_lowercase());
        }
        self.navigate_active(http_url);
    }

    pub fn https_back(&mut self) {
        match &self.webview {
            Some(wv) if self.core.can_back => web::com::go_back(wv),
            _ => self.open_internal("home"),
        }
    }
}

impl App {
    /// A normal window answers other launches: it says it is alive, and opens addresses they hand over.
    pub fn serve_other_launches(&mut self) {
        if self.private_window {
            return;
        }
        let dir = crate::storage::data_dir();
        if self.last_beat.elapsed() > std::time::Duration::from_secs(1) {
            self.last_beat = std::time::Instant::now();
            crate::launch::heartbeat(&dir);
        }
        if self.last_handoff_poll.elapsed() < std::time::Duration::from_millis(400) {
            return;
        }
        self.last_handoff_poll = std::time::Instant::now();
        let urls = crate::launch::take_handed_over(&dir);
        if urls.is_empty() {
            return;
        }
        for u in urls {
            let at = self.tabs.len();
            self.open_tab_at(&u, at, true);
        }
        self.window.set_minimized(false);
        self.window.set_focus();
    }
}

impl App {
    /// The New Tab page asked for the headlines of one category: answer from the cache or fetch them.
    pub fn news_request(&mut self, category: &str) {
        if !crate::news::CATEGORIES.iter().any(|(k, _)| *k == category) || !matches!(self.tabs[self.active].kind, TabKind::Home) {
            return;
        }
        if let Some((at, json)) = self.news_cache.get(category) {
            if at.elapsed().as_secs() < crate::news::CACHE_SECONDS {
                let js = format!("window.__news&&window.__news({},{})", serde_json::to_string(category).unwrap_or_default(), json);
                if let Some(wv) = &self.webview {
                    let _ = wv.evaluate_script(&js);
                }
                return;
            }
        }
        if !self.news_pending.insert(category.to_string()) {
            return;
        }
        let (shared, cat) = (self.shared(), category.to_string());
        std::thread::spawn(move || {
            let json = match crate::news::fetch(&cat) {
                Ok(items) if !items.is_empty() => crate::news::items_json(&items),
                _ => crate::news::failure_json(),
            };
            shared.push_event(web::WebEvent::PageData("news".into(), cat, json));
        });
    }

    fn news_arrived(&mut self, idx: usize, category: &str, json: &str) {
        self.news_pending.remove(category);
        if json.contains("\"ok\":true") {
            self.news_cache.insert(category.to_string(), (std::time::Instant::now(), json.to_string()));
        }
        if matches!(self.tabs[idx].kind, TabKind::Home) {
            if let Some(wv) = self.view_of(idx) {
                let _ = wv.evaluate_script(&format!("window.__news&&window.__news({},{})", serde_json::to_string(category).unwrap_or_default(), json));
            }
        }
    }
}

pub fn default_download_dir() -> std::path::PathBuf {
    match std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        Some(home) => std::path::PathBuf::from(home).join("Downloads"),
        None => std::path::PathBuf::from("."),
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
