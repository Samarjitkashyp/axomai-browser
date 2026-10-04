//! Address-bar suggestions: bookmarks and history entries that match what is being typed.

use crate::storage::{Bookmark, HistoryEntry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bookmark,
    History,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: Kind,
    pub title: String,
    pub url: String,
}

/// An address without scheme, `www.` and trailing slash, for matching and de-duplicating.
fn bare(url: &str) -> String {
    let u = url.trim();
    let u = u.strip_prefix("https://").or_else(|| u.strip_prefix("http://")).unwrap_or(u);
    let u = u.strip_prefix("www.").unwrap_or(u);
    u.trim_end_matches('/').to_ascii_lowercase()
}

/// Lower is better; `None` when the entry does not match at all.
fn score(query: &str, url: &str, title: &str) -> Option<u8> {
    let bare_url = bare(url);
    let title = title.to_ascii_lowercase();
    if bare_url.starts_with(query) {
        Some(0)
    } else if title.starts_with(query) {
        Some(1)
    } else if title.split_whitespace().any(|w| w.starts_with(query)) {
        Some(2)
    } else if bare_url.contains(query) || title.contains(query) {
        Some(3)
    } else {
        None
    }
}

/// Up to `max` matches, best first; bookmarks win ties. Only web addresses are suggested.
pub fn build(query: &str, bookmarks: &[Bookmark], history: &[HistoryEntry], max: usize) -> Vec<Suggestion> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(u8, u8, usize, Suggestion)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let candidates = bookmarks
        .iter()
        .map(|b| (Kind::Bookmark, b.url.as_str(), b.title.as_str()))
        .chain(history.iter().map(|h| (Kind::History, h.url.as_str(), h.title.as_str())));
    for (order, (kind, url, title)) in candidates.enumerate() {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        // Search-result pages are noise in a list of places to go.
        if kind == Kind::History && (url.contains("/search?") || url.contains("duckduckgo.com/html")) {
            continue;
        }
        let Some(s) = score(&q, url, title) else { continue };
        if !seen.insert(bare(url)) {
            continue;
        }
        let kind_rank = if kind == Kind::Bookmark { 0 } else { 1 };
        found.push((s, kind_rank, order, Suggestion { kind, title: title.to_string(), url: url.to_string() }));
    }
    found.sort_by_key(|(s, k, o, _)| (*s, *k, *o));
    found.into_iter().take(max).map(|(_, _, _, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bm(url: &str, title: &str) -> Bookmark {
        Bookmark { id: 1, url: url.into(), title: title.into(), folder: String::new(), created_at: String::new() }
    }
    fn hist(url: &str, title: &str) -> HistoryEntry {
        HistoryEntry { id: 1, url: url.into(), title: title.into(), visit_time: String::new() }
    }

    #[test]
    fn empty_query_suggests_nothing() {
        assert!(build("  ", &[bm("https://a.com", "A")], &[], 5).is_empty());
    }

    #[test]
    fn host_prefix_beats_title_substring_and_bookmarks_beat_history() {
        let b = [bm("https://news.example.com/", "Example news")];
        let h = [hist("https://example.org/", "Org"), hist("https://news.example.com", "dup of bookmark"), hist("https://x.com/", "all about example")];
        let r = build("example", &b, &h, 10);
        assert_eq!(r[0].url, "https://example.org/");
        assert_eq!(r.iter().filter(|s| s.url.contains("news.example.com")).count(), 1, "duplicates collapse");
        assert_eq!(r.iter().find(|s| s.url.contains("news.example.com")).unwrap().kind, Kind::Bookmark);
        assert_eq!(r.last().unwrap().url, "https://x.com/");
    }

    #[test]
    fn www_and_scheme_are_ignored_and_limit_applies() {
        let h = [hist("https://www.rust-lang.org/", "Rust"), hist("http://rust.example/", "r"), hist("https://rustup.rs", "")];
        assert_eq!(build("rust", &[], &h, 2).len(), 2);
        assert_eq!(build("www.rust", &[], &h, 5).len(), 0);
    }

    #[test]
    fn search_result_pages_are_not_suggested() {
        assert!(build("cats", &[], &[hist("https://www.google.com/search?q=cats", "cats - Google Search")], 5).is_empty());
    }

    #[test]
    fn non_web_addresses_are_skipped() {
        assert!(build("a", &[bm("javascript:alert(1)", "a")], &[hist("file:///a", "a")], 5).is_empty());
    }
}
