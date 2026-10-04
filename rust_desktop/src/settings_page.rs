//! The Settings page. Every control posts `set/<key>/<value>` (or a dedicated command) back to the browser,
//! which validates and stores it; nothing is applied only on the page.

use crate::extensions as ex;
use crate::i18n::{self, tr};
use crate::settings::{Settings, Startup, Tracking};
use crate::theme::THEMES;
use crate::types::{Extension, SearchEngine};
use crate::ui_shell::{self, PageCtx};
use crate::viewsource::escape;

pub struct SettingsView<'a> {
    pub settings: &'a Settings,
    pub engine: SearchEngine,
    pub extensions: &'a [Extension],
    pub theme_id: &'a str,
    /// The folder downloads really go to (the setting, or the system Downloads folder).
    pub download_dir: String,
    pub version: &'a str,
    /// Windows already uses Axomai for web links.
    pub is_default: bool,
    /// `(host, muted, blocked)` of the sites with a rule.
    pub site_rules: &'a [(String, bool, bool)],
}

fn row(label: &str, desc: &str, control: &str) -> String {
    format!(
        "<div class=\"row\"><div class=\"l\"><b>{}</b><span>{}</span></div>{}</div>",
        escape(label),
        escape(desc),
        control
    )
}

fn switch(data: &str, on: bool) -> String {
    format!("<label class=\"switch\"><input type=\"checkbox\" {} {}><span class=\"s\"></span></label>", data, if on { "checked" } else { "" })
}

fn select(key: &str, options: &[(String, String)], current: &str) -> String {
    let mut o = String::new();
    for (value, label) in options {
        o.push_str(&format!("<option value=\"{}\"{}>{}</option>", escape(value), if value == current { " selected" } else { "" }, escape(label)));
    }
    format!("<select data-set=\"{}\">{}</select>", key, o)
}

fn section(title: &str, body: &str) -> String {
    format!("<h2>{}</h2><div class=\"card\">{}</div>", escape(title), body)
}

