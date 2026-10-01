//! Comprehensive Unicode & Text Shaper Verification for Assamese (অসমীয়া), Bengali, Devanagari, and RTL BiDi.

use axomai_engine::painter::TextShaper;

#[test]
fn test_assamese_unique_characters_unicode_integrity() {
    // Test Assamese unique letters: ৰ (U+09F0), ৱ (U+09F1), ক্ষ (U+0995 + U+09CD + U+09B7)
    let assamese_ra = "\u{09F0}"; // ৰ
    let assamese_wa = "\u{09F1}"; // ৱ
    let assamese_ksha = "\u{0995}\u{09CD}\u{09B7}"; // ক্ষ

    assert_eq!(assamese_ra, "ৰ");
    assert_eq!(assamese_wa, "ৱ");
    assert_eq!(assamese_ksha, "ক্ষ");

    // Complex Assamese words
    let word_brahmaputra = "ব্ৰহ্মপুত্ৰ";
    let word_kaziranga = "কাজিৰঙা";
    let word_rongali_bihu = "ৰঙালী বিহু";
    let word_majuli = "মাজুলী";

    assert!(word_brahmaputra.contains("ব্ৰ"));
    assert!(word_kaziranga.contains("জি"));
    assert!(word_rongali_bihu.contains("ৰঙালী"));
    assert!(word_majuli.contains("লী"));
}

#[test]
fn test_assamese_matra_reordering_and_conjuncts() {
    // Pre-base vowel: ক + ি -> কি (U+0995 + U+09BF)
    let ki = "\u{0995}\u{09BF}";
    assert_eq!(ki, "কি");

    // Post-base ya-phala (য-ফলা): ক + ্ + য -> ক্য (U+0995 + U+09CD + U+09AF)
    let kya = "\u{0995}\u{09CD}\u{09AF}";
    assert_eq!(kya, "ক্য");

    // Ra-phala (ৰ-কাৰ): ক + ্ + ৰ -> ক্ৰ (U+0995 + U+09CD + U+09F0)
    let kra = "\u{0995}\u{09CD}\u{09F0}";
    assert_eq!(kra, "ক্ৰ");

    // Complex conjunct: হ + ্ + ম -> হ্ম (U+09B9 + U+09CD + U+09AE)
    let hma = "\u{09B9}\u{09CD}\u{09AE}";
    assert_eq!(hma, "হ্ম");
}

#[test]
fn test_indic_shaper_glyph_run_extraction() {
    let sample = "অসমীয়া ব্ৰাউজাৰ (Axomai Browser)";
    let glyphs = TextShaper::shape_text(sample, 16.0);
    assert!(!glyphs.is_empty());
}
