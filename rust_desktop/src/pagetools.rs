//! Page tools: picture-in-picture video, and the developer panel (user-agent and mobile-view emulation).

use crate::app::App;
use crate::overlays;
use serde_json::json;

/// `(key, label, user agent, width, height, device pixel ratio, mobile)`; the `default` profile clears every override.
pub const PROFILES: &[(&str, &str, &str, u32, u32, f32, bool)] = &[
    ("default", "Normal desktop", "", 0, 0, 1.0, false),
    (
        "iphone",
        "iPhone (mobile view)",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
        390,
        844,
        3.0,
        true,
    ),
    (
        "android",
        "Android phone (mobile view)",
        "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36",
        412,
        915,
        2.625,
        true,
    ),
    (
        "ipad",
        "iPad (tablet view)",
        "Mozilla/5.0 (iPad; CPU OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
        820,
        1180,
        2.0,
        true,
    ),
    (
        "firefox",
        "Firefox on Windows (user agent only)",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
        0,
        0,
        1.0,
        false,
    ),
    (
        "safari",
        "Safari on Mac (user agent only)",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
        0,
        0,
        1.0,
        false,
    ),
];

pub fn profile(key: &str) -> Option<&'static (&'static str, &'static str, &'static str, u32, u32, f32, bool)> {
    PROFILES.iter().find(|p| p.0 == key)
}

/// The DevTools-protocol calls (method, params) that put a profile in place.
pub fn emulation_calls(p: &(&str, &str, &str, u32, u32, f32, bool)) -> Vec<(&'static str, String)> {
    let (_, _, ua, w, h, dpr, mobile) = *p;
    let mut calls = vec![("Emulation.setUserAgentOverride", json!({"userAgent": ua}).to_string())];
    if w > 0 {
        calls.push(("Emulation.setDeviceMetricsOverride", json!({"width": w, "height": h, "deviceScaleFactor": dpr, "mobile": mobile}).to_string()));
        calls.push(("Emulation.setTouchEmulationEnabled", json!({"enabled": mobile}).to_string()));
    } else {
        calls.push(("Emulation.clearDeviceMetricsOverride", "{}".to_string()));
        calls.push(("Emulation.setTouchEmulationEnabled", json!({"enabled": false}).to_string()));
    }
    calls
}

/// Starts picture-in-picture for the video that is playing (or the biggest one); a second call leaves it.
pub const PIP_JS: &str = r#"(function(){
if(document.pictureInPictureElement){document.exitPictureInPicture();return 'exit'}
var vs=[].slice.call(document.querySelectorAll('video')).filter(function(v){return v.readyState>0&&v.videoWidth>0});
if(!vs.length)return 'none';
vs.sort(function(a,b){return (!b.paused-!a.paused)||(b.videoWidth*b.videoHeight-a.videoWidth*a.videoHeight)});
var v=vs[0];if(v.disablePictureInPicture)v.disablePictureInPicture=false;
v.requestPictureInPicture().catch(function(){});return 'pip';
})()"#;

const DEV_PANEL_JS: &str = r#"
var d=__DATA__;
css('.sub2{padding:2px 12px 8px;color:var(--muted);font-size:11.5px}.row.on .em{background:var(--primary);color:#fff}');
var card=el('div','card');
var hd=el('div','hd');hd.appendChild(el('b','','🛠️ Developer panel'));card.appendChild(hd);
card.appendChild(el('div','sub2','Pretend to be another browser or device for this tab. The page reloads.'));
d.profiles.forEach(function(p){
  var r=el('div','row'+(p[0]===d.active?' on':''));r.appendChild(el('span','em',p[2]));
  var tx=el('span','tx');tx.appendChild(el('b','',p[1]));r.appendChild(tx);
  r.onclick=function(){host.remove();go('emu/'+p[0])};card.appendChild(r)});
card.appendChild(el('div','div'));
[['🧰','Open DevTools (F12)','devtools'],['📄','View page source','viewsource-current']].forEach(function(a){
  var r=el('div','row');r.appendChild(el('span','em',a[0]));var tx=el('span','tx');tx.appendChild(el('b','',a[1]));r.appendChild(tx);
  r.onclick=function(){host.remove();go(a[2])};card.appendChild(r)});
finish(card);
"#;

/// The developer panel popup. `active` is the key of the profile the tab uses now.
pub fn dev_panel(theme: &crate::theme::Theme, token: &str, active: &str) -> String {
    let profiles: Vec<_> = PROFILES
        .iter()
        .map(|p| json!([p.0, p.1, if p.0 == "default" { "\u{1F5A5}" } else if p.6 { "\u{1F4F1}" } else { "\u{1F310}" }]))
        .collect();
    let data = json!({"profiles": profiles, "active": active});
    let mut js = overlays::open(theme, token, "__ax_pop_dev");
    js.push_str(&overlays::css_wrap(".card{right:24px;top:8px}", 330.0));
    js.push_str(&DEV_PANEL_JS.replace("__DATA__", &data.to_string()));
    js.push_str("})();");
    js
}

impl App {
    fn toast_tools(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    pub fn toggle_pip(&mut self) {
        let t = &self.tabs[self.active];
        if !matches!(t.kind, crate::tabs::TabKind::Web) {
            return self.toast_tools("Open a page with a video first");
        }
        if let Some(wv) = &self.webview {
            // A user gesture is needed for picture-in-picture; the DevTools protocol can supply one.
            crate::web::com::cdp(wv, "Runtime.evaluate", &json!({"expression": PIP_JS, "userGesture": true}).to_string());
        }
    }

    pub fn open_dev_panel(&mut self) {
        let active = self.tabs[self.active].emu;
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            let _ = wv.focus();
            let _ = wv.evaluate_script(&dev_panel(self.core.theme, &shared.token, active));
        }
    }

    pub fn set_emulation(&mut self, key: &str) {
        let Some(p) = profile(key) else { return };
        let idx = self.active;
        self.tabs[idx].emu = p.0;
        if let Some(wv) = &self.webview {
            for (method, params) in emulation_calls(p) {
                crate::web::com::cdp(wv, method, &params);
            }
            crate::web::com::reload_hard(wv);
        }
        self.toast_tools(&format!("Now showing this tab as: {}", p.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_clears_everything() {
        let calls = emulation_calls(profile("default").unwrap());
        assert!(calls.iter().any(|c| c.0 == "Emulation.clearDeviceMetricsOverride"));
        assert!(calls[0].1.contains("\"userAgent\":\"\""));
    }

    #[test]
    fn phone_profile_sets_agent_size_and_touch() {
        let calls = emulation_calls(profile("iphone").unwrap());
        assert!(calls[0].1.contains("iPhone"));
        let metrics = calls.iter().find(|c| c.0 == "Emulation.setDeviceMetricsOverride").unwrap();
        assert!(metrics.1.contains("\"width\":390") && metrics.1.contains("\"mobile\":true"));
        assert!(calls.iter().any(|c| c.0 == "Emulation.setTouchEmulationEnabled" && c.1.contains("true")));
    }

    #[test]
    fn profile_keys_are_unique_and_unknown_ones_rejected() {
        let mut keys: Vec<&str> = PROFILES.iter().map(|p| p.0).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), PROFILES.len());
        assert!(profile("nope").is_none());
    }
}
