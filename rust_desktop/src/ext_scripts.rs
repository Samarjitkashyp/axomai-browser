//! JavaScript for the built-in extensions.
//!
//! * `doc_start` runs at the start of every document (registered through WebView2, see `web::com::DocScript`)
//!   and carries the cosmetic half of the AdBlock Shield and the Anti-Fingerprint Privacy Guard.
//!   The network half of both lives in `web.rs` / `blocklist.rs`.
//! * `reader`, `translate` and `capture_menu` are run on demand when the user opens that extension.

use crate::theme::Theme;

/// Selectors are exact class/id tokens or attribute prefixes, never substring matches such as `[class*="ad"]`,
/// which would also hide `header`, `download`, `shadow`, ...
pub const COSMETIC_CSS: &str = r#"ins.adsbygoogle,.adsbygoogle,[id^="google_ads_"],[id^="div-gpt-ad"],[id^="taboola-"],[id^="outbrain_"],div[data-ad-slot],div[data-google-query-id],iframe[id^="google_ads_iframe"],iframe[src*="doubleclick.net"],iframe[src*="googlesyndication.com"],amp-ad,amp-sticky-ad,.ad-container,.ad-slot,.ad-banner,.ad-wrapper,.ad-unit,.advert,.advertisement,.sponsored-ad,.banner-ad,.native-ad,.sticky-ad,.OUTBRAIN,.trc_related_container,.taboola-widget,#ad-slot,#google_ads,#advertisement{display:none!important;visibility:hidden!important}"#;

const READY: &str = "function ready(fn){if(document.documentElement){fn()}else{var o=new MutationObserver(function(){if(document.documentElement){o.disconnect();fn()}});o.observe(document,{childList:true})}}";

const PRIVACY_JS: &str = r#"
try{
  var seed=(Math.random()*4294967296)>>>0;
  function rnd(){seed=(seed*1664525+1013904223)>>>0;return seed/4294967296}
  var origGet=CanvasRenderingContext2D.prototype.getImageData;
  function noisy(canvas){
    try{
      var w=canvas.width,h=canvas.height;
      if(!w||!h||w*h>4000000)return canvas;
      var c=document.createElement('canvas');c.width=w;c.height=h;
      var x=c.getContext('2d');x.drawImage(canvas,0,0);
      var d=origGet.call(x,0,0,w,h),a=d.data,step=4*(53+Math.floor(rnd()*40));
      for(var i=0;i<a.length;i+=step){a[i]=a[i]^1;a[i+1]=a[i+1]^((rnd()*2)|0)}
      x.putImageData(d,0,0);return c;
    }catch(e){return canvas}
  }
  var oURL=HTMLCanvasElement.prototype.toDataURL;
  HTMLCanvasElement.prototype.toDataURL=function(){return oURL.apply(noisy(this),arguments)};
  var oBlob=HTMLCanvasElement.prototype.toBlob;
  HTMLCanvasElement.prototype.toBlob=function(){return oBlob.apply(noisy(this),arguments)};
  CanvasRenderingContext2D.prototype.getImageData=function(){
    var r=origGet.apply(this,arguments),a=r.data;
    if(a.length<=4*65536){for(var i=0;i<a.length;i+=4*(37+Math.floor(rnd()*20))){a[i]=a[i]^1}}
    return r;
  };
}catch(e){}
try{
  [window.WebGLRenderingContext,window.WebGL2RenderingContext].forEach(function(C){
    if(!C)return;
    var g=C.prototype.getParameter;
    C.prototype.getParameter=function(p){
      if(p===37445)return 'Axomai Shield';
      if(p===37446)return 'Generic Renderer';
      return g.apply(this,arguments);
    };
  });
}catch(e){}
try{
  Object.defineProperty(Navigator.prototype,'hardwareConcurrency',{get:function(){return 4},configurable:true});
  Object.defineProperty(Navigator.prototype,'deviceMemory',{get:function(){return 8},configurable:true});
  if(navigator.getBattery){Object.defineProperty(Navigator.prototype,'getBattery',{value:undefined,configurable:true})}
}catch(e){}
try{
  if(window.AudioBuffer){
    var gcd=AudioBuffer.prototype.getChannelData;
    AudioBuffer.prototype.getChannelData=function(){
      var d=gcd.apply(this,arguments);
      if(!this.__axn){this.__axn=1;for(var i=0;i<d.length;i+=100){d[i]+=(rnd()-0.5)*1e-7}}
      return d;
    };
  }
  if(window.AnalyserNode){
    var gf=AnalyserNode.prototype.getFloatFrequencyData;
    AnalyserNode.prototype.getFloatFrequencyData=function(a){gf.apply(this,arguments);for(var i=0;i<a.length;i++){a[i]+=(rnd()-0.5)*1e-3}};
  }
}catch(e){}
"#;

