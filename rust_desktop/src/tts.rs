//! Read a page aloud with the Windows voices (SAPI), independent of the web view.

use crate::app::App;
use crate::web::WebEvent;

pub const MAX_CHARS: usize = 30_000;

/// What the page returns for "read this": the selected text, otherwise the main text of the page.
pub const EXTRACT_JS: &str = "(function(){var s=String(window.getSelection?window.getSelection():'').trim();if(s.length>20)return s;\
var a=document.querySelector('article')||document.querySelector('main,[role=main]')||document.body;return (a&&a.innerText||'').trim()})()";

/// Words per minute multiplier (0.8 .. 1.5) as a SAPI rate between -10 and 10.
pub fn sapi_rate(multiplier: f32) -> i32 {
    (((multiplier.clamp(0.5, 2.0) - 1.0) * 10.0).round() as i32).clamp(-10, 10)
}

/// Windows voices installed here speak English; text in another script would be read as nonsense.
pub fn is_readable(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    !letters.is_empty() && letters.iter().filter(|c| c.is_ascii()).count() * 10 >= letters.len() * 7
}

pub fn clean(text: &str) -> String {
    let t: String = text.chars().filter(|c| !c.is_control() || *c == '\n').take(MAX_CHARS).collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(windows)]
fn spawn_speaker(text: &str, rate: i32) -> Option<std::process::Child> {
    use std::os::windows::process::CommandExt;
    let path = std::env::temp_dir().join("axomai-tts.txt");
    std::fs::write(&path, text).ok()?;
    let script = format!(
        "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Rate={}; try{{ $s.Speak([IO.File]::ReadAllText('{}',[Text.Encoding]::UTF8)) }}catch{{ exit 2 }}",
        rate,
        path.to_string_lossy().replace('\'', "''")
    );
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()
}

#[cfg(not(windows))]
fn spawn_speaker(_text: &str, _rate: i32) -> Option<std::process::Child> {
    None
}

impl App {
    fn toast_tts(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    fn tts_label(&self, on: bool) {
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script(&format!("window.__axTts&&window.__axTts({})", on));
        }
    }

    pub fn tts_stop(&mut self) {
        if let Some(mut c) = self.tts.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.tts_label(false);
    }

    /// Start reading the current page, or stop if it is already being read.
    pub fn tts_toggle(&mut self, rate: f32) {
        if self.tts.is_some() {
            return self.tts_stop();
        }
        self.tts_rate = rate;
        let Some(wv) = &self.webview else { return };
        if !matches!(self.tabs[self.active].kind, crate::tabs::TabKind::Web) {
            return self.toast_tts("Open a web page to read it aloud");
        }
        let s = self.shared();
        let _ = wv.evaluate_script_with_callback(EXTRACT_JS, move |res| s.push_event(WebEvent::PageData("tts-text".into(), String::new(), res)));
    }

    /// The page text arrived (JSON-encoded string).
    pub fn tts_text(&mut self, raw: &str) {
        let text = clean(&serde_json::from_str::<String>(raw).unwrap_or_default());
        if text.len() < 5 {
            return self.toast_tts("There is no text to read on this page");
        }
        if !is_readable(&text) {
            return self.toast_tts("The Windows voices here only speak English; no voice for this language is installed");
        }
        self.tts = spawn_speaker(&text, sapi_rate(self.tts_rate));
        if self.tts.is_some() {
            self.tts_label(true);
        } else {
            self.toast_tts("Could not start the Windows voice");
        }
    }

    /// Called every tick: tell the page when the voice has finished.
    pub fn tts_poll(&mut self) {
        let status = match self.tts.as_mut() {
            Some(c) => c.try_wait().ok().flatten(),
            None => None,
        };
        if let Some(status) = status {
            self.tts = None;
            self.tts_label(false);
            if !status.success() {
                self.toast_tts("Windows could not play the voice. Is an audio output device connected and switched on?");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_map_onto_sapi_range() {
        assert_eq!(sapi_rate(1.0), 0);
        assert_eq!(sapi_rate(1.5), 5);
        assert_eq!(sapi_rate(0.8), -2);
        assert_eq!(sapi_rate(100.0), 10);
        assert_eq!(sapi_rate(0.0), -5);
    }

    #[test]
    fn english_is_readable_other_scripts_are_not() {
        assert!(is_readable("Assam tea is grown along the Brahmaputra."));
        assert!(is_readable("Tea 2024 \u{2014} price \u{20b9}250"));
        assert!(!is_readable("\u{0985}\u{09b8}\u{09ae}\u{09c0}\u{09af}\u{09bc}\u{09be} \u{09ad}\u{09be}\u{09b7}\u{09be}\u{09f0} \u{09b2}\u{09c7}\u{0996}\u{09be}"));
        assert!(!is_readable("123 456"));
    }

    #[test]
    fn text_is_trimmed_and_bounded() {
        assert_eq!(clean("a \n\n b\t c \u{7}"), "a b c");
        assert!(clean(&"x ".repeat(40_000)).len() <= MAX_CHARS);
    }
}
