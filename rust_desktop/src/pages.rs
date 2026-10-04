//! History, bookmarks, downloads and extensions pages, built on the shared themed frame.

use crate::extensions as ex;
use crate::storage::{Bookmark, DownloadEntry, HistoryEntry};
use crate::types::Extension;
use crate::ui_shell::{self, PageCtx};
use crate::viewsource::escape;

pub fn format_bytes(b: i64) -> String {
    if b <= 0 {
        return "\u{2014}".into();
    }
    let b = b as f64;
    if b < 1024.0 {
        format!("{} B", b as i64)
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", b / 1024.0 / 1024.0 / 1024.0)
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n).collect();
        t.push('\u{2026}');
        t
    }
}

/// Only plain web addresses are made clickable; anything else is shown as text.
fn link_attr(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        format!(" onclick=\"location.href={}\"", serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into()).replace('"', "&quot;"))
    } else {
        String::new()
    }
}

const FILTER_JS: &str = r#"
var q=document.getElementById('q');
if(q)q.addEventListener('input',function(){var v=q.value.toLowerCase();document.querySelectorAll('.item').forEach(function(e){e.style.display=e.textContent.toLowerCase().indexOf(v)>=0?'':'none'})});
"#;

pub fn history_page(ctx: &PageCtx, entries: &[HistoryEntry]) -> String {
    let mut rows = String::new();
    for e in entries {
        let title = if e.title.is_empty() { &e.url } else { &e.title };
        rows.push_str(&format!(
            "<div class=\"item\"{click}><div class=\"ic\">\u{1F552}</div><div class=\"t\"><b>{title}</b><span>{url}</span></div><div class=\"m\">{time}</div>\
             <button class=\"x\" title=\"Remove from history\" onclick=\"event.stopPropagation();go('history-delete/{id}')\">\u{2715}</button></div>",
            click = link_attr(&e.url),
            title = escape(&truncate(title, 90)),
            url = escape(&truncate(&e.url, 100)),
            time = escape(&e.visit_time),
            id = e.id
        ));
    }
    if rows.is_empty() {
        rows.push_str("<div class=\"empty\">No browsing history yet.</div>");
    }
    let body = format!(
        "{head}<div class=\"bar\"><input type=\"search\" id=\"q\" placeholder=\"Search history\" style=\"flex:1\"><button class=\"btn danger\" onclick=\"if(confirm('Clear all browsing history?'))go('clear-history')\">Clear all</button></div><div class=\"card\">{rows}</div>",
        head = ui_shell::heading("History", "Pages you have visited"),
        rows = rows
    );
    ui_shell::page(ctx, "history", "History", &body, FILTER_JS)
}

pub fn bookmarks_page(ctx: &PageCtx, bookmarks: &[Bookmark]) -> String {
    let mut rows = String::new();
    for b in bookmarks {
        let title = if b.title.is_empty() { &b.url } else { &b.title };
        rows.push_str(&format!(
            "<div class=\"item\"{click}><div class=\"ic\">\u{2B50}</div><div class=\"t\"><b>{title}</b><span>{url}</span></div><div class=\"m\">{folder}</div>\
             <button class=\"x\" title=\"Remove bookmark\" onclick=\"event.stopPropagation();go('remove-bookmark/{id}')\">\u{2715}</button></div>",
            click = link_attr(&b.url),
            title = escape(&truncate(title, 90)),
            url = escape(&truncate(&b.url, 100)),
            folder = escape(&b.folder),
            id = b.id
        ));
    }
    if rows.is_empty() {
        rows.push_str("<div class=\"empty\">No bookmarks yet. Press the star in the address bar to add one.</div>");
    }
    let body = format!(
        "{head}<div class=\"bar\"><input type=\"search\" id=\"q\" placeholder=\"Search bookmarks\" style=\"flex:1\"></div><div class=\"card\">{rows}</div>",
        head = ui_shell::heading("Bookmarks", "Pages you saved"),
        rows = rows
    );
    ui_shell::page(ctx, "bookmarks", "Bookmarks", &body, FILTER_JS)
}

/// Live numbers for a running download: (received, total, paused).
pub type Live = std::collections::HashMap<i64, (i64, i64, bool)>;

