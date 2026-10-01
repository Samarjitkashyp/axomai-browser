//! Floating Picture-in-Picture (PiP) & 300% Audio Booster for Axomai Browser.
//! Provides an always-on-top detached video playback window with audio equalization and live AI subtitles.

#[derive(Debug, Clone)]
pub struct PipWindowConfig {
    pub is_active: bool,
    pub width: u32,
    pub height: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub always_on_top: bool,
    pub audio_boost_multiplier: f32, // 1.0 (100%) to 3.0 (300%)
    pub playback_speed: f32,
    pub current_subtitle: Option<String>,
}

pub struct PipEngine {
    pub config: PipWindowConfig,
}

impl PipEngine {
    pub fn new() -> Self {
        PipEngine {
            config: PipWindowConfig {
                is_active: false,
                width: 480,
                height: 270,
                pos_x: 100,
                pos_y: 100,
                always_on_top: true,
                audio_boost_multiplier: 1.0,
                playback_speed: 1.0,
                current_subtitle: None,
            },
        }
    }

    pub fn enter_pip(&mut self, width: u32, height: u32) {
        self.config.is_active = true;
        self.config.width = width;
        self.config.height = height;
    }

    pub fn exit_pip(&mut self) {
        self.config.is_active = false;
    }

    pub fn set_audio_boost(&mut self, boost_multiplier: f32) {
        self.config.audio_boost_multiplier = boost_multiplier.clamp(1.0, 3.0);
    }

    pub fn set_playback_speed(&mut self, speed: f32) {
        self.config.playback_speed = speed.clamp(0.25, 4.0);
    }

    pub fn update_subtitles(&mut self, text: &str) {
        self.config.current_subtitle = Some(text.to_string());
    }
}
