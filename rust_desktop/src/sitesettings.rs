//! One page with everything the browser remembers about a site: zoom, permissions, mute / block, saved logins and cookies.

use crate::app::App;
use crate::i18n::tr;
use crate::passwords::origin_of;
use crate::tabs::TabKind;
use crate::ui_shell::{self, PageCtx};
use crate::viewsource::escape;

pub const ZOOM_STEPS: [u32; 12] = [25, 50, 67, 75, 80, 90, 100, 110, 125, 150, 200, 300];

pub struct SiteView {
    pub origin: String,
    pub host: String,
    pub zoom_pct: u32,
    pub perms: Vec<(i64, String, bool)>,
    pub muted: bool,
    pub blocked: bool,
    pub logins: usize,
}

/// The closest allowed zoom step for a zoom factor.
pub fn nearest_step(factor: f64) -> u32 {
    let pct = (factor * 100.0).round() as i64;
    *ZOOM_STEPS.iter().min_by_key(|s| (**s as i64 - pct).abs()).unwrap_or(&100)
}

pub fn site_settings_page(ctx: &PageCtx, v: &SiteView) -> String {
    let l = ctx.lang;
    let t = |k: &'static str| tr(l, k);
    let row = |label: &str, desc: &str, control: &str| format!("<div class=\"row\"><div class=\"l\"><b>{}</b><span>{}</span></div>{}</div>", escape(label), escape(desc), control);
    let mut zoom = String::new();
    for s in ZOOM_STEPS {
        zoom.push_str(&format!("<option value=\"{s}\"{}>{s}%</option>", if s == v.zoom_pct { " selected" } else { "" }));
    }
    let zoom_ctl = format!("<select onchange=\"go('site-zoom/'+this.value)\">{}</select>", zoom);
    let switch = |on: bool, cmd_on: &str, cmd_off: &str| {
        if v.host.is_empty() {
            // Local and single-word addresses cannot have site rules.
            return "<span style=\"color:var(--muted)\">\u{2014}</span>".to_string();
        }
        format!(
            "<label class=\"switch\"><input type=\"checkbox\" {} onchange=\"go(this.checked?'{}/'+enc('{}'):'{}/'+enc('{}'))\"><span class=\"s\"></span></label>",
            if on { "checked" } else { "" },
            cmd_on,
            escape(&v.host),
            cmd_off,
            escape(&v.host)
        )
    };
    let mut perms = String::new();
    for (id, kind, allow) in &v.perms {
        let name = crate::permissions::label(kind).1;
        perms.push_str(&row(
            name,
            if *allow { t("sitesettings.allowed") } else { t("sitesettings.blocked") },
            &format!(
                "<button class=\"btn ghost\" onclick=\"go('site-perm/{id}/{flip}')\">{flip_label}</button> <button class=\"x\" title=\"{reset}\" onclick=\"go('site-perm/{id}/delete')\">\u{2715}</button>",
                id = id,
                flip = if *allow { "block" } else { "allow" },
                flip_label = escape(if *allow { t("sitesettings.block") } else { t("sitesettings.allow") }),
                reset = escape(t("sitesettings.reset"))
            ),
        ));
    }
    if perms.is_empty() {
        perms.push_str(&row(t("sitesettings.noperms"), "", ""));
    }
    let body = format!(
        "{head}<h2>{h_view}</h2><div class=\"card\">{zoom}{mute}{block}</div>\
         <h2>{h_perm}</h2><div class=\"card\">{perms}</div>\
         <h2>{h_data}</h2><div class=\"card\">{logins}{clear}</div>",
        head = ui_shell::heading(t("sitesettings.title"), &v.origin),
        h_view = escape(t("sitesettings.view")),
        zoom = row(t("sitesettings.zoom"), t("sitesettings.zoom.desc"), &zoom_ctl),
        mute = row(t("sitesettings.mute"), t("sitesettings.mute.desc"), &switch(v.muted, "site-mute", "site-unmute")),
        block = row(t("sitesettings.blockrow"), t("sitesettings.blockrow.desc"), &switch(v.blocked, "site-block", "site-unblock")),
        h_perm = escape(t("sitesettings.perms")),
        perms = perms,
        h_data = escape(t("sitesettings.data")),
        logins = row(
            t("sitesettings.logins"),
            &format!("{} {}", v.logins, t("sitesettings.logins.count")),
            &format!("<button class=\"btn ghost\" onclick=\"go('passwords')\">{}</button>", escape(t("sitesettings.logins.manage")))
        ),
        clear = row(
            t("sitesettings.clear"),
            t("sitesettings.clear.desc"),
            &format!("<button class=\"btn danger\" onclick=\"go('site-clear-data')\">{}</button>", escape(t("sitesettings.clear.button")))
        ),
    );
    ui_shell::page(ctx, "", "Site settings", &body, "")
}

