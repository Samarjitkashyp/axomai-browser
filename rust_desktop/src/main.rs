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

const CHROME_HEIGHT: f32 = 48.0;
const ADDR_BAR_X: f32 = 140.0;
const ADDR_BAR_Y: f32 = 8.0;
const ADDR_BAR_H: f32 = 32.0;

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

    let backends = if cfg!(target_os = "windows") {
        wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL
    } else {
        wgpu::Backends::all()
    };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    println!("[Axomai] wgpu instance created (backends: {:?})", backends);

    let size: PhysicalSize<u32> = window.inner_size();

    let surface = unsafe {
        instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::from_window(&window)
                .expect("Failed to create surface target"),
        )
    }?;
    println!("[Axomai] Surface created successfully");

    let mut gpu_renderer = WgpuRenderer::new(&instance, surface, size.width, size.height);
    println!(
        "[Axomai] GPU renderer initialized: {}x{} — fully native, no WebView",
        size.width, size.height
    );

    if let Ok(mut eng) = engine.lock() {
        let _ = eng.load_html(
            "<html><body style='margin:0;padding:40px;font-family:system-ui;background:#f8fafc'>\
             <h1 style='color:#0f172a;font-size:32px'>Axomai Browser</h1>\
             <p style='color:#475569;font-size:18px'>Native GPU-rendered browser engine. No WebView.</p>\
             <div style='margin-top:16px;padding:16px;border:2px solid #3b82f6;background:#eff6ff'>\
               <p style='color:#1e40af;font-size:16px'>Borders, backgrounds, and text rendering work!</p>\
             </div>\
             <div style='margin-top:12px;padding:12px;background:#fef2f2;border:1px solid #fca5a5'>\
               <p style='color:#991b1b;font-size:14px'>Red-themed box with border.</p>\
             </div>\
             <p style='color:#64748b;font-size:14px;margin-top:12px'>Type a URL in the address bar to navigate.</p>\
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
                        let addr_bar_w = w - ADDR_BAR_X - 16.0;
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

fn build_chrome_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    address_text: &str,
    focused: bool,
    engine: &AxomaiEngine,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, viewport_w, CHROME_HEIGHT, c(240, 240, 245, 255)));
    quads.push(NativeGpuCompositor::solid_quad(0.0, CHROME_HEIGHT - 1.0, viewport_w, 1.0, c(200, 200, 210, 255)));

    let back_color = if engine.history_index > 0 { c(60, 60, 60, 255) } else { c(180, 180, 180, 255) };
    let back_glyph = compositor.glyph_atlas.rasterize('<', 18.0);
    quads.push(NativeGpuCompositor::text_quad(16.0, 14.0, &back_glyph, back_color));

    let fwd_color = if engine.history_index + 1 < engine.history.len() { c(60, 60, 60, 255) } else { c(180, 180, 180, 255) };
    let fwd_glyph = compositor.glyph_atlas.rasterize('>', 18.0);
    quads.push(NativeGpuCompositor::text_quad(52.0, 14.0, &fwd_glyph, fwd_color));

    let reload_glyph = compositor.glyph_atlas.rasterize('R', 14.0);
    quads.push(NativeGpuCompositor::text_quad(96.0, 16.0, &reload_glyph, c(80, 80, 80, 255)));

    let addr_bar_w = viewport_w - ADDR_BAR_X - 16.0;
    let border_color = if focused { c(59, 130, 246, 255) } else { c(200, 200, 210, 255) };
    quads.push(NativeGpuCompositor::solid_quad(ADDR_BAR_X, ADDR_BAR_Y, addr_bar_w, ADDR_BAR_H, border_color));
    quads.push(NativeGpuCompositor::solid_quad(ADDR_BAR_X + 1.0, ADDR_BAR_Y + 1.0, addr_bar_w - 2.0, ADDR_BAR_H - 2.0, c(255, 255, 255, 255)));

    let mut text_x = ADDR_BAR_X + 8.0;
    let text_y = ADDR_BAR_Y + 22.0;
    let max_x = ADDR_BAR_X + addr_bar_w - 8.0;
    for ch in address_text.chars() {
        if text_x > max_x { break; }
        let g = compositor.glyph_atlas.rasterize(ch, 14.0);
        if g.width > 0.0 {
            quads.push(NativeGpuCompositor::text_quad(text_x + g.offset_x, text_y - g.offset_y - g.height, &g, c(30, 30, 30, 255)));
        }
        text_x += g.advance_width;
    }

    if focused {
        quads.push(NativeGpuCompositor::solid_quad(text_x, ADDR_BAR_Y + 6.0, 1.0, ADDR_BAR_H - 12.0, c(59, 130, 246, 255)));
    }

    quads
}
