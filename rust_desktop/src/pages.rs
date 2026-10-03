use crate::storage::{HistoryEntry, Bookmark, DownloadEntry};

pub fn history_page_html(entries: &[HistoryEntry]) -> String {
    let mut rows = String::new();
    if entries.is_empty() {
        rows.push_str(r#"<div class="empty">No browsing history yet.</div>"#);
    } else {
        for entry in entries {
            let title_display = if entry.title.is_empty() { &entry.url } else { &entry.title };
            let safe_title = html_escape(title_display);
            let safe_url = html_escape(&entry.url);
            rows.push_str(&format!(
                r##"<div class="item" onclick="window.location.href='{url}'">
  <div class="item-icon">🕒</div>
  <div class="item-content">
    <div class="item-title">{title}</div>
    <div class="item-url">{url_display}</div>
  </div>
  <div class="item-time">{time}</div>
</div>"##,
                url = safe_url,
                title = safe_title,
                url_display = truncate_url(&safe_url, 60),
                time = &entry.visit_time,
            ));
        }
    }

    page_shell("History", "🕒", "Your browsing history", &rows, Some(r#"
<div class="actions">
  <button onclick="if(confirm('Clear all history?'))window.location.href='axomai://clear-history'" class="btn-danger">Clear History</button>
</div>"#))
}

pub fn bookmarks_page_html(bookmarks: &[Bookmark]) -> String {
    let mut rows = String::new();
    if bookmarks.is_empty() {
        rows.push_str(r#"<div class="empty">No bookmarks saved yet.<br>Press Ctrl+D on any page to add a bookmark.</div>"#);
    } else {
        for bm in bookmarks {
            let safe_title = html_escape(if bm.title.is_empty() { &bm.url } else { &bm.title });
            let safe_url = html_escape(&bm.url);
            rows.push_str(&format!(
                r##"<div class="item" onclick="window.location.href='{url}'">
  <div class="item-icon">⭐</div>
  <div class="item-content">
    <div class="item-title">{title}</div>
    <div class="item-url">{url_display}</div>
  </div>
  <div class="item-folder">{folder}</div>
  <button class="btn-remove" onclick="event.stopPropagation();window.location.href='axomai://remove-bookmark/{id}'" title="Remove">✕</button>
</div>"##,
                url = safe_url,
                title = safe_title,
                url_display = truncate_url(&safe_url, 60),
                folder = html_escape(&bm.folder),
                id = bm.id,
            ));
        }
    }

    page_shell("Bookmarks", "⭐", "Your saved bookmarks", &rows, None)
}

pub fn downloads_page_html(downloads: &[DownloadEntry]) -> String {
    let mut rows = String::new();
    if downloads.is_empty() {
        rows.push_str(r#"<div class="empty">No downloads yet.</div>"#);
    } else {
        for dl in downloads {
            let status_class = match dl.status.as_str() {
                "completed" => "status-done",
                "downloading" => "status-active",
                _ => "status-error",
            };
            let size_display = format_bytes(dl.size_bytes);
            rows.push_str(&format!(
                r##"<div class="item">
  <div class="item-icon">⬇️</div>
  <div class="item-content">
    <div class="item-title">{filename}</div>
    <div class="item-url">{url_display}</div>
  </div>
  <div class="item-size">{size}</div>
  <span class="status-badge {status_class}">{status}</span>
</div>"##,
                filename = html_escape(&dl.filename),
                url_display = truncate_url(&html_escape(&dl.url), 50),
                size = size_display,
                status_class = status_class,
                status = html_escape(&dl.status),
            ));
        }
    }

    page_shell("Downloads", "⬇️", "Your download history", &rows, Some(r#"
<div class="actions">
  <button onclick="if(confirm('Clear all downloads?'))window.location.href='axomai://clear-downloads'" class="btn-danger">Clear Downloads</button>
</div>"#))
}

fn page_shell(title: &str, icon: &str, subtitle: &str, content: &str, extra: Option<&str>) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} - Axomai Browser</title>
<style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg:#f8f9fc;--fg:#1a1d2e;--muted:#6b7085;
  --surface:#ffffff;--border:#e4e7f0;
  --accent:#4f46e5;--accent-light:#eef2ff;
  --green:#22c55e;--green-light:#dcfce7;
  --red:#ef4444;--red-light:#fef2f2;
  --radius:14px;--shadow:0 2px 12px rgba(0,0,0,.06);
}}
@media(prefers-color-scheme:dark){{
  :root{{
    --bg:#0c0d14;--fg:#e4e6f0;--muted:#7a7f96;
    --surface:#16171f;--border:#252736;
    --accent:#818cf8;--accent-light:rgba(129,140,248,.12);
    --green:#4ade80;--green-light:rgba(74,222,128,.12);
    --red:#f87171;--red-light:rgba(248,113,113,.12);
    --shadow:0 2px 12px rgba(0,0,0,.3);
    color-scheme:dark;
  }}
}}
body{{
  background:var(--bg);color:var(--fg);
  font-family:'Segoe UI',system-ui,-apple-system,sans-serif;
  padding:32px 28px;-webkit-user-select:none;user-select:none;
}}
.page-header{{margin-bottom:28px;display:flex;align-items:center;gap:14px}}
.page-header .icon{{font-size:28px}}
.page-header div h1{{font-size:24px;font-weight:700;letter-spacing:-.02em;margin-bottom:2px}}
.page-header div p{{color:var(--muted);font-size:14px}}
.item{{
  display:flex;align-items:center;gap:14px;
  padding:14px 18px;margin-bottom:2px;
  border-radius:10px;cursor:pointer;
  transition:background .15s;
}}
.item:hover{{background:var(--accent-light)}}
.item-icon{{font-size:20px;flex-shrink:0;width:32px;text-align:center}}
.item-content{{flex:1;min-width:0}}
.item-title{{font-size:14px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
.item-url{{color:var(--muted);font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
.item-time,.item-folder,.item-size{{color:var(--muted);font-size:12px;flex-shrink:0}}
.btn-remove{{
  background:none;border:none;color:var(--muted);cursor:pointer;
  font-size:14px;padding:4px 8px;border-radius:6px;
  transition:background .15s,color .15s;
}}
.btn-remove:hover{{background:var(--red-light);color:var(--red)}}
.empty{{
  text-align:center;padding:60px 20px;color:var(--muted);
  font-size:15px;line-height:1.8;
}}
.actions{{margin-top:24px;display:flex;gap:12px}}
.btn-danger{{
  background:var(--red-light);color:var(--red);
  border:1px solid transparent;padding:10px 20px;
  border-radius:10px;font-size:13px;font-weight:600;
  cursor:pointer;transition:all .2s;
}}
.btn-danger:hover{{background:var(--red);color:#fff}}
.status-badge{{
  font-size:11px;font-weight:600;padding:4px 10px;
  border-radius:100px;flex-shrink:0;
}}
.status-done{{background:var(--green-light);color:var(--green)}}
.status-active{{background:var(--accent-light);color:var(--accent)}}
.status-error{{background:var(--red-light);color:var(--red)}}
</style>
</head>
<body>
<div class="page-header">
  <span class="icon">{icon}</span>
  <div>
    <h1>{title}</h1>
    <p>{subtitle}</p>
  </div>
</div>
{content}
{extra}
</body>
</html>"##,
        title = title,
        icon = icon,
        subtitle = subtitle,
        content = content,
        extra = extra.unwrap_or(""),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}

fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len])
    }
}

fn format_bytes(bytes: i64) -> String {
    if bytes <= 0 { return "—".to_string(); }
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, units[unit_idx])
    }
}
