#[derive(Clone, Copy, PartialEq)]
pub enum SearchEngine {
    Google,
    Bing,
    Yahoo,
    DuckDuckGo,
}

impl SearchEngine {
    pub fn name(&self) -> &'static str {
        match self {
            SearchEngine::Google => "Google",
            SearchEngine::Bing => "Bing",
            SearchEngine::Yahoo => "Yahoo",
            SearchEngine::DuckDuckGo => "DuckDuckGo",
        }
    }
    pub fn search_url(&self, query: &str) -> String {
        let encoded: String = query.bytes().map(|b| {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                b' ' => "+".to_string(),
                _ => format!("%{:02X}", b),
            }
        }).collect();
        match self {
            SearchEngine::Google => format!("https://www.google.com/search?q={}", encoded),
            SearchEngine::Bing => format!("https://www.bing.com/search?q={}", encoded),
            SearchEngine::Yahoo => format!("https://search.yahoo.com/search?p={}", encoded),
            SearchEngine::DuckDuckGo => format!("https://html.duckduckgo.com/html/?q={}", encoded),
        }
    }
    pub fn js_search_template(&self) -> &'static str {
        match self {
            SearchEngine::Google => "https://www.google.com/search?q=",
            SearchEngine::Bing => "https://www.bing.com/search?q=",
            SearchEngine::Yahoo => "https://search.yahoo.com/search?p=",
            SearchEngine::DuckDuckGo => "https://html.duckduckgo.com/html/?q=",
        }
    }
    pub fn all() -> &'static [SearchEngine] {
        &[SearchEngine::Google, SearchEngine::Bing, SearchEngine::Yahoo, SearchEngine::DuckDuckGo]
    }
}

pub struct Extension {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub icon_letter: &'static str,
    pub icon_color: [u8; 3],
    pub enabled: bool,
}

pub const SIDEBAR_W: f32 = 0.0;
pub const TAB_BAR_H: f32 = 44.0;
pub const TOOLBAR_H: f32 = 48.0;
pub const CHROME_TOP: f32 = TAB_BAR_H + TOOLBAR_H;
