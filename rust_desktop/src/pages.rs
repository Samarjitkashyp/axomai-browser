//! History, bookmarks, downloads and extensions pages, built on the shared themed frame.

use crate::extensions as ex;
use crate::i18n::tr;
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

/// Replace fixed English labels in generated markup. Each entry is `(needle, English text inside it, key)`: the needle
/// carries enough markup around the text that page content can never match it by accident.
fn loc(lang: &str, mut html: String, pairs: &[(&str, &str, &'static str)]) -> String {
    if lang == "en" {
        return html;
    }
    for (needle, english, key) in pairs {
        html = html.replace(needle, &needle.replace(english, tr(lang, key)));
    }
    html
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

const HISTORY_JS: &str = r#"
var q=document.getElementById('q');
function filt(){var v=q.value.toLowerCase();document.querySelectorAll('.fgroup').forEach(function(g){var any=false;g.querySelectorAll('.item').forEach(function(e){var ok=e.textContent.toLowerCase().indexOf(v)>=0;e.style.display=ok?'':'none';if(ok)any=true});g.style.display=any?'':'none'})}
if(q)q.addEventListener('input',filt);
function delrange(){var r=document.getElementById('range').value;var label=document.getElementById('range').selectedOptions[0].textContent;if(confirm('Delete browsing history for: '+label+'?'))go('clear-data-range/'+r+'/h')}
"#;

pub fn history_page(ctx: &PageCtx, entries: &[HistoryEntry]) -> String {
    // Entries arrive newest first with local "YYYY-MM-DD HH:MM:SS" times; one group per day.
    let mut groups = String::new();
    let mut day = String::new();
    let mut open = false;
    for e in entries {
        let d: String = e.visit_time.chars().take(10).collect();
        if d != day || !open {
            if open {
                groups.push_str("</div></div>");
            }
            groups.push_str(&format!("<div class=\"fgroup\"><div class=\"fh\"><b>{}</b></div><div class=\"card\">", escape(&d)));
            day = d;
            open = true;
        }
        let title = if e.title.is_empty() { &e.url } else { &e.title };
        let time: String = e.visit_time.chars().skip(11).take(5).collect();
        groups.push_str(&format!(
            "<div class=\"item\"{click}><div class=\"ic\">\u{1F552}</div><div class=\"t\"><b>{title}</b><span>{url}</span></div><div class=\"m\">{time}</div>\
             <button class=\"x\" title=\"Remove from history\" onclick=\"event.stopPropagation();go('history-delete/{id}')\">\u{2715}</button></div>",
            click = link_attr(&e.url),
            title = escape(&truncate(title, 90)),
            url = escape(&truncate(&e.url, 100)),
            time = escape(&time),
            id = e.id
        ));
    }
    if open {
        groups.push_str("</div></div>");
    } else {
        groups.push_str(&format!("<div class=\"card\"><div class=\"empty\">{}</div></div>", tr(ctx.lang, "history.empty")));
    }
    let body = format!(
        "{head}<div class=\"bar\"><input type=\"search\" id=\"q\" placeholder=\"Search history\" style=\"flex:1\">\
         <select id=\"range\"><option value=\"hour\">Last hour</option><option value=\"day\">Last 24 hours</option><option value=\"week\">Last 7 days</option><option value=\"month\">Last 4 weeks</option><option value=\"all\" selected>All time</option></select>\
         <button class=\"btn danger\" onclick=\"delrange()\">Delete</button></div>\
         <style>.fh{{display:flex;align-items:center;gap:10px;margin:18px 4px 8px;color:var(--heading)}}\
         select{{border:1px solid var(--border);border-radius:8px;padding:7px 10px;background:var(--surface);color:var(--text);font:inherit}}</style>{groups}",
        head = ui_shell::heading(tr(ctx.lang, "history.title"), tr(ctx.lang, "history.sub")),
        groups = groups
    );
    let body = loc(ctx.lang, body, &[
        ("placeholder=\"Search history\"", "Search history", "history.search"),
        ("onclick=\"delrange()\">Delete<", "Delete", "common.delete"),
        (">Last hour</option>", "Last hour", "settings.clear.hour"),
        (">Last 24 hours</option>", "Last 24 hours", "settings.clear.day"),
        (">Last 7 days</option>", "Last 7 days", "settings.clear.week"),
        (">Last 4 weeks</option>", "Last 4 weeks", "settings.clear.month"),
        (">All time</option>", "All time", "settings.clear.all"),
    ]);
    ui_shell::page(ctx, "history", "History", &body, HISTORY_JS)
}

const BOOKMARKS_JS: &str = r#"
var q=document.getElementById('q');
function filt(){var v=q.value.toLowerCase();document.querySelectorAll('.fgroup').forEach(function(g){var any=false;g.querySelectorAll('.item').forEach(function(e){var ok=e.textContent.toLowerCase().indexOf(v)>=0;e.style.display=ok?'':'none';if(ok)any=true});g.style.display=any?'':'none'})}
if(q)q.addEventListener('input',filt);
function mv(id,sel){var v=sel.value;if(v==='\u0001new'){v=prompt('New folder name');if(!v||!v.trim()){sel.value=sel.dataset.cur;return}}go('bm-move/'+id+'/'+encodeURIComponent(v.trim()))}
function rn(id,t){var v=prompt('Bookmark title',t);if(v!==null&&v.trim())go('bm-rename/'+id+'/'+encodeURIComponent(v.trim()))}
function df(el){if(confirm('Delete this folder? Its bookmarks move to Unsorted.'))go('bm-folder-delete/'+encodeURIComponent(el.dataset.folder))}
"#;

pub fn bookmarks_page(ctx: &PageCtx, bookmarks: &[Bookmark]) -> String {
    // Folders in order of first appearance, "Bookmarks bar" and "Unsorted" always first.
    let mut folders: Vec<String> = vec![crate::bookmarks_io::BAR_FOLDER.to_string(), crate::bookmarks_io::DEFAULT_FOLDER.to_string()];
    for b in bookmarks {
        if !folders.contains(&b.folder) {
            folders.push(b.folder.clone());
        }
    }
    let option = |current: &str, f: &str| format!("<option value=\"{v}\"{sel}>{v}</option>", v = escape(f), sel = if f == current { " selected" } else { "" });
    let mut groups = String::new();
    for f in &folders {
        let members: Vec<&Bookmark> = bookmarks.iter().filter(|b| &b.folder == f).collect();
        if members.is_empty() {
            continue;
        }
        let protected = f == crate::bookmarks_io::DEFAULT_FOLDER || f == crate::bookmarks_io::BAR_FOLDER;
        let mut rows = String::new();
        for b in members {
            let title = if b.title.is_empty() { &b.url } else { &b.title };
            let options: String = folders.iter().map(|o| option(&b.folder, o)).collect();
            rows.push_str(&format!(
                "<div class=\"item\"{click}><div class=\"ic\">\u{2B50}</div><div class=\"t\"><b>{title}</b><span>{url}</span></div>\
                 <select data-cur=\"{cur}\" title=\"Move to folder\" onclick=\"event.stopPropagation()\" onchange=\"mv({id},this)\">{options}<option value=\"\u{1}new\">\u{FF0B} New folder\u{2026}</option></select>\
                 <button class=\"x\" title=\"Rename\" data-t=\"{title_attr}\" onclick=\"event.stopPropagation();rn({id},this.dataset.t)\">\u{270E}</button>\
                 <button class=\"x\" title=\"Remove bookmark\" onclick=\"event.stopPropagation();go('remove-bookmark/{id}')\">\u{2715}</button></div>",
                click = link_attr(&b.url),
                title = escape(&truncate(title, 90)),
                title_attr = escape(title),
                url = escape(&truncate(&b.url, 100)),
                cur = escape(&b.folder),
                options = options,
                id = b.id
            ));
        }
        groups.push_str(&format!(
            "<div class=\"fgroup\"><div class=\"fh\"><b>\u{1F4C1} {name}</b><span>{n}</span>{del}</div><div class=\"card\">{rows}</div></div>",
            name = escape(f),
            n = bookmarks.iter().filter(|b| &b.folder == f).count(),
            del = if protected { String::new() } else { format!("<button class=\"x\" title=\"Delete folder\" data-folder=\"{}\" onclick=\"df(this)\">\u{2715}</button>", escape(f)) },
            rows = rows
        ));
    }
    if groups.is_empty() {
        groups.push_str("<div class=\"card\"><div class=\"empty\">No bookmarks yet. Press the star in the address bar to add one, or import them below.</div></div>");
    }
    let body = format!(
        "{head}<div class=\"bar\"><input type=\"search\" id=\"q\" placeholder=\"Search bookmarks\" style=\"flex:1\">\
         <button class=\"btn ghost\" onclick=\"go('bm-import-chrome')\">Import from Chrome</button>\
         <button class=\"btn ghost\" onclick=\"go('bm-import-file')\">Import file\u{2026}</button>\
         <button class=\"btn ghost\" onclick=\"go('bm-export')\">Export</button></div>\
         <style>.fh{{display:flex;align-items:center;gap:10px;margin:18px 4px 8px;color:var(--heading)}}.fh span{{color:var(--muted);font-size:12px;flex:1}}\
         select{{border:1px solid var(--border);border-radius:8px;padding:5px 8px;background:var(--surface);color:var(--text);font:inherit;max-width:150px}}</style>{groups}",
        head = ui_shell::heading(tr(ctx.lang, "bookmarks.title"), tr(ctx.lang, "bookmarks.sub")),
        groups = groups
    );
    let body = loc(ctx.lang, body, &[
        ("placeholder=\"Search bookmarks\"", "Search bookmarks", "bookmarks.search"),
        (">Import from Chrome<", "Import from Chrome", "bookmarks.import_chrome"),
        (">Import file\u{2026}<", "Import file\u{2026}", "bookmarks.import_file"),
        (">Export<", "Export", "bookmarks.export"),
    ]);
    ui_shell::page(ctx, "bookmarks", "Bookmarks", &body, BOOKMARKS_JS)
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
            "completed" => ("ok", tr(ctx.lang, "dl.completed")),
            "downloading" if running => ("", tr(ctx.lang, "dl.downloading")),
            "cancelled" => ("bad", tr(ctx.lang, "dl.cancelled")),
            _ => ("bad", tr(ctx.lang, "dl.failed")),
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
                    if paused { tr(ctx.lang, "dl.resume") } else { tr(ctx.lang, "dl.pause") },
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
        rows.push_str(&format!("<div class=\"empty\">{}</div>", tr(ctx.lang, "downloads.empty")));
    }
    let body = format!(
        "{head}<div class=\"bar\"><span style=\"flex:1\"></span><button class=\"btn danger\" onclick=\"if(confirm('Clear the download list? Files stay on disk.'))go('clear-downloads')\">Clear list</button></div><div class=\"card\">{rows}</div>",
        head = ui_shell::heading(tr(ctx.lang, "downloads.title"), tr(ctx.lang, "downloads.sub")),
        rows = rows
    );
    let body = loc(ctx.lang, body, &[
        ("))go('clear-downloads')\">Clear list<", "Clear list", "downloads.clear"),
        (">Cancel</button>", "Cancel", "dl.cancel"),
        (">Open</button>", "Open", "dl.open"),
        (">Show in folder</button>", "Show in folder", "dl.show"),
    ]);
    ui_shell::page(ctx, "downloads", "Downloads", &body, DOWNLOADS_JS)
}

pub fn passwords_page(ctx: &PageCtx, logins: &[(i64, String, String, String)], never: &[String]) -> String {
    let mut rows = String::new();
    for (id, origin, user, updated) in logins {
        let shown_user = if user.is_empty() { "(no username)" } else { user.as_str() };
        rows.push_str(&format!(
            "<div class=\"item\" style=\"cursor:default\"><div class=\"ic\">\u{1F511}</div><div class=\"t\"><b>{origin}</b><span>{user} \u{00B7} \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}</span></div><div class=\"m\">{updated}</div>\
             <button class=\"btn ghost\" onclick=\"go('pw-copy-user/{id}')\">Copy username</button>\
             <button class=\"btn ghost\" onclick=\"go('pw-copy/{id}')\">Copy password</button>\
             <button class=\"x\" title=\"Delete\" onclick=\"if(confirm('Delete this saved password?'))go('pw-delete/{id}')\">\u{2715}</button></div>",
            origin = escape(origin),
            user = escape(shown_user),
            updated = escape(updated),
            id = id
        ));
    }
    if rows.is_empty() {
        rows.push_str("<div class=\"empty\">No saved passwords. When you sign in to a site, the browser offers to save the login.</div>");
    }
    let mut never_html = String::new();
    if !never.is_empty() {
        never_html.push_str("<h2>Never saved</h2><div class=\"card\">");
        for o in never {
            never_html.push_str(&format!(
                "<div class=\"item\" style=\"cursor:default\"><div class=\"t\"><b>{o}</b></div><button class=\"btn ghost\" data-o=\"{o}\" onclick=\"go('pw-never-remove/'+encodeURIComponent(this.dataset.o))\">Offer again</button></div>",
                o = escape(o)
            ));
        }
        never_html.push_str("</div>");
    }
    let body = format!(
        "{head}<div class=\"card\">{rows}</div>{never}",
        head = ui_shell::heading(tr(ctx.lang, "passwords.title"), tr(ctx.lang, "passwords.sub")),
        rows = rows,
        never = never_html
    );
    let body = loc(ctx.lang, body, &[
        (">Copy username<", "Copy username", "pw.copy_user"),
        (">Copy password<", "Copy password", "pw.copy"),
    ]);
    ui_shell::page(ctx, "passwords", "Passwords", &body, "")
}

pub fn permissions_page(ctx: &PageCtx, rows: &[(i64, String, String, bool)]) -> String {
    let mut list = String::new();
    let mut last = "";
    for (id, origin, kind, allow) in rows {
        if origin != last {
            if !last.is_empty() {
                list.push_str("</div>");
            }
            list.push_str(&format!("<div class=\"fh2\"><b>{}</b></div><div class=\"card\">", escape(origin)));
            last = origin;
        }
        let name = crate::permissions::label(kind).1;
        list.push_str(&format!(
            "<div class=\"item\" style=\"cursor:default\"><div class=\"t\"><b>{name}</b></div>\
             <select onchange=\"go('perm-set/{id}/'+this.value)\"><option value=\"allow\"{a}>Allow</option><option value=\"block\"{b}>Block</option></select>\
             <button class=\"x\" title=\"Reset (ask again)\" onclick=\"go('perm-delete/{id}')\">\u{2715}</button></div>",
            name = escape(name),
            id = id,
            a = if *allow { " selected" } else { "" },
            b = if *allow { "" } else { " selected" },
        ));
    }
    if !last.is_empty() {
        list.push_str("</div>");
    }
    if list.is_empty() {
        list.push_str("<div class=\"card\"><div class=\"empty\">No site has been given or refused a permission yet. When a page asks to use your camera, microphone or location, a bar appears under the toolbar.</div></div>");
    }
    let body = format!(
        "{head}<div class=\"bar\" style=\"display:flex\"><span style=\"flex:1\"></span><button class=\"btn danger\" onclick=\"if(confirm('Forget every site permission?'))go('perm-clear/all')\">Reset all</button></div>\
         <style>.fh2{{margin:18px 4px 8px;color:var(--heading)}}select{{border:1px solid var(--border);border-radius:8px;padding:6px 10px;background:var(--surface);color:var(--text);font:inherit}}</style>{list}",
        head = ui_shell::heading(tr(ctx.lang, "permissions.title"), tr(ctx.lang, "permissions.sub")),
        list = list
    );
    let body = loc(ctx.lang, body, &[
        (">Reset all<", "Reset all", "permissions.reset"),
    ]);
    ui_shell::page(ctx, "permissions", "Permissions", &body, "")
}

/// Shown instead of a site that has no https:// version while "Always use secure connections" is on.
pub fn https_warning_page(ctx: &PageCtx, host: &str, http_url: &str) -> String {
    let body = format!(
        "{head}<div class=\"card\" style=\"padding:22px 24px\"><p>Axomai tried to open <b>{host}</b> with a secure connection, but it could not. \
         If you continue, anything you send to or receive from this site (including passwords) can be read by others on the network.</p>\
         <p style=\"margin-top:16px;display:flex;gap:10px\"><button class=\"btn\" onclick=\"go('https-back')\">Go back</button>\
         <button class=\"btn ghost\" data-url=\"{url}\" onclick=\"go('https-continue/'+encodeURIComponent(this.dataset.url))\">Continue to the site (not secure)</button></p></div>",
        head = ui_shell::heading("Connection not secure", "This site does not support a private connection."),
        host = escape(host),
        url = escape(http_url)
    );
    ui_shell::page(ctx, "", "Connection not secure", &body, "")
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
        head = ui_shell::heading(tr(ctx.lang, "extensions.title"), tr(ctx.lang, "extensions.sub")),
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
    fn history_is_grouped_by_day_and_has_range_delete() {
        let e = |id, url: &str, t: &str| HistoryEntry { id, url: url.into(), title: String::new(), visit_time: t.into() };
        let html = history_page(&ctx(), &[e(3, "https://a.test", "2026-10-04 10:00:00"), e(2, "https://b.test", "2026-10-04 09:00:00"), e(1, "https://c.test", "2026-10-03 23:00:00")]);
        assert_eq!(html.matches("class=\"fgroup\"").count(), 2);
        assert!(html.contains("2026-10-03") && html.contains("delrange()"));
    }

    #[test]
    fn bookmarks_are_grouped_into_folders() {
        let b = |id, url: &str, folder: &str| Bookmark { id, url: url.into(), title: "T".into(), folder: folder.into(), created_at: String::new() };
        let html = bookmarks_page(&ctx(), &[b(1, "https://a.test", "Work"), b(2, "https://b.test", "Unsorted"), b(3, "https://c.test", "Work")]);
        assert_eq!(html.matches("class=\"fgroup\"").count(), 2, "empty folders are not listed");
        assert!(html.contains("go('bm-import-chrome')") && html.contains("go('bm-export')"));
        assert!(html.contains("data-folder=\"Work\""), "custom folders can be deleted");
        assert!(!html.contains("data-folder=\"Unsorted\""));
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
    fn passwords_page_never_shows_the_password() {
        let html = passwords_page(&ctx(), &[(3, "https://a.test".into(), "me<b>".into(), "2026-10-04 10:00:00".into())], &["https://n.test".into()]);
        assert!(html.contains("go('pw-copy/3')") && html.contains("me&lt;b&gt;"));
        assert!(html.contains("Never saved") && html.contains("https://n.test"));
        assert!(passwords_page(&ctx(), &[], &[]).contains("No saved passwords"));
    }

    #[test]
    fn permissions_page_groups_by_site() {
        let rows = vec![(1, "https://a.test".to_string(), "camera".to_string(), true), (2, "https://a.test".to_string(), "location".to_string(), false), (3, "https://b.test".to_string(), "microphone".to_string(), true)];
        let html = permissions_page(&ctx(), &rows);
        assert_eq!(html.matches("fh2").count(), 3, "two sites (plus the style rule)");
        assert!(html.contains("go('perm-set/2/'") && html.contains("Camera") && html.contains("Location"));
        assert!(permissions_page(&ctx(), &[]).contains("No site has been given"));
    }

    #[test]
    fn https_warning_names_the_site_and_escapes_it() {
        let html = https_warning_page(&ctx(), "ex<b>.com", "http://ex.com/?a=\"1\"");
        assert!(html.contains("ex&lt;b&gt;.com") && html.contains("https-continue/") && html.contains("https-back"));
        assert!(!html.contains("a=\"1\"\" onclick"));
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
