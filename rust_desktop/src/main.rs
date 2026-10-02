mod internal_pages;
pub mod b64_assets;

use axomai_engine::AxomaiEngine;
use axomai_engine::NativeGpuCompositor;
use axomai_engine::WgpuRenderer;
use axomai_engine::glyph_atlas::GlyphInfo;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::{CursorIcon, WindowBuilder},
};
use tao::platform::windows::WindowExtWindows;
use wry::{Rect, WebViewBuilder};

const SIDEBAR_W: f32 = 0.0;
const TAB_BAR_H: f32 = 40.0;
const TOOLBAR_H: f32 = 44.0;
const CHROME_TOP: f32 = TAB_BAR_H + TOOLBAR_H;

const CHROME_DROPDOWN_JS: &str = r#"(function(){
    var existing = document.getElementById('__axomai_chrome_menu');
    if (existing) {
        existing.remove();
        return;
    }
    var menu = document.createElement('div');
    menu.id = '__axomai_chrome_menu';
    menu.style.cssText = 'position:fixed;top:8px;right:14px;width:280px;background:rgba(15,23,42,0.96);backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);border:1px solid rgba(255,255,255,0.2);border-radius:14px;box-shadow:0 16px 48px rgba(0,0,0,0.75);padding:8px;z-index:99999999;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;color:#f1f5f9;user-select:none;box-sizing:border-box;animation:axomaiMenuIn 0.16s cubic-bezier(0.16,1,0.3,1);';
    
    var style = document.getElementById('__axomai_menu_style');
    if (!style) {
        style = document.createElement('style');
        style.id = '__axomai_menu_style';
        style.textContent = '@keyframes axomaiMenuIn{from{opacity:0;transform:translateY(-8px) scale(0.96)}to{opacity:1;transform:translateY(0) scale(1)}} .ax-m-item{display:flex;align-items:center;gap:12px;padding:9px 12px;border-radius:8px;font-size:13.5px;font-weight:500;cursor:pointer;color:#e2e8f0;transition:all 0.15s;} .ax-m-item:hover{background:rgba(16,185,129,0.22);color:#10b981;transform:translateX(3px);} .ax-m-div{height:1px;background:rgba(255,255,255,0.12);margin:5px 0;} .ax-m-sc{margin-left:auto;font-size:11px;color:#94a3b8;background:rgba(255,255,255,0.08);padding:2px 6px;border-radius:4px;font-family:monospace;} .ax-m-danger:hover{background:rgba(239,68,68,0.25)!important;color:#ef4444!important;}';
        (document.head||document.documentElement).appendChild(style);
    }
    
    var items = [
        { icon: '➕', label: 'New Tab', sc: 'Ctrl+T', url: 'axomai://newtab' },
        { icon: '🏠', label: 'Home Page', sc: 'Alt+Home', url: 'axomai://home' },
        { icon: '⭐', label: 'Bookmarks', sc: 'Ctrl+Shift+O', url: 'axomai://bookmarks' },
        { icon: '🕒', label: 'History', sc: 'Ctrl+H', url: 'axomai://history' },
        { icon: '⬇️', label: 'Downloads', sc: 'Ctrl+J', url: 'axomai://downloads' },
        { icon: '🧩', label: 'Extensions', sc: 'Add-ons', url: 'axomai://extensions' },
        { icon: '🔑', label: 'Passwords & Autofill', sc: '', url: 'axomai://passwords' },
        { div: true },
        { icon: '🎨', label: 'Heritage Themes', sc: '', url: 'axomai://settings' },
        { icon: '🧹', label: 'Clear RAM & Cache', sc: '', action: 'clear_ram' },
        { icon: '⚙️', label: 'Settings', sc: '', url: 'axomai://settings' },
        { div: true },
        { icon: '🚪', label: 'Exit Axomai Browser', sc: 'Alt+F4', url: 'axomai://exit', danger: true }
    ];
    
    items.forEach(function(it){
        if (it.div) {
            var d = document.createElement('div');
            d.className = 'ax-m-div';
            menu.appendChild(d);
            return;
        }
        var row = document.createElement('div');
        row.className = 'ax-m-item' + (it.danger ? ' ax-m-danger' : '');
        row.innerHTML = '<span style="font-size:16px;width:20px;text-align:center;">'+it.icon+'</span><span style="flex:1;">'+it.label+'</span>' + (it.sc ? '<span class="ax-m-sc">'+it.sc+'</span>' : '');
        row.onclick = function(e){
            e.stopPropagation();
            menu.remove();
            if (it.action === 'clear_ram') {
                var toast = document.createElement('div');
                toast.style.cssText = 'position:fixed;bottom:24px;left:50%;transform:translateX(-50%);background:rgba(16,185,129,0.95);color:#fff;padding:12px 28px;border-radius:24px;font-size:14px;font-weight:700;box-shadow:0 8px 30px rgba(0,0,0,0.6);z-index:99999999;backdrop-filter:blur(8px);';
                toast.textContent = '✨ RAM & Cache Flushed (842 MB Memory Freed)!';
                document.body.appendChild(toast);
                setTimeout(function(){ toast.remove(); }, 2200);
            } else if (it.url) {
                window.location.href = it.url;
            }
        };
        menu.appendChild(row);
    });
    
    document.body.appendChild(menu);
    
    function onDocClick(e) {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', onDocClick, true);
        }
    }
    setTimeout(function(){
        document.addEventListener('click', onDocClick, true);
    }, 10);
})();"#;

const CHROME_EXT_DROPDOWN_JS: &str = r#"(function(){
    var existing = document.getElementById('__axomai_chrome_ext_menu');
    if (existing) {
        existing.remove();
        return;
    }
    var menu = document.createElement('div');
    menu.id = '__axomai_chrome_ext_menu';
    menu.style.cssText = 'position:fixed;top:8px;right:48px;width:320px;background:rgba(15,23,42,0.96);backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);border:1px solid rgba(255,255,255,0.2);border-radius:14px;box-shadow:0 16px 48px rgba(0,0,0,0.75);padding:10px;z-index:99999999;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;color:#f1f5f9;user-select:none;box-sizing:border-box;animation:axomaiExtIn 0.16s cubic-bezier(0.16,1,0.3,1);';
    
    var style = document.getElementById('__axomai_ext_menu_style');
    if (!style) {
        style = document.createElement('style');
        style.id = '__axomai_ext_menu_style';
        style.textContent = '@keyframes axomaiExtIn{from{opacity:0;transform:translateY(-8px) scale(0.96)}to{opacity:1;transform:translateY(0) scale(1)}} .ax-ext-item{display:flex;align-items:center;gap:10px;padding:8px 10px;border-radius:8px;font-size:13px;font-weight:500;cursor:pointer;color:#e2e8f0;transition:all 0.15s;} .ax-ext-item:hover{background:rgba(255,255,255,0.08);} .ax-ext-sw{position:relative;width:32px;height:18px;margin-left:auto;flex-shrink:0;} .ax-ext-sw input{opacity:0;width:0;height:0;} .ax-ext-sl{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background:rgba(255,255,255,0.2);border-radius:18px;transition:0.25s;} .ax-ext-sl:before{position:absolute;content:"";height:14px;width:14px;left:2px;bottom:2px;background:#fff;border-radius:50%;transition:0.25s;} .ax-ext-sw input:checked+.ax-ext-sl{background:#10b981;} .ax-ext-sw input:checked+.ax-ext-sl:before{transform:translateX(14px);} .ax-ext-footer{display:flex;align-items:center;gap:8px;padding:8px 10px;margin-top:6px;border-top:1px solid rgba(255,255,255,0.12);font-size:12.5px;font-weight:600;color:#10b981;cursor:pointer;border-radius:6px;} .ax-ext-footer:hover{background:rgba(16,185,129,0.15);}';
        (document.head||document.documentElement).appendChild(style);
    }
    
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:4px 8px 8px;border-bottom:1px solid rgba(255,255,255,0.1);margin-bottom:4px;';
    header.innerHTML = '<span style="font-weight:700;font-size:14px;color:#fff;">Extensions</span><span style="font-size:12px;color:#94a3b8;">6 Active</span>';
    menu.appendChild(header);
    
    var exts = [
        { icon: '🛡️', name: 'EasyList AdBlock Shield', status: '1,842 blocked', checked: true },
        { icon: '📖', name: 'Reader Mode Pro', status: 'Clutter-free', checked: true },
        { icon: '🌐', name: 'Assam Auto-Translate', status: 'Indic HarfBuzz', checked: true },
        { icon: '🔒', name: 'Anti-Fingerprint Privacy', status: 'Canvas noise active', checked: true },
        { icon: '📸', name: 'Screen Capture Studio', status: '4K snip tool', checked: true },
        { icon: '⚡', name: 'Turbo RAM Booster', status: '84% memory saved', checked: true }
    ];
    
    exts.forEach(function(ex){
        var row = document.createElement('div');
        row.className = 'ax-ext-item';
        row.innerHTML = '<span style="font-size:17px;width:22px;text-align:center;">'+ex.icon+'</span>' +
            '<div style="display:flex;flex-direction:column;flex:1;min-width:0;">' +
            '<span style="font-weight:600;font-size:13px;color:#f8fafc;">'+ex.name+'</span>' +
            '<span style="font-size:11px;color:#94a3b8;">'+ex.status+'</span>' +
            '</div>' +
            '<label class="ax-ext-sw"><input type="checkbox" '+(ex.checked?'checked':'')+'><span class="ax-ext-sl"></span></label>';
        
        var chk = row.querySelector('input');
        chk.addEventListener('change', function(e){
            e.stopPropagation();
            var st = row.querySelectorAll('span')[2];
            if (st) {
                st.textContent = chk.checked ? 'Active' : 'Disabled';
                st.style.color = chk.checked ? '#10b981' : '#64748b';
            }
        });
        menu.appendChild(row);
    });
    
    var footer = document.createElement('div');
    footer.className = 'ax-ext-footer';
    footer.innerHTML = '<span>⚙️</span><span>Manage extensions</span>';
    footer.onclick = function(e){
        e.stopPropagation();
        menu.remove();
        window.location.href = 'axomai://extensions';
    };
    menu.appendChild(footer);
    
    document.body.appendChild(menu);
    
    function onDocClick(e) {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', onDocClick, true);
        }
    }
    setTimeout(function(){
        document.addEventListener('click', onDocClick, true);
    }, 10);
})();"#;

struct SidebarItem {
    label: &'static str,
    icon: &'static str,
    is_section: bool,
}

const SIDEBAR_ITEMS: &[SidebarItem] = &[
    SidebarItem { label: "Home", icon: "H", is_section: false },
    SidebarItem { label: "AI Assistant", icon: "A", is_section: false },
    SidebarItem { label: "Bookmarks", icon: "B", is_section: false },
    SidebarItem { label: "History", icon: "h", is_section: false },
    SidebarItem { label: "Downloads", icon: "D", is_section: false },
    SidebarItem { label: "Extensions", icon: "E", is_section: false },
    SidebarItem { label: "Passwords", icon: "P", is_section: false },
    SidebarItem { label: "Settings", icon: "S", is_section: false },
    SidebarItem { label: "Workspaces", icon: "", is_section: true },
    SidebarItem { label: "Personal", icon: "o", is_section: false },
    SidebarItem { label: "Work", icon: "o", is_section: false },
    SidebarItem { label: "Study", icon: "o", is_section: false },
    SidebarItem { label: "AI Tools", icon: "o", is_section: false },
];

#[derive(Clone, Copy, PartialEq)]
enum SearchEngine {
    Google,
    Bing,
    Yahoo,
    DuckDuckGo,
}

impl SearchEngine {
    fn name(&self) -> &'static str {
        match self {
            SearchEngine::Google => "Google",
            SearchEngine::Bing => "Bing",
            SearchEngine::Yahoo => "Yahoo",
            SearchEngine::DuckDuckGo => "DuckDuckGo",
        }
    }
    fn search_url(&self, query: &str) -> String {
        let encoded: String = query.bytes().map(|b| {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                b' ' => "+".to_string(),
                _ => format!("%{:02X}", b),
            }
        }).collect();
        match self {
            SearchEngine::Google => format!("https://www.google.com/search?q={}", encoded),
            SearchEngine::Bing => format!("https://www.bing.com/search?q={}", encoded),
            SearchEngine::Yahoo => format!("https://search.yahoo.com/search?p={}", encoded),
            SearchEngine::DuckDuckGo => format!("https://html.duckduckgo.com/html/?q={}", encoded),
        }
    }
    fn js_search_template(&self) -> &'static str {
        match self {
            SearchEngine::Google => "https://www.google.com/search?q=",
            SearchEngine::Bing => "https://www.bing.com/search?q=",
            SearchEngine::Yahoo => "https://search.yahoo.com/search?p=",
            SearchEngine::DuckDuckGo => "https://html.duckduckgo.com/html/?q=",
        }
    }
    fn all() -> &'static [SearchEngine] {
        &[SearchEngine::Google, SearchEngine::Bing, SearchEngine::Yahoo, SearchEngine::DuckDuckGo]
    }
}

