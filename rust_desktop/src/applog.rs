//! Log file for release builds. A release build has no console window, so what the program prints (start-up messages,
//! a panic message) would be lost; it is written to `%APPDATA%\AxomaiBrowser\axomai.log` instead. Debug builds
//! (`cargo run`) keep printing to the terminal.

/// Largest the log may grow before it is started afresh at the next launch.
pub const MAX_BYTES: u64 = 512 * 1024;

/// Whether an existing log of this size should be kept (appended to) or replaced.
pub fn keep_existing(size: u64) -> bool {
    size <= MAX_BYTES
}

#[cfg(all(windows, not(debug_assertions)))]
pub fn init() {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    let dir = crate::storage::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("axomai.log");
    let keep = std::fs::metadata(&path).map(|m| keep_existing(m.len())).unwrap_or(true);
    let file = std::fs::OpenOptions::new().create(true).write(true).append(keep).truncate(!keep).open(&path);
    if let Ok(file) = file {
        let handle = HANDLE(file.as_raw_handle() as *mut _);
        unsafe {
            let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
        }
        // The handle must stay open for as long as the program runs.
        std::mem::forget(file);
        eprintln!("---- Axomai Browser {} started ----", env!("CARGO_PKG_VERSION"));
    }
}

#[cfg(not(all(windows, not(debug_assertions))))]
pub fn init() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_big_log_is_replaced_not_appended() {
        assert!(keep_existing(0));
        assert!(keep_existing(MAX_BYTES));
        assert!(!keep_existing(MAX_BYTES + 1));
    }
}
