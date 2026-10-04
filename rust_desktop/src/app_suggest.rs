//! Address-bar suggestions and the bookmarks bar.

use crate::app::App;
use crate::overlays;
use crate::rendering::{c, fit_text, render_text, text_width, w_of};
use crate::storage::Bookmark;
use crate::suggest::{self, Kind};
use crate::toolbar::{rr_rect, toolbar_layout, Rect};
use crate::types::{BOOKMARK_BAR_H, CHROME_TOP};
use axomai_engine::{GpuQuad, NativeGpuCompositor};

const MAX_SUGGESTIONS: usize = 6;

impl App {
    /// Top edge of the web view: the tab strip, the toolbar and, when shown, the bookmarks bar.
    pub fn chrome_top(&self) -> f32 {
        if self.settings.bookmark_bar {
            CHROME_TOP + BOOKMARK_BAR_H
        } else {
            CHROME_TOP
        }
    }

    // ------------------------------------------------------------------ suggestions

    /// Recompute the dropdown for the text being typed and show it (or hide it when nothing matches).
    pub fn update_suggestions(&mut self) {
        self.suggestions.clear();
        self.sugg_sel = 0;
        if self.addr_focused && !self.addr_selected && !self.addr_text.trim().is_empty() {
            let q = self.addr_text.trim().to_string();
            let marks = self.storage.as_ref().and_then(|s| s.get_bookmarks().ok()).unwrap_or_default();
            // Private windows never suggest from the normal browsing history.
            let history = if self.private_window { Vec::new() } else { self.storage.as_ref().and_then(|s| s.search_history(&q, 60).ok()).unwrap_or_default() };
            self.suggestions = suggest::build(&q, &marks, &history, MAX_SUGGESTIONS);
        }
        self.show_suggestions();
    }

    pub fn hide_suggestions(&mut self) {
        if !self.suggestions.is_empty() || self.sugg_shown {
            self.suggestions.clear();
            self.sugg_sel = 0;
            self.show_suggestions();
        }
    }

    /// Row 0 is always what was typed (search or address); the matches follow.
    fn suggestion_rows(&self) -> Vec<(String, String, String)> {
        let engine = self.search_engine;
        let typed = self.addr_text.trim();
        let prefix = engine.search_url("");
        let first = match crate::omnibox::resolve(typed, |q| engine.search_url(q)) {
            Some(crate::omnibox::Target::Navigate(u)) if u.starts_with(prefix.as_str()) => {
                ("\u{1F50D}".to_string(), typed.to_string(), format!("Search with {}", engine.name()))
            }
            Some(crate::omnibox::Target::Navigate(u)) => ("\u{1F310}".to_string(), typed.to_string(), u),
            Some(_) => ("\u{1F4C4}".to_string(), typed.to_string(), "Open Axomai page".to_string()),
            None => (String::new(), String::new(), String::new()),
        };
        let mut rows = vec![first];
        for s in &self.suggestions {
            let icon = if s.kind == Kind::Bookmark { "\u{2B50}" } else { "\u{1F552}" };
            let title = if s.title.is_empty() { s.url.clone() } else { s.title.clone() };
            rows.push((icon.to_string(), title, s.url.clone()));
        }
        rows
    }

    fn show_suggestions(&mut self) {
        if self.suggestions.is_empty() {
            if self.sugg_shown {
                if let Some(wv) = self.webview.as_ref() {
                    let _ = wv.evaluate_script("var e=document.getElementById('__ax_pop_suggest');if(e)e.remove()");
                }
                self.sugg_shown = false;
            }
            return;
        }
        let o = toolbar_layout(w_of(&self.gpu)).omnibox;
        let rows = self.suggestion_rows();
        if let Some(wv) = self.webview.as_ref() {
            let _ = wv.evaluate_script(&overlays::suggest_popup(self.core.theme, &self.tabs[self.active].shared.token, o.x, o.w, &rows, self.sugg_sel));
            self.sugg_shown = true;
        }
    }

    /// Up (-1) / Down (+1) in the dropdown. Returns false when there is no dropdown to move in.
    pub fn move_suggestion(&mut self, delta: i32) -> bool {
        if self.suggestions.is_empty() {
            return false;
        }
        let n = self.suggestions.len() as i32 + 1;
        self.sugg_sel = (self.sugg_sel as i32 + delta).rem_euclid(n) as usize;
        if self.sugg_sel > 0 {
            // Like other browsers, the address bar shows the highlighted address.
            self.addr_text = self.suggestions[self.sugg_sel - 1].url.clone();
        }
        self.addr_cursor = self.addr_text.chars().count();
        self.addr_selected = false;
        self.show_suggestions();
        self.redraw = true;
        true
    }

