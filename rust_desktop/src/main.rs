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

#[derive(Clone, Copy, PartialEq)]
enum SearchEngine {
    Google,
    Bing,
    Yahoo,
    DuckDuckGo,
}

impl SearchEngine {
    fn name(&self) -> &'static str {
        match self {
            SearchEngine::Google => "Google",
            SearchEngine::Bing => "Bing",
            SearchEngine::Yahoo => "Yahoo",
            SearchEngine::DuckDuckGo => "DuckDuckGo",
        }
    }
    fn search_url(&self, query: &str) -> String {
        match self {
            SearchEngine::Google => format!("https://www.google.com/search?q={}", query),
            SearchEngine::Bing => format!("https://www.bing.com/search?q={}", query),
            SearchEngine::Yahoo => format!("https://search.yahoo.com/search?p={}", query),
            SearchEngine::DuckDuckGo => format!("https://html.duckduckgo.com/html/?q={}", query),
        }
    }
    fn all() -> &'static [SearchEngine] {
        &[SearchEngine::Google, SearchEngine::Bing, SearchEngine::Yahoo, SearchEngine::DuckDuckGo]
    }
}

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
        .with_inner_size(LogicalSize::new(1200.0, 700.0))
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

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    let mut address_bar_text = String::from("about:home");
    let mut address_bar_focused = false;
    let mut needs_chrome_redraw = true;
    let mut sidebar_active: usize = 0;
    let mut is_home_page = true;
    let mut home_scroll_y: f32 = 0.0;
    let mut hover_sidebar_idx: Option<usize> = None;
    let mut is_settings_page = false;
    let mut selected_search_engine = SearchEngine::Google;
    let mut hover_engine_idx: Option<usize> = None;

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
                // Sidebar hover tracking
                let old_hover = hover_sidebar_idx;
                hover_sidebar_idx = None;
                if mouse_x < SIDEBAR_W {
                    let mut item_y = 50.0;
                    for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                        if item.is_section {
                            item_y += 41.0;
                        } else {
                            let h = 33.0;
                            if mouse_y >= item_y && mouse_y < item_y + h {
                                hover_sidebar_idx = Some(i);
                                break;
                            }
                            item_y += h;
                        }
                    }
                }
                // Settings page hover tracking
                if is_settings_page && mouse_x > SIDEBAR_W && mouse_y > CHROME_TOP {
                    let content_x = mouse_x - SIDEBAR_W;
                    let content_y = mouse_y - CHROME_TOP;
                    let old_engine_hover = hover_engine_idx;
                    hover_engine_idx = None;
                    let card_x = 40.0;
                    let card_w = (gpu_renderer.surface_config.width as f32 - SIDEBAR_W) - 80.0;
                    let engine_start_y = 100.0;
                    let engine_h = 56.0;
                    for i in 0..SearchEngine::all().len() {
                        let ey = engine_start_y + i as f32 * (engine_h + 8.0);
                        if content_x >= card_x && content_x <= card_x + card_w
                            && content_y >= ey && content_y <= ey + engine_h
                        {
                            hover_engine_idx = Some(i);
                            break;
                        }
                    }
                    if old_engine_hover != hover_engine_idx {
                        needs_chrome_redraw = true;
                    }
                } else {
                    hover_engine_idx = None;
                }
                if old_hover != hover_sidebar_idx {
                    needs_chrome_redraw = true;
                }
                if !is_home_page && mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_move(mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let w = gpu_renderer.surface_config.width as f32;
                if state == ElementState::Pressed && button == MouseButton::Left {
                    if mouse_x < SIDEBAR_W {
                        let mut item_y = 50.0;
                        for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
                            if item.is_section {
                                item_y += 41.0;
                            } else {
                                let h = 33.0;
                                if mouse_y >= item_y && mouse_y < item_y + h {
                                    sidebar_active = i;
                                    if i == 0 {
                                        is_home_page = true;
                                        is_settings_page = false;
                                        address_bar_text = String::from("about:home");
                                        home_scroll_y = 0.0;
                                    } else if i == 7 {
                                        is_settings_page = true;
                                        is_home_page = false;
                                        address_bar_text = String::from("about:settings");
                                    } else {
                                        is_settings_page = false;
                                    }
                                    needs_chrome_redraw = true;
                                    break;
                                }
                                item_y += h;
                            }
                        }
                    } else if mouse_y < CHROME_TOP {
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
                            if is_home_page {
                                address_bar_text.clear();
                            }
                            needs_chrome_redraw = true;
                        } else if mouse_y >= TAB_BAR_H {
                            let nav_base_x = SIDEBAR_W + 8.0;
                            if mouse_x >= nav_base_x && mouse_x <= nav_base_x + 32.0 {
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index > 0 {
                                        eng.history_index -= 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        is_home_page = false;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 36.0 && mouse_x <= nav_base_x + 68.0 {
                                if let Ok(mut eng) = engine.lock() {
                                    if eng.history_index + 1 < eng.history.len() {
                                        eng.history_index += 1;
                                        let entry = eng.history[eng.history_index].clone();
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        let _ = eng.load_url(&entry.url, content_w, content_h);
                                        address_bar_text = entry.url;
                                        is_home_page = false;
                                        needs_chrome_redraw = true;
                                    }
                                }
                            } else if mouse_x >= nav_base_x + 72.0 && mouse_x <= nav_base_x + 104.0 {
                                if !is_home_page {
                                    if let Ok(mut eng) = engine.lock() {
                                        let content_w = w - SIDEBAR_W;
                                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                                        if let Some(url) = eng.current_url.as_ref().map(|u| u.as_string()) {
                                            let _ = eng.load_url(&url, content_w, content_h);
                                        }
                                    }
                                }
                            }
                            address_bar_focused = false;
                            needs_chrome_redraw = true;
                        }
                    } else if is_settings_page {
                        address_bar_focused = false;
                        let content_x = mouse_x - SIDEBAR_W;
                        let content_y = mouse_y - CHROME_TOP;
                        let card_x = 40.0;
                        let card_w = (w - SIDEBAR_W) - 80.0;
                        let engine_start_y = 100.0;
                        let engine_h = 56.0;
                        for (i, eng_option) in SearchEngine::all().iter().enumerate() {
                            let ey = engine_start_y + i as f32 * (engine_h + 8.0);
                            if content_x >= card_x && content_x <= card_x + card_w
                                && content_y >= ey && content_y <= ey + engine_h
                            {
                                selected_search_engine = *eng_option;
                                needs_chrome_redraw = true;
                                break;
                            }
                        }
                    } else if !is_home_page {
                        address_bar_focused = false;
                        let btn = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 2,
                            MouseButton::Middle => 1,
                            _ => 0,
                        };
                        if let Ok(mut eng) = engine.lock() {
                            if let Some(nav_url) = eng.handle_pointer_down(
                                mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, btn,
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
                if state == ElementState::Released && !is_home_page && mouse_y > CHROME_TOP && mouse_x > SIDEBAR_W {
                    let btn = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 1,
                        _ => 0,
                    };
                    if let Ok(mut eng) = engine.lock() {
                        let _ = eng.handle_pointer_up(mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, btn);
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
                                    selected_search_engine.search_url(&address_bar_text)
                                } else {
                                    return;
                                };
                                if let Ok(mut eng) = engine.lock() {
                                    let content_w = w_of(&gpu_renderer) - SIDEBAR_W;
                                    let content_h = h_of(&gpu_renderer) - CHROME_TOP;
                                    let _ = eng.load_url(&url, content_w, content_h);
                                    address_bar_text = url;
                                    is_home_page = false;
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Escape => {
                                address_bar_focused = false;
                                if is_home_page {
                                    address_bar_text = String::from("about:home");
                                }
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else {
                        if !is_home_page {
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
                                    let content_w = w_of(&gpu_renderer) - SIDEBAR_W;
                                    let content_h = h_of(&gpu_renderer) - CHROME_TOP;
                                    let _ = eng.load_url(&nav_url, content_w, content_h);
                                    address_bar_text = nav_url;
                                    needs_chrome_redraw = true;
                                }
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
                    if is_home_page {
                        home_scroll_y = (home_scroll_y + dy).min(0.0).max(-800.0);
                        needs_chrome_redraw = true;
                    } else if let Ok(mut eng) = engine.lock() {
                        let content_w = w_of(&gpu_renderer) - SIDEBAR_W;
                        let content_h = h_of(&gpu_renderer) - CHROME_TOP;
                        let _ = eng.handle_scroll_at(
                            mouse_x - SIDEBAR_W, mouse_y - CHROME_TOP, 0.0, dy, content_w, content_h,
                        );
                    }
                }
            }
            Event::MainEventsCleared => {
                let w = w_of(&gpu_renderer);
                let h = h_of(&gpu_renderer);
                let content_w = w - SIDEBAR_W;
                let content_h = h - CHROME_TOP;

                let should_render = if is_home_page || is_settings_page {
                    needs_chrome_redraw || gpu_renderer.presented_frames < 3
                } else if let Ok(mut eng) = engine.lock() {
                    let updated = eng.process_event_loop(content_w, content_h);
                    updated || needs_chrome_redraw || gpu_renderer.presented_frames < 3
                } else {
                    false
                };

                if should_render {
                    needs_chrome_redraw = false;
                    compositor.width = content_w as u32;
                    compositor.height = content_h as u32;

                    let title = if is_home_page {
                        "Axomai Browser".to_string()
                    } else if is_settings_page {
                        "Settings".to_string()
                    } else if let Ok(eng) = engine.lock() {
                        eng.current_title.clone()
                    } else {
                        String::new()
                    };

                    let history_back = if let Ok(eng) = engine.lock() { eng.history_index > 0 } else { false };
                    let history_fwd = if let Ok(eng) = engine.lock() { eng.history_index + 1 < eng.history.len() } else { false };

                    let mut quads = build_chrome_quads(
                        &mut compositor, w, h, &address_bar_text, address_bar_focused,
                        &title, history_back, history_fwd, sidebar_active, hover_sidebar_idx,
                    );

                    if is_home_page {
                        let home_quads = build_home_page_quads(
                            &mut compositor, content_w, content_h, home_scroll_y,
                        );
                        for mut q in home_quads {
                            for v in &mut q.vertices {
                                v.position[0] += SIDEBAR_W;
                                v.position[1] += CHROME_TOP;
                            }
                            quads.push(q);
                        }
                    } else if is_settings_page {
                        let settings_quads = build_settings_page_quads(
                            &mut compositor, content_w, content_h,
                            selected_search_engine, hover_engine_idx,
                        );
                        for mut q in settings_quads {
                            for v in &mut q.vertices {
                                v.position[0] += SIDEBAR_W;
                                v.position[1] += CHROME_TOP;
                            }
                            quads.push(q);
                        }
                    } else {
                        if let Ok(eng) = engine.lock() {
                            let mut page_quads = compositor.extract_gpu_quads(&eng.display_list);
                            for q in &mut page_quads {
                                for v in &mut q.vertices {
                                    v.position[0] += SIDEBAR_W;
                                    v.position[1] += CHROME_TOP;
                                }
                            }
                            quads.extend(page_quads);
                        }
                    }

                    if compositor.glyph_atlas.dirty {
                        gpu_renderer.upload_glyph_atlas(&compositor.glyph_atlas);
                        compositor.glyph_atlas.dirty = false;
                    }

                    match gpu_renderer.render_frame(&quads) {
                        Ok(frame_idx) => {
                            if frame_idx % 300 == 1 {
                                println!("[Axomai GPU] Frame #{} — {} quads", frame_idx, quads.len());
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

                    window.set_title(&format!("Axomai Browser — {}", title));
                }
            }
            _ => {}
        }
    });
}

fn w_of(r: &WgpuRenderer) -> f32 { r.surface_config.width as f32 }
fn h_of(r: &WgpuRenderer) -> f32 { r.surface_config.height as f32 }

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
            quads.push(NativeGpuCompositor::text_quad(x + g.offset_x, y - g.offset_y - g.height, &g, color));
        }
        x += g.advance_width;
    }
    x
}

fn render_text_centered(
    compositor: &mut NativeGpuCompositor,
    quads: &mut Vec<GpuQuad>,
    text: &str,
    center_x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let mut total_w = 0.0f32;
    for ch in text.chars() {
        let g = compositor.glyph_atlas.rasterize(ch, size);
        total_w += g.advance_width;
    }
    let start_x = center_x - total_w / 2.0;
    render_text(compositor, quads, text, start_x, y, size, color, center_x + total_w);
}

// ===== CHROME (Tab bar + Toolbar + Sidebar) =====

fn rq(x: f32, y: f32, w: f32, h: f32, _r: f32, color: [f32; 4]) -> GpuQuad {
    NativeGpuCompositor::solid_quad(x, y, w, h, color)
}

fn build_chrome_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    viewport_h: f32,
    address_text: &str,
    focused: bool,
    title: &str,
    history_back: bool,
    history_fwd: bool,
    sidebar_active: usize,
    hover_sidebar: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Chrome light theme colors
    let white = c(255, 255, 255, 255);
    let tab_bar_bg = c(222, 225, 230, 255);
    let toolbar_bg = white;
    let sidebar_bg = c(248, 249, 250, 255);
    let border = c(218, 220, 224, 255);
    let text_primary = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_disabled = c(155, 160, 168, 255);
    let blue = c(26, 115, 232, 255);
    let hover_bg = c(232, 234, 237, 255);
    let active_bg = c(210, 227, 252, 255);

    // === SIDEBAR ===
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, SIDEBAR_W, viewport_h, sidebar_bg));
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W - 1.0, 0.0, 1.0, viewport_h, border));

    // Logo
    render_text(compositor, &mut quads, "Axomai", 16.0, 28.0, 16.0, blue, SIDEBAR_W);
    render_text(compositor, &mut quads, "Browser", 88.0, 28.0, 10.0, text_secondary, SIDEBAR_W);
    quads.push(NativeGpuCompositor::solid_quad(12.0, 40.0, SIDEBAR_W - 24.0, 1.0, border));

    let mut item_y = 50.0;
    for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
        if item.is_section {
            item_y += 8.0;
            quads.push(NativeGpuCompositor::solid_quad(12.0, item_y, SIDEBAR_W - 24.0, 1.0, border));
            item_y += 10.0;
            render_text(compositor, &mut quads, item.label, 16.0, item_y + 12.0, 10.0, text_disabled, SIDEBAR_W);
            item_y += 22.0;
        } else {
            let is_active = i == sidebar_active;
            let is_hovered = hover_sidebar == Some(i);
            let h = 32.0;
            if is_active {
                quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, active_bg));
            } else if is_hovered {
                quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, hover_bg));
            }
            let tc = if is_active { blue } else { text_primary };
            render_text(compositor, &mut quads, item.icon, 18.0, item_y + 21.0, 13.0, tc, 36.0);
            render_text(compositor, &mut quads, item.label, 38.0, item_y + 21.0, 13.0, tc, SIDEBAR_W - 8.0);
            item_y += h + 1.0;
        }
    }
    item_y += 6.0;
    render_text(compositor, &mut quads, "+ Add Workspace", 18.0, item_y + 12.0, 11.0, blue, SIDEBAR_W);

    // === TAB BAR (Chrome-style) ===
    // Tab strip background - slightly darker than toolbar
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, 0.0, viewport_w - SIDEBAR_W, TAB_BAR_H, tab_bar_bg));

    let tab_title = if title.is_empty() { "New Tab" } else { title };
    let tab_x = SIDEBAR_W + 8.0;
    let tab_w = 240.0;
    let tab_h = TAB_BAR_H - 8.0;
    // Active tab: white rounded top, flat bottom (connects to toolbar)
    quads.push(rq(tab_x, 8.0, tab_w, tab_h + 2.0, 8.0, white));
    // Bottom fill to merge tab into toolbar seamlessly
    quads.push(NativeGpuCompositor::solid_quad(tab_x, TAB_BAR_H - 2.0, tab_w, 2.0, white));
    // Favicon circle
    quads.push(rq(tab_x + 12.0, 15.0, 16.0, 16.0, 8.0, c(66, 133, 244, 255)));
    render_text(compositor, &mut quads, "A", tab_x + 15.0, 27.0, 10.0, white, tab_x + 30.0);
    // Tab title
    render_text(compositor, &mut quads, tab_title, tab_x + 34.0, 27.0, 12.0, text_primary, tab_x + tab_w - 30.0);
    // Close button (x)
    render_text(compositor, &mut quads, "x", tab_x + tab_w - 20.0, 27.0, 12.0, text_disabled, tab_x + tab_w);

    // + New tab button (circular)
    let plus_x = tab_x + tab_w + 8.0;
    render_text(compositor, &mut quads, "+", plus_x + 4.0, 27.0, 16.0, text_secondary, plus_x + 24.0);

    // === TOOLBAR (Chrome-style clean white) ===
    let ty = TAB_BAR_H;
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, viewport_w - SIDEBAR_W, TOOLBAR_H, toolbar_bg));
    // Thin border at bottom
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, CHROME_TOP - 1.0, viewport_w - SIDEBAR_W, 1.0, border));

    // Navigation buttons (Chrome-style: circular hover areas)
    let nav_y = ty + TOOLBAR_H / 2.0;
    let nb = SIDEBAR_W + 16.0;
    let back_c = if history_back { text_primary } else { c(189, 193, 198, 255) };
    let fwd_c = if history_fwd { text_primary } else { c(189, 193, 198, 255) };
    // Back arrow
    render_text(compositor, &mut quads, "<", nb + 2.0, nav_y + 6.0, 16.0, back_c, nb + 20.0);
    // Forward arrow
    render_text(compositor, &mut quads, ">", nb + 30.0, nav_y + 6.0, 16.0, fwd_c, nb + 50.0);
    // Refresh
    render_text(compositor, &mut quads, "R", nb + 60.0, nav_y + 5.0, 13.0, text_secondary, nb + 78.0);
    // Home button
    render_text(compositor, &mut quads, "H", nb + 88.0, nav_y + 5.0, 13.0, text_secondary, nb + 106.0);

    // Address bar (Chrome-style: wide rounded pill, centered feel)
    let ax = SIDEBAR_W + 130.0;
    let ay = ty + 6.0;
    let ah = TOOLBAR_H - 12.0;
    let aw = viewport_w - ax - 140.0;
    if focused {
        // Focused: white bg with blue border
        quads.push(rq(ax, ay, aw, ah, ah / 2.0, blue));
        quads.push(rq(ax + 2.0, ay + 2.0, aw - 4.0, ah - 4.0, (ah - 4.0) / 2.0, white));
    } else {
        // Unfocused: subtle gray pill
        quads.push(rq(ax, ay, aw, ah, ah / 2.0, c(241, 243, 244, 255)));
    }

    // Search/lock icon
    let icon_c = if focused { text_secondary } else { text_secondary };
    render_text(compositor, &mut quads, "O", ax + 14.0, ay + ah / 2.0 + 6.0, 13.0, icon_c, ax + 30.0);

    let is_placeholder = (address_text == "about:home" || address_text == "about:settings") && !focused;
    let display = if is_placeholder { "Search Google or type a URL" } else { address_text };
    let dtc = if is_placeholder { c(154, 160, 166, 255) } else { text_primary };
    let end_x = render_text(compositor, &mut quads, display, ax + 34.0, ay + ah / 2.0 + 6.0, 14.0, dtc, ax + aw - 14.0);

    if focused {
        quads.push(NativeGpuCompositor::solid_quad(end_x + 1.0, ay + 6.0, 1.5, ah - 12.0, blue));
    }

    // Right toolbar icons (Chrome-style, compact)
    let iy = ty + 11.0;
    let ih = TOOLBAR_H - 22.0;
    let icy = iy + ih / 2.0 + 5.0;

    // AI button (branded pill)
    let ai_x = viewport_w - 120.0;
    quads.push(rq(ai_x, iy, 40.0, ih, ih / 2.0, blue));
    render_text(compositor, &mut quads, "AI", ai_x + 12.0, icy, 12.0, white, viewport_w);
    // Profile avatar circle
    let prof_x = viewport_w - 70.0;
    quads.push(rq(prof_x, iy, ih, ih, ih / 2.0, c(138, 180, 248, 255)));
    render_text(compositor, &mut quads, "S", prof_x + 6.0, icy, 11.0, white, viewport_w);
    // Three-dot menu
    let menu_x = viewport_w - 36.0;
    render_text(compositor, &mut quads, ":", menu_x, icy, 18.0, text_secondary, viewport_w);

    quads
}

