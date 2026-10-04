//! Headlines for the New Tab page from free RSS feeds: Assam newspapers (with their photos) and Google News
//! searches (headlines only). The browser fetches them (the page itself cannot, because of cross-origin rules)
//! and keeps them 15 minutes.

use std::io::Read;

pub struct Feed {
    pub url: &'static str,
    /// Shown as the source when an item does not name its own.
    pub name: &'static str,
}

/// `(key, feeds)`; Assam first.
pub const CATEGORIES: &[(&str, &[Feed])] = &[
    (
        "assam",
        &[
            Feed { url: "https://assamtribune.com/feed", name: "The Assam Tribune" },
            Feed { url: "https://nenow.in/feed", name: "NE Now" },
            Feed { url: "https://news.google.com/rss/search?q=Assam%20when%3A7d&hl=en-IN&gl=IN&ceid=IN:en", name: "Google News" },
        ],
    ),
    (
        "tech",
        &[
            Feed { url: "https://feeds.bbci.co.uk/news/technology/rss.xml", name: "BBC News" },
            Feed { url: "https://news.google.com/rss/search?q=technology%20OR%20artificial%20intelligence%20India%20when%3A3d&hl=en-IN&gl=IN&ceid=IN:en", name: "Google News" },
        ],
    ),
    (
        "wildlife",
        &[
            Feed { url: "https://feeds.bbci.co.uk/news/science_and_environment/rss.xml", name: "BBC News" },
            Feed { url: "https://news.google.com/rss/search?q=Kaziranga%20OR%20Manas%20OR%20%22Assam%20wildlife%22%20when%3A30d&hl=en-IN&gl=IN&ceid=IN:en", name: "Google News" },
        ],
    ),
    (
        "coding",
        &[Feed { url: "https://news.google.com/rss/search?q=%22Rust%20programming%20language%22%20when%3A14d&hl=en-IN&gl=IN&ceid=IN:en", name: "Google News" }],
    ),
];

pub const MAX_ITEMS: usize = 10;
pub const CACHE_SECONDS: u64 = 15 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub title: String,
    pub source: String,
    pub link: String,
    pub published: String,
    /// Photo of the story (https), when the feed has one.
    pub image: String,
    /// Site of the publisher, for its logo when there is no photo.
    pub domain: String,
}

pub fn feeds_for(category: &str) -> Option<&'static [Feed]> {
    CATEGORIES.iter().find(|(k, _)| *k == category).map(|(_, f)| *f)
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
    let mut from = 0;
    loop {
        let open = block[from..].find(&format!("<{}", tag))? + from;
        // `<image>` must not match `<imagecaption>`: the name has to end right after the tag name.
        let name_end = open + 1 + tag.len();
        match block[name_end..].chars().next() {
            Some('>') | Some(' ') | Some('/') => {}
            _ => {
                from = name_end;
                continue;
            }
        }
        let after_open = block[open..].find('>')? + open + 1;
        if block[..after_open].ends_with("/>") {
            return Some(String::new());
        }
        let close = block[after_open..].find(&format!("</{}>", tag))? + after_open;
        let raw = block[after_open..close].trim();
        let raw = raw.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")).unwrap_or(raw);
        return Some(decode_entities(raw).trim().to_string());
    }
}

/// Value of `attr="..."` on the first `<tag ...>` in `block`.
fn tag_attr(block: &str, tag: &str, attr: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = block[from..].find(&format!("<{}", tag)) {
        let open = from + i;
        let end = block[open..].find('>')? + open;
        let head = &block[open..end];
        if let Some(a) = head.find(&format!("{}=\"", attr)) {
            let start = a + attr.len() + 2;
            if let Some(len) = head[start..].find('"') {
                return Some(decode_entities(&head[start..start + len]));
            }
        }
        from = end;
    }
    None
}

fn https(url: &str) -> Option<String> {
    let u = url.trim();
    if u.starts_with("https://") {
        Some(u.to_string())
    } else {
        u.strip_prefix("http://").map(|r| format!("https://{}", r))
    }
}

fn domain_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or("").split(&['/', '?', '#'][..]).next().unwrap_or("").trim_start_matches("www.").to_ascii_lowercase()
}

/// The best photo an item carries: media tags, an enclosure, an `<image>` element, or the first `<img>` in its body.
fn item_image(block: &str) -> String {
    let candidates = [
        tag_attr(block, "media:content", "url"),
        tag_attr(block, "media:thumbnail", "url"),
        tag_attr(block, "enclosure", "url"),
        tag_text(block, "image"),
        tag_text(block, "content:encoded").or_else(|| tag_text(block, "description")).and_then(|body| tag_attr(&body, "img", "src")),
    ];
    for c in candidates.into_iter().flatten() {
        if let Some(u) = https(&c) {
            let lower = u.to_ascii_lowercase();
            // Skip tracking pixels and non-pictures.
            if !u.contains(' ') && !lower.contains("pixel") && (lower.contains(".jpg") || lower.contains(".jpeg") || lower.contains(".png") || lower.contains(".webp") || lower.contains(".gif") || lower.contains("image")) {
                return u;
            }
        }
    }
    String::new()
}

