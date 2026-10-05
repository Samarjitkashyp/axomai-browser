//! Sync between your own computers through a folder you choose (OneDrive, Google Drive, Dropbox, a USB stick...).
//! There is no Axomai server: the browser writes one end-to-end encrypted file (AES-256-GCM, key from your passphrase
//! with PBKDF2) into that folder, and every computer merges what is in it. The folder service only ever sees ciphertext.
//! Merging adds and combines; deleting something on one computer does not delete it on the others.

use crate::app::App;
use crate::storage::BrowserStorage;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "axomai-sync.json";
pub const DEFAULT_ITERATIONS: u32 = 200_000;
const MIN_ITERATIONS: u32 = 1_000;
const MAX_ITERATIONS: u32 = 5_000_000;

#[derive(Debug, PartialEq, Eq)]
pub enum SyncError {
    NotOurFile,
    WrongPassphrase,
    Io(String),
}

impl SyncError {
    pub fn message(&self) -> String {
        match self {
            SyncError::NotOurFile => "The sync file in that folder is not an Axomai sync file.".into(),
            SyncError::WrongPassphrase => "The passphrase does not open the sync file in that folder (or the file was changed).".into(),
            SyncError::Io(e) => format!("Could not use the sync folder: {}", e),
        }
    }
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    let _ = getrandom::getrandom(&mut b);
    b
}

pub fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, iterations, &mut key);
    key
}

pub struct Header {
    pub salt: Vec<u8>,
    pub iterations: u32,
}

pub fn read_header(file: &[u8]) -> Result<Header, SyncError> {
    let v: Value = serde_json::from_slice(file).map_err(|_| SyncError::NotOurFile)?;
    if v.get("app").and_then(|a| a.as_str()) != Some("Axomai Browser sync") || v.get("v").and_then(|x| x.as_u64()) != Some(1) {
        return Err(SyncError::NotOurFile);
    }
    let salt = v.get("salt").and_then(|s| s.as_str()).and_then(crate::sys::base64_decode).ok_or(SyncError::NotOurFile)?;
    let iterations = v.get("iters").and_then(|i| i.as_u64()).map(|i| i as u32).filter(|i| (MIN_ITERATIONS..=MAX_ITERATIONS).contains(i)).ok_or(SyncError::NotOurFile)?;
    if salt.len() < 8 || salt.len() > 64 {
        return Err(SyncError::NotOurFile);
    }
    Ok(Header { salt, iterations })
}

pub fn encrypt(key: &[u8; 32], salt: &[u8], iterations: u32, plain: &[u8]) -> Vec<u8> {
    let nonce_bytes: [u8; 12] = random_bytes();
    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key");
    let ct = cipher.encrypt(Nonce::from_slice(&nonce_bytes), plain).expect("encryption cannot fail for in-memory data");
    serde_json::to_vec(&json!({
        "app": "Axomai Browser sync",
        "v": 1,
        "salt": crate::sys::base64_encode(salt),
        "iters": iterations,
        "nonce": crate::sys::base64_encode(&nonce_bytes),
        "ct": crate::sys::base64_encode(&ct),
    }))
    .unwrap_or_default()
}

pub fn decrypt(key: &[u8; 32], file: &[u8]) -> Result<Vec<u8>, SyncError> {
    read_header(file)?;
    let v: Value = serde_json::from_slice(file).map_err(|_| SyncError::NotOurFile)?;
    let nonce = v.get("nonce").and_then(|s| s.as_str()).and_then(crate::sys::base64_decode).filter(|n| n.len() == 12).ok_or(SyncError::NotOurFile)?;
    let ct = v.get("ct").and_then(|s| s.as_str()).and_then(crate::sys::base64_decode).ok_or(SyncError::NotOurFile)?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| SyncError::WrongPassphrase)?;
    cipher.decrypt(Nonce::from_slice(&nonce), ct.as_ref()).map_err(|_| SyncError::WrongPassphrase)
}

// ------------------------------------------------------------------ what is synced

/// Bookmarks, reading list, notes, site rules, sessions and shortcuts, plus saved logins when `with_passwords`.
pub fn snapshot(st: &BrowserStorage, with_passwords: bool) -> Value {
    let mut v = crate::portability::backup_json(st);
    if let Some(o) = v.as_object_mut() {
        o.remove("history");
        let shortcuts = st.get_setting("shortcuts").ok().flatten().unwrap_or_default();
        o.insert("settings".into(), json!({ "shortcuts": shortcuts }));
        if with_passwords {
            let logins: Vec<Value> = st
                .password_rows_full()
                .into_iter()
                .filter_map(|(origin, user, blob, updated)| Some(json!({"origin": origin, "user": user, "pass": crate::secret::decrypt(&blob)?, "updated": updated})))
                .collect();
            o.insert("passwords".into(), Value::Array(logins));
        }
    }
    v
}

