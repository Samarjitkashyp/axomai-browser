//! Bookmark manager actions: move, rename, import (Chrome or file) and export.

use crate::app::App;
use crate::bookmarks_io as io;
use crate::tabs::TabKind;
use crate::web;

impl App {
    fn toast_active(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    /// Something about the saved bookmarks changed: refresh the bar, the star and the manager page.
    pub fn bookmarks_changed(&mut self) {
        self.refresh_bar();
        if matches!(self.tabs[self.active].kind, TabKind::Page("bookmarks")) {
            self.load_active_page();
        }
        self.redraw = true;
    }

    pub fn bookmark_move(&mut self, id: i64, folder: &str) {
        let folder = folder.trim();
        let folder: String = folder.chars().take(60).collect();
        if folder.is_empty() || id < 0 {
            return;
        }
        if let Some(s) = &self.storage {
            let title = s.get_bookmarks().ok().and_then(|all| all.into_iter().find(|b| b.id == id)).map(|b| b.title).unwrap_or_default();
            let _ = s.update_bookmark(id, &title, &folder);
        }
        self.bookmarks_changed();
    }

    pub fn bookmark_rename(&mut self, id: i64, title: &str) {
        let title: String = title.trim().chars().take(200).collect();
        if title.is_empty() || id < 0 {
            return;
        }
        if let Some(s) = &self.storage {
            if let Some(b) = s.get_bookmarks().ok().and_then(|all| all.into_iter().find(|b| b.id == id)) {
                let _ = s.update_bookmark(id, &title, &b.folder);
            }
        }
        self.bookmarks_changed();
    }

    fn import_items(&mut self, items: Vec<io::Imported>, source: &str) {
        if self.private_window {
            self.toast_active("Bookmarks are not saved in a private window");
            return;
        }
        let total = items.len();
        let added = self.storage.as_ref().map(|s| s.import_bookmarks(&items)).unwrap_or(0);
        self.bookmarks_changed();
        self.toast_active(&if total == 0 {
            format!("No bookmarks found in {}", source)
        } else {
            format!("Imported {} bookmarks from {} ({} already saved)", added, source, total - added)
        });
    }

    pub fn import_chrome_bookmarks(&mut self) {
        let Some(path) = io::chrome_bookmarks_path() else {
            self.toast_active("Chrome's bookmarks were not found on this computer");
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => self.import_items(io::parse_chrome_json(&text), "Chrome"),
            Err(_) => self.toast_active("Could not read Chrome's bookmarks (close Chrome and try again)"),
        }
    }

    /// File picker on a helper thread (the native dialog runs its own message loop); the answer comes back as a page-data event.
    pub fn pick_bookmark_file(&mut self) {
        let shared = self.shared();
        std::thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new().set_title("Import bookmarks").add_filter("Bookmarks", &["html", "htm", "json"]).pick_file() {
                shared.push_event(web::WebEvent::PageData("bookmark-file".into(), String::new(), p.to_string_lossy().to_string()));
            }
        });
    }

    pub fn import_bookmark_file(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else {
            self.toast_active("Could not read that file");
            return;
        };
        let items = if text.trim_start().starts_with('{') { io::parse_chrome_json(&text) } else { io::parse_netscape_html(&text) };
        self.import_items(items, "the file");
    }

    pub fn export_bookmarks(&mut self) {
        let all = self.storage.as_ref().and_then(|s| s.get_bookmarks().ok()).unwrap_or_default();
        if all.is_empty() {
            self.toast_active("There are no bookmarks to export");
            return;
        }
        let path = web::unique_path(&self.hub.download_dir(), "axomai-bookmarks.html");
        let _ = std::fs::create_dir_all(self.hub.download_dir());
        match std::fs::write(&path, io::export_netscape(&all)) {
            Ok(_) => self.toast_active(&format!("Exported {} bookmarks to {}", all.len(), path.display())),
            Err(_) => self.toast_active("Could not write the export file"),
        }
    }
}
