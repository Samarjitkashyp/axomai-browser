//! Voice commands with the Windows speech recogniser (System.Speech): "new tab", "go back", "zoom in", ...
//! The recogniser only listens for the phrases below, runs on this computer and needs a microphone.

use crate::app::App;
use crate::web::WebEvent;

/// `(phrase, command)`. The phrases in the second block are Hindi / Assamese words in English letters; the English
/// recogniser matches them by sound, so they work best with a clear voice.
pub const PHRASES: &[(&str, &str)] = &[
    ("new tab", "newtab"),
    ("open new tab", "newtab"),
    ("close tab", "closetab"),
    ("close this tab", "closetab"),
    ("reopen tab", "reopen-tab"),
    ("next tab", "tab-next"),
    ("previous tab", "tab-prev"),
    ("search tabs", "tab-search"),
    ("new window", "new-window"),
    ("private window", "new-incognito"),
    ("go back", "back"),
    ("go forward", "forward"),
    ("reload page", "reload"),
    ("refresh page", "reload"),
    ("go home", "home"),
    ("zoom in", "zoom-in"),
    ("zoom out", "zoom-out"),
    ("reset zoom", "zoom-reset"),
    ("scroll down", "voice-scroll/down"),
    ("scroll up", "voice-scroll/up"),
    ("go to top", "voice-scroll/top"),
    ("go to bottom", "voice-scroll/bottom"),
    ("find in page", "find"),
    ("full screen", "fullscreen"),
    ("split view", "split-view"),
    ("read aloud", "tts-toggle"),
    ("stop reading", "tts-stop"),
    ("picture in picture", "pip"),
    ("bookmark this page", "bookmark-toggle"),
    ("add to reading list", "reading-add"),
    ("save page as pdf", "save-pdf"),
    ("print page", "print"),
    ("show bookmarks", "bookmarks"),
    ("show history", "history"),
    ("show downloads", "downloads"),
    ("show reading list", "readinglist"),
    ("show notes", "notes"),
    ("show sessions", "sessions"),
    ("open settings", "settings"),
    ("open developer panel", "devpanel"),
    // Hindi / Assamese words in English letters
    ("naya tab kholo", "newtab"),
    ("tab band karo", "closetab"),
    ("peeche jao", "back"),
    ("aage jao", "forward"),
    ("refresh karo", "reload"),
    ("ghar jao", "home"),
    ("bookmark karo", "bookmark-toggle"),
    ("neeche jao", "voice-scroll/down"),
    ("upar jao", "voice-scroll/up"),
    ("nutun tab kholok", "newtab"),
];

/// The command for a recognised phrase.
pub fn command_for(phrase: &str) -> Option<&'static str> {
    let p = phrase.trim().to_ascii_lowercase();
    PHRASES.iter().find(|(ph, _)| *ph == p).map(|(_, c)| *c)
}

/// JavaScript for the scroll commands.
pub fn scroll_js(dir: &str) -> Option<&'static str> {
    Some(match dir {
        "down" => "window.scrollBy({top:Math.round(innerHeight*0.8),behavior:'smooth'})",
        "up" => "window.scrollBy({top:-Math.round(innerHeight*0.8),behavior:'smooth'})",
        "top" => "window.scrollTo({top:0,behavior:'smooth'})",
        "bottom" => "window.scrollTo({top:document.documentElement.scrollHeight,behavior:'smooth'})",
        _ => return None,
    })
}

/// The PowerShell that listens and prints `ready`, `heard:<phrase>` or `error:<message>` lines.
pub fn recogniser_script(phrases_file: &str) -> String {
    format!(
        "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Speech; try {{ \
         $phr = Get-Content -LiteralPath '{}' -Encoding UTF8; \
         $e = New-Object System.Speech.Recognition.SpeechRecognitionEngine([Globalization.CultureInfo]'en-US'); \
         $c = New-Object System.Speech.Recognition.Choices; foreach($p in $phr){{ if($p){{ $c.Add($p) }} }}; \
         $gb = New-Object System.Speech.Recognition.GrammarBuilder; $gb.Culture=[Globalization.CultureInfo]'en-US'; $gb.Append($c); \
         $e.LoadGrammar((New-Object System.Speech.Recognition.Grammar($gb))); \
         $wav=$env:AXOMAI_VOICE_WAV; if($wav){{ $e.SetInputToWaveFile($wav) }} else {{ $e.SetInputToDefaultAudioDevice() }}; \
         [Console]::Out.WriteLine('ready'); [Console]::Out.Flush(); \
         while($true){{ $r = $e.Recognize([TimeSpan]::FromSeconds(5)); \
           if($r -ne $null -and $r.Confidence -ge 0.6){{ [Console]::Out.WriteLine('heard:'+$r.Text); [Console]::Out.Flush() }}; \
           if($wav -and $r -eq $null){{ break }} }} \
         }} catch {{ [Console]::Out.WriteLine('error:'+$_.Exception.Message); [Console]::Out.Flush(); exit 2 }}",
        phrases_file.replace('\'', "''")
    )
}

