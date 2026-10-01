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

const SIDEBAR_W: f32 = 180.0;
const TAB_BAR_H: f32 = 40.0;
const TOOLBAR_H: f32 = 44.0;
const CHROME_TOP: f32 = TAB_BAR_H + TOOLBAR_H;

struct SidebarItem {
    label: &'static str,
    icon: &'static str,
    is_section: bool,
}

const SIDEBAR_ITEMS: &[SidebarItem] = &[
    SidebarItem { label: "Home", icon: "H", is_section: false },
    SidebarItem { label: "AI Assistant", icon: "A", is_section: false },
    SidebarItem { label: "Bookmarks", icon: "B", is_section: false },
    SidebarItem { label: "History", icon: "h", is_section: false },
    SidebarItem { label: "Downloads", icon: "D", is_section: false },
    SidebarItem { label: "Extensions", icon: "E", is_section: false },
    SidebarItem { label: "Passwords", icon: "P", is_section: false },
    SidebarItem { label: "Settings", icon: "S", is_section: false },
    SidebarItem { label: "Workspaces", icon: "", is_section: true },
    SidebarItem { label: "Personal", icon: "o", is_section: false },
    SidebarItem { label: "Work", icon: "o", is_section: false },
    SidebarItem { label: "Study", icon: "o", is_section: false },
    SidebarItem { label: "AI Tools", icon: "o", is_section: false },
];

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

    let mut gpu_renderer = gpu_renderer
        .expect("Failed to initialize GPU with any backend. Ensure GPU drivers are installed.");
    let _instance = chosen_instance.unwrap();
    println!(
        "[Axomai] GPU renderer initialized: {}x{} — fully native, no WebView",
        size.width, size.height
    );

    if let Ok(mut eng) = engine.lock() {
        let content_w = size.width as f32 - SIDEBAR_W;
        let content_h = size.height as f32 - CHROME_TOP;
        let _ = eng.load_html(HOME_PAGE_HTML, content_w, content_h);
    }

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    let mut address_bar_text = String::from("about:home");
    let mut address_bar_focused = false;
    let mut needs_chrome_redraw = true;
    let mut sidebar_active: usize = 0;

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
                if mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_move(
                            mouse_x - SIDEBAR_W,
                            mouse_y - CHROME_TOP,
                        );
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let w = gpu_renderer.surface_config.width as f32;
                if state == ElementState::Pressed && button == MouseButton::Left {
                    if mouse_x < SIDEBAR_W && mouse_y > CHROME_TOP {
                        // Sidebar click
                        let rel_y = mouse_y - CHROME_TOP - 16.0;
                        let mut y_off = 0.0f32;
                        for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                            let item_h = if item.is_section { 36.0 } else { 34.0 };
                            if rel_y >= y_off && rel_y < y_off + item_h && !item.is_section {
                                sidebar_active = i;
                                needs_chrome_redraw = true;
                                break;
                            }
                            y_off += item_h;
                        }
                    } else if mouse_y < CHROME_TOP {
                        // Chrome area
                        let addr_x = SIDEBAR_W + 140.0;
                        let addr_y = TAB_BAR_H + 8.0;
                        let addr_h = 30.0;
                        let addr_w = w - addr_x - 160.0;
                        if mouse_x >= addr_x
                            && mouse_x <= addr_x + addr_w
                            && mouse_y >= addr_y
                            && mouse_y <= addr_y + addr_h
                        {
                            address_bar_focused = true;
                            needs_chrome_redraw = true;
                        } else if mouse_y < TAB_BAR_H {
                            // Tab bar clicks handled here
                            address_bar_focused = false;
                        } else {
                            // Toolbar nav button clicks
                            let nav_base_x = SIDEBAR_W + 8.0;
                            if mouse_x >= nav_base_x && mouse_x <= nav_base_x + 32.0 {
                                // Back
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index > 0 {
                                        eng.history_index -= 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 36.0 && mouse_x <= nav_base_x + 68.0 {
                                // Forward
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index + 1 < eng.history.len() {
                                        eng.history_index += 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 72.0 && mouse_x <= nav_base_x + 104.0 {
                                // Reload
                                if let Ok(mut eng) = engine.lock() {
                                    let content_w = w - SIDEBAR_W;
                                    let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                    if let Some(url) = eng.current_url.as_ref().map(|u| u.as_string()) {
                                        let _ = eng.load_url(&url, content_w, content_h);
                                    }
                                }
                            }
                            address_bar_focused = false;
                            needs_chrome_redraw = true;
                        }
                    } else {
                        // Content area click
                        address_bar_focused = false;
                        let btn = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 2,
                            MouseButton::Middle => 1,
                            _ => 0,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            if let Some(nav_url) = eng.handle_pointer_down(
                                mouse_x - SIDEBAR_W,
                                mouse_y - CHROME_TOP,
                                btn,
                            ) {
                                let content_w = w - SIDEBAR_W;
                                let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                let _ = eng.load_url(&nav_url, content_w, content_h);
                                address_bar_text = nav_url;
                                needs_chrome_redraw = true;
                            }
                        }
                    }
                }
                if state == ElementState::Released && mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    let btn = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 1,
                        _ => 0,
                    };
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_up(
                            mouse_x - SIDEBAR_W,
                            mouse_y - CHROME_TOP,
                            btn,
                        );
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
                                let url = if address_bar_text.contains("://")
                                    || address_bar_text.contains('.')
                                {
                                    if !address_bar_text.contains("://") {
                                        format!("https://{}", address_bar_text)
                                    } else {
                                        address_bar_text.clone()
                                    }
                                } else if !address_bar_text.is_empty() {
                                    format!(
                                        "https://html.duckduckgo.com/html/?q={}",
                                        address_bar_text
                                    )
                                } else {
                                    return;
                                };
                                if let Ok(mut eng) = engine.lock() {
                                    let w = gpu_renderer.surface_config.width as f32;
                                    let content_w = w - SIDEBAR_W;
                                    let content_h =
                                        gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                    let _ = eng.load_url(&url, content_w, content_h);
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
                                "keydown", &key_str, "", 0, false, false, false, false, false,
                            ) {
                                let w = gpu_renderer.surface_config.width as f32;
                                let content_w = w - SIDEBAR_W;
                                let content_h =
                                    gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                let _ = eng.load_url(&nav_url, content_w, content_h);
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
                if mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    let dy = match delta {
                        tao::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                        tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        _ => 0.0,
                    };
                    if let Ok(mut eng) = engine.lock() {
                        let w = gpu_renderer.surface_config.width as f32;
                        let content_w = w - SIDEBAR_W;
                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                        let _ = eng.handle_scroll_at(
                            mouse_x - SIDEBAR_W,
                            mouse_y - CHROME_TOP,
                            0.0,
                            dy,
                            content_w,
                            content_h,
                        );
                    }
                }
            }
            Event::MainEventsCleared => {
                if let Ok(mut eng) = engine.lock() {
                    let w = gpu_renderer.surface_config.width as f32;
                    let h = gpu_renderer.surface_config.height as f32;
                    let content_w = w - SIDEBAR_W;
                    let content_h = h - CHROME_TOP;
                    let updated = eng.process_event_loop(content_w, content_h);

                    if updated || needs_chrome_redraw || gpu_renderer.presented_frames < 3 {
                        needs_chrome_redraw = false;
                        compositor.width = content_w as u32;
                        compositor.height = content_h as u32;

                        let mut quads = build_chrome_quads(
                            &mut compositor,
                            w,
                            h,
                            &address_bar_text,
                            address_bar_focused,
                            &eng,
                            sidebar_active,
                        );

                        let mut page_quads = compositor.extract_gpu_quads(&eng.display_list);
                        for q in &mut page_quads {
                            for v in &mut q.vertices {
                                v.position[0] += SIDEBAR_W;
                                v.position[1] += CHROME_TOP;
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
                                        "[Axomai GPU] Frame #{} — {} quads rendered",
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

                        window.set_title(&format!("Axomai Browser — {}", eng.current_title));
                    }
                }
            }
            _ => {}
        }
    });
}

