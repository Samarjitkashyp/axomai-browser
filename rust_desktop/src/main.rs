use axomai_engine::AxomaiEngine;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Axomai Core Rust Engine Instance
    let engine = Arc::new(Mutex::new(AxomaiEngine::new()));

    // 2. Initialize Tao Event Loop for Windows
    let event_loop = EventLoop::new();

    // 3. Build Native Application Window
    let window = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(1024.0, 680.0))
        .build(&event_loop)?;

    // 4. Resolve path to Axomai Browser UI bundle
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut ui_path = current_dir.join("ui").join("index.html");

    if !ui_path.exists() {
        ui_path = current_dir.join("..").join("ui").join("index.html");
    }

    let url = format!(
        "file:///{}",
        ui_path.canonicalize()?.to_string_lossy().replace('\\', "/")
    );

    // 5. Connect UI IPC to Native Rust Engine Pipeline
    let engine_ipc = Arc::clone(&engine);
    let builder = WebViewBuilder::new()
        .with_url(&url)
        .with_devtools(true)
        .with_ipc_handler(move |msg| {
            // Handle IPC requests between UI shell and Axomai Rust Core Engine
            if let Ok(mut eng) = engine_ipc.lock() {
                if msg.starts_with("navigate:") {
                    let nav_url = &msg[9..];
                    let _ = eng.load_url(nav_url, 1380.0, 860.0);
                }
            }
        });

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    let _webview = builder.build(&window)?;

    // 6. Run Event Loop with 60 FPS Engine Tick
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        );

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Tick the Axomai Rust Engine Event Loop for async fetch, navigations, and timers
                if let Ok(mut eng) = engine.lock() {
                    eng.process_event_loop(1380.0, 860.0);
                }
            }
            _ => {}
        }
    });
}