/// Items of an RSS document. Entries without a title or without an http(s) link are dropped.
pub fn parse_rss(xml: &str, max: usize, default_source: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for block in xml.split("<item>").skip(1) {
        let block = block.split("</item>").next().unwrap_or(block);
        let (Some(title), Some(link)) = (tag_text(block, "title"), tag_text(block, "link")) else { continue };
        if title.is_empty() || !(link.starts_with("https://") || link.starts_with("http://")) {
            continue;
        }
        let source = tag_text(block, "source").filter(|s| !s.is_empty()).unwrap_or_else(|| default_source.to_string());
        // Google appends " - Source" to the headline; the source is shown separately.
        let title = match title.rsplit_once(" - ") {
            Some((head, tail)) if tail.trim() == source => head.trim().to_string(),
            _ => title,
        };
        let domain = tag_attr(block, "source", "url").map(|u| domain_of(&u)).filter(|d| !d.is_empty()).unwrap_or_else(|| domain_of(&link));
        items.push(Item { title, source, link, published: tag_text(block, "pubDate").unwrap_or_default(), image: item_image(block), domain });
        if items.len() >= max {
            break;
        }
    }
    items
}

pub fn items_json(items: &[Item]) -> String {
    serde_json::json!({
        "ok": true,
        "items": items.iter().map(|i| serde_json::json!({"t": i.title, "s": i.source, "u": i.link, "d": i.published, "i": i.image, "h": i.domain})).collect::<Vec<_>>(),
    })
    .to_string()
}

pub fn failure_json() -> String {
    serde_json::json!({"ok": false}).to_string()
}

/// Newest first, one entry per headline, stories with a photo kept when two feeds carry the same one.
pub fn merge(mut items: Vec<Item>, max: usize) -> Vec<Item> {
    let mut seen: Vec<String> = Vec::new();
    items.sort_by(|a, b| a.image.is_empty().cmp(&b.image.is_empty()));
    items.retain(|i| {
        let key: String = i.title.to_lowercase().chars().filter(|c| c.is_alphanumeric()).take(40).collect();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    items.truncate(max.max(1) * 2);
    items
}

/// Download one feed. Runs on a helper thread.
fn fetch_feed(feed: &Feed) -> Result<Vec<Item>, String> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(8)).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AxomaiBrowser/1.6").build();
    let resp = agent.get(feed.url).call().map_err(|e| e.to_string())?;
    let mut text = String::new();
    resp.into_reader().take(3_000_000).read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(parse_rss(&text, MAX_ITEMS, feed.name))
}

