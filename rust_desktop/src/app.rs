//! The browser application: owns the window, the GPU chrome, every tab and all state, and turns window / web view
//! events into actions. Behaviour is split over `app_tabs.rs` (tab management), `app_input.rs` (mouse, keyboard and
//! the address bar) and `app_commands.rs` (`axomai://` commands and page loading).

use crate::actions::Core;
use crate::favicons::{self, Favicons};
use crate::rendering::{h_of, w_of};
use crate::settings::Settings;
use crate::storage::BrowserStorage;
use crate::tabs::{ClosedTab, Tab, TabKind};
use crate::toolbar::{self, TabItem, ToolbarIcons};
use crate::types::{Extension, SearchEngine, CHROME_TOP, SIDEBAR_W};
use crate::web::{self, WebEvent, WebShared};
use axomai_engine::{NativeGpuCompositor, WgpuRenderer};
use std::time::Instant;
use tao::event::{Event, WindowEvent};
use tao::event_loop::ControlFlow;
use tao::keyboard::ModifiersState;
use tao::window::Window;
use wry::WebView;

pub struct DragState {
    pub idx: usize,
    pub start_x: f32,
    pub moved: bool,
}

pub struct App {
    pub window: Window,
    pub gpu: WgpuRenderer,
    pub compositor: NativeGpuCompositor,
    pub icons: ToolbarIcons,
    pub favicons: Favicons,
    /// Handle with tab id 0: the shared queues, token and paths. Each tab has its own clone with its own id.
    pub hub: WebShared,
    pub core: Core,
    pub extensions: Vec<Extension>,
    pub storage: Option<BrowserStorage>,
    pub search_engine: SearchEngine,
    pub settings: Settings,

    pub tabs: Vec<Tab>,
    pub active: usize,
    /// The active tab's web view (inactive tabs keep theirs in `Tab::view`).
    pub webview: Option<WebView>,
    pub closed: Vec<ClosedTab>,
    pub next_tab_id: u64,

    pub addr_text: String,
    pub addr_cursor: usize,
    pub addr_focused: bool,
    pub addr_selected: bool,
    /// When the address bar last took focus; the web view's own "got focus" echo right after is ignored.
    pub addr_focus_at: Instant,
    /// Dropdown under the address bar (matches only; row 0 of the popup is the typed text).
    pub suggestions: Vec<crate::suggest::Suggestion>,
    pub sugg_sel: usize,
    pub sugg_shown: bool,
    /// A click in the page closes the dropdown shortly after, unless the click was on a row of it.
    pub sugg_hide_at: Option<Instant>,
    /// Bookmarks listed on the bookmarks bar (newest first, as stored).
    pub bar_marks: Vec<crate::storage::Bookmark>,
    /// Downloads that are running (or waiting for a "Save as" answer).
    pub dls: Vec<crate::downloads::DlState>,
    pub dl_push_at: Instant,
    pub pw_pending: Option<crate::passwords::PwPending>,
    pub pw_offer: Option<crate::passwords::PwOffer>,
    /// Permission questions waiting for an answer, a private window's decisions, and whether the bar is showing.
    pub perm_queue: Vec<crate::permissions::PermAsk>,
    pub perm_session: std::collections::HashMap<(String, String), bool>,
    pub infobar_on: bool,
    /// The window is full screen because a page asked for it (so leaving the page's full screen restores it).
    pub html_fullscreen: bool,
    /// Warning pages waiting to replace the engine's error page: (tab id, html, since).
    pub https_warn: Vec<(u64, String, Instant)>,

    pub mouse: (f32, f32),
    pub left_down: bool,
    pub mods: ModifiersState,
    pub scale: f32,
    pub drag: Option<DragState>,

    pub redraw: bool,
    pub loading_progress: f32,
    pub last_shield_count: u32,
    pub fullscreen: bool,
    pub private_window: bool,
    pub exit: bool,
    /// A web view has to be created but we are in an event-loop state where that is not safe (see `handle`).
    pub view_pending: bool,
    /// True while the event being handled is one during which creating a web view is safe.
    pub safe: bool,
    pub proxy: tao::event_loop::EventLoopProxy<()>,
    /// Keeps the wgpu instance alive for as long as the surface that was created from it.
    pub _instance: Option<wgpu::Instance>,
}

impl App {
    pub fn shared(&self) -> WebShared {
        self.tabs[self.active].shared.clone()
    }

