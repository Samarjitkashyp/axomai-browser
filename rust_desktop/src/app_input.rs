//! Mouse, keyboard and address-bar behaviour for `App`.

use crate::actions;
use crate::app::{App, DragState};
use crate::extensions as ex;
use crate::omnibox;
use crate::overlays;
use crate::sys;
use crate::tabs::TabKind;
use crate::toolbar::{self, ToolbarHit, ToolbarLayout};
use crate::types::{BOOKMARK_BAR_H, CHROME_TOP, TAB_BAR_H};
use tao::event::{ElementState, KeyEvent, MouseButton};
use tao::keyboard::{Key, KeyCode};
use tao::window::CursorIcon;

/// Browser shortcut for a key pressed together with Ctrl (or a lone F-key), as the command it triggers.
/// The same command names are produced by the web view's accelerator hook (`web::com::install_accelerators`).
pub fn shortcut_command(key: &Key, ctrl: bool, shift: bool, alt: bool) -> Option<String> {
    if alt {
        return match key {
            Key::ArrowLeft if !ctrl => Some("back".into()),
            Key::ArrowRight if !ctrl => Some("forward".into()),
            Key::Home if !ctrl => Some("home".into()),
            Key::Character(ch) if !ctrl && ch.eq_ignore_ascii_case("d") => Some("focus-url".into()),
            _ => None,
        };
    }
    if !ctrl {
        return match key {
            Key::F11 => Some("fullscreen".into()),
            Key::F5 => Some("reload".into()),
            Key::F6 => Some("focus-url".into()),
            _ => None,
        };
    }
    let cmd: String = match key {
        Key::Tab => (if shift { "tab-prev" } else { "tab-next" }).into(),
        Key::PageDown => "tab-next".into(),
        Key::PageUp => "tab-prev".into(),
        Key::F5 => "reload-hard".into(),
        Key::Delete if shift => "clear-data-dialog".into(),
        Key::Character(ch) => {
            let c = ch.chars().next()?.to_ascii_lowercase();
            match (c, shift) {
                ('t', false) => "newtab".into(),
                ('t', true) => "reopen-tab".into(),
                ('w', false) => "closetab".into(),
                ('l', false) => "focus-url".into(),
                ('h', false) => "history".into(),
                ('j', false) => "downloads".into(),
                ('d', false) => "bookmark-toggle".into(),
                ('u', false) => "viewsource-current".into(),
                ('f', false) => "find".into(),
                ('p', false) => "print".into(),
                ('r', false) => "reload".into(),
                ('r', true) => "reload-hard".into(),
                ('k', false) | ('e', false) => "focus-url".into(),
                ('b', true) => "bookmark-bar-toggle".into(),
                ('=', _) | ('+', _) => "zoom-in".into(),
                ('-', false) | ('_', false) => "zoom-out".into(),
                ('0', false) => "zoom-reset".into(),
                ('n', false) => "new-window".into(),
                ('n', true) => "new-incognito".into(),
                ('o', true) => "bookmarks".into(),
                ('1'..='9', false) => format!("tab-index/{}", c),
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(cmd)
}

impl App {
    // ---------------------------------------------------------------------- pointer

    pub fn on_cursor_moved(&mut self, x: f32, y: f32) {
        self.mouse = (x, y);
        if self.left_down {
            if let Some(d) = self.drag.as_mut() {
                if !d.moved && (x - d.start_x).abs() > 6.0 {
                    d.moved = true;
                }
                if d.moved {
                    self.drag_tab_to(x);
                }
            }
        }
        let (w, _) = self.window_size();
        let cursor = if y <= TAB_BAR_H {
            let strip = toolbar::tab_strip(w, &self.tab_items());
            if strip.index_at(x, y).is_some() || strip.plus.contains(x, y) {
                CursorIcon::Hand
            } else {
                CursorIcon::Default
            }
        } else if y <= CHROME_TOP {
            match toolbar::toolbar_layout(w).hit(x, y) {
                Some(ToolbarHit::Omnibox) => CursorIcon::Text,
                Some(_) => CursorIcon::Hand,
                None => CursorIcon::Default,
            }
        } else if self.settings.bookmark_bar && y < CHROME_TOP + BOOKMARK_BAR_H {
            if self.bar_layout().iter().any(|(r, _, _)| r.contains(x, y)) { CursorIcon::Hand } else { CursorIcon::Default }
        } else {
            CursorIcon::Default
        };
        self.window.set_cursor_icon(cursor);
    }

    pub fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let (x, y) = self.mouse;
        match (state, button) {
            (ElementState::Pressed, MouseButton::Left) => {
                self.left_down = true;
                if self.fullscreen {
                    // the page owns the window
                } else if y < CHROME_TOP {
                    self.chrome_click(x, y);
                } else if self.infobar_click(x, y) {
                    // handled by the permission bar
                } else if self.settings.bookmark_bar && y < CHROME_TOP + BOOKMARK_BAR_H {
                    self.core.close_popups(self.webview.as_ref());
                    self.addr_focused = false;
                    self.addr_selected = false;
                    self.hide_suggestions();
                    self.bar_click(x, y);
                }
            }
            (ElementState::Released, MouseButton::Left) => {
                self.left_down = false;
                self.drag = None;
            }
            (ElementState::Pressed, MouseButton::Right) if y <= TAB_BAR_H => {
                let (w, _) = self.window_size();
                let strip = toolbar::tab_strip(w, &self.tab_items());
                if let Some(i) = strip.index_at(x, y) {
                    self.open_tab_menu(i, strip.tab_rect(i).x);
                }
            }
            (ElementState::Pressed, MouseButton::Middle) if y <= TAB_BAR_H => {
                let (w, _) = self.window_size();
                let strip = toolbar::tab_strip(w, &self.tab_items());
                if let Some(i) = strip.index_at(x, y) {
                    self.close_tab(i);
                }
            }
            _ => {}
        }
    }

    fn chrome_click(&mut self, x: f32, y: f32) {
        let (w, _) = self.window_size();
        if y <= TAB_BAR_H {
            self.core.close_popups(self.webview.as_ref());
            let items = self.tab_items();
            let strip = toolbar::tab_strip(w, &items);
            if strip.plus.contains(x, y) {
                self.new_tab();
                return;
            }
            if let Some(i) = strip.index_at(x, y) {
                let r = strip.tab_rect(i);
                if toolbar::speaker_visible(&items[i], &r) && strip.speaker_rect(i).contains(x, y) {
                    self.toggle_mute(i);
                } else if toolbar::close_visible(&items[i], &r, i == self.active) && strip.close_rect(i).contains(x, y) {
                    self.close_tab(i);
                } else {
                    self.activate(i);
                    self.drag = Some(DragState { idx: i, start_x: x, moved: false });
                }
            }
            return;
        }
        let layout = toolbar::toolbar_layout(w);
        if crate::viewctl::percent(self.zoom_factor()) != 100 && layout.zoom.contains(x, y) {
            self.zoom_command("reset");
            return;
        }
        let hit = layout.hit(x, y);
        self.toolbar_click(hit, &layout);
    }

    fn toolbar_click(&mut self, hit: Option<ToolbarHit>, tb: &ToolbarLayout) {
        let (w, _) = self.window_size();
        let scale = 1.0;
        let anchor = |r: toolbar::Rect| actions::anchor_right(w, r.right(), scale);
        let shared = self.shared();
        // Clicking anything in the toolbar dismisses an open popup, as in other browsers. Buttons that open a
        // popup are excluded so that clicking the same button again toggles it closed.
        if !matches!(
            hit,
            Some(ToolbarHit::Extensions) | Some(ToolbarHit::Menu) | Some(ToolbarHit::Theme) | Some(ToolbarHit::Shield)
                | Some(ToolbarHit::Ai) | Some(ToolbarHit::Profile) | Some(ToolbarHit::Qr) | Some(ToolbarHit::Secure)
        ) {
            self.core.close_popups(self.webview.as_ref());
        }
        let idx = self.active;
        let url = self.tabs[idx].url.clone();
        let title = self.tabs[idx].title.clone();
        let mut leave_address = !matches!(hit, Some(ToolbarHit::Omnibox));
        match hit {
            Some(ToolbarHit::Extensions) => self.core.open_extensions(self.webview.as_ref(), &shared, &self.extensions, anchor(tb.extensions)),
            Some(ToolbarHit::Menu) => self.core.open_menu(self.webview.as_ref(), &shared, anchor(tb.menu), &self.settings.language),
            Some(ToolbarHit::Downloads) => self.open_internal("downloads"),
            Some(ToolbarHit::Theme) => self.core.open_theme_menu(self.webview.as_ref(), &shared, anchor(tb.theme)),
            Some(ToolbarHit::Shield) => self.core.open_shield(self.webview.as_ref(), &shared, &self.extensions, anchor(tb.shield)),
            Some(ToolbarHit::Ai) => self.core.open_ai(self.webview.as_ref(), &shared, anchor(tb.ai), &title),
            Some(ToolbarHit::Profile) => self.core.open_profile(self.webview.as_ref(), &shared, self.storage.as_ref(), anchor(tb.profile)),
            Some(ToolbarHit::Qr) => self.core.open_qr(self.webview.as_ref(), &shared, anchor(tb.qr), &url),
            Some(ToolbarHit::Secure) => {
                self.core.open_site_info(self.webview.as_ref(), &shared, actions::anchor_right(w, tb.secure_badge.x + 330.0, scale), &url)
            }
            Some(ToolbarHit::Reader) => self.core.run_extension(ex::READER, self.webview.as_ref(), &shared, &self.extensions, anchor(tb.reader)),
            Some(ToolbarHit::Bookmark) => self.toggle_bookmark_now(),
            Some(ToolbarHit::Omnibox) => {
                self.focus_address();
                leave_address = false;
            }
            Some(ToolbarHit::Back) => {
                if let (true, Some(wv)) = (self.core.can_back, &self.webview) {
                    crate::web::com::go_back(wv);
                }
            }
            Some(ToolbarHit::Forward) => {
                if let (true, Some(wv)) = (self.core.can_fwd, &self.webview) {
                    crate::web::com::go_forward(wv);
                }
            }
            Some(ToolbarHit::Reload) => self.reload_active(),
            Some(ToolbarHit::Home) => self.open_internal("home"),
            None => {}
        }
        if leave_address && self.addr_focused {
            self.addr_focused = false;
            self.addr_selected = false;
        }
        if !self.addr_focused {
            self.hide_suggestions();
        }
        self.redraw = true;
    }

    pub fn toggle_bookmark_now(&mut self) {
        let idx = self.active;
        let (url, title) = (self.tabs[idx].url.clone(), self.tabs[idx].title.clone());
        let result = if self.private_window { None } else { self.core.toggle_bookmark(&url, &title, self.storage.as_ref()) };
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            match result {
                Some(true) => self.core.toast(wv, "\u{2B50} Bookmark added", None, &shared),
                Some(false) => self.core.toast(wv, "Bookmark removed", None, &shared),
                None if self.private_window => self.core.toast(wv, "Bookmarks are not saved in a private window", None, &shared),
                None => self.core.toast(wv, "Only web pages can be bookmarked", None, &shared),
            }
        }
        self.refresh_bar();
        self.redraw = true;
    }

    pub fn reload_active(&mut self) {
        match self.tabs[self.active].kind {
            TabKind::Web => {
                if let Some(wv) = &self.webview {
                    crate::web::com::reload(wv);
                }
            }
            TabKind::Source => {
                // Fetch the source again.
                self.tabs[self.active].source_html = None;
                self.load_active_page();
            }
            _ => self.load_active_page(),
        }
    }

    /// Right-click on a tab.
    pub fn open_tab_menu(&mut self, idx: usize, tab_x: f32) {
        let Some(t) = self.tabs.get(idx) else { return };
        let id = t.id;
        let e = |emoji: &str, label: &str, cmd: String, danger: bool| (emoji.to_string(), label.to_string(), cmd, danger);
        let mut items = vec![
            e("\u{1F504}", "Reload", format!("tab-reload/{}", id), false),
            e("\u{1F4D1}", "Duplicate", format!("tab-duplicate/{}", id), false),
            e("\u{1F4CC}", if t.pinned { "Unpin tab" } else { "Pin tab" }, format!("tab-pin/{}", id), false),
        ];
        if t.audio || t.muted {
            items.push(e(if t.muted { "\u{1F50A}" } else { "\u{1F507}" }, if t.muted { "Unmute tab" } else { "Mute tab" }, format!("tab-mute/{}", id), false));
        }
        items.push(e("", "-", String::new(), false));
        items.push(e("\u{2795}", "New tab", "newtab".into(), false));
        items.push(e("\u{21A9}\u{FE0F}", "Reopen closed tab", "reopen-tab".into(), false));
        items.push(e("", "-", String::new(), false));
        items.push(e("\u{2716}\u{FE0F}", "Close tab", format!("tab-close/{}", id), true));
        items.push(e("", "Close other tabs", format!("tab-close-others/{}", id), true));
        items.push(e("", "Close tabs to the right", format!("tab-close-right/{}", id), true));
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            let left = tab_x;
            let _ = wv.evaluate_script(&overlays::tab_menu_popup(self.core.theme, &shared.token, left, &items));
        }
    }

    // ------------------------------------------------------------------ address bar

    pub fn focus_address(&mut self) {
        let url = self.tabs[self.active].url.clone();
        self.addr_focused = true;
        self.addr_focus_at = std::time::Instant::now();
        if url.starts_with("http") || url.starts_with("file:") || url.starts_with("view-source:") {
            self.addr_text = url;
            self.addr_cursor = self.addr_text.chars().count();
            self.addr_selected = true;
        } else {
            self.addr_text.clear();
            self.addr_cursor = 0;
            self.addr_selected = false;
        }
        #[cfg(target_os = "windows")]
        unsafe {
            use tao::platform::windows::WindowExtWindows;
            extern "system" {
                fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            }
            SetFocus(self.window.hwnd() as _);
        }
        self.redraw = true;
    }

    fn addr_byte_index(&self, chars: usize) -> usize {
        self.addr_text.char_indices().nth(chars).map(|(i, _)| i).unwrap_or(self.addr_text.len())
    }

    fn addr_replace_selection(&mut self) {
        if self.addr_selected {
            self.addr_text.clear();
            self.addr_cursor = 0;
            self.addr_selected = false;
        }
    }

    pub fn addr_insert(&mut self, s: &str) {
        self.addr_replace_selection();
        let at = self.addr_byte_index(self.addr_cursor);
        self.addr_text.insert_str(at, s);
        self.addr_cursor += s.chars().count();
        self.update_suggestions();
        self.redraw = true;
    }

    fn addr_backspace(&mut self) {
        if self.addr_selected {
            self.addr_replace_selection();
        } else if self.addr_cursor > 0 {
            let start = self.addr_byte_index(self.addr_cursor - 1);
            let end = self.addr_byte_index(self.addr_cursor);
            self.addr_text.replace_range(start..end, "");
            self.addr_cursor -= 1;
        }
        self.update_suggestions();
        self.redraw = true;
    }

    fn addr_delete(&mut self) {
        if self.addr_selected {
            self.addr_replace_selection();
        } else if self.addr_cursor < self.addr_text.chars().count() {
            let start = self.addr_byte_index(self.addr_cursor);
            let end = self.addr_byte_index(self.addr_cursor + 1);
            self.addr_text.replace_range(start..end, "");
        }
        self.update_suggestions();
        self.redraw = true;
    }

    fn addr_move(&mut self, to: usize) {
        self.addr_selected = false;
        self.addr_cursor = to.min(self.addr_text.chars().count());
        self.redraw = true;
    }

    /// Enter in the address bar.
    pub fn submit_address(&mut self) {
        let text = self.addr_text.clone();
        self.suggestions.clear();
        self.hide_suggestions();
        self.addr_focused = false;
        self.addr_selected = false;
        let engine = self.search_engine;
        match omnibox::resolve(&text, |q| engine.search_url(q)) {
            Some(omnibox::Target::Navigate(u)) => self.navigate_active(&u),
            Some(omnibox::Target::Internal(cmd)) => self.open_internal(cmd),
            Some(omnibox::Target::ViewSource(u)) => self.open_view_source(&u),
            None => {}
        }
        self.redraw = true;
    }

    // ---------------------------------------------------------------------- keyboard

    pub fn on_key(&mut self, ev: KeyEvent) {
        if ev.state != ElementState::Pressed {
            return;
        }
        let ctrl = self.mods.control_key();
        let shift = self.mods.shift_key();

        if ctrl && self.addr_focused {
            if let Key::Character(ch) = &ev.logical_key {
                match ch.to_ascii_lowercase().as_str() {
                    "a" => {
                        self.addr_cursor = self.addr_text.chars().count();
                        self.addr_selected = !self.addr_text.is_empty();
                        self.redraw = true;
                        return;
                    }
                    "v" => {
                        if let Some(text) = sys::clipboard_text() {
                            let text: String = text.chars().map(|c| if c == '\r' || c == '\n' || c == '\t' { ' ' } else { c }).collect();
                            self.addr_insert(&text);
                        }
                        return;
                    }
                    "c" | "x" => {
                        if self.addr_selected {
                            sys::set_clipboard_text(&self.addr_text);
                            if ch.eq_ignore_ascii_case("x") {
                                self.addr_replace_selection();
                                self.update_suggestions();
                                self.redraw = true;
                            }
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }
        if let Some(cmd) = shortcut_command(&ev.logical_key, ctrl, shift, self.mods.alt_key()) {
            self.hub.push_command(&cmd);
            return;
        }
        if !self.addr_focused {
            return;
        }

        let left = matches!(ev.logical_key, Key::ArrowLeft) || matches!(ev.physical_key, KeyCode::ArrowLeft);
        let right = matches!(ev.logical_key, Key::ArrowRight) || matches!(ev.physical_key, KeyCode::ArrowRight);
        let home = matches!(ev.logical_key, Key::Home) || matches!(ev.physical_key, KeyCode::Home);
        let end = matches!(ev.logical_key, Key::End) || matches!(ev.physical_key, KeyCode::End);
        let len = self.addr_text.chars().count();
        let up = matches!(ev.logical_key, Key::ArrowUp);
        let down = matches!(ev.logical_key, Key::ArrowDown);
        if (up || down) && self.move_suggestion(if down { 1 } else { -1 }) {
            return;
        }
        if left {
            // With everything selected, Left jumps to the start and Right to the end, like other browsers.
            let to = if self.addr_selected { 0 } else { self.addr_cursor.saturating_sub(1) };
            self.addr_move(to);
        } else if right {
            let to = if self.addr_selected { len } else { self.addr_cursor + 1 };
            self.addr_move(to);
        } else if home {
            self.addr_move(0);
        } else if end {
            self.addr_move(len);
        } else {
            match ev.logical_key {
                Key::Backspace => self.addr_backspace(),
                Key::Delete => self.addr_delete(),
                Key::Enter => {
                    if self.sugg_sel > 0 {
                        self.open_suggestion(self.sugg_sel);
                    } else {
                        self.submit_address();
                    }
                }
                Key::Escape => {
                    self.hide_suggestions();
                    self.addr_focused = false;
                    self.addr_selected = false;
                    self.redraw = true;
                }
                // Space arrives as its own key, not as `Character`.
                Key::Space => self.addr_insert(" "),
                Key::Character(ref ch) => self.addr_insert(ch),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn ctrl_shortcuts_map_to_commands() {
        assert_eq!(shortcut_command(&ch("t"), true, false, false).as_deref(), Some("newtab"));
        assert_eq!(shortcut_command(&ch("T"), true, true, false).as_deref(), Some("reopen-tab"));
        assert_eq!(shortcut_command(&ch("w"), true, false, false).as_deref(), Some("closetab"));
        assert_eq!(shortcut_command(&ch("5"), true, false, false).as_deref(), Some("tab-index/5"));
        assert_eq!(shortcut_command(&Key::Tab, true, false, false).as_deref(), Some("tab-next"));
        assert_eq!(shortcut_command(&Key::Tab, true, true, false).as_deref(), Some("tab-prev"));
        assert_eq!(shortcut_command(&ch("N"), true, true, false).as_deref(), Some("new-incognito"));
    }

    #[test]
    fn zoom_reload_and_navigation_shortcuts() {
        assert_eq!(shortcut_command(&ch("="), true, false, false).as_deref(), Some("zoom-in"));
        assert_eq!(shortcut_command(&ch("+"), true, true, false).as_deref(), Some("zoom-in"));
        assert_eq!(shortcut_command(&ch("-"), true, false, false).as_deref(), Some("zoom-out"));
        assert_eq!(shortcut_command(&ch("0"), true, false, false).as_deref(), Some("zoom-reset"));
        assert_eq!(shortcut_command(&Key::F5, false, false, false).as_deref(), Some("reload"));
        assert_eq!(shortcut_command(&Key::F5, true, false, false).as_deref(), Some("reload-hard"));
        assert_eq!(shortcut_command(&ch("R"), true, true, false).as_deref(), Some("reload-hard"));
        assert_eq!(shortcut_command(&Key::ArrowLeft, false, false, true).as_deref(), Some("back"));
        assert_eq!(shortcut_command(&Key::ArrowRight, false, false, true).as_deref(), Some("forward"));
        assert_eq!(shortcut_command(&ch("d"), false, false, true).as_deref(), Some("focus-url"));
        assert_eq!(shortcut_command(&ch("k"), true, false, false).as_deref(), Some("focus-url"));
        assert_eq!(shortcut_command(&ch("B"), true, true, false).as_deref(), Some("bookmark-bar-toggle"));
        assert_eq!(shortcut_command(&Key::ArrowLeft, false, false, false), None, "plain arrows are for the page");
    }

    #[test]
    fn plain_keys_are_not_shortcuts() {
        assert_eq!(shortcut_command(&ch("t"), false, false, false), None);
        assert_eq!(shortcut_command(&Key::Enter, false, false, false), None);
        assert_eq!(shortcut_command(&Key::F11, false, false, false).as_deref(), Some("fullscreen"));
        assert_eq!(shortcut_command(&ch("z"), true, false, false), None);
    }
}
