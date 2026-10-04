//! Download manager: live state of running downloads, pause / resume / cancel, open / show in folder, and the
//! optional "ask where to save" step.

use crate::app::App;
use crate::tabs::TabKind;
use crate::web::{self, DlEvent};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    No,
    Waiting,
    Chosen(PathBuf),
}

pub struct DlState {
    pub id: u64,
    pub db_id: Option<i64>,
    pub name: String,
    pub path: PathBuf,
    pub received: i64,
    pub total: i64,
    pub paused: bool,
    pub done: bool,
    pub ask: Ask,
}

/// Percent done, or `None` when the size is not known.
pub fn percent(received: i64, total: i64) -> Option<u32> {
    if total <= 0 {
        None
    } else {
        Some(((received.max(0) as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as u32)
    }
}

/// Move a finished download to where the user asked for it (rename, or copy across drives).
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(dir) = to.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

impl App {
    fn dl_mut(&mut self, id: u64) -> Option<&mut DlState> {
        self.dls.iter_mut().find(|d| d.id == id)
    }

    fn downloads_page_open(&self) -> bool {
        matches!(self.tabs[self.active].kind, TabKind::Page("downloads"))
    }

    pub fn on_download_event(&mut self, tab_idx: usize, ev: DlEvent) {
        match ev {
            DlEvent::Started { id, url, path, total } => {
                let name = file_name(&path);
                let private = self.private_window || self.tabs[tab_idx].private;
                let db_id = if private { None } else { self.storage.as_ref().and_then(|s| s.add_download(&url, &name, &path.to_string_lossy()).ok()) };
                let ask = if self.settings.ask_download { Ask::Waiting } else { Ask::No };
                if ask == Ask::Waiting {
                    let shared = self.tabs[tab_idx].shared.clone();
                    let (dir, suggested) = (self.hub.download_dir(), name.clone());
                    std::thread::spawn(move || {
                        let picked = rfd::FileDialog::new().set_title("Save file as").set_directory(dir).set_file_name(&suggested).save_file();
                        shared.push_event(web::WebEvent::PageData("download-target".into(), id.to_string(), picked.map(|p| p.to_string_lossy().to_string()).unwrap_or_default()));
                    });
                }
                self.dls.push(DlState { id, db_id, name: name.clone(), path, received: 0, total, paused: false, done: false, ask });
                if let Some(wv) = &self.webview {
                    let shared = self.shared();
                    self.core.toast(wv, &format!("\u{2B07}\u{FE0F} Downloading {}", name), Some(("Show downloads", &shared.token, "downloads")), &shared);
                }
                self.reload_downloads_page();
            }
            DlEvent::Progress { id, received, total } => {
                if let Some(d) = self.dl_mut(id) {
                    d.received = received;
                    if total > 0 {
                        d.total = total;
                    }
                }
            }
            DlEvent::Paused { id, paused } => {
                if let Some(d) = self.dl_mut(id) {
                    d.paused = paused;
                }
                self.reload_downloads_page();
            }
            DlEvent::Done { id } => self.download_finished(id, None),
            DlEvent::Failed { id, reason } => self.download_finished(id, Some(reason)),
        }
    }

    fn download_finished(&mut self, id: u64, failure: Option<String>) {
        let Some(pos) = self.dls.iter().position(|d| d.id == id) else { return };
        let (name, db_id, size) = {
            let d = &mut self.dls[pos];
            d.done = true;
            (d.name.clone(), d.db_id, d.received.max(d.total))
        };
        match &failure {
            None => {
                self.dls[pos].received = self.dls[pos].total.max(self.dls[pos].received);
                if let Ask::Chosen(target) = self.dls[pos].ask.clone() {
                    self.finalize_move(pos, &target);
                }
                if let (Some(s), Some(db)) = (&self.storage, db_id) {
                    let _ = s.update_download_status(db, "completed", size);
                }
            }
            Some(reason) => {
                let cancelled = reason == "canceled";
                let _ = std::fs::remove_file(&self.dls[pos].path);
                if let (Some(s), Some(db)) = (&self.storage, db_id) {
                    let _ = s.update_download_status(db, if cancelled { "cancelled" } else { "failed" }, 0);
                }
            }
        }
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            match failure.as_deref() {
                None => self.core.toast(wv, &format!("\u{2B07}\u{FE0F} Downloaded {}", name), Some(("Show downloads", &shared.token, "downloads")), &shared),
                Some("canceled") => {}
                Some(_) => self.core.toast(wv, &format!("Download failed: {}", name), None, &shared),
            }
        }
        // A finished entry no longer needs its live state once the page shows the stored row.
        self.reload_downloads_page();
        self.dls.retain(|d| !(d.done && d.id == id && d.ask != Ask::Waiting));
    }

    fn finalize_move(&mut self, pos: usize, target: &Path) {
        let from = self.dls[pos].path.clone();
        if from == target {
            return;
        }
        if move_file(&from, target).is_ok() {
            self.dls[pos].path = target.to_path_buf();
            self.dls[pos].name = file_name(target);
            if let (Some(s), Some(db)) = (&self.storage, self.dls[pos].db_id) {
                let _ = s.set_download_path(db, &target.to_string_lossy(), &file_name(target));
            }
        }
    }

    /// The "Save as" dialog came back: an empty path means the user cancelled.
    pub fn on_download_target(&mut self, id: u64, picked: &str) {
        let Some(pos) = self.dls.iter().position(|d| d.id == id) else { return };
        if picked.is_empty() {
            web::com::download_control(id, "cancel");
            return;
        }
        let target = PathBuf::from(picked);
        if self.dls[pos].done {
            self.finalize_move(pos, &target);
            if let (Some(s), Some(db)) = (&self.storage, self.dls[pos].db_id) {
                let _ = s.set_download_path(db, &target.to_string_lossy(), &file_name(&target));
            }
            self.dls.remove(pos);
            self.reload_downloads_page();
        } else {
            self.dls[pos].ask = Ask::Chosen(target);
        }
    }

    pub fn reload_downloads_page(&mut self) {
        if self.downloads_page_open() {
            self.load_active_page();
        }
    }

    /// Live numbers for the Downloads page, pushed a few times a second while something is downloading.
    pub fn push_download_progress(&mut self) {
        if self.dls.is_empty() || !self.downloads_page_open() || self.dl_push_at.elapsed() < Duration::from_millis(300) {
            return;
        }
        self.dl_push_at = Instant::now();
        let rows: Vec<serde_json::Value> = self
            .dls
            .iter()
            .filter_map(|d| d.db_id.map(|db| serde_json::json!([db, d.received, d.total, d.paused, d.done])))
            .collect();
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script(&format!("window.__dl&&__dl({})", serde_json::Value::Array(rows)));
        }
    }

    /// `(received, total, paused)` of the running download stored under this row id.
    pub fn live_downloads(&self) -> std::collections::HashMap<i64, (i64, i64, bool)> {
        self.dls.iter().filter(|d| !d.done).filter_map(|d| d.db_id.map(|db| (db, (d.received, d.total, d.paused)))).collect()
    }

    /// `dl-<action>/<row id>` from the Downloads page.
    pub fn download_command(&mut self, action: &str, db_id: i64) {
        match action {
            "pause" | "resume" | "cancel" => {
                if let Some(id) = self.dls.iter().find(|d| d.db_id == Some(db_id) && !d.done).map(|d| d.id) {
                    web::com::download_control(id, action);
                }
            }
            "open" | "show" => {
                let Some(row) = self.storage.as_ref().and_then(|s| s.get_download(db_id)) else { return };
                let path = PathBuf::from(&row.filepath);
                if row.status != "completed" || !path.is_file() {
                    if let Some(wv) = &self.webview {
                        let shared = self.shared();
                        self.core.toast(wv, "That file is no longer in its folder", None, &shared);
                    }
                    return;
                }
                #[cfg(target_os = "windows")]
                {
                    let mut cmd = std::process::Command::new("explorer");
                    if action == "show" {
                        cmd.arg(format!("/select,{}", path.display()));
                    } else {
                        cmd.arg(&path);
                    }
                    let _ = cmd.spawn();
                }
            }
            "remove" => {
                if let Some(s) = &self.storage {
                    let _ = s.delete_download(db_id);
                }
                self.reload_downloads_page();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_handles_unknown_sizes() {
        assert_eq!(percent(50, 200), Some(25));
        assert_eq!(percent(5, 0), None);
        assert_eq!(percent(500, 200), Some(100));
    }

    #[test]
    fn unique_names_get_a_counter() {
        let dir = std::env::temp_dir().join(format!("axomai_dl_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(web::unique_path(&dir, "a.txt"), dir.join("a.txt"));
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        assert_eq!(web::unique_path(&dir, "a.txt"), dir.join("a (1).txt"));
        std::fs::write(dir.join("a (1).txt"), b"x").unwrap();
        assert_eq!(web::unique_path(&dir, "a.txt"), dir.join("a (2).txt"));
        assert_eq!(web::unique_path(&dir, ""), dir.join("download"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moving_a_file_replaces_the_source() {
        let dir = std::env::temp_dir().join(format!("axomai_mv_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.bin"), dir.join("sub").join("b.bin"));
        std::fs::write(&a, b"hello").unwrap();
        move_file(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read(&b).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
