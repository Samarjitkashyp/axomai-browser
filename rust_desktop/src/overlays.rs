//! Popups for the toolbar buttons, injected into the current page.
//!
//! Every popup lives in a closed shadow root (the page can neither restyle nor read it), is themed from the
//! active heritage theme, and talks back to the browser with `postMessage('<token>/<command>')`, which
//! `web.rs` only accepts when the token matches. Opening a popup closes any other one.

use crate::ext_scripts::shell;
use crate::theme::{Theme, THEMES};
use serde_json::json;

fn open(theme: &Theme, token: &str, id: &str) -> String {
    format!(
        "(function(){{var TOKEN='{}';function go(c){{try{{window.chrome.webview.postMessage(TOKEN+'/'+c)}}catch(x){{location.href='axomai://'+TOKEN+'/'+c}}}}\n{}\n\
         function el(t,c,x){{var e=document.createElement(t);if(c)e.className=c;if(x!==undefined)e.textContent=x;return e}}\n\
         function finish(card){{root.appendChild(card);document.documentElement.appendChild(host);\
         document.addEventListener('keydown',function k(e){{if(e.key==='Escape'){{host.remove();document.removeEventListener('keydown',k,true)}}}},true);\
         document.addEventListener('mousedown',function m(e){{if(!e.composedPath().includes(host)){{host.remove();document.removeEventListener('mousedown',m,true)}}}},true)}}",
        token,
        shell(theme, id)
    )
}

const CARD_CSS: &str = r#"
.card{position:fixed;top:6px;background:var(--bg);backdrop-filter:blur(24px);border:1px solid var(--border);border-radius:14px;box-shadow:var(--shadow);padding:8px 6px;color:var(--text);font-size:13px;max-height:calc(100vh - 16px);overflow:auto;animation:in .16s cubic-bezier(.16,1,.3,1)}
@keyframes in{from{opacity:0;transform:translateY(-6px) scale(.97)}to{opacity:1;transform:none}}
.hd{display:flex;align-items:center;justify-content:space-between;padding:8px 12px 4px}
.hd b{font-size:14px;color:var(--heading)}.hd span{font-size:11.5px;color:var(--muted)}
.sub{padding:2px 12px 8px;font-size:11px;color:var(--muted);line-height:1.4}
.row{display:flex;align-items:center;gap:10px;padding:8px 10px;border-radius:10px;cursor:pointer;border:0;background:transparent;width:100%;text-align:left;color:inherit;font:inherit}
.row:hover{background:var(--hover)}.row .em{font-size:18px;width:24px;text-align:center}
.row .tx{display:flex;flex-direction:column;flex:1;min-width:0}.row .tx b{font-size:12.5px;color:var(--heading)}.row .tx i{font-style:normal;font-size:10.5px;color:var(--muted)}
.row .sc{font-size:11px;color:var(--muted)}.div{height:1px;background:var(--border);margin:6px 8px}
.sw{position:relative;width:34px;height:19px;flex-shrink:0}.sw input{opacity:0;width:0;height:0}
.sl{position:absolute;inset:0;background:rgba(128,128,128,.4);border-radius:19px;transition:.2s;cursor:pointer}
.sl:before{content:"";position:absolute;width:15px;height:15px;left:2px;top:2px;background:#fff;border-radius:50%;transition:.2s}
.sw input:checked+.sl{background:var(--primary)}.sw input:checked+.sl:before{transform:translateX(15px)}
.foot{display:flex;align-items:center;gap:8px;padding:9px 12px;border-radius:10px;cursor:pointer;color:var(--heading);font-size:12.5px;font-weight:600}.foot:hover{background:var(--hover)}
.btn{border:0;border-radius:10px;padding:8px 14px;background:var(--primary);color:#fff;font-weight:600;cursor:pointer;font-size:12.5px}
.btn.ghost{background:var(--hover);color:var(--heading)}.btn.danger{background:#dc2626}
"#;

fn css_literal(extra: &str, right: f32, width: f32) -> String {
    let all = format!("{}{}.card{{right:{:.0}px;width:{:.0}px}}", CARD_CSS, extra, right.max(6.0), width);
    serde_json::to_string(&all).unwrap_or_else(|_| "\"\"".into())
}

pub struct ExtItem {
    pub idx: usize,
    pub emoji: &'static str,
    pub name: &'static str,
    pub status: String,
    pub enabled: bool,
}

/// The puzzle-icon popup. Rows come from the real extension state, so it always matches the Extensions page.
pub fn extensions_popup(theme: &Theme, token: &str, right: f32, items: &[ExtItem]) -> String {
    let data: Vec<_> = items
        .iter()
        .map(|i| json!({"i": i.idx, "e": i.emoji, "n": i.name, "s": i.status, "on": i.enabled}))
        .collect();
    let active = items.iter().filter(|i| i.enabled).count();
    let mut js = open(theme, token, "__ax_pop_ext");
    js.push_str(&format!(
        r#"
css({css});
var data={data};
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','Extensions'));var cnt=el('span','','{active} of '+data.length+' on');hd.appendChild(cnt);card.appendChild(hd);
card.appendChild(el('div','sub','Click an extension to use it. Use the switch to turn it on or off.'));
function recount(){{var n=0;card.querySelectorAll('input').forEach(function(i){{if(i.checked)n++}});cnt.textContent=n+' of '+data.length+' on'}}
data.forEach(function(d){{
  var r=el('div','row');
  r.appendChild(el('span','em',d.e));
  var tx=el('span','tx');tx.appendChild(el('b','',d.n));var st=el('i','',d.on?d.s:'Turned off');tx.appendChild(st);r.appendChild(tx);
  var sw=el('label','sw');var inp=document.createElement('input');inp.type='checkbox';inp.checked=d.on;
  sw.appendChild(inp);sw.appendChild(el('span','sl'));r.appendChild(sw);
  sw.addEventListener('click',function(e){{e.stopPropagation()}});
  inp.addEventListener('change',function(){{go('ext-toggle/'+d.i);st.textContent=inp.checked?d.s:'Turned off';recount()}});
  r.addEventListener('click',function(){{if(!inp.checked){{inp.checked=true;go('ext-toggle/'+d.i);recount()}}host.remove();go('ext-run/'+d.i)}});
  card.appendChild(r);
}});
card.appendChild(el('div','div'));
var f=el('div','foot');f.appendChild(el('span','','⚙️'));f.appendChild(el('span','','Manage extensions'));f.onclick=function(){{host.remove();go('extensions')}};card.appendChild(f);
finish(card);
}})();"#,
        css = css_literal("", right, 330.0),
        data = serde_json::to_string(&data).unwrap_or_else(|_| "[]".into()),
        active = active
    ));
    js
}

