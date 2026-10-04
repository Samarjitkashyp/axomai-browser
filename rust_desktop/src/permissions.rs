//! Site permissions (camera, microphone, location, ...), HTTPS-only upgrades and the privacy switches that go
//! with them.
//!
//! A permission question is answered by a bar drawn by the browser itself above the page (never by page-level
//! HTML), so a web page cannot click "Allow" on its own behalf.

use crate::app::App;
use crate::passwords::origin_of;
use crate::rendering::{c, fit_text, render_text, render_text_centered, w_of};
use crate::tabs::TabKind;
use crate::toolbar::{rr_rect, Rect};
use crate::types::{BOOKMARK_BAR_H, CHROME_TOP};
use axomai_engine::{GpuQuad, NativeGpuCompositor};
use std::collections::HashSet;

pub const INFOBAR_H: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    Allow,
    Ask,
    Deny,
}

/// What happens to a permission request before the user is asked.
pub fn policy(name: &str) -> Policy {
    match name {
        "autoplay" => Policy::Allow,
        "other" => Policy::Deny,
        _ => Policy::Ask,
    }
}

/// `(phrase for the bar, short name for the list)`.
pub fn label(name: &str) -> (&'static str, &'static str) {
    match name {
        "camera" => ("use your camera", "Camera"),
        "microphone" => ("use your microphone", "Microphone"),
        "location" => ("know your location", "Location"),
        "notifications" => ("show notifications", "Notifications"),
        "sensors" => ("use motion sensors", "Motion sensors"),
        "clipboard" => ("see text and images you copied", "Clipboard"),
        "downloads" => ("download several files", "Automatic downloads"),
        "files" => ("edit files on your device", "File access"),
        "fonts" => ("see the fonts installed on your device", "Local fonts"),
        "midi" => ("control your MIDI devices", "MIDI devices"),
        "windows" => ("manage windows on your screens", "Window management"),
        "autoplay" => ("play media automatically", "Autoplay"),
        _ => ("use a feature of your device", "Other"),
    }
}

/// `http://` address -> the `https://` address to try instead, or `None` when it must stay as it is: local and
/// private names, explicit ports, and hosts the user chose to keep on http for this session.
pub fn upgrade_target(url: &str, exempt: &HashSet<String>) -> Option<String> {
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("HTTP://"))?;
    let authority = rest.split(&['/', '?', '#'][..]).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = match authority.rsplit_once(':') {
        Some((h, "80")) => h,
        Some(_) if !authority.starts_with('[') => return None, // explicit non-default port
        _ => authority,
    };
    let host = host.to_ascii_lowercase();
    let is_ipv4 = host.split('.').count() == 4 && host.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    let local = host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") || host.ends_with(".lan");
    if host.is_empty() || !host.contains('.') || host.starts_with('[') || is_ipv4 || local || exempt.contains(&host) {
        return None;
    }
    Some(format!("https://{}", &url[7..]))
}

pub struct PermAsk {
    pub id: u64,
    pub tab_id: u64,
    pub origin: String,
    pub name: String,
}

impl App {
    fn perm_active(&self) -> Option<&PermAsk> {
        let tab = self.tabs[self.active].id;
        self.perm_queue.iter().find(|p| p.tab_id == tab)
    }

    /// Top edge of the web view: tab strip, toolbar, bookmarks bar and the permission bar when shown.
    pub fn chrome_top(&self) -> f32 {
        self.infobar_y() + if self.infobar_on { INFOBAR_H } else { 0.0 }
    }

    fn infobar_y(&self) -> f32 {
        CHROME_TOP + if self.settings.bookmark_bar { BOOKMARK_BAR_H } else { 0.0 }
    }

    /// Show or hide the permission bar when the question for the active tab appears or goes away.
    pub fn sync_infobar(&mut self) {
        let want = self.perm_active().is_some();
        if want != self.infobar_on {
            self.infobar_on = want;
            self.fit_active_view();
            self.redraw = true;
        }
    }