/// Download every feed of a category; one that fails is skipped. Errors only when none worked.
pub fn fetch(category: &str) -> Result<Vec<Item>, String> {
    let feeds = feeds_for(category).ok_or("unknown category")?;
    let mut all = Vec::new();
    let mut last_error = String::from("no feeds");
    for f in feeds {
        match fetch_feed(f) {
            Ok(items) => all.extend(items),
            Err(e) => last_error = e,
        }
    }
    if all.is_empty() {
        Err(last_error)
    } else {
        Ok(merge(all, MAX_ITEMS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOGLE: &str = r#"<?xml version="1.0"?><rss><channel><title>x</title>
<item><title>Flood waters recede in Dibrugarh &amp; Tinsukia - The Assam Tribune</title><link>https://news.google.com/rss/articles/AAA?oc=5</link><pubDate>Sat, 04 Oct 2026 10:00:00 GMT</pubDate><source url="https://assamtribune.com">The Assam Tribune</source></item>
<item><title><![CDATA[Kaziranga: 3 rhinos <b>born</b> - Hindustan Times]]></title><link>https://news.google.com/rss/articles/BBB</link><pubDate>Sat, 04 Oct 2026 08:00:00 GMT</pubDate><source url="https://www.hindustantimes.com">Hindustan Times</source></item>
<item><title>No link here</title><link>javascript:alert(1)</link></item>
<item><title></title><link>https://x.test</link></item>
<item><title>Plain headline</title><link>http://plain.test/a</link></item>
</channel></rss>"#;

    const TRIBUNE: &str = r#"<rss><channel><image><url>https://assamtribune.com/images/logo.png</url></image>
<item><title><![CDATA[Nagaon bypoll: final pitches]]></title><description><![CDATA[Short]]></description><link>https://assamtribune.com/assam/a-1</link><pubDate>Sun, 04 Oct 2026 12:21:04 GMT</pubDate><imagecaption><![CDATA[caption]]></imagecaption><image><![CDATA[https://assamtribune.com/h-upload/2026/10/04/photo.webp]]></image></item>
<item><title>With body image</title><link>https://nenow.in/b</link><content:encoded><![CDATA[<p>x</p><img src="http://nenow.in/wp/up/pic.jpg" width="3"/>]]></content:encoded></item>
<item><title>Enclosure story</title><link>https://toi.test/c</link><enclosure url="https://toi.test/img/c.jpg" type="image/jpeg" length="1"/></item>
<item><title>Media story</title><link>https://thehindu.test/d</link><media:content url="https://thehindu.test/d.jpg?w=300&amp;h=200" medium="image"/></item>
<item><title>Thumb story</title><link>https://bbc.test/e</link><media:thumbnail width="240" height="135" url="https://ichef.test/e.jpg"/></item>
<item><title>Tracker only</title><link>https://t.test/f</link><description>&lt;img src="https://t.test/pixel.gif"&gt;</description></item>
</channel></rss>"#;

    #[test]
    fn google_items_are_parsed_cleaned_and_filtered() {
        let items = parse_rss(GOOGLE, 8, "Google News");
        assert_eq!(items.len(), 3, "unsafe links and empty titles are dropped");
        assert_eq!(items[0].title, "Flood waters recede in Dibrugarh & Tinsukia", "source suffix removed, entities decoded");
        assert_eq!(items[0].source, "The Assam Tribune");
        assert_eq!(items[0].domain, "assamtribune.com", "publisher site, for its logo");
        assert_eq!(items[1].domain, "hindustantimes.com", "www. is dropped");
        assert!(items[1].title.starts_with("Kaziranga: 3 rhinos <b>born</b>"), "CDATA kept as text");
        assert_eq!(items[2].source, "Google News", "the feed's own name when an item has none");
        assert_eq!(items[0].image, "");
        assert_eq!(parse_rss(GOOGLE, 1, "G").len(), 1);
    }

    #[test]
    fn photos_come_from_every_common_place() {
        let items = parse_rss(TRIBUNE, 20, "Paper");
        assert_eq!(items[0].image, "https://assamtribune.com/h-upload/2026/10/04/photo.webp", "<image> beats <imagecaption>");
        assert_eq!(items[1].image, "https://nenow.in/wp/up/pic.jpg", "first <img> of the body, upgraded to https");
        assert_eq!(items[2].image, "https://toi.test/img/c.jpg", "enclosure");
        assert_eq!(items[3].image, "https://thehindu.test/d.jpg?w=300&h=200", "media:content, entities decoded");
        assert_eq!(items[4].image, "https://ichef.test/e.jpg", "media:thumbnail");
        assert_eq!(items[5].image, "", "tracking pixels are not photos");
        assert_eq!(items[0].domain, "assamtribune.com", "no <source>: the article's own site");
    }

    #[test]
    fn merging_keeps_one_copy_newest_photos_first() {
        let it = |t: &str, img: &str| Item { title: t.into(), source: "S".into(), link: "https://a.test".into(), published: String::new(), image: img.into(), domain: "a.test".into() };
        let merged = merge(vec![it("Same story!", ""), it("Other", ""), it("Same story", "https://img.test/a.jpg")], 10);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].image, "https://img.test/a.jpg", "the copy with the photo wins and comes first");
    }

    #[test]
    fn feeds_exist_for_every_category_and_only_those() {
        for (k, feeds) in CATEGORIES {
            assert!(!feeds.is_empty(), "{}", k);
            assert!(feeds.iter().all(|f| f.url.starts_with("https://")));
        }
        assert!(feeds_for("assam").unwrap().iter().any(|f| f.url.contains("assamtribune")));
        assert!(feeds_for("../../etc").is_none());
    }

    #[test]
    fn entities() {
        assert_eq!(decode_entities("a &amp; b &#39;c&#39; &#x41; &unknown; & alone"), "a & b 'c' A &unknown; & alone");
    }

    #[test]
    fn json_is_valid_and_escaped() {
        let j = items_json(&[Item { title: "He said \"hi\" </script>".into(), source: "S".into(), link: "https://a.test".into(), published: "d".into(), image: "https://i.test/x.jpg".into(), domain: "a.test".into() }]);
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["items"][0]["t"], "He said \"hi\" </script>");
        assert_eq!(v["items"][0]["i"], "https://i.test/x.jpg");
        assert_eq!(v["ok"], true);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&failure_json()).unwrap()["ok"], false);
    }
}
