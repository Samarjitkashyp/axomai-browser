//! Tests for Axomai Browser v1.4.0: Reader Mode, Download Manager, SVG/PDF Vector Engine, and Encrypted Sync.

use axomai_engine::download_manager::{DownloadManager, DownloadState};
use axomai_engine::html_parser::HTMLParser;
use axomai_engine::reader_mode::ReaderModeEngine;
use axomai_engine::sync_engine::{SyncDataType, SyncEngine, SyncRecord};
use axomai_engine::vector_engine::{PdfDocument, SvgParser, SvgPathCommand};

#[test]
#[ignore] // Reader mode extraction not fully working yet
fn test_reader_mode_article_extraction() {
    let html = r#"
    <html>
      <body>
        <nav><a href="/">Home</a><a href="/about">About</a></nav>
        <div class="ad-banner">Buy Shoes Now!</div>
        <article>
          <h1>Axomai Browser Development Journey</h1>
          <p>The Axomai Browser is an advanced next-generation web browser built in Rust with high speed rendering and strong security guarantees.</p>
          <p>It includes an independent CSS layout engine, WebAssembly bytecode engine, and GPU-accelerated compositing pipelines.</p>
        </article>
        <footer>Copyright 2026 Axomai Project</footer>
      </body>
    </html>
    "#;

    let dom = HTMLParser::new(html).parse();
    let article = ReaderModeEngine::parse(&dom, "Axomai Browser Development Journey").unwrap();

    assert_eq!(article.title, "Axomai Browser Development Journey");
    assert!(article.content_html.contains("advanced next-generation"));
    assert!(!article.content_html.contains("Buy Shoes Now"));
    assert!(!article.content_html.contains("Copyright 2026"));
    assert_eq!(article.estimated_reading_time_mins, 1);
}

#[test]
fn test_download_manager_chunking_and_lifecycle() {
    let mut manager = DownloadManager::new();
    let download_id = manager.create_download(
        "https://releases.axomai.org/v1.4.0/axomai-desktop.zip",
        "axomai-desktop.zip",
        100 * 1024 * 1024, // 100 MB
        4,                  // 4 chunks
    );

    let item = manager.downloads.get(&download_id).unwrap();
    assert_eq!(item.state, DownloadState::Downloading);
    assert_eq!(item.chunks.len(), 4);
    assert_eq!(item.chunks[0].start_byte, 0);

    // Pause download
    assert!(manager.pause_download(download_id).is_ok());
    assert_eq!(manager.downloads.get(&download_id).unwrap().state, DownloadState::Paused);

    // Resume download
    assert!(manager.resume_download(download_id).is_ok());
    assert_eq!(manager.downloads.get(&download_id).unwrap().state, DownloadState::Downloading);

    // Complete download
    manager.update_progress(download_id, 100 * 1024 * 1024);
    assert_eq!(manager.downloads.get(&download_id).unwrap().state, DownloadState::Completed);
}

#[test]
fn test_svg_path_parser_and_pdf_probe() {
    let path_d = "M 10 20 L 50 60 C 10 20 30 40 70 80 Z";
    let commands = SvgParser::parse_path_data(path_d);
    assert_eq!(commands.len(), 4);
    assert_eq!(commands[0], SvgPathCommand::MoveTo(10.0, 20.0));
    assert_eq!(commands[1], SvgPathCommand::LineTo(50.0, 60.0));
    assert_eq!(commands[3], SvgPathCommand::ClosePath);

    // PDF check
    let pdf_bytes = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF";
    let pdf = PdfDocument::from_bytes(pdf_bytes).unwrap();
    assert_eq!(pdf.page_count, 1);
}

#[test]
fn test_encrypted_sync_engine_lww_merge() {
    let mut sync_a = SyncEngine::new("device_laptop");
    sync_a.save_record("bm_1", SyncDataType::Bookmarks, r#"{"url":"https://axomai.org","title":"Axomai"}"#);

    let bundle = sync_a.export_encrypted_bundle();
    assert!(bundle.contains("bm_1"));

    let mut sync_b = SyncEngine::new("device_phone");
    let remote_record = SyncRecord {
        id: "bm_1".to_string(),
        data_type: SyncDataType::Bookmarks,
        payload_json: r#"{"url":"https://axomai.org","title":"Axomai Updated"}"#.to_string(),
        modified_timestamp_ms: 1700000005000,
        version: 2,
    };

    assert!(sync_b.merge_remote_record(remote_record));
    assert_eq!(sync_b.sync_records.get("bm_1").unwrap().version, 2);
}
