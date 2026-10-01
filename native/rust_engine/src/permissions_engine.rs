//! Permissions, Geolocation & Device Sensors Engine for Axomai Browser.
//! Implements W3C Permissions API, Geolocation API, and Sensor APIs.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionName {
    Geolocation,
    Notifications,
    Camera,
    Microphone,
    ClipboardRead,
    ClipboardWrite,
    Midi,
    BackgroundSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionState {
    Granted,
    Prompt,
    Denied,
}

#[derive(Debug, Clone)]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: f64,
    pub heading: Option<f64>,
    pub speed: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GeoPosition {
    pub coords: GeoCoordinates,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DeviceOrientationData {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub absolute: bool,
}

pub struct PermissionsManager {
    pub permissions: HashMap<PermissionName, PermissionState>,
    pub mock_location: GeoPosition,
}

impl PermissionsManager {
    pub fn new() -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(PermissionName::Geolocation, PermissionState::Prompt);
        permissions.insert(PermissionName::Notifications, PermissionState::Prompt);
        permissions.insert(PermissionName::Camera, PermissionState::Prompt);
        permissions.insert(PermissionName::Microphone, PermissionState::Prompt);
        permissions.insert(PermissionName::ClipboardRead, PermissionState::Prompt);
        permissions.insert(PermissionName::ClipboardWrite, PermissionState::Granted);

        PermissionsManager {
            permissions,
            mock_location: GeoPosition {
                coords: GeoCoordinates {
                    latitude: 26.1445,  // Guwahati, Assam default
                    longitude: 91.7362,
                    altitude: Some(55.0),
                    accuracy: 10.0,
                    heading: None,
                    speed: None,
                },
                timestamp_ms: 1700000000000,
            },
        }
    }

    /// navigator.permissions.query({ name: ... })
    pub fn query(&self, name: PermissionName) -> PermissionState {
        self.permissions.get(&name).copied().unwrap_or(PermissionState::Prompt)
    }

    /// Request permission from user prompt
    pub fn request_permission(&mut self, name: PermissionName, grant: bool) -> PermissionState {
        let state = if grant {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        };
        self.permissions.insert(name, state);
        state
    }

    /// navigator.geolocation.getCurrentPosition()
    pub fn get_current_position(&self) -> Result<GeoPosition, String> {
        let state = self.query(PermissionName::Geolocation);
        if state == PermissionState::Granted {
            Ok(self.mock_location.clone())
        } else {
            Err("User denied Geolocation permission".to_string())
        }
    }

    /// window.addEventListener('deviceorientation')
    pub fn get_device_orientation(&self) -> DeviceOrientationData {
        DeviceOrientationData {
            alpha: 0.0,
            beta: 90.0,
            gamma: 0.0,
            absolute: true,
        }
    }
}