pub struct MenuEntry {
    pub emoji: &'static str,
    pub label: &'static str,
    pub cmd: &'static str,
    pub danger: bool,
}

/// Three-dot menu (also used when the page area is a web page).
pub fn menu_popup(theme: &Theme, token: &str, right: f32, entries: &[MenuEntry]) -> String {
    let data: Vec<_> = entries
        .iter()
        .map(|m| json!({"e": m.emoji, "l": m.label, "c": m.cmd, "d": m.danger}))
        .collect();
    let mut js = open(theme, token, "__ax_pop_menu");
    js.push_str(&format!(
        r#"
css({css});
var card=el('div','card');
{data}.forEach(function(m){{
  if(m.l==='-'){{card.appendChild(el('div','div'));return}}
  var r=el('button','row');r.appendChild(el('span','em',m.e));var tx=el('span','tx');var b=el('b','',m.l);if(m.d)b.style.color='#dc2626';tx.appendChild(b);r.appendChild(tx);
  r.onclick=function(){{host.remove();go(m.c)}};card.appendChild(r);
}});
finish(card);
}})();"#,
        css = css_literal("", right, 270.0),
        data = serde_json::to_string(&data).unwrap_or_else(|_| "[]".into())
    ));
    js
}

/// A context menu placed from the left edge of the page (used under a tab). Entries are
/// `(emoji, label, command, danger)`; a label of "-" is a divider.
pub fn tab_menu_popup(theme: &Theme, token: &str, left: f32, entries: &[(String, String, String, bool)]) -> String {
    let data: Vec<_> = entries.iter().map(|(e, l, c, d)| json!({"e": e, "l": l, "c": c, "d": d})).collect();
    let mut js = open(theme, token, "__ax_pop_tabmenu");
    let css = serde_json::to_string(&format!("{}.card{{left:{:.0}px;width:250px}}", CARD_CSS, left.max(6.0))).unwrap_or_else(|_| "\"\"".into());
    js.push_str(&format!(
        r#"
css({css});
var card=el('div','card');
{data}.forEach(function(m){{
  if(m.l==='-'){{card.appendChild(el('div','div'));return}}
  var r=el('button','row');r.appendChild(el('span','em',m.e));var tx=el('span','tx');var b=el('b','',m.l);if(m.d)b.style.color='#dc2626';tx.appendChild(b);r.appendChild(tx);
  r.onclick=function(){{host.remove();go(m.c)}};card.appendChild(r);
}});
finish(card);
}})();"#,
        css = css,
        data = serde_json::to_string(&data).unwrap_or_else(|_| "[]".into())
    ));
    js
}

/// Find-in-page bar, living inside the page so the web view cannot cover it. Uses `window.find`.
pub fn find_popup(theme: &Theme) -> String {
    let mut js = String::from("(function(){");
    js.push_str(&shell(theme, "__ax_pop_find"));
    js.push_str(
        r#"
var card=document.createElement('div');card.className='bar';
var inp=document.createElement('input');inp.placeholder='Find in page';inp.spellcheck=false;
function clearSel(){try{window.getSelection().removeAllRanges()}catch(e){}}
function run(back){var q=inp.value;if(!q){clearSel();info.textContent='';return}
  var ok=window.find(q,false,back,true,false,false,false);info.textContent=ok?'':'No matches';info.style.color=ok?'':'#dc2626'}
function btn(t,title,fn){var b=document.createElement('button');b.textContent=t;b.title=title;b.onclick=fn;card.appendChild(b);return b}
var info=document.createElement('span');info.className='info';
inp.addEventListener('input',function(){clearSel();run(false)});
inp.addEventListener('keydown',function(e){e.stopPropagation();
  if(e.key==='Enter'){e.preventDefault();run(e.shiftKey)}
  else if(e.key==='Escape'){e.preventDefault();clearSel();host.remove()}},true);
card.appendChild(inp);card.appendChild(info);
btn('↑','Previous (Shift+Enter)',function(){run(true)});
btn('↓','Next (Enter)',function(){run(false)});
btn('✕','Close (Esc)',function(){clearSel();host.remove()});
css('.bar{position:fixed;top:8px;right:24px;display:flex;align-items:center;gap:6px;background:var(--bg);border:1px solid var(--border);border-radius:12px;box-shadow:var(--shadow);padding:6px 8px;font-size:13px;color:var(--text)}'+
'input{width:210px;border:1px solid var(--border);border-radius:8px;padding:6px 10px;background:transparent;color:var(--heading);font:inherit;outline:none}input:focus{border-color:var(--primary)}'+
'.info{font-size:11px;min-width:58px;color:var(--muted)}button{border:0;background:var(--hover);color:var(--heading);border-radius:8px;width:28px;height:28px;cursor:pointer;font-size:13px}button:hover{background:var(--primary-light)}');
root.appendChild(card);document.documentElement.appendChild(host);inp.focus();inp.select();
})();"#,
    );
    js
}

