//! Host / URL classification for the built-in AdBlock Shield and Anti-Fingerprint Privacy Guard.
//! Matching is done on the real network request (see `web.rs`), so blocked resources are never fetched.
//! The lists are deliberately limited to hosts that exist only to serve ads or telemetry, so that
//! blocking them does not break the sites people actually browse.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ad,
    Tracker,
}

/// Domains that exist to serve advertising.
const AD_HOSTS: &[&str] = &[
    "doubleclick.net", "googlesyndication.com", "googleadservices.com", "adservice.google.com",
    "pagead2.googlesyndication.com", "2mdn.net", "adsafeprotected.com", "moatads.com",
    "amazon-adsystem.com", "adnxs.com", "adsrvr.org", "advertising.com", "adform.net",
    "rubiconproject.com", "pubmatic.com", "openx.net", "casalemedia.com", "indexexchange.com",
    "criteo.com", "criteo.net", "taboola.com", "outbrain.com", "revcontent.com", "mgid.com",
    "smartadserver.com", "contextweb.com", "lijit.com", "sovrn.com", "sharethrough.com",
    "triplelift.com", "33across.com", "yieldmo.com", "teads.tv", "media.net", "bidswitch.net",
    "adition.com", "adtech.de", "adcolony.com", "applovin.com", "unityads.unity3d.com",
    "inmobi.com", "mopub.com", "smaato.net", "spotxchange.com", "springserve.com",
    "tremorhub.com", "undertone.com", "zedo.com", "popads.net", "popcash.net", "propellerads.com",
    "exoclick.com", "juicyads.com", "trafficjunky.net", "adsterra.com", "revenuehits.com",
    "clickadu.com", "hilltopads.net", "bannerflow.com", "serving-sys.com", "flashtalking.com",
    "innovid.com", "yieldlab.net", "adscale.de", "adspirit.de", "nativo.com", "colombiaonline.com",
    "adblade.com", "content.ad", "ads.yahoo.com", "advertising.yahoo.com", "gemini.yahoo.com",
    "ads.twitter.com", "static.ads-twitter.com", "ads.linkedin.com", "ads.pinterest.com",
    "ads.tiktok.com", "analytics.tiktok.com", "an.facebook.com", "adsymptotic.com", "adroll.com",
    "perfectaudience.com", "bidr.io", "turn.com", "mathtag.com", "rlcdn.com", "agkn.com",
    "everesttech.net", "demdex.net", "krxd.net", "bluekai.com", "exelator.com", "eyeota.net",
    "tapad.com", "adgrx.com", "gumgum.com", "justpremium.com", "improvedigital.com",
    "emxdgt.com", "richaudience.com", "admixer.net", "onetag-sys.com", "e-planning.net",
    "pagead.l.doubleclick.net", "securepubads.g.doubleclick.net", "tpc.googlesyndication.com",
];

/// Domains that exist to collect analytics / behavioural telemetry.
const TRACKER_HOSTS: &[&str] = &[
    "google-analytics.com", "analytics.google.com", "stats.g.doubleclick.net",
    "googletagmanager.com", "googletagservices.com", "connect.facebook.net", "pixel.facebook.com",
    "hotjar.com", "hotjar.io", "mouseflow.com", "fullstory.com", "logrocket.com", "logrocket.io",
    "smartlook.com", "crazyegg.com", "luckyorange.com", "clarity.ms", "inspectlet.com",
    "mixpanel.com", "segment.io", "segment.com", "amplitude.com", "heap.io", "heapanalytics.com",
    "kissmetrics.com", "chartbeat.com", "chartbeat.net", "parsely.com", "scorecardresearch.com",
    "quantserve.com", "quantcount.com", "newrelic.com", "nr-data.net", "bugsnag.com",
    "sentry-cdn.com", "optimizely.com", "abtasty.com", "vwo.com", "visualwebsiteoptimizer.com",
    "hs-analytics.net", "hsadspixel.net", "marketo.net", "mktoresp.com",
    "pardot.com", "eloqua.com", "omtrdc.net", "2o7.net", "adobedtm.com", "demandbase.com",
    "6sc.co", "bat.bing.com", "px.ads.linkedin.com", "snap.licdn.com", "sc-static.net",
    "tr.snapchat.com", "ct.pinterest.com", "cdn.mxpnl.com", "adjust.com",
    "appsflyer.com", "kochava.com", "singular.net", "braze.com", "onesignal.com", "clevertap.com",
    "mc.yandex.ru", "counter.yadro.ru", "top-fwz1.mail.ru", "stat.dealer.com",
    "statcounter.com", "histats.com", "webtrekk.net", "etracker.com", "matomo.cloud",
    "piwik.pro", "plausible.io", "simpleanalytics.com", "ravenjs.com", "trackjs.com",
    "datadoghq.com", "rum-http-intake.logs.datadoghq.com", "browser-intake-datadoghq.com",
    "tiqcdn.com", "ensighten.com", "bounceexchange.com", "wunderkind.co",
        "taboolasyndication.com", "pippio.com", "id5-sync.com", "liveramp.com", "permutive.com",
    "ml314.com", "dotomi.com", "bttrack.com", "owneriq.net", "w55c.net", "sitescout.com",
];

