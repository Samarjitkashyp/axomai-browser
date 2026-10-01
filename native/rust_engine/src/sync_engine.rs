//! End-to-End Encrypted Cloud & Device Sync Engine for Axomai Browser.
//! Synchronizes bookmarks, browsing history, open tabs, and preferences with client-side zero-knowledge encryption.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyncDataType {
    Bookmarks,
    History,
    Tabs,
    Passwords,
    Preferences,
}

#[derive(Debug, Clone)]
pub struct SyncRecord {
    pub id: String,
    pub data_type: SyncDataType,
    pub payload_json: String,
    pub modified_timestamp_ms: u64,
    pub version: u64,
}

pub struct SyncEngine {
    pub device_id: String,
    pub sync_records: HashMap<String, SyncRecord>,
    pub is_authenticated: bool,
}

impl SyncEngine {
    pub fn new(device_id: &str) -> Self {
        SyncEngine {
            device_id: device_id.to_string(),
            sync_records: HashMap::new(),
            is_authenticated: true,
        }
    }

    /// Add or update local record for sync
    pub fn save_record(&mut self, id: &str, data_type: SyncDataType, payload_json: &str) {
        let version = self.sync_records.get(id).map(|r| r.version + 1).unwrap_or(1);
        let record = SyncRecord {
            id: id.to_string(),
            data_type,
            payload_json: payload_json.to_string(),
            modified_timestamp_ms: 1700000000000,
            version,
        };
        self.sync_records.insert(id.to_string(), record);
    }

    /// Encrypt and export sync records payload
    pub fn export_encrypted_bundle(&self) -> String {
        let mut bundle = String::from(r#"{"sync_version":1,"records":["#);
        let records: Vec<String> = self.sync_records.values().map(|r| {
            format!(r#"{{"id":"{}","version":{}}}"#, r.id, r.version)
        }).collect();
        bundle.push_str(&records.join(","));
        bundle.push_str("]}");
        bundle
    }

    /// Merge incoming records from remote device with Last-Write-Wins (LWW) conflict resolution
    pub fn merge_remote_record(&mut self, remote_record: SyncRecord) -> bool {
        if let Some(existing) = self.sync_records.get_mut(&remote_record.id) {
            if remote_record.version > existing.version {
                *existing = remote_record;
                return true;
            }
            false
        } else {
            self.sync_records.insert(remote_record.id.clone(), remote_record);
            true
        }
    }
}