const DOWNLOADS_JS: &str = r#"
function fmt(b){if(b<=0)return '\u2014';if(b<1024)return b+' B';if(b<1048576)return (b/1024).toFixed(1)+' KB';if(b<1073741824)return (b/1048576).toFixed(1)+' MB';return (b/1073741824).toFixed(2)+' GB'}
window.__dl=function(rows){rows.forEach(function(r){var e=document.querySelector('[data-id="'+r[0]+'"]');if(!e)return;
 var bar=e.querySelector('.bar i'),m=e.querySelector('.m'),p=r[2]>0?Math.min(100,r[1]*100/r[2]):0;
 if(bar)bar.style.width=p+'%';
 if(m)m.textContent=(r[3]?'Paused \u00b7 ':'')+fmt(r[1])+(r[2]>0?' of '+fmt(r[2]):'')}) };
"#;

pub fn downloads_page(ctx: &PageCtx, downloads: &[DownloadEntry], live: &Live) -> String {
    let mut rows = String::new();
    for d in downloads {
        let running = d.status == "downloading" && live.contains_key(&d.id);
        let (class, label) = match d.status.as_str() {
            "completed" => ("ok", "Completed"),
            "downloading" if running => ("", "Downloading"),
            "cancelled" => ("bad", "Cancelled"),
            _ => ("bad", "Failed"),
        };
        let (middle, actions) = if running {
            let (got, total, paused) = live[&d.id];
            let pct = crate::downloads::percent(got, total).unwrap_or(0);
            (
                format!(
                    "<div class=\"bar\" style=\"height:5px;border-radius:3px;background:var(--surface2);margin-top:6px;overflow:hidden\"><i style=\"display:block;height:100%;width:{pct}%;background:var(--primary)\"></i></div>",
                    pct = pct
                ),
                format!(
                    "<button class=\"btn ghost\" onclick=\"go('dl-{}/{}')\">{}</button><button class=\"btn danger\" onclick=\"go('dl-cancel/{}')\">Cancel</button>",
                    if paused { "resume" } else { "pause" },
                    d.id,
                    if paused { "Resume" } else { "Pause" },
                    d.id
                ),
            )
        } else if d.status == "completed" {
            (
                String::new(),
                format!(
                    "<button class=\"btn ghost\" onclick=\"go('dl-open/{id}')\">Open</button><button class=\"btn ghost\" onclick=\"go('dl-show/{id}')\">Show in folder</button>",
                    id = d.id
                ),
            )
        } else {
            (String::new(), String::new())
        };
        let size = if running {
            let (got, total, _) = live[&d.id];
            if total > 0 {
                format!("{} of {}", format_bytes(got), format_bytes(total))
            } else {
                format_bytes(got)
            }
        } else {
            format_bytes(d.size_bytes)
        };
        rows.push_str(&format!(
            "<div class=\"item\" data-id=\"{id}\" style=\"cursor:default\"><div class=\"ic\">\u{2B07}\u{FE0F}</div><div class=\"t\"><b>{name}</b><span>{url}</span>{middle}</div>\
             <div class=\"m\">{size}</div><span class=\"pill {class}\">{label}</span>{actions}\
             <button class=\"x\" title=\"Remove from list\" onclick=\"go('dl-remove/{id}')\">\u{2715}</button></div>",
            id = d.id,
            name = escape(&d.filename),
            url = escape(&truncate(&d.url, 90)),
            middle = middle,
            size = size,
            class = class,
            label = label,
            actions = actions
        ));
    }
    if rows.is_empty() {
        rows.push_str("<div class=\"empty\">No downloads yet.</div>");
    }
    let body = format!(
        "{head}<div class=\"bar\"><span style=\"flex:1\"></span><button class=\"btn danger\" onclick=\"if(confirm('Clear the download list? Files stay on disk.'))go('clear-downloads')\">Clear list</button></div><div class=\"card\">{rows}</div>",
        head = ui_shell::heading("Downloads", "Files you downloaded"),
        rows = rows
    );
    ui_shell::page(ctx, "downloads", "Downloads", &body, DOWNLOADS_JS)
}

