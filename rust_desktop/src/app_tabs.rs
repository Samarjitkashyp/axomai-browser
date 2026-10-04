//! Tab management for `App`: create, activate, close, pin, duplicate, reorder, reopen.

use crate::app::App;
use crate::tabs::{self, ClosedTab, Tab, TabKind, CLOSED_LIMIT};
use crate::toolbar;
use crate::web;
use std::time::Instant;
use wry::WebView;

impl App {
    pub fn pinned_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.pinned).count()
    }

    pub fn make_tab(&mut self, address: &str) -> Tab {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let shared = self.hub.for_tab(id, self.private_window);
        Tab::new(id, address, shared)
    }

    /// The web view of tab `idx`, whether it is the active tab or a hidden one.
    pub fn view_of(&self, idx: usize) -> Option<&WebView> {
        if idx == self.active {
            self.webview.as_ref()
        } else {
            self.tabs.get(idx).and_then(|t| t.view.as_ref())
        }
    }

    pub fn idx_of_id(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Open `address` in a new tab at position `at` (never inside the pinned group).
    pub fn open_tab_at(&mut self, address: &str, at: usize, activate: bool) -> usize {
        let tab = self.make_tab(address);
        let at = at.min(self.tabs.len()).max(self.pinned_count());
        self.tabs.insert(at, tab);
        if at <= self.active {
            self.active += 1; // the active tab moved one place to the right
        }
        if activate {
            self.activate(at);
        }
        self.redraw = true;
        at
    }

    /// Ctrl+T / the "+" button: a new Home tab at the end, with the address bar ready for typing.
    pub fn new_tab(&mut self) {
        let at = self.tabs.len();
        self.open_tab_at("about:home", at, true);
        self.focus_address();
    }

    /// Like `stash_active_view`, but the view stays on screen (it is the other pane of a split view).
    fn stash_active_view_visible(&mut self) {
        if let Some(wv) = self.webview.take() {
            let t = &mut self.tabs[self.active];
            t.last_active = Instant::now();
            t.view = Some(wv);
        }
    }

    fn stash_active_view(&mut self) {
        if let Some(wv) = self.webview.take() {
            let _ = wv.set_visible(false);
            let t = &mut self.tabs[self.active];
            t.last_active = Instant::now();
            t.view = Some(wv);
        }
    }

    fn unstash_view(&mut self) {
        let t = &mut self.tabs[self.active];
        self.webview = t.view.take();
        t.last_active = Instant::now();
        let was_suspended = std::mem::replace(&mut t.suspended, false);
        match &self.webview {
            Some(wv) => {
                if was_suspended {
                    web::com::resume(wv);
                }
                self.fit_active_view();
                if let Some(wv) = &self.webview {
                    let _ = wv.set_visible(true);
                    let _ = wv.focus();
                }
            }
            None => self.load_active_page(),
        }
    }

    pub fn after_activation(&mut self) {
        self.addr_focused = false;
        self.addr_selected = false;
        self.loading_progress = 0.0;
        self.sync_extensions_for_active();
        self.core.can_back = false;
        self.core.can_fwd = false;
        self.redraw = true;
    }

    /// Make tab `idx` the visible one. Its web view keeps its page, scroll position and history.
    pub fn activate(&mut self, idx: usize) {
        if idx >= self.tabs.len() || idx == self.active {
            return;
        }
        let new_id = self.tabs[idx].id;
        match self.split {
            // Moving focus between the two panes: nothing is hidden.
            Some((l, r)) if new_id == l || new_id == r => {
                self.stash_active_view_visible();
                self.active = idx;
                self.unstash_view();
                self.fit_partner();
                self.after_activation();
                return;
            }
            // Any other tab ends the split.
            Some(_) => self.end_split(),
            None => {}
        }
        self.stash_active_view();
        self.active = idx;
        self.unstash_view();
        self.after_activation();
    }

    pub fn activate_relative(&mut self, delta: i32) {
        let n = self.tabs.len() as i32;
        if n < 2 {
            return;
        }
        let next = (self.active as i32 + delta).rem_euclid(n) as usize;
        self.activate(next);
    }

    /// Ctrl+1..8 jump to that tab; Ctrl+9 always jumps to the last one.
    pub fn activate_number(&mut self, n: usize) {
        if n == 0 || self.tabs.is_empty() {
            return;
        }
        let idx = if n >= 9 { self.tabs.len() - 1 } else { (n - 1).min(self.tabs.len() - 1) };
        self.activate(idx);
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let closing_id = self.tabs[idx].id;
        if self.split.is_some_and(|(l, r)| l == closing_id || r == closing_id) {
            self.end_split();
        }
        self.drop_permissions_for_tab(closing_id);
        let t = &self.tabs[idx];
        if !(t.kind == TabKind::Home) && !t.private {
            self.closed.push(ClosedTab { address: t.url.clone(), pinned: t.pinned });
            if self.closed.len() > CLOSED_LIMIT {
                self.closed.remove(0);
            }
        }
        if self.tabs.len() == 1 {
            // The last tab never disappears; it turns into a fresh Home tab.
            self.webview = None;
            let tab = self.make_tab("about:home");
            self.tabs[0] = tab;
            self.active = 0;
            self.load_active_page();
            self.after_activation();
            self.focus_address();
            return;
        }
        let was_active = idx == self.active;
        if was_active {
            self.webview = None;
        }
        self.tabs.remove(idx);
        self.active = tabs::next_active_after_close(self.active, idx, self.tabs.len());
        if was_active {
            self.unstash_view();
            self.after_activation();
        } else {
            self.redraw = true;
        }
    }

    /// Close every tab except `keep` and the pinned ones.
    pub fn close_others(&mut self, keep: usize) {
        let Some(keep_id) = self.tabs.get(keep).map(|t| t.id) else { return };
        let doomed: Vec<u64> = self.tabs.iter().filter(|t| t.id != keep_id && !t.pinned).map(|t| t.id).collect();
        for id in doomed {
            if let Some(i) = self.idx_of_id(id) {
                self.close_tab(i);
            }
        }
    }

    pub fn close_right_of(&mut self, idx: usize) {
        while self.tabs.len() > idx + 1 {
            let last = self.tabs.len() - 1;
            if self.tabs[last].pinned {
                break;
            }
            self.close_tab(last);
        }
    }

    pub fn duplicate_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let address = self.tabs[idx].url.clone();
        self.open_tab_at(&address, idx + 1, true);
    }

    pub fn toggle_pin(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let pinned = !self.tabs[idx].pinned;
        self.tabs[idx].pinned = pinned;
        let count = self.pinned_count();
        let target = if pinned { count - 1 } else { count };
        self.active = tabs::move_tab(&mut self.tabs, self.active, idx, target);
        self.redraw = true;
    }

    pub fn toggle_mute(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let muted = !self.tabs[idx].muted;
        self.tabs[idx].muted = muted;
        if let Some(wv) = self.view_of(idx) {
            web::com::set_muted(wv, muted);
        }
        self.redraw = true;
    }

    pub fn reopen_closed_tab(&mut self) {
        let Some(c) = self.closed.pop() else { return };
        let at = self.tabs.len();
        let idx = self.open_tab_at(&c.address, at, true);
        if c.pinned {
            self.toggle_pin(idx);
        }
    }

    /// While a tab is being dragged: swap it with its neighbours as the pointer crosses them.
    pub fn drag_tab_to(&mut self, mouse_x: f32) {
        let Some(d) = self.drag.as_ref() else { return };
        let from = d.idx;
        let (w, _) = self.window_size();
        let strip = toolbar::tab_strip(w, &self.tab_items());
        let desired = strip.rects.iter().position(|r| mouse_x < r.right()).unwrap_or(self.tabs.len().saturating_sub(1));
        let to = tabs::drag_target(from, desired, self.pinned_count(), self.tabs.len());
        if to != from {
            self.active = tabs::move_tab(&mut self.tabs, self.active, from, to);
            if let Some(d) = self.drag.as_mut() {
                d.idx = to;
            }
            self.redraw = true;
        }
    }

    pub fn sync_extensions_for_active(&mut self) {
        let idx = self.active;
        if self.tabs[idx].ext_gen == self.core.ext_generation {
            return;
        }
        if self.webview.is_some() {
            let doc = self.tabs[idx].doc.clone();
            let shared = self.tabs[idx].shared.clone();
            self.core.apply_extensions(self.webview.as_ref(), &doc, &shared, &self.extensions);
            self.tabs[idx].ext_gen = self.core.ext_generation;
        }
    }

    /// Persist the tab list (used when the window closes) and the lifetime shield total.
    pub fn save_session(&mut self) {
        if self.private_window {
            return;
        }
        if let Some(s) = &self.storage {
            let _ = s.set_setting("blocked_total", &self.hub.shield.total_blocked.load(std::sync::atomic::Ordering::Relaxed).to_string());
            let saved: Vec<crate::storage::SavedTab> = self
                .tabs
                .iter()
                .enumerate()
                .map(|(i, t)| crate::storage::SavedTab { position: i as i32, url: t.url.clone(), title: t.title.clone(), is_active: i == self.active })
                .collect();
            let _ = s.save_tabs(&saved);
        }
    }

    /// First window start: restore the saved tabs (only the active one gets a web view now, the rest load on demand),
    /// or open a single Home tab.
    pub fn restore_or_start(&mut self, saved: Vec<crate::storage::SavedTab>) {
        if saved.is_empty() {
            let t = self.make_tab("about:home");
            self.tabs.push(t);
            self.active = 0;
        } else {
            for s in &saved {
                let mut t = self.make_tab(&s.url);
                if !s.title.is_empty() && t.kind == TabKind::Web {
                    t.title = s.title.clone();
                }
                self.tabs.push(t);
                if s.is_active {
                    self.active = self.tabs.len() - 1;
                }
            }
        }
        // The web view itself is created from the event loop (see `App::tick`): creating it earlier would pump
        // window messages before the event handler exists.
        self.view_pending = true;
        let _ = self.proxy.send_event(());
    }
}