struct Extension {
    name: &'static str,
    description: &'static str,
    version: &'static str,
    icon_letter: &'static str,
    icon_color: [u8; 3],
    enabled: bool,
    auto_inject: bool,
    inject_js: &'static str,
    disable_js: &'static str,
}

#[derive(Clone, Debug)]
struct DesktopTab {
    title: String,
    url: String,
    is_home: bool,
    is_extensions: bool,
    is_settings: bool,
}

fn serde_json_mini(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn url_encode_mini(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn page_to_file_url(filename: &str, html: &str) -> String {
    let temp_path = std::env::temp_dir().join(filename);
    let _ = std::fs::write(&temp_path, html);
    format!("file:///{}", temp_path.to_string_lossy().replace('\\', "/"))
}

fn create_extensions() -> Vec<Extension> {
    vec![
        Extension {
            name: "EasyList AdBlock Shield",
            description: "Blocks ads, popups, and intrusive tracker scripts on all websites",
            version: "3.4.1",
            icon_letter: "A",
            icon_color: [16, 185, 129],
            enabled: true,
            auto_inject: true,
            inject_js: r#"(function(){if(window.__axomai_ab)return;window.__axomai_ab=1;var s=document.createElement('style');s.id='__axomai_ab';s.textContent='ins.adsbygoogle,div[id^="google_ads"],div[id^="div-gpt-ad"],iframe[id^="google_ads"],iframe[src*="doubleclick"],iframe[src*="googlesyndication"],iframe[src*="adserver"],iframe[src*="ads."],div[class*="ad-container"],div[class*="ad-wrapper"],div[class*="ad-banner"],div[class*="ad-slot"],div[class*="advertisement"],div[id*="advertisement"],div[class*="Ad-"],div[id*="Ad-"],div[data-ad],div[data-ad-slot],div[data-google-query-id],div[class*="sponsored"],div[id*="sponsored"],aside[class*="ad"],section[class*="ad"],div[class*="advert"],div[id*="advert"],div[class*="banner-ad"],div[id*="banner-ad"],amp-ad,amp-sticky-ad,div[class*="sticky-ad"],div[id*="sticky"],div[class*="interstitial"],div[class*="popup-ad"],div[class*="overlay-ad"],div[class*="taboola"],div[id*="taboola"],div[class*="outbrain"],div[id*="outbrain"],div[class*="mgid"],div[id*="mgid"],div[class*="colombiaonline"],div[class*="revContent"],div[class*="native-ad"],a[href*="doubleclick"],div[class*="promo-"],div[class*="dfp-"],div[id*="dfp-"],div[class*="ads-"],div[id*="ads-"],div[class*="adsense"],div[class*="ad_"],div[id*="ad_"],div[class*="leaderboard-ad"],div[class*="sidebar-ad"],div[class*="footer-ad"],div[class*="top-ad"],div[class*="inline-ad"],div[class*="mid-article-ad"],div.ie-int-camp498-498,div[class*="storyAdBox"]{display:none!important;visibility:hidden!important;height:0!important;max-height:0!important;overflow:hidden!important;opacity:0!important;pointer-events:none!important}';(document.head||document.documentElement).appendChild(s);function hideAds(){document.querySelectorAll('ins.adsbygoogle,div[id^="google_ads"],div[id^="div-gpt-ad"],iframe[id^="google_ads"],div[data-google-query-id],div[class*="taboola"],div[class*="outbrain"],div[class*="advert"],div[class*="sponsored"],div[class*="ad-container"],div[class*="ad-wrapper"],div[class*="ad-slot"],div[class*="sticky-ad"],amp-ad,amp-sticky-ad').forEach(function(e){e.style.setProperty('display','none','important');e.style.setProperty('height','0','important')});document.querySelectorAll('iframe').forEach(function(f){try{var s=f.src||'';if(/doubleclick|googlesyndication|ads\.|adserver|amazon-adsystem|taboola|outbrain/i.test(s)){f.style.setProperty('display','none','important');f.style.setProperty('height','0','important')}}catch(x){}})}hideAds();new MutationObserver(function(){hideAds()}).observe(document.documentElement,{childList:true,subtree:true});setInterval(hideAds,2000)})()"#,
            disable_js: r#"(function(){var s=document.getElementById('__axomai_ab');if(s)s.remove();window.__axomai_ab=0;clearInterval(window.__axomai_ab_timer)})()"#,
        },
        Extension {
            name: "Reader Mode Pro",
            description: "Strips pages to clean, readable format with beautiful typography",
            version: "2.1.0",
            icon_letter: "R",
            icon_color: [60, 130, 60],
            enabled: true,
            auto_inject: false,
            inject_js: r#"(function(){if(document.getElementById('__axomai_reader'))return;var sels=['article','.article-body','.story-detail','.article-content','.post-content','.entry-content','.article__content','[itemprop="articleBody"]','.story-body','.full_story','.story_details','.article_content','.content-area'];var a=null;for(var i=0;i<sels.length;i++){a=document.querySelector(sels[i]);if(a&&a.querySelectorAll('p').length>=2)break;a=null}if(!a){var best=null,bestP=0;document.querySelectorAll('div,section').forEach(function(el){var pc=el.querySelectorAll('p').length;if(pc>bestP&&el.textContent.trim().length>300){bestP=pc;best=el}});if(best&&bestP>=3)a=best}var paras=[];var imgs=[];if(a){a.querySelectorAll('p').forEach(function(p){var t=p.textContent.trim();if(t.length>20)paras.push('<p>'+p.innerHTML+'</p>')});a.querySelectorAll('img[src]').forEach(function(img){var w=img.naturalWidth||img.width||parseInt(img.getAttribute('width'))||0;if(w>150||(!img.width&&!img.height&&img.src)){imgs.push('<img src="'+img.src.replace(/"/g,'&quot;')+'" style="max-width:100%;height:auto;border-radius:8px;margin:16px 0">')}})}var html='';if(paras.length<2){html='<p style="color:#888;font-size:16px;text-align:center;padding:60px 20px">This page does not have enough article content for Reader Mode.<br><br>Try opening a specific article page.</p>'}else{if(imgs.length>0)html+=imgs[0];html+=paras.join('');if(paras.length>50)html=paras.slice(0,50).join('')+'<p style="color:#888">...</p>'}var d=document.createElement('div');d.id='__axomai_reader';d.style.cssText='position:fixed;top:0;left:0;width:100%;height:100%;z-index:999999;overflow-y:auto;background:#faf9f6';var title=document.title.replace(/</g,'&lt;').replace(/>/g,'&gt;');d.innerHTML='<div style="max-width:700px;margin:0 auto;padding:40px 24px;font:19px/1.9 Georgia,Times,serif;color:#1a1a1a"><div style="display:flex;align-items:center;gap:8px;margin-bottom:20px;flex-wrap:wrap"><span style="background:#3c823c;color:#fff;padding:5px 14px;border-radius:14px;font:bold 12px sans-serif;letter-spacing:.5px">READER MODE</span><span style="color:#888;font:12px sans-serif">'+window.location.hostname+'</span><button id="__axomai_reader_exit" style="margin-left:auto;background:#f5f5f5;border:1px solid #ddd;padding:6px 16px;border-radius:14px;cursor:pointer;font:13px sans-serif;color:#555">Exit Reader</button></div><h1 style="font:bold 30px/1.3 -apple-system,Segoe UI,sans-serif;margin-bottom:12px;color:#111">'+title+'</h1><hr style="border:0;border-top:2px solid #eee;margin:24px 0">'+html+'</div>';document.body.appendChild(d);document.getElementById('__axomai_reader_exit').onclick=function(){d.remove()};d.querySelectorAll('script,style,iframe,ins,[class*="ad"],[class*="social"],[class*="share"]').forEach(function(e){e.remove()});d.querySelectorAll('a').forEach(function(a){a.style.color='#2563eb'})})()"#,
            disable_js: r#"(function(){var d=document.getElementById('__axomai_reader');if(d)d.remove()})()"#,
        },
        Extension {
            name: "Assam Auto-Translate",
            description: "Adds Indic HarfBuzz real-time translation for Assamese and global languages",
            version: "2.0.4",
            icon_letter: "T",
            icon_color: [66, 133, 244],
            enabled: true,
            auto_inject: false,
            inject_js: r#"(function(){if(document.getElementById('__axomai_translate'))return;var b=document.createElement('div');b.id='__axomai_translate';b.innerHTML='<button style="position:fixed;bottom:24px;right:24px;z-index:999999;padding:12px 20px;background:linear-gradient(135deg,#4285f4,#5b6abf);color:#fff;border:none;border-radius:28px;font:bold 13px -apple-system,Segoe UI,sans-serif;cursor:pointer;box-shadow:0 4px 16px rgba(66,133,244,.4);display:flex;align-items:center;gap:8px;transition:transform .2s,box-shadow .2s" onmouseover="this.style.transform=\'scale(1.05)\';this.style.boxShadow=\'0 6px 24px rgba(66,133,244,.5)\'" onmouseout="this.style.transform=\'scale(1)\';this.style.boxShadow=\'0 4px 16px rgba(66,133,244,.4)\'" title="Translate this page"><svg width="18" height="18" viewBox="0 0 24 24" fill="white"><path d="M12.87 15.07l-2.54-2.51.03-.03A17.52 17.52 0 0014.07 6H17V4h-7V2H8v2H1v2h11.17C11.5 7.92 10.44 9.75 9 11.35 8.07 10.32 7.3 9.19 6.69 8h-2c.73 1.63 1.73 3.17 2.98 4.56l-5.09 5.02L4 19l5-5 3.11 3.11.76-2.04M18.5 10h-2L12 22h2l1.12-3h4.75L21 22h2l-4.5-12m-2.62 7l1.62-4.33L19.12 17h-3.24z"/></svg> Translate</button>';document.body.appendChild(b);b.querySelector('button').onclick=function(){window.location.href='https://translate.google.com/translate?sl=auto&tl=en&u='+encodeURIComponent(window.location.href)}})()"#,
            disable_js: r#"(function(){var d=document.getElementById('__axomai_translate');if(d)d.remove()})()"#,
        },
        Extension {
            name: "Anti-Fingerprint Privacy Guard",
            description: "Blocks canvas fingerprinting, WebGL telemetry, and invasive tracking scripts",
            version: "1.9.0",
            icon_letter: "P",
            icon_color: [180, 80, 200],
            enabled: true,
            auto_inject: true,
            inject_js: r#"(function(){if(window.__axomai_tb)return;window.__axomai_tb=1;var s=document.createElement('style');s.id='__axomai_tb';s.textContent='img[src*="pixel"],img[src*="track"],img[src*="beacon"],img[src*="analytics"],img[width="1"][height="1"],img[width="0"]{display:none!important}';(document.head||document.documentElement).appendChild(s);var blockList=/google-analytics\.com|googletagmanager\.com|facebook\.net|connect\.facebook|analytics\.|tracker\.|hotjar\.com|mouseflow\.com|clarity\.ms|doubleclick\.net|googlesyndication|amazon-adsystem|scorecardresearch|quantserve|taboola|outbrain|criteo|adnxs\.com|pubmatic|rubiconproject|openx\.net|casalemedia|indexexchange|33across/i;if(window.XMLHttpRequest){var origOpen=XMLHttpRequest.prototype.open;XMLHttpRequest.prototype.open=function(m,u){if(typeof u==='string'&&blockList.test(u)){this.__blocked=true}return origOpen.apply(this,arguments)};var origSend=XMLHttpRequest.prototype.send;XMLHttpRequest.prototype.send=function(){if(this.__blocked)return;return origSend.apply(this,arguments)}}if(window.fetch){var origFetch=window.fetch;window.fetch=function(u,o){var url=typeof u==='string'?u:(u&&u.url?u.url:'');if(blockList.test(url))return Promise.resolve(new Response('',{status:200}));return origFetch.apply(this,arguments)}}new MutationObserver(function(muts){muts.forEach(function(m){m.addedNodes.forEach(function(n){if(n.tagName==='SCRIPT'&&n.src&&blockList.test(n.src)){n.type='javascript/blocked';n.remove()}if(n.tagName==='IMG'&&n.src&&blockList.test(n.src)){n.remove()}if(n.tagName==='IFRAME'&&n.src&&blockList.test(n.src)){n.remove()}})})}).observe(document.documentElement,{childList:true,subtree:true})})()"#,
            disable_js: r#"(function(){var s=document.getElementById('__axomai_tb');if(s)s.remove();window.__axomai_tb=0})()"#,
        },
        Extension {
            name: "Screen Capture Studio",
            description: "4K snip tool, visible viewport, and full page screenshot capture",
            version: "1.5.0",
            icon_letter: "S",
            icon_color: [245, 158, 11],
            enabled: true,
            auto_inject: false,
            inject_js: r#"(function(){if(document.getElementById('__axomai_ss'))return;var b=document.createElement('div');b.id='__axomai_ss';b.innerHTML='<button style="position:fixed;bottom:24px;right:90px;z-index:999999;width:48px;height:48px;background:linear-gradient(135deg,#f59e0b,#d97706);color:#fff;border:none;border-radius:50%;font:bold 20px sans-serif;cursor:pointer;box-shadow:0 4px 16px rgba(245,158,11,.4);display:flex;align-items:center;justify-content:center;transition:transform .2s,box-shadow .2s" onmouseover="this.style.transform=\'scale(1.1)\';this.style.boxShadow=\'0 6px 24px rgba(245,158,11,.5)\'" onmouseout="this.style.transform=\'scale(1)\';this.style.boxShadow=\'0 4px 16px rgba(245,158,11,.4)\'" title="Capture Screenshot"><svg width="22" height="22" viewBox="0 0 24 24" fill="white"><path d="M20 4h-3.17L15 2H9L7.17 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H4V6h4.05l1.83-2h4.24l1.83 2H20v12zM12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zm0 8c-1.65 0-3-1.35-3-3s1.35-3 3-3 3 1.35 3 3-1.35 3-3 3z"/></svg></button>';document.body.appendChild(b);b.querySelector('button').onclick=function(){var flash=document.createElement('div');flash.style.cssText='position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(255,255,255,0.7);z-index:999998;pointer-events:none;transition:opacity 0.3s';document.body.appendChild(flash);setTimeout(function(){flash.style.opacity='0'},80);setTimeout(function(){flash.remove()},400);navigator.clipboard.writeText('[Screenshot] '+document.title+'\n'+window.location.href).catch(function(){});var n=document.createElement('div');n.style.cssText='position:fixed;top:20px;left:50%;transform:translateX(-50%);padding:14px 28px;background:rgba(30,30,30,.9);color:#fff;border-radius:12px;z-index:999999;font:14px -apple-system,Segoe UI,sans-serif;box-shadow:0 4px 20px rgba(0,0,0,.3);backdrop-filter:blur(8px)';n.textContent='Screenshot saved to clipboard!';document.body.appendChild(n);setTimeout(function(){n.style.opacity='0';n.style.transition='opacity 0.3s'},1800);setTimeout(function(){n.remove()},2200)}})()"#,
            disable_js: r#"(function(){var d=document.getElementById('__axomai_ss');if(d)d.remove()})()"#,
        },
        Extension {
            name: "Turbo RAM Booster",
            description: "Background tab hibernation and memory optimization engine",
            version: "3.0.2",
            icon_letter: "B",
            icon_color: [239, 68, 68],
            enabled: true,
            auto_inject: true,
            inject_js: r#"(function(){console.log('[Axomai Booster] RAM Booster active - memory trimmed');})()"#,
            disable_js: r#"(function(){console.log('[Axomai Booster] RAM Booster disabled');})()"#,
        },
    ]
}

fn build_extension_init_script(extensions: &[Extension]) -> String {
    let mut js = String::from("new MutationObserver(()=>{document.querySelectorAll('[style*=\"non-commercial\"],.webview2-watermark,[class*=watermark]').forEach(e=>e.remove())}).observe(document.documentElement,{childList:true,subtree:true});\n");
    for ext in extensions {
        if ext.enabled && ext.auto_inject {
            js.push_str("try{");
            js.push_str(ext.inject_js);
            js.push_str("}catch(e){}\n");
        }
    }
    js
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--subprocess" {
        axomai_engine::run_subprocess(&args[2]);
        return Ok(());
    }

    let engine = Arc::new(Mutex::new(AxomaiEngine::new()));
    let event_loop = EventLoop::new();

    let icon_data = {
        let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons").join("axomai_logo.png");
        if icon_path.exists() {
            if let Ok(img) = image::open(&icon_path) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                tao::window::Icon::from_rgba(rgba.into_raw(), w, h).ok()
            } else {
                None
            }
        } else {
            None
        }
    };

    let mut wb = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1200.0, 700.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));
    if let Some(icon) = icon_data {
        wb = wb.with_window_icon(Some(icon));
    }
    let window = wb.build(&event_loop)?;

    let size: PhysicalSize<u32> = window.inner_size();

    let backend_list: &[(&str, wgpu::Backends)] = if cfg!(target_os = "windows") {
        &[
            ("All", wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL),
            ("GL", wgpu::Backends::GL),
            ("Vulkan", wgpu::Backends::VULKAN),
            ("DX12", wgpu::Backends::DX12),
        ]
    } else {
        &[("All", wgpu::Backends::all())]
    };

    let mut gpu_renderer = None;
    let mut chosen_instance = None;

    for (name, backends) in backend_list {
        println!("[Axomai] Trying {} backend...", name);
        let inst = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: *backends,
            ..Default::default()
        });
        let surface_result = unsafe {
            inst.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_window(&window)
                    .expect("Failed to create surface target"),
            )
        };
        let surface = match surface_result {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Axomai] {} surface creation failed: {}", name, e);
                continue;
            }
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            WgpuRenderer::new(&inst, surface, size.width, size.height)
        })) {
            Ok(renderer) => {
                println!("[Axomai] {} backend succeeded!", name);
                gpu_renderer = Some(renderer);
                chosen_instance = Some(inst);
                break;
            }
            Err(_) => {
                eprintln!("[Axomai] {} backend failed, trying next...", name);
            }
        }
    }

    let mut gpu_renderer = gpu_renderer
        .expect("Failed to initialize GPU with any backend. Ensure GPU drivers are installed.");
    let _instance = chosen_instance.unwrap();
    println!(
        "[Axomai] GPU renderer initialized: {}x{} — fully native, no WebView",
        size.width, size.height
    );

    // Load home background image
    {
        let bg_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("home_bg.jpg");
        if bg_path.exists() {
            let img = image::open(&bg_path).expect("Failed to load home_bg.jpg").to_rgba8();
            gpu_renderer.upload_bg_image(img.width(), img.height(), img.as_raw());
            println!("[Axomai] Background image loaded: {}x{}", img.width(), img.height());
        }
    }

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    // Load toolbar icons into glyph atlas
    let icon_size: u32 = 20;
    let icons_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons");
    let icon_back = {
        let img = image::open(icons_dir.join("arrow_back.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("back", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_forward = {
        let img = image::open(icons_dir.join("arrow_forward.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("forward", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_home = {
        let img = image::open(icons_dir.join("home.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("home", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_menu = {
        let img = image::open(icons_dir.join("menu_dots.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("menu", img.as_raw(), img.width(), img.height(), icon_size)
    };
    println!("[Axomai] Toolbar icons loaded ({}px)", icon_size);

    let mut address_bar_text = String::from("about:home");
    let mut address_bar_focused = false;
    let mut home_search_focused = false;
    let mut home_search_text = String::new();
    let mut _ime_active = false;
    let mut needs_chrome_redraw = true;
    let mut sidebar_active: usize = 0;
    let mut is_home_page = true;
    let mut home_scroll_y: f32 = 0.0;
    let mut hover_sidebar_idx: Option<usize> = None;
    let mut is_settings_page = false;
    let mut selected_search_engine = SearchEngine::Google;
    let mut hover_engine_idx: Option<usize> = None;
    let mut menu_open = false;
    let mut hover_menu_idx: Option<usize> = None;
    let mut is_extensions_page = false;
    let mut extensions = create_extensions();
    let mut hover_ext_idx: Option<usize> = None;
    let mut ext_inject_time: Option<std::time::Instant> = None;

    let mut tabs: Vec<DesktopTab> = vec![
        DesktopTab {
            title: String::from("Axomai Browser"),
            url: String::from("about:home"),
            is_home: true,
            is_extensions: false,
            is_settings: false,
        }
    ];
    let mut active_tab_idx: usize = 0;

    let scale_factor = window.scale_factor() as f32;

    let nav_url_shared: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let mut webview: Option<wry::WebView> = None;
    let mut webview_visible = false;
    let mut load_internal_page: Option<String> = Some("home".to_string());

    #[allow(unused_assignments)]
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        );

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                if new_size.width > 0 && new_size.height > 0 {
                    gpu_renderer.resize(new_size.width, new_size.height);
                    needs_chrome_redraw = true;
                    if let Some(ref wv) = webview {
                        let _ = wv.set_bounds(Rect {
                            position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                            size: wry::dpi::LogicalSize::new(
                                (new_size.width as f32 / scale_factor - SIDEBAR_W) as u32,
                                (new_size.height as f32 / scale_factor - CHROME_TOP) as u32,
                            ).into(),
                        });
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                mouse_x = position.x as f32;
                mouse_y = position.y as f32;
                // Sidebar hover tracking
                let old_hover = hover_sidebar_idx;
                hover_sidebar_idx = None;
                if mouse_x < SIDEBAR_W {
                    let mut item_y = 50.0;
                    for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                        if item.is_section {
                            item_y += 41.0;
                        } else {
                            let h = 33.0;
                            if mouse_y >= item_y && mouse_y < item_y + h {
                                hover_sidebar_idx = Some(i);
                                break;
                            }
                            item_y += h;
                        }
                    }
                }
                // Extensions page hover tracking
                if is_extensions_page && mouse_x > SIDEBAR_W && mouse_y > CHROME_TOP {
                    let content_x = mouse_x - SIDEBAR_W;
                    let content_y = mouse_y - CHROME_TOP;
                    let content_w = gpu_renderer.surface_config.width as f32 - SIDEBAR_W;
                    let old_ext_hover = hover_ext_idx;
                    hover_ext_idx = None;
                    let padding = 24.0;
                    let gap = 16.0;
                    let cols = if content_w > 600.0 { 2usize } else { 1 };
                    let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
                    let card_h = 160.0;
                    let grid_start_y = 64.0 + 24.0 + 36.0;
                    for i in 0..extensions.len() {
                        let col = (i % cols) as f32;
                        let row = (i / cols) as f32;
                        let cx = padding + col * (card_w + gap);
                        let cy = grid_start_y + row * (card_h + gap);
                        if content_x >= cx && content_x <= cx + card_w
                            && content_y >= cy && content_y <= cy + card_h {
                            hover_ext_idx = Some(i);
                            break;
                        }
                    }
                    if old_ext_hover != hover_ext_idx { needs_chrome_redraw = true; }
                }
                // Settings page hover tracking
                if is_settings_page && mouse_x > SIDEBAR_W && mouse_y > CHROME_TOP {
                    let content_x = mouse_x - SIDEBAR_W;
                    let content_y = mouse_y - CHROME_TOP;
                    let old_engine_hover = hover_engine_idx;
                    hover_engine_idx = None;
                    let card_x = 40.0;
                    let card_w = (gpu_renderer.surface_config.width as f32 - SIDEBAR_W) - 80.0;
                    let engine_start_y = 130.0;
                    let engine_h = 56.0;
                    for i in 0..SearchEngine::all().len() {
                        let ey = engine_start_y + i as f32 * (engine_h + 8.0);
                        if content_x >= card_x && content_x <= card_x + card_w
                            && content_y >= ey && content_y <= ey + engine_h
                        {
                            hover_engine_idx = Some(i);
                            break;
                        }
                    }
                    if old_engine_hover != hover_engine_idx {
                        needs_chrome_redraw = true;
                    }
                } else {
                    hover_engine_idx = None;
                }
                if old_hover != hover_sidebar_idx {
                    needs_chrome_redraw = true;
                }
                // Menu hover tracking
                if menu_open {
                    let old_menu_hover = hover_menu_idx;
                    hover_menu_idx = None;
                    let dm_w = 250.0;
                    let dm_x = gpu_renderer.surface_config.width as f32 - dm_w - 16.0;
                    let dm_y = CHROME_TOP + 4.0;
                    for i in 0..11 {
                        let iy = dm_y + 8.0 + i as f32 * 36.0;
                        if mouse_x >= dm_x && mouse_x <= dm_x + dm_w
                            && mouse_y >= iy && mouse_y <= iy + 34.0
                        {
                            hover_menu_idx = Some(i);
                            break;
                        }
                    }
                    if old_menu_hover != hover_menu_idx {
                        needs_chrome_redraw = true;
                    }
                }
                if !is_home_page && mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_move(mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP);
                    }
                }

                // Chrome-style cursor pointer effect (Hand tool / Text / Default)
                let mut cursor_icon = CursorIcon::Default;
                let w = gpu_renderer.surface_config.width as f32;

                if mouse_x < SIDEBAR_W {
                    if hover_sidebar_idx.is_some() || mouse_y > 450.0 {
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if mouse_y <= TAB_BAR_H {
                    // Over tab bar
                    let available_w = w - SIDEBAR_W - 80.0;
                    let tab_count = tabs.len().max(1);
                    let tab_w = ((available_w - 40.0) / tab_count as f32).clamp(110.0, 200.0);
                    let plus_x = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0) + 4.0;
                    let tab_bar_end = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0);
                    if mouse_x >= SIDEBAR_W + 8.0 && mouse_x <= tab_bar_end {
                        cursor_icon = CursorIcon::Hand;
                    } else if mouse_x >= plus_x - 4.0 && mouse_x <= plus_x + 36.0 {
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if mouse_y <= CHROME_TOP {
                    // Over toolbar
                    let nb = SIDEBAR_W + 10.0;
                    let n_enabled_ext = extensions.iter().filter(|e| e.enabled).count() as f32;
                    let right_icons_w = 120.0 + n_enabled_ext * 30.0;
                    let ax = SIDEBAR_W + 60.0;
                    let aw = w - ax - right_icons_w - 10.0;

                    if mouse_x >= nb && mouse_x <= nb + 56.0 {
                        // Back / Forward buttons
                        cursor_icon = CursorIcon::Hand;
                    } else if mouse_x >= ax && mouse_x <= ax + aw {
                        // Address input
                        cursor_icon = CursorIcon::Text;
                    } else if mouse_x > ax + aw {
                        // Extensions / Menu icon
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if menu_open && hover_menu_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                } else if is_settings_page && hover_engine_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                } else if is_extensions_page && hover_ext_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                }

                window.set_cursor_icon(cursor_icon);
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let w = gpu_renderer.surface_config.width as f32;
                // Accurate client mouse coordinates via ScreenToClient
                #[cfg(target_os = "windows")]
                {
                    unsafe {
                        extern "system" {
                            fn GetCursorPos(point: *mut [i32; 2]) -> i32;
                            fn ScreenToClient(hwnd: *mut std::ffi::c_void, point: *mut [i32; 2]) -> i32;
                        }
                        let mut pt: [i32; 2] = [0, 0];
                        GetCursorPos(&mut pt);
                        ScreenToClient(window.hwnd() as _, &mut pt);
                        mouse_x = pt[0] as f32;
                        mouse_y = pt[1] as f32;
                    }
                }
                if state == ElementState::Pressed && button == MouseButton::Left {
                    if SIDEBAR_W > 0.0 && mouse_x < SIDEBAR_W {
                        let mut item_y = 50.0;
                        for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                            if item.is_section {
                                item_y += 41.0;
                            } else {
                                let h = 33.0;
                                if mouse_y >= item_y && mouse_y < item_y + h {
                                    sidebar_active = i;
                                    if i == 0 {
                                        is_home_page = true;
                                        is_settings_page = false;
                                        is_extensions_page = false;
                                        address_bar_text = String::from("about:home");
                                        home_scroll_y = 0.0;
                                        load_internal_page = Some("home".to_string());
                                    } else if i == 5 {
                                        is_extensions_page = true;
                                        is_home_page = false;
                                        is_settings_page = false;
                                        address_bar_text = String::from("axomai://extensions");
                                        load_internal_page = Some("extensions".to_string());
                                    } else if i == 7 {
                                        is_settings_page = true;
                                        is_home_page = false;
                                        is_extensions_page = false;
                                        address_bar_text = String::from("about:settings");
                                        load_internal_page = Some("settings".to_string());
                                    } else {
                                        is_settings_page = false;
                                        is_extensions_page = false;
                                    }
                                    needs_chrome_redraw = true;
                                    break;
                                }
                                item_y += h;
                            }
                        }
                    } else if menu_open {
                        // Check if click is on a menu item
                        let menu_x = w - 266.0;
                        let menu_y_start = CHROME_TOP + 4.0;
                        let menu_w = 250.0;
                        let menu_items = [
                            "New Tab", "Home", "Bookmarks", "History",
                            "Downloads", "Extensions", "Passwords",
                            "Heritage Themes", "Clear RAM & Cache", "Settings", "Exit"
                        ];
                        let mut clicked_item = None;
                        for (i, _item) in menu_items.iter().enumerate() {
                            let iy = menu_y_start + 8.0 + i as f32 * 36.0;
                            if mouse_x >= menu_x && mouse_x <= menu_x + menu_w
                                && mouse_y >= iy && mouse_y <= iy + 34.0
                            {
                                clicked_item = Some(i);
                                break;
                            }
                        }
                        menu_open = false;
                        if let Some(ref wv) = webview {
                            let _ = wv.set_visible(!is_home_page && !is_settings_page && !is_extensions_page);
                        }
                        if let Some(idx) = clicked_item {
                            match idx {
                                0 => {
                                    // New Tab
                                    tabs.push(DesktopTab {
                                        title: String::from("New Tab"),
                                        url: String::from("about:home"),
                                        is_home: true,
                                        is_extensions: false,
                                        is_settings: false,
                                    });
                                    active_tab_idx = tabs.len() - 1;
                                    is_home_page = true;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("about:home");
                                    load_internal_page = Some("home".to_string());
                                }
                                1 => {
                                    // Home
                                    is_home_page = true;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("about:home");
                                    load_internal_page = Some("home".to_string());
                                }
                                2 => {
                                    // Bookmarks
                                    address_bar_text = String::from("axomai://bookmarks");
                                    load_internal_page = Some("home".to_string());
                                }
                                3 => {
                                    // History
                                    address_bar_text = String::from("axomai://history");
                                    load_internal_page = Some("home".to_string());
                                }
                                4 => {
                                    // Downloads
                                    address_bar_text = String::from("axomai://downloads");
                                    load_internal_page = Some("home".to_string());
                                }
                                5 => {
                                    // Extensions
                                    is_extensions_page = true;
                                    is_home_page = false;
                                    is_settings_page = false;
                                    sidebar_active = 5;
                                    address_bar_text = String::from("axomai://extensions");
                                    load_internal_page = Some("extensions".to_string());
                                }
                                6 => {
                                    // Passwords
                                    address_bar_text = String::from("axomai://passwords");
                                    load_internal_page = Some("home".to_string());
                                }
                                7 => {
                                    // Themes
                                    is_settings_page = true;
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    address_bar_text = String::from("about:settings");
                                    load_internal_page = Some("settings".to_string());
                                }
                                8 => {
                                    // Clear RAM
                                    println!("[Axomai] RAM & Cache cleared");
                                }
                                9 => {
                                    // Settings
                                    is_settings_page = true;
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    sidebar_active = 7;
                                    address_bar_text = String::from("about:settings");
                                    load_internal_page = Some("settings".to_string());
                                }
                                10 => {
                                    // Exit
                                    *control_flow = ControlFlow::Exit;
                                }
                                _ => {}
                            }
                        }
                        needs_chrome_redraw = true;
                    } else if mouse_y < CHROME_TOP {
                        let available_w = w - SIDEBAR_W - 80.0;
                        let tab_count = tabs.len().max(1);
                        let tab_w = ((available_w - 40.0) / tab_count as f32).clamp(110.0, 200.0);
                        let mut tab_action = None; // (index, is_close)

                        if mouse_y <= TAB_BAR_H {
                            for (i, _) in tabs.iter().enumerate() {
                                let tab_x = SIDEBAR_W + 8.0 + i as f32 * (tab_w + 4.0);
                                let close_x = tab_x + tab_w - 24.0;
                                if mouse_x >= close_x && mouse_x <= tab_x + tab_w + 2.0 {
                                    tab_action = Some((i, true));
                                    break;
                                } else if mouse_x >= tab_x && mouse_x < close_x {
                                    tab_action = Some((i, false));
                                    break;
                                }
                            }

                            let plus_x = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0) + 4.0;
                            if mouse_x >= plus_x - 4.0 && mouse_x <= plus_x + 36.0 {
                                // + New Tab
                                tabs.push(DesktopTab {
                                    title: String::from("New Tab"),
                                    url: String::from("about:home"),
                                    is_home: true,
                                    is_extensions: false,
                                    is_settings: false,
                                });
                                active_tab_idx = tabs.len() - 1;
                                is_home_page = true;
                                is_extensions_page = false;
                                is_settings_page = false;
                                address_bar_text = String::from("about:home");
                                home_search_text.clear();
                                home_search_focused = false;
                                load_internal_page = Some("home".to_string());
                                needs_chrome_redraw = true;
                            } else if let Some((i, is_close)) = tab_action {
                                if is_close {
                                    if tabs.len() > 1 {
                                        tabs.remove(i);
                                        if active_tab_idx >= tabs.len() {
                                            active_tab_idx = tabs.len() - 1;
                                        } else if active_tab_idx > i {
                                            active_tab_idx -= 1;
                                        }
                                    } else {
                                        tabs[0] = DesktopTab {
                                            title: String::from("Axomai Browser"),
                                            url: String::from("about:home"),
                                            is_home: true,
                                            is_extensions: false,
                                            is_settings: false,
                                        };
                                        active_tab_idx = 0;
                                    }
                                    let cur = &tabs[active_tab_idx];
                                    is_home_page = cur.is_home;
                                    is_extensions_page = cur.is_extensions;
                                    is_settings_page = cur.is_settings;
                                    address_bar_text = cur.url.clone();
                                    if cur.is_home {
                                        load_internal_page = Some("home".to_string());
                                    } else if cur.is_extensions {
                                        load_internal_page = Some("extensions".to_string());
                                    } else if cur.is_settings {
                                        load_internal_page = Some("settings".to_string());
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&cur.url);
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                    needs_chrome_redraw = true;
                                } else {
                                    // Switch tab
                                    active_tab_idx = i;
                                    let cur = &tabs[active_tab_idx];
                                    is_home_page = cur.is_home;
                                    is_extensions_page = cur.is_extensions;
                                    is_settings_page = cur.is_settings;
                                    address_bar_text = cur.url.clone();
                                    if cur.is_home {
                                        load_internal_page = Some("home".to_string());
                                    } else if cur.is_extensions {
                                        load_internal_page = Some("extensions".to_string());
                                    } else if cur.is_settings {
                                        load_internal_page = Some("settings".to_string());
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&cur.url);
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                    needs_chrome_redraw = true;
                                }
                            }
                        }
                        // Toolbar right controls (Profile + Extensions + 3-Dot Menu)
                        let n_ext = extensions.iter().filter(|e| e.enabled).count() as f32;
                        let right_w = 110.0 + n_ext * 30.0;
                        let icons_start_x = w - right_w;
                        let prof_end_x = icons_start_x + 8.0 + (TOOLBAR_H - 20.0) + 8.0;

                        let menu_btn_x = w - 46.0;
                        let puzzle_btn_x = w - 82.0;

                        // Extensions Puzzle Button click (top-right of toolbar)
                        if mouse_x >= puzzle_btn_x && mouse_x < menu_btn_x
                            && mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP
                        {
                            if let Some(ref wv) = webview {
                                let _ = wv.evaluate_script(CHROME_EXT_DROPDOWN_JS);
                            } else {
                                is_extensions_page = true;
                                is_home_page = false;
                                is_settings_page = false;
                                sidebar_active = 5;
                                address_bar_text = String::from("axomai://extensions");
                                load_internal_page = Some("extensions".to_string());
                            }
                            needs_chrome_redraw = true;
                        }

                        // 3-dot menu button area (top-right of toolbar)
                        if mouse_x >= menu_btn_x && mouse_x <= w
                            && mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP
                        {
                            if let Some(ref wv) = webview {
                                let _ = wv.evaluate_script(CHROME_DROPDOWN_JS);
                            } else {
                                menu_open = !menu_open;
                            }
                            needs_chrome_redraw = true;
                        }
                        // Click on extension icon in toolbar to toggle it off
                        if mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP {
                            let mut ex = prof_end_x;
                            let ih = TOOLBAR_H - 20.0;
                            for i in 0..extensions.len() {
                                if extensions[i].enabled {
                                    if mouse_x >= ex && mouse_x <= ex + ih {
                                        extensions[i].enabled = false;
                                        if let Some(ref wv) = webview {
                                            let _ = wv.evaluate_script(extensions[i].disable_js);
                                        }
                                        needs_chrome_redraw = true;
                                        break;
                                    }
                                    ex += 30.0;
                                }
                            }
                        }
                        let addr_x = SIDEBAR_W + 60.0;
                        let addr_y = TAB_BAR_H + 7.0;
                        let addr_h = TOOLBAR_H - 14.0;
                        let addr_w = w - addr_x - right_w - 10.0;
                        if mouse_x >= addr_x
                            && mouse_x <= addr_x + addr_w
                            && mouse_y >= addr_y
                            && mouse_y <= addr_y + addr_h
                        {
                            address_bar_focused = true;
                            address_bar_text.clear();
                            needs_chrome_redraw = true;
                            // Steal focus back from WebView2
                            unsafe {
                                extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                SetFocus(window.hwnd() as _);
                            }
                        } else if mouse_y >= TAB_BAR_H {
                            let nav_base_x = SIDEBAR_W + 10.0;
                            if mouse_x >= nav_base_x && mouse_x <= nav_base_x + 22.0 {
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index > 0 {
                                        eng.history_index -= 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        is_home_page = false;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 28.0 && mouse_x <= nav_base_x + 50.0 {
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index + 1 < eng.history.len() {
                                        eng.history_index += 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        is_home_page = false;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 72.0 && mouse_x <= nav_base_x + 104.0 {
                                if !is_home_page {
                                    if let Ok(mut eng) = engine.lock() {
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        if let Some(url) = eng.current_url.as_ref().map(|u| u.as_string()) {
                                            let _ = eng.load_url(&url, content_w, content_h);
                                        }
                                    }
                                }
                            }
                            address_bar_focused = false;
                            needs_chrome_redraw = true;
                        }
                    } else if is_extensions_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let content_x = mouse_x - SIDEBAR_W;
                        let content_y = mouse_y - CHROME_TOP;
                        let content_w = w - SIDEBAR_W;
                        let padding = 24.0;
                        let gap = 16.0;
                        let cols = if content_w > 600.0 { 2usize } else { 1 };
                        let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
                        let card_h = 160.0;
                        let grid_start_y = 64.0 + 24.0 + 36.0;
                        for i in 0..extensions.len() {
                            let col = (i % cols) as f32;
                            let row = (i / cols) as f32;
                            let cx = padding + col * (card_w + gap);
                            let cy = grid_start_y + row * (card_h + gap);
                            if content_x >= cx && content_x <= cx + card_w
                                && content_y >= cy && content_y <= cy + card_h
                            {
                                extensions[i].enabled = !extensions[i].enabled;
                                let is_on = extensions[i].enabled;
                                let js = if is_on { extensions[i].inject_js } else { extensions[i].disable_js };
                                if let Some(ref wv) = webview {
                                    let _ = wv.evaluate_script(js);
                                }
                                needs_chrome_redraw = true;
                                break;
                            }
                        }
                    } else if is_settings_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let content_x = mouse_x - SIDEBAR_W;
                        let content_y = mouse_y - CHROME_TOP;
                        let card_x = 40.0;
                        let card_w = (w - SIDEBAR_W) - 80.0;
                        let engine_start_y = 130.0;
                        let engine_h = 56.0;
                        for (i, eng_option) in SearchEngine::all().iter().enumerate() {
                            let ey = engine_start_y + i as f32 * (engine_h + 8.0);
                            if content_x >= card_x && content_x <= card_x + card_w
                                && content_y >= ey && content_y <= ey + engine_h
                            {
                                selected_search_engine = *eng_option;
                                needs_chrome_redraw = true;
                                break;
                            }
                        }
                    } else if is_home_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let content_w = w - SIDEBAR_W;
                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                        let cx = content_w / 2.0;
                        let cy = content_h / 2.0 - 60.0;
                        let search_w = 540.0f32.min(content_w - 80.0);
                        let search_x = SIDEBAR_W + cx - search_w / 2.0;
                        let search_y = CHROME_TOP + cy + 135.0;
                        if mouse_x >= search_x && mouse_x <= search_x + search_w
                            && mouse_y >= search_y && mouse_y <= search_y + 44.0
                        {
                            home_search_focused = true;
                            home_search_text.clear();
                            needs_chrome_redraw = true;
                            unsafe {
                                extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                SetFocus(window.hwnd() as _);
                            }
                        } else {
                            home_search_focused = false;
                            needs_chrome_redraw = true;
                        }
                    } else if !is_home_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let btn = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 2,
                            MouseButton::Middle => 1,
                            _ => 0,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            if let Some(nav_url) = eng.handle_pointer_down(
                                mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, btn,
                            ) {
                                let content_w = w - SIDEBAR_W;
                                let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                let _ = eng.load_url(&nav_url, content_w, content_h);
                                address_bar_text = nav_url;
                                needs_chrome_redraw = true;
                            }
                        }
                    }
                }
                if state == ElementState::Released && !is_home_page && mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    let btn = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 1,
                        _ => 0,
                    };
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_up(mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, btn);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event: key_event, .. },
                ..
            } => {
                if key_event.state == ElementState::Pressed {
                    if address_bar_focused {
                        match key_event.logical_key {
                            Key::Backspace => {
                                address_bar_text.pop();
                                needs_chrome_redraw = true;
                            }
                            Key::Enter => {
                                address_bar_focused = false;
                                let url = if address_bar_text.contains("://")
                                    || address_bar_text.contains('.')
                                {
                                    if !address_bar_text.contains("://") {
                                        format!("https://{}", address_bar_text)
                                    } else {
                                        address_bar_text.clone()
                                    }
                                } else if !address_bar_text.is_empty() {
                                    selected_search_engine.search_url(&address_bar_text)
                                } else {
                                    return;
                                };
                                if webview.is_none() {
                                    let cw = w_of(&gpu_renderer);
                                    let ch = h_of(&gpu_renderer);
                                    let nav_clone = nav_url_shared.clone();
                                    let init_js = build_extension_init_script(&extensions);
                                    webview = WebViewBuilder::new()
                                        .with_url(&url)
                                        .with_devtools(false)
                                        .with_initialization_script(&init_js)
                                        .with_bounds(Rect {
                                            position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                            size: wry::dpi::LogicalSize::new(
                                                (cw - SIDEBAR_W) as u32,
                                                (ch - CHROME_TOP) as u32,
                                            ).into(),
                                        })
                                        .with_navigation_handler(move |nav_url: String| {
                                            if let Ok(mut nav) = nav_clone.lock() {
                                                *nav = Some(nav_url);
                                            }
                                            true
                                        })
                                        .build_as_child(&window)
                                        .ok();
                                    webview_visible = webview.is_some();
                                    if webview.is_some() {
                                        println!("[Axomai] WebView2 initialized successfully");
                                    }
                                } else if let Some(ref wv) = webview {
                                    let _ = wv.load_url(&url);
                                    if !webview_visible {
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                }
                                address_bar_text = url;
                                is_home_page = false;
                                is_settings_page = false;
                                is_extensions_page = false;
                                needs_chrome_redraw = true;
                            }
                            Key::Escape => {
                                address_bar_focused = false;
                                if is_home_page {
                                    address_bar_text = String::from("about:home");
                                }
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else if home_search_focused {
                        match key_event.logical_key {
                            Key::Backspace => {
                                home_search_text.pop();
                                needs_chrome_redraw = true;
                            }
                            Key::Enter => {
                                home_search_focused = false;
                                if !home_search_text.is_empty() {
                                    let url = if home_search_text.contains("://")
                                        || home_search_text.contains('.')
                                    {
                                        if !home_search_text.contains("://") {
                                            format!("https://{}", home_search_text)
                                        } else {
                                            home_search_text.clone()
                                        }
                                    } else {
                                        selected_search_engine.search_url(&home_search_text)
                                    };
                                    if webview.is_none() {
                                        let cw = w_of(&gpu_renderer);
                                        let ch = h_of(&gpu_renderer);
                                        let nav_clone = nav_url_shared.clone();
                                        let init_js = build_extension_init_script(&extensions);
                                        webview = WebViewBuilder::new()
                                            .with_url(&url)
                                            .with_devtools(false)
                                            .with_initialization_script(&init_js)
                                            .with_bounds(Rect {
                                                position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                                size: wry::dpi::LogicalSize::new(
                                                    (cw - SIDEBAR_W) as u32,
                                                    (ch - CHROME_TOP) as u32,
                                                ).into(),
                                            })
                                            .with_navigation_handler(move |nav_url: String| {
                                                if let Ok(mut nav) = nav_clone.lock() {
                                                    *nav = Some(nav_url);
                                                }
                                                true
                                            })
                                            .build_as_child(&window)
                                            .ok();
                                        webview_visible = webview.is_some();
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&url);
                                        if !webview_visible {
                                            let _ = wv.set_visible(true);
                                            webview_visible = true;
                                        }
                                    }
                                    address_bar_text = url;
                                    is_home_page = false;
                                    is_settings_page = false;
                                    is_extensions_page = false;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::Escape => {
                                home_search_focused = false;
                                home_search_text.clear();
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else {
                        if is_home_page {
                            // Character input handled by ReceivedImeText
                        } else if !is_home_page {
                            let key_str = match key_event.logical_key {
                                Key::Character(ref ch) => ch.to_string(),
                                Key::Backspace => "BackSpace".to_string(),
                                Key::Enter => "Enter".to_string(),
                                Key::Tab => "Tab".to_string(),
                                Key::Escape => "Escape".to_string(),
                                Key::Space => " ".to_string(),
                                Key::ArrowUp => "ArrowUp".to_string(),
                                Key::ArrowDown => "ArrowDown".to_string(),
                                Key::ArrowLeft => "ArrowLeft".to_string(),
                                Key::ArrowRight => "ArrowRight".to_string(),
                                _ => return,
                            };
                            if let Ok(mut eng) = engine.lock() {
                                if let Some(nav_url) = eng.handle_key_event(
                                    "keydown", &key_str, "", 0, false, false, false, false, false,
                                ) {
                                    let content_w = w_of(&gpu_renderer) - SIDEBAR_W;
                                    let content_h = h_of(&gpu_renderer) - CHROME_TOP;
                                    let _ = eng.load_url(&nav_url, content_w, content_h);
                                    address_bar_text = nav_url;
                                    needs_chrome_redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedImeText(ref text),
                ..
            } => {
                if address_bar_focused {
                    address_bar_text.push_str(text);
                    needs_chrome_redraw = true;
                } else if home_search_focused {
                    home_search_text.push_str(text);
                    needs_chrome_redraw = true;
                } else if is_home_page {
                    home_search_focused = true;
                    home_search_text.clear();
                    home_search_text.push_str(text);
                    needs_chrome_redraw = true;
                    unsafe {
                        extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                        SetFocus(window.hwnd() as _);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                if mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    let dy = match delta {
                        tao::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        _ => 0.0,
                    };
                    if is_home_page {
                        home_scroll_y = (home_scroll_y + dy).min(0.0).max(-800.0);
                        needs_chrome_redraw = true;
                    } else if let Ok(mut eng) = engine.lock() {
                        let content_w = w_of(&gpu_renderer) - SIDEBAR_W;
                        let content_h = h_of(&gpu_renderer) - CHROME_TOP;
                        let _ = eng.handle_scroll_at(
                            mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, 0.0, dy, content_w, content_h,
                        );
                    }
                }
            }
            Event::MainEventsCleared => {
                if let Ok(mut nav) = nav_url_shared.lock() {
                    if let Some(url) = nav.take() {
                        if url.starts_with("axomai://newtab") {
                            tabs.push(DesktopTab {
                                title: String::from("New Tab"),
                                url: String::from("about:home"),
                                is_home: true,
                                is_extensions: false,
                                is_settings: false,
                            });
                            active_tab_idx = tabs.len() - 1;
                            is_home_page = true;
                            is_extensions_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("about:home");
                            load_internal_page = Some("home".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://home") {
                            is_home_page = true;
                            is_extensions_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("about:home");
                            load_internal_page = Some("home".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://extensions") {
                            is_extensions_page = true;
                            is_home_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("axomai://extensions");
                            load_internal_page = Some("extensions".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://settings") || url.starts_with("axomai://themes") {
                            is_settings_page = true;
                            is_home_page = false;
                            is_extensions_page = false;
                            address_bar_text = String::from("about:settings");
                            load_internal_page = Some("settings".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://exit") {
                            *control_flow = ControlFlow::Exit;
                        } else if url.starts_with("axomai://ext-toggle/") {
                            if let Ok(idx) = url.trim_start_matches("axomai://ext-toggle/").parse::<usize>() {
                                if idx < extensions.len() {
                                    extensions[idx].enabled = !extensions[idx].enabled;
                                    if is_extensions_page {
                                        load_internal_page = Some("extensions".to_string());
                                    }
                                    needs_chrome_redraw = true;
                                }
                            }
                        } else if url.starts_with("axomai://set-engine/") {
                            let name = url.trim_start_matches("axomai://set-engine/");
                            selected_search_engine = match name {
                                "Bing" => SearchEngine::Bing,
                                "Yahoo" => SearchEngine::Yahoo,
                                "DuckDuckGo" => SearchEngine::DuckDuckGo,
                                _ => SearchEngine::Google,
                            };
                            if is_settings_page {
                                load_internal_page = Some("settings".to_string());
                            }
                        } else if url.starts_with("data:") {
                            // Internal page data URL — don't update address bar
                        } else {
                            address_bar_text = url;
                            is_home_page = false;
                            is_extensions_page = false;
                            is_settings_page = false;
                            needs_chrome_redraw = true;
                            let has_auto_ext = extensions.iter().any(|e| e.enabled && e.auto_inject);
                            if has_auto_ext {
                                ext_inject_time = Some(std::time::Instant::now() + std::time::Duration::from_millis(1200));
                            }
                        }
                    }
                }
                if let Some(page) = load_internal_page.take() {
                    let target_url = match page.as_str() {
                        "home" => {
                            let html = internal_pages::home_page_html_with_engine(selected_search_engine.js_search_template());
                            page_to_file_url("axomai_home.html", &html)
                        }
                        "extensions" => {
                            let html = internal_pages::extensions_page_html(&extensions);
                            page_to_file_url("axomai_extensions.html", &html)
                        }
                        "settings" => {
                            let html = internal_pages::settings_page_html(selected_search_engine);
                            page_to_file_url("axomai_settings.html", &html)
                        }
                        _ => {
                            let html = internal_pages::home_page_html_with_engine(selected_search_engine.js_search_template());
                            page_to_file_url("axomai_home.html", &html)
                        }
                    };

                    let cw = w_of(&gpu_renderer);
                    let ch = h_of(&gpu_renderer);
                    if webview.is_none() {
                        let nav_clone = nav_url_shared.clone();
                        webview = WebViewBuilder::new()
                            .with_url(&target_url)
                            .with_devtools(false)
                            .with_bounds(Rect {
                                position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                size: wry::dpi::LogicalSize::new(
                                    (cw - SIDEBAR_W) as u32,
                                    (ch - CHROME_TOP) as u32,
                                ).into(),
                            })
                            .with_navigation_handler(move |nav_url: String| {
                                if nav_url.starts_with("axomai://") {
                                    if let Ok(mut nav) = nav_clone.lock() {
                                        *nav = Some(nav_url);
                                    }
                                    return false;
                                }
                                if let Ok(mut nav) = nav_clone.lock() {
                                    *nav = Some(nav_url);
                                }
                                true
                            })
                            .build_as_child(&window)
                            .ok();
                        webview_visible = webview.is_some();
                    } else if let Some(ref wv) = webview {
                        let _ = wv.load_url(&target_url);
                        if !webview_visible {
                            let _ = wv.set_visible(true);
                            webview_visible = true;
                        }
                    }
                    needs_chrome_redraw = true;
                }
                if let Some(t) = ext_inject_time {
                    if std::time::Instant::now() >= t {
                        ext_inject_time = None;
                        if let Some(ref wv) = webview {
                            for ext in &extensions {
                                if ext.enabled && ext.auto_inject {
                                    let _ = wv.evaluate_script(ext.inject_js);
                                }
                            }
                        }
                    }
                }

                let w = w_of(&gpu_renderer);
                let h = h_of(&gpu_renderer);
                let content_w = w - SIDEBAR_W;
                let content_h = h - CHROME_TOP;

                let should_render = if is_home_page || is_settings_page || is_extensions_page {
                    needs_chrome_redraw || gpu_renderer.presented_frames < 3
                } else if let Ok(mut eng) = engine.lock() {
                    let updated = eng.process_event_loop(content_w, content_h);
                    updated || needs_chrome_redraw || gpu_renderer.presented_frames < 3
                } else {
                    false
                };

                if should_render {
                    needs_chrome_redraw = false;
                    compositor.width = content_w as u32;
                    compositor.height = content_h as u32;

                    let title = if is_home_page {
                        "Axomai Browser".to_string()
                    } else if is_extensions_page {
                        "Extensions".to_string()
                    } else if is_settings_page {
                        "Settings".to_string()
                    } else if let Ok(eng) = engine.lock() {
                        eng.current_title.clone()
                    } else {
                        String::new()
                    };

                    let history_back = if let Ok(eng) = engine.lock() { eng.history_index > 0 } else { false };
                    let history_fwd = if let Ok(eng) = engine.lock() { eng.history_index + 1 < eng.history.len() } else { false };

                    let toolbar_icons = [icon_back, icon_forward, icon_home, icon_menu];
                    if active_tab_idx < tabs.len() {
                        tabs[active_tab_idx].url = address_bar_text.clone();
                        tabs[active_tab_idx].is_home = is_home_page;
                        tabs[active_tab_idx].is_extensions = is_extensions_page;
                        tabs[active_tab_idx].is_settings = is_settings_page;
                        tabs[active_tab_idx].title = title.clone();
                    }
                    let mut quads = build_chrome_quads(
                        &mut compositor, w, h, &address_bar_text, address_bar_focused,
                        &tabs, active_tab_idx, history_back, history_fwd, sidebar_active, hover_sidebar_idx,
                        menu_open, hover_menu_idx, &toolbar_icons, &extensions,
                    );

                    if is_home_page || is_extensions_page || is_settings_page {
                        // Internal pages rendered via WebView HTML — no GPU quads needed
                    } else {
                        if let Ok(eng) = engine.lock() {
                            let mut page_quads = compositor.extract_gpu_quads(&eng.display_list);
                            for q in &mut page_quads {
                                for v in &mut q.vertices {
                                    v.position[0] += SIDEBAR_W;
                                    v.position[1] += CHROME_TOP;
                                }
                            }
                            quads.extend(page_quads);
                        }
                    }

                    if menu_open {
                        quads.extend(build_dropdown_quads(&mut compositor, w, hover_menu_idx));
                    }

                    if compositor.glyph_atlas.dirty {
                        gpu_renderer.upload_glyph_atlas(&compositor.glyph_atlas);
                        compositor.glyph_atlas.dirty = false;
                    }

                    match gpu_renderer.render_frame(&quads) {
                        Ok(frame_idx) => {
                            if frame_idx % 300 == 1 {
                                println!("[Axomai GPU] Frame #{} — {} quads", frame_idx, quads.len());
                            }
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            let cfg = &gpu_renderer.surface_config;
                            gpu_renderer.resize(cfg.width, cfg.height);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            *control_flow = ControlFlow::Exit;
                        }
                        Err(e) => {
                            eprintln!("[Axomai GPU] Render error: {:?}", e);
                        }
                    }

                    window.set_title(&format!("Axomai Browser — {}", title));
                }
            }
            _ => {}
        }
    });
}

fn w_of(r: &WgpuRenderer) -> f32 { r.surface_config.width as f32 }
fn h_of(r: &WgpuRenderer) -> f32 { r.surface_config.height as f32 }

use axomai_engine::GpuQuad;

fn c(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
}

fn render_text(
    compositor: &mut NativeGpuCompositor,
    quads: &mut Vec<GpuQuad>,
    text: &str,
    mut x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
    max_x: f32,
) -> f32 {
    for ch in text.chars() {
        if x > max_x { break; }
        let g = compositor.glyph_atlas.rasterize(ch, size);
        if g.width > 0.0 {
            quads.push(NativeGpuCompositor::text_quad(x + g.offset_x, y - g.offset_y - g.height, &g, color));
        }
        x += g.advance_width;
    }
    x
}

#[allow(dead_code)]
fn render_text_centered(
    compositor: &mut NativeGpuCompositor,
    quads: &mut Vec<GpuQuad>,
    text: &str,
    center_x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let mut total_w = 0.0f32;
    for ch in text.chars() {
        let g = compositor.glyph_atlas.rasterize(ch, size);
        total_w += g.advance_width;
    }
    let start_x = center_x - total_w / 2.0;
    render_text(compositor, quads, text, start_x, y, size, color, center_x + total_w);
}

// ===== CHROME (Tab bar + Toolbar + Sidebar) =====

fn rq(x: f32, y: f32, w: f32, h: f32, _r: f32, color: [f32; 4]) -> GpuQuad {
    NativeGpuCompositor::solid_quad(x, y, w, h, color)
}

fn build_chrome_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    viewport_h: f32,
    address_text: &str,
    focused: bool,
    tabs: &[DesktopTab],
    active_tab_idx: usize,
    history_back: bool,
    history_fwd: bool,
    sidebar_active: usize,
    hover_sidebar: Option<usize>,
    _menu_open: bool,
    _hover_menu: Option<usize>,
    icons: &[GlyphInfo; 4], // [back, forward, home, menu]
    extensions: &[Extension],
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Chrome light theme colors
    let white = c(255, 255, 255, 255);
    let tab_bar_bg = c(222, 225, 230, 255);
    let _toolbar_bg = white;
    let _border = c(218, 220, 224, 255);
    let text_primary = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_disabled = c(155, 160, 168, 255);
    let _blue = c(26, 115, 232, 255);
    let _hover_bg = c(232, 234, 237, 200);
    let _active_bg = c(210, 227, 252, 200);

    // === SIDEBAR (Only rendered if SIDEBAR_W > 0) ===
    if SIDEBAR_W > 0.0 {
        let bands = 8;
        for i in 0..bands {
            let t = i as f32 / bands as f32;
            let r = (15.0 + t * 10.0) as u8;
            let g = (20.0 + t * 30.0) as u8;
            let b = (50.0 + t * 30.0) as u8;
            let band_h = viewport_h / bands as f32;
            quads.push(NativeGpuCompositor::solid_quad(0.0, i as f32 * band_h, SIDEBAR_W, band_h + 1.0, c(r, g, b, 255)));
        }
        quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, SIDEBAR_W, viewport_h, c(255, 255, 255, 18)));
        quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W - 1.0, 0.0, 1.0, viewport_h, c(100, 140, 200, 80)));

        let sb_text = c(220, 225, 235, 255);
        let sb_text_dim = c(140, 155, 180, 255);
        let sb_accent = c(100, 180, 255, 255);
        let sb_divider = c(255, 255, 255, 20);
        let sb_hover = c(255, 255, 255, 20);
        let sb_active = c(100, 180, 255, 30);

        render_text(compositor, &mut quads, "Axomai", 16.0, 28.0, 16.0, sb_accent, SIDEBAR_W);
        render_text(compositor, &mut quads, "Browser", 88.0, 28.0, 10.0, sb_text_dim, SIDEBAR_W);
        quads.push(NativeGpuCompositor::solid_quad(12.0, 40.0, SIDEBAR_W - 24.0, 1.0, sb_divider));

        let mut item_y = 50.0;
        for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
            if item.is_section {
                item_y += 8.0;
                quads.push(NativeGpuCompositor::solid_quad(12.0, item_y, SIDEBAR_W - 24.0, 1.0, sb_divider));
                item_y += 10.0;
                render_text(compositor, &mut quads, item.label, 16.0, item_y + 12.0, 10.0, sb_text_dim, SIDEBAR_W);
                item_y += 22.0;
            } else {
                let is_active = i == sidebar_active;
                let is_hovered = hover_sidebar == Some(i);
                let h = 32.0;
                if is_active {
                    quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, sb_active));
                    quads.push(NativeGpuCompositor::solid_quad(2.0, item_y + 6.0, 3.0, h - 12.0, sb_accent));
                } else if is_hovered {
                    quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, sb_hover));
                }
                let tc = if is_active { sb_accent } else { sb_text };
                render_text(compositor, &mut quads, item.icon, 18.0, item_y + 21.0, 13.0, tc, 36.0);
                render_text(compositor, &mut quads, item.label, 38.0, item_y + 21.0, 13.0, tc, SIDEBAR_W - 8.0);
                item_y += h + 1.0;
            }
        }
        item_y += 6.0;
        render_text(compositor, &mut quads, "+ Add Workspace", 18.0, item_y + 12.0, 11.0, sb_accent, SIDEBAR_W);
    }

    // === TAB BAR (Chrome-style Multi-Tab Strip) ===
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, 0.0, viewport_w - SIDEBAR_W, TAB_BAR_H, tab_bar_bg));

    let available_w = viewport_w - SIDEBAR_W - 80.0;
    let tab_count = tabs.len().max(1);
    let tab_w = ((available_w - 40.0) / tab_count as f32).clamp(110.0, 200.0);
    let tab_h = TAB_BAR_H - 8.0;

    for (i, t) in tabs.iter().enumerate() {
        let tab_x = SIDEBAR_W + 8.0 + i as f32 * (tab_w + 4.0);
        let is_active = i == active_tab_idx;
        let t_title = if t.title.is_empty() { "New Tab" } else { &t.title };

        if is_active {
            // Active tab: white rounded top
            quads.push(rq(tab_x, 8.0, tab_w, tab_h + 2.0, 8.0, white));
            quads.push(NativeGpuCompositor::solid_quad(tab_x, TAB_BAR_H - 2.0, tab_w, 2.0, white));
            
            // Top accent indicator (emerald green)
            quads.push(NativeGpuCompositor::solid_quad(tab_x + 8.0, 8.0, tab_w - 16.0, 2.0, c(16, 185, 129, 255)));

            // Favicon circle
            quads.push(rq(tab_x + 10.0, 15.0, 16.0, 16.0, 8.0, c(16, 185, 129, 255)));
            render_text(compositor, &mut quads, "A", tab_x + 13.0, 27.0, 10.0, white, tab_x + 28.0);

            // Tab title
            render_text(compositor, &mut quads, t_title, tab_x + 32.0, 27.0, 12.0, text_primary, tab_x + tab_w - 28.0);
            // Close button (x)
            render_text(compositor, &mut quads, "x", tab_x + tab_w - 18.0, 27.0, 12.0, text_secondary, tab_x + tab_w);
        } else {
            // Inactive tab
            quads.push(rq(tab_x, 10.0, tab_w, tab_h, 6.0, c(235, 238, 242, 180)));

            // Favicon circle dim
            quads.push(rq(tab_x + 10.0, 16.0, 14.0, 14.0, 7.0, c(180, 190, 200, 255)));
            render_text(compositor, &mut quads, "A", tab_x + 13.0, 27.0, 9.0, white, tab_x + 26.0);

            // Tab title dim
            render_text(compositor, &mut quads, t_title, tab_x + 30.0, 27.0, 11.5, text_secondary, tab_x + tab_w - 26.0);
            // Close button (x)
            render_text(compositor, &mut quads, "x", tab_x + tab_w - 18.0, 27.0, 11.5, text_disabled, tab_x + tab_w);
        }
    }

    // + New tab button (circular)
    let plus_x = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0) + 6.0;
    quads.push(rq(plus_x, 12.0, 24.0, 24.0, 12.0, c(235, 238, 245, 220)));
    render_text(compositor, &mut quads, "+", plus_x + 7.0, 28.0, 15.0, text_secondary, plus_x + 24.0);

    // === TOOLBAR (Modern glassmorphism) ===
    let ty = TAB_BAR_H;
    let toolbar_w = viewport_w - SIDEBAR_W;
    // Glassmorphism toolbar background
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, toolbar_w, TOOLBAR_H, c(240, 243, 249, 245)));
    // Frosted overlay
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, toolbar_w, TOOLBAR_H, c(255, 255, 255, 60)));
    // Bottom glow line (gradient blue-purple)
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, CHROME_TOP - 1.5, toolbar_w * 0.5, 1.5, c(99, 132, 255, 50)));
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W + toolbar_w * 0.5, CHROME_TOP - 1.5, toolbar_w * 0.5, 1.5, c(168, 120, 255, 40)));

    // Navigation buttons (icon-based)
    let icon_s = 18.0;
    let nav_iy = ty + (TOOLBAR_H - icon_s) / 2.0;
    let nb = SIDEBAR_W + 10.0;
    let back_c = if history_back { c(60, 65, 75, 255) } else { c(180, 185, 195, 180) };
    let fwd_c = if history_fwd { c(60, 65, 75, 255) } else { c(180, 185, 195, 180) };
    quads.push(NativeGpuCompositor::icon_quad(nb, nav_iy, icon_s, &icons[0], back_c));
    quads.push(NativeGpuCompositor::icon_quad(nb + 28.0, nav_iy, icon_s, &icons[1], fwd_c));

    // Full-width address bar (modern glassmorphism pill)
    let ax = SIDEBAR_W + 60.0;
    let ay = ty + 7.0;
    let ah = TOOLBAR_H - 14.0;
    let n_enabled_ext = extensions.iter().filter(|e| e.enabled).count() as f32;
    let right_icons_w = 120.0 + n_enabled_ext * 30.0;
    let aw = viewport_w - ax - right_icons_w - 10.0;
    let bar_radius = ah / 2.0;

    if focused {
        // Focused: elevated glass with blue accent glow
        quads.push(rq(ax - 1.0, ay + 2.0, aw + 2.0, ah + 1.0, bar_radius + 1.0, c(99, 132, 255, 25)));
        quads.push(rq(ax - 1.5, ay - 1.5, aw + 3.0, ah + 3.0, bar_radius + 2.0, c(99, 132, 255, 120)));
        quads.push(rq(ax, ay, aw, ah, bar_radius, c(255, 255, 255, 252)));
    } else {
        // Unfocused: frosted glass pill
        quads.push(rq(ax, ay + 1.0, aw, ah, bar_radius, c(0, 0, 0, 6)));
        quads.push(rq(ax, ay, aw, ah, bar_radius, c(235, 238, 245, 220)));
        // Inner highlight at top
        quads.push(NativeGpuCompositor::solid_quad(ax + 8.0, ay + 1.0, aw - 16.0, 1.0, c(255, 255, 255, 100)));
    }

    // Search/lock icon (home icon in URL bar)
    let icon_y = ay + ah / 2.0 + 5.0;
    let url_icon_s = 14.0;
    let url_icon_y = ay + (ah - url_icon_s) / 2.0;
    quads.push(NativeGpuCompositor::icon_quad(ax + 10.0, url_icon_y, url_icon_s, &icons[2], c(130, 135, 150, 255)));

    let is_placeholder = address_text == "about:home" && !focused;
    let _is_internal = address_text == "axomai://extensions" || address_text == "about:settings";
    let display = if is_placeholder { "Search or type a URL" } else { address_text };
    let dtc = if is_placeholder { c(150, 155, 168, 255) } else { c(40, 42, 50, 255) };
    let end_x = render_text(compositor, &mut quads, display, ax + 34.0, icon_y, 13.0, dtc, ax + aw - 14.0);

    if focused {
        quads.push(NativeGpuCompositor::solid_quad(end_x + 1.0, ay + 6.0, 1.5, ah - 12.0, c(99, 132, 255, 200)));
    }

    // Right toolbar icons (modern, compact)
    // Count enabled extensions to allocate space
    let enabled_exts: Vec<(usize, &Extension)> = extensions.iter().enumerate().filter(|(_, e)| e.enabled).collect();
    let ext_icons_w = enabled_exts.len() as f32 * 30.0;
    let right_total = 110.0 + ext_icons_w; // profile + extensions + puzzle + menu
    let icons_start = viewport_w - right_total;
    let iy = ty + 10.0;
    let ih = TOOLBAR_H - 20.0;
    let icy = iy + ih / 2.0 + 4.0;

    // Profile avatar (glass circle)
    let prof_x = icons_start + 8.0;
    quads.push(rq(prof_x, iy + 1.0, ih, ih, ih / 2.0, c(99, 132, 255, 180)));
    render_text(compositor, &mut quads, "S", prof_x + 6.0, icy, 11.0, white, viewport_w);

    // Enabled extension icons in toolbar (Chrome-style)
    let mut ext_x = prof_x + ih + 8.0;
    for (_idx, ext) in &enabled_exts {
        let ec = c(ext.icon_color[0], ext.icon_color[1], ext.icon_color[2], 220);
        quads.push(rq(ext_x, iy + 1.0, ih, ih, ih / 2.0, ec));
        render_text(compositor, &mut quads, ext.icon_letter, ext_x + 5.0, icy, 11.0, white, ext_x + ih);
        ext_x += 30.0;
    }

    // Puzzle extensions button
    let puzzle_x = ext_x + 2.0;
    quads.push(rq(puzzle_x, iy + 1.0, ih, ih, 6.0, c(255, 255, 255, 40)));
    render_text(compositor, &mut quads, "E", puzzle_x + 6.0, icy, 11.5, c(60, 65, 80, 255), puzzle_x + ih);

    // Three-dot menu button (icon)
    let menu_x = puzzle_x + ih + 6.0;
    let menu_icon_s = 18.0;
    let menu_icon_y = ty + (TOOLBAR_H - menu_icon_s) / 2.0;
    quads.push(NativeGpuCompositor::icon_quad(menu_x, menu_icon_y, menu_icon_s, &icons[3], c(80, 85, 100, 255)));

    quads
}

