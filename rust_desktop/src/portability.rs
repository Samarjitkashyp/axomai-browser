//! Moving data in and out of the browser: a backup file (bookmarks, history, settings, reading list, notes, site rules,
//! sessions) and password import / export as CSV (the same columns Chrome uses).

use crate::app::App;
use crate::storage::BrowserStorage;
use serde_json::{json, Value};

// ---------------------------------------------------------------------- CSV

/// Parse CSV text: quoted fields, `""` for a quote, commas and line breaks inside quotes.
pub fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

pub fn csv_line(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|f| if f.contains(&[',', '"', '\n', '\r'][..]) { format!("\"{}\"", f.replace('"', "\"\"")) } else { f.to_string() })
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Login {
    pub origin: String,
    pub user: String,
    pub pass: String,
}

/// Logins from a CSV with a header row naming a url, a username and a password column (Chrome, Edge, Firefox, ...).
pub fn parse_logins(text: &str) -> Vec<Login> {
    let rows = csv_rows(text);
    let Some(header) = rows.first() else { return Vec::new() };
    let find = |names: &[&str]| header.iter().position(|h| names.contains(&h.trim().to_ascii_lowercase().as_str()));
    let (Some(url_i), Some(pass_i)) = (find(&["url", "origin", "website", "login_uri", "web site"]), find(&["password", "pass"])) else { return Vec::new() };
    let user_i = find(&["username", "login", "user", "login_username", "user name"]);
    rows.iter()
        .skip(1)
        .filter_map(|r| {
            let origin = crate::passwords::origin_of(r.get(url_i)?.trim())?;
            let pass = r.get(pass_i)?.clone();
            if pass.is_empty() || pass.len() > 1000 {
                return None;
            }
            let user = user_i.and_then(|i| r.get(i)).cloned().unwrap_or_default();
            (user.len() <= 500).then_some(Login { origin, user, pass })
        })
        .collect()
}

pub fn logins_csv(logins: &[Login]) -> String {
    let mut out = String::from("name,url,username,password,note\r\n");
    for l in logins {
        let host = crate::blocklist::host_of(&l.origin);
        out.push_str(&csv_line(&[&host, &l.origin, &l.user, &l.pass, ""]));
        out.push_str("\r\n");
    }
    out
}

// ---------------------------------------------------------------------- backup file

const FORMAT: u32 = 1;
const MAX_HISTORY: usize = 5000;

