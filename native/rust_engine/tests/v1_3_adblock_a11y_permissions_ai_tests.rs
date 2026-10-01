//! Tests for Axomai Browser v1.3.0: AdBlocker, Accessibility Tree, Permissions, AI Assistant, and Android FFI.

use axomai_engine::accessibility_engine::{AccessibilityTree, AriaRole};
use axomai_engine::adblock_engine::AdBlockEngine;
use axomai_engine::ai_assistant::{AiAssistant, SupportedLanguage};
use axomai_engine::html_parser::HTMLParser;
use axomai_engine::permissions_engine::{PermissionName, PermissionState, PermissionsManager};

#[test]
fn test_adblock_network_filtering_and_cosmetic_css() {
    let mut adblock = AdBlockEngine::new();
    assert!(adblock.is_enabled);

    // Network blocking
    assert!(adblock.should_block_url("https://securepubads.g.doubleclick.net/gampad/ads"));
    assert!(adblock.should_block_url("https://www.google-analytics.com/analytics.js"));
    assert!(!adblock.should_block_url("https://en.wikipedia.org/wiki/Assam"));

    assert_eq!(adblock.stats.blocked_requests_count, 2);
    assert_eq!(adblock.stats.total_inspected_requests, 3);

    // Cosmetic stylesheet generation
    let css = adblock.generate_cosmetic_css();
    assert!(css.contains("display: none !important"));
    assert!(css.contains(".ad-banner"));
}

#[test]
fn test_accessibility_tree_generation() {
    let html = r#"<html><body><header role="banner"><h1>Axomai</h1></header><nav><a href="/">Home</a></nav><main><button aria-label="Submit Form">Send</button><input type="checkbox" checked /></main></body></html>"#;
    let dom = HTMLParser::new(html).parse();

    let mut a11y_tree = AccessibilityTree::new();
    a11y_tree.build(&dom);

    assert!(a11y_tree.root.is_some());
    let root = a11y_tree.root.unwrap();
    assert!(!root.children.is_empty());
}

#[test]
fn test_permissions_and_geolocation() {
    let mut mgr = PermissionsManager::new();
    assert_eq!(mgr.query(PermissionName::Geolocation), PermissionState::Prompt);
    assert_eq!(mgr.query(PermissionName::ClipboardWrite), PermissionState::Granted);

    // Denied request
    assert!(mgr.get_current_position().is_err());

    // Granted request
    mgr.request_permission(PermissionName::Geolocation, true);
    assert_eq!(mgr.query(PermissionName::Geolocation), PermissionState::Granted);

    let pos = mgr.get_current_position().unwrap();
    assert_eq!(pos.coords.latitude, 26.1445);
    assert_eq!(pos.coords.longitude, 91.7362);
}

#[test]
fn test_ai_assistant_summarizer_and_translation() {
    let ai = AiAssistant::new();

    // Summarizer test
    let article = "Axomai Browser is an ultra-fast web browser built with pure Rust. It includes independent HTML5, CSS Grid, and WebAssembly rendering pipelines. Users can browse safely with native on-device privacy protection.";
    let summary = ai.summarize(article, 2);
    assert!(summary.contains("Axomai Browser is an ultra-fast web browser"));

    // Keyword extractor test
    let keywords = ai.extract_keywords(article);
    assert!(!keywords.is_empty());

    // Multilingual translation test (English -> Assamese)
    let translated = ai.translator.translate("hello browser", SupportedLanguage::English, SupportedLanguage::Assamese);
    assert!(translated.contains("নমস্কাৰ"));
    assert!(translated.contains("ব্ৰাউজাৰ"));
}