pub struct ShieldView {
    pub host: String,
    pub page_blocked: u32,
    pub total_blocked: u64,
    pub adblock: bool,
    pub privacy: bool,
}

pub fn shield_popup(theme: &Theme, token: &str, right: f32, v: &ShieldView) -> String {
    let mut js = open(theme, token, "__ax_pop_shield");
    js.push_str(&format!(
        r#"
css({css}+'.big{{font-size:38px;font-weight:800;color:var(--primary);line-height:1}}.stat{{padding:6px 12px 10px}}.stat small{{display:block;color:var(--muted);font-size:11px;margin-top:4px}}');
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','🛡️ Axomai Shield'));hd.appendChild(el('span','',{host}));card.appendChild(hd);
var s=el('div','stat');s.appendChild(el('div','big','{page}'));s.appendChild(el('small','','ads & trackers blocked on this page · {total} blocked in total'));card.appendChild(s);
card.appendChild(el('div','div'));
function toggle(label,sub,on,idx){{
  var r=el('div','row');var tx=el('span','tx');tx.appendChild(el('b','',label));tx.appendChild(el('i','',sub));r.appendChild(tx);
  var sw=el('label','sw');var inp=document.createElement('input');inp.type='checkbox';inp.checked=on;sw.appendChild(inp);sw.appendChild(el('span','sl'));r.appendChild(sw);
  inp.addEventListener('change',function(){{go('ext-toggle/'+idx)}});card.appendChild(r);
}}
toggle('Ad blocker','Blocks ad networks before they load',{adblock},0);
toggle('Privacy guard','Blocks trackers and fingerprinting',{privacy},3);
var f=el('div','foot');f.appendChild(el('span','','⚙️'));f.appendChild(el('span','','Manage extensions'));f.onclick=function(){{host.remove();go('extensions')}};card.appendChild(f);
finish(card);
}})();"#,
        css = css_literal("", right, 320.0),
        host = serde_json::to_string(&v.host).unwrap_or_else(|_| "\"\"".into()),
        page = v.page_blocked,
        total = v.total_blocked,
        adblock = v.adblock,
        privacy = v.privacy,
    ));
    js
}

pub fn site_info_popup(
    theme: &Theme,
    token: &str,
    right: f32,
    security: crate::toolbar::Security,
    host: &str,
    blocked: u32,
) -> String {
    use crate::toolbar::Security;
    let (emoji, headline, detail) = match security {
        Security::Secure => ("🔒", "Connection is secure", "Your connection to this site is encrypted with HTTPS, so other people on the network cannot read or change what you send."),
        Security::Insecure => ("⚠️", "Connection is not secure", "This page uses plain HTTP. Anything you type here, such as passwords or card numbers, can be seen by others on the network."),
        Security::Internal => ("🏠", "Axomai Browser page", "This is a built-in page. It is stored on your computer and does not use the network."),
    };
    let mut js = open(theme, token, "__ax_pop_site");
    js.push_str(&format!(
        r#"
css({css}+'.big{{font-size:28px;padding:10px 14px 2px}}.p{{padding:2px 14px 10px;font-size:12px;line-height:1.5;color:var(--text)}}.h{{padding:0 14px;font-weight:700;color:var(--heading);font-size:14px}}.sm{{padding:0 14px 6px;font-size:11px;color:var(--muted)}}');
var card=el('div','card');
card.appendChild(el('div','big',{emoji}));card.appendChild(el('div','h',{headline}));card.appendChild(el('div','sm',{host}));
card.appendChild(el('div','p',{detail}));
card.appendChild(el('div','div'));
var r=el('button','row');r.appendChild(el('span','em','🛡️'));var tx=el('span','tx');tx.appendChild(el('b','','Axomai Shield blocked {blocked} ads & trackers here'));tx.appendChild(el('i','','Click for details'));r.appendChild(tx);
r.onclick=function(){{host.remove();go('shield')}};card.appendChild(r);
finish(card);
}})();"#,
        css = css_literal("", right, 330.0),
        emoji = serde_json::to_string(emoji).unwrap_or_default(),
        headline = serde_json::to_string(headline).unwrap_or_default(),
        detail = serde_json::to_string(detail).unwrap_or_default(),
        host = serde_json::to_string(host).unwrap_or_default(),
        blocked = blocked,
    ));
    js
}

pub struct ProfileView {
    pub name: String,
    pub bookmarks: usize,
    pub history: usize,
    pub downloads: usize,
    pub blocked_total: u64,
}

