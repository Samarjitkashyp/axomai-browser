//! "View page source" (Ctrl+U, `view-source:` addresses, the page context menu).
//!
//! WebView2 does not display `view-source:` pages itself, so the browser fetches the raw HTML and shows it in
//! its own viewer: monospace, line numbers, long lines wrapped, themed like the rest of the chrome.

use crate::web::{WebEvent, WebShared};
use std::io::Read;

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Edg/124.0";

/// Percent-encode a URL so it can travel inside an `axomai://viewsource/<url>` command.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Fetch `url` on a background thread; the result arrives as `PageData("viewsource", url, html)`.
pub fn fetch(url: String, shared: &WebShared) {
    let shared = shared.clone();
    std::thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build();
        let body = match agent.get(&url).call() {
            Ok(resp) => {
                let mut bytes = Vec::new();
                match resp.into_reader().take(MAX_BYTES).read_to_end(&mut bytes) {
                    Ok(_) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                    Err(e) => Err(e.to_string()),
                }
            }
            Err(e) => Err(e.to_string()),
        };
        let html = match body {
            Ok(source) => page(&url, &source, None),
            Err(err) => page(&url, "", Some(&err)),
        };
        shared.push_event(WebEvent::PageData("viewsource".into(), url, html));
    });
}

/// Placeholder shown while the source is being fetched.
pub fn loading_page(url: &str) -> String {
    page(url, "", Some("Loading\u{2026}")).replace("Could not load the source: Loading\u{2026}", "Loading the source\u{2026}")
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The viewer page. Everything from the fetched page is escaped, so the source can never run as script.
pub fn page(url: &str, source: &str, error: Option<&str>) -> String {
    let mut rows = String::new();
    if let Some(e) = error {
        rows.push_str(&format!("<div class=\"err\">Could not load the source: {}</div>", escape(e)));
    } else {
        for (i, line) in source.lines().enumerate() {
            rows.push_str(&format!("<tr><td class=\"n\">{}</td><td class=\"c\">{}</td></tr>", i + 1, escape(line)));
        }
        if rows.is_empty() {
            rows.push_str("<div class=\"err\">The page returned no content.</div>");
        }
    }
    format!(
        r##"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8">
<title>view-source:{title} - Axomai Browser</title>
<style>
:root{{color-scheme:light dark;--bg:#fff;--fg:#1f2937;--muted:#9ca3af;--bar:#f3f4f6;--line:#e5e7eb}}
@media(prefers-color-scheme:dark){{:root{{--bg:#0f172a;--fg:#e2e8f0;--muted:#64748b;--bar:#111c33;--line:#1e293b}}}}
body{{margin:0;background:var(--bg);color:var(--fg);font:13px/1.55 Consolas,"Cascadia Mono",Menlo,monospace}}
.bar{{position:sticky;top:0;background:var(--bar);border-bottom:1px solid var(--line);padding:8px 14px;font:600 12px "Segoe UI",sans-serif;display:flex;gap:12px;align-items:center}}
.bar span{{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1}}
.bar label{{font-weight:500;cursor:pointer;user-select:none}}
table{{border-collapse:collapse;width:100%}}
td.n{{width:1%;min-width:48px;text-align:right;padding:0 12px;color:var(--muted);user-select:none;border-right:1px solid var(--line);vertical-align:top}}
td.c{{padding:0 14px;white-space:pre-wrap;word-break:break-all}}
body.nowrap td.c{{white-space:pre;word-break:normal}}
.err{{padding:24px;font:14px "Segoe UI",sans-serif;color:#dc2626}}
</style></head><body>
<div class="bar"><span>view-source:{url}</span><label><input type="checkbox" id="w" checked> Wrap long lines</label></div>
<table>{rows}</table>
<script>document.getElementById('w').addEventListener('change',function(){{document.body.classList.toggle('nowrap',!this.checked)}});</script>
</body></html>"##,
        title = escape(url),
        url = escape(url),
        rows = rows
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_escaped_so_it_cannot_run() {
        let html = page("https://x.test/?a=1&b=2", "<script>alert(1)</script>\n<p class=\"a\">hi</p>", None);
        assert!(!html.contains("<script>alert(1)"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("a=1&amp;b=2"));
        assert!(html.contains("<td class=\"n\">2</td>"));
    }

    #[test]
    fn errors_are_shown() {
        assert!(page("https://x.test", "", Some("timed out")).contains("Could not load the source: timed out"));
    }

    #[test]
    fn url_round_trips_through_command_encoding() {
        let url = "https://example.com/a b?x=1&y=\u{985}";
        let enc = encode(url);
        assert!(!enc.contains(' ') && !enc.contains('&') && !enc.contains('?'));
        assert!(enc.contains("%E0%A6%85"));
    }
}
