//! Tests for Axomai Browser v1.5.0: Smart Workspaces, VPN/DoH, PiP, Split View, Capture Studio, Limiter & Web3 Wallet.

use rust_engine::capture_studio::{AnnotationType, CaptureStudio};
use rust_engine::performance_limiter::ResourceLimiter;
use rust_engine::pip_engine::PipEngine;
use rust_engine::split_view::{SplitLayoutMode, SplitViewEngine};
use rust_engine::vpn_doh_engine::{DohProvider, ProxyProtocol, VpnDohEngine};
use rust_engine::web3_wallet::{SupportedChain, Web3Wallet};
use rust_engine::workspaces_engine::{TabHibernationState, WorkspaceManager};

#[test]
fn test_workspaces_and_tab_hibernation() {
    let mut mgr = WorkspaceManager::new();
    let tab_id1 = mgr.add_tab("Rust GitHub Repo", "https://github.com/rust-lang/rust");
    let tab_id2 = mgr.add_tab("Assam Tribune", "https://assamtribune.com");

    let tab1 = mgr.tabs.get(&tab_id1).unwrap();
    assert_eq!(tab1.workspace_name, "Coding");

    let tab2 = mgr.tabs.get(&tab_id2).unwrap();
    assert_eq!(tab2.workspace_name, "Assam & News");

    // Hibernate idle tabs
    let count = mgr.hibernate_inactive_tabs(500, 1700000010000);
    assert_eq!(count, 2);
    assert_eq!(mgr.tabs.get(&tab_id1).unwrap().state, TabHibernationState::Hibernated);

    // Restore tab
    assert!(mgr.activate_tab(tab_id1, 1700000015000).is_ok());
    assert_eq!(mgr.tabs.get(&tab_id1).unwrap().state, TabHibernationState::Active);
}

#[test]
fn test_vpn_and_doh_resolution() {
    let mut vpn = VpnDohEngine::new();
    let resolved = vpn.resolve_domain_doh("axomai.org").unwrap();
    assert!(resolved.contains("cloudflare-dns.com"));

    vpn.connect_vpn(ProxyProtocol::WireGuardTunnel { endpoint: "wg0.axomai.net:51820".to_string() });
    assert!(vpn.is_vpn_active);
    assert!(vpn.route_outbound_data(4096).is_ok());
    assert_eq!(vpn.bytes_encrypted, 4096);
}

#[test]
fn test_pip_and_audio_booster() {
    let mut pip = PipEngine::new();
    pip.enter_pip(640, 360);
    assert!(pip.config.is_active);

    pip.set_audio_boost(2.5); // 250%
    assert_eq!(pip.config.audio_boost_multiplier, 2.5);

    pip.update_subtitles("Welcome to Axomai Browser");
    assert_eq!(pip.config.current_subtitle, Some("Welcome to Axomai Browser".to_string()));
}

#[test]
fn test_split_view_modes() {
    let mut split = SplitViewEngine::new();
    assert_eq!(split.mode, SplitLayoutMode::Single);

    split.set_split_mode(SplitLayoutMode::VerticalDual, Some(5));
    assert_eq!(split.panes.len(), 2);
    assert_eq!(split.panes[1].tab_id, 5);

    assert!(split.toggle_vertical_tabs());
}

#[test]
fn test_capture_studio_annotations() {
    let mut studio = CaptureStudio::new(1920, 1080);
    let ann_id = studio.add_annotation(
        AnnotationType::Arrow { start_x: 10.0, start_y: 10.0, end_x: 100.0, end_y: 100.0 },
        "#ff0000",
        3.0,
    );
    assert_eq!(ann_id, 1);
    assert_eq!(studio.annotations.len(), 1);

    let png = studio.export_png();
    assert_eq!(png.len(), 1920 * 1080 * 4);
}

#[test]
fn test_resource_performance_limiter() {
    let mut limiter = ResourceLimiter::new();
    limiter.set_ram_limit(1024, true); // 1 GB limit
    assert!(limiter.config.is_ram_limiter_active);

    // Requesting 2GB should trigger throttle
    let should_throttle = limiter.should_throttle_allocation(2 * 1024 * 1024 * 1024);
    assert!(should_throttle);
}

#[test]
fn test_web3_native_wallet() {
    let mut wallet = Web3Wallet::new();
    assert_eq!(wallet.active_chain, SupportedChain::EthereumMainnet);

    let accounts = wallet.handle_rpc("https://uniswap.org", "eth_requestAccounts", "").unwrap();
    assert!(accounts.contains("0x71C7656EC7ab88b098defB751B7401B5f6d8976F"));

    let chain = wallet.handle_rpc("https://uniswap.org", "eth_chainId", "").unwrap();
    assert_eq!(chain, "\"0x1\"");
}