pub fn doc_start(adblock: bool, privacy: bool) -> String {
    if !adblock && !privacy {
        return String::new();
    }
    let mut js = String::from("(function(){if(window.__axomaiDS)return;try{Object.defineProperty(window,'__axomaiDS',{value:1})}catch(e){return}\n");
    js.push_str(READY);
    js.push('\n');
    if adblock {
        js.push_str(&format!(
            "try{{ready(function(){{var s=document.createElement('style');s.id='__axomai_cosmetic';s.textContent='{}';document.documentElement.appendChild(s)}})}}catch(e){{}}\n",
            COSMETIC_CSS.replace('\\', "\\\\").replace('\'', "\\'")
        ));
    }
    if privacy {
        js.push_str(PRIVACY_JS);
    }
    js.push_str("})();");
    js
}

/// Shared helpers for the on-demand scripts: closed shadow root + base styling.
pub fn shell(theme: &Theme, id: &str) -> String {
    format!(
        r#"var old=document.getElementById('{id}');if(old){{old.remove();return}}
document.querySelectorAll('[id^="__ax_pop_"]').forEach(function(e){{e.remove()}});
var host=document.createElement('div');host.id='{id}';host.style.cssText='all:initial;position:fixed;top:0;left:0;width:0;height:0;z-index:2147483647';
var root=host.attachShadow({{mode:'closed'}});
var base=document.createElement('style');
base.textContent=':host{{all:initial;{vars}}}*{{box-sizing:border-box;font-family:"Plus Jakarta Sans","Segoe UI",system-ui,sans-serif}}';
root.appendChild(base);
function css(t){{var s=document.createElement('style');s.textContent=t;root.appendChild(s)}}"#,
        id = id,
        vars = theme.css_vars()
    )
}

