//! Optional page tweaks injected at the start of every web page: dark mode for websites, hiding cookie banners,
//! skipping YouTube ads and a cleaner printout. Each one is a Settings switch.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Tweaks {
    pub dark: bool,
    pub cookies: bool,
    pub youtube: bool,
    pub print: bool,
}

const HELPERS: &str = "function css(t,id){var s=document.createElement('style');s.id=id;s.textContent=t;var add=function(){(document.head||document.documentElement).appendChild(s)};if(document.documentElement){add()}else{var o=new MutationObserver(function(){if(document.documentElement){o.disconnect();add()}});o.observe(document,{childList:true})}return s}function ready(fn){if(document.readyState!=='loading'){fn()}else{document.addEventListener('DOMContentLoaded',fn)}}";

/// Inverts the page colours; images, video and similar are inverted back. A page that is already dark is left alone.
const DARK_JS: &str = r#"
(function(){
var st=css('html{filter:invert(1) hue-rotate(180deg)!important;background:#fff!important}img,picture,video,canvas,iframe,embed,object,svg image,[style*="background-image"]{filter:invert(1) hue-rotate(180deg)!important}','__ax_dark');
function lum(c){var m=c.match(/[\d.]+/g);if(!m||m.length<3)return 1;var a=m.length>3?+m[3]:1;if(a<0.1)return 1;return (0.2126*m[0]+0.7152*m[1]+0.0722*m[2])/255}
ready(function(){try{
  var meta=document.querySelector('meta[name=color-scheme]');
  var b=getComputedStyle(document.body||document.documentElement).backgroundColor,h=getComputedStyle(document.documentElement).backgroundColor;
  var l=lum(b)<1&&b!=='rgba(0, 0, 0, 0)'?lum(b):lum(h);
  if((meta&&/^\s*dark/.test(meta.content))||l<0.35){st.remove()}
}catch(e){}});
})();
"#;

const COOKIE_SELECTORS: &str = "#onetrust-banner-sdk,#onetrust-consent-sdk,#CybotCookiebotDialog,#CybotCookiebotDialogBodyUnderlay,#cookiebanner,#cookie-banner,#cookie-notice,#cookie-law-info-bar,#cookieChoiceInfo,#gdpr-cookie-message,#truste-consent-track,#didomi-host,#usercentrics-root,#qc-cmp2-container,.qc-cmp2-container,.cookie-banner,.cookie-notice,.cookie-consent,.cookie-popup,.cc-window,.cc-banner,.cmplz-cookiebanner,.fc-consent-root,.osano-cm-window,.evidon-banner,.gdpr-cookie-notice,[id^=\"sp_message_container_\"],[aria-label=\"cookieconsent\"],[aria-label=\"Cookie banner\"]";

const COOKIE_JS: &str = r#"
(function(){
var sel=SELECTORS;
css(sel+'{display:none!important;visibility:hidden!important}','__ax_cookie');
var tries=0;
function unlock(){try{if(document.querySelector(sel)){[document.documentElement,document.body].forEach(function(e){if(e&&getComputedStyle(e).overflow==='hidden'){e.style.setProperty('overflow','auto','important')}})}}catch(e){}}
var t=setInterval(function(){unlock();if(++tries>20)clearInterval(t)},500);
})();
"#;

const YOUTUBE_JS: &str = r#"
(function(){
if(!/(^|\.)youtube\.com$/.test(location.hostname))return;
css('.ytp-ad-module,.ytp-ad-overlay-container,.ytp-ad-image-overlay,#masthead-ad,#player-ads,ytd-ad-slot-renderer,ytd-in-feed-ad-layout-renderer,ytd-banner-promo-renderer,ytd-promoted-sparkles-web-renderer,ytd-display-ad-renderer,ytd-companion-slot-renderer{display:none!important}','__ax_yt');
var fast=false;
setInterval(function(){try{
  var p=document.querySelector('.html5-video-player'),v=document.querySelector('video.html5-main-video')||document.querySelector('video');
  if(p&&v&&p.classList.contains('ad-showing')){
    var skip=document.querySelector('.ytp-ad-skip-button,.ytp-ad-skip-button-modern,.ytp-skip-ad-button');
    if(skip){skip.click()}
    else if(isFinite(v.duration)&&v.duration>0&&v.currentTime<v.duration-0.1){v.muted=true;v.playbackRate=16;fast=true}
  }else if(fast&&v){v.playbackRate=1;v.muted=false;fast=false}
  var auto=document.querySelector('.ytp-autonav-toggle-button[aria-checked="true"]');
  if(auto){auto.click()}
}catch(e){}},500);
})();
"#;

const PRINT_CSS: &str = "@media print{nav,aside,footer,body>header,iframe,video,audio,button,[role=banner],[role=navigation],[role=complementary],.sidebar,#sidebar,.ads,.advert,.advertisement,ins.adsbygoogle,.cookie-banner,.share,.social,.sharing,.comments,#comments,.related,.newsletter,.popup,.modal{display:none!important}body{background:#fff!important;color:#000!important}*{box-shadow:none!important;text-shadow:none!important}}";

/// One tweak failing must not stop the others.
fn guard(js: &str) -> String {
    format!("try{{{}}}catch(e){{}}
", js)
}

impl Tweaks {
    pub fn any(&self) -> bool {
        self.dark || self.cookies || self.youtube || self.print
    }

    /// The document-start script for these tweaks; empty when all are off. It does nothing on our own pages.
    pub fn script(&self) -> String {
        if !self.any() {
            return String::new();
        }
        let mut s = format!("(function(){{if(!/^https?:$/.test(location.protocol))return;{}\n", HELPERS);
        if self.dark {
            s.push_str(&guard(DARK_JS));
        }
        if self.cookies {
            s.push_str(&guard(&COOKIE_JS.replace("SELECTORS", &serde_json::to_string(COOKIE_SELECTORS).unwrap_or_default())));
        }
        if self.youtube {
            s.push_str(&guard(YOUTUBE_JS));
        }
        if self.print {
            s.push_str(&format!("css({},'__ax_print');\n", serde_json::to_string(PRINT_CSS).unwrap_or_default()));
        }
        s.push_str("})();");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_on_means_no_script() {
        assert!(Tweaks::default().script().is_empty());
    }

    #[test]
    fn each_switch_adds_only_its_own_part() {
        let dark = Tweaks { dark: true, ..Default::default() }.script();
        assert!(dark.contains("__ax_dark") && !dark.contains("__ax_cookie") && !dark.contains("__ax_yt") && !dark.contains("__ax_print"));
        let cookie = Tweaks { cookies: true, ..Default::default() }.script();
        assert!(cookie.contains("onetrust") && !cookie.contains("SELECTORS") && !cookie.contains("__ax_dark"));
        assert!(Tweaks { youtube: true, ..Default::default() }.script().contains("ad-showing"));
        assert!(Tweaks { print: true, ..Default::default() }.script().contains("@media print"));
    }

    #[test]
    fn script_only_runs_on_web_pages() {
        let s = Tweaks { dark: true, cookies: true, youtube: true, print: true }.script();
        assert!(s.starts_with("(function(){if(!/^https?:$/.test(location.protocol))return;"));
        assert!(s.ends_with("})();"));
        assert_eq!(s.matches('{').count(), s.matches('}').count(), "braces balance");
    }
}
