//! User settings: typed values loaded from the `settings` table, validated when changed.

use crate::storage::BrowserStorage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Startup {
    /// Open the New Tab page.
    NewTab,
    /// Reopen the tabs that were open when the browser was closed.
    Restore,
    /// Open the configured home page.
    HomePage,
}

impl Startup {
    pub fn key(&self) -> &'static str {
        match self {
            Startup::NewTab => "newtab",
            Startup::Restore => "restore",
            Startup::HomePage => "home",
        }
    }

    pub fn parse(s: &str) -> Option<Startup> {
        match s {
            "newtab" => Some(Startup::NewTab),
            "restore" => Some(Startup::Restore),
            "home" => Some(Startup::HomePage),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tracking {
    Basic,
    Balanced,
    Strict,
}

impl Tracking {
    pub fn key(&self) -> &'static str {
        match self {
            Tracking::Basic => "basic",
            Tracking::Balanced => "balanced",
            Tracking::Strict => "strict",
        }
    }

    pub fn parse(s: &str) -> Option<Tracking> {
        match s {
            "basic" => Some(Tracking::Basic),
            "balanced" => Some(Tracking::Balanced),
            "strict" => Some(Tracking::Strict),
            _ => None,
        }
    }
}

/// Cities the home page can show the weather for: (key, name).
pub const CITIES: &[(&str, &str)] = &[
    ("jorhat", "Jorhat"),
    ("guwahati", "Guwahati"),
    ("dibrugarh", "Dibrugarh"),
    ("silchar", "Silchar"),
    ("tezpur", "Tezpur"),
    ("tinsukia", "Tinsukia"),
    ("nagaon", "Nagaon"),
    ("sivasagar", "Sivasagar"),
    ("golaghat", "Golaghat"),
    ("north-lakhimpur", "North Lakhimpur"),
    ("dhemaji", "Dhemaji"),
    ("dhubri", "Dhubri"),
    ("bongaigaon", "Bongaigaon"),
    ("kokrajhar", "Kokrajhar"),
    ("goalpara", "Goalpara"),
    ("barpeta", "Barpeta"),
    ("nalbari", "Nalbari"),
    ("mangaldoi", "Mangaldoi"),
    ("diphu", "Diphu"),
    ("haflong", "Haflong"),
    ("karimganj", "Karimganj"),
    ("hailakandi", "Hailakandi"),
    ("shillong", "Shillong"),
    ("itanagar", "Itanagar"),
    ("kohima", "Kohima"),
    ("imphal", "Imphal"),
    ("aizawl", "Aizawl"),
    ("agartala", "Agartala"),
    ("gangtok", "Gangtok"),
    ("kolkata", "Kolkata"),
    ("delhi", "Delhi"),
    ("mumbai", "Mumbai"),
    ("bengaluru", "Bengaluru"),
    ("chennai", "Chennai"),
    ("hyderabad", "Hyderabad"),
];

#[derive(Clone, Debug)]
pub struct Settings {
    pub startup: Startup,
    /// Address opened by the Home button (and at startup when `startup` is `HomePage`); empty = New Tab page.
    pub home_url: String,
    /// Minutes a hidden tab may sit idle before it is put to sleep; 0 = never.
    pub sleep_minutes: u32,
    /// Where downloads go; empty = the system Downloads folder.
    pub download_dir: String,
    /// Ask where to save every file instead of saving straight into `download_dir`.
    pub ask_download: bool,
    pub bookmark_bar: bool,
    pub https_only: bool,
    pub tracking: Tracking,
    /// Send the "Global Privacy Control" signal.
    pub gpc: bool,
    /// Offer to save and fill passwords.
    pub password_manager: bool,
    /// Address-bar search shortcuts you added: (`@keyword`, address with `%s`).
    pub shortcuts: Vec<(String, String)>,
    /// Dark mode for websites.
    pub dark_sites: bool,
    /// Hide cookie-consent banners.
    pub cookie_banners: bool,
    /// Skip YouTube ads and switch autoplay off.
    pub youtube_ads: bool,
    /// Strip menus, ads and comments from printouts.
    pub print_clean: bool,
    /// Key of the city whose weather the home page shows.
    pub weather_city: String,
    /// UI language code ("en", "as", "hi", ...).
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            startup: Startup::NewTab,
            home_url: String::new(),
            sleep_minutes: 5,
            download_dir: String::new(),
            ask_download: false,
            bookmark_bar: false,
            https_only: false,
            tracking: Tracking::Balanced,
            gpc: true,
            password_manager: true,
            shortcuts: Vec::new(),
            dark_sites: false,
            cookie_banners: true,
            youtube_ads: true,
            print_clean: true,
            weather_city: "jorhat".into(),
            language: "en".into(),
        }
    }
}