/// Reader Mode: strips the page to its article and shows it with typography controls.
pub fn reader(theme: &Theme) -> String {
    let mut js = String::from("(function(){");
    js.push_str(&shell(theme, "__ax_pop_reader"));
    js.push_str(
        r#"
function score(el){
  var ps=el.querySelectorAll('p'),n=0;
  for(var i=0;i<ps.length;i++){var t=ps[i].innerText||'';if(t.length>40)n+=t.length}
  return n-el.querySelectorAll('a').length*15;
}
function pick(){
  // The best container is the one holding the most paragraph text with the least link clutter; wrappers that
  // also hold navigation score lower than the article inside them.
  var list=document.querySelectorAll('article,main,[role="main"],[itemprop="articleBody"],.mw-parser-output,#mw-content-text,.post-content,.entry-content,.article-body,.article-content,.story-body,.story-detail,.full_story,.content-area,div,section');
  var best=null,bs=0,max=Math.min(list.length,2500);
  for(var i=0;i<max;i++){var s=score(list[i]);if(s>bs){bs=s;best=list[i]}}
  return bs>300?best:null;
}
var src=pick();
var box=document.createElement('div');box.className='wrap paper';
var bar=document.createElement('div');bar.className='bar';
function btn(t,fn,title){var b=document.createElement('button');b.textContent=t;b.title=title||t;b.onclick=fn;bar.appendChild(b);return b}
var badge=document.createElement('span');badge.className='badge';badge.textContent='READER MODE';bar.appendChild(badge);
var art=document.createElement('article');
var h=document.createElement('h1');h.textContent=document.title;art.appendChild(h);
var meta=document.createElement('div');meta.className='meta';art.appendChild(meta);
var words=0;
if(src){
  var clone=src.cloneNode(true);
  clone.querySelectorAll('script,style,iframe,form,nav,aside,footer,button,input,select,noscript,svg,video,audio,ins,[class*="share"],[class*="social"],[class*="related"],[class*="comment"],[class*="newsletter"],[class*="advert"],[id*="comment"]').forEach(function(e){e.remove()});
  clone.querySelectorAll('*').forEach(function(e){
    Array.prototype.slice.call(e.attributes).forEach(function(a){
      var n=a.name.toLowerCase();
      if(n.indexOf('on')===0||n==='style'||n==='class'||n==='id')e.removeAttribute(a.name);
    });
    if(e.tagName==='A'){e.setAttribute('target','_blank');e.setAttribute('rel','noopener noreferrer')}
    if(e.tagName==='IMG'&&!e.getAttribute('src')&&e.dataset&&e.dataset.src){e.setAttribute('src',e.dataset.src)}
  });
  words=(clone.innerText||'').split(/\s+/).filter(Boolean).length;
  art.appendChild(clone);
}else{
  var p=document.createElement('p');p.textContent='This page does not look like an article, so Reader Mode has nothing to show. Open a specific article page and try again.';art.appendChild(p);
}
meta.textContent=location.hostname+(words?'  ·  '+Math.max(1,Math.round(words/200))+' min read':'');
var size=19,theme='paper';
function apply(){box.style.fontSize=size+'px';box.className='wrap '+theme}
btn('A−',function(){size=Math.max(14,size-2);apply()},'Smaller text');
btn('A+',function(){size=Math.min(30,size+2);apply()},'Larger text');
btn('Paper',function(){theme='paper';apply()});
btn('Sepia',function(){theme='sepia';apply()});
btn('Dark',function(){theme='dark';apply()});
var x=btn('Exit',function(){host.remove()},'Close Reader Mode (Esc)');x.className='exit';
box.appendChild(bar);box.appendChild(art);
css('.wrap{position:fixed;inset:0;overflow:auto;padding:0 0 80px;font-family:Georgia,"Times New Roman",serif;line-height:1.85}'+
'.wrap.paper{background:#faf9f6;color:#1a1a1a}.wrap.sepia{background:#f4ecd8;color:#433422}.wrap.dark{background:#16181d;color:#d7dae0}'+
'.bar{position:sticky;top:0;display:flex;gap:8px;align-items:center;padding:10px 18px;background:inherit;border-bottom:1px solid rgba(128,128,128,.25);font:600 12px "Segoe UI",sans-serif;z-index:2}'+
'.badge{background:var(--primary);color:#fff;padding:5px 12px;border-radius:14px;letter-spacing:.5px;margin-right:auto}'+
'.bar button{border:1px solid rgba(128,128,128,.35);background:rgba(128,128,128,.12);color:inherit;border-radius:14px;padding:5px 12px;cursor:pointer;font:inherit}'+
'.bar button:hover{background:rgba(128,128,128,.25)}.bar .exit{background:var(--primary);color:#fff;border-color:transparent}'+
'article{max-width:720px;margin:0 auto;padding:28px 24px}h1{font:700 1.7em/1.3 "Segoe UI",sans-serif;margin:12px 0 6px}'+
'.meta{font:13px "Segoe UI",sans-serif;opacity:.65;margin-bottom:22px}article img{max-width:100%;height:auto;border-radius:8px;margin:14px 0}'+
'article a{color:#2563eb}article blockquote{border-left:4px solid var(--primary);margin:16px 0;padding:4px 16px;opacity:.9}'+
'article pre{overflow:auto;background:rgba(128,128,128,.15);padding:12px;border-radius:8px}article p{margin:0 0 1em}');
root.appendChild(box);
document.documentElement.appendChild(host);
document.addEventListener('keydown',function k(e){if(e.key==='Escape'){host.remove();document.removeEventListener('keydown',k,true)}},true);
})();"#,
    );
    js
}

