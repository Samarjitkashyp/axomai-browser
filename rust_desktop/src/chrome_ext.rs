//! Chrome extensions: "Load unpacked" copies an unpacked extension folder into the browser's own folder and the web
//! engine loads every extension found there when a tab starts. Switching one off moves it aside, removing deletes it;
//! both take effect after a restart (the engine reads the folder when a view is created).

use crate::app::App;
use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_COPY_BYTES: u64 = 150 * 1024 * 1024;
const MAX_FILES: usize = 6000;

pub fn enabled_root() -> PathBuf {
    crate::storage::data_dir().join("chrome-extensions")
}

pub fn disabled_root() -> PathBuf {
    crate::storage::data_dir().join("chrome-extensions-disabled")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeExt {
    /// Folder name (also its id here).
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
}

pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub manifest_version: u64,
}

pub fn parse_manifest(text: &str) -> Option<Manifest> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let mv = v.get("manifest_version")?.as_u64()?;
    if !(mv == 2 || mv == 3) {
        return None;
    }
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    Some(Manifest { name: s("name"), version: s("version"), description: s("description"), manifest_version: mv })
}

/// A folder name that is safe to create: letters, digits, `-`, `_` and `.`; never empty or a path.
pub fn safe_dir_name(raw: &str) -> String {
    let n: String = raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let n = n.trim_matches('_').chars().take(40).collect::<String>();
    if n.is_empty() {
        "extension".to_string()
    } else {
        n
    }
}

/// Is `id` the name of a direct child folder (no separators or `..`)?
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 60 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn dir_size(dir: &Path, files: &mut usize) -> Option<u64> {
    let mut total = 0u64;
    for e in std::fs::read_dir(dir).ok()? {
        let e = e.ok()?;
        let meta = e.metadata().ok()?;
        if meta.file_type().is_symlink() {
            return None;
        }
        *files += 1;
        if *files > MAX_FILES {
            return None;
        }
        total += if meta.is_dir() { dir_size(&e.path(), files)? } else { meta.len() };
        if total > MAX_COPY_BYTES {
            return None;
        }
    }
    Some(total)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), target)?;
        }
    }
    Ok(())
}

/// Copy an unpacked extension into `root`. Returns the new folder name, or why it was refused.
pub fn install_from(src: &Path, root: &Path) -> Result<String, String> {
    let manifest = std::fs::read_to_string(src.join("manifest.json")).map_err(|_| "That folder has no manifest.json. Choose the folder that contains it.".to_string())?;
    let m = parse_manifest(&manifest).ok_or_else(|| "manifest.json is not a valid Chrome extension manifest (version 2 or 3).".to_string())?;
    let mut files = 0;
    dir_size(src, &mut files).ok_or_else(|| "The extension folder is too big or contains shortcuts, so it was not copied.".to_string())?;
    let base = safe_dir_name(if m.name.starts_with("__MSG_") { src.file_name().and_then(|n| n.to_str()).unwrap_or("extension") } else { &m.name });
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() || disabled_root().join(&id).exists() {
        id = format!("{}-{}", base, n);
        n += 1;
    }
    copy_dir(src, &root.join(&id)).map_err(|e| {
        let _ = std::fs::remove_dir_all(root.join(&id));
        e.to_string()
    })?;
    Ok(id)
}

