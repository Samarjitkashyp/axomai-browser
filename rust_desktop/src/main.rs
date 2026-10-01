use axomai_engine::AxomaiEngine;
use axomai_engine::NativeGpuCompositor;
use axomai_engine::WgpuRenderer;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::WindowBuilder,
};

const CHROME_HEIGHT: f32 = 76.0;
const TAB_BAR_H: f32 = 36.0;
const TOOLBAR_H: f32 = 40.0;
const ADDR_BAR_X: f32 = 140.0;
const ADDR_BAR_Y: f32 = TAB_BAR_H + 6.0;
const ADDR_BAR_H: f32 = 28.0;
const SIDEBAR_W: f32 = 160.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--subprocess" {
        axomai_engine::run_subprocess(&args[2]);
        return Ok(());
    }

    let engine = Arc::new(Mutex::new(AxomaiEngine::new()));

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0))
        .build(&event_loop)?;

    let size: PhysicalSize<u32> = window.inner_size();

    let backend_list: &[(&str, wgpu::Backends)] = if cfg!(target_os = "windows") {
        &[
            ("All", wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL),
            ("GL", wgpu::Backends::GL),
            ("Vulkan", wgpu::Backends::VULKAN),
            ("DX12", wgpu::Backends::DX12),
        ]
    } else {
        &[("All", wgpu::Backends::all())]
    };

    let mut gpu_renderer = None;
    let mut chosen_instance = None;

    for (name, backends) in backend_list {
        println!("[Axomai] Trying {} backend...", name);
        let inst = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: *backends,
            ..Default::default()
        });
        let surface_result = unsafe {
            inst.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_window(&window)
                    .expect("Failed to create surface target"),
            )
        };
        let surface = match surface_result {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Axomai] {} surface creation failed: {}", name, e);
                continue;
            }
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            WgpuRenderer::new(&inst, surface, size.width, size.height)
        })) {
            Ok(renderer) => {
                println!("[Axomai] {} backend succeeded!", name);
                gpu_renderer = Some(renderer);
                chosen_instance = Some(inst);
                break;
            }
            Err(_) => {
                eprintln!("[Axomai] {} backend failed, trying next...", name);
            }
        }
    }

    let mut gpu_renderer = gpu_renderer.expect("Failed to initialize GPU with any backend. Ensure GPU drivers are installed.");
    let _instance = chosen_instance.unwrap();
    println!(
        "[Axomai] GPU renderer initialized: {}x{} — fully native, no WebView",
        size.width, size.height
    );

    if let Ok(mut eng) = engine.lock() {
        let _ = eng.load_html(
            "<html><body style='margin:0;padding:0;font-family:system-ui;background:#0f1729'>\
             <div style='padding:60px 40px 30px 40px;background:#0f1729'>\
               <h1 style='color:#60a5fa;font-size:36px;margin:0'>Axomai Browser</h1>\
               <p style='color:#94a3b8;font-size:16px;margin-top:4px'>Fast. Private. AI-Powered. Built for Everyone.</p>\
             </div>\
             <div style='margin:0 40px;padding:14px 20px;background:#1e293b;border:1px solid #334155'>\
               <p style='color:#94a3b8;font-size:15px'>Search the web with Axomai...</p>\
             </div>\
             <div style='padding:30px 40px'>\
               <p style='color:#cbd5e1;font-size:14px;margin-bottom:12px'>Quick Links</p>\
               <div style='display:flex'>\
                 <div style='padding:14px 24px;background:#1e293b;border:1px solid #334155;margin-right:12px'>\
                   <p style='color:#f8fafc;font-size:14px'>Google</p>\
                 </div>\
                 <div style='padding:14px 24px;background:#1e293b;border:1px solid #334155;margin-right:12px'>\
                   <p style='color:#f8fafc;font-size:14px'>YouTube</p>\
                 </div>\
                 <div style='padding:14px 24px;background:#1e293b;border:1px solid #334155;margin-right:12px'>\
                   <p style='color:#f8fafc;font-size:14px'>Facebook</p>\
                 </div>\
                 <div style='padding:14px 24px;background:#1e293b;border:1px solid #334155;margin-right:12px'>\
                   <p style='color:#f8fafc;font-size:14px'>Amazon</p>\
                 </div>\
                 <div style='padding:14px 24px;background:#1e293b;border:1px solid #334155'>\
                   <p style='color:#f8fafc;font-size:14px'>Flipkart</p>\
                 </div>\
               </div>\
             </div>\
             <div style='padding:20px 40px'>\
               <p style='color:#cbd5e1;font-size:14px;margin-bottom:12px'>AI Assistant</p>\
               <div style='padding:16px 20px;background:#172554;border:1px solid #1e40af'>\
                 <p style='color:#93c5fd;font-size:15px'>Hello! I am Axomai AI</p>\
                 <p style='color:#64748b;font-size:13px;margin-top:4px'>Your intelligent browsing assistant for a better web experience.</p>\
               </div>\
               <div style='margin-top:8px;padding:10px 16px;background:#1e293b;border:1px solid #334155'>\
                 <p style='color:#94a3b8;font-size:13px'>Summarize this page</p>\
               </div>\
               <div style='margin-top:6px;padding:10px 16px;background:#1e293b;border:1px solid #334155'>\
                 <p style='color:#94a3b8;font-size:13px'>Translate to Assamese</p>\
               </div>\
               <div style='margin-top:6px;padding:10px 16px;background:#1e293b;border:1px solid #334155'>\
                 <p style='color:#94a3b8;font-size:13px'>Find similar content</p>\
               </div>\
             </div>\
             <div style='padding:20px 40px'>\
               <p style='color:#cbd5e1;font-size:14px;margin-bottom:12px'>Quick Tools</p>\
               <div style='display:flex'>\
                 <div style='padding:12px 20px;background:#1e293b;border:1px solid #334155;margin-right:10px'>\
                   <p style='color:#f8fafc;font-size:12px'>Reader Mode</p>\
                 </div>\
                 <div style='padding:12px 20px;background:#1e293b;border:1px solid #334155;margin-right:10px'>\
                   <p style='color:#f8fafc;font-size:12px'>AI Notes</p>\
                 </div>\
                 <div style='padding:12px 20px;background:#1e293b;border:1px solid #334155;margin-right:10px'>\
                   <p style='color:#f8fafc;font-size:12px'>Bookmarks</p>\
                 </div>\
                 <div style='padding:12px 20px;background:#1e293b;border:1px solid #334155'>\
                   <p style='color:#f8fafc;font-size:12px'>History</p>\
                 </div>\
               </div>\
             </div>\
             <div style='padding:30px 40px'>\
               <p style='color:#475569;font-size:12px'>Axomai Browser v1.6 - Native GPU-Rendered Engine - No WebView</p>\
             </div>\
             </body></html>",
            size.width as f32,
            size.height as f32 - CHROME_HEIGHT,
        );
    }

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    let mut address_bar_text = String::from("about:home");
    let mut address_bar_focused = false;
    let mut needs_chrome_redraw = true;

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
                    needs_chrome_redraw = true;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                mouse_x = position.x as f32;
                mouse_y = position.y as f32;
                if mouse_y > CHROME_HEIGHT {
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_move(mouse_x, mouse_y - CHROME_HEIGHT);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let w = gpu_renderer.surface_config.width as f32;
                    if mouse_y < CHROME_HEIGHT {
                        // Chrome area clicks
                        let addr_bar_w = w - ADDR_BAR_X - 120.0;
                        if mouse_x >= ADDR_BAR_X
                            && mouse_x <= ADDR_BAR_X + addr_bar_w
                            && mouse_y >= ADDR_BAR_Y
                            && mouse_y <= ADDR_BAR_Y + ADDR_BAR_H
                        {
                            address_bar_focused = true;
                            needs_chrome_redraw = true;
                        } else if mouse_x >= 8.0 && mouse_x <= 40.0 {
                            // Back button
                            if let Ok(mut eng) = engine.lock() {
                                if eng.history_index > 0 {
                                    eng.history_index -= 1;
                                    let entry = eng.history[eng.history_index].clone();
                                    let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                    let _ = eng.load_url(&entry.url, w, h);
                                    address_bar_text = entry.url;
                                    needs_chrome_redraw = true;
                                }
                            }
                        } else if mouse_x >= 44.0 && mouse_x <= 76.0 {
                            // Forward button
                            if let Ok(mut eng) = engine.lock() {
                                if eng.history_index + 1 < eng.history.len() {
                                    eng.history_index += 1;
                                    let entry = eng.history[eng.history_index].clone();
                                    let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                    let _ = eng.load_url(&entry.url, w, h);
                                    address_bar_text = entry.url;
                                    needs_chrome_redraw = true;
                                }
                            }
                        } else if mouse_x >= 80.0 && mouse_x <= 130.0 {
                            // Reload button
                            if let Ok(mut eng) = engine.lock() {
                                let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                if let Some(url) = eng.current_url.as_ref().map(|u| u.as_string()) {
                                    let _ = eng.load_url(&url, w, h);
                                }
                            }
                        }
                    } else {
                        address_bar_focused = false;
                        let btn = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 2,
                            MouseButton::Middle => 1,
                            _ => 0,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            if let Some(nav_url) = eng.handle_pointer_down(mouse_x, mouse_y - CHROME_HEIGHT, btn) {
                                let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                let _ = eng.load_url(&nav_url, w, h);
                                address_bar_text = nav_url;
                                needs_chrome_redraw = true;
                            }
                        }
                    }
                }
                if state == ElementState::Released {
                    if mouse_y > CHROME_HEIGHT {
                        let btn = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 2,
                            MouseButton::Middle => 1,
                            _ => 0,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            let _ = eng.handle_pointer_up(mouse_x, mouse_y - CHROME_HEIGHT, btn);
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event: key_event, .. },
                ..
            } => {
                if key_event.state == ElementState::Pressed {
                    if address_bar_focused {
                        match key_event.logical_key {
                            Key::Character(ref ch) => {
                                address_bar_text.push_str(ch.as_ref());
                                needs_chrome_redraw = true;
                            }
                            Key::Backspace => {
                                address_bar_text.pop();
                                needs_chrome_redraw = true;
                            }
                            Key::Enter => {
                                address_bar_focused = false;
                                let url = if address_bar_text.contains("://") || address_bar_text.contains('.') {
                                    if !address_bar_text.contains("://") {
                                        format!("https://{}", address_bar_text)
                                    } else {
                                        address_bar_text.clone()
                                    }
                                } else if !address_bar_text.is_empty() {
                                    format!("https://html.duckduckgo.com/html/?q={}", address_bar_text)
                                } else {
                                    return;
                                };
                                if let Ok(mut eng) = engine.lock() {
                                    let w = gpu_renderer.surface_config.width as f32;
                                    let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                    let _ = eng.load_url(&url, w, h);
                                    address_bar_text = url;
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Escape => {
                                address_bar_focused = false;
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else {
                        let key_str = match key_event.logical_key {
                            Key::Character(ref ch) => ch.to_string(),
                            Key::Backspace => "BackSpace".to_string(),
                            Key::Enter => "Enter".to_string(),
                            Key::Tab => "Tab".to_string(),
                            Key::Escape => "Escape".to_string(),
                            Key::Space => " ".to_string(),
                            Key::ArrowUp => "ArrowUp".to_string(),
                            Key::ArrowDown => "ArrowDown".to_string(),
                            Key::ArrowLeft => "ArrowLeft".to_string(),
                            Key::ArrowRight => "ArrowRight".to_string(),
                            _ => return,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            if let Some(nav_url) = eng.handle_key_event(
                                "keydown",
                                &key_str,
                                "",
                                0,
                                false, false, false, false, false,
                            ) {
                                let w = gpu_renderer.surface_config.width as f32;
                                let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                                let _ = eng.load_url(&nav_url, w, h);
                                address_bar_text = nav_url;
                                needs_chrome_redraw = true;
                            }
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                if mouse_y > CHROME_HEIGHT {
                    let dy = match delta {
                        tao::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        _ => 0.0,
                    };
                    if let Ok(mut eng) = engine.lock() {
                        let w = gpu_renderer.surface_config.width as f32;
                        let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                        let _ = eng.handle_scroll_at(mouse_x, mouse_y - CHROME_HEIGHT, 0.0, dy, w, h);
                    }
                }
            }
            Event::MainEventsCleared => {
                if let Ok(mut eng) = engine.lock() {
                    let w = gpu_renderer.surface_config.width as f32;
                    let h = gpu_renderer.surface_config.height as f32 - CHROME_HEIGHT;
                    let updated = eng.process_event_loop(w, h);

                    if updated || needs_chrome_redraw || gpu_renderer.presented_frames < 3 {
                        needs_chrome_redraw = false;
                        compositor.width = w as u32;
                        compositor.height = h as u32;

                        let mut quads = build_chrome_quads(
                            &mut compositor,
                            w,
                            &address_bar_text,
                            address_bar_focused,
                            &eng,
                        );

                        let mut page_quads = compositor.extract_gpu_quads(&eng.display_list);
                        for q in &mut page_quads {
                            for v in &mut q.vertices {
                                v.position[1] += CHROME_HEIGHT;
                            }
                        }
                        quads.extend(page_quads);

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

use axomai_engine::GpuQuad;

fn c(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
}

fn render_text(
    compositor: &mut NativeGpuCompositor,
    quads: &mut Vec<GpuQuad>,
    text: &str,
    mut x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
    max_x: f32,
) -> f32 {
    for ch in text.chars() {
        if x > max_x { break; }
        let g = compositor.glyph_atlas.rasterize(ch, size);
        if g.width > 0.0 {
            quads.push(NativeGpuCompositor::text_quad(
                x + g.offset_x,
                y - g.offset_y - g.height,
                &g,
                color,
            ));
        }
        x += g.advance_width;
    }
    x
}

fn build_chrome_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    address_text: &str,
    focused: bool,
    engine: &AxomaiEngine,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Tab bar background (dark)
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, viewport_w, TAB_BAR_H, c(30, 32, 44, 255)));

    // Active tab
    let tab_title = if engine.current_title.is_empty() { "New Tab" } else { &engine.current_title };
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, 220.0, TAB_BAR_H, c(45, 48, 62, 255)));
    // Tab bottom highlight
    quads.push(NativeGpuCompositor::solid_quad(0.0, TAB_BAR_H - 2.0, 220.0, 2.0, c(96, 165, 250, 255)));
    // Tab text
    render_text(compositor, &mut quads, tab_title, 12.0, 23.0, 12.0, c(230, 230, 240, 255), 200.0);
    // Tab close X
    render_text(compositor, &mut quads, "x", 200.0, 23.0, 11.0, c(140, 140, 160, 255), 220.0);

    // + button for new tab
    quads.push(NativeGpuCompositor::solid_quad(224.0, 4.0, 28.0, 28.0, c(40, 42, 54, 255)));
    render_text(compositor, &mut quads, "+", 232.0, 23.0, 14.0, c(140, 140, 160, 255), 260.0);

    // Toolbar background
    quads.push(NativeGpuCompositor::solid_quad(0.0, TAB_BAR_H, viewport_w, TOOLBAR_H, c(38, 40, 54, 255)));
    // Toolbar bottom border
    quads.push(NativeGpuCompositor::solid_quad(0.0, CHROME_HEIGHT - 1.0, viewport_w, 1.0, c(55, 58, 72, 255)));

    // Navigation buttons
    let nav_y = TAB_BAR_H + 10.0;
    let back_color = if engine.history_index > 0 { c(200, 200, 220, 255) } else { c(80, 82, 96, 255) };
    render_text(compositor, &mut quads, "<", 16.0, nav_y + 12.0, 16.0, back_color, 40.0);

    let fwd_color = if engine.history_index + 1 < engine.history.len() { c(200, 200, 220, 255) } else { c(80, 82, 96, 255) };
    render_text(compositor, &mut quads, ">", 44.0, nav_y + 12.0, 16.0, fwd_color, 70.0);

    // Reload
    render_text(compositor, &mut quads, "R", 76.0, nav_y + 12.0, 13.0, c(160, 162, 180, 255), 100.0);

    // Shield icon placeholder
    render_text(compositor, &mut quads, "O", 108.0, nav_y + 12.0, 13.0, c(96, 165, 250, 255), 130.0);

    // Address bar (dark themed)
    let addr_bar_w = viewport_w - ADDR_BAR_X - 120.0;
    let border_color = if focused { c(96, 165, 250, 255) } else { c(55, 58, 72, 255) };
    quads.push(NativeGpuCompositor::solid_quad(ADDR_BAR_X, ADDR_BAR_Y, addr_bar_w, ADDR_BAR_H, border_color));
    quads.push(NativeGpuCompositor::solid_quad(ADDR_BAR_X + 1.0, ADDR_BAR_Y + 1.0, addr_bar_w - 2.0, ADDR_BAR_H - 2.0, c(24, 26, 38, 255)));

    // Address bar text
    let text_x = ADDR_BAR_X + 10.0;
    let text_y = ADDR_BAR_Y + 20.0;
    let max_x = ADDR_BAR_X + addr_bar_w - 10.0;
    let end_x = render_text(compositor, &mut quads, address_text, text_x, text_y, 13.0, c(200, 205, 220, 255), max_x);

    // Cursor
    if focused {
        quads.push(NativeGpuCompositor::solid_quad(end_x, ADDR_BAR_Y + 5.0, 1.0, ADDR_BAR_H - 10.0, c(96, 165, 250, 255)));
    }

    // Right side toolbar buttons
    let right_x = viewport_w - 110.0;
    // Bookmark star
    render_text(compositor, &mut quads, "*", right_x, nav_y + 12.0, 16.0, c(140, 142, 160, 255), right_x + 24.0);
    // Download arrow
    render_text(compositor, &mut quads, "v", right_x + 28.0, nav_y + 12.0, 14.0, c(140, 142, 160, 255), right_x + 52.0);
    // AI button
    quads.push(NativeGpuCompositor::solid_quad(right_x + 54.0, nav_y - 2.0, 26.0, 22.0, c(96, 165, 250, 255)));
    render_text(compositor, &mut quads, "AI", right_x + 57.0, nav_y + 11.0, 11.0, c(255, 255, 255, 255), right_x + 80.0);
    // Menu
    render_text(compositor, &mut quads, "=", right_x + 86.0, nav_y + 12.0, 14.0, c(140, 142, 160, 255), right_x + 110.0);

    quads
}