pub const TRANSLATE_LANGS: &[(&str, &str)] = &[
    ("as", "অসমীয়া  Assamese"),
    ("en", "English"),
    ("hi", "हिन्दी  Hindi"),
    ("bn", "বাংলা  Bengali"),
    ("ta", "தமிழ்  Tamil"),
    ("te", "తెలుగు  Telugu"),
    ("mr", "मराठी  Marathi"),
    ("gu", "ગુજરાતી  Gujarati"),
    ("es", "Español  Spanish"),
    ("fr", "Français  French"),
    ("de", "Deutsch  German"),
    ("ja", "日本語  Japanese"),
];

/// Language picker; translation itself is performed by Google Translate's page proxy.
pub fn translate(theme: &Theme) -> String {
    let mut js = String::from("(function(){");
    js.push_str(&shell(theme, "__ax_pop_translate"));
    let langs: Vec<String> = TRANSLATE_LANGS
        .iter()
        .map(|(code, label)| format!("['{}','{}']", code, label))
        .collect();
    js.push_str(&format!(
        r#"
var langs=[{langs}];
var card=document.createElement('div');card.className='card';
var t=document.createElement('div');t.className='title';t.textContent='🌐 Translate this page to…';card.appendChild(t);
langs.forEach(function(l){{
  var b=document.createElement('button');b.className='lang';b.textContent=l[1];
  b.onclick=function(){{
    var u='https://translate.google.com/translate?sl=auto&tl='+l[0]+'&u='+encodeURIComponent(location.href);
    host.remove();location.href=u;
  }};
  card.appendChild(b);
}});
var c=document.createElement('button');c.className='close';c.textContent='Close';c.onclick=function(){{host.remove()}};card.appendChild(c);
css('.card{{position:fixed;right:24px;bottom:24px;width:360px;max-height:80vh;overflow:auto;background:var(--bg);border:1px solid var(--border);border-radius:14px;box-shadow:var(--shadow);padding:10px;color:var(--text);display:grid;grid-template-columns:1fr 1fr;gap:4px;font-size:13px}}'+
'.title{{grid-column:1/-1;font-weight:700;color:var(--heading);padding:4px 6px 8px}}'+
'.lang{{text-align:left;border:0;background:transparent;color:var(--text);padding:8px 10px;border-radius:9px;cursor:pointer;font-size:13px}}.lang:hover{{background:var(--hover)}}'+
'.close{{grid-column:1/-1;margin-top:6px;border:0;border-radius:9px;padding:8px;background:var(--primary);color:#fff;font-weight:600;cursor:pointer}}');
root.appendChild(card);document.documentElement.appendChild(host);
}})();"#,
        langs = langs.join(",")
    ));
    js
}

