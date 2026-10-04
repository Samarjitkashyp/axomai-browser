//! Page zoom (remembered per site), full-screen (F11 and a page's own full-screen button), "Save as PDF".

use crate::app::App;
use crate::passwords::origin_of;
use crate::tabs::TabKind;
use crate::web;

/// The zoom levels the browser steps through (same ladder as Chrome).
pub const ZOOM_STEPS: [f64; 17] = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0];

/// The next level above (`dir` > 0) or below (`dir` < 0) `current`; the nearest step is used as the starting point.
pub fn step_zoom(current: f64, dir: i32) -> f64 {
    let nearest = ZOOM_STEPS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - current).abs().partial_cmp(&(b.1 - current).abs()).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(7);
    // Sitting between two steps (as after Ctrl+wheel), the first press goes to the step in that direction.
    let at = ZOOM_STEPS[nearest];
    let idx = if dir > 0 {
        if current > at + 0.001 { nearest + 1 } else if current < at - 0.001 { nearest } else { nearest + 1 }
    } else if current < at - 0.001 {
        nearest.saturating_sub(1)
    } else if current > at + 0.001 {
        nearest
    } else {
        nearest.saturating_sub(1)
    };
    ZOOM_STEPS[idx.min(ZOOM_STEPS.len() - 1)]
}

pub fn percent(factor: f64) -> u32 {
    (factor * 100.0).round().clamp(1.0, 999.0) as u32
}