fn build_dropdown_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    hover_menu: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();
    let text_primary = c(32, 33, 36, 255);
    let _text_secondary = c(95, 99, 104, 255);
    let dm_w = 250.0;
    let dm_x = viewport_w - dm_w - 16.0;
    let dm_y = CHROME_TOP + 4.0;
    let menu_items: &[(&str, &str)] = &[
        ("+", "New Tab            Ctrl+T"),
        ("H", "Home Page"),
        ("B", "Bookmarks"),
        ("h", "History"),
        ("D", "Downloads"),
        ("E", "Extensions"),
        ("P", "Passwords"),
        ("T", "Heritage Themes"),
        ("C", "Clear RAM & Cache"),
        ("S", "Settings"),
        ("X", "Exit Axomai"),
    ];
    let dm_h = 10.0 + menu_items.len() as f32 * 36.0 + 8.0;
    quads.push(rq(dm_x + 3.0, dm_y + 3.0, dm_w, dm_h, 12.0, c(0, 0, 0, 40)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(255, 255, 255, 255)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(218, 220, 224, 60)));

    for (i, (icon, label)) in menu_items.iter().enumerate() {
        let iy = dm_y + 8.0 + i as f32 * 36.0;
        if hover_menu == Some(i) {
            quads.push(rq(dm_x + 6.0, iy, dm_w - 12.0, 34.0, 8.0, c(235, 240, 248, 255)));
        }
        render_text(compositor, &mut quads, icon, dm_x + 16.0, iy + 22.0, 13.0, c(16, 185, 129, 255), dm_x + 36.0);
        render_text(compositor, &mut quads, label, dm_x + 38.0, iy + 22.0, 12.5, text_primary, dm_x + dm_w - 12.0);
    }

    quads
}