/// Screen Capture Studio menu + drag-to-select overlay. Commands go back to the browser as
/// `axomai://<token>/capture/...` and `axomai://<token>/snip/x,y,w,h,dpr`.
pub fn capture_menu(theme: &Theme, token: &str) -> String {
    let mut js = format!("(function(){{var TOKEN='{}';function go(c){{try{{window.chrome.webview.postMessage(TOKEN+'/'+c)}}catch(x){{location.href='axomai://'+TOKEN+'/'+c}}}}", token);
    js.push_str(&shell(theme, "__ax_pop_capture"));
    js.push_str(
        r#"
function snip(){
  host.remove();
  var ov=document.createElement('div');ov.id='__ax_pop_snip';
  ov.style.cssText='all:initial;position:fixed;inset:0;z-index:2147483647;cursor:crosshair;background:rgba(0,0,0,.28)';
  var r=ov.attachShadow({mode:'closed'});var box=document.createElement('div');
  box.style.cssText='position:fixed;border:2px solid #10b981;background:rgba(16,185,129,.15);display:none;pointer-events:none';
  var tip=document.createElement('div');tip.textContent='Drag to select an area · Esc to cancel';
  tip.style.cssText='position:fixed;top:14px;left:50%;transform:translateX(-50%);background:#111827;color:#fff;padding:8px 16px;border-radius:20px;font:600 13px Segoe UI,sans-serif';
  r.appendChild(box);r.appendChild(tip);document.documentElement.appendChild(ov);
  var sx=0,sy=0,drag=false;
  function done(ok,x,y,w,h){ov.remove();document.removeEventListener('keydown',key,true);
    if(ok&&w>4&&h>4){setTimeout(function(){go('snip/'+Math.round(x+scrollX)+','+Math.round(y+scrollY)+','+Math.round(w)+','+Math.round(h)+','+(window.devicePixelRatio||1))},160)}}
  function key(e){if(e.key==='Escape'){e.preventDefault();done(false)}}
  document.addEventListener('keydown',key,true);
  ov.addEventListener('mousedown',function(e){drag=true;sx=e.clientX;sy=e.clientY;box.style.display='block';e.preventDefault()});
  ov.addEventListener('mousemove',function(e){if(!drag)return;var x=Math.min(sx,e.clientX),y=Math.min(sy,e.clientY);
    box.style.left=x+'px';box.style.top=y+'px';box.style.width=Math.abs(e.clientX-sx)+'px';box.style.height=Math.abs(e.clientY-sy)+'px'});
  ov.addEventListener('mouseup',function(e){if(!drag)return;drag=false;var x=Math.min(sx,e.clientX),y=Math.min(sy,e.clientY);
    ov.style.background='transparent';box.style.display='none';tip.style.display='none';
    done(true,x,y,Math.abs(e.clientX-sx),Math.abs(e.clientY-sy))});
}
var card=document.createElement('div');card.className='card';
var t=document.createElement('div');t.className='title';t.textContent='📸 Screen Capture Studio';card.appendChild(t);
function item(emoji,title,sub,fn){var b=document.createElement('button');b.className='item';
  b.innerHTML='<span class="e"></span><span class="tx"><b></b><i></i></span>';
  b.querySelector('.e').textContent=emoji;b.querySelector('b').textContent=title;b.querySelector('i').textContent=sub;
  b.onclick=fn;card.appendChild(b)}
item('🖥️','Visible area','What you see right now',function(){host.remove();setTimeout(function(){go('capture/visible')},160)});
item('📜','Full page','Whole page, top to bottom',function(){host.remove();setTimeout(function(){go('capture/full')},160)});
item('✂️','Select area','Drag a rectangle (snip tool)',snip);
item('📂','Open screenshots folder','Where captures are saved',function(){host.remove();go('open-folder')});
css('.card{position:fixed;top:6px;right:120px;width:290px;background:var(--bg);border:1px solid var(--border);border-radius:14px;box-shadow:var(--shadow);padding:8px;color:var(--text)}'+
'.title{font-weight:700;color:var(--heading);padding:6px 8px 8px;font-size:14px}'+
'.item{display:flex;gap:12px;align-items:center;width:100%;text-align:left;border:0;background:transparent;color:inherit;padding:9px 10px;border-radius:10px;cursor:pointer}'+
'.item:hover{background:var(--hover)}.e{font-size:20px;width:26px;text-align:center}.tx{display:flex;flex-direction:column}'+
'.tx b{font-size:13px;color:var(--heading)}.tx i{font-style:normal;font-size:11px;color:var(--muted)}');
root.appendChild(card);document.documentElement.appendChild(host);
document.addEventListener('keydown',function k(e){if(e.key==='Escape'){host.remove();document.removeEventListener('keydown',k,true)}},true);
})();"#,
    );
    js
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_start_is_empty_when_nothing_enabled() {
        assert!(doc_start(false, false).is_empty());
    }

    #[test]
    fn doc_start_includes_selected_parts() {
        let both = doc_start(true, true);
        assert!(both.contains("__axomai_cosmetic"));
        assert!(both.contains("UNMASKED") || both.contains("37445"));
        let only_ads = doc_start(true, false);
        assert!(only_ads.contains("__axomai_cosmetic"));
        assert!(!only_ads.contains("37445"));
    }

    #[test]
    fn cosmetic_css_has_no_substring_class_selectors() {
        assert!(!COSMETIC_CSS.contains("[class*="));
        assert!(!COSMETIC_CSS.contains("[id*="));
    }

    #[test]
    fn generated_scripts_are_balanced() {
        let t = crate::theme::by_id("tea-garden");
        for js in [reader(t), translate(t), capture_menu(t, "tok")] {
            assert_eq!(js.matches('{').count(), js.matches('}').count(), "unbalanced braces");
            assert_eq!(js.matches('(').count(), js.matches(')').count(), "unbalanced parens");
        }
    }
}