use axomai_engine::GpuQuad;

fn c(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
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
        if x > max_x {
            break;
        }
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
    viewport_h: f32,
    address_text: &str,
    focused: bool,
    engine: &AxomaiEngine,
    sidebar_active: usize,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // === SIDEBAR (full height, left side) ===
    let sb_bg = c(17, 20, 33, 255);
    let sb_active_bg = c(37, 99, 235, 255);
    let _sb_hover_bg = c(30, 35, 52, 255);
    let sb_text = c(180, 185, 200, 255);
    let sb_text_active = c(255, 255, 255, 255);
    let sb_section_text = c(100, 105, 120, 255);

    // Sidebar background
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, SIDEBAR_W, viewport_h, sb_bg));
    // Sidebar right border
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W - 1.0, 0.0, 1.0, viewport_h, c(40, 44, 60, 255)));

    // Logo area at top of sidebar
    let logo_color = c(96, 165, 250, 255);
    render_text(compositor, &mut quads, "Axomai", 16.0, 30.0, 16.0, logo_color, SIDEBAR_W - 8.0);
    render_text(compositor, &mut quads, "Browser", 86.0, 30.0, 10.0, c(140, 145, 165, 255), SIDEBAR_W - 8.0);

    // Sidebar divider after logo
    quads.push(NativeGpuCompositor::solid_quad(12.0, 42.0, SIDEBAR_W - 24.0, 1.0, c(40, 44, 60, 255)));

    // Sidebar items
    let mut item_y = 52.0;
    for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
        if item.is_section {
            // Section header
            item_y += 8.0;
            quads.push(NativeGpuCompositor::solid_quad(12.0, item_y, SIDEBAR_W - 24.0, 1.0, c(40, 44, 60, 255)));
            item_y += 8.0;
            render_text(compositor, &mut quads, item.label, 16.0, item_y + 14.0, 10.0, sb_section_text, SIDEBAR_W - 8.0);
            item_y += 24.0;
        } else {
            let is_active = i == sidebar_active;
            let item_h = 32.0;
            if is_active {
                quads.push(NativeGpuCompositor::solid_quad(8.0, item_y, SIDEBAR_W - 16.0, item_h, sb_active_bg));
            }
            let text_color = if is_active { sb_text_active } else { sb_text };
            // Icon placeholder
            render_text(compositor, &mut quads, item.icon, 20.0, item_y + 21.0, 13.0, text_color, 40.0);
            // Label
            render_text(compositor, &mut quads, item.label, 40.0, item_y + 21.0, 13.0, text_color, SIDEBAR_W - 8.0);
            item_y += item_h + 2.0;
        }
    }

    // + Add Workspace at bottom of sidebar items
    item_y += 4.0;
    render_text(compositor, &mut quads, "+ Add Workspace", 20.0, item_y + 14.0, 11.0, c(96, 165, 250, 255), SIDEBAR_W - 8.0);

    // === TAB BAR (right of sidebar, at top) ===
    let tab_bg = c(22, 25, 38, 255);
    let tab_active_bg = c(32, 36, 52, 255);

    // Tab bar background
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, 0.0, viewport_w - SIDEBAR_W, TAB_BAR_H, tab_bg));
    // Tab bar bottom border
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, TAB_BAR_H - 1.0, viewport_w - SIDEBAR_W, 1.0, c(40, 44, 60, 255)));

    // Active tab
    let tab_title = if engine.current_title.is_empty() {
        "New Tab"
    } else {
        &engine.current_title
    };
    let tab_x = SIDEBAR_W + 4.0;
    let tab_w = 200.0;
    quads.push(NativeGpuCompositor::solid_quad(tab_x, 4.0, tab_w, TAB_BAR_H - 4.0, tab_active_bg));
    // Active tab bottom accent
    quads.push(NativeGpuCompositor::solid_quad(tab_x, TAB_BAR_H - 2.0, tab_w, 2.0, c(96, 165, 250, 255)));
    // Tab icon placeholder (circle)
    quads.push(NativeGpuCompositor::solid_quad(tab_x + 10.0, 14.0, 14.0, 14.0, c(96, 165, 250, 255)));
    // Tab title
    render_text(
        compositor, &mut quads, tab_title,
        tab_x + 30.0, 26.0, 12.0, c(220, 225, 240, 255), tab_x + tab_w - 24.0,
    );
    // Tab close X
    render_text(
        compositor, &mut quads, "x",
        tab_x + tab_w - 18.0, 26.0, 11.0, c(120, 125, 140, 255), tab_x + tab_w,
    );

    // + New tab button
    let plus_x = tab_x + tab_w + 8.0;
    quads.push(NativeGpuCompositor::solid_quad(plus_x, 8.0, 28.0, 26.0, c(28, 32, 48, 255)));
    render_text(compositor, &mut quads, "+", plus_x + 8.0, 26.0, 14.0, c(120, 125, 140, 255), plus_x + 28.0);

    // === TOOLBAR (below tab bar, right of sidebar) ===
    let toolbar_bg = c(26, 29, 44, 255);
    let toolbar_y = TAB_BAR_H;

    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, toolbar_y, viewport_w - SIDEBAR_W, TOOLBAR_H, toolbar_bg));
    // Toolbar bottom border
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, CHROME_TOP - 1.0, viewport_w - SIDEBAR_W, 1.0, c(45, 48, 64, 255)));

    // Navigation buttons
    let nav_y = toolbar_y + 8.0;
    let nav_base_x = SIDEBAR_W + 12.0;

    // Back arrow
    let back_color = if engine.history_index > 0 { c(200, 205, 220, 255) } else { c(60, 64, 80, 255) };
    render_text(compositor, &mut quads, "<", nav_base_x + 8.0, nav_y + 18.0, 16.0, back_color, nav_base_x + 32.0);

    // Forward arrow
    let fwd_color = if engine.history_index + 1 < engine.history.len() { c(200, 205, 220, 255) } else { c(60, 64, 80, 255) };
    render_text(compositor, &mut quads, ">", nav_base_x + 44.0, nav_y + 18.0, 16.0, fwd_color, nav_base_x + 68.0);

    // Reload
    render_text(compositor, &mut quads, "R", nav_base_x + 80.0, nav_y + 18.0, 14.0, c(140, 145, 165, 255), nav_base_x + 104.0);

    // Shield icon
    quads.push(NativeGpuCompositor::solid_quad(nav_base_x + 108.0, nav_y + 4.0, 20.0, 20.0, c(96, 165, 250, 80)));
    render_text(compositor, &mut quads, "S", nav_base_x + 112.0, nav_y + 18.0, 12.0, c(96, 165, 250, 255), nav_base_x + 130.0);

    // Address bar
    let addr_x = SIDEBAR_W + 140.0;
    let addr_y = toolbar_y + 7.0;
    let addr_h = 30.0;
    let addr_w = viewport_w - addr_x - 170.0;

    // Address bar border
    let border_c = if focused { c(96, 165, 250, 255) } else { c(50, 54, 70, 255) };
    quads.push(NativeGpuCompositor::solid_quad(addr_x, addr_y, addr_w, addr_h, border_c));
    // Address bar inner fill
    quads.push(NativeGpuCompositor::solid_quad(addr_x + 1.0, addr_y + 1.0, addr_w - 2.0, addr_h - 2.0, c(18, 21, 34, 255)));

    // Search icon placeholder in address bar
    render_text(compositor, &mut quads, "O", addr_x + 10.0, addr_y + 21.0, 12.0, c(96, 165, 250, 255), addr_x + 28.0);

    // Address bar text
    let text_x = addr_x + 28.0;
    let text_max_x = addr_x + addr_w - 10.0;
    let display_text = if address_text == "about:home" && !focused {
        "Search Axomai or enter address"
    } else {
        address_text
    };
    let text_color = if address_text == "about:home" && !focused {
        c(100, 105, 125, 255)
    } else {
        c(200, 205, 225, 255)
    };
    let end_x = render_text(compositor, &mut quads, display_text, text_x, addr_y + 21.0, 13.0, text_color, text_max_x);

    // Cursor blink
    if focused {
        quads.push(NativeGpuCompositor::solid_quad(end_x + 1.0, addr_y + 6.0, 1.5, addr_h - 12.0, c(96, 165, 250, 255)));
    }

    // Right-side toolbar buttons
    let right_base = viewport_w - 160.0;

    // Bookmark star
    render_text(compositor, &mut quads, "*", right_base, nav_y + 18.0, 18.0, c(130, 135, 155, 255), right_base + 24.0);

    // Capture icon
    render_text(compositor, &mut quads, "C", right_base + 30.0, nav_y + 18.0, 13.0, c(130, 135, 155, 255), right_base + 50.0);

    // Download icon
    render_text(compositor, &mut quads, "v", right_base + 56.0, nav_y + 18.0, 14.0, c(130, 135, 155, 255), right_base + 74.0);

    // AI button (highlighted)
    quads.push(NativeGpuCompositor::solid_quad(right_base + 80.0, nav_y + 2.0, 30.0, 24.0, c(96, 165, 250, 255)));
    render_text(compositor, &mut quads, "AI", right_base + 84.0, nav_y + 17.0, 11.0, c(255, 255, 255, 255), right_base + 110.0);

    // Profile circle
    quads.push(NativeGpuCompositor::solid_quad(right_base + 116.0, nav_y + 2.0, 24.0, 24.0, c(60, 64, 80, 255)));
    render_text(compositor, &mut quads, "U", right_base + 121.0, nav_y + 17.0, 12.0, c(180, 185, 200, 255), right_base + 140.0);

    // Menu (hamburger)
    render_text(compositor, &mut quads, "=", right_base + 146.0, nav_y + 18.0, 14.0, c(130, 135, 155, 255), right_base + 166.0);

    quads
}