pub fn profile_popup(theme: &Theme, token: &str, right: f32, v: &ProfileView) -> String {
    let initial = v.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "A".into());
    let mut js = open(theme, token, "__ax_pop_profile");
    js.push_str(&format!(
        r#"
css({css}+'.av{{width:54px;height:54px;border-radius:50%;background:linear-gradient(135deg,var(--primary),var(--accent));color:#fff;font-size:24px;font-weight:700;display:flex;align-items:center;justify-content:center}}.who{{display:flex;gap:14px;align-items:center;padding:12px}}.who b{{font-size:15px;color:var(--heading)}}.who i{{display:block;font-style:normal;font-size:11px;color:var(--muted);margin-top:2px}}.stats{{display:grid;grid-template-columns:repeat(4,1fr);gap:6px;padding:4px 10px 10px}}.stats div{{text-align:center;background:var(--hover);border-radius:10px;padding:8px 2px}}.stats b{{display:block;font-size:15px;color:var(--heading)}}.stats small{{font-size:10px;color:var(--muted)}}input.nm{{flex:1;border:1px solid var(--border);border-radius:8px;padding:6px 8px;background:transparent;color:var(--heading);font:inherit}}.edit{{display:flex;gap:6px;padding:0 12px 8px}}');
var card=el('div','card');
var who=el('div','who');who.appendChild(el('div','av',{initial}));
var wi=el('div');var nm=el('b','',{name});wi.appendChild(nm);wi.appendChild(el('i','','Local profile · data stays on this device'));who.appendChild(wi);card.appendChild(who);
var stats=el('div','stats');
[['Bookmarks','{bm}'],['History','{hi}'],['Downloads','{dl}'],['Blocked','{bl}']].forEach(function(p){{var d=el('div');d.appendChild(el('b','',p[1]));d.appendChild(el('small','',p[0]));stats.appendChild(d)}});
card.appendChild(stats);
var ed=el('div','edit');var inp=document.createElement('input');inp.className='nm';inp.value={name};inp.maxLength=32;
var sv=el('button','btn','Save name');sv.onclick=function(){{var v=inp.value.trim();if(v){{go('profile-name/'+encodeURIComponent(v));host.remove()}}}};
inp.addEventListener('keydown',function(e){{e.stopPropagation();if(e.key==='Enter')sv.click()}},true);
ed.appendChild(inp);ed.appendChild(sv);card.appendChild(ed);
card.appendChild(el('div','div'));
function row(e,t,c){{var r=el('button','row');r.appendChild(el('span','em',e));var tx=el('span','tx');tx.appendChild(el('b','',t));r.appendChild(tx);r.onclick=function(){{host.remove();go(c)}};card.appendChild(r)}}
row('⭐','Bookmarks','bookmarks');row('🕒','History','history');row('⬇️','Downloads','downloads');row('⚙️','Settings','settings');
card.appendChild(el('div','div'));
var clr=el('button','row');clr.appendChild(el('span','em','🧹'));var ct=el('span','tx');var cb=el('b','','Clear browsing data…');cb.style.color='#dc2626';ct.appendChild(cb);ct.appendChild(el('i','','History, downloads list, cookies and cache'));clr.appendChild(ct);
var armed=false;clr.onclick=function(){{if(!armed){{armed=true;cb.textContent='Click again to confirm';return}}host.remove();go('clear-data')}};card.appendChild(clr);
finish(card);
}})();"#,
        css = css_literal("", right, 340.0),
        initial = serde_json::to_string(&initial).unwrap_or_else(|_| "\"A\"".into()),
        name = serde_json::to_string(&v.name).unwrap_or_else(|_| "\"\"".into()),
        bm = v.bookmarks,
        hi = v.history,
        dl = v.downloads,
        bl = v.blocked_total,
    ));
    js
}

pub fn theme_popup(theme: &Theme, token: &str, right: f32) -> String {
    let data: Vec<_> = THEMES
        .iter()
        .map(|t| json!({"id": t.id, "n": t.name, "d": t.desc, "e": t.emoji, "sw": t.swatch, "on": t.id == theme.id}))
        .collect();
    let mut js = open(theme, token, "__ax_pop_theme");
    js.push_str(&format!(
        r#"
css({css}+'.sw2{{width:38px;height:38px;border-radius:10px;flex-shrink:0;border:2px solid transparent}}.row.on{{background:var(--primary-light)}}.row.on .sw2{{border-color:var(--primary)}}');
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','🎨 Heritage Themes'));card.appendChild(hd);
card.appendChild(el('div','sub','Colours the toolbar and every popup. Saved for next time.'));
{data}.forEach(function(t){{
  var r=el('button','row'+(t.on?' on':''));var s=el('span','sw2');s.style.background=t.sw;r.appendChild(s);
  var tx=el('span','tx');tx.appendChild(el('b','',t.e+' '+t.n));tx.appendChild(el('i','',t.d));r.appendChild(tx);
  if(t.on)r.appendChild(el('span','sc','✓ Active'));
  r.onclick=function(){{host.remove();go('theme/'+t.id)}};card.appendChild(r);
}});
finish(card);
}})();"#,
        css = css_literal("", right, 330.0),
        data = serde_json::to_string(&data).unwrap_or_else(|_| "[]".into())
    ));
    js
}