pub fn list_in(root: &Path, enabled: bool) -> Vec<ChromeExt> {
    let Ok(rd) = std::fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<ChromeExt> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let m = parse_manifest(&std::fs::read_to_string(e.path().join("manifest.json")).ok()?)?;
            let name = if m.name.is_empty() || m.name.starts_with("__MSG_") { id.clone() } else { m.name };
            Some(ChromeExt { id, name, version: m.version, description: m.description, enabled })
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn list_all() -> Vec<ChromeExt> {
    let mut v = list_in(&enabled_root(), true);
    v.extend(list_in(&disabled_root(), false));
    v
}

impl App {
    fn toast_cext(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    pub fn pick_chrome_ext_folder(&self) {
        let shared = self.shared();
        std::thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new().set_title("Choose an unpacked Chrome extension folder (the one with manifest.json)").pick_folder() {
                shared.push_event(crate::web::WebEvent::PageData("cext-folder".into(), String::new(), p.to_string_lossy().to_string()));
            }
        });
    }

    pub fn chrome_ext_install(&mut self, path: &str) {
        match install_from(Path::new(path), &enabled_root()) {
            Ok(id) => {
                self.toast_cext(&format!("Added \"{}\". Restart the browser to start using it.", id));
                self.refresh_internal_page();
            }
            Err(e) => self.toast_cext(&e),
        }
    }

    pub fn chrome_ext_command(&mut self, action: &str, id: &str) {
        if !valid_id(id) {
            return;
        }
        let (on, off) = (enabled_root(), disabled_root());
        match action {
            "toggle" => {
                let (from, to) = if on.join(id).is_dir() { (on.join(id), off.join(id)) } else { (off.join(id), on.join(id)) };
                if from.is_dir() && !to.exists() {
                    let _ = std::fs::create_dir_all(to.parent().unwrap_or(Path::new(".")));
                    if std::fs::rename(&from, &to).is_err() {
                        return self.toast_cext("Could not switch that extension (is it in use? close other Axomai windows)");
                    }
                }
            }
            "remove" => {
                for root in [&on, &off] {
                    let p = root.join(id);
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    }
                }
            }
            _ => {}
        }
        self.toast_cext("Restart the browser for this change to take effect.");
        self.refresh_internal_page();
    }

    /// Save the session, start a fresh copy of the program and quit this one.
    pub fn restart_app(&mut self) {
        self.save_session();
        if let Some(st) = &self.storage {
            let _ = st.set_setting("restore_once", "1");
        }
        let dir = crate::storage::data_dir().join("handoff");
        let _ = std::fs::remove_dir_all(&dir); // so the new copy does not hand its work to this one
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
        self.exit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("axcext-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn manifests_are_validated() {
        let m = parse_manifest("{\"manifest_version\":3,\"name\":\"Tea\",\"version\":\"1.2\",\"description\":\"d\"}").unwrap();
        assert_eq!((m.name.as_str(), m.version.as_str(), m.manifest_version), ("Tea", "1.2", 3));
        assert!(parse_manifest("{\"manifest_version\":1,\"name\":\"x\"}").is_none());
        assert!(parse_manifest("not json").is_none());
        assert!(parse_manifest("{\"name\":\"x\"}").is_none());
    }

    #[test]
    fn folder_names_are_safe() {
        assert_eq!(safe_dir_name("My Cool/Ext: v2"), "My_Cool_Ext__v2");
        assert_eq!(safe_dir_name("..\\.."), "extension");
        assert!(valid_id("abc-1_x") && !valid_id("../x") && !valid_id("a/b") && !valid_id(""));
    }

    #[test]
    fn unpacked_extensions_are_copied_listed_and_refused_when_wrong() {
        let src = temp("src");
        std::fs::write(src.join("manifest.json"), "{\"manifest_version\":3,\"name\":\"Hello Ext\",\"version\":\"0.1\"}").unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub").join("a.js"), "1").unwrap();
        let root = temp("root");
        let id = install_from(&src, &root).unwrap();
        assert_eq!(id, "Hello_Ext");
        assert!(root.join("Hello_Ext").join("sub").join("a.js").is_file());
        assert_eq!(install_from(&src, &root).unwrap(), "Hello_Ext-2", "same name does not overwrite");
        let l = list_in(&root, true);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].name, "Hello Ext");
        let empty = temp("empty");
        assert!(install_from(&empty, &root).is_err());
        std::fs::write(empty.join("manifest.json"), "{\"manifest_version\":9}").unwrap();
        assert!(install_from(&empty, &root).is_err());
        for d in [src, root, empty] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