/// Merge a snapshot from another computer into this one. Returns how many items were new here.
pub fn merge(st: &BrowserStorage, remote: &Value, with_passwords: bool) -> usize {
    let mut added = 0;
    if let Some(r) = crate::portability::restore_backup(st, remote) {
        added += r.bookmarks + r.reading + r.notes + r.sites + r.sessions;
        // sessions with the same name replaced each other; shortcuts keep the longer list
        let local = st.get_setting("shortcuts").ok().flatten().unwrap_or_default();
        let remote_sc = remote.get("settings").and_then(|s| s.get("shortcuts")).and_then(|s| s.as_str()).unwrap_or("");
        if remote_sc.lines().count() > local.lines().count() {
            let _ = st.set_setting("shortcuts", remote_sc);
        } else if !local.is_empty() {
            let _ = st.set_setting("shortcuts", &local);
        }
    }
    if with_passwords {
        let local: Vec<(String, String, Vec<u8>, String)> = st.password_rows_full();
        for p in remote.get("passwords").and_then(|p| p.as_array()).cloned().unwrap_or_default() {
            let s = |k: &str| p.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            let (origin, user, pass, updated) = (s("origin"), s("user"), s("pass"), s("updated"));
            if crate::passwords::origin_of(&origin).as_deref() != Some(origin.as_str()) || pass.is_empty() || pass.len() > 1000 || user.len() > 500 {
                continue;
            }
            let existing = local.iter().find(|l| l.0 == origin && l.1 == user);
            let take = match existing {
                None => true,
                Some(l) => updated > l.3 && crate::secret::decrypt(&l.2).as_deref() != Some(pass.as_str()),
            };
            if take {
                if let Some(blob) = crate::secret::encrypt(&pass) {
                    if st.save_password_at(&origin, &user, &blob, &updated).is_ok() {
                        added += 1;
                    }
                }
            }
        }
    }
    added
}

// ------------------------------------------------------------------ app side

fn sync_file(dir: &str) -> PathBuf {
    Path::new(dir).join(FILE_NAME)
}

// ------------------------------------------------------------------ settings section