    /// A page asked for a permission (`uri` is the page asking, as seen by the web view).
    pub fn on_permission_ask(&mut self, tab_idx: usize, id: u64, uri: &str, name: &str) {
        match policy(name) {
            Policy::Allow => return crate::web::com::permission_answer(id, true),
            Policy::Deny => return crate::web::com::permission_answer(id, false),
            Policy::Ask => {}
        }
        let Some(origin) = origin_of(uri) else {
            // One of our own pages: only the home page's voice search may use the microphone.
            let ours = self.tabs[tab_idx].shared.trusted.load(std::sync::atomic::Ordering::SeqCst);
            return crate::web::com::permission_answer(id, ours && name == "microphone");
        };
        let private = self.private_window || self.tabs[tab_idx].private;
        let known = if private {
            self.perm_session.get(&(origin.clone(), name.to_string())).copied()
        } else {
            self.storage.as_ref().and_then(|s| s.get_permission(&origin, name))
        };
        match known {
            Some(allow) => crate::web::com::permission_answer(id, allow),
            None => {
                self.perm_queue.push(PermAsk { id, tab_id: self.tabs[tab_idx].id, origin, name: name.to_string() });
                self.sync_infobar();
            }
        }
    }

    /// 1 = Allow, 0 = Block, anything else = dismissed (denied this time only).
    fn answer_permission(&mut self, answer: i32) {
        let Some(pos) = self.perm_queue.iter().position(|p| p.tab_id == self.tabs[self.active].id) else { return };
        let ask = self.perm_queue.remove(pos);
        if answer == 1 || answer == 0 {
            let allow = answer == 1;
            let private = self.private_window || self.tabs[self.active].private;
            if private {
                self.perm_session.insert((ask.origin.clone(), ask.name.clone()), allow);
            } else if let Some(s) = &self.storage {
                let _ = s.set_permission(&ask.origin, &ask.name, allow);
            }
        }
        crate::web::com::permission_answer(ask.id, answer == 1);
        self.sync_infobar();
        self.reload_permissions_page();
    }

    /// Deny and forget every question a closed tab was still waiting on.
    pub fn drop_permissions_for_tab(&mut self, tab_id: u64) {
        let (gone, keep): (Vec<PermAsk>, Vec<PermAsk>) = std::mem::take(&mut self.perm_queue).into_iter().partition(|p| p.tab_id == tab_id);
        self.perm_queue = keep;
        for p in gone {
            crate::web::com::permission_answer(p.id, false);
        }
    }

    pub fn reload_permissions_page(&mut self) {
        if matches!(self.tabs[self.active].kind, TabKind::Page("permissions")) {
            self.load_active_page();
        }
    }

    /// `perm-<action>/<arg>` from the Site permissions page.
    pub fn permission_command(&mut self, action: &str, arg: &str) {
        match action {
            "set" => {
                if let Some((id, decision)) = arg.split_once('/') {
                    if let (Ok(id), Some(s)) = (id.parse::<i64>(), &self.storage) {
                        let _ = s.set_permission_by_id(id, decision == "allow");
                    }
                }
            }
            "delete" => {
                if let (Ok(id), Some(s)) = (arg.parse::<i64>(), &self.storage) {
                    let _ = s.delete_permission(id);
                }
            }
            "clear" => {
                if let Some(s) = &self.storage {
                    let _ = s.clear_permissions();
                }
            }
            _ => {}
        }
        self.reload_permissions_page();
    }

    // ---------------------------------------------------------------- the bar

    fn infobar_buttons(&self) -> (Rect, Rect, Rect) {
        let w = w_of(&self.gpu);
        let y = self.infobar_y() + (INFOBAR_H - 28.0) / 2.0;
        let close = Rect::new(w - 40.0, y, 28.0, 28.0);
        let block = Rect::new(close.x - 8.0 - 76.0, y, 76.0, 28.0);
        let allow = Rect::new(block.x - 8.0 - 76.0, y, 76.0, 28.0);
        (allow, block, close)
    }