#[cfg(windows)]
fn spawn_listener(shared: crate::web::WebShared) -> Option<std::process::Child> {
    use std::io::{BufRead, BufReader};
    use std::os::windows::process::CommandExt;
    let file = std::env::temp_dir().join("axomai-voice-phrases.txt");
    let list: Vec<&str> = PHRASES.iter().map(|p| p.0).collect();
    std::fs::write(&file, list.join("\n")).ok()?;
    let mut child = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &recogniser_script(&file.to_string_lossy())])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let out = child.stdout.take()?;
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let (kind, rest) = line.split_once(':').unwrap_or((line.as_str(), ""));
            shared.push_event(WebEvent::PageData("voice".into(), kind.to_string(), rest.to_string()));
        }
        shared.push_event(WebEvent::PageData("voice".into(), "ended".into(), String::new()));
    });
    Some(child)
}

#[cfg(not(windows))]
fn spawn_listener(_shared: crate::web::WebShared) -> Option<std::process::Child> {
    None
}

impl App {
    fn toast_voice(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    pub fn voice_stop(&mut self) {
        if let Some(mut c) = self.voice.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    pub fn voice_toggle(&mut self) {
        if self.voice.is_some() {
            self.voice_stop();
            return self.toast_voice("\u{1F3A4} Voice commands are off");
        }
        let shared = self.shared();
        self.voice = spawn_listener(shared);
        if self.voice.is_none() {
            self.toast_voice("Could not start the Windows speech recogniser");
        }
    }

    /// Lines from the listener: `ready`, `heard`, `error`, `ended`.
    pub fn on_voice_event(&mut self, kind: &str, text: &str) {
        match kind {
            "ready" => self.toast_voice("\u{1F3A4} Listening \u{2014} say \"new tab\", \"go back\", \"zoom in\"\u{2026}"),
            "heard" => self.voice_say(text),
            "error" => {
                self.voice_stop();
                let friendly = if text.contains("Cannot find the requested data item") || text.to_ascii_lowercase().contains("audio") {
                    "No microphone was found. Connect one and switch voice commands on again.".to_string()
                } else {
                    format!("Voice commands stopped: {}", text.chars().take(120).collect::<String>())
                };
                self.toast_voice(&friendly);
            }
            "ended" => {
                // A listener that ended on its own (not through `voice_stop`) is simply forgotten.
                if let Some(c) = self.voice.as_mut() {
                    if c.try_wait().ok().flatten().is_some() {
                        self.voice = None;
                    }
                }
            }
            _ => {}
        }
    }

    /// Run what was said.
    pub fn voice_say(&mut self, phrase: &str) {
        let Some(cmd) = command_for(phrase) else { return };
        self.toast_voice(&format!("\u{1F3A4} {}", phrase.trim()));
        if let Some(dir) = cmd.strip_prefix("voice-scroll/") {
            if let (Some(js), Some(wv)) = (scroll_js(dir), &self.webview) {
                let _ = wv.evaluate_script(js);
            }
            return;
        }
        self.command(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_map_to_commands_ignoring_case() {
        assert_eq!(command_for("New Tab"), Some("newtab"));
        assert_eq!(command_for("  go back "), Some("back"));
        assert_eq!(command_for("naya tab kholo"), Some("newtab"));
        assert_eq!(command_for("delete everything"), None);
    }

    #[test]
    fn phrases_are_unique_and_plain_words() {
        let mut p: Vec<&str> = PHRASES.iter().map(|p| p.0).collect();
        p.sort();
        let n = p.len();
        p.dedup();
        assert_eq!(p.len(), n, "duplicate phrase");
        assert!(PHRASES.iter().all(|(ph, _)| ph.chars().all(|c| c.is_ascii_lowercase() || c == ' ')));
    }

    #[test]
    fn scroll_commands_have_scripts() {
        for (_, cmd) in PHRASES {
            if let Some(dir) = cmd.strip_prefix("voice-scroll/") {
                assert!(scroll_js(dir).is_some(), "{}", cmd);
            }
        }
        assert!(scroll_js("sideways").is_none());
    }

    #[test]
    fn script_quotes_the_phrase_file() {
        let s = recogniser_script("C:\\Users\\o'brien\\p.txt");
        assert!(s.contains("o''brien"));
        assert!(s.contains("SetInputToDefaultAudioDevice"));
    }
}