    /// Open the highlighted (or clicked) match. Row numbers are the dropdown's: 0 is the typed text.
    pub fn open_suggestion(&mut self, row: usize) {
        let url = match row.checked_sub(1).and_then(|i| self.suggestions.get(i)) {
            Some(s) => s.url.clone(),
            None => return self.submit_address(),
        };
        self.addr_text = url.clone();
        self.addr_focused = false;
        self.addr_selected = false;
        self.suggestions.clear();
        self.show_suggestions();
        self.navigate_active(&url);
        self.redraw = true;
    }

    // ------------------------------------------------------------------ bookmarks bar

    pub fn refresh_bar(&mut self) {
        self.bar_marks = if self.settings.bookmark_bar { self.storage.as_ref().and_then(|s| s.get_bookmarks().ok()).unwrap_or_default() } else { Vec::new() };
        self.redraw = true;
    }

    /// Chips of the bar in order (oldest bookmark first): rectangle, label and bookmark index.
    pub fn bar_layout(&mut self) -> Vec<(Rect, String, usize)> {
        let y = CHROME_TOP + (BOOKMARK_BAR_H - 24.0) / 2.0;
        let max_x = w_of(&self.gpu) - 12.0;
        let mut x = 12.0;
        let mut out = Vec::new();
        for i in (0..self.bar_marks.len()).rev() {
            let b: &Bookmark = &self.bar_marks[i];
            let label = if b.title.trim().is_empty() { crate::blocklist::host_of(&b.url) } else { b.title.trim().to_string() };
            let label = fit_text(&mut self.compositor, &label, 12.5, 130.0);
            let w = text_width(&mut self.compositor, &label, 12.5) + 36.0;
            if x + w > max_x {
                break;
            }
            out.push((Rect::new(x, y, w, 24.0), label, i));
            x += w + 4.0;
        }
        out
    }

    pub fn bar_click(&mut self, x: f32, y: f32) {
        let hit = self.bar_layout().into_iter().find(|(r, _, _)| r.contains(x, y));
        if let Some((_, _, i)) = hit {
            let url = self.bar_marks[i].url.clone();
            self.navigate_active(&url);
        }
    }

    pub fn bar_quads(&mut self) -> Vec<GpuQuad> {
        let mut quads = Vec::new();
        if !self.settings.bookmark_bar {
            return quads;
        }
        let th = self.core.theme;
        let w = w_of(&self.gpu);
        let rgb = |v: [u8; 3]| c(v[0], v[1], v[2], 255);
        quads.push(NativeGpuCompositor::solid_quad(0.0, CHROME_TOP, w, BOOKMARK_BAR_H, rgb(th.toolbar)));
        quads.push(NativeGpuCompositor::solid_quad(0.0, CHROME_TOP + BOOKMARK_BAR_H - 1.0, w, 1.0, c(th.primary[0], th.primary[1], th.primary[2], 51)));
        let (mx, my) = self.mouse;
        let items = self.bar_layout();
        if items.is_empty() {
            render_text(&mut self.compositor, &mut quads, "Bookmarks you add with the star appear here", 14.0, CHROME_TOP + 20.0, 12.0, rgb(th.muted), w - 12.0);
            return quads;
        }
        for (r, label, i) in items {
            if r.contains(mx, my) {
                quads.push(rr_rect(r, 8.0, c(th.primary[0], th.primary[1], th.primary[2], 40)));
            }
            let b = &self.bar_marks[i];
            let letter = b
                .title
                .chars()
                .chain(b.url.chars().skip_while(|c| !c.is_alphanumeric()))
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "*".into());
            let chip = Rect::new(r.x + 7.0, r.cy() - 7.0, 14.0, 14.0);
            quads.push(rr_rect(chip, 3.5, rgb(th.primary)));
            render_text(&mut self.compositor, &mut quads, &letter, chip.x + 3.5, chip.y + 11.0, 9.5, c(255, 255, 255, 255), chip.right());
            render_text(&mut self.compositor, &mut quads, &label, r.x + 26.0, r.cy() + 4.5, 12.5, rgb(th.heading), r.right());
        }
        quads
    }
}
