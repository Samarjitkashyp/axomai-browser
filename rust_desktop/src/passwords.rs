//! Password manager: offers to save logins, fills them in on request, and manages the saved list.
//!
//! Trust rules: a page can only say "here is a login that was just submitted" or "I have a login form"; the site a
//! message belongs to is always taken from the web view (`WebMessageReceived.Source`), never from the message.
//! Nothing is stored until the user presses Save, and secrets are encrypted with the Windows account key.

use crate::app::App;
use crate::overlays;
use crate::tabs::TabKind;
use serde_json::Value;
use std::time::{Duration, Instant};

/// Injected into every web page (top frame only) while the password manager is on.
pub const PAGE_SCRIPT: &str = r#"(function(){
if(window.top!==window||window.__axpw||!/^https?:$/.test(location.protocol))return;
try{Object.defineProperty(window,'__axpw',{value:1})}catch(e){return}
function post(o){try{o.ax='pw';window.chrome.webview.postMessage(o)}catch(e){}}
function shown(e){return !!(e.offsetWidth||e.offsetHeight||e.getClientRects().length)}
function pwFields(){return Array.prototype.filter.call(document.querySelectorAll('input[type=password]'),shown)}
function userFor(p){var scope=p.form||document,all=scope.querySelectorAll('input:not([type]),input[type=text],input[type=email],input[type=tel]'),best=null;
 for(var i=0;i<all.length;i++){if(shown(all[i])&&(all[i].compareDocumentPosition(p)&4))best=all[i]}return best}
function capture(p){if(!p||!p.value)return;var u=userFor(p);post({k:'save',u:u?String(u.value):'',p:String(p.value)})}
document.addEventListener('submit',function(e){var f=e.target;if(f&&f.querySelector)capture(f.querySelector('input[type=password]'))},true);
document.addEventListener('keydown',function(e){var t=e.target;if(e.key==='Enter'&&t&&t.type==='password'&&!t.form)capture(t)},true);
document.addEventListener('click',function(e){var b=e.target&&e.target.closest&&e.target.closest('button,[type=submit],[role=button]');if(!b)return;var p=pwFields()[0];if(p&&p.value&&!p.form)capture(p)},true);
document.addEventListener('focusin',function(e){var t=e.target;if(!e.isTrusted||!t||t.tagName!=='INPUT')return;var ps=pwFields();if(!ps.length)return;
 var ok=t.type==='password'||(/^(text|email|tel|)$/.test(t.type||'')&&userFor(ps[0])===t);if(!ok)return;
 var r=t.getBoundingClientRect();post({k:'req',x:r.left,y:r.bottom,w:r.width})},true);
})();"#;

/// `scheme://host[:port]` of an http(s) address, lower-cased; `None` for anything else.
pub fn origin_of(url: &str) -> Option<String> {
    let lower = url.trim().to_ascii_lowercase();
    let (scheme, rest) = lower.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = rest.split(&['/', '?', '#'][..]).next()?;
    let host = authority.rsplit('@').next()?;
    if host.is_empty() || host.contains(char::is_whitespace) {
        return None;
    }
    Some(format!("{}://{}", scheme, host))
}

pub struct PwPending {
    pub tab_id: u64,
    pub origin: String,
    pub user: String,
    pub pass: String,
    pub update: bool,
    pub at: Instant,
    /// The prompt has been drawn on the page that is currently loaded.
    pub shown: bool,
}

pub struct PwOffer {
    pub origin: String,
    pub ids: Vec<i64>,
    /// When the list was shown; an old offer is not honoured.
    pub at: Instant,
}

fn host_label(origin: &str) -> &str {
    origin.split_once("://").map(|(_, h)| h).unwrap_or(origin)
}