pub fn ai_popup(theme: &Theme, token: &str, right: f32, host_name: &str, title: &str) -> String {
    let mut js = open(theme, token, "__ax_pop_ai");
    js.push_str(&format!(
        r#"
css({css}+'.out{{padding:4px 12px 10px;font-size:12.5px;line-height:1.55;color:var(--text);max-height:42vh;overflow:auto}}.out p{{margin:0 0 8px}}.out h4{{margin:6px 0;color:var(--heading);font-size:12px}}.chips{{display:flex;flex-wrap:wrap;gap:6px}}.chip{{background:var(--primary-light);color:var(--primary);border-radius:12px;padding:3px 10px;font-size:11.5px;font-weight:600}}.ask{{display:flex;gap:6px;padding:4px 12px 10px}}.ask input{{flex:1;border:1px solid var(--border);border-radius:10px;padding:8px 10px;background:transparent;color:var(--heading);font:inherit}}.acts{{display:flex;gap:6px;padding:4px 12px 8px;flex-wrap:wrap}}.muted{{color:var(--muted)}}');
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','✨ Axomai AI Copilot'));hd.appendChild(el('span','',{badge}));card.appendChild(hd);
card.appendChild(el('div','sub',{title}+' · '+{host}));
var acts=el('div','acts');
function act(t,c){{var b=el('button','btn ghost',t);b.onclick=function(){{busy();go(c)}};acts.appendChild(b)}}
act('Summarize','ai/summarize');act('Key topics','ai/keywords');act('Reading stats','ai/stats');
card.appendChild(acts);
var ask=el('div','ask');var q=document.createElement('input');q.placeholder='Ask about this page…';
var go2=el('button','btn','Ask');
function send(){{var v=q.value.trim();if(v){{busy();go('ai/ask/'+encodeURIComponent(v))}}}}
go2.onclick=send;q.addEventListener('keydown',function(e){{e.stopPropagation();if(e.key==='Enter')send()}},true);
ask.appendChild(q);ask.appendChild(go2);card.appendChild(ask);
var out=el('div','out');out.appendChild(el('p','muted',{note}));card.appendChild(out);
function busy(){{out.textContent='';out.appendChild(el('p','muted','Reading the page…'))}}
host.__axOut=out;
host.__axSet=function(r){{
  out.textContent='';
  if(r.title)out.appendChild(el('h4','',r.title));
  (r.paragraphs||[]).forEach(function(t){{out.appendChild(el('p','',t))}});
  if(r.chips&&r.chips.length){{var c=el('div','chips');r.chips.forEach(function(t){{c.appendChild(el('span','chip',t))}});out.appendChild(c)}}
  if(!out.firstChild)out.appendChild(el('p','muted','Nothing to show for this page.'));
}};
finish(card);
}})();"#,
        css = css_literal("", right, 380.0),
        title = serde_json::to_string(&truncate(title, 60)).unwrap_or_else(|_| "\"\"".into()),
        host = serde_json::to_string(host_name).unwrap_or_else(|_| "\"\"".into()),
        badge = serde_json::to_string("on-device").unwrap_or_default(),
        note = serde_json::to_string("Runs on your device \u{2014} the page text never leaves this computer. Results are extractive (picked from the page itself).").unwrap_or_default(),
    ));
    js
}

/// Push a result into an open AI popup (no-op if it was closed).
pub fn ai_result(title: &str, paragraphs: &[String], chips: &[String]) -> String {
    let payload = json!({"title": title, "paragraphs": paragraphs, "chips": chips});
    format!(
        "(function(){{var h=document.getElementById('__ax_pop_ai');if(h&&h.__axSet)h.__axSet({})}})();",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    )
}

/// QR code of the current address. `svg_modules` is the module matrix, row-major, true = dark.
pub fn qr_popup(theme: &Theme, token: &str, right: f32, url: &str, size: usize, modules: &[bool]) -> String {
    let quiet = 2usize;
    let total = size + quiet * 2;
    let mut path = String::new();
    for y in 0..size {
        for x in 0..size {
            if modules[y * size + x] {
                path.push_str(&format!("M{} {}h1v1h-1z", x + quiet, y + quiet));
            }
        }
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {t} {t}" shape-rendering="crispEdges"><rect width="{t}" height="{t}" fill="#fff"/><path d="{p}" fill="#0f172a"/></svg>"##,
        t = total,
        p = path
    );
    let mut js = open(theme, token, "__ax_pop_qr");
    js.push_str(&format!(
        r#"
css({css}+'.qr{{display:flex;justify-content:center;padding:8px 0}}.qr svg{{width:210px;height:210px;border-radius:10px;border:1px solid var(--border)}}.url{{padding:0 14px 8px;font-size:11px;color:var(--muted);word-break:break-all;text-align:center}}.acts{{display:flex;gap:8px;justify-content:center;padding:0 12px 10px}}');
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','📱 Scan to open on your phone'));card.appendChild(hd);
var q=el('div','qr');q.innerHTML={svg};card.appendChild(q);
card.appendChild(el('div','url',{url}));
var acts=el('div','acts');
var cp=el('button','btn','Copy link');cp.onclick=function(){{try{{navigator.clipboard.writeText({url}).then(function(){{cp.textContent='Copied ✓'}},function(){{cp.textContent='Copy failed'}})}}catch(e){{cp.textContent='Copy failed'}}}};
var cl=el('button','btn ghost','Close');cl.onclick=function(){{host.remove()}};
acts.appendChild(cp);acts.appendChild(cl);card.appendChild(acts);
finish(card);
}})();"#,
        css = css_literal("", right, 270.0),
        svg = serde_json::to_string(&svg).unwrap_or_else(|_| "\"\"".into()),
        url = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into()),
    ));
    js
}

