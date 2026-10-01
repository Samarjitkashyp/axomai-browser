use axomai_engine::network::{base64_decode, base64_encode, CacheControl, CspPolicy, URL};

#[test]
fn test_cache_control_parsing() {
    let header = "public, max-age=86400, must-revalidate";
    let cc = CacheControl::parse(header);
    assert_eq!(cc.max_age, Some(86400));
    assert!(cc.must_revalidate);
    assert!(!cc.no_store);
    assert!(!cc.no_cache);

    let no_store_header = "no-store, no-cache";
    let cc2 = CacheControl::parse(no_store_header);
    assert!(cc2.no_store);
    assert!(cc2.no_cache);
}

#[test]
fn test_csp_policy_validation() {
    let csp_str = "default-src 'self'; script-src 'self' https://trusted.cdn.com; img-src *";
    let policy = CspPolicy::parse(csp_str);

    let current_origin = "https://axomai.org";

    // Same origin script allowed
    assert!(policy.is_allowed("script-src", "https://axomai.org/app.js", current_origin));
    // Whitelisted CDN script allowed
    assert!(policy.is_allowed("script-src", "https://trusted.cdn.com/lib.js", current_origin));
    // Untrusted script blocked
    assert!(!policy.is_allowed("script-src", "https://evil.com/malware.js", current_origin));

    // Image wildcard allows everything
    assert!(policy.is_allowed("img-src", "https://images.unsplash.com/photo.jpg", current_origin));
    assert!(policy.is_allowed("img-src", "https://evil.com/tracker.png", current_origin));

    // Connect-src falls back to default-src ('self')
    assert!(policy.is_allowed("connect-src", "https://axomai.org/api/data", current_origin));
    assert!(!policy.is_allowed("connect-src", "https://api.external.com/v1", current_origin));
}

#[test]
fn test_base64_encoding_decoding() {
    let original = b"Axomai Browser - Fast, Private, AI-Powered";
    let encoded = base64_encode(original);
    let decoded = base64_decode(&encoded).expect("Decoded bytes should match");
    assert_eq!(decoded, original);
}

#[test]
fn test_url_origin_and_resolution() {
    let base = URL::parse("https://axomai.org:8080/path/to/page.html?view=grid#section1").unwrap();
    assert_eq!(base.origin(), "https://axomai.org:8080");
    assert_eq!(base.scheme, "https");
    assert_eq!(base.host, "axomai.org");
    assert_eq!(base.port, 8080);
    assert_eq!(base.path, "/path/to/page.html");

    let resolved = base.resolve("../images/logo.png");
    assert_eq!(resolved, "https://axomai.org:8080/path/images/logo.png");

    let root_resolved = base.resolve("/assets/app.css");
    assert_eq!(root_resolved, "https://axomai.org:8080/assets/app.css");
}