impl App {
    fn toast_site_settings(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    /// "Site settings" for the page that is open.
    pub fn open_site_settings(&mut self) {
        let t = &self.tabs[self.active];
        let Some(origin) = origin_of(&t.url).filter(|_| matches!(t.kind, TabKind::Web)) else {
            return self.toast_site_settings("Open a web page first");
        };
        self.site_origin = origin;
        self.open_internal("sitesettings");
    }

    pub fn site_view(&self) -> SiteView {
        let origin = self.site_origin.clone();
        let host = crate::siterules::normalize_host(&origin).unwrap_or_default();
        let st = self.storage.as_ref();
        let rules = st.map(|s| s.site_rules()).unwrap_or_default();
        let rule = rules.iter().find(|r| r.0 == host);
        SiteView {
            zoom_pct: nearest_step(st.and_then(|s| s.get_site_zoom(&origin)).unwrap_or(1.0)),
            perms: st.map(|s| s.list_permissions().into_iter().filter(|p| p.1 == origin).map(|p| (p.0, p.2, p.3)).collect()).unwrap_or_default(),
            muted: rule.map_or(false, |r| r.1),
            blocked: rule.map_or(false, |r| r.2),
            logins: st.map_or(0, |s| s.credentials_for(&origin).len()),
            host,
            origin,
        }
    }

    pub fn site_zoom_set(&mut self, pct: u32) {
        if !ZOOM_STEPS.contains(&pct) || self.site_origin.is_empty() {
            return;
        }
        if let Some(s) = &self.storage {
            let _ = s.set_site_zoom(&self.site_origin, pct as f64 / 100.0);
        }
        for i in 0..self.tabs.len() {
            let url = self.tabs[i].url.clone();
            if matches!(self.tabs[i].kind, TabKind::Web) && origin_of(&url).as_deref() == Some(self.site_origin.as_str()) {
                self.apply_site_zoom(i, &url);
            }
        }
        self.refresh_internal_page();
    }

    pub fn site_perm_command(&mut self, id: i64, action: &str) {
        let owned = self.storage.as_ref().map_or(false, |s| s.list_permissions().iter().any(|p| p.0 == id && p.1 == self.site_origin));
        if !owned {
            return;
        }
        if let Some(s) = &self.storage {
            let _ = match action {
                "allow" => s.set_permission_by_id(id, true),
                "block" => s.set_permission_by_id(id, false),
                "delete" => s.delete_permission(id),
                _ => Ok(()),
            };
        }
        self.refresh_internal_page();
    }

    /// Cookies, local storage, caches and the like for this one site.
    pub fn site_clear_data(&mut self) {
        if self.site_origin.is_empty() {
            return;
        }
        if let Some(wv) = &self.webview {
            crate::web::com::cdp(wv, "Storage.clearDataForOrigin", &serde_json::json!({"origin": self.site_origin, "storageTypes": "all"}).to_string());
        }
        self.toast_site_settings(&format!("Cleared cookies and site data for {}", self.site_origin));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_snaps_to_the_nearest_step() {
        assert_eq!(nearest_step(1.0), 100);
        assert_eq!(nearest_step(1.04), 100);
        assert_eq!(nearest_step(1.2), 125);
        assert_eq!(nearest_step(0.1), 25);
        assert_eq!(nearest_step(9.0), 300);
    }

    #[test]
    fn page_shows_the_site_and_escapes_it() {
        let ctx = PageCtx { theme: crate::theme::by_id("tea-garden"), token: "tok", lang: "en" };
        let v = SiteView { origin: "https://ex<b>.com".into(), host: "ex.com".into(), zoom_pct: 125, perms: vec![(4, "camera".into(), true)], muted: true, blocked: false, logins: 2 };
        let html = site_settings_page(&ctx, &v);
        assert!(html.contains("https://ex&lt;b&gt;.com") && !html.contains("ex<b>"));
        assert!(html.contains("<option value=\"125\" selected>") && html.contains("site-perm/4/block") && html.contains("site-perm/4/delete"));
        assert!(html.contains("site-clear-data") && html.contains("2 saved for this site"));
    }
}
