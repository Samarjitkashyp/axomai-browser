use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visit_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub folder: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub id: i64,
    pub url: String,
    pub filename: String,
    pub filepath: String,
    pub size_bytes: i64,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Clone)]
pub struct ReadingItem {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub added_at: String,
    pub read: bool,
}

#[derive(Debug, Clone)]
pub struct SavedSession {
    pub id: i64,
    pub name: String,
    pub urls: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub page: String,
    pub text: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SavedTab {
    pub position: i32,
    pub url: String,
    pub title: String,
    pub is_active: bool,
    /// `"<colour>|<name>"` of the tab's group, or empty.
    pub group: String,
}

pub struct BrowserStorage {
    conn: Connection,
}

impl BrowserStorage {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_dir = dirs_db();
        let _ = std::fs::create_dir_all(&db_dir);
        let db_path = std::path::Path::new(&db_dir).join("axomai_browser.db");
        let conn = Connection::open(db_path)?;
        let storage = BrowserStorage { conn };
        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<(), rusqlite::Error> {
        self.create_tables()?;
        // Added later: the tab group of each saved tab (older databases get the column here).
        let _ = self.conn.execute("ALTER TABLE tabs ADD COLUMN grp TEXT NOT NULL DEFAULT ''", []);
        Ok(())
    }

    fn create_tables(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                visit_time TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                folder TEXT NOT NULL DEFAULT 'Unsorted',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                filename TEXT NOT NULL DEFAULT '',
                filepath TEXT NOT NULL DEFAULT '',
                size_bytes INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                started_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS passwords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                origin TEXT NOT NULL,
                username TEXT NOT NULL DEFAULT '',
                secret BLOB NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(origin, username)
            );
            CREATE TABLE IF NOT EXISTS pw_never (origin TEXT PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                urls TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS site_rules (
                host TEXT PRIMARY KEY,
                muted INTEGER NOT NULL DEFAULT 0,
                blocked INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS reading_list (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL DEFAULT '',
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                read INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page TEXT NOT NULL,
                text TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_notes_page ON notes(page);
            CREATE TABLE IF NOT EXISTS favicons (host TEXT PRIMARY KEY, icon BLOB NOT NULL, updated_at TEXT NOT NULL DEFAULT (datetime('now')));
            CREATE TABLE IF NOT EXISTS site_zoom (origin TEXT PRIMARY KEY, factor REAL NOT NULL);
            CREATE TABLE IF NOT EXISTS site_permissions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                origin TEXT NOT NULL,
                kind TEXT NOT NULL,
                allow INTEGER NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(origin, kind)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tabs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                position INTEGER NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                is_active INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_history_time ON history(visit_time DESC);
            CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
            CREATE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);"
        )
    }

