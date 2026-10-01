//! Tests for Axomai Browser v1.6.0: Final Production Edition (Auto-Updater & Heritage Theme Engine).

use axomai_engine::theme_engine::{HeritagePreset, ThemeEngine};
use axomai_engine::updater_engine::{ReleaseManifest, UpdateChannel, UpdateStatus, UpdaterEngine};

#[test]
fn test_auto_updater_lifecycle_and_verification() {
    let mut updater = UpdaterEngine::new("1.5.0", UpdateChannel::Stable);
    assert_eq!(updater.status, UpdateStatus::UpToDate);

    let manifest = ReleaseManifest {
        version: "1.6.0".to_string(),
        channel: UpdateChannel::Stable,
        release_notes: "Axomai Browser v1.6.0 Final Production Release".to_string(),
        download_url: "https://releases.axomai.org/v1.6.0/axomai.zip".to_string(),
        sha256_checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        signature_hex: "d85f8e02d8f993f4123456789abcdef0123456789abcdef0123456789abcdef0".to_string(),
        package_size_bytes: 45 * 1024 * 1024,
    };

    let status = updater.check_for_updates(&manifest);
    assert_eq!(status, UpdateStatus::UpdateAvailable("1.6.0".to_string()));

    assert!(updater.verify_integrity(&[], &manifest));
    assert!(updater.apply_update().is_ok());
    assert_eq!(updater.status, UpdateStatus::ReadyToInstall);
}

#[test]
fn test_heritage_theme_engine_css_variables() {
    let mut theme = ThemeEngine::new();
    theme.apply_preset(HeritagePreset::KazirangaGreen);
    assert_eq!(theme.colors.primary_accent, "#10b981");

    let css = theme.generate_theme_css();
    assert!(css.contains("--ax-primary: #10b981"));
    assert!(css.contains("--ax-backdrop: blur(16px)"));

    theme.apply_preset(HeritagePreset::BihuGold);
    assert_eq!(theme.colors.primary_accent, "#f59e0b");
    let bihu_css = theme.generate_theme_css();
    assert!(bihu_css.contains("--ax-primary: #f59e0b"));
}
