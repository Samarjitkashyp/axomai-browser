//! Turns whatever was typed in the address bar (or the home search box) into an action.

/// Internal pages that can be opened by name (`about:settings`, `axomai://history`, ...).
const INTERNAL: &[(&str, &str)] = &[
    ("home", "home"),
    ("newtab", "home"),
    ("settings", "settings"),
    ("themes", "settings"),
    ("extensions", "extensions"),
    ("history", "history"),
    ("bookmarks", "bookmarks"),
    ("downloads", "downloads"),
    ("passwords", "passwords"),
    ("readinglist", "readinglist"),
    ("notes", "notes"),
    ("permissions", "permissions"),
    ("about", "about"),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// Load this URL in the web view.
    Navigate(String),
    /// Open one of the browser's own pages; the value is an `axomai://` command name.
    Internal(&'static str),
    /// `view-source:<http(s) url>`: show the raw HTML of that address.
    ViewSource(String),
}

fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Host part of something that may not have a scheme yet: no userinfo, port, path, query or fragment.
fn host_part(s: &str) -> &str {
    let authority = s.split(&['/', '?', '#'][..]).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    if authority.starts_with('[') {
        return authority;
    }
    authority.split(':').next().unwrap_or("")
}

fn looks_like_domain(host: &str) -> bool {
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') || host.contains("..") {
        return false;
    }
    let tld = host.rsplit('.').next().unwrap_or("");
    let labels_ok = host.split('.').all(|l| !l.is_empty() && l.chars().all(|c| c.is_alphanumeric() || c == '-'));
    labels_ok && tld.chars().count() >= 2 && tld.chars().all(|c| c.is_alphabetic())
}

/// `search` builds the search-engine URL for a query.
pub fn resolve(input: &str, search: impl Fn(&str) -> String) -> Option<Target> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();

    for prefix in ["about:", "axomai://"] {
        if let Some(name) = lower.strip_prefix(prefix) {
            let name = name.trim_end_matches('/');
            if let Some((_, cmd)) = INTERNAL.iter().find(|(n, _)| *n == name) {
                return Some(Target::Internal(cmd));
            }
        }
    }

    if let Some(rest) = lower.strip_prefix("view-source:") {
        let original = &text["view-source:".len()..];
        if rest.starts_with("http://") || rest.starts_with("https://") {
            return Some(Target::ViewSource(original.to_string()));
        }
        return Some(Target::Navigate(search(text)));
    }

    // Anything with a space is a search, even if it also contains a dot ("what is rust.lang").
    if text.chars().any(|c| c.is_whitespace()) {
        return Some(Target::Navigate(search(text)));
    }

    if let Some(idx) = lower.find("://") {
        let scheme = &lower[..idx];
        return Some(match scheme {
            "http" | "https" | "file" | "ftp" => Target::Navigate(text.to_string()),
            // javascript:, data:, custom schemes: never run what was typed, search for it instead.
            _ => Target::Navigate(search(text)),
        });
    }
    if lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("vbscript:") {
        return Some(Target::Navigate(search(text)));
    }

    let host = host_part(&lower);
    if host == "localhost" || is_ipv4(host) || host.ends_with(".localhost") {
        return Some(Target::Navigate(format!("http://{}", text)));
    }
    if looks_like_domain(host) {
        return Some(Target::Navigate(format!("https://{}", text)));
    }
    Some(Target::Navigate(search(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(q: &str) -> String {
        format!("SEARCH:{}", q)
    }

    fn nav(x: &str) -> Option<Target> {
        Some(Target::Navigate(x.to_string()))
    }

    #[test]
    fn words_with_spaces_are_searches() {
        assert_eq!(resolve("digihive assam", s), nav("SEARCH:digihive assam"));
        assert_eq!(resolve("  what is rust.lang  ", s), nav("SEARCH:what is rust.lang"));
    }

    #[test]
    fn single_word_is_a_search() {
        assert_eq!(resolve("assam", s), nav("SEARCH:assam"));
    }

    #[test]
    fn domains_get_https() {
        assert_eq!(resolve("example.com", s), nav("https://example.com"));
        assert_eq!(resolve("en.wikipedia.org/wiki/Assam", s), nav("https://en.wikipedia.org/wiki/Assam"));
        assert_eq!(resolve("example.com:8443/x?y=1", s), nav("https://example.com:8443/x?y=1"));
    }

    #[test]
    fn local_addresses_get_http() {
        assert_eq!(resolve("localhost:3000", s), nav("http://localhost:3000"));
        assert_eq!(resolve("127.0.0.1:8765/index.html", s), nav("http://127.0.0.1:8765/index.html"));
        assert_eq!(resolve("192.168.1.1", s), nav("http://192.168.1.1"));
    }

    #[test]
    fn explicit_schemes_are_kept_but_scripts_are_not_run() {
        assert_eq!(resolve("https://example.com/a b".replace(' ', "%20").as_str(), s), nav("https://example.com/a%20b"));
        assert_eq!(resolve("javascript:alert(1)", s), nav("SEARCH:javascript:alert(1)"));
        assert_eq!(resolve("data:text/html,<b>x</b>", s), nav("SEARCH:data:text/html,<b>x</b>"));
        assert_eq!(resolve("file:///C:/x.html", s), nav("file:///C:/x.html"));
    }

    #[test]
    fn internal_pages_open_by_name() {
        assert_eq!(resolve("about:home", s), Some(Target::Internal("home")));
        assert_eq!(resolve("about:settings", s), Some(Target::Internal("settings")));
        assert_eq!(resolve("axomai://history", s), Some(Target::Internal("history")));
        assert_eq!(resolve("AXOMAI://Downloads/", s), Some(Target::Internal("downloads")));
    }

    #[test]
    fn view_source_addresses() {
        assert_eq!(resolve("view-source:https://example.com/a", s), Some(Target::ViewSource("https://example.com/a".into())));
        assert_eq!(resolve("VIEW-SOURCE:http://127.0.0.1:8765/x.html", s), Some(Target::ViewSource("http://127.0.0.1:8765/x.html".into())));
        assert_eq!(resolve("view-source:javascript:1", s), nav("SEARCH:view-source:javascript:1"));
    }

    #[test]
    fn empty_input_does_nothing() {
        assert_eq!(resolve("   ", s), None);
    }

    #[test]
    fn odd_dots_are_not_domains() {
        assert_eq!(resolve("v1.2", s), nav("SEARCH:v1.2"));
        assert_eq!(resolve("a..b", s), nav("SEARCH:a..b"));
        assert_eq!(resolve("end.", s), nav("SEARCH:end."));
    }
}