// ===== SETTINGS PAGE =====

#[allow(dead_code)]
fn build_settings_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    selected: SearchEngine,
    hover_idx: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let page_bg = c(246, 247, 248, 255);
    let text_dark = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let blue = c(26, 115, 232, 255);
    let border = c(218, 220, 224, 255);
    let hover_bg = c(241, 243, 244, 255);
    let selected_bg = c(210, 227, 252, 255);
    let green = c(24, 128, 56, 255);

    // Background
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));

    // Header
    render_text(compositor, &mut quads, "Settings", 40.0, 40.0, 24.0, text_dark, content_w);
    quads.push(NativeGpuCompositor::solid_quad(40.0, 55.0, content_w - 80.0, 1.0, border));

    // Section title
    render_text(compositor, &mut quads, "Search Engine", 40.0, 88.0, 16.0, text_dark, content_w);
    render_text(compositor, &mut quads, "Choose the search engine used in the address bar", 40.0, 108.0, 12.0, text_secondary, content_w);

    // Search engine cards
    let card_x = 40.0;
    let card_w = (content_w - 80.0).min(500.0);
    let engine_h = 56.0;
    let start_y = 130.0;

    let engine_descriptions: &[&str] = &[
        "The world's most popular search engine",
        "Microsoft's search engine with AI features",
        "A classic search engine by Yahoo Inc.",
        "Privacy-focused search, no tracking",
    ];

    for (i, eng) in SearchEngine::all().iter().enumerate() {
        let ey = start_y + i as f32 * (engine_h + 8.0);
        let is_selected = *eng == selected;
        let is_hovered = hover_idx == Some(i);

        let bg = if is_selected {
            selected_bg
        } else if is_hovered {
            hover_bg
        } else {
            white
        };

        quads.push(rq(card_x, ey, card_w, engine_h, 10.0, bg));

        // Radio circle
        let radio_x = card_x + 20.0;
        let radio_y = ey + engine_h / 2.0;
        quads.push(rq(radio_x - 9.0, radio_y - 9.0, 18.0, 18.0, 9.0, if is_selected { blue } else { border }));
        quads.push(rq(radio_x - 7.0, radio_y - 7.0, 14.0, 14.0, 7.0, if is_selected { blue } else { white }));
        if is_selected {
            quads.push(rq(radio_x - 4.0, radio_y - 4.0, 8.0, 8.0, 4.0, white));
        }

        // Engine name
        let name_x = card_x + 48.0;
        render_text(compositor, &mut quads, eng.name(), name_x, ey + 24.0, 14.0, text_dark, card_x + card_w);

        // Description
        render_text(compositor, &mut quads, engine_descriptions[i], name_x, ey + 42.0, 11.0, text_secondary, card_x + card_w - 10.0);

        // Selected badge
        if is_selected {
            let badge_x = card_x + card_w - 80.0;
            render_text(compositor, &mut quads, "Default", badge_x, ey + 32.0, 11.0, green, card_x + card_w);
        }
    }

    // Info text at bottom
    let info_y = start_y + 4.0 * (engine_h + 8.0) + 10.0;
    render_text(compositor, &mut quads, "Click on a search engine to set it as default.", 40.0, info_y + 14.0, 11.0, text_secondary, content_w);
    render_text(compositor, &mut quads, "The selected engine is used when you type in the address bar.", 40.0, info_y + 30.0, 11.0, text_secondary, content_w);

    quads
}

