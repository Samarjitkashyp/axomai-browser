//! Headlines for the New Tab page from Google News' public RSS feeds (free, no account or key).
//! The browser fetches them (the page itself cannot, because of cross-origin rules) and keeps them 15 minutes.

use std::io::Read;

/// `(key, search words)`; Assam first. `when:7d` keeps the list to the last week.
pub const CATEGORIES: &[(&str, &str)] = &[
    ("assam", "Assam when:7d"),
    ("tech", "technology OR artificial intelligence India when:3d"),
    ("wildlife", "Kaziranga OR Manas OR \"Assam wildlife\" when:30d"),
    ("coding", "\"Rust programming language\" when:14d"),
];

pub const MAX_ITEMS: usize = 8;
pub const CACHE_SECONDS: u64 = 15 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub title: String,
    pub source: String,
    pub link: String,
    pub published: String,
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn feed_url(category: &str) -> Option<String> {
    let (_, query) = CATEGORIES.iter().find(|(k, _)| *k == category)?;
    Some(format!("https://news.google.com/rss/search?q={}&hl=en-IN&gl=IN&ceid=IN:en", percent_encode(query)))
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Text of `<tag>...</tag>` inside `block` (CDATA unwrapped, entities decoded).
fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = block.find(&format!("<{}", tag))?;
    let after_open = block[open..].find('>')? + open + 1;
    let close = block[after_open..].find(&format!("</{}>", tag))? + after_open;
    let raw = block[after_open..close].trim();
    let raw = raw.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")).unwrap_or(raw);
    Some(decode_entities(raw).trim().to_string())
}

/// Items of an RSS document. Entries without a title or without an http(s) link are dropped.
pub fn parse_rss(xml: &str, max: usize) -> Vec<Item> {
    let mut items = Vec::new();
    for block in xml.split("<item>").skip(1) {
        let block = block.split("</item>").next().unwrap_or(block);
        let (Some(title), Some(link)) = (tag_text(block, "title"), tag_text(block, "link")) else { continue };
        if title.is_empty() || !(link.starts_with("https://") || link.starts_with("http://")) {
            continue;
        }
        let source = tag_text(block, "source").unwrap_or_default();
        // Google appends " - Source" to the headline; the source is shown separately.
        let title = match title.rsplit_once(" - ") {
            Some((head, tail)) if !source.is_empty() && tail.trim() == source => head.trim().to_string(),
            _ => title,
        };
        items.push(Item { title, source, link, published: tag_text(block, "pubDate").unwrap_or_default() });
        if items.len() >= max {
            break;
        }
    }
    items
}

pub fn items_json(items: &[Item]) -> String {
    serde_json::json!({
        "ok": true,
        "items": items.iter().map(|i| serde_json::json!({"t": i.title, "s": i.source, "u": i.link, "d": i.published})).collect::<Vec<_>>(),
    })
    .to_string()
}

pub fn failure_json() -> String {
    serde_json::json!({"ok": false}).to_string()
}

/// Download and parse one category. Runs on a helper thread.
pub fn fetch(category: &str) -> Result<Vec<Item>, String> {
    let url = feed_url(category).ok_or("unknown category")?;
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(12)).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AxomaiBrowser/1.6").build();
    let resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    let mut text = String::new();
    resp.into_reader().take(3_000_000).read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(parse_rss(&text, MAX_ITEMS))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?><rss><channel><title>x</title>
<item><title>Flood waters recede in Dibrugarh &amp; Tinsukia - The Assam Tribune</title><link>https://news.google.com/rss/articles/AAA?oc=5</link><pubDate>Sat, 04 Oct 2026 10:00:00 GMT</pubDate><source url="https://assamtribune.com">The Assam Tribune</source></item>
<item><title><![CDATA[Kaziranga: 3 rhinos <b>born</b> - Hindustan Times]]></title><link>https://news.google.com/rss/articles/BBB</link><pubDate>Sat, 04 Oct 2026 08:00:00 GMT</pubDate><source url="https://ht.com">Hindustan Times</source></item>
<item><title>No link here</title><link>javascript:alert(1)</link></item>
<item><title></title><link>https://x.test</link></item>
<item><title>Plain headline</title><link>http://plain.test/a</link></item>
</channel></rss>"#;

    #[test]
    fn items_are_parsed_cleaned_and_filtered() {
        let items = parse_rss(SAMPLE, 8);
        assert_eq!(items.len(), 3, "unsafe links and empty titles are dropped");
        assert_eq!(items[0].title, "Flood waters recede in Dibrugarh & Tinsukia", "source suffix removed, entities decoded");
        assert_eq!(items[0].source, "The Assam Tribune");
        assert_eq!(items[0].published, "Sat, 04 Oct 2026 10:00:00 GMT");
        assert!(items[1].title.starts_with("Kaziranga: 3 rhinos <b>born</b>"), "CDATA kept as text");
        assert_eq!(items[2].source, "");
        assert_eq!(parse_rss(SAMPLE, 1).len(), 1);
    }

    #[test]
    fn urls_are_encoded_and_only_known_categories_work() {
        let u = feed_url("assam").unwrap();
        assert!(u.starts_with("https://news.google.com/rss/search?q=Assam%20when%3A7d&hl=en-IN"));
        assert!(feed_url("wildlife").unwrap().contains("%22Assam%20wildlife%22"));
        assert_eq!(feed_url("../../etc"), None);
    }

    #[test]
    fn entities() {
        assert_eq!(decode_entities("a &amp; b &#39;c&#39; &#x41; &unknown; & alone"), "a & b 'c' A &unknown; & alone");
    }

    #[test]
    fn json_is_valid_and_escaped() {
        let j = items_json(&[Item { title: "He said \"hi\" </script>".into(), source: "S".into(), link: "https://a.test".into(), published: "d".into() }]);
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["items"][0]["t"], "He said \"hi\" </script>");
        assert_eq!(v["ok"], true);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&failure_json()).unwrap()["ok"], false);
    }
}
