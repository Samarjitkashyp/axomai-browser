//! Multi-Threaded Download Manager & Range Chunks Engine for Axomai Browser.
//! Supports segmented parallel chunk downloads (Range: bytes=X-Y), pause, resume, and bandwidth estimation.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DownloadChunk {
    pub chunk_id: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub received_byte: u64,
    pub is_completed: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub id: u64,
    pub url: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub received_bytes: u64,
    pub state: DownloadState,
    pub speed_bytes_per_sec: u64,
    pub chunks: Vec<DownloadChunk>,
}

pub struct DownloadManager {
    next_id: u64,
    pub downloads: HashMap<u64, DownloadItem>,
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager {
            next_id: 1,
            downloads: HashMap::new(),
        }
    }

    /// Register and start a new multi-threaded chunked download
    pub fn create_download(&mut self, url: &str, file_name: &str, total_bytes: u64, chunk_count: usize) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut chunks = Vec::new();
        if total_bytes > 0 && chunk_count > 1 {
            let chunk_size = total_bytes / (chunk_count as u64);
            for i in 0..chunk_count {
                let start_byte = (i as u64) * chunk_size;
                let end_byte = if i == chunk_count - 1 {
                    total_bytes - 1
                } else {
                    start_byte + chunk_size - 1
                };
                chunks.push(DownloadChunk {
                    chunk_id: i,
                    start_byte,
                    end_byte,
                    received_byte: 0,
                    is_completed: false,
                });
            }
        }

        let item = DownloadItem {
            id,
            url: url.to_string(),
            file_name: file_name.to_string(),
            total_bytes,
            received_bytes: 0,
            state: DownloadState::Downloading,
            speed_bytes_per_sec: 1024 * 1024 * 5, // 5 MB/s simulated initial bandwidth
            chunks,
        };

        self.downloads.insert(id, item);
        id
    }

    pub fn pause_download(&mut self, id: u64) -> Result<(), String> {
        if let Some(item) = self.downloads.get_mut(&id) {
            item.state = DownloadState::Paused;
            Ok(())
        } else {
            Err(format!("Download {} not found", id))
        }
    }

    pub fn resume_download(&mut self, id: u64) -> Result<(), String> {
        if let Some(item) = self.downloads.get_mut(&id) {
            item.state = DownloadState::Downloading;
            Ok(())
        } else {
            Err(format!("Download {} not found", id))
        }
    }

    pub fn update_progress(&mut self, id: u64, bytes_added: u64) {
        if let Some(item) = self.downloads.get_mut(&id) {
            item.received_bytes = (item.received_bytes + bytes_added).min(item.total_bytes);
            if item.received_bytes >= item.total_bytes && item.total_bytes > 0 {
                item.state = DownloadState::Completed;
            }
        }
    }
}
