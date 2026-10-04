//! Starting the browser from outside: addresses on the command line ("Open with", links from other apps), handing
//! them to a window that is already running, and registering Axomai as a web browser in Windows.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILE_EXTENSIONS: &[&str] = &["html", "htm", "xhtml", "pdf", "svg", "png", "jpg", "jpeg", "gif", "webp", "txt"];
const MAX_URLS: usize = 10;

/// `file:///C:/a%20b.html` or a plain path -> an existing local file with a safe extension.
fn local_file(arg: &str) -> Option<PathBuf> {
    let path = if let Some(rest) = arg.strip_prefix("file:///") { crate::app_commands::url_decode(rest) } else { arg.to_string() };
    let p = Path::new(&path);
    let ext = p.extension()?.to_string_lossy().to_ascii_lowercase();
    (FILE_EXTENSIONS.contains(&ext.as_str()) && p.is_file()).then(|| p.to_path_buf())
}

/// One command-line argument as an address to open: web addresses as they are, existing documents as file:// URLs,
/// anything else (flags, other schemes such as `javascript:`) is ignored.
pub fn address_from_arg(arg: &str) -> Option<String> {
    let arg = arg.trim().trim_matches('"');
    let lower = arg.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return (arg.len() < 4096 && !arg.contains(char::is_whitespace)).then(|| arg.to_string());
    }
    if arg.starts_with('-') || lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("axomai:") {
        return None;
    }
    local_file(arg).map(|p| format!("file:///{}", p.to_string_lossy().replace('\\', "/")))
}

pub fn addresses_from_args(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for a in args {
        if let Some(u) = address_from_arg(a) {
            if !out.contains(&u) {
                out.push(u);
            }
        }
        if out.len() >= MAX_URLS {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------- hand-over to a running window

fn handoff_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("handoff")
}

fn now_millis() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Written about once a second by a running window so a new launch can tell it is there.
pub fn heartbeat(data_dir: &Path) {
    let _ = std::fs::create_dir_all(handoff_dir(data_dir));
    let _ = std::fs::write(handoff_dir(data_dir).join("alive"), now_millis().to_string());
}

pub fn is_running(data_dir: &Path) -> bool {
    std::fs::read_to_string(handoff_dir(data_dir).join("alive"))
        .ok()
        .and_then(|t| t.trim().parse::<u128>().ok())
        .is_some_and(|at| now_millis().saturating_sub(at) < 4000)
}

/// Pass addresses to the running window. Returns false when none is running (then this process opens them itself).
pub fn hand_over(data_dir: &Path, urls: &[String]) -> bool {
    if urls.is_empty() || !is_running(data_dir) {
        return false;
    }
    let dir = handoff_dir(data_dir);
    let stamp = now_millis();
    for (i, u) in urls.iter().enumerate() {
        let tmp = dir.join(format!("{}-{}.tmp", stamp, i));
        if std::fs::write(&tmp, u).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join(format!("{}-{}.url", stamp, i)));
        }
    }
    true
}

/// Addresses another launch left for this window (each file is removed as it is read).
pub fn take_handed_over(data_dir: &Path) -> Vec<String> {
    let dir = handoff_dir(data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "url")).collect();
    files.sort();
    let mut out = Vec::new();
    for f in files.into_iter().take(MAX_URLS) {
        if let Ok(text) = std::fs::read_to_string(&f) {
            // The file is not trusted either: it goes through the same filter as a command-line argument.
            if let Some(u) = address_from_arg(&text) {
                out.push(u);
            }
        }
        let _ = std::fs::remove_file(&f);
    }
    out
}

// ---------------------------------------------------------------- registering as a web browser

/// `(key under HKCU, value name or "" for the default value, data)`.
pub type RegEntry = (String, String, String);

pub const PROG_HTML: &str = "AxomaiHTML";
pub const PROG_URL: &str = "AxomaiURL";