const HOME_PAGE_HTML: &str = "\
<html><body style='margin:0;padding:0;font-family:system-ui;background:#0d1117'>\
<div style='padding:48px 40px 24px 40px;background:#0d1117'>\
  <div style='padding:0 0 0 0'>\
    <p style='color:#60a5fa;font-size:42px;margin:0'>Axomai Browser</p>\
    <p style='color:#7c8ba1;font-size:16px;margin-top:6px'>Fast. Private. AI-Powered. Built for Everyone.</p>\
  </div>\
</div>\
<div style='margin:0 40px;padding:14px 20px;background:#161b28;border:1px solid #2a3040'>\
  <p style='color:#6b7a8d;font-size:15px'>Search the web with Axomai AI...</p>\
</div>\
<div style='padding:28px 40px 8px 40px'>\
  <p style='color:#8b95a5;font-size:13px;margin-bottom:14px'>Quick Links</p>\
  <div style='display:flex'>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>YouTube</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>Google</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>Facebook</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>Instagram</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>Amazon</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:14px'>Flipkart</p>\
    </div>\
    <div style='padding:16px 26px;background:#161b28;border:1px solid #2a3040'>\
      <p style='color:#e2e8f0;font-size:14px'>X</p>\
    </div>\
  </div>\
</div>\
<div style='padding:24px 40px'>\
  <p style='color:#8b95a5;font-size:13px;margin-bottom:14px'>Axomai AI Assistant</p>\
  <div style='padding:18px 22px;background:#0c1a3a;border:1px solid #1e3a6e'>\
    <p style='color:#60a5fa;font-size:17px;margin:0'>Hello! I am Axomai AI</p>\
    <p style='color:#5a6a80;font-size:13px;margin-top:6px'>Your intelligent browsing assistant for a better web experience.</p>\
  </div>\
  <div style='margin-top:8px;padding:11px 18px;background:#161b28;border:1px solid #2a3040'>\
    <p style='color:#8b95a5;font-size:13px'>Summarize this page</p>\
  </div>\
  <div style='margin-top:6px;padding:11px 18px;background:#161b28;border:1px solid #2a3040'>\
    <p style='color:#8b95a5;font-size:13px'>Explain like 5 year old</p>\
  </div>\
  <div style='margin-top:6px;padding:11px 18px;background:#161b28;border:1px solid #2a3040'>\
    <p style='color:#8b95a5;font-size:13px'>Translate to Assamese</p>\
  </div>\
  <div style='margin-top:6px;padding:11px 18px;background:#161b28;border:1px solid #2a3040'>\
    <p style='color:#8b95a5;font-size:13px'>Find similar content</p>\
  </div>\
  <div style='margin-top:6px;padding:11px 18px;background:#161b28;border:1px solid #2a3040'>\
    <p style='color:#8b95a5;font-size:13px'>Generate notes</p>\
  </div>\
</div>\
<div style='padding:20px 40px'>\
  <p style='color:#8b95a5;font-size:13px;margin-bottom:14px'>Quick Tools</p>\
  <div style='display:flex'>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:12px'>Reader Mode</p>\
    </div>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:12px'>AI Notes</p>\
    </div>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:12px'>Bookmarks</p>\
    </div>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:12px'>History</p>\
    </div>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040;margin-right:10px'>\
      <p style='color:#e2e8f0;font-size:12px'>QR Code</p>\
    </div>\
    <div style='padding:14px 20px;background:#161b28;border:1px solid #2a3040'>\
      <p style='color:#e2e8f0;font-size:12px'>More Tools</p>\
    </div>\
  </div>\
</div>\
<div style='padding:30px 40px'>\
  <p style='color:#3a4050;font-size:11px'>Axomai Browser v1.7.0 - Native GPU Engine - No WebView - Built with Rust</p>\
</div>\
</body></html>";