    pub fn add_history(&self, url: &str, title: &str) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO history (url, title) VALUES (?1, ?2)",
            params![url, title],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_history(&self, limit: usize) -> Result<Vec<HistoryEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, datetime(visit_time, 'localtime') FROM history ORDER BY id DESC LIMIT ?1"
        )?;
        let entries = stmt.query_map(params![limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visit_time: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn search_history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, rusqlite::Error> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, datetime(visit_time, 'localtime') FROM history
             WHERE url LIKE ?1 OR title LIKE ?1
             ORDER BY id DESC LIMIT ?2"
        )?;
        let entries = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visit_time: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn clear_history(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn add_bookmark(&self, url: &str, title: &str, folder: &str) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO bookmarks (url, title, folder) VALUES (?1, ?2, ?3)",
            params![url, title, folder],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_bookmarks(&self) -> Result<Vec<Bookmark>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, folder, created_at FROM bookmarks ORDER BY id DESC"
        )?;
        let entries = stmt.query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                folder: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn update_bookmark(&self, id: i64, title: &str, folder: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE bookmarks SET title = ?1, folder = ?2 WHERE id = ?3", params![title, folder, id])?;
        Ok(())
    }

    /// Delete a folder: its bookmarks move to `fallback`.
    pub fn delete_folder(&self, folder: &str, fallback: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE bookmarks SET folder = ?2 WHERE folder = ?1", params![folder, fallback])?;
        Ok(())
    }

    /// Add imported bookmarks, skipping addresses that are already saved. Returns how many were new.
    pub fn import_bookmarks(&self, items: &[(String, String, String)]) -> usize {
        let mut added = 0;
        for (folder, title, url) in items {
            if self.is_bookmarked(url).unwrap_or(true) {
                continue;
            }
            if self.add_bookmark(url, title, folder).is_ok() {
                added += 1;
            }
        }
        added
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn is_bookmarked(&self, url: &str) -> Result<bool, rusqlite::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Attach a page title to the most recent visit of `url`.
    pub fn update_history_title(&self, url: &str, title: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE history SET title = ?2 WHERE id = (SELECT MAX(id) FROM history WHERE url = ?1)",
            params![url, title],
        )?;
        Ok(())
    }

    pub fn delete_history(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete history from the last `seconds` seconds (`None` = everything).
    pub fn clear_history_since(&self, seconds: Option<i64>) -> Result<(), rusqlite::Error> {
        match seconds {
            None => self.conn.execute("DELETE FROM history", [])?,
            Some(s) => self.conn.execute("DELETE FROM history WHERE visit_time >= datetime('now', ?1)", params![format!("-{} seconds", s.max(0))])?,
        };
        Ok(())
    }

    /// Delete download-list entries from the last `seconds` seconds (`None` = everything).
    pub fn clear_downloads_since(&self, seconds: Option<i64>) -> Result<(), rusqlite::Error> {
        match seconds {
            None => self.conn.execute("DELETE FROM downloads", [])?,
            Some(s) => self.conn.execute("DELETE FROM downloads WHERE started_at >= datetime('now', ?1)", params![format!("-{} seconds", s.max(0))])?,
        };
        Ok(())
    }

    pub fn remove_bookmark_by_url(&self, url: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM bookmarks WHERE url = ?1", params![url])?;
        Ok(())
    }

    /// Row count of one of the browsing-data tables (`history`, `bookmarks`, `downloads`).
    pub fn count(&self, table: &str) -> usize {
        let sql = match table {
            "history" => "SELECT COUNT(*) FROM history",
            "bookmarks" => "SELECT COUNT(*) FROM bookmarks",
            "downloads" => "SELECT COUNT(*) FROM downloads",
            _ => return 0,
        };
        self.conn.query_row(sql, [], |r| r.get::<_, i64>(0)).map(|n| n as usize).unwrap_or(0)
    }

    pub fn add_download(&self, url: &str, filename: &str, filepath: &str) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO downloads (url, filename, filepath, status) VALUES (?1, ?2, ?3, 'downloading')",
            params![url, filename, filepath],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_download_status(&self, id: i64, status: &str, size_bytes: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE downloads SET status = ?1, size_bytes = ?2 WHERE id = ?3",
            params![status, size_bytes, id],
        )?;
        Ok(())
    }

    pub fn get_downloads(&self, limit: usize) -> Result<Vec<DownloadEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, filename, filepath, size_bytes, status, started_at
             FROM downloads ORDER BY started_at DESC LIMIT ?1"
        )?;
        let entries = stmt.query_map(params![limit as i64], |row| {
            Ok(DownloadEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                filename: row.get(2)?,
                filepath: row.get(3)?,
                size_bytes: row.get(4)?,
                status: row.get(5)?,
                started_at: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Insert or replace the saved login for (origin, username). `secret` is already encrypted.
    pub fn save_password(&self, origin: &str, username: &str, secret: &[u8]) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO passwords (origin, username, secret) VALUES (?1, ?2, ?3)
             ON CONFLICT(origin, username) DO UPDATE SET secret = excluded.secret, updated_at = datetime('now')",
            params![origin, username, secret],
        )?;
        self.conn.query_row("SELECT id FROM passwords WHERE origin = ?1 AND username = ?2", params![origin, username], |r| r.get(0))
    }

    /// `(id, username)` of the logins saved for exactly this origin.
    pub fn credentials_for(&self, origin: &str) -> Vec<(i64, String)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, username FROM passwords WHERE origin = ?1 ORDER BY updated_at DESC, id DESC") else { return Vec::new() };
        stmt.query_map(params![origin], |r| Ok((r.get(0)?, r.get(1)?))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn password_secret(&self, id: i64) -> Option<Vec<u8>> {
        self.conn.query_row("SELECT secret FROM passwords WHERE id = ?1", params![id], |r| r.get(0)).ok()
    }

    /// `(origin, username)` of a saved login.
    pub fn password_row(&self, id: i64) -> Option<(String, String)> {
        self.conn.query_row("SELECT origin, username FROM passwords WHERE id = ?1", params![id], |r| Ok((r.get(0)?, r.get(1)?))).ok()
    }

    /// `(id, origin, username, updated)` of every saved login, grouped by site.
    pub fn list_passwords(&self) -> Vec<(i64, String, String, String)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, origin, username, datetime(updated_at, 'localtime') FROM passwords ORDER BY origin, username") else { return Vec::new() };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn delete_password(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM passwords WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_favicon(&self, host: &str) -> Option<Vec<u8>> {
        self.conn.query_row("SELECT icon FROM favicons WHERE host = ?1", params![host], |r| r.get(0)).ok()
    }

    pub fn set_favicon(&self, host: &str, icon: &[u8]) -> Result<(), rusqlite::Error> {
        self.conn.execute("INSERT OR REPLACE INTO favicons (host, icon) VALUES (?1, ?2)", params![host, icon])?;
        Ok(())
    }

    /// Add a page to the reading list. `Ok(false)` when it was already there.
    pub fn reading_add(&self, url: &str, title: &str) -> Result<bool, rusqlite::Error> {
        Ok(self.conn.execute("INSERT OR IGNORE INTO reading_list (url, title) VALUES (?1, ?2)", params![url, title])? > 0)
    }

    /// Unread items first, then newest first.
    pub fn reading_items(&self) -> Vec<ReadingItem> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, url, title, datetime(added_at, 'localtime'), read FROM reading_list ORDER BY read ASC, id DESC") else { return Vec::new() };
        stmt.query_map([], |r| Ok(ReadingItem { id: r.get(0)?, url: r.get(1)?, title: r.get(2)?, added_at: r.get(3)?, read: r.get::<_, i64>(4)? != 0 }))
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    }

