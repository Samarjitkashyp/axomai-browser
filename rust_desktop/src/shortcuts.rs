//! Search shortcuts for the address bar: `@yt lofi music` searches YouTube, `@wiki tea` searches Wikipedia.

use crate::app::App;

/// `(keyword, site name, address with %s where the search words go)`.
pub const BUILTIN: &[(&str, &str, &str)] = &[
    ("@yt", "YouTube", "https://www.youtube.com/results?search_query=%s"),
    ("@wiki", "Wikipedia", "https://en.wikipedia.org/w/index.php?search=%s"),
    ("@as", "Assamese Wikipedia", "https://as.wikipedia.org/w/index.php?search=%s"),
    ("@gh", "GitHub", "https://github.com/search?q=%s"),
    ("@maps", "Google Maps", "https://www.google.com/maps/search/%s"),
    ("@news", "Google News", "https://news.google.com/search?q=%s"),
    ("@amazon", "Amazon India", "https://www.amazon.in/s?k=%s"),
    ("@g", "Google", "https://www.google.com/search?q=%s"),
    ("@bing", "Bing", "https://www.bing.com/search?q=%s"),
    ("@ddg", "DuckDuckGo", "https://duckduckgo.com/?q=%s"),
];

pub const MAX_USER_SHORTCUTS: usize = 30;

pub fn valid_keyword(k: &str) -> bool {
    k.len() >= 2 && k.len() <= 13 && k.starts_with('@') && k[1..].chars().all(|c| c.is_ascii_alphanumeric())
}

/// An http(s) address that holds `%s` exactly once and no whitespace.
pub fn valid_template(t: &str) -> bool {
    (t.starts_with("https://") || t.starts_with("http://")) && t.len() < 500 && t.matches("%s").count() == 1 && !t.contains(char::is_whitespace)
}

fn encode(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// The site's front page for a shortcut typed without search words.
fn origin(template: &str) -> String {
    let after = template.find("://").map_or(0, |i| i + 3);
    let end = template[after..].find('/').map_or(template.len(), |i| after + i);
    template[..end].to_string()
}

/// `(address to open, site name)` when `text` starts with a shortcut keyword. Your own shortcuts win over built-in ones.
pub fn expand(text: &str, user: &[(String, String)]) -> Option<(String, String)> {
    let text = text.trim();
    if !text.starts_with('@') {
        return None;
    }
    let (kw, query) = match text.split_once(char::is_whitespace) {
        Some((k, q)) => (k, q.trim()),
        None => (text, ""),
    };
    let kw = kw.to_ascii_lowercase();
    let (name, template) = user
        .iter()
        .find(|(k, _)| *k == kw)
        .map(|(k, t)| (k.clone(), t.clone()))
        .or_else(|| BUILTIN.iter().find(|(k, _, _)| *k == kw).map(|(_, n, t)| (n.to_string(), t.to_string())))?;
    if query.is_empty() {
        return Some((origin(&template), name));
    }
    Some((template.replacen("%s", &encode(query), 1), name))
}

pub fn parse_list(stored: &str) -> Vec<(String, String)> {
    stored
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(k, t)| valid_keyword(k) && valid_template(t))
        .map(|(k, t)| (k.to_string(), t.to_string()))
        .take(MAX_USER_SHORTCUTS)
        .collect()
}

pub fn serialize(list: &[(String, String)]) -> String {
    list.iter().map(|(k, t)| format!("{}\t{}", k, t)).collect::<Vec<_>>().join("\n")
}

impl App {
    pub fn shortcut_add(&mut self, keyword: &str, template: &str) {
        let kw = keyword.trim().to_ascii_lowercase();
        let tpl = template.trim().to_string();
        if !valid_keyword(&kw) || !valid_template(&tpl) {
            return;
        }
        let list = &mut self.settings.shortcuts;
        if let Some(e) = list.iter_mut().find(|(k, _)| *k == kw) {
            e.1 = tpl;
        } else if list.len() < MAX_USER_SHORTCUTS {
            list.push((kw, tpl));
        }
        self.save_shortcuts();
    }

    pub fn shortcut_remove(&mut self, keyword: &str) {
        self.settings.shortcuts.retain(|(k, _)| k != keyword);
        self.save_shortcuts();
    }

    fn save_shortcuts(&mut self) {
        if let Some(s) = &self.storage {
            let _ = s.set_setting("shortcuts", &serialize(&self.settings.shortcuts));
        }
        self.refresh_internal_page();
        self.redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_shortcuts_search_the_site() {
        assert_eq!(expand("@yt lofi music", &[]), Some(("https://www.youtube.com/results?search_query=lofi+music".into(), "YouTube".into())));
        assert_eq!(expand("@WIKI Tea & Coffee", &[]).unwrap().0, "https://en.wikipedia.org/w/index.php?search=Tea+%26+Coffee");
        assert_eq!(expand("@yt", &[]).unwrap().0, "https://www.youtube.com");
    }

    #[test]
    fn unknown_keywords_and_plain_text_are_left_alone() {
        assert_eq!(expand("@nothing hello", &[]), None);
        assert_eq!(expand("hello @yt", &[]), None);
        assert_eq!(expand("user@example.com", &[]), None);
    }

    #[test]
    fn your_shortcuts_override_built_in_ones() {
        let mine = vec![("@yt".to_string(), "https://example.org/?q=%s".to_string())];
        assert_eq!(expand("@yt cats", &mine).unwrap().0, "https://example.org/?q=cats");
    }

    #[test]
    fn keywords_and_templates_are_validated() {
        assert!(valid_keyword("@docs") && !valid_keyword("docs") && !valid_keyword("@") && !valid_keyword("@a b") && !valid_keyword("@waytoolongkeyword"));
        assert!(valid_template("https://a.test/?q=%s"));
        assert!(!valid_template("javascript:%s") && !valid_template("https://a.test/") && !valid_template("https://a.test/%s%s") && !valid_template("https://a.test/ %s"));
    }

    #[test]
    fn list_round_trips_and_drops_bad_lines() {
        let l = vec![("@a".to_string(), "https://a.test/?q=%s".to_string())];
        assert_eq!(parse_list(&serialize(&l)), l);
        assert!(parse_list("@bad\tjavascript:%s\nnotab").is_empty());
    }
}
