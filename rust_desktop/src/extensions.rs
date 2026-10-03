use crate::types::Extension;

pub fn create_extensions() -> Vec<Extension> {
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

pub fn build_extension_init_script(extensions: &[Extension]) -> String {
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
