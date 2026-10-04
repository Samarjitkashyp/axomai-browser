//! Split view: two tabs shown side by side. The tab you last clicked is the active one (address bar, shortcuts and
//! toolbar act on it); the other keeps its own page, history and scroll position.

use crate::app::App;
use crate::rendering::c;
use crate::types::SIDEBAR_W;
use axomai_engine::{GpuQuad, NativeGpuCompositor};

pub const GAP: f32 = 8.0;

/// `(x, width)` of the left and the right pane for a window `w` logical units wide.
pub fn split_panes(w: f32) -> ((f32, f32), (f32, f32)) {
    let usable = (w - SIDEBAR_W - GAP).max(2.0);
    let half = (usable / 2.0).floor();
    ((SIDEBAR_W, half), (SIDEBAR_W + half + GAP, usable - half))
}

impl App {
    /// `(x, width)` the web view of tab `tab_id` occupies.
    pub fn pane_of(&self, tab_id: u64) -> (f32, f32) {
        let (w, _) = self.window_size();
        match self.split {
            Some((l, _)) if l == tab_id => split_panes(w).0,
            Some((_, r)) if r == tab_id => split_panes(w).1,
            _ => (SIDEBAR_W, (w - SIDEBAR_W).max(1.0)),
        }
    }

    /// Id of the tab shown beside the active one, if the view is split.
    pub fn split_partner(&self) -> Option<u64> {
        let active = self.tabs.get(self.active)?.id;
        self.split.and_then(|(l, r)| if l == active { Some(r) } else if r == active { Some(l) } else { None })
    }

    /// Size and show the partner's web view (the active one is placed by `fit_active_view`).
    pub fn fit_partner(&self) {
        let Some(id) = self.split_partner() else { return };
        let Some(idx) = self.idx_of_id(id) else { return };
        let Some(wv) = self.tabs[idx].view.as_ref() else { return };
        let (_, h) = self.window_size();
        let (s, top) = (self.scale.max(0.5), self.chrome_top());
        let (x, w) = self.pane_of(id);
        let _ = wv.set_bounds(wry::Rect {
            position: wry::dpi::PhysicalPosition::new((x * s).round() as i32, (top * s).round() as i32).into(),
            size: wry::dpi::PhysicalSize::new((w * s).max(1.0) as u32, ((h - top) * s).max(1.0) as u32).into(),
        });
        let _ = wv.set_visible(true);
    }

    /// Show the current tab and a new tab side by side.
    pub fn start_split(&mut self) {
        if self.split.is_some() {
            return;
        }
        let left = self.tabs[self.active].id;
        let at = self.tabs.len();
        self.open_tab_at("about:home", at, false);
        let right = self.tabs[at].id;
        self.split = Some((left, right));
        self.activate(at);
        self.fit_active_view();
        self.redraw = true;
    }

    /// Go back to one pane. The other tab stays open, hidden, with its page intact.
    pub fn end_split(&mut self) {
        let Some(partner) = self.split_partner() else {
            self.split = None;
            return;
        };
        self.split = None;
        if let Some(idx) = self.idx_of_id(partner) {
            if let Some(wv) = self.tabs[idx].view.as_ref() {
                let _ = wv.set_visible(false);
            }
        }
        self.fit_active_view();
        self.redraw = true;
    }

    pub fn toggle_split(&mut self) {
        if self.split.is_some() {
            self.end_split();
        } else {
            self.start_split();
        }
    }

    /// The divider between the panes and a coloured bar over the pane that has the focus.
    pub fn split_quads(&mut self) -> Vec<GpuQuad> {
        let mut quads = Vec::new();
        let Some((l, r)) = self.split else { return quads };
        let (w, h) = self.window_size();
        let top = self.chrome_top();
        let th = self.core.theme;
        let active = self.tabs[self.active].id;
        let ((lx, lw), (rx, rw)) = split_panes(w);
        quads.push(NativeGpuCompositor::solid_quad(lx + lw, top - 3.0, GAP, h - top + 3.0, c(th.primary[0], th.primary[1], th.primary[2], 70)));
        for (id, x, width) in [(l, lx, lw), (r, rx, rw)] {
            let color = if id == active { c(th.primary[0], th.primary[1], th.primary[2], 255) } else { c(th.primary[0], th.primary[1], th.primary[2], 60) };
            quads.push(NativeGpuCompositor::solid_quad(x, top - 3.0, width, 3.0, color));
        }
        quads
    }

    /// A page in one of the panes was clicked: that pane becomes the active one.
    pub fn on_pane_focus(&mut self, tab_id: u64) {
        if let Some((l, r)) = self.split {
            if (tab_id == l || tab_id == r) && self.tabs[self.active].id != tab_id {
                if let Some(idx) = self.idx_of_id(tab_id) {
                    self.activate(idx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panes_share_the_width_with_a_gap_between() {
        let ((lx, lw), (rx, rw)) = split_panes(1200.0);
        assert_eq!(lx, 0.0);
        assert_eq!(rx, lw + GAP);
        assert!((lw + GAP + rw - 1200.0).abs() < 0.001, "nothing is lost or overlapping");
        assert!((lw - rw).abs() <= 1.0);
    }

    #[test]
    fn tiny_windows_do_not_collapse() {
        let ((_, lw), (_, rw)) = split_panes(5.0);
        assert!(lw >= 1.0 && rw >= 1.0);
    }
}