pub fn registry_entries(exe: &str) -> Vec<RegEntry> {
    let open = format!("\"{}\" \"%1\"", exe);
    let e = |k: &str, n: &str, v: &str| (k.to_string(), n.to_string(), v.to_string());
    let caps = "Software\\Axomai\\Capabilities";
    vec![
        e(&format!("Software\\Classes\\{}", PROG_HTML), "", "Axomai HTML Document"),
        e(&format!("Software\\Classes\\{}\\DefaultIcon", PROG_HTML), "", &format!("{},0", exe)),
        e(&format!("Software\\Classes\\{}\\shell\\open\\command", PROG_HTML), "", &open),
        e(&format!("Software\\Classes\\{}", PROG_URL), "", "Axomai URL"),
        e(&format!("Software\\Classes\\{}", PROG_URL), "URL Protocol", ""),
        e(&format!("Software\\Classes\\{}\\DefaultIcon", PROG_URL), "", &format!("{},0", exe)),
        e(&format!("Software\\Classes\\{}\\shell\\open\\command", PROG_URL), "", &open),
        e(caps, "ApplicationName", "Axomai Browser"),
        e(caps, "ApplicationDescription", "Smart. Assamese. AI for all."),
        e(&format!("{}\\URLAssociations", caps), "http", PROG_URL),
        e(&format!("{}\\URLAssociations", caps), "https", PROG_URL),
        e(&format!("{}\\FileAssociations", caps), ".html", PROG_HTML),
        e(&format!("{}\\FileAssociations", caps), ".htm", PROG_HTML),
        e(&format!("{}\\FileAssociations", caps), ".xhtml", PROG_HTML),
        e("Software\\RegisteredApplications", "Axomai Browser", caps),
    ]
}

/// What `reg add` needs for one entry (no shell involved, so nothing in the values can be misread).
pub fn reg_add_args(entry: &RegEntry) -> Vec<String> {
    let (key, name, value) = entry;
    let mut a = vec!["add".to_string(), format!("HKCU\\{}", key)];
    if name.is_empty() {
        a.push("/ve".into());
    } else {
        a.push("/v".into());
        a.push(name.clone());
    }
    a.extend(["/t".to_string(), "REG_SZ".to_string(), "/d".to_string(), value.clone(), "/f".to_string()]);
    a
}

#[cfg(windows)]
fn reg(args: &[String]) -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(not(windows))]
fn reg(_args: &[String]) -> bool {
    false
}

/// Make Axomai appear in Windows' list of browsers. Windows itself keeps the final choice with the user.
pub fn register(exe: &str) -> bool {
    registry_entries(exe).iter().all(|e| reg(&reg_add_args(e)))
}

pub fn unregister() {
    for key in [
        format!("HKCU\\Software\\Classes\\{}", PROG_HTML),
        format!("HKCU\\Software\\Classes\\{}", PROG_URL),
        "HKCU\\Software\\Axomai".to_string(),
    ] {
        reg(&["delete".to_string(), key, "/f".to_string()]);
    }
    reg(&["delete".to_string(), "HKCU\\Software\\RegisteredApplications".to_string(), "/v".to_string(), "Axomai Browser".to_string(), "/f".to_string()]);
}

/// Open Windows' "Default apps" page, focused on Axomai.
pub fn open_default_apps_settings() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer").arg("ms-settings:defaultapps?registeredAppUser=Axomai%20Browser").creation_flags(0x0800_0000).spawn();
    }
}

