//! User-interface strings. English is the reference; a missing translation falls back to English, and a key that
//! does not exist at all comes back as the key itself so a typo is visible rather than blank.

/// (code, name in its own language)
pub const LANGUAGES: &[(&str, &str)] = &[("en", "English")];

pub fn is_supported(code: &str) -> bool {
    LANGUAGES.iter().any(|(c, _)| *c == code)
}

const EN: &[(&str, &str)] = &[
    ("settings.title", "Settings"),
    ("settings.appearance", "Appearance"),
    ("settings.theme", "Heritage theme"),
    ("settings.theme.desc", "Colours the toolbar, popups and every Axomai page."),
    ("settings.bookmark_bar", "Show bookmarks bar"),
    ("settings.bookmark_bar.desc", "A row of your bookmarks under the toolbar."),
    ("settings.language", "Language"),
    ("settings.language.desc", "Language of the browser's own menus and pages."),
    ("settings.startup", "On startup"),
    ("settings.startup.newtab", "Open the New Tab page"),
    ("settings.startup.restore", "Continue where I left off"),
    ("settings.startup.home", "Open my home page"),
    ("settings.home_url", "Home page"),
    ("settings.home_url.desc", "Used by the Home button. Leave empty to use the New Tab page."),
    ("settings.search", "Search engine"),
    ("settings.search.desc", "Used when you type words in the address bar."),
    ("settings.privacy", "Privacy and security"),
    ("settings.adblock", "Ad blocker"),
    ("settings.adblock.desc", "Blocks ad networks before they load."),
    ("settings.privacy_guard", "Privacy guard"),
    ("settings.privacy_guard.desc", "Blocks trackers and fingerprinting."),
    ("settings.https_only", "Always use secure connections"),
    ("settings.https_only.desc", "Upgrade http:// addresses to https:// and warn when a site has no secure version."),
    ("settings.tracking", "Tracking prevention"),
    ("settings.tracking.desc", "How strictly the web engine blocks cross-site trackers."),
    ("settings.tracking.basic", "Basic"),
    ("settings.tracking.balanced", "Balanced"),
    ("settings.tracking.strict", "Strict"),
    ("settings.passwords", "Offer to save passwords"),
    ("settings.passwords.desc", "Saved logins are encrypted with your Windows account and only filled in on the same site."),
    ("settings.passwords.manage", "Manage"),
    ("settings.gpc", "Send \"Do not sell or share my data\""),
    ("settings.gpc.desc", "Sends the Global Privacy Control signal to every site."),
    ("settings.permissions", "Site permissions"),
    ("settings.permissions.desc", "Camera, microphone, location and notifications you allowed or blocked."),
    ("settings.clear", "Clear browsing data"),
    ("settings.clear.range", "Time range"),
    ("settings.clear.hour", "Last hour"),
    ("settings.clear.day", "Last 24 hours"),
    ("settings.clear.week", "Last 7 days"),
    ("settings.clear.month", "Last 4 weeks"),
    ("settings.clear.all", "All time"),
    ("settings.clear.history", "Browsing history"),
    ("settings.clear.downloads", "Download list"),
    ("settings.clear.cookies", "Cookies and site data"),
    ("settings.clear.cache", "Cached images and files"),
    ("settings.clear.button", "Clear data"),
    ("settings.downloads", "Downloads"),
    ("settings.download_dir", "Download location"),
    ("settings.download_dir.change", "Change"),
    ("settings.ask_download", "Ask where to save each file"),
    ("settings.performance", "Performance"),
    ("settings.sleep", "Put inactive tabs to sleep"),
    ("settings.sleep.desc", "Sleeping tabs free their memory and wake instantly when you open them."),
    ("settings.sleep.never", "Never"),
    ("settings.sleep.minutes", "minutes"),
    ("settings.booster", "Memory booster"),
    ("settings.booster.desc", "Low-memory mode for the web engine, with regular clean-ups."),
    ("settings.default_browser", "Default browser"),
    ("settings.default_browser.desc", "Open web links from other apps in Axomai."),
    ("settings.default_browser.button", "Make Axomai default"),
    ("settings.reset", "Reset settings"),
    ("settings.reset.desc", "Restore every setting on this page to its original value. Bookmarks and history are kept."),
    ("settings.reset.button", "Reset"),
    ("settings.saved", "Saved"),
];

fn table(lang: &str) -> &'static [(&'static str, &'static str)] {
    match lang {
        _ => EN,
    }
}

/// Look `key` up in `lang`, then in English.
pub fn tr(lang: &str, key: &'static str) -> &'static str {
    if let Some((_, v)) = table(lang).iter().find(|(k, _)| *k == key) {
        return v;
    }
    if let Some((_, v)) = EN.iter().find(|(k, _)| *k == key) {
        return v;
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_keys_are_unique() {
        for (i, (a, _)) in EN.iter().enumerate() {
            for (b, _) in &EN[i + 1..] {
                assert_ne!(a, b, "duplicate key");
            }
        }
    }

    #[test]
    fn unknown_language_falls_back_to_english() {
        assert_eq!(tr("zz", "settings.title"), "Settings");
    }

    #[test]
    fn unknown_key_is_returned_as_is() {
        assert_eq!(tr("en", "no.such.key"), "no.such.key");
    }

    #[test]
    fn supported_languages() {
        assert!(is_supported("en"));
        assert!(!is_supported("xx"));
    }
}
