use rust_engine::engine::{AxomaiEngine, CdpInspector, IpcBus, IpcMessage, ProcessKind, SandboxPolicy};
use rust_engine::html_parser::{find_element_by_id, HTMLParser};

#[test]
fn test_sandbox_policy_and_ipc_bus() {
    let policy = SandboxPolicy {
        disallow_disk_io: true,
        disallow_raw_sockets: true,
        isolated_origin: Some("https://axomai.org".to_string()),
        max_memory_mb: 256,
    };
    assert!(policy.disallow_disk_io);
    assert_eq!(policy.max_memory_mb, 256);

    let mut ipc = IpcBus::new(ProcessKind::RendererSandbox, policy);
    ipc.send(IpcMessage::Navigate { url: "https://axomai.org/home".to_string() });
    ipc.send(IpcMessage::RenderFrame { width: 1920.0, height: 1080.0 });

    let msgs = ipc.drain_messages();
    assert_eq!(msgs.len(), 2, "IPC Bus should drain 2 queued messages");
}

#[test]
fn test_cdp_inspector_protocol() {
    let mut engine = AxomaiEngine::new();
    engine.load_html("<html><head><title>CDP Test Page</title></head><body><div id='target-div'>Inspect Me</div></body></html>", "http://localhost/");

    let doc_res = CdpInspector::handle_command(&mut engine, 1, "DOM.getDocument", "{}");
    assert!(doc_res.contains(r#""id":1"#));
    assert!(doc_res.contains("CDP Test Page"));

    let query_res = CdpInspector::handle_command(&mut engine, 2, "DOM.querySelector", r#"{"selector":"#target-div"}"#);
    assert!(query_res.contains(r#""id":2"#));

    let eval_res = CdpInspector::handle_command(&mut engine, 3, "Runtime.evaluate", r#"{"expression":"1 + 1"}"#);
    assert!(eval_res.contains(r#""id":3"#));
}

#[test]
fn test_media_elements_dom_parsing() {
    let html = r#"
        <html>
            <body>
                <video id="player" src="movie.mp4" width="640" height="360" controls autoplay></video>
                <audio id="sound" src="track.mp3" controls></audio>
            </body>
        </html>
    "#;

    let mut parser = HTMLParser::new();
    let root = parser.parse(html);

    let vid = find_element_by_id(&root, "player");
    assert!(vid.is_some(), "Video element should be parsed");

    let aud = find_element_by_id(&root, "sound");
    assert!(aud.is_some(), "Audio element should be parsed");
}