/// Small message at the bottom of the page.
pub fn toast(theme: &Theme, message: &str, action: Option<(&str, &str, &str)>) -> String {
    // action = (label, token, command)
    let act = match action {
        Some((label, token, cmd)) => format!(
            "var a=document.createElement('button');a.textContent={};a.style.cssText='margin-left:14px;background:rgba(255,255,255,.2);border:0;color:#fff;border-radius:12px;padding:4px 12px;font:600 12px Segoe UI,sans-serif;cursor:pointer';a.onclick=function(){{try{{window.chrome.webview.postMessage('{}/{}')}}catch(e){{}};h.remove()}};box.appendChild(a);",
            serde_json::to_string(label).unwrap_or_default(),
            token,
            cmd
        ),
        None => String::new(),
    };
    let p = theme.primary;
    format!(
        "(function(){{var old=document.getElementById('__ax_toast');if(old)old.remove();var h=document.createElement('div');h.id='__ax_toast';h.style.cssText='all:initial;position:fixed;bottom:24px;left:50%;transform:translateX(-50%);z-index:2147483647';\
         var r=h.attachShadow({{mode:'closed'}});var box=document.createElement('div');box.textContent={};\
         box.style.cssText='display:flex;align-items:center;background:rgb({},{},{});color:#fff;padding:11px 22px;border-radius:24px;font:600 13.5px Segoe UI,system-ui,sans-serif;box-shadow:0 8px 30px rgba(0,0,0,.35);max-width:80vw';\
         {}r.appendChild(box);document.documentElement.appendChild(h);setTimeout(function(){{h.remove()}},{});}})();",
        serde_json::to_string(message).unwrap_or_default(),
        p[0], p[1], p[2],
        act,
        if action.is_some() { 6500 } else { 2600 },
    )
}

/// The address-bar dropdown: rows are (icon, title, subtitle); clicking one posts `suggest/<row>`.
pub fn suggest_popup(theme: &Theme, token: &str, left: f32, width: f32, rows: &[(String, String, String)], selected: usize) -> String {
    let data = serde_json::to_string(&rows.iter().map(|(i, t, s)| json!([i, truncate(t, 90), truncate(s, 100)])).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".into());
    let extra = format!(".card{{top:4px;left:{:.0}px;padding:6px;border-radius:16px}}.row{{padding:7px 10px}}.row.sel{{background:var(--hover)}}", left);
    format!(
        "var __o=document.getElementById('__ax_pop_suggest');if(__o)__o.remove();{}         var rows={};var sel={};css({});var card=el('div','card');         rows.forEach(function(r,i){{var b=el('div','row'+(i===sel?' sel':''));var em=el('span','em',r[0]);var tx=el('span','tx');tx.appendChild(el('b','',r[1]));tx.appendChild(el('i','',r[2]));b.appendChild(em);b.appendChild(tx);         b.addEventListener('mousedown',function(e){{e.preventDefault();e.stopPropagation();go('suggest/'+i)}});card.appendChild(b)}});         root.appendChild(card);document.documentElement.appendChild(host)}})();",
        open(theme, token, "__ax_pop_suggest"),
        data,
        selected,
        css_literal(&extra, 6.0, width),
    )
}

const PW_PROMPT_JS: &str = r#"
var d=__DATA__;
function send(a){try{window.chrome.webview.postMessage({ax:'pw',k:'confirm',a:a})}catch(e){}host.remove()}
var bar=document.createElement('div');bar.className='bar';
var tx=document.createElement('div');tx.className='tx';var b=document.createElement('b');b.textContent=d[0];var i=document.createElement('i');i.textContent='\u{1F511} '+d[1];tx.appendChild(b);tx.appendChild(i);bar.appendChild(tx);
function btn(t,cls,a){var e=document.createElement('button');e.textContent=t;e.className=cls;e.onclick=function(){send(a)};bar.appendChild(e)}
btn(d[2],'p','save');btn('Never','g','never');btn('Not now','g','no');
css('.bar{position:fixed;top:10px;right:24px;display:flex;align-items:center;gap:8px;background:var(--bg);border:1px solid var(--border);border-radius:14px;box-shadow:var(--shadow);padding:12px 14px;color:var(--text);font-size:13px;max-width:min(560px,92vw)}'+
'.tx{display:flex;flex-direction:column;margin-right:8px;min-width:0}.tx b{color:var(--heading);font-size:13.5px}.tx i{font-style:normal;color:var(--muted);font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}'+
'button{border:0;border-radius:10px;padding:8px 14px;font:600 12.5px inherit;font-family:inherit;cursor:pointer}.p{background:var(--primary);color:#fff}.g{background:var(--hover);color:var(--heading)}');
root.appendChild(bar);document.documentElement.appendChild(host);
"#;

const PW_OFFER_JS: &str = r#"
var d=__DATA__;
var card=document.createElement('div');card.className='card';
var hd=document.createElement('div');hd.className='hd';hd.textContent='\u{1F511} Saved passwords for '+d.host;card.appendChild(hd);
d.users.forEach(function(u,i){var r=document.createElement('div');r.className='row';r.textContent=u;
 r.addEventListener('mousedown',function(e){e.preventDefault();e.stopPropagation();try{window.chrome.webview.postMessage({ax:'pw',k:'fill',i:i})}catch(x){}host.remove()});card.appendChild(r)});
css('.card{position:fixed;left:'+Math.max(6,Math.min(d.x,innerWidth-Math.max(240,d.w)-12))+'px;top:'+(d.y+4)+'px;min-width:'+Math.max(240,d.w)+'px;background:var(--bg);border:1px solid var(--border);border-radius:12px;box-shadow:var(--shadow);padding:6px;color:var(--text);font-size:13px}'+
'.hd{padding:6px 10px;font-size:11.5px;color:var(--muted)}.row{padding:9px 10px;border-radius:8px;cursor:pointer;color:var(--heading);font-weight:600}.row:hover{background:var(--hover)}');
root.appendChild(card);document.documentElement.appendChild(host);
document.addEventListener('mousedown',function m(e){if(!e.composedPath().includes(host)){host.remove();document.removeEventListener('mousedown',m,true)}},true);
document.addEventListener('keydown',function k(e){if(e.key==='Escape'){host.remove();document.removeEventListener('keydown',k,true)}},true);
"#;

/// "Save password?" bar. Its buttons post `{ax:'pw',k:'confirm',a}`; the browser decides what that means.
pub fn pw_prompt(theme: &Theme, host: &str, user: &str, update: bool) -> String {
    let title = if update { format!("Update password for {}?", host) } else { format!("Save password for {}?", host) };
    let shown_user = if user.is_empty() { "(no username)" } else { user };
    let data = json!([title, shown_user, if update { "Update" } else { "Save" }]);
    format!("(function(){{{}{}}})();", shell(theme, "__ax_pop_pwsave"), PW_PROMPT_JS.replace("__DATA__", &data.to_string()))
}

/// Dropdown of saved usernames under a login field; choosing one posts `{ax:'pw',k:'fill',i}`.
pub fn pw_offer(theme: &Theme, host: &str, x: f64, y: f64, w: f64, users: &[String]) -> String {
    let data = json!({"host": host, "x": x, "y": y, "w": w, "users": users});
    format!(
        "(function(){{var __o=document.getElementById('__ax_pop_pw');if(__o)__o.remove();{}{}}})();",
        shell(theme, "__ax_pop_pw"),
        PW_OFFER_JS.replace("__DATA__", &data.to_string())
    )
}

/// Put a login into the page's form: the visible password field, and the visible text field before it.
pub fn pw_fill(user: &str, pass: &str) -> String {
    format!(
        r#"(function(u,p){{
function shown(e){{return !!(e.offsetWidth||e.offsetHeight||e.getClientRects().length)}}
var pf=Array.prototype.filter.call(document.querySelectorAll('input[type=password]'),shown)[0];if(!pf)return;
var scope=pf.form||document,all=scope.querySelectorAll('input:not([type]),input[type=text],input[type=email],input[type=tel]'),uf=null;
for(var i=0;i<all.length;i++){{if(shown(all[i])&&(all[i].compareDocumentPosition(pf)&4))uf=all[i]}}
var set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
function put(el,v){{set.call(el,v);el.dispatchEvent(new Event('input',{{bubbles:true}}));el.dispatchEvent(new Event('change',{{bubbles:true}}))}}
if(uf&&u)put(uf,u);put(pf,p);
var h=document.getElementById('__ax_pop_pw');if(h)h.remove();
}})({},{});"#,
        serde_json::to_string(user).unwrap_or_else(|_| "\"\"".into()),
        serde_json::to_string(pass).unwrap_or_else(|_| "\"\"".into())
    )
}