pub fn settings_page(ctx: &PageCtx, v: &SettingsView) -> String {
    let l = ctx.lang;
    let s = v.settings;
    let t = |k: &'static str| tr(l, k);

    // ---- appearance
    let mut themes = String::new();
    for th in THEMES {
        themes.push_str(&format!(
            "<div class=\"theme{}\" onclick=\"go('theme/{}')\"><div class=\"sw\" style=\"background:{}\"></div><b>{} {}</b><span>{}</span></div>",
            if th.id == v.theme_id { " on" } else { "" },
            th.id,
            th.swatch,
            th.emoji,
            escape(th.name),
            escape(th.desc)
        ));
    }
    let languages: Vec<(String, String)> = i18n::LANGUAGES.iter().map(|(c, n)| (c.to_string(), n.to_string())).collect();
    let appearance = format!(
        "<div class=\"row col\"><div class=\"l\"><b>{}</b><span>{}</span></div></div><div class=\"themes\">{}</div>{}{}",
        escape(t("settings.theme")),
        escape(t("settings.theme.desc")),
        themes,
        row(t("settings.bookmark_bar"), t("settings.bookmark_bar.desc"), &switch("data-set=\"bookmark_bar\"", s.bookmark_bar)),
        row(t("settings.language"), t("settings.language.desc"), &select("language", &languages, &s.language)),
    );

    // ---- startup & search
    let radio = |value: &str, label: &str, on: bool| {
        format!("<label class=\"radio\"><input type=\"radio\" name=\"startup\" data-set=\"startup\" value=\"{}\" {}>{}</label>", value, if on { "checked" } else { "" }, escape(label))
    };
    let startup = format!(
        "<div class=\"row col\">{}{}{}</div>{}",
        radio("newtab", t("settings.startup.newtab"), s.startup == Startup::NewTab),
        radio("restore", t("settings.startup.restore"), s.startup == Startup::Restore),
        radio("home", t("settings.startup.home"), s.startup == Startup::HomePage),
        row(
            t("settings.home_url"),
            t("settings.home_url.desc"),
            &format!("<input type=\"text\" data-set=\"home_url\" placeholder=\"https://\" value=\"{}\">", escape(&s.home_url))
        ),
    );
    let engines: Vec<(String, String)> = SearchEngine::all().iter().map(|e| (e.name().to_string(), e.name().to_string())).collect();
    let cities: Vec<(String, String)> = crate::settings::CITIES.iter().map(|(k, n)| (k.to_string(), n.to_string())).collect();
    let search = format!(
        "{}{}",
        row(t("settings.search"), t("settings.search.desc"), &select("search_engine", &engines, v.engine.name())),
        row(t("settings.weather"), t("settings.weather.desc"), &select("weather_city", &cities, &s.weather_city)),
    );

    // ---- privacy
    let tracking_opts = vec![
        (Tracking::Basic.key().to_string(), t("settings.tracking.basic").to_string()),
        (Tracking::Balanced.key().to_string(), t("settings.tracking.balanced").to_string()),
        (Tracking::Strict.key().to_string(), t("settings.tracking.strict").to_string()),
    ];
    let clear = format!(
        "<div class=\"row col\"><div class=\"l\"><b>{}</b></div>\
         <div style=\"display:flex;gap:12px;align-items:center;flex-wrap:wrap\"><span>{}</span><select id=\"range\">\
         <option value=\"hour\">{}</option><option value=\"day\">{}</option><option value=\"week\">{}</option><option value=\"month\">{}</option><option value=\"all\" selected>{}</option></select></div>\
         <div class=\"checks\"><label><input type=\"checkbox\" id=\"c-h\" checked>{}</label><label><input type=\"checkbox\" id=\"c-d\">{}</label>\
         <label><input type=\"checkbox\" id=\"c-k\">{}</label><label><input type=\"checkbox\" id=\"c-c\">{}</label></div>\
         <div><button class=\"btn danger\" onclick=\"clearData()\">{}</button></div></div>",
        escape(t("settings.clear")),
        escape(t("settings.clear.range")),
        escape(t("settings.clear.hour")),
        escape(t("settings.clear.day")),
        escape(t("settings.clear.week")),
        escape(t("settings.clear.month")),
        escape(t("settings.clear.all")),
        escape(t("settings.clear.history")),
        escape(t("settings.clear.downloads")),
        escape(t("settings.clear.cookies")),
        escape(t("settings.clear.cache")),
        escape(t("settings.clear.button")),
    );
    let privacy = format!(
        "{}{}{}{}{}{}{}{}",
        row(t("settings.adblock"), t("settings.adblock.desc"), &switch(&format!("data-ext=\"{}\"", ex::ADBLOCK), v.extensions[ex::ADBLOCK].enabled)),
        row(t("settings.privacy_guard"), t("settings.privacy_guard.desc"), &switch(&format!("data-ext=\"{}\"", ex::PRIVACY), v.extensions[ex::PRIVACY].enabled)),
        row(t("settings.https_only"), t("settings.https_only.desc"), &switch("data-set=\"https_only\"", s.https_only)),
        row(t("settings.tracking"), t("settings.tracking.desc"), &select("tracking", &tracking_opts, s.tracking.key())),
        row(t("settings.passwords"), t("settings.passwords.desc"), &format!("<button class=\"btn ghost\" onclick=\"go('passwords')\">{}</button> {}", escape(t("settings.passwords.manage")), switch("data-set=\"password_manager\"", s.password_manager))),
        row(t("settings.gpc"), t("settings.gpc.desc"), &switch("data-set=\"gpc\"", s.gpc)),
        row(
            t("settings.permissions"),
            t("settings.permissions.desc"),
            "<button class=\"btn ghost\" onclick=\"go('permissions')\">Manage</button>"
        ),
        clear,
    );

    // ---- web pages
    let web_pages = format!(
        "{}{}{}{}",
        row(t("settings.dark_sites"), t("settings.dark_sites.desc"), &switch("data-set=\"dark_sites\"", s.dark_sites)),
        row(t("settings.cookie_banners"), t("settings.cookie_banners.desc"), &switch("data-set=\"cookie_banners\"", s.cookie_banners)),
        row(t("settings.youtube_ads"), t("settings.youtube_ads.desc"), &switch("data-set=\"youtube_ads\"", s.youtube_ads)),
        row(t("settings.print_clean"), t("settings.print_clean.desc"), &switch("data-set=\"print_clean\"", s.print_clean)),
    );

    // ---- site rules
    let mut rules = String::new();
    for (host, muted, blocked) in v.site_rules {
        let mut tags = String::new();
        if *muted {
            tags.push_str(&format!("<button class=\"btn ghost\" data-host=\"{h}\" onclick=\"go('site-unmute/'+enc(this.dataset.host))\">&#128263; {l}</button> ", h = escape(host), l = escape(t("siterules.unmute"))));
        }
        if *blocked {
            tags.push_str(&format!("<button class=\"btn ghost\" data-host=\"{h}\" onclick=\"go('site-unblock/'+enc(this.dataset.host))\">&#9940; {l}</button>", h = escape(host), l = escape(t("siterules.unblock"))));
        }
        rules.push_str(&row(host, "", &tags));
    }
    if rules.is_empty() {
        rules.push_str(&row(t("siterules.none"), "", ""));
    }
    let site_rules = format!(
        "{}{}",
        row(
            t("siterules.add"),
            t("siterules.add.desc"),
            &format!("<input type=\"text\" id=\"blockHost\" placeholder=\"example.com\" onkeydown=\"if(event.key==='Enter')go('site-block/'+enc(this.value))\"> <button class=\"btn ghost keepscroll\" onclick=\"go('site-block/'+enc(document.getElementById('blockHost').value))\">{}</button>", escape(t("siterules.block")))
        ),
        rules
    );

    // ---- search shortcuts
    let mut sc = String::new();
    let builtin: Vec<String> = crate::shortcuts::BUILTIN.iter().map(|(k, n, _)| format!("{} ({})", k, n)).collect();
    sc.push_str(&row(t("shortcuts.builtin"), &builtin.join(", "), ""));
    for (k, tpl) in &s.shortcuts {
        sc.push_str(&row(k, tpl, &format!("<button class=\"btn ghost keepscroll\" data-kw=\"{k}\" onclick=\"go('shortcut-del/'+enc(this.dataset.kw))\">{l}</button>", k = escape(k), l = escape(t("shortcuts.remove")))));
    }
    sc.push_str(&row(
        t("shortcuts.add"),
        t("shortcuts.add.desc"),
        &format!("<input type=\"text\" id=\"scKw\" placeholder=\"@docs\" style=\"width:90px\"> <input type=\"text\" id=\"scTpl\" placeholder=\"https://example.com/?q=%s\" style=\"width:230px\"> <button class=\"btn ghost keepscroll\" onclick=\"go('shortcut-add/'+enc(document.getElementById('scKw').value)+'/'+enc(document.getElementById('scTpl').value))\">{}</button>", escape(t("shortcuts.save"))),
    ));

    // ---- downloads & performance
    let downloads = format!(
        "{}{}",
        row(
            t("settings.download_dir"),
            &v.download_dir,
            &format!("<button class=\"btn ghost\" onclick=\"go('pick-download-dir')\">{}</button>", escape(t("settings.download_dir.change")))
        ),
        row(t("settings.ask_download"), "", &switch("data-set=\"ask_download\"", s.ask_download)),
    );
    let sleep_opts: Vec<(String, String)> = [0u32, 1, 5, 15, 30, 60]
        .iter()
        .map(|m| (m.to_string(), if *m == 0 { t("settings.sleep.never").to_string() } else { format!("{} {}", m, t("settings.sleep.minutes")) }))
        .collect();
    let performance = format!(
        "{}{}",
        row(t("settings.sleep"), t("settings.sleep.desc"), &select("sleep_minutes", &sleep_opts, &s.sleep_minutes.to_string())),
        row(t("settings.booster"), t("settings.booster.desc"), &switch(&format!("data-ext=\"{}\"", ex::BOOSTER), v.extensions[ex::BOOSTER].enabled)),
    );

    // ---- system
    let system = format!(
        "{}{}",
        row(
            t("settings.default_browser"),
            &if v.is_default { format!("{} \u{2014} {}", t("settings.default_browser.desc"), t("settings.default_browser.yes")) } else { t("settings.default_browser.desc").to_string() },
            &format!("<button class=\"btn ghost\" onclick=\"go('default-browser')\">{}</button>", escape(t("settings.default_browser.button")))
        ),
        row(
            t("settings.reset"),
            t("settings.reset.desc"),
            &format!("<button class=\"btn ghost\" onclick=\"if(confirm('Reset all settings?'))go('settings-reset')\">{}</button>", escape(t("settings.reset.button")))
        ),
    );

    let body = format!(
        "{head}{a}{b}{c}{w}{sr}{sh}{d}{e}{f}{g}<p class=\"sub\" style=\"margin-top:28px\">Axomai Browser {ver}</p>",
        head = ui_shell::heading(t("settings.title"), ""),
        a = section(t("settings.appearance"), &appearance),
        b = section(t("settings.startup"), &format!("{}{}", startup, search)),
        c = section(t("settings.privacy"), &privacy),
        w = section(t("settings.web_pages"), &web_pages),
        sr = section(t("settings.siterules"), &site_rules),
        sh = section(t("settings.shortcuts"), &sc),
        d = section(t("settings.downloads"), &downloads),
        e = section(t("settings.performance"), &performance),
        f = section(t("settings.browser"), &system),
        g = "",
        ver = escape(v.version),
    );

    let script = r#"
var km=/^axs:(\d+)$/.exec(window.name||'');if(km){window.name='';window.scrollTo(0,+km[1])}
document.addEventListener('click',function(e){if(e.target.closest&&e.target.closest('[data-host],.keepscroll')){window.name='axs:'+Math.round(window.scrollY)}},true);
document.querySelectorAll('[data-set]').forEach(function(el){
  el.addEventListener('change',function(){
    if(el.type==='radio'&&!el.checked)return;
    var v=el.type==='checkbox'?(el.checked?'1':'0'):el.value;
    go('set/'+el.dataset.set+'/'+enc(v));
  });
});
document.querySelectorAll('[data-ext]').forEach(function(el){el.addEventListener('change',function(){go('ext-toggle/'+el.dataset.ext)})});
function clearData(){
  var f=(document.getElementById('c-h').checked?'h':'')+(document.getElementById('c-d').checked?'d':'')+(document.getElementById('c-k').checked?'k':'')+(document.getElementById('c-c').checked?'c':'');
  if(!f){toast('Choose what to clear');return}
  if(!confirm('Clear the selected browsing data?'))return;
  go('clear-data-range/'+document.getElementById('range').value+'/'+f);
}
window.__saved=function(ok,msg){toast(ok?'Saved':(msg||'Not saved'))};
"#;
    ui_shell::page(ctx, "settings", t("settings.title"), &body, script)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(s: &Settings) -> String {
        let exts = crate::extensions::create_extensions();
        let ctx = PageCtx { theme: crate::theme::by_id("tea-garden"), token: "tok", lang: "en" };
        settings_page(
            &ctx,
            &SettingsView { settings: s, engine: SearchEngine::Bing, extensions: &exts, theme_id: "kaziranga", download_dir: "C:\\Users\\x\\Downloads".into(), version: "1.0", is_default: false, site_rules: &[("a.com".to_string(), true, true)] },
        )
    }

    #[test]
    fn page_reflects_current_values() {
        let mut s = Settings::default();
        s.startup = Startup::Restore;
        s.home_url = "https://example.com".into();
        s.sleep_minutes = 15;
        let html = render(&s);
        assert!(html.contains("<title>Settings - Axomai Browser</title>"));
        assert!(html.contains("value=\"restore\" checked"));
        assert!(html.contains("value=\"https://example.com\""));
        assert!(html.contains("<option value=\"15\" selected>"));
        assert!(html.contains("<option value=\"Bing\" selected>"));
        assert!(html.contains("theme on\" onclick=\"go('theme/kaziranga')"));
        assert!(html.contains("C:\\Users\\x\\Downloads"));
    }

    #[test]
    fn home_url_value_is_escaped() {
        let mut s = Settings::default();
        s.home_url = "https://a.b/?x=\"><script>1</script>".into();
        // set() would reject this, but the page must stay safe even if bad data were stored
        let html = render(&s);
        assert!(!html.contains("\"><script>1</script>"));
    }

    #[test]
    fn every_setting_key_the_page_posts_is_accepted_by_settings() {
        let html = render(&Settings::default());
        let mut keys: Vec<String> = Vec::new();
        for part in html.split("data-set=\"").skip(1) {
            keys.push(part.split('"').next().unwrap_or("").to_string());
        }
        keys.sort();
        keys.dedup();
        let mut s = Settings::default();
        for k in keys {
            if k == "search_engine" {
                continue; // handled by its own command, validated in app_commands
            }
            let sample = match k.as_str() {
                "startup" => "restore",
                "home_url" => "https://example.com",
                "sleep_minutes" => "5",
                "tracking" => "strict",
                "language" => "en",
                "weather_city" => "guwahati",
                "ai_model" => "claude-sonnet-5-5",
                "ai_provider" => "openai",
                "download_dir" => "C:\\x",
                _ => "1",
            };
            assert!(s.set(None, &k, sample), "settings rejected the key the page posts: {}", k);
        }
    }
}
