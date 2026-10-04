//! Per-site rules: sites that are always muted and sites that are blocked.

use crate::app::App;
use crate::tabs::TabKind;
use std::collections::HashSet;

/// A site name from user input or an address: lower case, no scheme, path, port or leading `www.`.
pub fn normalize_host(input: &str) -> Option<String> {
    let s = input.trim().to_ascii_lowercase();
    let s = s.strip_prefix("http://").or_else(|| s.strip_prefix("https://")).unwrap_or(&s);
    let host = s.split(&['/', '?', '#'][..]).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("").trim_start_matches("www.").trim_end_matches('.');
    let ok = host.len() >= 3
        && host.len() <= 253
        && host.contains('.')
        && !host.starts_with('.')
        && !host.contains("..")
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    ok.then(|| host.to_string())
}

/// True when `host` is `rule` or one of its subdomains.
pub fn host_matches(rule: &str, host: &str) -> bool {
    let host = host.trim_start_matches("www.");
    host == rule || host.strip_suffix(rule).map_or(false, |p| p.ends_with('.'))
}

pub fn matches_any(set: &HashSet<String>, url: &str) -> bool {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    let Some(host) = normalize_host(url) else { return false };
    set.iter().any(|r| host_matches(r, &host))
}

impl App {
    fn toast_site(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    /// Read the saved rules into memory (blocked hosts are also visible to the navigation hook).
    pub fn load_site_rules(&mut self) {
        let rules = self.storage.as_ref().map(|s| s.site_rules()).unwrap_or_default();
        self.muted_sites = rules.iter().filter(|r| r.1).map(|r| r.0.clone()).collect();
        let blocked: HashSet<String> = rules.iter().filter(|r| r.2).map(|r| r.0.clone()).collect();
        if let Ok(mut b) = self.hub.shield.blocked_sites.lock() {
            *b = blocked;
        }
    }

    fn set_site_rule(&mut self, host: &str, muted: Option<bool>, blocked: Option<bool>) {
        if let Some(s) = &self.storage {
            let _ = s.site_rule_set(host, muted, blocked);
        }
        self.load_site_rules();
        for i in 0..self.tabs.len() {
            let url = self.tabs[i].url.clone();
            self.apply_site_mute(i, &url);
        }
        self.refresh_internal_page();
        self.redraw = true;
    }

    fn current_web_host(&self) -> Option<String> {
        let t = &self.tabs[self.active];
        matches!(t.kind, TabKind::Web).then(|| normalize_host(&t.url)).flatten()
    }

    /// "Mute this site": every tab of the site stays silent, now and in the future.
    pub fn site_mute_current(&mut self) {
        let Some(host) = self.current_web_host() else { return self.toast_site("Open a web page first") };
        if self.private_window || self.tabs[self.active].private {
            return self.toast_site("Site rules are not saved in a private window");
        }
        let was = self.muted_sites.iter().any(|r| host_matches(r, &host));
        self.set_site_rule(&host, Some(!was), None);
        self.toast_site(&if was { format!("{} is no longer muted", host) } else { format!("\u{1F507} {} is muted", host) });
    }

    /// "Block this site": it can no longer be opened until it is unblocked in Settings.
    pub fn site_block_current(&mut self) {
        let Some(host) = self.current_web_host() else { return self.toast_site("Open a web page first") };
        if self.private_window || self.tabs[self.active].private {
            return self.toast_site("Site rules are not saved in a private window");
        }
        self.set_site_rule(&host, None, Some(true));
        let idx = self.active;
        let url = self.tabs[idx].url.clone();
        self.on_site_blocked(idx, &url);
    }

    /// Commands from the Settings page and the block page.
    pub fn site_rule_command(&mut self, action: &str, raw_host: &str) {
        let Some(host) = normalize_host(raw_host) else {
            return self.toast_site("That is not a site name");
        };
        match action {
            "block" => self.set_site_rule(&host, None, Some(true)),
            "unblock" => {
                self.set_site_rule(&host, None, Some(false));
                // The block page of that site goes back to the site itself.
                for i in 0..self.tabs.len() {
                    if self.tabs[i].title.starts_with("Site blocked") && normalize_host(&self.tabs[i].url).as_deref() == Some(host.as_str()) {
                        let url = self.tabs[i].url.clone();
                        if let Some(wv) = self.view_of(i) {
                            let _ = wv.load_url(&url);
                        }
                    }
                }
            }
            "mute" => self.set_site_rule(&host, Some(true), None),
            "unmute" => self.set_site_rule(&host, Some(false), None),
            _ => {}
        }
    }

    /// The navigation hook refused a blocked site: show the block page in that tab.
    pub fn on_site_blocked(&mut self, idx: usize, url: &str) {
        let Some(host) = normalize_host(url) else { return };
        let shared = self.tabs[idx].shared.clone();
        let html = crate::pages::site_blocked_page(&self.page_ctx(&shared), &host);
        let t = &mut self.tabs[idx];
        t.kind = TabKind::Web;
        t.url = url.to_string();
        t.title = "Site blocked".to_string();
        t.loading = false;
        if let Some(wv) = self.view_of(idx) {
            let _ = wv.load_html(&html);
        }
        self.redraw = true;
    }

    /// A tab went to `url`: mute it when the site is muted, and undo that when it leaves.
    pub fn apply_site_mute(&mut self, idx: usize, url: &str) {
        if idx >= self.tabs.len() {
            return;
        }
        let should = matches_any(&self.muted_sites, url);
        let t = &mut self.tabs[idx];
        let want = if should {
            t.site_muted = true;
            true
        } else if t.site_muted {
            t.site_muted = false;
            false
        } else {
            return;
        };
        if t.muted != want {
            t.muted = want;
            if let Some(wv) = self.view_of(idx) {
                crate::web::com::set_muted(wv, want);
            }
            self.redraw = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_normalised() {
        assert_eq!(normalize_host("https://www.Example.com/path?q=1").as_deref(), Some("example.com"));
        assert_eq!(normalize_host("example.com:8080").as_deref(), Some("example.com"));
        assert_eq!(normalize_host("  news.site.in ").as_deref(), Some("news.site.in"));
        assert_eq!(normalize_host("user@evil.com").as_deref(), Some("evil.com"));
        assert_eq!(normalize_host("localhost"), None);
        assert_eq!(normalize_host("bad host.com"), None);
        assert_eq!(normalize_host("a..b.com"), None);
        assert_eq!(normalize_host("javascript:alert(1)"), None);
        assert_eq!(normalize_host(""), None);
    }

    #[test]
    fn a_rule_covers_subdomains_but_not_lookalikes() {
        assert!(host_matches("example.com", "example.com"));
        assert!(host_matches("example.com", "www.example.com"));
        assert!(host_matches("example.com", "m.example.com"));
        assert!(!host_matches("example.com", "notexample.com"));
        assert!(!host_matches("example.com", "example.com.evil.org"));
    }

    #[test]
    fn only_web_addresses_match() {
        let set: HashSet<String> = ["example.com".to_string()].into_iter().collect();
        assert!(matches_any(&set, "https://sub.example.com/a"));
        assert!(!matches_any(&set, "file:///example.com"));
        assert!(!matches_any(&set, "https://other.org/example.com"));
    }
}
