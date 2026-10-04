//! Site icons for bookmarks and history: saved from the pages you visit, fetched (once) for bookmarks that have none.

use crate::app::App;
use crate::blocklist::host_of;
use crate::tabs::TabKind;
use std::collections::HashMap;

/// host -> `data:image/png;base64,...`, for the pages that list sites.
pub type Icons = HashMap<String, String>;

/// Where icons of sites we have not visited come from (Google's public favicon service, free).
fn service_url(host: &str) -> String {
    format!("https://www.google.com/s2/favicons?domain={}&sz=64", host)
}

/// Only plain host names are looked up: no ports, paths, IP addresses or odd characters.
pub fn lookup_host(url: &str) -> Option<String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let h = host_of(url);
    let ok = h.contains('.') && h.len() < 100 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') && !h.chars().all(|c| c.is_ascii_digit() || c == '.');
    ok.then_some(h)
}

/// An icon image the browser is willing to keep: a real PNG / ICO / GIF / JPEG of sane size.
pub fn plausible_icon(bytes: &[u8]) -> bool {
    bytes.len() > 100 && bytes.len() < 200_000 && (bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"\x00\x00\x01\x00") || bytes.starts_with(b"GIF8") || bytes.starts_with(b"\xff\xd8"))
}

pub fn data_uri(png: &[u8]) -> String {
    let mime = if png.starts_with(b"\x89PNG") {
        "image/png"
    } else if png.starts_with(b"GIF8") {
        "image/gif"
    } else if png.starts_with(b"\xff\xd8") {
        "image/jpeg"
    } else {
        "image/x-icon"
    };
    format!("data:{};base64,{}", mime, crate::sys::base64_encode(png))
}

impl App {
    /// Remember the icon a page announced for its site.
    pub fn store_site_icon(&mut self, tab_idx: usize, bytes: &[u8]) {
        let t = &self.tabs[tab_idx];
        if self.private_window || t.private || !matches!(t.kind, TabKind::Web) || !plausible_icon(bytes) {
            return;
        }
        if let (Some(host), Some(s)) = (lookup_host(&t.url), &self.storage) {
            let _ = s.set_favicon(&host, bytes);
            self.host_slots.remove(&host);
        }
    }

    /// Atlas slot of a site's saved icon (loaded on first use).
    pub fn icon_slot(&mut self, host: &str) -> Option<usize> {
        if let Some(slot) = self.host_slots.get(host) {
            return Some(*slot);
        }
        let bytes = self.storage.as_ref()?.get_favicon(host)?;
        let slot = self.favicons.add(&bytes)?;
        self.host_slots.insert(host.to_string(), slot);
        Some(slot)
    }

    /// Fetch icons for sites that have none yet (a few at a time, once each).
    pub fn ensure_icons(&mut self, urls: &[String]) {
        if self.tabs.is_empty() {
            return;
        }
        let need: Vec<String> = {
            let Some(st) = &self.storage else { return };
            let mut hosts: Vec<String> = Vec::new();
            for url in urls {
                let Some(host) = lookup_host(url) else { continue };
                if !hosts.contains(&host) && !self.icon_pending.contains(&host) && st.get_favicon(&host).is_none() {
                    hosts.push(host);
                }
                if hosts.len() >= 30 {
                    break;
                }
            }
            hosts
        };
        for host in need {
            self.icon_pending.insert(host.clone());
            let shared = self.shared();
            std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(6)).build();
                let bytes = agent.get(&service_url(&host)).call().ok().and_then(|r| {
                    use std::io::Read;
                    let mut b = Vec::new();
                    r.into_reader().take(200_000).read_to_end(&mut b).ok().map(|_| b)
                });
                let payload = bytes.filter(|b| plausible_icon(b)).map(|b| crate::sys::base64_encode(&b)).unwrap_or_default();
                shared.push_event(crate::web::WebEvent::PageData("site-icon".into(), host, payload));
            });
        }
    }

    /// A fetched icon arrived (empty payload: the service had none).
    pub fn site_icon_arrived(&mut self, host: &str, payload: &str) {
        self.icon_pending.remove(host);
        if let (Some(bytes), Some(s)) = (crate::sys::base64_decode(payload).filter(|b| plausible_icon(b)), &self.storage) {
            let _ = s.set_favicon(host, &bytes);
            self.host_slots.remove(host);
            self.redraw = true;
            if matches!(self.tabs[self.active].kind, TabKind::Page("bookmarks") | TabKind::Page("history")) {
                self.load_active_page();
            }
        }
    }

    /// Icons (as data URIs) for the hosts of these addresses, for the pages that list sites.
    pub fn icon_map(&self, urls: &[&str]) -> Icons {
        let mut map = Icons::new();
        let Some(st) = &self.storage else { return map };
        for u in urls {
            if let Some(h) = lookup_host(u) {
                if !map.contains_key(&h) {
                    if let Some(png) = st.get_favicon(&h) {
                        map.insert(h, data_uri(&png));
                    }
                }
            }
        }
        map
    }

    /// Bookmarks bar drawn with each site's icon (letter chip until the icon is known).
    pub fn bar_chip_icon(&mut self, url: &str) -> Option<usize> {
        let host = lookup_host(url)?;
        self.icon_slot(&host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ordinary_hosts_are_looked_up() {
        assert_eq!(lookup_host("https://www.Example.com/a?b=1").as_deref(), Some("www.example.com"));
        for bad in ["http://127.0.0.1:8765/x", "http://localhost/", "https://192.168.1.1/", "javascript:1", "https://a b.com/", "file:///c:/x.html", "https://evil.com@x/\"><"] {
            assert_eq!(lookup_host(bad), None, "{}", bad);
        }
    }

    #[test]
    fn icon_bytes_are_checked() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(std::iter::repeat(0u8).take(200));
        assert!(plausible_icon(&png));
        assert!(!plausible_icon(b"<html>error page</html>"));
        assert!(!plausible_icon(&[0u8; 10]));
        assert!(data_uri(&png).starts_with("data:image/png;base64,"));
    }
}