// ===== SETTINGS PAGE =====

fn build_settings_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    selected: SearchEngine,
    hover_idx: Option<usize>,
) -> Vec<GpuQuad> {
    use axomai_engine::GpuQuad;
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let page_bg = c(246, 247, 248, 255);
    let text_dark = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let blue = c(26, 115, 232, 255);
    let border = c(218, 220, 224, 255);
    let hover_bg = c(241, 243, 244, 255);
    let selected_bg = c(210, 227, 252, 255);
    let green = c(24, 128, 56, 255);

    // Background
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));

    // Header
    render_text(compositor, &mut quads, "Settings", 40.0, 40.0, 24.0, text_dark, content_w);
    quads.push(NativeGpuCompositor::solid_quad(40.0, 55.0, content_w - 80.0, 1.0, border));

    // Section title
    render_text(compositor, &mut quads, "Search Engine", 40.0, 88.0, 16.0, text_dark, content_w);
    render_text(compositor, &mut quads, "Choose the search engine used in the address bar", 40.0, 108.0, 12.0, text_secondary, content_w);

    // Search engine cards
    let card_x = 40.0;
    let card_w = (content_w - 80.0).min(500.0);
    let engine_h = 56.0;
    let start_y = 130.0;

    let engine_descriptions: &[&str] = &[
        "The world's most popular search engine",
        "Microsoft's search engine with AI features",
        "A classic search engine by Yahoo Inc.",
        "Privacy-focused search, no tracking",
    ];

    for (i, eng) in SearchEngine::all().iter().enumerate() {
        let ey = start_y + i as f32 * (engine_h + 8.0);
        let is_selected = *eng == selected;
        let is_hovered = hover_idx == Some(i);

        let bg = if is_selected {
            selected_bg
        } else if is_hovered {
            hover_bg
        } else {
            white
        };

        quads.push(rq(card_x, ey, card_w, engine_h, 10.0, bg));

        // Radio circle
        let radio_x = card_x + 20.0;
        let radio_y = ey + engine_h / 2.0;
        quads.push(rq(radio_x - 9.0, radio_y - 9.0, 18.0, 18.0, 9.0, if is_selected { blue } else { border }));
        quads.push(rq(radio_x - 7.0, radio_y - 7.0, 14.0, 14.0, 7.0, if is_selected { blue } else { white }));
        if is_selected {
            quads.push(rq(radio_x - 4.0, radio_y - 4.0, 8.0, 8.0, 4.0, white));
        }

        // Engine name
        let name_x = card_x + 48.0;
        render_text(compositor, &mut quads, eng.name(), name_x, ey + 24.0, 14.0, text_dark, card_x + card_w);

        // Description
        render_text(compositor, &mut quads, engine_descriptions[i], name_x, ey + 42.0, 11.0, text_secondary, card_x + card_w - 10.0);

        // Selected badge
        if is_selected {
            let badge_x = card_x + card_w - 80.0;
            render_text(compositor, &mut quads, "Default", badge_x, ey + 32.0, 11.0, green, card_x + card_w);
        }
    }

    // Info text at bottom
    let info_y = start_y + 4.0 * (engine_h + 8.0) + 10.0;
    render_text(compositor, &mut quads, "Click on a search engine to set it as default.", 40.0, info_y + 14.0, 11.0, text_secondary, content_w);
    render_text(compositor, &mut quads, "The selected engine is used when you type in the address bar.", 40.0, info_y + 30.0, 11.0, text_secondary, content_w);

    quads
}

