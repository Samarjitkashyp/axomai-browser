use std::path::PathBuf;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Tao Event Loop for Windows
    let event_loop = EventLoop::new();

    // 2. Build Native Application Window
    let window = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(1024.0, 680.0))
        .build(&event_loop)?;

    // 3. Resolve path to Axomai Browser UI bundle
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut ui_path = current_dir.join("ui").join("index.html");

    // Check if running from rust_desktop subfolder
    if !ui_path.exists() {
        ui_path = current_dir.join("..").join("ui").join("index.html");
    }

    let url = format!("file:///{}", ui_path.canonicalize()?.to_string_lossy().replace('\\', "/"));

    // 4. Attach High-Performance Wry WebView (Edge Chromium WebView2)
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    let _webview = WebViewBuilder::new()
        .with_url(&url)
        .with_devtools(true)
        .build(&window)?;

    // 5. Run Event Loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