/// Is Axomai the program Windows uses for https links?
pub fn is_default() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("reg")
            .args(["query", "HKCU\\Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice", "/v", "ProgId"])
            .creation_flags(0x0800_0000)
            .output();
        if let Ok(o) = out {
            return String::from_utf8_lossy(&o.stdout).contains(PROG_URL);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("axomai_launch_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn only_web_addresses_and_real_documents_are_opened() {
        assert_eq!(address_from_arg("https://example.com/a?b=1").as_deref(), Some("https://example.com/a?b=1"));
        assert_eq!(address_from_arg("\"HTTP://Example.com\"").as_deref(), Some("HTTP://Example.com"));
        for bad in ["--incognito", "javascript:alert(1)", "data:text/html,hi", "axomai://settings", "ftp://x.test", "C:\\Windows\\System32\\cmd.exe", "https://a.test/ with space", "", "not a path"] {
            assert_eq!(address_from_arg(bad), None, "{}", bad);
        }
    }

    #[test]
    fn local_documents_become_file_urls() {
        let d = temp("docs");
        let page = d.join("my page.html");
        std::fs::write(&page, "<p>x</p>").unwrap();
        let exe = d.join("tool.exe");
        std::fs::write(&exe, "x").unwrap();
        let url = address_from_arg(&page.to_string_lossy()).unwrap();
        assert!(url.starts_with("file:///") && url.ends_with("my page.html"));
        assert_eq!(address_from_arg(&exe.to_string_lossy()), None, "programs are never opened");
        assert_eq!(address_from_arg(&d.join("missing.html").to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn argument_list_is_deduplicated_and_capped() {
        let mut args: Vec<String> = vec!["axomai.exe".into(), "--incognito".into(), "https://a.test".into(), "https://a.test".into()];
        args.extend((0..30).map(|i| format!("https://site{}.test", i)));
        let urls = addresses_from_args(&args);
        assert_eq!(urls.len(), MAX_URLS);
        assert_eq!(urls[0], "https://a.test");
    }

    #[test]
    fn hand_over_needs_a_live_window_and_delivers_each_address_once() {
        let d = temp("handoff");
        let urls = vec!["https://one.test".to_string(), "https://two.test".to_string()];
        assert!(!hand_over(&d, &urls), "nobody is running yet");
        heartbeat(&d);
        assert!(is_running(&d));
        assert!(hand_over(&d, &urls));
        assert_eq!(take_handed_over(&d), urls);
        assert!(take_handed_over(&d).is_empty(), "files are consumed");
        // A stale heartbeat means the window is gone.
        std::fs::write(handoff_dir(&d).join("alive"), "1").unwrap();
        assert!(!is_running(&d));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn handed_over_files_are_filtered_like_arguments() {
        let d = temp("hostile");
        heartbeat(&d);
        std::fs::write(handoff_dir(&d).join("1-0.url"), "javascript:alert(1)").unwrap();
        std::fs::write(handoff_dir(&d).join("1-1.url"), "C:\\Windows\\System32\\calc.exe").unwrap();
        std::fs::write(handoff_dir(&d).join("1-2.url"), "https://fine.test").unwrap();
        assert_eq!(take_handed_over(&d), vec!["https://fine.test".to_string()]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn registry_entries_cover_http_https_and_html() {
        let entries = registry_entries("C:\\Program Files\\Axomai\\axomai.exe");
        let has = |k: &str, n: &str, v: &str| entries.iter().any(|(ek, en, ev)| ek.ends_with(k) && en == n && ev == v);
        assert!(has("URLAssociations", "http", PROG_URL) && has("URLAssociations", "https", PROG_URL));
        assert!(has("FileAssociations", ".html", PROG_HTML));
        assert!(entries.iter().any(|(k, _, v)| k.ends_with("AxomaiURL\\shell\\open\\command") && v == "\"C:\\Program Files\\Axomai\\axomai.exe\" \"%1\""));
        assert!(entries.iter().all(|(k, _, _)| k.starts_with("Software\\")), "everything lives under the current user");
    }

    #[test]
    fn reg_arguments_are_separate_values() {
        let a = reg_add_args(&("Software\\X".into(), "".into(), "a b".into()));
        assert_eq!(a, vec!["add", "HKCU\\Software\\X", "/ve", "/t", "REG_SZ", "/d", "a b", "/f"]);
        let b = reg_add_args(&("Software\\X".into(), "Name".into(), "v".into()));
        assert!(b.contains(&"/v".to_string()) && b.contains(&"Name".to_string()));
    }
}