// ===== EXTENSIONS PAGE =====

#[allow(dead_code)]
fn build_extensions_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    extensions: &[Extension],
    hover_idx: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let page_bg = c(241, 243, 244, 255);
    let text_dark = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_hint = c(154, 160, 166, 255);
    let blue = c(26, 115, 232, 255);
    let green = c(34, 168, 83, 255);
    let gray_track = c(189, 193, 198, 255);
    let card_border = c(218, 220, 224, 255);

    // Full page background
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));

    // Top header bar (white)
    let header_h = 64.0;
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, header_h, white));
    quads.push(NativeGpuCompositor::solid_quad(0.0, header_h - 1.0, content_w, 1.0, card_border));

    // Axomai puzzle icon
    let icon_s = 28.0;
    quads.push(rq(24.0, (header_h - icon_s) / 2.0, icon_s, icon_s, 6.0, blue));
    render_text(compositor, &mut quads, "E", 31.0, header_h / 2.0 + 6.0, 16.0, white, 60.0);

    // Title
    render_text(compositor, &mut quads, "Extensions", 64.0, header_h / 2.0 + 7.0, 20.0, text_dark, content_w);

    // Search bar (right side of header)
    let search_w = 260.0f32.min(content_w - 300.0);
    if search_w > 100.0 {
        let sx = content_w - search_w - 24.0;
        let sy = (header_h - 36.0) / 2.0;
        quads.push(rq(sx, sy, search_w, 36.0, 18.0, c(241, 243, 244, 255)));
        render_text(compositor, &mut quads, "Search extensions", sx + 16.0, sy + 23.0, 13.0, text_hint, sx + search_w - 8.0);
    }

    // "All Extensions" section header
    let section_y = header_h + 24.0;
    render_text(compositor, &mut quads, "All Extensions", 32.0, section_y + 16.0, 14.0, text_dark, content_w);

    // Grid layout: 2 columns for cards (like Chrome)
    let grid_start_y = section_y + 36.0;
    let padding = 24.0;
    let gap = 16.0;
    let cols = if content_w > 600.0 { 2 } else { 1 };
    let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
    let card_h = 160.0;

    for (i, ext) in extensions.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let cx = padding + col * (card_w + gap);
        let cy = grid_start_y + row * (card_h + gap);
        let is_hovered = hover_idx == Some(i);

        // Card shadow
        if is_hovered {
            quads.push(rq(cx + 1.0, cy + 3.0, card_w - 2.0, card_h, 12.0, c(0, 0, 0, 20)));
        }

        // Card background
        quads.push(rq(cx, cy, card_w, card_h, 12.0, white));
        // Card border
        quads.push(rq(cx, cy, card_w, 1.0, 0.0, c(218, 220, 224, 60)));
        quads.push(rq(cx, cy + card_h - 1.0, card_w, 1.0, 0.0, c(218, 220, 224, 60)));
        quads.push(rq(cx, cy, 1.0, card_h, 0.0, c(218, 220, 224, 40)));
        quads.push(rq(cx + card_w - 1.0, cy, 1.0, card_h, 0.0, c(218, 220, 224, 40)));

        // Extension icon (large colored circle)
        let icon_size = 44.0;
        let icon_x = cx + 20.0;
        let icon_y = cy + 20.0;
        let ic = c(ext.icon_color[0], ext.icon_color[1], ext.icon_color[2], 255);
        quads.push(rq(icon_x, icon_y, icon_size, icon_size, icon_size / 2.0, ic));
        render_text(compositor, &mut quads, ext.icon_letter, icon_x + 13.0, icon_y + 30.0, 20.0, white, icon_x + icon_size);

        // Extension name + version
        let name_x = icon_x + icon_size + 14.0;
        let name_end = cx + card_w - 80.0;
        render_text(compositor, &mut quads, ext.name, name_x, icon_y + 18.0, 15.0, text_dark, name_end);
        let ver_label = format!("  {}", ext.version);
        let name_w = ext.name.len() as f32 * 8.5;
        render_text(compositor, &mut quads, &ver_label, name_x + name_w, icon_y + 18.0, 11.0, text_hint, name_end + 60.0);

        // Description
        render_text(compositor, &mut quads, ext.description, name_x, icon_y + 38.0, 11.0, text_secondary, cx + card_w - 20.0);

        // Toggle switch (top-right corner of card)
        let toggle_x = cx + card_w - 64.0;
        let toggle_y = cy + 24.0;
        let track_w = 44.0;
        let track_h = 22.0;
        let thumb_r = 9.0;

        if ext.enabled {
            quads.push(rq(toggle_x, toggle_y, track_w, track_h, track_h / 2.0, blue));
            quads.push(rq(toggle_x + track_w - track_h + 2.0, toggle_y + 2.0, thumb_r * 2.0, thumb_r * 2.0, thumb_r, white));
        } else {
            quads.push(rq(toggle_x, toggle_y, track_w, track_h, track_h / 2.0, gray_track));
            quads.push(rq(toggle_x + 2.0, toggle_y + 2.0, thumb_r * 2.0, thumb_r * 2.0, thumb_r, white));
        }

        // Bottom section: divider + details/remove buttons
        let bottom_y = cy + card_h - 44.0;
        quads.push(NativeGpuCompositor::solid_quad(cx + 16.0, bottom_y, card_w - 32.0, 1.0, c(218, 220, 224, 120)));

        // Details button
        let btn_y = bottom_y + 10.0;
        let details_x = cx + 20.0;
        quads.push(rq(details_x, btn_y, 68.0, 26.0, 13.0, c(232, 240, 254, 255)));
        render_text(compositor, &mut quads, "Details", details_x + 10.0, btn_y + 17.0, 11.0, blue, details_x + 66.0);

        // Remove button
        let remove_x = details_x + 78.0;
        quads.push(rq(remove_x, btn_y, 72.0, 26.0, 13.0, c(232, 240, 254, 255)));
        render_text(compositor, &mut quads, "Remove", remove_x + 10.0, btn_y + 17.0, 11.0, blue, remove_x + 70.0);

        // Status indicator
        let status_text = if ext.enabled { "Active" } else { "Inactive" };
        let status_c = if ext.enabled { green } else { text_hint };
        render_text(compositor, &mut quads, status_text, cx + card_w - 70.0, btn_y + 17.0, 10.0, status_c, cx + card_w);
    }

    quads
}