    /// Click inside the bar. Returns true when the click belonged to it.
    pub fn infobar_click(&mut self, x: f32, y: f32) -> bool {
        if !self.infobar_on || y < self.infobar_y() || y >= self.infobar_y() + INFOBAR_H {
            return false;
        }
        let (allow, block, close) = self.infobar_buttons();
        if allow.contains(x, y) {
            self.answer_permission(1);
        } else if block.contains(x, y) {
            self.answer_permission(0);
        } else if close.contains(x, y) {
            self.answer_permission(-1);
        }
        true
    }

    pub fn infobar_quads(&mut self) -> Vec<GpuQuad> {
        let mut quads = Vec::new();
        let Some((origin, name)) = self.perm_active().map(|p| (p.origin.clone(), p.name.clone())) else { return quads };
        if !self.infobar_on {
            return quads;
        }
        let th = self.core.theme;
        let w = w_of(&self.gpu);
        let y0 = self.infobar_y();
        let rgb = |v: [u8; 3]| c(v[0], v[1], v[2], 255);
        quads.push(NativeGpuCompositor::solid_quad(0.0, y0, w, INFOBAR_H, rgb(th.toolbar)));
        quads.push(NativeGpuCompositor::solid_quad(0.0, y0, w, 3.0, rgb(th.primary)));
        quads.push(NativeGpuCompositor::solid_quad(0.0, y0 + INFOBAR_H - 1.0, w, 1.0, c(th.primary[0], th.primary[1], th.primary[2], 70)));
        let host = origin.split_once("://").map(|(_, h)| h).unwrap_or(&origin).to_string();
        let (allow, block, close) = self.infobar_buttons();
        let text = format!("{} wants to {}", host, label(&name).0);
        let shown = fit_text(&mut self.compositor, &text, 13.5, allow.x - 32.0);
        render_text(&mut self.compositor, &mut quads, &shown, 16.0, y0 + 25.0, 13.5, rgb(th.heading), allow.x - 8.0);
        quads.push(rr_rect(allow, 9.0, rgb(th.primary)));
        render_text_centered(&mut self.compositor, &mut quads, "Allow", allow.cx(), allow.cy() + 4.5, 12.5, c(255, 255, 255, 255));
        quads.push(rr_rect(block, 9.0, c(th.primary[0], th.primary[1], th.primary[2], 40)));
        render_text_centered(&mut self.compositor, &mut quads, "Block", block.cx(), block.cy() + 4.5, 12.5, rgb(th.heading));
        render_text_centered(&mut self.compositor, &mut quads, "x", close.cx(), close.cy() + 4.5, 14.0, rgb(th.muted));
        quads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> HashSet<String> {
        HashSet::new()
    }

    #[test]
    fn plain_http_sites_are_upgraded() {
        assert_eq!(upgrade_target("http://example.com/a?b=1#c", &none()).as_deref(), Some("https://example.com/a?b=1#c"));
        assert_eq!(upgrade_target("http://example.com:80/x", &none()).as_deref(), Some("https://example.com:80/x"));
        assert_eq!(upgrade_target("http://www.example.co.uk", &none()).as_deref(), Some("https://www.example.co.uk"));
    }

    #[test]
    fn local_private_and_odd_addresses_stay_on_http() {
        for u in ["http://localhost:3000/", "http://127.0.0.1:8765/a", "http://192.168.1.10/", "http://printer.local/", "http://intranet/", "http://example.com:8080/", "http://[::1]/", "https://example.com/", "ftp://example.com/", "http://app.localhost/"] {
            assert_eq!(upgrade_target(u, &none()), None, "{}", u);
        }
    }

    #[test]
    fn exempt_hosts_are_left_alone() {
        let mut ex = HashSet::new();
        ex.insert("old-site.org".to_string());
        assert_eq!(upgrade_target("http://old-site.org/", &ex), None);
        assert!(upgrade_target("http://other.org/", &ex).is_some());
    }

    #[test]
    fn userinfo_cannot_hide_the_real_host() {
        assert_eq!(upgrade_target("http://good.com@localhost/", &none()), None);
    }

    #[test]
    fn policies() {
        assert_eq!(policy("camera"), Policy::Ask);
        assert_eq!(policy("autoplay"), Policy::Allow);
        assert_eq!(policy("other"), Policy::Deny);
        assert_eq!(label("microphone").1, "Microphone");
    }
}
