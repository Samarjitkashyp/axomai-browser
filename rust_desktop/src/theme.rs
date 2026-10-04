//! Heritage themes. Values mirror the `[data-theme=...]` blocks in `ui/style.css`, so the native chrome,
//! the popups injected into pages and the web UI all share one palette.

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub emoji: &'static str,
    pub dark: bool,
    pub primary: [u8; 3],
    pub primary_light: [u8; 4],
    pub accent: [u8; 3],
    pub titlebar: [u8; 3],
    pub toolbar: [u8; 3],
    pub text_main: [u8; 3],
    pub heading: [u8; 3],
    pub muted: [u8; 3],
    /// Preview swatch for the picker (CSS background).
    pub swatch: &'static str,
}

pub const THEMES: &[Theme] = &[
    Theme {
        id: "tea-garden", name: "Assam Tea Garden", desc: "Lush emerald green & golden sunrise", emoji: "🍵", dark: false,
        primary: [5, 150, 105], primary_light: [236, 253, 245, 255], accent: [245, 158, 11],
        titlebar: [230, 244, 234], toolbar: [253, 255, 253], text_main: [6, 78, 59], heading: [2, 44, 34], muted: [100, 116, 139],
        swatch: "linear-gradient(135deg,#059669,#f59e0b)",
    },
    Theme {
        id: "kaziranga", name: "Kaziranga Mist", desc: "Forest green & wildlife tones", emoji: "🦏", dark: false,
        primary: [21, 128, 61], primary_light: [240, 253, 244, 255], accent: [217, 119, 6],
        titlebar: [229, 239, 233], toolbar: [252, 254, 253], text_main: [20, 83, 45], heading: [5, 46, 22], muted: [100, 116, 139],
        swatch: "linear-gradient(135deg,#15803d,#d97706)",
    },
    Theme {
        id: "brahmaputra", name: "Brahmaputra Azure", desc: "Deep river blue & silver glow", emoji: "🌊", dark: false,
        primary: [2, 132, 199], primary_light: [240, 249, 255, 255], accent: [6, 182, 212],
        titlebar: [224, 240, 254], toolbar: [252, 254, 255], text_main: [12, 74, 110], heading: [8, 47, 73], muted: [100, 116, 139],
        swatch: "linear-gradient(135deg,#0284c7,#06b6d4)",
    },
    Theme {
        id: "bihu-crimson", name: "Gamosa Crimson", desc: "Traditional Assamese red & ivory", emoji: "🧣", dark: false,
        primary: [220, 38, 38], primary_light: [254, 242, 242, 255], accent: [234, 88, 12],
        titlebar: [254, 232, 232], toolbar: [255, 253, 253], text_main: [127, 29, 29], heading: [69, 10, 10], muted: [113, 113, 122],
        swatch: "linear-gradient(135deg,#dc2626,#ea580c)",
    },
    Theme {
        id: "cyber-dark", name: "Obsidian Dark Glass", desc: "Deep OLED black & neon cyan", emoji: "🌌", dark: true,
        primary: [16, 185, 129], primary_light: [22, 50, 55, 255], accent: [56, 189, 248],
        titlebar: [15, 23, 42], toolbar: [20, 29, 48], text_main: [226, 232, 240], heading: [255, 255, 255], muted: [148, 163, 184],
        swatch: "linear-gradient(135deg,#090d16,#1e1b4b)",
    },
];

pub fn by_id(id: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.id == id).unwrap_or(&THEMES[0])
}

impl Theme {
    pub fn index(&self) -> usize {
        THEMES.iter().position(|t| t.id == self.id).unwrap_or(0)
    }

    fn rgb(c: [u8; 3]) -> String {
        format!("rgb({},{},{})", c[0], c[1], c[2])
    }

    /// CSS custom properties used by every injected popup.
    pub fn css_vars(&self) -> String {
        let (bg, border, shadow) = if self.dark {
            ("rgb(15,23,42)", "rgba(255,255,255,0.12)", "0 16px 40px rgba(0,0,0,0.6)")
        } else {
            ("rgb(255,255,255)", "rgba(16,185,129,0.22)", "0 16px 40px rgba(5,150,105,0.18)")
        };
        let hover = if self.dark { "rgba(255,255,255,0.07)" } else { "rgba(0,0,0,0.05)" };
        format!(
            "--primary:{};--primary-light:rgba({},{},{},{});--accent:{};--text:{};--heading:{};--muted:{};--bg:{};--border:{};--shadow:{};--hover:{};",
            Self::rgb(self.primary),
            self.primary_light[0], self.primary_light[1], self.primary_light[2], self.primary_light[3] as f32 / 255.0,
            Self::rgb(self.accent), Self::rgb(self.text_main), Self::rgb(self.heading), Self::rgb(self.muted),
            bg, border, shadow, hover,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_theme_falls_back_to_default() {
        assert_eq!(by_id("nope").id, "tea-garden");
        assert_eq!(by_id("cyber-dark").index(), 4);
    }

    #[test]
    fn every_theme_has_unique_id() {
        for (i, a) in THEMES.iter().enumerate() {
            for b in &THEMES[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
    }
}