const GROUP_PROMPT_JS: &str = r#"
var d=__DATA__;var sel=3;
css('.card input{width:100%;border:1px solid var(--border);border-radius:10px;padding:9px 12px;background:transparent;color:var(--heading);font:inherit;outline:none;margin:2px 0 10px}.card input:focus{border-color:var(--primary)}'+
 '.sw2{display:flex;gap:8px;padding:2px 2px 12px}.sw2 button{width:30px;height:30px;border-radius:50%;border:3px solid transparent;cursor:pointer}.sw2 button.on{border-color:var(--heading)}'+
 '.acts2{display:flex;gap:8px;justify-content:flex-end}');
var card=el('div','card');card.style.padding='14px 16px';
card.appendChild(el('b','','New tab group'));card.firstChild.style.cssText='display:block;margin-bottom:10px;color:var(--heading)';
var inp=document.createElement('input');inp.placeholder='Group name (optional)';inp.maxLength=24;card.appendChild(inp);
var sw=el('div','sw2');var btns=[];
d.colors.forEach(function(c,i){var b=document.createElement('button');b.style.background='rgb('+c[1]+')';b.title=c[0];b.onclick=function(){sel=i;paint()};sw.appendChild(b);btns.push(b)});
function paint(){btns.forEach(function(b,i){b.className=i===sel?'on':''})}paint();card.appendChild(sw);
var acts=el('div','acts2');var cancel=el('button','btn ghost','Cancel');cancel.onclick=function(){host.remove()};var ok=el('button','btn','Create group');
function create(){var n=inp.value.trim()||d.colors[sel][0];host.remove();go('tab-group-new/'+d.tab+'/'+sel+'/'+encodeURIComponent(n))}
ok.onclick=create;acts.appendChild(cancel);acts.appendChild(ok);card.appendChild(acts);
inp.addEventListener('keydown',function(e){e.stopPropagation();if(e.key==='Enter'){e.preventDefault();create()}else if(e.key==='Escape'){host.remove()}},true);
finish(card);inp.focus();
"#;

const TAB_SEARCH_JS: &str = r#"
var d=__DATA__;var sel=0,shown=[];
css('.card{padding:10px}.card input{width:100%;border:1px solid var(--border);border-radius:12px;padding:11px 14px;background:transparent;color:var(--heading);font:inherit;font-size:14px;outline:none;margin-bottom:8px}.card input:focus{border-color:var(--primary)}'+
 '.row.sel{background:var(--hover)}.row .em{background:var(--primary-light);color:var(--primary);border-radius:8px;font-weight:800;font-size:13px;width:28px;height:28px;display:flex;align-items:center;justify-content:center}'+
 '.tag{font-size:10.5px;font-weight:700;border-radius:99px;padding:2px 8px;color:#fff;margin-left:6px}.lst{max-height:360px;overflow:auto}');