    pub fn tab_items(&self) -> Vec<TabItem> {
        self.tabs
            .iter()
            .map(|t| TabItem {
                title: t.title.clone(),
                pinned: t.pinned,
                private: t.private,
                favicon: t.favicon,
                audio: t.audio,
                muted: t.muted,
                loading: t.loading,
                sleeping: t.suspended,
            })
            .collect()
    }

    pub fn window_size(&self) -> (f32, f32) {
        (w_of(&self.gpu), h_of(&self.gpu))
    }

    // ---------------------------------------------------------------- event routing

    pub fn handle(&mut self, event: Event<()>, flow: &mut ControlFlow) {
        // Creating a web view runs a nested Windows message loop. tao re-enters its own event handler from that
        // loop, which panics unless the runner is in its "handling main events" state, i.e. while we are inside
        // a NewEvents / WindowEvent / UserEvent callback. Inside MainEventsCleared it is not safe, so view
        // creation requested there is deferred to a user event (see `load_active_page`).
        self.safe = matches!(event, Event::NewEvents(_) | Event::WindowEvent { .. } | Event::UserEvent(_));
        match event {
            Event::NewEvents(_) | Event::UserEvent(_) => self.create_pending_view(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    self.save_session();
                    self.exit = true;
                }
                WindowEvent::Resized(size) => self.on_resize(size.width, size.height),
                WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(position.x as f32, position.y as f32),
                WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(state, button),
                WindowEvent::ModifiersChanged(m) => self.mods = m,
                WindowEvent::KeyboardInput { event, .. } => self.on_key(event),
                WindowEvent::Focused(true) => self.redraw = true,
                _ => {}
            },
            Event::MainEventsCleared => self.tick(),
            _ => {}
        }
        if self.exit {
            *flow = ControlFlow::Exit;
        }
    }

    fn create_pending_view(&mut self) {
        if self.view_pending && self.safe {
            self.view_pending = false;
            self.load_active_page();
            self.after_activation();
        }
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.gpu.resize(width, height);
        self.redraw = true;
        self.fit_active_view();
    }

    /// Place the active web view under the toolbar, filling the rest of the window.
    pub fn fit_active_view(&self) {
        let (w, h) = self.window_size();
        let top = self.chrome_top();
        if let Some(wv) = &self.webview {
            let _ = wv.set_bounds(wry::Rect {
                position: wry::dpi::PhysicalPosition::new(SIDEBAR_W as i32, top as i32).into(),
                size: wry::dpi::PhysicalSize::new((w - SIDEBAR_W).max(1.0) as u32, (h - top).max(1.0) as u32).into(),
            });
        }
    }

    // ------------------------------------------------------------------- per-frame work

    fn tick(&mut self) {
        self.process_commands();
        self.process_web_events();
        if self.sugg_hide_at.is_some_and(|t| t.elapsed() > std::time::Duration::from_millis(300)) {
            self.sugg_hide_at = None;
            self.hide_suggestions();
        }
        self.push_download_progress();
        self.show_pending_prompt();
        self.sync_infobar();
        self.show_https_warnings();
        self.core.tick(&self.extensions);
        self.sleep_idle_tabs();
        self.sync_with_page();
        if self.tabs[self.active].loading {
            self.redraw = true;
        }
        if self.redraw || self.gpu.presented_frames < 3 {
            self.render();
        }
    }

    /// Keep the address bar, back/forward buttons and shield counter in step with the page.
    fn sync_with_page(&mut self) {
        let idx = self.active;
        if let Some(wv) = &self.webview {
            if !self.addr_focused && self.tabs[idx].kind == TabKind::Web {
                if let Ok(u) = wv.url() {
                    if (u.starts_with("http://") || u.starts_with("https://")) && u != self.tabs[idx].url {
                        self.tabs[idx].url = u;
                        self.redraw = true;
                    }
                }
            }
        }
        let probe = if self.tabs[idx].kind == TabKind::Web { self.tabs[idx].url.clone() } else { String::new() };
        // A private window never looks at (or reveals) the saved bookmarks.
        let store = if self.private_window { None } else { self.storage.as_ref() };
        if self.core.poll(self.webview.as_ref(), store, &probe) {
            self.redraw = true;
        }
        let blocked = self.tabs[idx].shared.shield.page_blocked.load(std::sync::atomic::Ordering::Relaxed);
        if blocked != self.last_shield_count {
            self.last_shield_count = blocked;
            self.redraw = true;
        }
    }

    fn process_web_events(&mut self) {
        for (tab_id, ev) in self.hub.drain_events() {
            let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) else { continue };
            match ev {
                WebEvent::Navigating(url) => self.on_navigating(idx, &url),
                WebEvent::Download(d) => self.on_download_event(idx, d),
                WebEvent::PageMsg(source, json) => self.on_page_message(idx, &source, &json),
                WebEvent::PermissionAsk { id, uri, name } => self.on_permission_ask(idx, id, &uri, &name),
                WebEvent::Upgrade(url) => self.on_https_upgrade(idx, &url),
                WebEvent::Zoom(z) => self.on_zoom_event(idx, z),
                WebEvent::HtmlFullscreen(on) => self.on_html_fullscreen(on),
                WebEvent::UpgradeFailed(url) => self.on_https_failed(idx, &url),
                WebEvent::LoadStarted(url) => {
                    self.password_page_changed(tab_id);
                    if url.starts_with("http://") || url.starts_with("https://") {
                        self.tabs[idx].loading = true;
                        if idx == self.active {
                            self.loading_progress = 0.0;
                            if let Some(wv) = &self.webview {
                                // Web pages that paint no background must show white, not the dark internal-page colour.
                                let _ = wv.set_background_color((255, 255, 255, 255));
                            }
                        }
                        self.redraw = true;
                    }
                }
                WebEvent::LoadFinished(_) => {
                    self.tabs[idx].loading = false;
                    if idx == self.active {
                        self.loading_progress = 0.0;
                    }
                    self.redraw = true;
                }
                WebEvent::Title(t) => self.on_title(idx, t),
                WebEvent::Favicon(bytes) => {
                    if let Some(slot) = self.favicons.add(&bytes) {
                        self.tabs[idx].favicon = Some(slot);
                        self.redraw = true;
                    }
                }
                WebEvent::Audio(playing) => {
                    self.tabs[idx].audio = playing;
                    self.redraw = true;
                }
                WebEvent::OpenUrl(url) => {
                    let at = idx + 1;
                    self.open_tab_at(&url, at, true);
                }
                WebEvent::Captured(r) => {
                    let shared = self.shared();
                    self.core.on_captured(&r, self.webview.as_ref(), &shared);
                }
                WebEvent::PageData(kind, question, payload) => self.on_page_data(idx, &kind, &question, &payload),
            }
        }
    }

    fn on_navigating(&mut self, idx: usize, url: &str) {
        let lower = url.to_ascii_lowercase();
        let prefix = self.hub.ui_prefix.to_string();
        if lower.starts_with("data:") || lower.starts_with("about:blank") {
            return; // our own generated pages: the tab already knows what it is showing
        }
        if lower.starts_with(&prefix) {
            // One of our own files (home / about): show its friendly name, never the file path.
            let file = lower.trim_start_matches(&prefix).trim_start_matches('/');
            let file = file.split(&['?', '#'][..]).next().unwrap_or("");
            let kind = match file {
                "home.html" => TabKind::Home,
                "index.html" => TabKind::About,
                _ => return,
            };
            let t = &mut self.tabs[idx];
            t.kind = kind;
            t.url = kind.address().unwrap_or_default();
            t.title = kind.default_title().to_string();
            t.favicon = None;
            self.redraw = true;
            return;
        }
        if !(lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("file:") || lower.starts_with("ftp:")) {
            return;
        }
        self.apply_site_zoom(idx, url);
        let private = self.tabs[idx].private || self.private_window;
        let t = &mut self.tabs[idx];
        t.kind = TabKind::Web;
        t.url = url.to_string();
        t.favicon = None;
        t.audio = false;
        if idx == self.active && self.addr_focused {
            // keep what the user is typing; the committed address is already stored in the tab
        }
        if !private {
            if let Some(s) = &self.storage {
                let _ = s.add_history(url, "");
            }
        }
        self.redraw = true;
    }

    fn on_title(&mut self, idx: usize, title: String) {
        let title = title.trim().to_string();
        if title.is_empty() {
            return;
        }
        // Our generated pages announce themselves by title; that keeps tab and address right after Back/Forward.
        if self.tabs[idx].shared.trusted.load(std::sync::atomic::Ordering::SeqCst) {
            if let Some((addr, kind)) = crate::app_commands::internal_page_for_title(&title) {
                let t = &mut self.tabs[idx];
                t.kind = kind;
                t.url = addr.to_string();
                t.title = kind.default_title().to_string();
                self.redraw = true;
                return;
            }
        }
        let t = &mut self.tabs[idx];
        match t.kind {
            // Source tabs keep their own "Source: host" title; every other generated page is handled above.
            TabKind::Web => {
                t.title = title.clone();
                let (mut url, private) = (t.url.clone(), t.private || self.private_window);
                // The tab's address can still be the previous page right after typing a new one; the web view's own
                // source is the document this title belongs to.
                if idx == self.active {
                    if let Some(u) = self.webview.as_ref().and_then(|wv| wv.url().ok()).filter(|u| u.starts_with("http")) {
                        url = u;
                    }
                }
                if !private && matches!(t.kind, TabKind::Web) {
                    if let Some(s) = &self.storage {
                        let _ = s.update_history_title(&url, &title);
                    }
                }
            }
            _ => {}
        }
        self.redraw = true;
    }

    /// Hidden tabs that have been idle for a while release their memory but keep their state.
    fn sleep_idle_tabs(&mut self) {
        let Some(limit) = self.settings.sleep_after() else { return };
        let now = Instant::now();
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if i == self.active || t.suspended || t.audio {
                continue;
            }
            if let Some(v) = &t.view {
                if now.duration_since(t.last_active) >= limit {
                    web::com::suspend(v);
                    t.suspended = true;
                    self.redraw = true;
                }
            }
        }
    }

    // ---------------------------------------------------------------------- drawing

    fn render(&mut self) {
        self.redraw = false;
        let (w, _) = self.window_size();
        let idx = self.active;
        let tab_url = self.tabs[idx].url.clone();
        let shown = if self.addr_focused { self.addr_text.clone() } else { tab_url.clone() };
        let items = self.tab_items();
        let avatar = self.core.avatar_letter();
        let chrome = toolbar::ChromeState {
            theme: self.core.theme,
            // While typing, the badge keeps describing the page that is actually loaded.
            security: toolbar::Security::from_address(&tab_url),
            shield_count: self.tabs[idx].shared.shield.page_blocked.load(std::sync::atomic::Ordering::Relaxed),
            bookmarked: self.core.bookmarked,
            address_selected: self.addr_selected && self.addr_focused,
            avatar: &avatar,
            private: self.private_window,
            zoom: Some(crate::viewctl::percent(self.tabs[idx].zoom)).filter(|p| *p != 100),
        };
        let mut quads = toolbar::build_toolbar_quads(
            &mut self.compositor,
            w,
            &shown,
            self.addr_focused,
            self.addr_cursor,
            &items,
            idx,
            self.core.can_back,
            self.core.can_fwd,
            &self.icons,
            &chrome,
        );

        quads.extend(self.bar_quads());
        quads.extend(self.infobar_quads());
        if self.fullscreen {
            // Full screen: the page owns the whole window.
            quads.clear();
        }
        if self.tabs[idx].loading {
            // Eases toward 90% and stays there until the page reports that it has finished loading.
            self.loading_progress += (0.9 - self.loading_progress) * 0.04;
            let p = self.core.theme.primary;
            quads.push(NativeGpuCompositor::solid_quad(
                SIDEBAR_W,
                CHROME_TOP - 3.0,
                (w - SIDEBAR_W) * self.loading_progress,
                3.0,
                crate::rendering::c(p[0], p[1], p[2], 230),
            ));
        }

        if self.favicons.dirty {
            self.gpu.upload_bg_image(favicons::ATLAS, favicons::ATLAS, self.favicons.pixels());
            self.favicons.dirty = false;
        }
        if self.compositor.glyph_atlas.dirty {
            self.gpu.upload_glyph_atlas(&self.compositor.glyph_atlas);
            self.compositor.glyph_atlas.dirty = false;
        }
        match self.gpu.render_frame(&quads) {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => {
                let cfg = &self.gpu.surface_config;
                let (cw, ch) = (cfg.width, cfg.height);
                self.gpu.resize(cw, ch);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => self.exit = true,
            Err(e) => eprintln!("[Axomai GPU] Render error: {:?}", e),
        }
        let title = self.tabs[idx].title.clone();
        let brand = if self.private_window { "Axomai Browser (Incognito)" } else { "Axomai Browser" };
        self.window.set_title(&format!("{} \u{2014} {}", brand, title));
    }
}
