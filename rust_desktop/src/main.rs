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
                } else if msg.starts_with("pointer:") {
                    let parts: Vec<&str> = msg[8..].split(',').collect();
                    if parts.len() == 4 {
                        let action = parts[0];
                        let btn = parts[1].parse::<i32>().unwrap_or(0);
                        if let (Ok(x), Ok(y)) = (parts[2].parse::<f32>(), parts[3].parse::<f32>()) {
                            match action {
                                "down" => {
                                    if let Some(nav_url) = eng.handle_pointer_down(x, y, btn) {
                                        println!("[Axomai Desktop] Pointer down navigating: {}", nav_url);
                                        let _ = eng.load_url(&nav_url, 1380.0, 860.0);
                                    }
                                }
                                "move" => {
                                    let _ = eng.handle_pointer_move(x, y);
                                }
                                "up" => {
                                    let _ = eng.handle_pointer_up(x, y, btn);
                                }
                                _ => {}
                            }
                        }
                    }
                } else if msg.starts_with("click:") {
                    let coords = &msg[6..];
                    if let Some(comma) = coords.find(',') {
                        if let (Ok(x), Ok(y)) = (coords[..comma].parse::<f32>(), coords[comma + 1..].parse::<f32>()) {
                            if let Some(nav_url) = eng.handle_click(x, y, 0.0) {
                                println!("[Axomai Desktop] Navigating to clicked link: {}", nav_url);
                                let _ = eng.load_url(&nav_url, 1380.0, 860.0);
                            }
                        }
                    }
                } else if msg.starts_with("key:") {
                    let payload = &msg[4..];
                    let parts: Vec<&str> = payload.split(',').collect();
                    if parts.len() >= 9 {
                        let ev_type = parts[0];
                        let key_str = parts[1];
                        let code_str = parts[2];
                        let key_code = parts[3].parse::<u32>().unwrap_or(0);
                        let ctrl = parts[4] == "1";
                        let alt = parts[5] == "1";
                        let shift = parts[6] == "1";
                        let meta = parts[7] == "1";
                        let repeat = parts[8] == "1";

                        if let Some(nav_url) = eng.handle_key_event(
                            ev_type, key_str, code_str, key_code, ctrl, alt, shift, meta, repeat,
                        ) {
                            println!("[Axomai Desktop] Key event triggered navigation: {}", nav_url);
                            let _ = eng.load_url(&nav_url, 1380.0, 860.0);
                        }
                    } else if let Some(nav_url) = eng.handle_key(payload) {
                        println!("[Axomai Desktop] Key triggered navigation: {}", nav_url);
                        let _ = eng.load_url(&nav_url, 1380.0, 860.0);
                    }
                } else if msg.starts_with("wheel:") {
                    let parts: Vec<&str> = msg[6..].split(',').collect();
                    if parts.len() == 4 {
                        if let (Ok(dy), Ok(dx), Ok(cx), Ok(cy)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                            parts[3].parse::<f32>(),
                        ) {
                            let _ = eng.handle_scroll_at(cx, cy, dx, dy, 1380.0, 860.0);
                        }
                    } else if let Ok(dy) = msg[6..].parse::<f32>() {
                        let _ = eng.handle_scroll(dy);
                    }
                } else if msg == "back" {
                    let _ = eng.go_back(1380.0, 860.0);
                } else if msg == "forward" {
                    let _ = eng.go_forward(1380.0, 860.0);
                } else if msg == "reload" {
                    let _ = eng.reload(1380.0, 860.0);
                }
            }
        });

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    let webview = builder.build(&window)?;

    // 6. Run Event Loop with 60 FPS Engine Tick & Display List Bridge
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
                // Tick the Axomai Rust Engine Event Loop for async fetch, navigations, timers, and scripts
                if let Ok(mut eng) = engine.lock() {
                    let updated = eng.process_event_loop(1380.0, 860.0);
                    if updated {
                        let json = eng.get_display_list_json();
                        let script = format!(
                            "if (window.__axomai_render_display_list) {{ window.__axomai_render_display_list({}); }}",
                            json
                        );
                        let _ = webview.evaluate_script(&script);

                        let url_str = eng.current_url.as_ref().map(|u| u.as_string()).unwrap_or_default();
                        let title_str = eng.current_title.replace('\\', "\\\\").replace('"', "\\\"");
                        let can_back = eng.can_go_back();
                        let can_fwd = eng.can_go_forward();
                        let sync_script = format!(
                            "if (window.__axomai_sync_navigation) {{ window.__axomai_sync_navigation(\"{}\", \"{}\", {}, {}); }}",
                            url_str, title_str, can_back, can_fwd
                        );
                        let _ = webview.evaluate_script(&sync_script);
                    }
                }
            }
            _ => {}
        }
    });
}