// ===== HOME PAGE (drawn entirely via GPU quads — no HTML engine) =====

fn build_home_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    scroll_y: f32,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Light theme colors
    let white = c(255, 255, 255, 255);
    let page_bg = c(246, 247, 248, 255);
    let text_dark = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_light = c(155, 160, 168, 255);
    let blue = c(26, 115, 232, 255);
    let card_bg = white;
    let border = c(218, 220, 224, 255);

    // Background
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));

    let cx = content_w / 2.0;
    let base_y = scroll_y;

    // ---- Hero section (white) ----
    quads.push(NativeGpuCompositor::solid_quad(0.0, base_y, content_w, 280.0, white));
    quads.push(NativeGpuCompositor::solid_quad(0.0, base_y + 279.0, content_w, 1.0, border));

    // Location / weather widget (top right of hero)
    let wr_x = content_w - 200.0;
    render_text(compositor, &mut quads, "Guwahati", wr_x, base_y + 30.0, 12.0, text_secondary, content_w);
    render_text(compositor, &mut quads, "28 C  Partly Cloudy", wr_x, base_y + 48.0, 11.0, text_light, content_w);

    // Logo icon
    let logo_size = 56.0;
    let logo_x = cx - logo_size / 2.0;
    quads.push(rq(logo_x, base_y + 40.0, logo_size, logo_size, 14.0, blue));
    render_text_centered(compositor, &mut quads, "A", cx, base_y + 82.0, 28.0, white);

    // Title
    render_text_centered(compositor, &mut quads, "Axomai Browser", cx, base_y + 125.0, 26.0, text_dark);
    render_text_centered(compositor, &mut quads, "Fast. Private. AI-Powered. Built for Everyone.", cx, base_y + 152.0, 13.0, text_secondary);

    // ---- Search bar (centered, Chrome-style pill) ----
    let search_w = 540.0f32.min(content_w - 80.0);
    let search_x = cx - search_w / 2.0;
    let search_y = base_y + 175.0;
    quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, white));
    // Shadow simulation (slightly darker border)
    quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(0, 0, 0, 18)));
    quads.push(rq(search_x + 1.0, search_y + 1.0, search_w - 2.0, 42.0, 21.0, white));
    // Search icon
    render_text(compositor, &mut quads, "G", search_x + 16.0, search_y + 30.0, 16.0, blue, search_x + 36.0);
    // Placeholder
    render_text(compositor, &mut quads, "Search the web with Axomai AI...", search_x + 42.0, search_y + 28.0, 14.0, text_light, search_x + search_w - 40.0);
    render_text(compositor, &mut quads, "Q", search_x + search_w - 32.0, search_y + 28.0, 14.0, text_secondary, search_x + search_w);

    // ---- Quick Links row ----
    let links = ["YouTube", "Google", "Facebook", "Instagram", "X", "Amazon", "Flipkart", "+"];
    let link_colors: &[[u8; 3]] = &[
        [255, 0, 0],
        [66, 133, 244],
        [24, 119, 242],
        [225, 48, 108],
        [100, 100, 110],
        [255, 153, 0],
        [255, 209, 0],
        [26, 115, 232],
    ];
    let link_count = links.len() as f32;
    let link_box_w = 72.0;
    let link_gap = 14.0;
    let total_links_w = link_count * link_box_w + (link_count - 1.0) * link_gap;
    let links_start_x = cx - total_links_w / 2.0;
    let links_y = base_y + 240.0;

    for (i, label) in links.iter().enumerate() {
        let lx = links_start_x + i as f32 * (link_box_w + link_gap);
        let ly = links_y;
        let icon_size = 44.0;
        let icon_x = lx + (link_box_w - icon_size) / 2.0;
        // Light circle background
        quads.push(rq(icon_x, ly, icon_size, icon_size, 22.0, c(241, 243, 244, 255)));
        let cc = link_colors[i];
        quads.push(rq(icon_x + 6.0, ly + 6.0, icon_size - 12.0, icon_size - 12.0, 16.0, c(cc[0], cc[1], cc[2], 220)));
        let first_char = &label[..1];
        render_text_centered(compositor, &mut quads, first_char, lx + link_box_w / 2.0, ly + 30.0, 16.0, white);
        render_text_centered(compositor, &mut quads, label, lx + link_box_w / 2.0, ly + 58.0, 10.0, text_secondary);
    }

    // ---- Category tabs ----
    let tabs_y = links_y + 76.0;
    let tab_labels = ["Top Sites", "News", "Technology", "Assam", "AI Tools", "Coding"];
    let mut tx = 30.0;
    for (i, label) in tab_labels.iter().enumerate() {
        let tc = if i == 0 { blue } else { text_secondary };
        let end = render_text(compositor, &mut quads, label, tx, tabs_y + 16.0, 13.0, tc, content_w - 30.0);
        if i == 0 {
            quads.push(NativeGpuCompositor::solid_quad(tx, tabs_y + 22.0, end - tx, 2.0, blue));
        }
        tx = end + 24.0;
    }
    quads.push(NativeGpuCompositor::solid_quad(30.0, tabs_y + 26.0, content_w - 60.0, 1.0, border));

    // ---- Content cards and Quick Tools ----
    let cards_y = tabs_y + 40.0;
    let cards_w = (content_w - 90.0) * 0.6;
    let tools_x = 30.0 + cards_w + 30.0;
    let tools_w = content_w - tools_x - 30.0;

    // Card 1
    let card_h = 100.0;
    quads.push(rq(30.0, cards_y, cards_w, card_h, 10.0, card_bg));
    quads.push(rq(44.0, cards_y + 12.0, 60.0, 20.0, 4.0, c(26, 115, 232, 255)));
    render_text(compositor, &mut quads, "Assam", 50.0, cards_y + 26.0, 10.0, white, 100.0);
    render_text(compositor, &mut quads, "Kaziranga National Park sees", 44.0, cards_y + 52.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "rise in tourist footfall this season", 44.0, cards_y + 72.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "10 hours ago", 44.0, cards_y + 90.0, 10.0, text_light, 200.0);

    // Card 2
    let card2_y = cards_y + card_h + 10.0;
    quads.push(rq(30.0, card2_y, cards_w, card_h, 10.0, card_bg));
    quads.push(rq(44.0, card2_y + 12.0, 60.0, 20.0, 4.0, c(22, 163, 74, 255)));
    render_text(compositor, &mut quads, "Wildlife", 48.0, card2_y + 26.0, 10.0, white, 110.0);
    render_text(compositor, &mut quads, "One-Horned Rhino population", 44.0, card2_y + 52.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "shows positive growth", 44.0, card2_y + 72.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "12 hours ago", 44.0, card2_y + 90.0, 10.0, text_light, 200.0);

    // Card 3
    let card3_y = card2_y + card_h + 10.0;
    quads.push(rq(30.0, card3_y, cards_w, card_h, 10.0, card_bg));
    quads.push(rq(44.0, card3_y + 12.0, 60.0, 20.0, 4.0, c(168, 85, 247, 255)));
    render_text(compositor, &mut quads, "Culture", 50.0, card3_y + 26.0, 10.0, white, 110.0);
    render_text(compositor, &mut quads, "Bihu Festival 2026: Dates,", 44.0, card3_y + 52.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "Events and Travel Guide", 44.0, card3_y + 72.0, 14.0, text_dark, 30.0 + cards_w - 10.0);
    render_text(compositor, &mut quads, "1 day ago", 44.0, card3_y + 90.0, 10.0, text_light, 200.0);

    // ---- Quick Tools (right column) ----
    render_text(compositor, &mut quads, "Quick Tools", tools_x, cards_y + 16.0, 14.0, text_dark, tools_x + tools_w);
    render_text(compositor, &mut quads, "View All ->", tools_x + tools_w - 80.0, cards_y + 16.0, 11.0, blue, tools_x + tools_w);

    let tool_labels = [
        "Word to PDF", "PDF to Word", "Image to PDF",
        "PDF to JPG", "PDF to PNG", "Image Convert",
        "AI Notes", "QR Code", "More Tools",
    ];
    let cols = 3;
    let tool_box_w = (tools_w - 16.0) / cols as f32;
    let tool_box_h = 50.0;

    for (i, label) in tool_labels.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let bx = tools_x + col as f32 * (tool_box_w + 4.0);
        let by = cards_y + 30.0 + row as f32 * (tool_box_h + 4.0);
        quads.push(rq(bx, by, tool_box_w - 4.0, tool_box_h, 8.0, card_bg));
        quads.push(rq(bx + (tool_box_w - 4.0) / 2.0 - 10.0, by + 8.0, 20.0, 20.0, 10.0, c(232, 240, 254, 255)));
        render_text_centered(compositor, &mut quads, label, bx + (tool_box_w - 4.0) / 2.0, by + 42.0, 10.0, text_secondary);
    }

    // ---- AI Assistant panel ----
    let ai_y = card3_y + card_h + 30.0;
    let ai_w = content_w - 60.0;
    quads.push(rq(30.0, ai_y, ai_w, 200.0, 12.0, card_bg));

    quads.push(rq(44.0, ai_y + 14.0, 32.0, 32.0, 16.0, blue));
    render_text(compositor, &mut quads, "A", 53.0, ai_y + 38.0, 16.0, white, 72.0);
    render_text(compositor, &mut quads, "Hello! I'm Axomai AI", 86.0, ai_y + 30.0, 16.0, text_dark, 30.0 + ai_w);
    render_text(compositor, &mut quads, "Your intelligent browsing assistant for a better web experience.", 86.0, ai_y + 50.0, 12.0, text_secondary, 30.0 + ai_w);

    let actions = ["Summarize this page", "Explain like 5 year old", "Translate to Assamese", "Find similar content", "Generate notes"];
    for (i, action) in actions.iter().enumerate() {
        let ay_btn = ai_y + 68.0 + i as f32 * 26.0;
        quads.push(rq(44.0, ay_btn, ai_w - 28.0, 22.0, 6.0, c(241, 243, 244, 255)));
        render_text(compositor, &mut quads, action, 56.0, ay_btn + 15.0, 12.0, text_secondary, 30.0 + ai_w - 10.0);
    }

    // ---- Footer ----
    let footer_y = ai_y + 220.0;
    render_text_centered(compositor, &mut quads, "Axomai Browser v1.7.0 - Native GPU Engine - No WebView - Built with Rust", cx, footer_y + 14.0, 10.0, text_light);

    quads
}