pub fn backup_json(st: &BrowserStorage) -> Value {
    json!({
        "app": "Axomai Browser",
        "format": FORMAT,
        "bookmarks": st.get_bookmarks().unwrap_or_default().iter().map(|b| json!({"url": b.url, "title": b.title, "folder": b.folder})).collect::<Vec<_>>(),
        "history": st.get_history(MAX_HISTORY).unwrap_or_default().iter().map(|h| json!({"url": h.url, "title": h.title, "time": h.visit_time})).collect::<Vec<_>>(),
        "settings": st.all_settings().into_iter().map(|(k, v)| (k, Value::String(v))).collect::<serde_json::Map<_, _>>(),
        "reading_list": st.reading_items().iter().map(|r| json!({"url": r.url, "title": r.title, "read": r.read})).collect::<Vec<_>>(),
        "notes": st.notes_all().iter().map(|n| json!({"page": n.page, "text": n.text})).collect::<Vec<_>>(),
        "site_rules": st.site_rules().iter().map(|(h, m, b)| json!({"host": h, "muted": m, "blocked": b})).collect::<Vec<_>>(),
        "sessions": st.sessions().iter().map(|s| json!({"name": s.name, "urls": s.urls})).collect::<Vec<_>>(),
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Restored {
    pub bookmarks: usize,
    pub history: usize,
    pub settings: usize,
    pub reading: usize,
    pub notes: usize,
    pub sites: usize,
    pub sessions: usize,
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

fn web_url(u: &str) -> bool {
    (u.starts_with("http://") || u.starts_with("https://")) && u.len() < 2048
}

/// Merge a backup into the data that is already there. Returns `None` when the file is not one of our backups.
pub fn restore_backup(st: &BrowserStorage, v: &Value) -> Option<Restored> {
    if v.get("app").and_then(|x| x.as_str()) != Some("Axomai Browser") || v.get("format").and_then(|x| x.as_u64()) != Some(FORMAT as u64) {
        return None;
    }
    let list = |k: &str| v.get(k).and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let mut r = Restored::default();
    for b in list("bookmarks") {
        let url = s(&b, "url");
        if web_url(url) && !st.is_bookmarked(url).unwrap_or(true) {
            let folder = if s(&b, "folder").is_empty() { "Unsorted" } else { s(&b, "folder") };
            if st.add_bookmark(url, s(&b, "title"), folder).is_ok() {
                r.bookmarks += 1;
            }
        }
    }
    for h in list("history") {
        if web_url(s(&h, "url")) && st.import_history(s(&h, "url"), s(&h, "title"), s(&h, "time")).is_ok() {
            r.history += 1;
        }
    }
    if let Some(map) = v.get("settings").and_then(|x| x.as_object()) {
        for (k, val) in map {
            let Some(val) = val.as_str() else { continue };
            if k.len() <= 40 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && val.len() <= 8192 && st.set_setting(k, val).is_ok() {
                r.settings += 1;
            }
        }
    }
    for i in list("reading_list") {
        if web_url(s(&i, "url")) && st.reading_add(s(&i, "url"), s(&i, "title")).unwrap_or(false) {
            r.reading += 1;
        }
    }
    for n in list("notes") {
        let page = s(&n, "page");
        let text = crate::app_notes::clean_note(s(&n, "text"));
        if web_url(page) && !text.trim().is_empty() && !st.notes_for(page).iter().any(|(_, t)| *t == text) && st.note_add(page, &text).is_ok() {
            r.notes += 1;
        }
    }
    for rule in list("site_rules") {
        if let Some(host) = crate::siterules::normalize_host(s(&rule, "host")) {
            let muted = rule.get("muted").and_then(|x| x.as_bool()).unwrap_or(false);
            let blocked = rule.get("blocked").and_then(|x| x.as_bool()).unwrap_or(false);
            if (muted || blocked) && st.site_rule_set(&host, Some(muted), Some(blocked)).is_ok() {
                r.sites += 1;
            }
        }
    }
    for sess in list("sessions") {
        let name = crate::sessions::clean_session_name(s(&sess, "name"));
        let urls: Vec<String> = sess.get("urls").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|u| u.as_str()).filter(|u| crate::sessions::keepable(u)).map(String::from).collect()).unwrap_or_default();
        if !name.is_empty() && !urls.is_empty() && st.session_save(&name, &urls).is_ok() {
            r.sessions += 1;
        }
    }
    Some(r)
}

// ---------------------------------------------------------------------- app side

impl App {
    fn toast_port(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    fn pick_file_async(&self, title: &'static str, kind: &'static str, exts: &'static [&'static str]) {
        let shared = self.shared();
        std::thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new().set_title(title).add_filter("File", exts).pick_file() {
                shared.push_event(crate::web::WebEvent::PageData(kind.into(), String::new(), p.to_string_lossy().to_string()));
            }
        });
    }

    pub fn pick_backup_file(&self) {
        self.pick_file_async("Restore a backup", "backup-file", &["json"]);
    }

    pub fn pick_passwords_file(&self) {
        self.pick_file_async("Import passwords", "passwords-file", &["csv"]);
    }

    pub fn backup_export(&mut self) {
        let Some(st) = &self.storage else { return };
        let text = serde_json::to_string_pretty(&backup_json(st)).unwrap_or_default();
        let dir = self.hub.download_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = crate::web::unique_path(&dir, "axomai-backup.json");
        match std::fs::write(&path, text) {
            Ok(_) => self.toast_port(&format!("Backup saved to {}", path.display())),
            Err(_) => self.toast_port("Could not write the backup file"),
        }
    }

    pub fn backup_import(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else { return self.toast_port("Could not read that file") };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { return self.toast_port("That is not an Axomai backup") };
        let Some(st) = &self.storage else { return };
        match restore_backup(st, &v) {
            Some(r) => {
                self.settings = crate::settings::Settings::load(self.storage.as_ref());
                self.core.set_tweaks(self.settings.tweaks());
                self.load_site_rules();
                self.refresh_bar();
                self.refresh_internal_page();
                self.toast_port(&format!(
                    "Restored {} bookmarks, {} history entries, {} reading-list items, {} notes, {} sessions. Restart to apply every setting.",
                    r.bookmarks, r.history, r.reading, r.notes, r.sessions
                ));
            }
            None => self.toast_port("That is not an Axomai backup"),
        }
    }

    pub fn passwords_export(&mut self) {
        if !self.hello_guard("pw-export", "") {
            return;
        }
        let Some(st) = &self.storage else { return };
        let logins: Vec<Login> = st
            .list_passwords()
            .into_iter()
            .filter_map(|(id, origin, user, _)| Some(Login { origin, user, pass: st.password_secret(id).and_then(|b| crate::secret::decrypt(&b))? }))
            .collect();
        if logins.is_empty() {
            return self.toast_port("There are no saved passwords to export");
        }
        let dir = self.hub.download_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = crate::web::unique_path(&dir, "axomai-passwords.csv");
        match std::fs::write(&path, logins_csv(&logins)) {
            Ok(_) => self.toast_port(&format!("Exported {} logins to {} \u{2014} it is NOT encrypted, delete it when done", logins.len(), path.display())),
            Err(_) => self.toast_port("Could not write the export file"),
        }
    }

    pub fn passwords_import(&mut self, path: &str) {
        let Ok(text) = std::fs::read_to_string(path) else { return self.toast_port("Could not read that file") };
        let logins = parse_logins(&text);
        if logins.is_empty() {
            return self.toast_port("No logins found. The file needs url, username and password columns");
        }
        let mut n = 0;
        if let Some(st) = &self.storage {
            for l in &logins {
                if let Some(blob) = crate::secret::encrypt(&l.pass) {
                    if st.save_password(&l.origin, &l.user, &blob).is_ok() {
                        n += 1;
                    }
                }
            }
        }
        self.reload_passwords_page();
        self.toast_port(&format!("Imported {} logins", n));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_handles_quotes_commas_and_newlines() {
        let rows = csv_rows("a,\"b,c\",\"d\"\"e\"\r\n\"line\nbreak\",x,\r\n\r\nlast,row,here");
        assert_eq!(rows[0], vec!["a", "b,c", "d\"e"]);
        assert_eq!(rows[1], vec!["line\nbreak", "x", ""]);
        assert_eq!(rows[2], vec!["last", "row", "here"]);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn chrome_password_csv_is_understood() {
        let csv = "name,url,username,password,note\nexample.com,https://example.com/login,me@x.test,\"p,w\"\"1\",\nbad,javascript:1,u,p,\nnopass,https://a.test,u,,\n";
        let logins = parse_logins(csv);
        assert_eq!(logins, vec![Login { origin: "https://example.com".into(), user: "me@x.test".into(), pass: "p,w\"1".into() }]);
        assert!(parse_logins("a,b\n1,2").is_empty(), "no url/password columns");
        assert!(parse_logins("").is_empty());
    }

    #[test]
    fn exported_passwords_can_be_imported_again() {
        let l = vec![Login { origin: "https://a.test".into(), user: "u,1".into(), pass: "pa\"ss\nword".into() }];
        let csv = logins_csv(&l);
        assert!(csv.starts_with("name,url,username,password,note\r\n"));
        assert_eq!(parse_logins(&csv), l);
    }

    #[test]
    fn backup_round_trips_into_a_fresh_profile() {
        let a = BrowserStorage::in_memory();
        a.add_bookmark("https://b.test", "B", "Work").unwrap();
        a.add_history("https://h.test", "H").unwrap();
        a.set_setting("weather_city", "guwahati").unwrap();
        a.reading_add("https://r.test", "R").unwrap();
        a.note_add("https://n.test/p", "hello").unwrap();
        a.site_rule_set("blocked.test", Some(true), Some(true)).unwrap();
        a.session_save("S", &["https://s.test".to_string(), "javascript:1".to_string()]).unwrap();
        let text = serde_json::to_string(&backup_json(&a)).unwrap();

        let b = BrowserStorage::in_memory();
        let r = restore_backup(&b, &serde_json::from_str(&text).unwrap()).unwrap();
        assert_eq!((r.bookmarks, r.history, r.reading, r.notes, r.sites, r.sessions), (1, 1, 1, 1, 1, 1));
        assert_eq!(b.get_bookmarks().unwrap()[0].folder, "Work");
        assert_eq!(b.get_setting("weather_city").unwrap().as_deref(), Some("guwahati"));
        assert_eq!(b.site_rules(), vec![("blocked.test".to_string(), true, true)]);
        assert_eq!(b.sessions()[0].urls, vec!["https://s.test".to_string()]);
        // restoring twice does not duplicate anything that has an identity
        let again = restore_backup(&b, &serde_json::from_str(&text).unwrap()).unwrap();
        assert_eq!((again.bookmarks, again.reading, again.notes), (0, 0, 0));
    }

    #[test]
    fn other_files_are_refused() {
        let st = BrowserStorage::in_memory();
        assert!(restore_backup(&st, &json!({"app": "Other", "format": 1})).is_none());
        assert!(restore_backup(&st, &json!({"app": "Axomai Browser", "format": 99})).is_none());
        let evil = json!({"app": "Axomai Browser", "format": 1, "bookmarks": [{"url": "javascript:alert(1)", "title": "x"}], "settings": {"bad key!": "x"}});
        let r = restore_backup(&st, &evil).unwrap();
        assert_eq!((r.bookmarks, r.settings), (0, 0));
    }
}
