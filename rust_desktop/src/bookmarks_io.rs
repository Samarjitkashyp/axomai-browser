//! Importing bookmarks (Chrome's `Bookmarks` file, or a Netscape-style HTML export) and exporting them.

use crate::storage::Bookmark;
use crate::viewsource::escape;

/// Folder that new bookmarks (the star button) land in; it is also what the bookmarks bar shows.
pub const DEFAULT_FOLDER: &str = "Unsorted";
pub const BAR_FOLDER: &str = "Bookmarks bar";

/// A bookmark read from a file: `(folder, title, url)`.
pub type Imported = (String, String, String);

fn is_web(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Chrome's profile bookmarks file for the default profile, if Chrome is installed.
pub fn chrome_bookmarks_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let p = std::path::PathBuf::from(base).join("Google").join("Chrome").join("User Data").join("Default").join("Bookmarks");
    p.is_file().then_some(p)
}

/// Parse Chrome's `Bookmarks` JSON. Nested folders are joined with " / "; the three roots get friendly names.
pub fn parse_chrome_json(text: &str) -> Vec<Imported> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    fn walk(node: &serde_json::Value, path: &str, out: &mut Vec<Imported>) {
        match node.get("type").and_then(|t| t.as_str()) {
            Some("url") => {
                let url = node.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let title = node.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if is_web(url) {
                    out.push((path.to_string(), title.to_string(), url.to_string()));
                }
            }
            Some("folder") => {
                let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let here = if path.is_empty() { name.to_string() } else { format!("{} / {}", path, name) };
                if let Some(kids) = node.get("children").and_then(|c| c.as_array()) {
                    for k in kids {
                        walk(k, &here, out);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(roots) = root.get("roots").and_then(|r| r.as_object()) {
        for (key, label) in [("bookmark_bar", BAR_FOLDER), ("other", "Other bookmarks"), ("synced", "Mobile bookmarks")] {
            if let Some(node) = roots.get(key) {
                if let Some(kids) = node.get("children").and_then(|c| c.as_array()) {
                    for k in kids {
                        walk(k, label, &mut out);
                    }
                }
            }
        }
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&format!("{}=\"", name))? + name.len() + 2;
    let end = tag[at..].find('"')?;
    Some(unescape(&tag[at..at + end]))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

/// Parse a Netscape bookmark file (what Chrome, Firefox and Edge export). `<H3>` opens a folder, `</DL>` closes it.
pub fn parse_netscape_html(text: &str) -> Vec<Imported> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    let mut i = 0;
    while let Some(rel) = lower[i..].find('<') {
        let start = i + rel;
        let Some(end_rel) = lower[start..].find('>') else { break };
        let end = start + end_rel;
        let tag = &text[start + 1..end];
        let tag_lower = &lower[start + 1..end];
        i = end + 1;
        if tag_lower.starts_with("h3") {
            let close = lower[i..].find("</h3>").map(|c| i + c).unwrap_or(text.len());
            pending = Some(unescape(text[i..close].trim()));
            i = close;
        } else if tag_lower.starts_with("dl") {
            if let Some(name) = pending.take() {
                stack.push(name);
            }
        } else if tag_lower.starts_with("/dl") {
            stack.pop();
        } else if tag_lower.starts_with("a ") {
            let close = lower[i..].find("</a>").map(|c| i + c).unwrap_or(text.len());
            let title = unescape(text[i..close].trim());
            if let Some(url) = attr(tag, "href").filter(|u| is_web(u)) {
                let folder = if stack.is_empty() { DEFAULT_FOLDER.to_string() } else { stack.join(" / ") };
                out.push((folder, title, url));
            }
            i = close;
        }
    }
    out
}

/// Export as a Netscape bookmark file, grouped by folder, readable by every browser.
pub fn export_netscape(bookmarks: &[Bookmark]) -> String {
    let mut folders: Vec<&str> = Vec::new();
    for b in bookmarks {
        if !folders.contains(&b.folder.as_str()) {
            folders.push(&b.folder);
        }
    }
    folders.sort();
    let mut html = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n<TITLE>Bookmarks</TITLE>\n<H1>Bookmarks</H1>\n<DL><p>\n",
    );
    for f in folders {
        html.push_str(&format!("    <DT><H3>{}</H3>\n    <DL><p>\n", escape(f)));
        // Oldest first, as they were added.
        for b in bookmarks.iter().filter(|b| b.folder == f).rev() {
            let title = if b.title.is_empty() { &b.url } else { &b.title };
            html.push_str(&format!("        <DT><A HREF=\"{}\">{}</A>\n", escape(&b.url), escape(title)));
        }
        html.push_str("    </DL><p>\n");
    }
    html.push_str("</DL><p>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_json_is_flattened_with_folder_paths() {
        let json = r#"{"roots":{"bookmark_bar":{"children":[
            {"type":"url","name":"Rust","url":"https://rust-lang.org/"},
            {"type":"folder","name":"News","children":[{"type":"url","name":"BBC","url":"https://bbc.com"},{"type":"url","name":"js","url":"javascript:1"}]}
          ],"type":"folder","name":"Bookmarks bar"},
          "other":{"children":[{"type":"url","name":"X","url":"http://x.test"}],"type":"folder","name":"Other bookmarks"}}}"#;
        let r = parse_chrome_json(json);
        assert_eq!(r.len(), 3, "javascript: links are dropped");
        assert_eq!(r[0], (BAR_FOLDER.into(), "Rust".into(), "https://rust-lang.org/".into()));
        assert_eq!(r[1].0, "Bookmarks bar / News");
        assert_eq!(r[2].0, "Other bookmarks");
        assert!(parse_chrome_json("not json").is_empty());
    }

    #[test]
    fn export_then_import_round_trips() {
        let marks = vec![
            Bookmark { id: 2, url: "https://b.test/?a=1&b=2".into(), title: "B <&> site".into(), folder: "Work".into(), created_at: String::new() },
            Bookmark { id: 1, url: "https://a.test".into(), title: "A".into(), folder: DEFAULT_FOLDER.into(), created_at: String::new() },
        ];
        let html = export_netscape(&marks);
        let mut back = parse_netscape_html(&html);
        back.sort();
        assert_eq!(back.len(), 2);
        assert!(back.contains(&("Work".into(), "B <&> site".into(), "https://b.test/?a=1&b=2".into())));
        assert!(back.contains(&(DEFAULT_FOLDER.into(), "A".into(), "https://a.test".into())));
    }

    #[test]
    fn netscape_nesting_and_loose_links() {
        let html = "<DL><p><DT><A HREF=\"https://top.test\">Top</A><DT><H3>One</H3><DL><p><DT><H3>Two</H3><DL><p><DT><A HREF=\"https://deep.test\">Deep</A></DL><p><DT><A HREF=\"https://one.test\">One link</A></DL><p></DL>";
        let r = parse_netscape_html(html);
        assert_eq!(r[0].0, DEFAULT_FOLDER);
        assert_eq!(r[1].0, "One / Two");
        assert_eq!(r[2].0, "One");
    }
}
