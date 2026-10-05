//! Small OS helpers: base64 decoding, real memory trimming, user folders.

pub fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

/// Result of a memory trim: working-set bytes of the browser and its WebView2 children before / after.
#[derive(Clone, Copy, Debug, Default)]
pub struct TrimResult {
    pub before: u64,
    pub after: u64,
}

impl TrimResult {
    pub fn freed_mb(&self) -> f64 {
        self.before.saturating_sub(self.after) as f64 / (1024.0 * 1024.0)
    }
    pub fn after_mb(&self) -> f64 {
        self.after as f64 / (1024.0 * 1024.0)
    }
}

/// Ask Windows to page out the idle working set of this process and every descendant (the WebView2
/// browser / renderer / GPU processes). This is a real reduction of resident memory, not a cosmetic number.
#[cfg(windows)]
pub fn trim_memory() -> TrimResult {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::*;
    use windows::Win32::System::ProcessStatus::*;
    use windows::Win32::System::Threading::*;

    unsafe {
        let me = std::process::id();
        let mut pids = vec![me];
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut table: Vec<(u32, u32)> = Vec::new();
            let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snap, &mut entry).is_ok() {
                loop {
                    table.push((entry.th32ProcessID, entry.th32ParentProcessID));
                    if Process32NextW(snap, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
            let mut i = 0;
            while i < pids.len() {
                let parent = pids[i];
                for (pid, ppid) in &table {
                    if *ppid == parent && !pids.contains(pid) {
                        pids.push(*pid);
                    }
                }
                i += 1;
            }
        }

        let measure = |pid: u32, trim: bool| -> u64 {
            let rights = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_SET_QUOTA;
            match OpenProcess(rights, false, pid) {
                Ok(h) => {
                    if trim {
                        let _ = EmptyWorkingSet(h);
                    }
                    let mut counters = PROCESS_MEMORY_COUNTERS::default();
                    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                    let ok = GetProcessMemoryInfo(h, &mut counters, counters.cb).is_ok();
                    let _ = CloseHandle(h);
                    if ok { counters.WorkingSetSize as u64 } else { 0 }
                }
                Err(_) => 0,
            }
        };

        let before: u64 = pids.iter().map(|p| measure(*p, false)).sum();
        for p in &pids {
            measure(*p, true);
        }
        let after: u64 = pids.iter().map(|p| measure(*p, false)).sum();
        TrimResult { before, after }
    }
}

#[cfg(not(windows))]
pub fn trim_memory() -> TrimResult {
    TrimResult::default()
}

/// Folder where screenshots are stored: `<Pictures>/Axomai Screenshots` (override with `AXOMAI_SCREENSHOTS_DIR`).
pub fn screenshots_dir() -> std::path::PathBuf {
    if let Some(custom) = std::env::var_os("AXOMAI_SCREENSHOTS_DIR") {
        return std::path::PathBuf::from(custom);
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from);
    home.map(|h| h.join("Pictures").join("Axomai Screenshots"))
        .unwrap_or_else(|| std::path::PathBuf::from("Axomai Screenshots"))
}

/// Folders to look in for files that ship with the browser: next to the program, then in each parent folder
/// (a development build runs from `target/debug`), and last the folder this build was made from.
fn resource_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..6 {
            match dir {
                Some(d) => {
                    roots.push(d.clone());
                    dir = d.parent().map(|p| p.to_path_buf());
                }
                None => break,
            }
        }
    }
    roots
}

/// The folder with the browser's own pages (`home.html`, images, styles).
pub fn ui_dir() -> std::path::PathBuf {
    for root in resource_roots() {
        let ui = root.join("ui");
        if ui.join("home.html").is_file() {
            return ui;
        }
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or(std::path::Path::new(".")).join("ui")
}

/// A file from the desktop app's `assets` folder, e.g. `icons/axomai_logo.png`.
pub fn asset_path(rel: &str) -> std::path::PathBuf {
    for root in resource_roots() {
        for base in [root.join("assets"), root.join("rust_desktop").join("assets")] {
            let p = base.join(rel);
            if p.is_file() {
                return p;
            }
        }
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join(rel)
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_round_trips() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"\x00\xff\x10\x80"] {
            assert_eq!(super::base64_decode(&super::base64_encode(data)).unwrap(), data);
        }
        assert_eq!(super::base64_encode(b"Man"), "TWFu");
        assert_eq!(super::base64_encode(b"Ma"), "TWE=");
    }

    #[test]
    fn resources_are_found_from_where_the_program_runs() {
        assert!(super::ui_dir().join("home.html").is_file());
        assert!(super::asset_path("icons/axomai_logo.png").is_file());
    }

    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8gd29ybGQ").unwrap(), b"hello world");
        assert!(base64_decode("a*b").is_none());
    }
}

/// `YYYYMMDD-HHMMSS` in UTC, used for screenshot file names.
pub fn unix_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // days since 1970-01-01 -> civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", y, m, d, rem / 3_600, (rem % 3_600) / 60, rem % 60)
}

#[cfg(test)]
mod time_tests {
    #[test]
    fn timestamp_has_expected_shape() {
        let t = super::timestamp();
        assert_eq!(t.len(), 15);
        assert_eq!(&t[8..9], "-");
        assert!(t.starts_with("20"));
    }
}

/// Plain text currently on the clipboard (for Ctrl+V in the address bar, home search and find bar).
#[cfg(windows)]
pub fn clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() || OpenClipboard(None).is_err() {
            return None;
        }
        let result = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
            let global = HGLOBAL(handle.0);
            let ptr = GlobalLock(global) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0usize;
            while len < 100_000 && *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            let _ = GlobalUnlock(global);
            Some(text)
        })();
        let _ = CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
pub fn clipboard_text() -> Option<String> {
    None
}

/// Put plain text on the clipboard (Ctrl+C / Ctrl+X in the address bar).
#[cfg(windows)]
pub fn set_clipboard_text(text: &str) {
    use windows::Win32::Foundation::{HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        if OpenClipboard(None).is_err() {
            return;
        }
        let _ = EmptyClipboard();
        if let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2) {
            let ptr = GlobalLock(mem) as *mut u16;
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                let _ = GlobalUnlock(mem);
                let _ = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(mem.0)));
            }
            let _ = HGLOBAL(mem.0);
        }
        let _ = CloseClipboard();
    }
}

#[cfg(not(windows))]
pub fn set_clipboard_text(_text: &str) {}