    pub fn reading_set_read(&self, id: i64, read: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE reading_list SET read = ?1 WHERE id = ?2", params![read as i64, id])?;
        Ok(())
    }

    pub fn reading_delete(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM reading_list WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn reading_clear_read(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM reading_list WHERE read = 1", [])?;
        Ok(())
    }

    /// Change the mute and/or block flag of a site (`None` keeps the current value). Rows with both off are removed.
    pub fn site_rule_set(&self, host: &str, muted: Option<bool>, blocked: Option<bool>) -> Result<(), rusqlite::Error> {
        self.conn.execute("INSERT OR IGNORE INTO site_rules (host) VALUES (?1)", params![host])?;
        if let Some(m) = muted {
            self.conn.execute("UPDATE site_rules SET muted = ?1 WHERE host = ?2", params![m as i64, host])?;
        }
        if let Some(b) = blocked {
            self.conn.execute("UPDATE site_rules SET blocked = ?1 WHERE host = ?2", params![b as i64, host])?;
        }
        self.conn.execute("DELETE FROM site_rules WHERE muted = 0 AND blocked = 0", [])?;
        Ok(())
    }

    /// `(host, muted, blocked)` for every site with a rule.
    pub fn site_rules(&self) -> Vec<(String, bool, bool)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT host, muted, blocked FROM site_rules ORDER BY host") else { return Vec::new() };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get::<_, i64>(2)? != 0))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// Save (or replace) a named set of addresses.
    pub fn session_save(&self, name: &str, urls: &[String]) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO sessions (name, urls) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET urls = excluded.urls, created_at = datetime('now')",
            params![name, urls.join("\n")],
        )?;
        Ok(())
    }

    fn session_from_row(r: &rusqlite::Row) -> rusqlite::Result<SavedSession> {
        let urls: String = r.get(2)?;
        Ok(SavedSession { id: r.get(0)?, name: r.get(1)?, urls: urls.lines().filter(|l| !l.is_empty()).map(String::from).collect(), created_at: r.get(3)? })
    }

    /// Newest first.
    pub fn sessions(&self) -> Vec<SavedSession> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, name, urls, datetime(created_at, 'localtime') FROM sessions ORDER BY created_at DESC, id DESC") else { return Vec::new() };
        stmt.query_map([], Self::session_from_row).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn session_get(&self, id: i64) -> Option<SavedSession> {
        self.conn.query_row("SELECT id, name, urls, datetime(created_at, 'localtime') FROM sessions WHERE id = ?1", params![id], Self::session_from_row).ok()
    }

    pub fn session_delete(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Every saved setting as `(key, value)`.
    pub fn all_settings(&self) -> Vec<(String, String)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT key, value FROM settings ORDER BY key") else { return Vec::new() };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// A history row with its original visit time (used when restoring a backup). A bad time becomes "now".
    pub fn import_history(&self, url: &str, title: &str, time: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO history (url, title, visit_time) VALUES (?1, ?2, COALESCE(datetime(?3, 'utc'), datetime('now')))",
            params![url, title, time],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        let st = BrowserStorage { conn: Connection::open_in_memory().unwrap() };
        st.init_tables().unwrap();
        st
    }

    /// `(origin, username, encrypted secret, updated_at in UTC)` of every saved login.
    pub fn password_rows_full(&self) -> Vec<(String, String, Vec<u8>, String)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT origin, username, secret, updated_at FROM passwords ORDER BY origin, username") else { return Vec::new() };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// A login with a given update time (used when merging from another computer).
    pub fn save_password_at(&self, origin: &str, username: &str, secret: &[u8], updated: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO passwords (origin, username, secret, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(origin, username) DO UPDATE SET secret = excluded.secret, updated_at = excluded.updated_at",
            params![origin, username, secret, updated],
        )?;
        Ok(())
    }

    pub fn note_add(&self, page: &str, text: &str) -> Result<i64, rusqlite::Error> {
        self.conn.execute("INSERT INTO notes (page, text) VALUES (?1, ?2)", params![page, text])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// `(id, text)` of the notes of one page, oldest first.
    pub fn notes_for(&self, page: &str) -> Vec<(i64, String)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, text FROM notes WHERE page = ?1 ORDER BY id ASC") else { return Vec::new() };
        stmt.query_map(params![page], |r| Ok((r.get(0)?, r.get(1)?))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn note_update(&self, id: i64, text: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE notes SET text = ?1, updated_at = datetime('now') WHERE id = ?2", params![text, id])?;
        Ok(())
    }

    pub fn note_delete(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Every non-empty note, newest first.
    pub fn notes_all(&self) -> Vec<Note> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, page, text, datetime(updated_at, 'localtime') FROM notes WHERE trim(text) <> '' ORDER BY updated_at DESC, id DESC") else { return Vec::new() };
        stmt.query_map([], |r| Ok(Note { id: r.get(0)?, page: r.get(1)?, text: r.get(2)?, updated_at: r.get(3)? })).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn get_site_zoom(&self, origin: &str) -> Option<f64> {
        self.conn.query_row("SELECT factor FROM site_zoom WHERE origin = ?1", params![origin], |r| r.get(0)).ok()
    }

    /// Remember a site's zoom; 100% means "default", so that row is simply removed.
    pub fn set_site_zoom(&self, origin: &str, factor: f64) -> Result<(), rusqlite::Error> {
        if (factor - 1.0).abs() < 0.001 {
            self.conn.execute("DELETE FROM site_zoom WHERE origin = ?1", params![origin])?;
        } else {
            self.conn.execute("INSERT OR REPLACE INTO site_zoom (origin, factor) VALUES (?1, ?2)", params![origin, factor])?;
        }
        Ok(())
    }

    pub fn get_permission(&self, origin: &str, kind: &str) -> Option<bool> {
        self.conn.query_row("SELECT allow FROM site_permissions WHERE origin = ?1 AND kind = ?2", params![origin, kind], |r| r.get::<_, i64>(0)).ok().map(|v| v != 0)
    }

    pub fn set_permission(&self, origin: &str, kind: &str, allow: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO site_permissions (origin, kind, allow) VALUES (?1, ?2, ?3)
             ON CONFLICT(origin, kind) DO UPDATE SET allow = excluded.allow, updated_at = datetime('now')",
            params![origin, kind, allow as i64],
        )?;
        Ok(())
    }

    pub fn set_permission_by_id(&self, id: i64, allow: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE site_permissions SET allow = ?1, updated_at = datetime('now') WHERE id = ?2", params![allow as i64, id])?;
        Ok(())
    }

    /// `(id, origin, kind, allow)` for every remembered decision.
    pub fn list_permissions(&self) -> Vec<(i64, String, String, bool)> {
        let Ok(mut stmt) = self.conn.prepare("SELECT id, origin, kind, allow FROM site_permissions ORDER BY origin, kind") else { return Vec::new() };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? != 0))).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn delete_permission(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM site_permissions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_permissions(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM site_permissions", [])?;
        Ok(())
    }

    pub fn pw_never_has(&self, origin: &str) -> bool {
        self.conn.query_row("SELECT 1 FROM pw_never WHERE origin = ?1", params![origin], |_| Ok(())).is_ok()
    }

    pub fn pw_never_add(&self, origin: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("INSERT OR IGNORE INTO pw_never (origin) VALUES (?1)", params![origin])?;
        Ok(())
    }

    pub fn pw_never_remove(&self, origin: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM pw_never WHERE origin = ?1", params![origin])?;
        Ok(())
    }

    pub fn pw_never_list(&self) -> Vec<String> {
        let Ok(mut stmt) = self.conn.prepare("SELECT origin FROM pw_never ORDER BY origin") else { return Vec::new() };
        stmt.query_map([], |r| r.get(0)).map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    pub fn get_download(&self, id: i64) -> Option<DownloadEntry> {
        self.conn
            .query_row("SELECT id, url, filename, filepath, size_bytes, status, started_at FROM downloads WHERE id = ?1", params![id], |row| {
                Ok(DownloadEntry { id: row.get(0)?, url: row.get(1)?, filename: row.get(2)?, filepath: row.get(3)?, size_bytes: row.get(4)?, status: row.get(5)?, started_at: row.get(6)? })
            })
            .ok()
    }

    pub fn delete_download(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_download_path(&self, id: i64, filepath: &str, filename: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE downloads SET filepath = ?1, filename = ?2 WHERE id = ?3", params![filepath, filename, id])?;
        Ok(())
    }

    /// Downloads that were still running when the browser last closed can never finish.
    pub fn fail_stale_downloads(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE downloads SET status = 'failed' WHERE status = 'downloading'", [])?;
        Ok(())
    }

    pub fn clear_downloads(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM downloads", [])?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let result = self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn save_tabs(&self, tabs: &[SavedTab]) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM tabs", [])?;
        let mut stmt = self.conn.prepare(
            "INSERT INTO tabs (position, url, title, is_active, grp) VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;
        for tab in tabs {
            stmt.execute(params![tab.position, tab.url, tab.title, tab.is_active as i32, tab.group])?;
        }
        Ok(())
    }

    pub fn get_tabs(&self) -> Result<Vec<SavedTab>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT position, url, title, is_active, grp FROM tabs ORDER BY position ASC"
        )?;
        let entries = stmt.query_map([], |row| {
            Ok(SavedTab {
                position: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                is_active: row.get::<_, i32>(3)? != 0,
                group: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

/// The folder that holds the database and the other per-user files.
pub fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(dirs_db())
}

fn dirs_db() -> String {
    if let Some(data_dir) = std::env::var_os("APPDATA") {
        let p = std::path::Path::new(&data_dir).join("AxomaiBrowser");
        return p.to_string_lossy().to_string();
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = std::path::Path::new(&home).join(".axomai-browser");
        return p.to_string_lossy().to_string();
    }
    ".axomai-browser".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage() -> BrowserStorage {
        let conn = Connection::open_in_memory().unwrap();
        let st = BrowserStorage { conn };
        st.init_tables().unwrap();
        st
    }

    #[test]
    fn history_delete_and_range_clear() {
        let st = temp_storage();
        let a = st.add_history("https://a.test", "A").unwrap();
        st.add_history("https://b.test", "B").unwrap();
        st.conn.execute("UPDATE history SET visit_time = datetime('now','-3 days') WHERE url='https://a.test'", []).unwrap();
        st.clear_history_since(Some(3600)).unwrap(); // last hour: removes only B
        let left = st.get_history(10).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, a);
        st.delete_history(a).unwrap();
        assert!(st.get_history(10).unwrap().is_empty());
    }

    #[test]
    fn clear_everything() {
        let st = temp_storage();
        st.add_history("https://a.test", "A").unwrap();
        st.add_download("https://a.test/f", "f", "C:/f").unwrap();
        st.clear_history_since(None).unwrap();
        st.clear_downloads_since(None).unwrap();
        assert_eq!(st.count("history"), 0);
        assert_eq!(st.count("downloads"), 0);
    }

    #[test]
    fn bookmark_editing_and_import() {
        let st = temp_storage();
        let id = st.add_bookmark("https://a.test", "A", "Unsorted").unwrap();
        st.update_bookmark(id, "Alpha", "Work").unwrap();
        let b = st.get_bookmarks().unwrap();
        assert_eq!((b[0].title.as_str(), b[0].folder.as_str()), ("Alpha", "Work"));
        let items = vec![
            ("Imp".to_string(), "dup".to_string(), "https://a.test".to_string()),
            ("Imp".to_string(), "B".to_string(), "https://b.test".to_string()),
        ];
        assert_eq!(st.import_bookmarks(&items), 1, "existing addresses are skipped");
        st.delete_folder("Imp", "Unsorted").unwrap();
        assert!(st.get_bookmarks().unwrap().iter().all(|b| b.folder != "Imp"));
    }

    #[test]
    fn passwords_are_per_origin_and_replaced_not_duplicated() {
        let st = temp_storage();
        let a = st.save_password("https://a.test", "me", b"one").unwrap();
        let again = st.save_password("https://a.test", "me", b"two").unwrap();
        assert_eq!(a, again, "same login keeps its row");
        st.save_password("https://b.test", "me", b"x").unwrap();
        assert_eq!(st.credentials_for("https://a.test").len(), 1);
        assert_eq!(st.credentials_for("https://a.test:8443").len(), 0, "a different port is a different site");
        assert_eq!(st.password_secret(a).unwrap(), b"two");
        st.delete_password(a).unwrap();
        assert!(st.password_secret(a).is_none());
        assert_eq!(st.list_passwords().len(), 1);
    }

    #[test]
    fn permissions_are_remembered_per_site_and_kind() {
        let st = temp_storage();
        assert_eq!(st.get_permission("https://a.test", "camera"), None);
        st.set_permission("https://a.test", "camera", true).unwrap();
        st.set_permission("https://a.test", "microphone", false).unwrap();
        assert_eq!(st.get_permission("https://a.test", "camera"), Some(true));
        assert_eq!(st.get_permission("https://a.test", "microphone"), Some(false));
        assert_eq!(st.get_permission("https://b.test", "camera"), None);
        st.set_permission("https://a.test", "camera", false).unwrap();
        assert_eq!(st.get_permission("https://a.test", "camera"), Some(false));
        let all = st.list_permissions();
        assert_eq!(all.len(), 2);
        st.set_permission_by_id(all[0].0, true).unwrap();
        st.delete_permission(all[1].0).unwrap();
        assert_eq!(st.list_permissions().len(), 1);
        st.clear_permissions().unwrap();
        assert!(st.list_permissions().is_empty());
    }

    #[test]
    fn site_zoom_is_remembered_and_default_is_forgotten() {
        let st = temp_storage();
        assert_eq!(st.get_site_zoom("https://a.test"), None);
        st.set_site_zoom("https://a.test", 1.5).unwrap();
        assert_eq!(st.get_site_zoom("https://a.test"), Some(1.5));
        st.set_site_zoom("https://a.test", 1.0).unwrap();
        assert_eq!(st.get_site_zoom("https://a.test"), None);
    }

    #[test]
    fn favicons_are_kept_per_host() {
        let st = temp_storage();
        assert_eq!(st.get_favicon("a.test"), None);
        st.set_favicon("a.test", b"icon-1").unwrap();
        st.set_favicon("a.test", b"icon-2").unwrap();
        assert_eq!(st.get_favicon("a.test").unwrap(), b"icon-2");
        assert_eq!(st.get_favicon("b.test"), None);
    }

    #[test]
    fn saved_tabs_keep_their_group() {
        let st = temp_storage();
        let tabs = vec![
            SavedTab { position: 0, url: "https://a.test".into(), title: "A".into(), is_active: false, group: "2|Work".into() },
            SavedTab { position: 1, url: "https://b.test".into(), title: "B".into(), is_active: true, group: String::new() },
        ];
        st.save_tabs(&tabs).unwrap();
        let back = st.get_tabs().unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].group, "2|Work");
        assert_eq!(back[1].group, "");
    }

    #[test]
    fn sessions_are_saved_by_name_and_replaced_on_reuse() {
        let st = temp_storage();
        st.session_save("Work", &["https://a.test".to_string(), "https://b.test".to_string()]).unwrap();
        st.session_save("Work", &["https://c.test".to_string()]).unwrap();
        st.session_save("Fun", &["https://d.test".to_string()]).unwrap();
        let all = st.sessions();
        assert_eq!(all.len(), 2);
        let work = all.iter().find(|s| s.name == "Work").unwrap();
        assert_eq!(work.urls, vec!["https://c.test".to_string()]);
        assert_eq!(st.session_get(work.id).unwrap().name, "Work");
        st.session_delete(work.id).unwrap();
        assert!(st.session_get(work.id).is_none());
        assert_eq!(st.sessions().len(), 1);
    }

    #[test]
    fn site_rules_keep_each_flag_and_drop_empty_rows() {
        let st = temp_storage();
        st.site_rule_set("a.com", Some(true), None).unwrap();
        st.site_rule_set("a.com", None, Some(true)).unwrap();
        st.site_rule_set("b.com", None, Some(true)).unwrap();
        assert_eq!(st.site_rules(), vec![("a.com".to_string(), true, true), ("b.com".to_string(), false, true)]);
        st.site_rule_set("a.com", Some(false), None).unwrap();
        st.site_rule_set("b.com", None, Some(false)).unwrap();
        assert_eq!(st.site_rules(), vec![("a.com".to_string(), false, true)]);
        st.site_rule_set("a.com", None, Some(false)).unwrap();
        assert!(st.site_rules().is_empty());
    }

    #[test]
    fn reading_list_adds_once_and_orders_unread_first() {
        let st = temp_storage();
        assert!(st.reading_add("https://a.test/1", "One").unwrap());
        assert!(!st.reading_add("https://a.test/1", "One again").unwrap(), "no duplicates");
        st.reading_add("https://a.test/2", "Two").unwrap();
        let items = st.reading_items();
        assert_eq!(items[0].title, "Two", "newest unread first");
        st.reading_set_read(items[0].id, true).unwrap();
        let items = st.reading_items();
        assert_eq!(items[0].title, "One");
        assert!(items[1].read);
        st.reading_clear_read().unwrap();
        assert_eq!(st.reading_items().len(), 1);
        st.reading_delete(st.reading_items()[0].id).unwrap();
        assert!(st.reading_items().is_empty());
    }

    #[test]
    fn notes_are_per_page_and_empty_ones_are_not_listed() {
        let st = temp_storage();
        let a = st.note_add("https://a.test/p", "").unwrap();
        let b = st.note_add("https://a.test/p", "remember this").unwrap();
        st.note_add("https://b.test/", "other page").unwrap();
        assert_eq!(st.notes_for("https://a.test/p").len(), 2);
        assert_eq!(st.notes_for("https://c.test/").len(), 0);
        st.note_update(a, "typed later").unwrap();
        assert_eq!(st.notes_all().len(), 3);
        st.note_update(a, "   ").unwrap();
        assert_eq!(st.notes_all().len(), 2, "blank notes stay off the Notes page");
        st.note_delete(b).unwrap();
        assert_eq!(st.notes_for("https://a.test/p").len(), 1);
    }

    #[test]
    fn never_list_round_trips() {
        let st = temp_storage();
        assert!(!st.pw_never_has("https://a.test"));
        st.pw_never_add("https://a.test").unwrap();
        st.pw_never_add("https://a.test").unwrap();
        assert!(st.pw_never_has("https://a.test"));
        assert_eq!(st.pw_never_list(), vec!["https://a.test".to_string()]);
        st.pw_never_remove("https://a.test").unwrap();
        assert!(!st.pw_never_has("https://a.test"));
    }

    #[test]
    fn bookmarks_round_trip() {
        let st = temp_storage();
        st.add_bookmark("https://a.test", "A", "Unsorted").unwrap();
        assert!(st.is_bookmarked("https://a.test").unwrap());
        st.remove_bookmark_by_url("https://a.test").unwrap();
        assert!(!st.is_bookmarked("https://a.test").unwrap());
    }
}
