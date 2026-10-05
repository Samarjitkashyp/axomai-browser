//! Named sets of tabs: save what is open under a name and open it all again later.

use crate::app::App;
use crate::tabs::TabKind;

pub const MAX_SESSION_TABS: usize = 60;

/// A session name from user input: no control characters, at most 40 characters.
pub fn clean_session_name(raw: &str) -> String {
    raw.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(40).collect()
}

/// Only addresses that can be opened again safely are kept.
pub fn keepable(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("axomai://") || url == "about:settings"
}

impl App {
    fn toast_sessions(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    pub fn open_session_prompt(&mut self) {
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            let _ = wv.focus();
            let _ = wv.evaluate_script(&crate::overlays::name_prompt(self.core.theme, &shared.token, "Save tabs as a session", "Session name", "Save", "session-save"));
        }
    }

    pub fn session_save(&mut self, raw_name: &str) {
        let name = clean_session_name(raw_name);
        if name.is_empty() {
            return self.toast_sessions("Give the session a name");
        }
        if self.private_window {
            return self.toast_sessions("Sessions are not saved in a private window");
        }
        let urls: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| !t.private)
            .map(|t| match t.kind {
                TabKind::Web | TabKind::Source => t.url.clone(),
                other => other.address().unwrap_or_default(),
            })
            .filter(|u| keepable(u))
            .take(MAX_SESSION_TABS)
            .collect();
        if urls.is_empty() {
            return self.toast_sessions("There are no tabs to save");
        }
        let ok = self.storage.as_ref().map_or(false, |s| s.session_save(&name, &urls).is_ok());
        self.toast_sessions(&if ok { format!("\u{1F4BE} Saved {} tabs as \"{}\"", urls.len(), name) } else { "Could not save the session".to_string() });
        self.refresh_internal_page();
    }

    /// Open every tab of a saved session next to the current ones; the first one comes to the front.
    pub fn session_open(&mut self, id: i64) {
        let Some(session) = self.storage.as_ref().and_then(|s| s.session_get(id)) else { return };
        for (i, url) in session.urls.iter().filter(|u| keepable(u)).take(MAX_SESSION_TABS).enumerate() {
            let at = self.tabs.len();
            self.open_tab_at(url, at, i == 0);
        }
        self.redraw = true;
    }

    pub fn session_delete(&mut self, id: i64) {
        if let Some(s) = &self.storage {
            let _ = s.session_delete(id);
        }
        self.refresh_internal_page();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_session_name("  Work \u{7}day "), "Work day");
        assert_eq!(clean_session_name(&"x".repeat(100)).chars().count(), 40);
        assert_eq!(clean_session_name("   "), "");
    }

    #[test]
    fn only_safe_addresses_are_kept() {
        assert!(keepable("https://a.test") && keepable("axomai://history") && keepable("about:settings"));
        assert!(!keepable("javascript:alert(1)") && !keepable("file:///c:/x") && !keepable("data:text/html,x") && !keepable(""));
    }
}