/// File name for a saved PDF: the page title made safe for Windows, or "page".
pub fn pdf_name(title: &str) -> String {
    let cleaned: String = title.chars().map(|c| if c.is_control() || "\\/:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned: String = cleaned.chars().take(80).collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    format!("{}.pdf", if cleaned.is_empty() { "page" } else { &cleaned })
}

impl App {
    /// Current zoom of the active tab.
    pub fn zoom_factor(&self) -> f64 {
        self.tabs[self.active].zoom
    }

    pub fn zoom_command(&mut self, what: &str) {
        let cur = self.zoom_factor();
        let target = match what {
            "in" => step_zoom(cur, 1),
            "out" => step_zoom(cur, -1),
            _ => 1.0,
        };
        self.tabs[self.active].zoom = target;
        if let Some(wv) = &self.webview {
            web::com::set_zoom(wv, target);
        }
        self.remember_zoom(self.active, target);
        self.redraw = true;
    }

    /// The web view reported a new zoom (wheel, keys, or our own change).
    pub fn on_zoom_event(&mut self, idx: usize, factor: f64) {
        if (self.tabs[idx].zoom - factor).abs() < 0.0005 {
            return;
        }
        self.tabs[idx].zoom = factor;
        if matches!(self.tabs[idx].kind, TabKind::Web) {
            self.remember_zoom(idx, factor);
        }
        self.redraw = true;
    }

    fn page_origin(&self, idx: usize) -> Option<String> {
        let live = self.view_of(idx).and_then(|wv| wv.url().ok());
        origin_of(live.as_deref().unwrap_or(&self.tabs[idx].url)).or_else(|| origin_of(&self.tabs[idx].url))
    }

    fn remember_zoom(&mut self, idx: usize, factor: f64) {
        if self.private_window || self.tabs[idx].private || !matches!(self.tabs[idx].kind, TabKind::Web) {
            return;
        }
        if let (Some(origin), Some(s)) = (self.page_origin(idx), &self.storage) {
            let _ = s.set_site_zoom(&origin, factor);
        }
    }

    /// A new address is being opened in tab `idx`: use that site's remembered zoom (100% when it has none).
    pub fn apply_site_zoom(&mut self, idx: usize, url: &str) {
        let Some(origin) = origin_of(url) else { return };
        let wanted = if self.private_window || self.tabs[idx].private { 1.0 } else { self.storage.as_ref().and_then(|s| s.get_site_zoom(&origin)).unwrap_or(1.0) };
        if (self.tabs[idx].zoom - wanted).abs() > 0.0005 {
            self.tabs[idx].zoom = wanted;
            if let Some(wv) = self.view_of(idx) {
                web::com::set_zoom(wv, wanted);
            }
            self.redraw = true;
        }
    }

    /// Our own pages are always shown at 100%.
    pub fn reset_internal_zoom(&mut self, idx: usize) {
        if (self.tabs[idx].zoom - 1.0).abs() > 0.0005 {
            self.tabs[idx].zoom = 1.0;
            if let Some(wv) = self.view_of(idx) {
                web::com::set_zoom(wv, 1.0);
            }
        }
    }

    // ------------------------------------------------------------------ full screen

    pub fn set_fullscreen(&mut self, on: bool) {
        if self.fullscreen == on {
            return;
        }
        self.fullscreen = on;
        self.window.set_fullscreen(if on { Some(tao::window::Fullscreen::Borderless(None)) } else { None });
        self.fit_active_view();
        self.redraw = true;
    }

    pub fn toggle_fullscreen(&mut self) {
        // F11 always wins over a page's own full-screen request.
        self.html_fullscreen = false;
        self.set_fullscreen(!self.fullscreen);
    }

    /// A page asked to show an element full screen (a video's full-screen button) or left it again.
    pub fn on_html_fullscreen(&mut self, on: bool) {
        if on {
            if !self.fullscreen {
                self.html_fullscreen = true;
                self.set_fullscreen(true);
            }
        } else if self.html_fullscreen {
            self.html_fullscreen = false;
            self.set_fullscreen(false);
        }
    }

    // ------------------------------------------------------------------ save as PDF

    pub fn save_pdf(&mut self) {
        let t = &self.tabs[self.active];
        if !matches!(t.kind, TabKind::Web) {
            if let Some(wv) = &self.webview {
                let shared = self.shared();
                self.core.toast(wv, "Open a web page to save it as a PDF", None, &shared);
            }
            return;
        }
        let dir = self.hub.download_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = web::unique_path(&dir, &pdf_name(&t.title));
        if let Some(wv) = &self.webview {
            web::com::print_to_pdf(wv, &path, self.shared());
        }
    }

    pub fn on_pdf_done(&mut self, path: &str, ok: bool) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            let msg = if ok { format!("Saved PDF: {}", path) } else { "Could not save the PDF".to_string() };
            self.core.toast(wv, &msg, if ok { Some(("Show downloads", &shared.token, "downloads")) } else { None }, &shared);
        }
    }

    // ------------------------------------------------------------------ small commands

    pub fn toggle_bookmark_bar(&mut self) {
        let on = !self.settings.bookmark_bar;
        self.apply_setting("bookmark_bar", if on { "true" } else { "false" });
    }

    pub fn hard_reload(&mut self) {
        if matches!(self.tabs[self.active].kind, TabKind::Web) {
            if let Some(wv) = &self.webview {
                web::com::reload_hard(wv);
            }
        } else {
            self.reload_active();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_up_and_down_the_ladder() {
        assert_eq!(step_zoom(1.0, 1), 1.1);
        assert_eq!(step_zoom(1.0, -1), 0.9);
        assert_eq!(step_zoom(5.0, 1), 5.0, "top of the ladder");
        assert_eq!(step_zoom(0.25, -1), 0.25, "bottom of the ladder");
        assert_eq!(step_zoom(3.0, -1), 2.5);
    }

    #[test]
    fn zoom_between_steps_goes_to_the_next_step_in_that_direction() {
        assert_eq!(step_zoom(1.15, 1), 1.25);
        assert_eq!(step_zoom(1.15, -1), 1.1);
        assert_eq!(step_zoom(1.2, 1), 1.25);
    }

    #[test]
    fn percent_rounds() {
        assert_eq!(percent(1.0), 100);
        assert_eq!(percent(1.25), 125);
        assert_eq!(percent(0.67), 67);
    }

    #[test]
    fn pdf_names_are_safe() {
        assert_eq!(pdf_name("My: page / <title>?"), "My page title.pdf");
        assert_eq!(pdf_name("   "), "page.pdf");
        assert_eq!(pdf_name("..."), "page.pdf");
        assert!(pdf_name(&"x".repeat(300)).len() <= 84);
    }
}
