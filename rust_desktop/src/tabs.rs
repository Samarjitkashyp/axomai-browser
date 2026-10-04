//! Browser tabs. Every tab owns its own web view, so switching tabs keeps scroll position, form contents and
//! back/forward history. Only the active tab's view is shown; hidden ones are suspended after a while.

use crate::web::{com::DocScript, WebShared};
use std::time::Instant;
use wry::WebView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Home,
    Extensions,
    Settings,
    /// Generated pages: "history", "bookmarks", "downloads", ...
    Page(&'static str),
    About,
    Source,
    Web,
}

impl TabKind {
    /// Which kind of page an address stands for.
    pub fn from_address(address: &str) -> TabKind {
        match address {
            "about:home" | "axomai://home" | "axomai://newtab" | "" => TabKind::Home,
            "axomai://extensions" => TabKind::Extensions,
            "about:settings" | "axomai://settings" => TabKind::Settings,
            "axomai://history" => TabKind::Page("history"),
            "axomai://bookmarks" => TabKind::Page("bookmarks"),
            "axomai://downloads" => TabKind::Page("downloads"),
            "axomai://passwords" => TabKind::Page("passwords"),
            "axomai://permissions" => TabKind::Page("permissions"),
            "axomai://about" => TabKind::About,
            a if a.starts_with("view-source:") => TabKind::Source,
            _ => TabKind::Web,
        }
    }

    /// The address bar text for pages that are not ordinary web addresses.
    pub fn address(&self) -> Option<String> {
        Some(match self {
            TabKind::Home => "about:home".into(),
            TabKind::Extensions => "axomai://extensions".into(),
            TabKind::Settings => "about:settings".into(),
            TabKind::Page(name) => format!("axomai://{}", name),
            TabKind::About => "axomai://about".into(),
            TabKind::Source | TabKind::Web => return None,
        })
    }

    pub fn default_title(&self) -> &'static str {
        match self {
            TabKind::Home => "New Tab",
            TabKind::Extensions => "Extensions",
            TabKind::Settings => "Settings",
            TabKind::Page("history") => "History",
            TabKind::Page("bookmarks") => "Bookmarks",
            TabKind::Page("downloads") => "Downloads",
            TabKind::Page("passwords") => "Passwords",
            TabKind::Page("permissions") => "Permissions",
            TabKind::Page(_) => "Axomai",
            TabKind::About => "About Axomai",
            TabKind::Source => "Source",
            TabKind::Web => "New Tab",
        }
    }
}

pub struct Tab {
    pub id: u64,
    pub kind: TabKind,
    pub title: String,
    /// What the address bar shows for this tab.
    pub url: String,
    /// The tab's web view while it is NOT the active tab (the active tab's view lives in `App::webview`).
    pub view: Option<WebView>,
    pub shared: WebShared,
    pub doc: DocScript,
    pub pinned: bool,
    pub private: bool,
    pub favicon: Option<usize>,
    pub audio: bool,
    pub muted: bool,
    pub loading: bool,
    pub suspended: bool,
    pub last_active: Instant,
    /// `Core::ext_generation` this tab's document-start script was built for.
    pub ext_gen: u64,
    /// For view-source tabs: the finished viewer page once the source has been fetched.
    pub source_html: Option<String>,
    /// Page zoom (1.0 = 100%).
    pub zoom: f64,
    /// What the tab showed before a typed address, until the new page proves itself with a title (a download
    /// never does, so the tab goes back to this).
    pub prev_nav: Option<(TabKind, String, String)>,
    /// Tab group (`App::groups`).
    pub group: Option<u32>,
}

