use axomai_engine::AxomaiEngine;
use axomai_engine::NativeGpuCompositor;
use axomai_engine::WgpuRenderer;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle subprocess worker mode: `axomai_browser --subprocess <role>`
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--subprocess" {
        axomai_engine::run_subprocess(&args[2]);
        return Ok(());
    }

    // 1. Initialize Axomai Core Rust Engine
    let engine = Arc::new(Mutex::new(AxomaiEngine::new()));

    // 2. Create single native window — no WebView dependency
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0))
        .build(&event_loop)?;

    // 3. Initialize real wgpu GPU renderer on the native window
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let size: PhysicalSize<u32> = window.inner_size();

    let surface = unsafe {
        instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::from_window(&window)
                .expect("Failed to create surface target"),
        )
    }?;

    let mut gpu_renderer = WgpuRenderer::new(&instance, surface, size.width, size.height);
    println!(
        "[Axomai] GPU renderer initialized: {}x{} — fully native, no WebView",
        size.width, size.height
    );

    // 4. Load initial page
    if let Ok(mut eng) = engine.lock() {
        let _ = eng.load_html(
            "<html><body style='margin:0;padding:40px;font-family:system-ui;background:white'>\
             <h1 style='color:#0f172a;font-size:32px'>Axomai Browser</h1>\
             <p style='color:#475569;font-size:18px'>Native GPU-rendered browser engine. No WebView.</p>\
             <div style='margin-top:16px;padding:16px;border:2px solid #3b82f6;background:#eff6ff'>\
               <p style='color:#1e40af;font-size:16px'>This box has a blue border and light blue background.</p>\
             </div>\
             <p style='color:#64748b;font-size:14px;margin-top:12px'>Type a URL in the address bar to navigate.</p>\
             </body></html>",
            size.width as f32,
            size.height as f32,
        );
    }

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    // 5. Run Event Loop — all rendering via real wgpu, no WebView
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
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                if new_size.width > 0 && new_size.height > 0 {
                    gpu_renderer.resize(new_size.width, new_size.height);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                mouse_x = position.x as f32;
                mouse_y = position.y as f32;
                if let Ok(mut eng) = engine.lock() {
                    let _ = eng.handle_pointer_move(mouse_x, mouse_y);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let btn = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 2,
                    MouseButton::Middle => 1,
                    _ => 0,
                };
                if let Ok(mut eng) = engine.lock() {
                    match state {
                        ElementState::Pressed => {
                            if let Some(nav_url) = eng.handle_pointer_down(mouse_x, mouse_y, btn) {
                                let w = gpu_renderer.surface_config.width as f32;
                                let h = gpu_renderer.surface_config.height as f32;
                                let _ = eng.load_url(&nav_url, w, h);
                            }
                        }
                        ElementState::Released => {
                            let _ = eng.handle_pointer_up(mouse_x, mouse_y, btn);
                        }
                        _ => {}
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                let dy = match delta {
                    tao::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    _ => 0.0,
                };
                if let Ok(mut eng) = engine.lock() {
                    let w = gpu_renderer.surface_config.width as f32;
                    let h = gpu_renderer.surface_config.height as f32;
                    let _ = eng.handle_scroll_at(mouse_x, mouse_y, 0.0, dy, w, h);
                }
            }
            Event::MainEventsCleared => {
                if let Ok(mut eng) = engine.lock() {
                    let w = gpu_renderer.surface_config.width as f32;
                    let h = gpu_renderer.surface_config.height as f32;
                    let updated = eng.process_event_loop(w, h);

                    if updated || gpu_renderer.presented_frames < 3 {
                        compositor.width = w as u32;
                        compositor.height = h as u32;
                        let quads = compositor.extract_gpu_quads(&eng.display_list);

                        if compositor.glyph_atlas.dirty {
                            gpu_renderer.upload_glyph_atlas(&compositor.glyph_atlas);
                            compositor.glyph_atlas.dirty = false;
                        }

                        match gpu_renderer.render_frame(&quads) {
                            Ok(frame_idx) => {
                                if frame_idx % 300 == 1 {
                                    println!(
                                        "[Axomai GPU] Frame #{} — {} quads rendered via hardware GPU",
                                        frame_idx,
                                        quads.len()
                                    );
                                }
                            }
                            Err(wgpu::SurfaceError::Lost) => {
                                let cfg = &gpu_renderer.surface_config;
                                gpu_renderer.resize(cfg.width, cfg.height);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                *control_flow = ControlFlow::Exit;
                            }
                            Err(e) => {
                                eprintln!("[Axomai GPU] Render error: {:?}", e);
                            }
                        }

                        window.set_title(&format!(
                            "Axomai Browser — {}",
                            eng.current_title
                        ));
                    }
                }
            }
            _ => {}
        }
    });
}
