//! Common frame for Axomai's own pages (settings, history, bookmarks, downloads, extensions, ...): side navigation,
//! themed styles and the little script that lets a page send commands back to the browser.

use crate::theme::Theme;
use crate::viewsource::escape;

pub struct PageCtx<'a> {
    pub theme: &'a Theme,
    /// Per-run token that authorises commands posted from the page.
    pub token: &'a str,
    pub lang: &'a str,
}

const NAV: &[(&str, &str, &str)] = &[
    ("history", "\u{1F552}", "History"),
    ("bookmarks", "\u{2B50}", "Bookmarks"),
    ("downloads", "\u{2B07}\u{FE0F}", "Downloads"),
    ("passwords", "\u{1F511}", "Passwords"),
    ("extensions", "\u{1F9E9}", "Extensions"),
    ("settings", "\u{2699}\u{FE0F}", "Settings"),
];

const CSS: &str = r#"
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
html,body{height:100%}
body{background:var(--page-bg);color:var(--text);font:14px/1.5 "Plus Jakarta Sans","Segoe UI",system-ui,sans-serif;-webkit-font-smoothing:antialiased}
.app{display:flex;min-height:100%}
.side{width:220px;flex-shrink:0;padding:22px 14px;border-right:1px solid var(--border);position:sticky;top:0;height:100vh;overflow:auto}
.brand{display:flex;align-items:center;gap:10px;padding:0 10px 18px;font-weight:800;color:var(--heading);font-size:15px}
.brand i{width:26px;height:26px;border-radius:7px;background:linear-gradient(135deg,var(--primary),var(--accent));display:flex;align-items:center;justify-content:center;color:#fff;font-style:normal;font-size:14px}
.side a{display:flex;align-items:center;gap:10px;padding:9px 12px;border-radius:10px;color:var(--text);text-decoration:none;cursor:pointer;font-weight:600;font-size:13.5px}
.side a:hover{background:var(--hover)}
.side a.on{background:var(--primary-light);color:var(--primary)}
main{flex:1;min-width:0;padding:30px 40px 80px;max-width:980px}
h1{font-size:24px;color:var(--heading);letter-spacing:-.02em;margin-bottom:4px}
.sub{color:var(--muted);margin-bottom:22px}
h2{font-size:13px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted);margin:26px 0 10px}
.card{background:var(--surface);border:1px solid var(--border);border-radius:14px;overflow:hidden}
.row{display:flex;align-items:center;gap:16px;padding:14px 18px;border-bottom:1px solid var(--border)}
.row:last-child{border-bottom:0}
.row .l{flex:1;min-width:0}.row .l b{display:block;color:var(--heading);font-weight:600}.row .l span{color:var(--muted);font-size:12.5px}
.row.col{flex-direction:column;align-items:stretch;gap:10px}
input[type=text],input[type=search],select{background:transparent;color:var(--heading);border:1px solid var(--border);border-radius:10px;padding:8px 12px;font:inherit;min-width:200px;outline:none}
input[type=text]:focus,input[type=search]:focus,select:focus{border-color:var(--primary)}
select option{color:#111;background:#fff}
.btn{border:0;border-radius:10px;padding:8px 16px;background:var(--primary);color:#fff;font:600 13px inherit;font-family:inherit;cursor:pointer}
.btn:hover{filter:brightness(1.07)}.btn.ghost{background:var(--surface2);color:var(--heading)}.btn.danger{background:var(--danger)}
.btn:disabled{opacity:.45;cursor:default}
.switch{position:relative;width:42px;height:24px;flex-shrink:0}.switch input{opacity:0;width:0;height:0}
.switch .s{position:absolute;inset:0;background:rgba(128,128,128,.4);border-radius:24px;transition:.2s;cursor:pointer}
.switch .s:before{content:"";position:absolute;width:18px;height:18px;left:3px;top:3px;background:#fff;border-radius:50%;transition:.2s}
.switch input:checked+.s{background:var(--primary)}.switch input:checked+.s:before{transform:translateX(18px)}
.themes{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:12px;padding:16px 18px}
.theme{border:2px solid var(--border);border-radius:12px;padding:10px;cursor:pointer;background:var(--surface2)}
.theme:hover{border-color:var(--primary)}.theme.on{border-color:var(--primary);background:var(--primary-light)}
.theme .sw{height:44px;border-radius:8px;margin-bottom:8px}.theme b{display:block;color:var(--heading);font-size:12.5px}.theme span{color:var(--muted);font-size:11px}
.radio{display:flex;align-items:center;gap:10px;cursor:pointer;padding:4px 0}.radio input{accent-color:var(--primary);width:16px;height:16px}
.checks{display:flex;flex-wrap:wrap;gap:6px 22px}.checks label{display:flex;align-items:center;gap:8px;cursor:pointer}.checks input{accent-color:var(--primary);width:15px;height:15px}
.empty{padding:50px 20px;text-align:center;color:var(--muted)}
.item{display:flex;align-items:center;gap:14px;padding:11px 18px;border-bottom:1px solid var(--border);cursor:pointer}
.item:last-child{border-bottom:0}.item:hover{background:var(--hover)}
.item .ic{width:34px;height:34px;border-radius:9px;background:var(--surface2);display:flex;align-items:center;justify-content:center;flex-shrink:0;font-size:16px}
.item .t{flex:1;min-width:0}.item .t b{display:block;color:var(--heading);font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.item .t span{display:block;color:var(--muted);font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.item .m{color:var(--muted);font-size:12px;white-space:nowrap}
.x{border:0;background:transparent;color:var(--muted);font-size:16px;cursor:pointer;border-radius:8px;width:30px;height:30px}.x:hover{background:var(--hover);color:var(--danger)}
.bar{display:flex;gap:10px;align-items:center;margin-bottom:14px;flex-wrap:wrap}
.pill{display:inline-block;padding:2px 10px;border-radius:99px;font-size:11.5px;font-weight:700;background:var(--surface2);color:var(--muted)}
.pill.ok{background:var(--primary-light);color:var(--primary)}.pill.bad{background:rgba(220,38,38,.12);color:var(--danger)}
.toast{position:fixed;bottom:22px;left:50%;transform:translateX(-50%);background:var(--primary);color:#fff;padding:9px 20px;border-radius:99px;font-weight:600;opacity:0;transition:.25s;pointer-events:none}
.toast.show{opacity:1}
@media(max-width:760px){.side{display:none}main{padding:20px}}
"#;

/// Wrap `body` in the shared frame. `active` is the nav entry to highlight ("settings", "history", ...).
pub fn page(ctx: &PageCtx, active: &str, title: &str, body: &str, script: &str) -> String {
    let mut nav = String::new();
    for (key, icon, label) in NAV {
        nav.push_str(&format!(
            "<a class=\"{}\" onclick=\"go('{}')\"><span>{}</span>{}</a>",
            if *key == active { "on" } else { "" },
            key,
            icon,
            label
        ));
    }
    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} - Axomai Browser</title>
<style>:root{{{vars}color-scheme:{scheme}}}{css}</style></head>
<body><div class="app">
<nav class="side"><div class="brand"><i>A</i>Axomai</div>{nav}</nav>
<main>{body}</main></div>
<div class="toast" id="toast"></div>
<script>
var TOKEN={token};
function go(c){{try{{window.chrome.webview.postMessage(TOKEN+'/'+c)}}catch(e){{location.href='axomai://'+c}}}}
function toast(t){{var e=document.getElementById('toast');e.textContent=t;e.classList.add('show');clearTimeout(window.__tt);window.__tt=setTimeout(function(){{e.classList.remove('show')}},1800)}}
function enc(v){{return encodeURIComponent(v)}}
{script}
</script></body></html>"##,
        lang = escape(ctx.lang),
        title = escape(title),
        vars = ctx.theme.page_vars(),
        scheme = if ctx.theme.dark { "dark" } else { "light" },
        css = CSS,
        nav = nav,
        body = body,
        token = serde_json::to_string(ctx.token).unwrap_or_else(|_| "\"\"".into()),
        script = script,
    )
}

/// Page title heading block.
pub fn heading(title: &str, subtitle: &str) -> String {
    format!("<h1>{}</h1><p class=\"sub\">{}</p>", escape(title), escape(subtitle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_has_title_theme_and_token() {
        let theme = crate::theme::by_id("cyber-dark");
        let ctx = PageCtx { theme, token: "tok123", lang: "en" };
        let html = page(&ctx, "settings", "Settings", "<p>hi</p>", "");
        assert!(html.contains("<title>Settings - Axomai Browser</title>"));
        assert!(html.contains("var TOKEN=\"tok123\""));
        assert!(html.contains("--page-bg:#0b1220"));
        assert!(html.contains("color-scheme:dark"));
        assert!(html.contains("<a class=\"on\" onclick=\"go('settings')\">"));
    }

    #[test]
    fn titles_are_escaped() {
        let theme = crate::theme::by_id("tea-garden");
        let ctx = PageCtx { theme, token: "t", lang: "en" };
        let html = page(&ctx, "", "<b>x</b>", "", "");
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt; - Axomai Browser"));
    }
}
