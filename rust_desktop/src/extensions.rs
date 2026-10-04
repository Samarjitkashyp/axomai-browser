use crate::types::Extension;

pub const ADBLOCK: usize = 0;
pub const READER: usize = 1;
pub const TRANSLATE: usize = 2;
pub const PRIVACY: usize = 3;
pub const CAPTURE: usize = 4;
pub const BOOSTER: usize = 5;

pub const EMOJI: [&str; 6] = ["🛡️", "📖", "🌐", "🔒", "📸", "⚡"];

/// What clicking the extension does; shown as the button label on the Extensions page.
pub const ACTION_LABEL: [&str; 6] = ["Open shield", "Open reader", "Translate page", "Open shield", "Capture", "Free memory now"];

pub fn create_extensions() -> Vec<Extension> {
    vec![
        Extension {
            name: "EasyList AdBlock Shield",
            description: "Blocks ads and ad networks before they load, and hides leftover ad slots",
            version: "3.5.0",
            icon_letter: "A",
            icon_color: [16, 185, 129],
            enabled: true,
        },
        Extension {
            name: "Reader Mode Pro",
            description: "Strips pages to clean, readable text with adjustable size and Paper / Sepia / Dark themes",
            version: "2.2.0",
            icon_letter: "R",
            icon_color: [60, 130, 60],
            enabled: true,
        },
        Extension {
            name: "Assam Auto-Translate",
            description: "Translates the page into Assamese, Hindi, Bengali and other languages",
            version: "2.1.0",
            icon_letter: "T",
            icon_color: [66, 133, 244],
            enabled: true,
        },
        Extension {
            name: "Anti-Fingerprint Privacy Guard",
            description: "Blocks trackers and adds noise to canvas, WebGL and audio fingerprinting",
            version: "2.0.0",
            icon_letter: "P",
            icon_color: [180, 80, 200],
            enabled: true,
        },
        Extension {
            name: "Screen Capture Studio",
            description: "Capture the visible area, the full page or a selected region as a PNG",
            version: "2.0.0",
            icon_letter: "S",
            icon_color: [245, 158, 11],
            enabled: true,
        },
        Extension {
            name: "Turbo RAM Booster",
            description: "Puts the browser engine in low-memory mode and releases idle memory",
            version: "3.1.0",
            icon_letter: "B",
            icon_color: [239, 68, 68],
            enabled: true,
        },
    ]
}
