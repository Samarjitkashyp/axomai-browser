//! Reading list (pages saved to read later) and notes attached to web pages.

use crate::app::App;
use crate::overlays;
use crate::tabs::TabKind;
use serde_json::Value;

pub const MAX_NOTE_CHARS: usize = 2000;
pub const MAX_NOTES_PER_PAGE: usize = 8;

/// The address a note belongs to: the page without its `#fragment`.
pub fn page_key(url: &str) -> Option<String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() > 2000 {
        return None;
    }
    Some(url.split('#').next().unwrap_or(url).to_string())
}

pub fn clean_note(text: &str) -> String {
    text.chars().filter(|c| !c.is_control() || *c == '\n').take(MAX_NOTE_CHARS).collect()
}

impl App {
    fn toast_here(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    // ------------------------------------------------------------------ reading list

    pub fn reading_add(&mut self) {
        let t = &self.tabs[self.active];
        let Some(url) = page_key(&t.url).filter(|_| matches!(t.kind, TabKind::Web)) else {
            return self.toast_here("Only web pages can be added to the reading list");
        };
        if self.private_window || t.private {
            return self.toast_here("The reading list is not used in a private window");
        }
        let title = if t.title.is_empty() { crate::blocklist::host_of(&url) } else { t.title.clone() };
        let added = self.storage.as_ref().and_then(|s| s.reading_add(&url, &title).ok());
        self.toast_here(match added {
            Some(true) => "\u{1F4D6} Added to your reading list",
            Some(false) => "Already in your reading list",
            None => "Could not save to the reading list",
        });
        self.ensure_icons(&[url]);
        self.reload_library_page();
    }

    pub fn reading_command(&mut self, action: &str, id: i64) {
        if let Some(s) = &self.storage {
            let _ = match action {
                "read" => s.reading_set_read(id, true),
                "unread" => s.reading_set_read(id, false),
                "delete" => s.reading_delete(id),
                "clear-read" => s.reading_clear_read(),
                _ => Ok(()),
            };
        }
        self.reload_library_page();
    }

    fn reload_library_page(&mut self) {
        if matches!(self.tabs[self.active].kind, TabKind::Page("readinglist") | TabKind::Page("notes")) {
            self.load_active_page();
        }
    }

    // ------------------------------------------------------------------ notes

    /// A page finished loading: show the notes that belong to it.
    pub fn show_notes_for(&mut self, tab_idx: usize, url: &str) {
        if self.private_window || self.tabs[tab_idx].private || !matches!(self.tabs[tab_idx].kind, TabKind::Web) {
            return;
        }
        let Some(key) = page_key(url) else { return };
        let notes = self.storage.as_ref().map(|s| s.notes_for(&key)).unwrap_or_default();
        if notes.is_empty() {
            return;
        }
        if let Some(wv) = self.view_of(tab_idx) {
            let _ = wv.evaluate_script(&overlays::notes_overlay(self.core.theme, &notes, 0));
        }
    }

    /// "Add note to this page": a new empty note, focused.
    pub fn note_new(&mut self) {
        let t = &self.tabs[self.active];
        let Some(key) = page_key(&t.url).filter(|_| matches!(t.kind, TabKind::Web)) else {
            return self.toast_here("Open a web page to add a note to it");
        };
        if self.private_window || t.private {
            return self.toast_here("Notes are not saved in a private window");
        }
        let Some(s) = &self.storage else { return };
        if s.notes_for(&key).len() >= MAX_NOTES_PER_PAGE {
            return self.toast_here("This page already has the most notes it can hold");
        }
        let Ok(id) = s.note_add(&key, "") else { return };
        let notes = s.notes_for(&key);
        if let Some(wv) = &self.webview {
            let _ = wv.focus();
            let _ = wv.evaluate_script(&overlays::notes_overlay(self.core.theme, &notes, id));
        }
    }

    /// `ax: "note"` messages from the sticky notes on a page. The page is taken from the web view, and a note can
    /// only be changed from the page it belongs to.
    pub fn on_note_message(&mut self, tab_idx: usize, source: &str, msg: &Value) {
        let Some(key) = page_key(source) else { return };
        let id = msg.get("id").and_then(|v| v.as_i64()).unwrap_or(-1);
        let Some(s) = &self.storage else { return };
        if self.private_window || self.tabs[tab_idx].private || !s.notes_for(&key).iter().any(|(nid, _)| *nid == id) {
            return;
        }
        match msg.get("k").and_then(|v| v.as_str()) {
            Some("save") => {
                let text = clean_note(msg.get("t").and_then(|v| v.as_str()).unwrap_or(""));
                let _ = s.note_update(id, &text);
            }
            Some("del") => {
                let _ = s.note_delete(id);
            }
            _ => {}
        }
    }

    pub fn note_command(&mut self, action: &str, id: i64) {
        if action == "delete" {
            if let Some(s) = &self.storage {
                let _ = s.note_delete(id);
            }
        }
        self.reload_library_page();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_belong_to_the_page_without_its_fragment() {
        assert_eq!(page_key("https://a.test/p?x=1#top").as_deref(), Some("https://a.test/p?x=1"));
        assert_eq!(page_key("javascript:1"), None);
        assert_eq!(page_key("file:///a.html"), None);
        assert_eq!(page_key(&format!("https://a.test/{}", "x".repeat(3000))), None);
    }

    #[test]
    fn note_text_is_bounded_and_clean() {
        assert_eq!(clean_note("a\u{7}b\nc"), "ab\nc");
        assert_eq!(clean_note(&"x".repeat(5000)).chars().count(), MAX_NOTE_CHARS);
    }
}