impl Tab {
    pub fn new(id: u64, address: &str, shared: WebShared) -> Tab {
        let kind = TabKind::from_address(address);
        let url = kind.address().unwrap_or_else(|| address.to_string());
        let private = shared.private;
        Tab {
            id,
            kind,
            title: if matches!(kind, TabKind::Web) { crate::blocklist::host_of(address) } else { kind.default_title().to_string() },
            url,
            view: None,
            shared,
            doc: DocScript::default(),
            pinned: false,
            private,
            favicon: None,
            audio: false,
            muted: false,
            loading: false,
            suspended: false,
            last_active: Instant::now(),
            ext_gen: u64::MAX,
            source_html: None,
            zoom: 1.0,
            prev_nav: None,
            group: None,
        }
    }
}

/// A tab that was closed, kept so Ctrl+Shift+T can bring it back.
#[derive(Clone, Debug)]
pub struct ClosedTab {
    pub address: String,
    pub pinned: bool,
}

pub const CLOSED_LIMIT: usize = 20;

/// Index of the tab to show after closing `closed`, given the list length *after* removal.
pub fn next_active_after_close(active: usize, closed: usize, len_after: usize) -> usize {
    if len_after == 0 {
        return 0;
    }
    if closed < active {
        active - 1
    } else if closed == active {
        // The tab that slid into this position, or the last one when we closed the end.
        active.min(len_after - 1)
    } else {
        active
    }
}

/// Where a dragged tab should go. `pinned_count` is the number of pinned tabs (always the leftmost ones); a tab can
/// not be dragged across that boundary.
pub fn drag_target(from: usize, desired: usize, pinned_count: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let desired = desired.min(len - 1);
    if from < pinned_count {
        desired.min(pinned_count.saturating_sub(1))
    } else {
        desired.max(pinned_count)
    }
}

/// Move `idx` to `to`, returning the new active index when the active tab (or the one it shifted past) moved.
pub fn move_tab<T>(items: &mut Vec<T>, active: usize, idx: usize, to: usize) -> usize {
    if idx >= items.len() || to >= items.len() || idx == to {
        return active;
    }
    let item = items.remove(idx);
    items.insert(to, item);
    if active == idx {
        to
    } else if idx < active && to >= active {
        active - 1
    } else if idx > active && to <= active {
        active + 1
    } else {
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_kinds() {
        assert_eq!(TabKind::from_address("about:home"), TabKind::Home);
        assert_eq!(TabKind::from_address("https://example.com"), TabKind::Web);
        assert_eq!(TabKind::from_address("view-source:https://x.test"), TabKind::Source);
        assert_eq!(TabKind::from_address("axomai://history"), TabKind::Page("history"));
        assert_eq!(TabKind::Home.address().as_deref(), Some("about:home"));
        assert_eq!(TabKind::Web.address(), None);
    }

    #[test]
    fn closing_picks_a_sensible_neighbour() {
        assert_eq!(next_active_after_close(2, 2, 3), 2); // closed the active middle tab -> next one slides in
        assert_eq!(next_active_after_close(3, 3, 3), 2); // closed the last tab -> previous
        assert_eq!(next_active_after_close(2, 0, 3), 1); // closed one before the active tab
        assert_eq!(next_active_after_close(1, 2, 3), 1); // closed one after the active tab
    }

    #[test]
    fn dragging_respects_the_pinned_boundary() {
        assert_eq!(drag_target(3, 0, 2, 5), 2); // unpinned tab cannot enter the pinned zone
        assert_eq!(drag_target(0, 4, 2, 5), 1); // pinned tab cannot leave it
        assert_eq!(drag_target(3, 4, 2, 5), 4);
        assert_eq!(drag_target(1, 99, 0, 5), 4);
    }

    #[test]
    fn move_tab_keeps_the_active_tab_selected() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert_eq!(move_tab(&mut v, 0, 0, 2), 2); // moving the active tab
        assert_eq!(v, vec!['b', 'c', 'a', 'd']);
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert_eq!(move_tab(&mut v, 2, 0, 3), 1); // active tab shifts left when one jumps over it
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert_eq!(move_tab(&mut v, 1, 3, 0), 2); // active tab shifts right
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert_eq!(move_tab(&mut v, 0, 2, 3), 0); // unaffected
    }
}