/// The "Sync between your computers" part of the Settings page.
pub fn settings_section(lang: &str, dir: &str, pending: &str, last: &str, passwords: bool) -> String {
    use crate::i18n::tr;
    use crate::settings_page::{row, switch};
    use crate::viewsource::escape;
    let t = |k: &'static str| tr(lang, k);
    if !dir.is_empty() {
        let when = if last.is_empty() { String::new() } else { format!(" \u{b7} {} {}", t("sync.last"), last) };
        let buttons = format!(
            r#"<button class="btn" onclick="go('sync-now')">{}</button> <button class="btn ghost" onclick="go('sync-off')">{}</button>"#,
            escape(t("sync.now")),
            escape(t("sync.off"))
        );
        return format!(
            "{}{}{}",
            row(t("sync.on"), &format!("{} {}{}", t("sync.folder"), dir, when), &buttons),
            row(t("sync.passwords"), t("sync.passwords.desc"), &switch("data-set=\"sync_passwords\"", passwords)),
            row(t("sync.note"), "", "")
        );
    }
    let shown = if pending.is_empty() { t("sync.nofolder").to_string() } else { pending.to_string() };
    let choose = format!(r#"<button class="btn ghost keepscroll" onclick="go('sync-folder')">{}</button>"#, escape(t("sync.choose")));
    let turn_on = format!(
        r#"<input type="password" id="syncPass" placeholder="{}" style="width:200px" autocomplete="off"> <button class="btn" data-f="{}" onclick="go('sync-setup/'+enc(this.dataset.f)+'/'+enc(document.getElementById('syncPass').value))">{}</button>"#,
        escape(t("sync.pass")),
        escape(pending),
        escape(t("sync.turnon"))
    );
    format!("{}{}", row(t("sync.title"), t("sync.desc"), &choose), row(&shown, t("sync.pass.desc"), &turn_on))
}

impl App {
    fn toast_sync(
&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    pub fn pick_sync_folder(&self) {
        let shared = self.shared();
        std::thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new().set_title("Choose the folder to sync through (for example in OneDrive or Google Drive)").pick_folder() {
                shared.push_event(crate::web::WebEvent::PageData("sync-folder".into(), String::new(), p.to_string_lossy().to_string()));
            }
        });
    }

    pub fn sync_folder_chosen(&mut self, folder: &str) {
        if let Some(st) = &self.storage {
            let _ = st.set_setting("sync_dir_pending", folder);
        }
        self.refresh_internal_page();
    }

    /// Turn sync on with a folder and a passphrase. The first computer creates the file, later ones open it.
    pub fn sync_setup(&mut self, folder: &str, passphrase: &str) {
        let folder = folder.trim();
        if passphrase.chars().count() < 8 {
            return self.toast_sync("Use a passphrase of at least 8 characters.");
        }
        if !Path::new(folder).is_dir() {
            return self.toast_sync("Choose a folder first.");
        }
        let file = sync_file(folder);
        let (salt, iterations, existing) = match std::fs::read(&file) {
            Ok(bytes) => match read_header(&bytes) {
                Ok(h) => (h.salt, h.iterations, Some(bytes)),
                Err(e) => return self.toast_sync(&e.message()),
            },
            Err(_) => (random_bytes::<16>().to_vec(), DEFAULT_ITERATIONS, None),
        };
        let key = derive_key(passphrase, &salt, iterations);
        if let Some(bytes) = &existing {
            if let Err(e) = decrypt(&key, bytes) {
                return self.toast_sync(&e.message());
            }
        }
        let Some(blob) = crate::secret::encrypt(&crate::sys::base64_encode(&key)) else { return self.toast_sync("Could not protect the key with your Windows account.") };
        if let Some(st) = &self.storage {
            let _ = st.set_setting("sync_dir", folder);
            let _ = st.set_setting("sync_key", &crate::sys::base64_encode(&blob));
            // The key was made with this salt; a file written later must carry the same one.
            let _ = st.set_setting("sync_salt", &crate::sys::base64_encode(&salt));
            let _ = st.set_setting("sync_iters", &iterations.to_string());
            let _ = st.delete_setting("sync_dir_pending");
            let _ = st.set_setting("sync_enabled", "1");
        }
        self.sync_now(true);
    }

    pub fn sync_off(&mut self) {
        if let Some(st) = &self.storage {
            for k in ["sync_dir", "sync_key", "sync_salt", "sync_iters", "sync_enabled", "sync_last", "sync_dir_pending"] {
                let _ = st.delete_setting(k);
            }
        }
        self.refresh_internal_page();
        self.toast_sync("Sync is off. The file in the folder was left where it is.");
    }

    fn sync_key(&self) -> Option<[u8; 32]> {
        let st = self.storage.as_ref()?;
        let blob = crate::sys::base64_decode(&st.get_setting("sync_key").ok().flatten()?)?;
        let key = crate::sys::base64_decode(&crate::secret::decrypt(&blob)?)?;
        key.try_into().ok()
    }

    /// Read the folder's file, merge it into this computer, write the combined result back.
    pub fn sync_now(&mut self, manual: bool) {
        let Some(st) = &self.storage else { return };
        let (Some(dir), Some(key)) = (st.get_setting("sync_dir").ok().flatten(), self.sync_key()) else {
            if manual {
                self.toast_sync("Turn sync on first (Settings > Sync).");
            }
            return;
        };
        let with_pw = self.settings.sync_passwords;
        let file = sync_file(&dir);
        let mut merged = 0;
        let mut salt_iters: Option<(Vec<u8>, u32)> = None;
        if let Ok(bytes) = std::fs::read(&file) {
            match (read_header(&bytes), decrypt(&key, &bytes)) {
                (Ok(h), Ok(plain)) => {
                    if let Ok(remote) = serde_json::from_slice::<Value>(&plain) {
                        merged = merge(st, &remote, with_pw);
                    }
                    salt_iters = Some((h.salt, h.iterations));
                }
                (_, Err(e)) | (Err(e), _) => {
                    if manual {
                        self.toast_sync(&e.message());
                    }
                    return;
                }
            }
        }
        let stored = || -> Option<(Vec<u8>, u32)> {
            let st = self.storage.as_ref()?;
            let salt = crate::sys::base64_decode(&st.get_setting("sync_salt").ok().flatten()?)?;
            let iters = st.get_setting("sync_iters").ok().flatten()?.parse().ok()?;
            Some((salt, iters))
        };
        let Some((salt, iterations)) = salt_iters.or_else(stored) else {
            return self.toast_sync("The sync settings are incomplete; turn sync off and on again.");
        };
        let plain = serde_json::to_vec(&snapshot(st, with_pw)).unwrap_or_default();
        let out = encrypt(&key, &salt, iterations, &plain);
        let tmp = file.with_extension("json.tmp");
        let ok = std::fs::write(&tmp, &out).and_then(|_| std::fs::rename(&tmp, &file)).is_ok();
        if let Some(st) = &self.storage {
            if ok {
                let _ = st.set_setting("sync_last", &crate::sys::timestamp());
            }
        }
        if merged > 0 {
            self.refresh_bar();
        }
        self.refresh_internal_page();
        if manual || merged > 0 {
            self.toast_sync(&if ok { format!("\u{1F504} Synced. {} new item{} from your other computers.", merged, if merged == 1 { "" } else { "s" }) } else { "Could not write to the sync folder.".to_string() });
        }
    }

    pub fn sync_startup(&mut self) {
        if self.private_window || self.storage.as_ref().and_then(|s| s.get_setting("sync_enabled").ok().flatten()).as_deref() != Some("1") {
            return;
        }
        self.sync_now(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_files_open_only_with_the_right_passphrase() {
        let salt = [7u8; 16];
        let key = derive_key("correct horse battery", &salt, 1000);
        let file = encrypt(&key, &salt, 1000, b"secret data");
        assert!(!String::from_utf8_lossy(&file).contains("secret data"));
        assert_eq!(decrypt(&key, &file).unwrap(), b"secret data");
        let wrong = derive_key("incorrect horse battery", &salt, 1000);
        assert_eq!(decrypt(&wrong, &file), Err(SyncError::WrongPassphrase));
        let h = read_header(&file).unwrap();
        assert_eq!((h.salt, h.iterations), (salt.to_vec(), 1000));
    }

    #[test]
    fn tampering_and_foreign_files_are_detected() {
        let salt = [1u8; 16];
        let key = derive_key("a long enough passphrase", &salt, 1000);
        let file = encrypt(&key, &salt, 1000, b"data");
        let mut v: Value = serde_json::from_slice(&file).unwrap();
        let mut ct = crate::sys::base64_decode(v["ct"].as_str().unwrap()).unwrap();
        ct[0] ^= 1;
        v["ct"] = Value::String(crate::sys::base64_encode(&ct));
        assert_eq!(decrypt(&key, &serde_json::to_vec(&v).unwrap()), Err(SyncError::WrongPassphrase));
        assert_eq!(read_header(b"not json").err(), Some(SyncError::NotOurFile));
        assert_eq!(read_header(b"{\"app\":\"x\"}").err(), Some(SyncError::NotOurFile));
        v["iters"] = json!(5);
        assert_eq!(read_header(&serde_json::to_vec(&v).unwrap()).err(), Some(SyncError::NotOurFile), "absurd iteration counts are refused");
    }

    #[test]
    fn two_computers_end_up_with_both_sets_of_data() {
        let a = BrowserStorage::in_memory();
        a.add_bookmark("https://a.test", "A", "Unsorted").unwrap();
        a.reading_add("https://read-a.test", "RA").unwrap();
        a.set_setting("shortcuts", "@a\thttps://a.test/?q=%s").unwrap();
        let b = BrowserStorage::in_memory();
        b.add_bookmark("https://b.test", "B", "Unsorted").unwrap();
        b.add_bookmark("https://a.test", "A", "Unsorted").unwrap();

        let from_a = snapshot(&a, false);
        assert!(from_a.get("history").is_none() && from_a.get("passwords").is_none());
        let added = merge(&b, &from_a, false);
        assert_eq!(added, 1, "the reading-list item is new; a.test was already there as a bookmark");
        assert_eq!(b.get_bookmarks().unwrap().len(), 2);
        assert_eq!(b.get_setting("shortcuts").unwrap().as_deref(), Some("@a\thttps://a.test/?q=%s"));

        let from_b = snapshot(&b, false);
        merge(&a, &from_b, false);
        assert_eq!(a.get_bookmarks().unwrap().len(), 2);
        assert_eq!(merge(&a, &from_b, false), 0, "merging the same data again changes nothing");
    }
}
