//! Native Theme & Assamese Heritage Design Engine for Axomai Browser.
//! Generates vibrant glassmorphic UI color schemes, Windows Mica/Acrylic vibrancy, and cultural presets.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Dark,
    Light,
    SystemDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeritagePreset {
    KazirangaGreen,
    BrahmaputraBlue,
    BihuGold,
    MugaSilk,
    MajuliSunset,
    CyberpunkNeon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassmorphicEffect {
    Acrylic,
    Mica,
    FrostedGlass,
    Opaque,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary_accent: String,
    pub background: String,
    pub surface: String,
    pub text_main: String,
    pub text_muted: String,
    pub border_color: String,
}

pub struct ThemeEngine {
    pub color_mode: ColorMode,
    pub preset: HeritagePreset,
    pub glass_effect: GlassmorphicEffect,
    pub colors: ThemeColors,
}

impl ThemeEngine {
    pub fn new() -> Self {
        let mut engine = ThemeEngine {
            color_mode: ColorMode::Dark,
            preset: HeritagePreset::BrahmaputraBlue,
            glass_effect: GlassmorphicEffect::Acrylic,
            colors: ThemeColors {
                primary_accent: "#00d2ff".to_string(),
                background: "#0a0e17".to_string(),
                surface: "rgba(18, 26, 44, 0.75)".to_string(),
                text_main: "#ffffff".to_string(),
                text_muted: "#94a3b8".to_string(),
                border_color: "rgba(0, 210, 255, 0.25)".to_string(),
            },
        };
        engine.apply_preset(HeritagePreset::BrahmaputraBlue);
        engine
    }

    pub fn apply_preset(&mut self, preset: HeritagePreset) {
        self.preset = preset;
        match preset {
            HeritagePreset::KazirangaGreen => {
                self.colors.primary_accent = "#10b981".to_string(); // Emerald
                self.colors.border_color = "rgba(16, 185, 129, 0.3)".to_string();
            }
            HeritagePreset::BrahmaputraBlue => {
                self.colors.primary_accent = "#00d2ff".to_string(); // Deep River Cyan
                self.colors.border_color = "rgba(0, 210, 255, 0.3)".to_string();
            }
            HeritagePreset::BihuGold => {
                self.colors.primary_accent = "#f59e0b".to_string(); // Festive Gold
                self.colors.border_color = "rgba(245, 158, 11, 0.3)".to_string();
            }
            HeritagePreset::MugaSilk => {
                self.colors.primary_accent = "#eab308".to_string(); // Golden Silk
                self.colors.border_color = "rgba(234, 179, 8, 0.3)".to_string();
            }
            HeritagePreset::MajuliSunset => {
                self.colors.primary_accent = "#f43f5e".to_string(); // Sunset Rose
                self.colors.border_color = "rgba(244, 63, 94, 0.3)".to_string();
            }
            HeritagePreset::CyberpunkNeon => {
                self.colors.primary_accent = "#a855f7".to_string(); // Neon Purple
                self.colors.border_color = "rgba(168, 85, 247, 0.4)".to_string();
            }
        }
    }

    /// Generate dynamic CSS variables for UI injection
    pub fn generate_theme_css(&self) -> String {
        format!(
            ":root {{\n  --ax-primary: {};\n  --ax-bg: {};\n  --ax-surface: {};\n  --ax-text: {};\n  --ax-border: {};\n  --ax-backdrop: blur(16px);\n}}",
            self.colors.primary_accent,
            self.colors.background,
            self.colors.surface,
            self.colors.text_main,
            self.colors.border_color
        )
    }
}