impl App {
    fn toast_pw(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    /// A web page sent a JSON message (`source` is its address as reported by the web view).
    pub fn on_page_message(&mut self, tab_idx: usize, source: &str, json: &str) {
        if json.len() > 16 * 1024 {
            return;
        }
        let Ok(msg) = serde_json::from_str::<Value>(json) else { return };
        let ax = msg.get("ax").and_then(|v| v.as_str());
        if ax == Some("note") {
            self.on_note_message(tab_idx, source, &msg);
            return;
        }
        if ax != Some("pw") || !self.settings.password_manager {
            return;
        }
        let Some(origin) = origin_of(source) else { return };
        if !matches!(self.tabs[tab_idx].kind, TabKind::Web) {
            return;
        }
        match msg.get("k").and_then(|v| v.as_str()) {
            Some("save") => {
                let user = msg.get("u").and_then(|v| v.as_str()).unwrap_or("");
                let pass = msg.get("p").and_then(|v| v.as_str()).unwrap_or("");
                self.password_submitted(tab_idx, origin, user, pass);
            }
            Some("req") if tab_idx == self.active => {
                let (x, y, w) = (num(&msg, "x"), num(&msg, "y"), num(&msg, "w"));
                self.offer_logins(origin, x, y, w);
            }
            Some("fill") if tab_idx == self.active => {
                let i = msg.get("i").and_then(|v| v.as_u64()).unwrap_or(u64::MAX) as usize;
                self.fill_login(&origin, i);
            }
            Some("confirm") => {
                let answer = msg.get("a").and_then(|v| v.as_str()).unwrap_or("");
                self.answer_save_prompt(tab_idx, answer);
            }
            _ => {}
        }
    }

    fn password_submitted(&mut self, tab_idx: usize, origin: String, user: &str, pass: &str) {
        if self.private_window || self.tabs[tab_idx].private || pass.is_empty() || pass.chars().count() > 1000 || user.chars().count() > 200 {
            return;
        }
        let Some(s) = &self.storage else { return };
        if s.pw_never_has(&origin) {
            return;
        }
        let mut update = false;
        if let Some((id, _)) = s.credentials_for(&origin).into_iter().find(|(_, u)| u == user) {
            if s.password_secret(id).and_then(|b| crate::secret::decrypt(&b)).as_deref() == Some(pass) {
                return; // already saved with this password
            }
            update = true;
        }
        self.pw_pending = Some(PwPending { tab_id: self.tabs[tab_idx].id, origin, user: user.to_string(), pass: pass.to_string(), update, at: Instant::now(), shown: false });
    }

    /// Called every frame: draw the save prompt once the page after the login has settled.
    pub fn show_pending_prompt(&mut self) {
        let Some(p) = &mut self.pw_pending else { return };
        if p.at.elapsed() > Duration::from_secs(90) {
            self.pw_pending = None;
            return;
        }
        let tab = &self.tabs[self.active];
        if p.shown || tab.id != p.tab_id || tab.loading || p.at.elapsed() < Duration::from_millis(1200) {
            return;
        }
        p.shown = true;
        let (host, user, update) = (host_label(&p.origin).to_string(), p.user.clone(), p.update);
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script(&overlays::pw_prompt(self.core.theme, &host, &user, update));
        }
    }

    /// A new page started loading in this tab: a prompt that was drawn on the old page must be drawn again.
    pub fn password_page_changed(&mut self, tab_id: u64) {
        if let Some(p) = &mut self.pw_pending {
            if p.tab_id == tab_id {
                p.shown = false;
            }
        }
    }

    fn answer_save_prompt(&mut self, tab_idx: usize, answer: &str) {
        let Some(p) = self.pw_pending.take() else { return };
        if p.tab_id != self.tabs[tab_idx].id {
            self.pw_pending = Some(p);
            return;
        }
        match answer {
            "save" => {
                let stored = crate::secret::encrypt(&p.pass).and_then(|blob| self.storage.as_ref().and_then(|s| s.save_password(&p.origin, &p.user, &blob).ok()));
                self.toast_pw(if stored.is_some() { "\u{1F511} Password saved" } else { "Could not save the password" });
            }
            "never" => {
                if let Some(s) = &self.storage {
                    let _ = s.pw_never_add(&p.origin);
                }
                self.toast_pw("Passwords will not be offered for this site");
            }
            _ => {}
        }
        self.reload_passwords_page();
    }