pub fn extensions_page(ctx: &PageCtx, extensions: &[Extension]) -> String {
    let mut rows = String::new();
    for (i, e) in extensions.iter().enumerate() {
        rows.push_str(&format!(
            "<div class=\"row\"><div class=\"ic\" style=\"width:38px;height:38px;border-radius:10px;background:rgb({r},{g},{b});color:#fff;display:flex;align-items:center;justify-content:center;font-weight:800\">{letter}</div>\
             <div class=\"l\"><b>{name} <span class=\"pill\">v{version}</span></b><span>{desc}</span></div>\
             <button class=\"btn ghost\" {disabled} onclick=\"go('ext-run/{i}')\">{action}</button>\
             <label class=\"switch\"><input type=\"checkbox\" {checked} onchange=\"go('ext-toggle/{i}')\"><span class=\"s\"></span></label></div>",
            r = e.icon_color[0],
            g = e.icon_color[1],
            b = e.icon_color[2],
            letter = escape(e.icon_letter),
            name = escape(e.name),
            version = escape(e.version),
            desc = escape(e.description),
            disabled = if e.enabled { "" } else { "disabled" },
            action = escape(ex::ACTION_LABEL.get(i).copied().unwrap_or("Open")),
            checked = if e.enabled { "checked" } else { "" },
            i = i
        ));
    }
    let body = format!(
        "{head}<div class=\"card\">{rows}</div>",
        head = ui_shell::heading("Extensions", "Built-in tools. Switch them on or off here or from the puzzle icon in the toolbar."),
        rows = rows
    );
    ui_shell::page(ctx, "extensions", "Extensions", &body, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> PageCtx<'static> {
        PageCtx { theme: crate::theme::by_id("tea-garden"), token: "tok", lang: "en" }
    }

    #[test]
    fn history_rows_are_escaped_and_clickable_only_for_web_urls() {
        let entries = vec![
            HistoryEntry { id: 7, url: "https://example.com/?a=1&b=<x>".into(), title: "<script>alert(1)</script>".into(), visit_time: "2026-10-04 10:00:00".into() },
            HistoryEntry { id: 8, url: "javascript:alert(1)".into(), title: String::new(), visit_time: String::new() },
        ];
        let html = history_page(&ctx(), &entries);
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("go('history-delete/7')"));
        assert_eq!(html.matches("location.href=&quot;").count(), 1, "only the https row is clickable");
        assert!(html.contains("<title>History - Axomai Browser</title>"));
    }

    #[test]
    fn empty_pages_say_so() {
        assert!(bookmarks_page(&ctx(), &[]).contains("No bookmarks yet"));
        assert!(downloads_page(&ctx(), &[], &Live::new()).contains("No downloads yet"));
    }

    #[test]
    fn running_download_shows_progress_and_controls() {
        let d = DownloadEntry { id: 4, url: "https://x.test/f.zip".into(), filename: "f.zip".into(), filepath: "C:/f.zip".into(), size_bytes: 0, status: "downloading".into(), started_at: String::new() };
        let mut live = Live::new();
        live.insert(4, (50, 200, false));
        let html = downloads_page(&ctx(), &[d.clone()], &live);
        assert!(html.contains("width:25%"));
        assert!(html.contains("go('dl-pause/4')") && html.contains("go('dl-cancel/4')"));
        live.insert(4, (50, 200, true));
        assert!(downloads_page(&ctx(), &[d], &live).contains("go('dl-resume/4')"));
    }

    #[test]
    fn finished_download_can_be_opened() {
        let d = DownloadEntry { id: 9, url: "u".into(), filename: "f".into(), filepath: "p".into(), size_bytes: 10, status: "completed".into(), started_at: String::new() };
        let html = downloads_page(&ctx(), &[d], &Live::new());
        assert!(html.contains("go('dl-open/9')") && html.contains("go('dl-show/9')"));
    }

    #[test]
    fn byte_formatting() {
        assert_eq!(format_bytes(0), "\u{2014}");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn extension_rows_reflect_state() {
        let mut exts = crate::extensions::create_extensions();
        exts[1].enabled = false;
        let html = extensions_page(&ctx(), &exts);
        assert!(html.contains("go('ext-toggle/1')"));
        assert!(html.matches("checked").count() >= 5);
        assert!(html.contains("disabled"));
    }
}