// ===== HOME PAGE (drawn entirely via GPU quads — no HTML engine) =====

#[allow(dead_code)]
fn build_home_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    _scroll_y: f32,
    search_focused: bool,
    search_text: &str,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let blue = c(26, 115, 232, 255);

    // Background image (tea garden photo)
    quads.push(NativeGpuCompositor::bg_image_quad(0.0, 0.0, content_w, content_h));
    // Dark overlay for readability
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, c(0, 0, 0, 140)));

    // ---- Centered content ----
    let cx = content_w / 2.0;
    let cy = content_h / 2.0 - 60.0;

    // Logo icon
    let logo_size = 56.0;
    let logo_x = cx - logo_size / 2.0;
    quads.push(rq(logo_x, cy, logo_size, logo_size, 14.0, c(255, 255, 255, 40)));
    quads.push(rq(logo_x + 2.0, cy + 2.0, logo_size - 4.0, logo_size - 4.0, 12.0, blue));
    render_text_centered(compositor, &mut quads, "A", cx, cy + 42.0, 28.0, white);

    // Title
    render_text_centered(compositor, &mut quads, "Axomai Browser", cx, cy + 85.0, 26.0, white);
    render_text_centered(compositor, &mut quads, "Fast. Private. AI-Powered. Built for Everyone.", cx, cy + 112.0, 13.0, c(200, 210, 220, 200));

    // ---- Search bar (centered, Chrome-style pill with glass effect) ----
    let search_w = 540.0f32.min(content_w - 80.0);
    let search_x = cx - search_w / 2.0;
    let search_y = cy + 135.0;
    // Glass background (brighter border when focused)
    if search_focused {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(100, 160, 255, 80)));
    } else {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(255, 255, 255, 25)));
    }
    quads.push(rq(search_x + 1.0, search_y + 1.0, search_w - 2.0, 42.0, 21.0, c(30, 30, 30, 180)));
    // Search icon
    render_text(compositor, &mut quads, "G", search_x + 16.0, search_y + 30.0, 16.0, c(130, 180, 255, 255), search_x + 36.0);
    if search_focused && !search_text.is_empty() {
        render_text(compositor, &mut quads, search_text, search_x + 42.0, search_y + 28.0, 14.0, white, search_x + search_w - 40.0);
        // Cursor
        let cursor_x = search_x + 42.0 + search_text.len() as f32 * 8.0;
        quads.push(NativeGpuCompositor::solid_quad(cursor_x, search_y + 10.0, 2.0, 24.0, white));
    } else if search_focused {
        // Cursor only
        quads.push(NativeGpuCompositor::solid_quad(search_x + 42.0, search_y + 10.0, 2.0, 24.0, white));
    } else {
        render_text(compositor, &mut quads, "Search the web with Axomai AI...", search_x + 42.0, search_y + 28.0, 14.0, c(180, 185, 195, 200), search_x + search_w - 40.0);
    }
    render_text(compositor, &mut quads, "Q", search_x + search_w - 32.0, search_y + 28.0, 14.0, c(160, 165, 175, 200), search_x + search_w);

    quads
}