    fn offer_logins(&mut self, origin: String, x: f64, y: f64, w: f64) {
        let logins = self.storage.as_ref().map(|s| s.credentials_for(&origin)).unwrap_or_default();
        if logins.is_empty() {
            return;
        }
        let users: Vec<String> = logins.iter().map(|(_, u)| if u.is_empty() { "(no username)".to_string() } else { u.clone() }).collect();
        self.pw_offer = Some(PwOffer { origin: origin.clone(), ids: logins.into_iter().map(|(id, _)| id).collect(), at: Instant::now() });
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script(&overlays::pw_offer(self.core.theme, host_label(&origin), x, y, w, &users));
        }
    }

    fn fill_login(&mut self, origin: &str, index: usize) {
        let Some(offer) = self.pw_offer.take() else { return };
        if offer.at.elapsed() > Duration::from_secs(30) {
            return;
        }
        // The offer was made for this site, and the page that is loaded now must still be that site.
        let current = self.webview.as_ref().and_then(|wv| wv.url().ok()).and_then(|u| origin_of(&u));
        if offer.origin != origin || current.as_deref() != Some(origin) {
            return;
        }
        let Some(id) = offer.ids.get(index).copied() else { return };
        let Some(s) = &self.storage else { return };
        let (Some((_, user)), Some(pass)) = (s.password_row(id), s.password_secret(id).and_then(|b| crate::secret::decrypt(&b))) else { return };
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script(&overlays::pw_fill(&user, &pass));
        }
    }

    pub fn reload_passwords_page(&mut self) {
        if matches!(self.tabs[self.active].kind, TabKind::Page("passwords")) {
            self.load_active_page();
        }
    }

    /// `pw-<action>/<row id or origin>` from the Passwords page.
    pub fn password_command(&mut self, action: &str, arg: &str) {
        if action == "copy" && !self.hello_guard(action, arg) {
            return;
        }
        let id: i64 = arg.parse().unwrap_or(-1);
        match action {
            "copy" | "copy-user" => {
                let Some(s) = &self.storage else { return };
                let text = if action == "copy" { s.password_secret(id).and_then(|b| crate::secret::decrypt(&b)) } else { s.password_row(id).map(|(_, u)| u) };
                if let Some(t) = text {
                    crate::sys::set_clipboard_text(&t);
                    self.toast_pw(if action == "copy" { "Password copied" } else { "Username copied" });
                }
            }
            "delete" => {
                if let Some(s) = &self.storage {
                    let _ = s.delete_password(id);
                }
                self.reload_passwords_page();
            }
            "never-remove" => {
                if let Some(s) = &self.storage {
                    let _ = s.pw_never_remove(arg);
                }
                self.reload_passwords_page();
            }
            _ => {}
        }
    }
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).filter(|x| x.is_finite()).unwrap_or(0.0).clamp(-10_000.0, 100_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_taken_from_the_address() {
        assert_eq!(origin_of("https://Example.com/login?x=1").as_deref(), Some("https://example.com"));
        assert_eq!(origin_of("http://localhost:8080/a").as_deref(), Some("http://localhost:8080"));
        assert_eq!(origin_of("https://user:pw@evil.test/").as_deref(), Some("https://evil.test"));
        assert_eq!(origin_of("data:text/html,hi"), None);
        assert_eq!(origin_of("file:///c:/a.html"), None);
        assert_eq!(origin_of("axomai://passwords"), None);
        assert_eq!(origin_of("https:///nohost"), None);
    }

    #[test]
    fn different_ports_and_schemes_are_different_sites() {
        assert_ne!(origin_of("http://a.test"), origin_of("https://a.test"));
        assert_ne!(origin_of("https://a.test:444"), origin_of("https://a.test"));
    }

    #[test]
    fn page_script_never_contains_secrets_and_is_top_frame_only() {
        assert!(PAGE_SCRIPT.contains("window.top!==window"));
        assert!(!PAGE_SCRIPT.contains("TOKEN"));
    }
}