/// URL substrings that identify ad requests on third-party hosts.
const AD_PATH_PATTERNS: &[&str] = &[
    "/pagead/", "/adserver/", "/adsense/", "/doubleclick/", "/ad-manager/", "/prebid", "/gpt.js",
    "/show_ads", "/adframe", "/banner_ads", "/ads/ads.js", "/ads.js", "/adx/", "/advert.",
];

fn host_matches(host: &str, list: &[&str]) -> bool {
    list.iter().any(|d| {
        // Entries containing a path (e.g. "yandex.ru/metrika") are matched against host only by their host part.
        let d = d.split('/').next().unwrap_or(d);
        host == d || (host.len() > d.len() && host.ends_with(d) && host.as_bytes()[host.len() - d.len() - 1] == b'.')
    })
}

pub fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let authority = rest.split(&['/', '?', '#'][..]).next().unwrap_or("");
    let no_user = authority.rsplit('@').next().unwrap_or(authority);
    no_user.split(':').next().unwrap_or("").to_ascii_lowercase()
}

/// Rough registrable-domain comparison (eTLD+1) without shipping the public-suffix list.
pub fn same_site(a: &str, b: &str) -> bool {
    fn base(h: &str) -> String {
        let parts: Vec<&str> = h.split('.').collect();
        if parts.len() <= 2 {
            return h.to_string();
        }
        let n = parts.len();
        let second_level = ["co", "com", "org", "net", "gov", "edu", "ac", "nic"];
        let take = if parts[n - 1].len() == 2 && second_level.contains(&parts[n - 2]) { 3 } else { 2 };
        parts[n - take..].join(".")
    }
    !a.is_empty() && base(a) == base(b)
}

/// Classify a network request. `page_host` is the host of the top-level page.
pub fn classify(url: &str, page_host: &str) -> Option<Kind> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let host = host_of(url);
    if host.is_empty() || same_site(&host, page_host) {
        return None;
    }
    if host_matches(&host, AD_HOSTS) {
        return Some(Kind::Ad);
    }
    if host_matches(&host, TRACKER_HOSTS) {
        return Some(Kind::Tracker);
    }
    let lower = url.to_ascii_lowercase();
    if AD_PATH_PATTERNS.iter().any(|p| lower.contains(p)) {
        return Some(Kind::Ad);
    }
    None
}

/// Host list as a JS regex source, used by the page-level fallback in the privacy script.
pub fn tracker_regex_source() -> String {
    let mut parts: Vec<String> = Vec::new();
    for d in TRACKER_HOSTS.iter().chain(AD_HOSTS.iter()) {
        let host = d.split('/').next().unwrap_or(d);
        parts.push(host.replace('.', "\\\\."));
    }
    parts.join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_third_party_ad_host() {
        assert_eq!(classify("https://securepubads.g.doubleclick.net/gpt/pubads_impl.js", "example.com"), Some(Kind::Ad));
        assert_eq!(classify("https://www.google-analytics.com/analytics.js", "news.site"), Some(Kind::Tracker));
    }

    #[test]
    fn never_blocks_first_party_or_unknown() {
        assert_eq!(classify("https://doubleclick.net/x", "doubleclick.net"), None);
        assert_eq!(classify("https://static.example.com/app.js", "example.com"), None);
        assert_eq!(classify("https://www.google.com/search?q=rust", "www.google.com"), None);
    }

    #[test]
    fn suffix_match_requires_dot_boundary() {
        assert_eq!(classify("https://notdoubleclick.net/x", "example.com"), None);
        assert_eq!(classify("https://ad.doubleclick.net/x", "example.com"), Some(Kind::Ad));
    }

    #[test]
    fn same_site_handles_country_codes() {
        assert!(same_site("www.bbc.co.uk", "news.bbc.co.uk"));
        assert!(!same_site("a.co.uk", "b.co.uk"));
    }
}