fn truthy(v: &str) -> Option<bool> {
    match v {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Accept only an absolute http(s) address (or nothing) as a home page.
pub fn valid_home_url(v: &str) -> bool {
    let v = v.trim();
    v.is_empty() || ((v.starts_with("http://") || v.starts_with("https://")) && v.len() < 2048 && !v.contains(char::is_whitespace))
}

impl Settings {
    pub fn load(storage: Option<&BrowserStorage>) -> Settings {
        let mut s = Settings::default();
        let Some(st) = storage else { return s };
        let get = |k: &str| st.get_setting(k).ok().flatten();
        // The old on/off "restore_session" switch still counts if the new key was never written.
        if let Some(v) = get("startup").and_then(|v| Startup::parse(&v)) {
            s.startup = v;
        } else if get("restore_session").as_deref() == Some("true") {
            s.startup = Startup::Restore;
        }
        if let Some(v) = get("home_url").filter(|v| valid_home_url(v)) {
            s.home_url = v;
        }
        if let Some(v) = get("sleep_minutes").and_then(|v| v.parse::<u32>().ok()) {
            s.sleep_minutes = v.min(24 * 60);
        }
        if let Some(v) = get("download_dir") {
            s.download_dir = v;
        }
        if let Some(v) = get("ask_download").and_then(|v| truthy(&v)) {
            s.ask_download = v;
        }
        if let Some(v) = get("bookmark_bar").and_then(|v| truthy(&v)) {
            s.bookmark_bar = v;
        }
        if let Some(v) = get("https_only").and_then(|v| truthy(&v)) {
            s.https_only = v;
        }
        if let Some(v) = get("tracking").and_then(|v| Tracking::parse(&v)) {
            s.tracking = v;
        }
        if let Some(v) = get("weather_city").filter(|v| CITIES.iter().any(|(k, _)| k == v)) {
            s.weather_city = v;
        }
        if let Some(v) = get("shortcuts") {
            s.shortcuts = crate::shortcuts::parse_list(&v);
        }
        if let Some(v) = get("password_manager").and_then(|v| truthy(&v)) {
            s.password_manager = v;
        }
        for (k, slot) in [("dark_sites", &mut s.dark_sites), ("cookie_banners", &mut s.cookie_banners), ("youtube_ads", &mut s.youtube_ads), ("print_clean", &mut s.print_clean)] {
            if let Some(v) = get(k).and_then(|v| truthy(&v)) {
                *slot = v;
            }
        }
        if let Some(v) = get("gpc").and_then(|v| truthy(&v)) {
            s.gpc = v;
        }
        if let Some(v) = get("language").filter(|v| crate::i18n::is_supported(v)) {
            s.language = v;
        }
        s
    }

    /// Change one setting from its string form. Returns false (and changes nothing) for unknown keys or bad values.
    pub fn tweaks(&self) -> crate::site_tweaks::Tweaks {
        crate::site_tweaks::Tweaks { dark: self.dark_sites, cookies: self.cookie_banners, youtube: self.youtube_ads, print: self.print_clean }
    }

    pub fn set(&mut self, storage: Option<&BrowserStorage>, key: &str, value: &str) -> bool {
        let ok = match key {
            "startup" => Startup::parse(value).map(|v| self.startup = v).is_some(),
            "home_url" => {
                let v = value.trim();
                if valid_home_url(v) {
                    self.home_url = v.to_string();
                    true
                } else {
                    false
                }
            }
            "sleep_minutes" => value.parse::<u32>().ok().filter(|v| *v <= 24 * 60).map(|v| self.sleep_minutes = v).is_some(),
            "download_dir" => {
                let v = value.trim();
                if v.len() < 1024 && !v.contains('\0') {
                    self.download_dir = v.to_string();
                    true
                } else {
                    false
                }
            }
            "ask_download" => truthy(value).map(|v| self.ask_download = v).is_some(),
            "bookmark_bar" => truthy(value).map(|v| self.bookmark_bar = v).is_some(),
            "https_only" => truthy(value).map(|v| self.https_only = v).is_some(),
            "tracking" => Tracking::parse(value).map(|v| self.tracking = v).is_some(),
            "dark_sites" => truthy(value).map(|v| self.dark_sites = v).is_some(),
            "cookie_banners" => truthy(value).map(|v| self.cookie_banners = v).is_some(),
            "youtube_ads" => truthy(value).map(|v| self.youtube_ads = v).is_some(),
            "print_clean" => truthy(value).map(|v| self.print_clean = v).is_some(),
            "gpc" => truthy(value).map(|v| self.gpc = v).is_some(),
            "weather_city" => CITIES.iter().any(|(k, _)| *k == value).then(|| self.weather_city = value.to_string()).is_some(),
            "password_manager" => truthy(value).map(|v| self.password_manager = v).is_some(),
            "language" => {
                if crate::i18n::is_supported(value) {
                    self.language = value.to_string();
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if ok {
            if let Some(st) = storage {
                let stored = match key {
                    "startup" => self.startup.key().to_string(),
                    "tracking" => self.tracking.key().to_string(),
                    _ => value.trim().to_string(),
                };
                let _ = st.set_setting(key, &stored);
            }
        }
        ok
    }

    pub fn sleep_after(&self) -> Option<std::time::Duration> {
        if let Some(secs) = std::env::var("AXOMAI_SLEEP_SECS").ok().and_then(|v| v.parse::<u64>().ok()) {
            return Some(std::time::Duration::from_secs(secs));
        }
        if self.sleep_minutes == 0 {
            None
        } else {
            Some(std::time::Duration::from_secs(self.sleep_minutes as u64 * 60))
        }
    }

    /// Address the Home button opens.
    pub fn home_address(&self) -> String {
        if self.home_url.is_empty() {
            "about:home".into()
        } else {
            self.home_url.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_weather_city_is_known_to_the_home_page() {
        let home = include_str!("../../ui/home.html");
        for (key, name) in CITIES {
            let quoted = format!("'{}'", name);
            assert!(home.contains(&quoted), "home.html has no coordinates for {}", name);
            assert!(home.contains(&format!("{}:[", key)) || home.contains(&format!("'{}':[", key)), "home.html has no entry for key {}", key);
        }
        let mut keys: Vec<&str> = CITIES.iter().map(|c| c.0).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), CITIES.len(), "duplicate city keys");
    }

    #[test]
    fn defaults_are_sensible() {
        let s = Settings::default();
        assert_eq!(s.startup, Startup::NewTab);
        assert_eq!(s.sleep_after(), Some(std::time::Duration::from_secs(300)));
        assert_eq!(s.home_address(), "about:home");
    }

    #[test]
    fn set_validates_values() {
        let mut s = Settings::default();
        assert!(s.set(None, "startup", "restore"));
        assert_eq!(s.startup, Startup::Restore);
        assert!(!s.set(None, "startup", "bogus"));
        assert_eq!(s.startup, Startup::Restore);
        assert!(s.set(None, "home_url", "https://example.com"));
        assert_eq!(s.home_address(), "https://example.com");
        assert!(!s.set(None, "home_url", "javascript:alert(1)"));
        assert!(!s.set(None, "home_url", "https://exa mple.com"));
        assert_eq!(s.home_url, "https://example.com");
        assert!(s.set(None, "sleep_minutes", "0"));
        assert_eq!(s.sleep_after(), None);
        assert!(!s.set(None, "sleep_minutes", "99999"));
        assert!(!s.set(None, "sleep_minutes", "-1"));
        assert!(s.set(None, "https_only", "1"));
        assert!(s.https_only);
        assert!(!s.set(None, "https_only", "maybe"));
        assert!(s.set(None, "tracking", "strict"));
        assert_eq!(s.tracking, Tracking::Strict);
        assert!(!s.set(None, "nonsense", "1"));
    }

    #[test]
    fn home_url_rules() {
        assert!(valid_home_url(""));
        assert!(valid_home_url("http://localhost:3000/a"));
        assert!(!valid_home_url("file:///C:/x"));
        assert!(!valid_home_url("data:text/html,x"));
    }
}
