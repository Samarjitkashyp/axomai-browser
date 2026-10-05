//! Update check and one-click update: looks for a newer release, downloads its installer, verifies the SHA-256 that
//! is published next to it and runs it. Nothing is installed without that checksum matching.

use crate::app::App;
use crate::web::WebEvent;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const DEFAULT_URL: &str = "https://api.github.com/repos/Samarjitkashyp/axomai-browser/releases/latest";
const MAX_INSTALLER_BYTES: u64 = 300 * 1024 * 1024;
const CHECK_EVERY_SECS: u64 = 20 * 3600;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// `AXOMAI_UPDATE_URL` replaces the release address (used to test the whole flow against a local server).
pub fn update_url() -> String {
    std::env::var("AXOMAI_UPDATE_URL").ok().filter(|u| !u.trim().is_empty()).unwrap_or_else(|| DEFAULT_URL.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub installer_url: String,
    pub sha256_url: String,
    pub notes: String,
}

pub fn version_tuple(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = it.next().map_or(Some(0), |p| p.parse().ok())?;
    it.next().is_none().then_some((major, minor, patch))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (version_tuple(candidate), version_tuple(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// The release a GitHub-style JSON describes: its version and the `Axomai-Setup-*.exe` asset with its `.sha256` file.
pub fn parse_release(v: &Value) -> Option<Release> {
    let tag = v.get("tag_name")?.as_str()?;
    version_tuple(tag)?;
    let assets = v.get("assets")?.as_array()?;
    let url_of = |pred: &dyn Fn(&str) -> bool| {
        assets.iter().find_map(|a| {
            let name = a.get("name")?.as_str()?;
            pred(name).then(|| a.get("browser_download_url").and_then(|u| u.as_str()).map(String::from))?
        })
    };
    let installer_url = url_of(&|n| n.starts_with("Axomai-Setup-") && n.ends_with(".exe"))?;
    let sha256_url = url_of(&|n| n.starts_with("Axomai-Setup-") && n.ends_with(".exe.sha256"))?;
    Some(Release {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        installer_url,
        sha256_url,
        notes: v.get("body").and_then(|b| b.as_str()).unwrap_or("").chars().take(600).collect(),
    })
}

/// May the browser fetch this address for an update? https from GitHub, or exactly the host of `AXOMAI_UPDATE_URL`.
pub fn trusted_url(url: &str, override_url: Option<&str>) -> bool {
    let host_of = |u: &str| u.split("://").nth(1).and_then(|r| r.split(['/', '?', '#']).next()).map(|h| h.to_ascii_lowercase());
    let Some(host) = host_of(url) else { return false };
    if url.starts_with("https://") {
        let github = host == "github.com" || host == "api.github.com" || host.ends_with(".githubusercontent.com");
        if github {
            return true;
        }
    }
    override_url.map_or(false, |o| (o.starts_with("http://127.0.0.1") || o.starts_with("http://localhost") || o.starts_with("https://")) && host_of(o).as_deref() == Some(host.as_str()) && url.starts_with(&o[..o.find("://").unwrap_or(0) + 3]))
}

/// The hash in a `.sha256` file (`<64 hex>  <name>`).
pub fn parse_sha256_file(text: &str) -> Option<String> {
    let h = text.split_whitespace().next()?.to_ascii_lowercase();
    (h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit())).then_some(h)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

fn fetch(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let resp = ureq::get(url).set("User-Agent", "AxomaiBrowser-updater").timeout(std::time::Duration::from_secs(60)).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_reader().take(limit + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > limit {
        return Err("the download is larger than expected".into());
    }
    Ok(buf)
}

impl App {
    fn toast_update(&self, msg: &str, action: Option<(&str, &str, &str)>) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, action, &shared);
            let _ = shared;
        }
    }

    /// Ask for the newest release in the background. `manual` checks report "up to date" and errors; automatic ones stay quiet.
    pub fn update_check(&mut self, manual: bool) {
        let shared = self.shared();
        let url = update_url();
        let over = std::env::var("AXOMAI_UPDATE_URL").ok();
        if !trusted_url(&url, over.as_deref()) && url != DEFAULT_URL {
            return;
        }
        if let Some(st) = &self.storage {
            let _ = st.set_setting("update_last", &crate::sys::unix_secs().to_string());
        }
        let tag = if manual { "manual" } else { "auto" };
        std::thread::spawn(move || {
            let result = fetch(&url, 2_000_000).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()));
            let (kind, payload) = match result {
                Ok(v) => match parse_release(&v) {
                    Some(r) if is_newer(&r.version, current_version()) => ("found", json!({"version": r.version, "installer": r.installer_url, "sha": r.sha256_url, "notes": r.notes}).to_string()),
                    Some(_) => ("none", String::new()),
                    None => ("none", String::new()),
                },
                Err(e) => ("error", e),
            };
            shared.push_event(WebEvent::PageData("update".into(), format!("{}:{}", kind, tag), payload));
        });
    }

    /// Once a day at start-up, when the user has not switched it off.
    pub fn update_check_startup(&mut self) {
        if !self.settings.update_check || self.private_window {
            return;
        }
        let last = self.storage.as_ref().and_then(|s| s.get_setting("update_last").ok().flatten()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        if crate::sys::unix_secs().saturating_sub(last) >= CHECK_EVERY_SECS {
            self.update_check(false);
        }
    }

    pub fn on_update_event(&mut self, what: &str, payload: &str) {
        let (kind, tag) = what.split_once(':').unwrap_or((what, "auto"));
        let manual = tag == "manual";
        match kind {
            "found" => {
                if let Ok(v) = serde_json::from_str::<Value>(payload) {
                    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    self.update_release = Some(Release { version: s("version"), installer_url: s("installer"), sha256_url: s("sha"), notes: s("notes") });
                    let token = self.shared().token.clone();
                    self.toast_update(&format!("Axomai {} is available", s("version")), Some(("Update now", &token, "update-install")));
                }
            }
            "none" if manual => self.toast_update(&format!("You have the latest version ({})", current_version()), None),
            "error" if manual => self.toast_update(&format!("Could not check for updates: {}", payload.chars().take(100).collect::<String>()), None),
            "downloading" => self.toast_update("Downloading the update\u{2026}", None),
            "ready" => {
                self.toast_update("Installing the update\u{2026} Axomai restarts in a moment.", None);
                self.start_installer(payload);
            }
            "failed" => self.toast_update(&format!("The update was not installed: {}", payload.chars().take(140).collect::<String>()), None),
            _ => {}
        }
    }

    /// "Update now": download the installer, check its SHA-256, then hand over to it.
    pub fn update_install(&mut self) {
        let Some(r) = self.update_release.clone() else { return self.update_check(true) };
        let over = std::env::var("AXOMAI_UPDATE_URL").ok();
        if !trusted_url(&r.installer_url, over.as_deref()) || !trusted_url(&r.sha256_url, over.as_deref()) {
            return self.toast_update("The update address is not trusted, so it was not downloaded.", None);
        }
        let shared = self.shared();
        shared.push_event(WebEvent::PageData("update".into(), "downloading".into(), String::new()));
        std::thread::spawn(move || {
            let outcome = (|| -> Result<String, String> {
                let want = parse_sha256_file(&String::from_utf8_lossy(&fetch(&r.sha256_url, 4096)?)).ok_or("the published checksum is not valid")?;
                let bytes = fetch(&r.installer_url, MAX_INSTALLER_BYTES)?;
                if sha256_hex(&bytes) != want {
                    return Err("the downloaded file does not match its checksum, so it was thrown away".into());
                }
                let dir = std::env::temp_dir().join("axomai-update");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(format!("Axomai-Setup-{}.exe", r.version.replace(|c: char| !(c.is_ascii_alphanumeric() || c == '.'), "_")));
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                Ok(path.to_string_lossy().to_string())
            })();
            let (kind, payload) = match outcome {
                Ok(p) => ("ready", p),
                Err(e) => ("failed", e),
            };
            shared.push_event(WebEvent::PageData("update".into(), format!("{}:auto", kind), payload));
        });
    }

    fn start_installer(&mut self, path: &str) {
        let p = std::path::Path::new(path);
        if !p.is_file() || p.extension().map_or(true, |e| !e.eq_ignore_ascii_case("exe")) || !p.starts_with(std::env::temp_dir().join("axomai-update")) {
            return;
        }
        self.save_session();
        if let Some(st) = &self.storage {
            let _ = st.set_setting("restore_once", "1");
        }
        let _ = std::fs::remove_dir_all(crate::storage::data_dir().join("handoff"));
        // `/Q` makes the installer run quietly and start the new version when it is done.
        if std::process::Command::new(p).arg("/Q").spawn().is_ok() {
            self.exit = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_json(tag: &str) -> Value {
        json!({
            "tag_name": tag,
            "body": "Notes",
            "assets": [
                {"name": "Axomai-Setup-4.1.0.exe.sha256", "browser_download_url": "https://github.com/x/y/releases/download/v4.1.0/Axomai-Setup-4.1.0.exe.sha256"},
                {"name": "Axomai-Setup-4.1.0.exe", "browser_download_url": "https://github.com/x/y/releases/download/v4.1.0/Axomai-Setup-4.1.0.exe"},
                {"name": "source.zip", "browser_download_url": "https://github.com/x/y/archive/v4.1.0.zip"}
            ]
        })
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v4.1.0", "4.0.0"));
        assert!(is_newer("4.10.0", "4.9.9"));
        assert!(!is_newer("4.0.0", "4.0.0"));
        assert!(!is_newer("3.9.9", "4.0.0"));
        assert!(!is_newer("garbage", "4.0.0"));
        assert_eq!(version_tuple("v5"), Some((5, 0, 0)));
        assert_eq!(version_tuple("1.2.3-beta"), Some((1, 2, 3)));
        assert_eq!(version_tuple("1.2.3.4"), None);
    }

    #[test]
    fn release_needs_installer_and_checksum() {
        let r = parse_release(&release_json("v4.1.0")).unwrap();
        assert_eq!(r.version, "4.1.0");
        assert!(r.installer_url.ends_with("Axomai-Setup-4.1.0.exe") && r.sha256_url.ends_with(".exe.sha256"));
        let mut no_sha = release_json("v4.1.0");
        no_sha["assets"].as_array_mut().unwrap().remove(0);
        assert!(parse_release(&no_sha).is_none(), "an update without a published checksum is ignored");
        assert!(parse_release(&release_json("nightly")).is_none());
    }

    #[test]
    fn only_github_or_the_test_server_is_trusted() {
        assert!(trusted_url("https://github.com/a/b/releases/download/v1/x.exe", None));
        assert!(trusted_url("https://objects.githubusercontent.com/x", None));
        assert!(!trusted_url("http://github.com/a", None), "plain http is refused");
        assert!(!trusted_url("https://github.com.evil.org/x", None));
        assert!(!trusted_url("https://evil.org/x", None));
        assert!(trusted_url("http://127.0.0.1:9000/x.exe", Some("http://127.0.0.1:9000/release.json")));
        assert!(!trusted_url("http://127.0.0.1:9001/x.exe", Some("http://127.0.0.1:9000/release.json")));
    }

    #[test]
    fn checksums() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let good = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD  Axomai-Setup.exe\n";
        assert_eq!(parse_sha256_file(good).unwrap(), sha256_hex(b"abc"));
        assert!(parse_sha256_file("nothex  file").is_none());
        assert!(parse_sha256_file("").is_none());
    }
}
