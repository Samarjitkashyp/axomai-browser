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
pub struct SavedTab {
    pub position: i32,
    pub url: String,
    pub title: String,
    pub is_active: bool,
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
            "SELECT id, url, title, visit_time FROM history ORDER BY visit_time DESC LIMIT ?1"
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
            "SELECT id, url, title, visit_time FROM history
             WHERE url LIKE ?1 OR title LIKE ?1
             ORDER BY visit_time DESC LIMIT ?2"
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
            "SELECT id, url, title, folder, created_at FROM bookmarks ORDER BY created_at DESC"
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
            "INSERT INTO tabs (position, url, title, is_active) VALUES (?1, ?2, ?3, ?4)"
        )?;
        for tab in tabs {
            stmt.execute(params![tab.position, tab.url, tab.title, tab.is_active as i32])?;
        }
        Ok(())
    }

    pub fn get_tabs(&self) -> Result<Vec<SavedTab>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT position, url, title, is_active FROM tabs ORDER BY position ASC"
        )?;
        let entries = stmt.query_map([], |row| {
            Ok(SavedTab {
                position: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                is_active: row.get::<_, i32>(3)? != 0,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
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
    fn bookmarks_round_trip() {
        let st = temp_storage();
        st.add_bookmark("https://a.test", "A", "Unsorted").unwrap();
        assert!(st.is_bookmarked("https://a.test").unwrap());
        st.remove_bookmark_by_url("https://a.test").unwrap();
        assert!(!st.is_bookmarked("https://a.test").unwrap());
    }
}