var card=el('div','card');var inp=document.createElement('input');inp.placeholder='Search your open tabs';card.appendChild(inp);
var list=el('div','lst');card.appendChild(list);
function pick(r){host.remove();go('tab-go/'+r[0])}
function render(){var q=inp.value.toLowerCase().trim();list.textContent='';
 shown=d.filter(function(r){return (r[1]+' '+r[2]+' '+r[3]).toLowerCase().indexOf(q)>=0});if(sel>=shown.length)sel=0;
 shown.forEach(function(r,i){var b=el('button','row'+(i===sel?' sel':''));b.appendChild(el('span','em',(r[1]||r[2]||'?').charAt(0).toUpperCase()));
  var tx=el('span','tx');var t=el('b','',r[1]||r[2]);if(r[5])t.textContent+='  \u00b7 current';tx.appendChild(t);tx.appendChild(el('i','',r[2]));b.appendChild(tx);
  if(r[3]){var g=el('span','tag',r[3]);g.style.background='rgb('+r[4]+')';b.appendChild(g)}
  b.onclick=function(){pick(r)};list.appendChild(b)});
 if(!shown.length)list.appendChild(el('div','sub','No tab matches'))}
inp.addEventListener('input',function(){sel=0;render()});
inp.addEventListener('keydown',function(e){e.stopPropagation();
 if(e.key==='ArrowDown'){e.preventDefault();sel=Math.min(sel+1,shown.length-1);render()}
 else if(e.key==='ArrowUp'){e.preventDefault();sel=Math.max(sel-1,0);render()}
 else if(e.key==='Enter'){e.preventDefault();if(shown[sel])pick(shown[sel])}
 else if(e.key==='Escape'){host.remove()}},true);
render();finish(card);inp.focus();
"#;

/// Asks for a tab group's name and colour; creating it posts `tab-group-new/<tab>/<colour>/<name>`.
pub fn group_prompt(theme: &Theme, token: &str, tab_id: u64) -> String {
    let colors: Vec<_> = crate::tabgroups::GROUP_COLORS.iter().map(|(n, _, c)| json!([n, format!("{},{},{}", c[0], c[1], c[2])])).collect();
    let data = json!({"tab": tab_id, "colors": colors});
    let mut js = open(theme, token, "__ax_pop_group");
    js.push_str(&css_wrap(".card{left:calc(50% - 150px);top:70px}", 300.0));
    js.push_str(&GROUP_PROMPT_JS.replace("__DATA__", &data.to_string()));
    js.push_str("})();");
    js
}

/// Search box over the list of open tabs (Ctrl+Shift+A); choosing one posts `tab-go/<id>`.
pub fn tab_search_popup(theme: &Theme, token: &str, rows: &[(u64, String, String, String, [u8; 3], bool)]) -> String {
    let data: Vec<_> = rows.iter().map(|(id, t, u, g, c, a)| json!([id, truncate(t, 80), truncate(u, 90), g, format!("{},{},{}", c[0], c[1], c[2]), a])).collect();
    let mut js = open(theme, token, "__ax_pop_tabsearch");
    js.push_str(&css_wrap(".card{left:calc(50% - 270px);top:70px}", 540.0));
    js.push_str(&TAB_SEARCH_JS.replace("__DATA__", &serde_json::to_string(&data).unwrap_or_else(|_| "[]".into())));
    js.push_str("})();");
    js
}

/// `css(...)` call for a popup placed with its own `.card` rule.
fn css_wrap(extra: &str, width: f32) -> String {
    format!("\ncss({});\n", css_literal(extra, 6.0, width))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n).collect();
        t.push('…');
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced(js: &str) -> bool {
        js.matches('{').count() == js.matches('}').count() && js.matches('(').count() == js.matches(')').count()
    }

    #[test]
    fn popups_are_syntactically_balanced() {
        let t = crate::theme::by_id("tea-garden");
        let items = vec![ExtItem { idx: 0, emoji: "🛡️", name: "AdBlock", status: "12 blocked".into(), enabled: true }];
        assert!(balanced(&extensions_popup(t, "tok", 80.0, &items)));
        assert!(balanced(&menu_popup(t, "tok", 14.0, &[MenuEntry { emoji: "➕", label: "New Tab", cmd: "newtab", danger: false }])));
        let sv = ShieldView { host: "example.com".into(), page_blocked: 3, total_blocked: 99, adblock: true, privacy: false };
        assert!(balanced(&shield_popup(t, "tok", 200.0, &sv)));
        let pv = ProfileView { name: "Asha".into(), bookmarks: 1, history: 2, downloads: 3, blocked_total: 4 };
        assert!(balanced(&profile_popup(t, "tok", 60.0, &pv)));
        assert!(balanced(&theme_popup(t, "tok", 150.0)));
        assert!(balanced(&site_info_popup(t, "tok", 300.0, crate::toolbar::Security::Insecure, "example.com", 2)));
        assert!(balanced(&ai_popup(t, "tok", 100.0, "example.com", "A title")));
        assert!(balanced(&qr_popup(t, "tok", 300.0, "https://example.com", 2, &[true, false, false, true])));
        assert!(balanced(&toast(t, "Saved", Some(("Open", "tok", "open-folder")))));
    }

    #[test]
    fn user_text_is_json_escaped() {
        let t = crate::theme::by_id("tea-garden");
        let js = ai_popup(t, "tok", 10.0, "evil\"</script>.com", "x'; alert(1); '");
        assert!(js.contains("evil\\\"</script>.com"));
        // Single quotes are harmless inside a JSON (double-quoted) literal; they must never appear as a bare JS string.
        assert!(js.contains("\"x'; alert(1); '\""));
    }

    #[test]
    fn ai_result_serialises_payload() {
        let js = ai_result("Summary", &["one".into()], &["tea".into()]);
        assert!(js.contains("\"paragraphs\":[\"one\"]"));
    }
}
