use axomai_engine::AxomaiEngine;
use axomai_engine::NativeGpuCompositor;
use axomai_engine::WgpuRenderer;
use axomai_engine::glyph_atlas::GlyphInfo;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::WindowBuilder,
};
use tao::platform::windows::WindowExtWindows;
use wry::{Rect, WebViewBuilder};

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

    // Load home background image
    {
        let bg_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("home_bg.jpg");
        if bg_path.exists() {
            let img = image::open(&bg_path).expect("Failed to load home_bg.jpg").to_rgba8();
            gpu_renderer.upload_bg_image(img.width(), img.height(), img.as_raw());
            println!("[Axomai] Background image loaded: {}x{}", img.width(), img.height());
        }
    }

    let mut mouse_x: f32 = 0.0;
    let mut mouse_y: f32 = 0.0;
    let mut compositor = NativeGpuCompositor::new(size.width, size.height);

    // Load toolbar icons into glyph atlas
    let icon_size: u32 = 20;
    let icons_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons");
    let icon_back = {
        let img = image::open(icons_dir.join("arrow_back.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("back", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_forward = {
        let img = image::open(icons_dir.join("arrow_forward.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("forward", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_home = {
        let img = image::open(icons_dir.join("home.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("home", img.as_raw(), img.width(), img.height(), icon_size)
    };
    let icon_menu = {
        let img = image::open(icons_dir.join("menu_dots.png")).expect("icon").to_rgba8();
        compositor.glyph_atlas.blit_icon("menu", img.as_raw(), img.width(), img.height(), icon_size)
    };
    println!("[Axomai] Toolbar icons loaded ({}px)", icon_size);

    let mut address_bar_text = String::from("about:home");
    let mut address_bar_focused = false;
    let mut home_search_focused = false;
    let mut home_search_text = String::new();
    let mut ime_active = false;
    let mut needs_chrome_redraw = true;
    let mut sidebar_active: usize = 0;
    let mut is_home_page = true;
    let mut home_scroll_y: f32 = 0.0;
    let mut hover_sidebar_idx: Option<usize> = None;
    let mut is_settings_page = false;
    let mut selected_search_engine = SearchEngine::Google;
    let mut hover_engine_idx: Option<usize> = None;
    let mut menu_open = false;
    let mut hover_menu_idx: Option<usize> = None;

    let scale_factor = window.scale_factor() as f32;

    let nav_url_shared: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let mut webview: Option<wry::WebView> = None;
    let mut webview_visible = false;

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
                    if let Some(ref wv) = webview {
                        let _ = wv.set_bounds(Rect {
                            position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                            size: wry::dpi::LogicalSize::new(
                                (new_size.width as f32 / scale_factor - SIDEBAR_W) as u32,
                                (new_size.height as f32 / scale_factor - CHROME_TOP) as u32,
                            ).into(),
                        });
                    }
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
                    let engine_start_y = 130.0;
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
                // Menu hover tracking
                if menu_open {
                    let old_menu_hover = hover_menu_idx;
                    hover_menu_idx = None;
                    let dm_w = 240.0;
                    let dm_x = gpu_renderer.surface_config.width as f32 - dm_w - 20.0;
                    let dm_y = CHROME_TOP + 4.0;
                    for i in 0..8 {
                        let iy = dm_y + 8.0 + i as f32 * 38.0;
                        if mouse_x >= dm_x && mouse_x <= dm_x + dm_w
                            && mouse_y >= iy && mouse_y <= iy + 36.0
                        {
                            hover_menu_idx = Some(i);
                            break;
                        }
                    }
                    if old_menu_hover != hover_menu_idx {
                        needs_chrome_redraw = true;
                    }
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
                // Query actual cursor position from OS (CursorMoved events may not fire with automation)
                {
                    let hwnd = window.hwnd() as *mut std::ffi::c_void;
                    #[repr(C)]
                    struct POINT { x: i32, y: i32 }
                    extern "system" {
                        fn GetCursorPos(lp: *mut POINT) -> i32;
                        fn ScreenToClient(hwnd: *mut std::ffi::c_void, lp: *mut POINT) -> i32;
                    }
                    let mut pt = POINT { x: 0, y: 0 };
                    unsafe {
                        if GetCursorPos(&mut pt) != 0 {
                            ScreenToClient(hwnd, &mut pt);
                            mouse_x = pt.x as f32;
                            mouse_y = pt.y as f32;
                        }
                    }
                }
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
                                        if webview_visible {
                                            if let Some(ref wv) = webview { let _ = wv.set_visible(false); }
                                            webview_visible = false;
                                        }
                                    } else if i == 7 {
                                        is_settings_page = true;
                                        is_home_page = false;
                                        address_bar_text = String::from("about:settings");
                                        if webview_visible {
                                            if let Some(ref wv) = webview { let _ = wv.set_visible(false); }
                                            webview_visible = false;
                                        }
                                    } else {
                                        is_settings_page = false;
                                    }
                                    needs_chrome_redraw = true;
                                    break;
                                }
                                item_y += h;
                            }
                        }
                    } else if menu_open {
                        // Check if click is on a menu item
                        let menu_x = w - 260.0;
                        let menu_y_start = CHROME_TOP + 4.0;
                        let menu_w = 240.0;
                        let menu_items = ["Profile Management", "Dark Theme", "Light Theme", "Font Size +", "Font Size -", "Clear Memory", "Theme Management", "Settings"];
                        let mut clicked_item = None;
                        for (i, _item) in menu_items.iter().enumerate() {
                            let iy = menu_y_start + 8.0 + i as f32 * 38.0;
                            if mouse_x >= menu_x && mouse_x <= menu_x + menu_w
                                && mouse_y >= iy && mouse_y <= iy + 36.0
                            {
                                clicked_item = Some(i);
                                break;
                            }
                        }
                        menu_open = false;
                        if let Some(idx) = clicked_item {
                            match idx {
                                7 => {
                                    // Settings
                                    is_settings_page = true;
                                    is_home_page = false;
                                    sidebar_active = 7;
                                    address_bar_text = String::from("about:settings");
                                    if let Some(ref wv) = webview {
                                        let _ = wv.set_visible(false);
                                        webview_visible = false;
                                    }
                                }
                                _ => {}
                            }
                        }
                        needs_chrome_redraw = true;
                    } else if mouse_y < CHROME_TOP {
                        // 3-dot menu button
                        let menu_btn_x = w - 120.0 + 46.0;

                        if mouse_x >= menu_btn_x - 10.0 && mouse_x <= menu_btn_x + 20.0
                            && mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP
                        {
                            menu_open = !menu_open;

                            needs_chrome_redraw = true;
                        }
                        let addr_x = SIDEBAR_W + 60.0;
                        let addr_y = TAB_BAR_H + 7.0;
                        let addr_h = TOOLBAR_H - 14.0;
                        let addr_w = w - addr_x - 130.0;
                        if mouse_x >= addr_x
                            && mouse_x <= addr_x + addr_w
                            && mouse_y >= addr_y
                            && mouse_y <= addr_y + addr_h
                        {
                            address_bar_focused = true;
                            address_bar_text.clear();
                            needs_chrome_redraw = true;
                            // Steal focus back from WebView2
                            unsafe {
                                extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                SetFocus(window.hwnd() as _);
                            }
                        } else if mouse_y >= TAB_BAR_H {
                            let nav_base_x = SIDEBAR_W + 10.0;
                            if mouse_x >= nav_base_x && mouse_x <= nav_base_x + 22.0 {
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
                            } else if mouse_x >= nav_base_x + 28.0 && mouse_x <= nav_base_x + 50.0 {
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
                        menu_open = false;
                        let content_x = mouse_x - SIDEBAR_W;
                        let content_y = mouse_y - CHROME_TOP;
                        let card_x = 40.0;
                        let card_w = (w - SIDEBAR_W) - 80.0;
                        let engine_start_y = 130.0;
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
                    } else if is_home_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let content_w = w - SIDEBAR_W;
                        let content_h = gpu_renderer.surface_config.height as f32 - CHROME_TOP;
                        let cx = content_w / 2.0;
                        let cy = content_h / 2.0 - 60.0;
                        let search_w = 540.0f32.min(content_w - 80.0);
                        let search_x = SIDEBAR_W + cx - search_w / 2.0;
                        let search_y = CHROME_TOP + cy + 135.0;
                        if mouse_x >= search_x && mouse_x <= search_x + search_w
                            && mouse_y >= search_y && mouse_y <= search_y + 44.0
                        {
                            home_search_focused = true;
                            home_search_text.clear();
                            needs_chrome_redraw = true;
                            unsafe {
                                extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                SetFocus(window.hwnd() as _);
                            }
                        } else {
                            home_search_focused = false;
                            needs_chrome_redraw = true;
                        }
                    } else if !is_home_page {
                        address_bar_focused = false;
                        menu_open = false;
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
                                if !ime_active {
                                    address_bar_text.push_str(ch.as_ref());
                                    needs_chrome_redraw = true;
                                }
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
                                if webview.is_none() {
                                    let cw = w_of(&gpu_renderer);
                                    let ch = h_of(&gpu_renderer);
                                    let nav_clone = nav_url_shared.clone();
                                    webview = WebViewBuilder::new()
                                        .with_url(&url)
                                        .with_devtools(false)
                                        .with_initialization_script("new MutationObserver(()=>{document.querySelectorAll('[style*=\"non-commercial\"],.webview2-watermark,[class*=watermark]').forEach(e=>e.remove())}).observe(document.documentElement,{childList:true,subtree:true});")
                                        .with_bounds(Rect {
                                            position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                            size: wry::dpi::LogicalSize::new(
                                                (cw - SIDEBAR_W) as u32,
                                                (ch - CHROME_TOP) as u32,
                                            ).into(),
                                        })
                                        .with_navigation_handler(move |nav_url: String| {
                                            if let Ok(mut nav) = nav_clone.lock() {
                                                *nav = Some(nav_url);
                                            }
                                            true
                                        })
                                        .build_as_child(&window)
                                        .ok();
                                    webview_visible = webview.is_some();
                                    if webview.is_some() {
                                        println!("[Axomai] WebView2 initialized successfully");
                                    }
                                } else if let Some(ref wv) = webview {
                                    let _ = wv.load_url(&url);
                                    if !webview_visible {
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                }
                                address_bar_text = url;
                                is_home_page = false;
                                is_settings_page = false;
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
                    } else if home_search_focused {
                        match key_event.logical_key {
                            Key::Character(ref ch) => {
                                if !ime_active {
                                    home_search_text.push_str(ch.as_ref());
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::Backspace => {
                                home_search_text.pop();
                                needs_chrome_redraw = true;
                            }
                            Key::Enter => {
                                home_search_focused = false;
                                if !home_search_text.is_empty() {
                                    let url = if home_search_text.contains("://")
                                        || home_search_text.contains('.')
                                    {
                                        if !home_search_text.contains("://") {
                                            format!("https://{}", home_search_text)
                                        } else {
                                            home_search_text.clone()
                                        }
                                    } else {
                                        selected_search_engine.search_url(&home_search_text)
                                    };
                                    if webview.is_none() {
                                        let cw = w_of(&gpu_renderer);
                                        let ch = h_of(&gpu_renderer);
                                        let nav_clone = nav_url_shared.clone();
                                        webview = WebViewBuilder::new()
                                            .with_url(&url)
                                            .with_devtools(false)
                                            .with_initialization_script("new MutationObserver(()=>{document.querySelectorAll('[style*=\"non-commercial\"],.webview2-watermark,[class*=watermark]').forEach(e=>e.remove())}).observe(document.documentElement,{childList:true,subtree:true});")
                                            .with_bounds(Rect {
                                                position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                                size: wry::dpi::LogicalSize::new(
                                                    (cw - SIDEBAR_W) as u32,
                                                    (ch - CHROME_TOP) as u32,
                                                ).into(),
                                            })
                                            .with_navigation_handler(move |nav_url: String| {
                                                if let Ok(mut nav) = nav_clone.lock() {
                                                    *nav = Some(nav_url);
                                                }
                                                true
                                            })
                                            .build_as_child(&window)
                                            .ok();
                                        webview_visible = webview.is_some();
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&url);
                                        if !webview_visible {
                                            let _ = wv.set_visible(true);
                                            webview_visible = true;
                                        }
                                    }
                                    address_bar_text = url;
                                    is_home_page = false;
                                    is_settings_page = false;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::Escape => {
                                home_search_focused = false;
                                home_search_text.clear();
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else {
                        if is_home_page {
                            if let Key::Character(ref ch) = key_event.logical_key {
                                if !ime_active {
                                    home_search_focused = true;
                                    home_search_text.clear();
                                    home_search_text.push_str(ch.as_ref());
                                    needs_chrome_redraw = true;
                                    unsafe {
                                        extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                        SetFocus(window.hwnd() as _);
                                    }
                                }
                            }
                        } else if !is_home_page {
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
                event: WindowEvent::ReceivedImeText(ref text),
                ..
            } => {
                ime_active = true;
                if address_bar_focused {
                    address_bar_text.push_str(text);
                    needs_chrome_redraw = true;
                } else if home_search_focused {
                    home_search_text.push_str(text);
                    needs_chrome_redraw = true;
                } else if is_home_page {
                    home_search_focused = true;
                    home_search_text.clear();
                    home_search_text.push_str(text);
                    needs_chrome_redraw = true;
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
                if let Ok(mut nav) = nav_url_shared.lock() {
                    if let Some(url) = nav.take() {
                        address_bar_text = url;
                        needs_chrome_redraw = true;
                    }
                }

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

                    let toolbar_icons = [icon_back, icon_forward, icon_home, icon_menu];
                    let mut quads = build_chrome_quads(
                        &mut compositor, w, h, &address_bar_text, address_bar_focused,
                        &title, history_back, history_fwd, sidebar_active, hover_sidebar_idx,
                        menu_open, hover_menu_idx, &toolbar_icons,
                    );

                    if is_home_page {
                        let home_quads = build_home_page_quads(
                            &mut compositor, content_w, content_h, home_scroll_y,
                            home_search_focused, &home_search_text,
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

                    if menu_open {
                        quads.extend(build_dropdown_quads(&mut compositor, w, hover_menu_idx));
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
    menu_open: bool,
    hover_menu: Option<usize>,
    icons: &[GlyphInfo; 4], // [back, forward, home, menu]
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Chrome light theme colors
    let white = c(255, 255, 255, 255);
    let tab_bar_bg = c(222, 225, 230, 255);
    let toolbar_bg = white;
    let border = c(218, 220, 224, 255);
    let text_primary = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_disabled = c(155, 160, 168, 255);
    let blue = c(26, 115, 232, 255);
    let hover_bg = c(232, 234, 237, 200);
    let active_bg = c(210, 227, 252, 200);

    // === SIDEBAR (Glassmorphism gradient) ===
    // Gradient background: deep blue-purple to teal
    let bands = 8;
    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let r = (15.0 + t * 10.0) as u8;
        let g = (20.0 + t * 30.0) as u8;
        let b = (50.0 + t * 30.0) as u8;
        let band_h = viewport_h / bands as f32;
        quads.push(NativeGpuCompositor::solid_quad(0.0, i as f32 * band_h, SIDEBAR_W, band_h + 1.0, c(r, g, b, 255)));
    }
    // Glass overlay (frosted white)
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, SIDEBAR_W, viewport_h, c(255, 255, 255, 18)));
    // Right border glow
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W - 1.0, 0.0, 1.0, viewport_h, c(100, 140, 200, 80)));

    // Sidebar colors (light on dark glass)
    let sb_text = c(220, 225, 235, 255);
    let sb_text_dim = c(140, 155, 180, 255);
    let sb_accent = c(100, 180, 255, 255);
    let sb_divider = c(255, 255, 255, 20);
    let sb_hover = c(255, 255, 255, 20);
    let sb_active = c(100, 180, 255, 30);

    // Logo
    render_text(compositor, &mut quads, "Axomai", 16.0, 28.0, 16.0, sb_accent, SIDEBAR_W);
    render_text(compositor, &mut quads, "Browser", 88.0, 28.0, 10.0, sb_text_dim, SIDEBAR_W);
    quads.push(NativeGpuCompositor::solid_quad(12.0, 40.0, SIDEBAR_W - 24.0, 1.0, sb_divider));

    let mut item_y = 50.0;
    for (i, item) in SIDEBAR_ITEMS.iter().enumerate() {
        if item.is_section {
            item_y += 8.0;
            quads.push(NativeGpuCompositor::solid_quad(12.0, item_y, SIDEBAR_W - 24.0, 1.0, sb_divider));
            item_y += 10.0;
            render_text(compositor, &mut quads, item.label, 16.0, item_y + 12.0, 10.0, sb_text_dim, SIDEBAR_W);
            item_y += 22.0;
        } else {
            let is_active = i == sidebar_active;
            let is_hovered = hover_sidebar == Some(i);
            let h = 32.0;
            if is_active {
                quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, sb_active));
                // Left accent bar
                quads.push(NativeGpuCompositor::solid_quad(2.0, item_y + 6.0, 3.0, h - 12.0, sb_accent));
            } else if is_hovered {
                quads.push(rq(6.0, item_y, SIDEBAR_W - 12.0, h, 16.0, sb_hover));
            }
            let tc = if is_active { sb_accent } else { sb_text };
            render_text(compositor, &mut quads, item.icon, 18.0, item_y + 21.0, 13.0, tc, 36.0);
            render_text(compositor, &mut quads, item.label, 38.0, item_y + 21.0, 13.0, tc, SIDEBAR_W - 8.0);
            item_y += h + 1.0;
        }
    }
    item_y += 6.0;
    render_text(compositor, &mut quads, "+ Add Workspace", 18.0, item_y + 12.0, 11.0, sb_accent, SIDEBAR_W);

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

    // === TOOLBAR (Modern glassmorphism) ===
    let ty = TAB_BAR_H;
    let toolbar_w = viewport_w - SIDEBAR_W;
    // Glassmorphism toolbar background
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, toolbar_w, TOOLBAR_H, c(240, 243, 249, 245)));
    // Frosted overlay
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, toolbar_w, TOOLBAR_H, c(255, 255, 255, 60)));
    // Bottom glow line (gradient blue-purple)
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, CHROME_TOP - 1.5, toolbar_w * 0.5, 1.5, c(99, 132, 255, 50)));
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W + toolbar_w * 0.5, CHROME_TOP - 1.5, toolbar_w * 0.5, 1.5, c(168, 120, 255, 40)));

    // Navigation buttons (icon-based)
    let icon_s = 18.0;
    let nav_iy = ty + (TOOLBAR_H - icon_s) / 2.0;
    let nb = SIDEBAR_W + 10.0;
    let back_c = if history_back { c(60, 65, 75, 255) } else { c(180, 185, 195, 180) };
    let fwd_c = if history_fwd { c(60, 65, 75, 255) } else { c(180, 185, 195, 180) };
    quads.push(NativeGpuCompositor::icon_quad(nb, nav_iy, icon_s, &icons[0], back_c));
    quads.push(NativeGpuCompositor::icon_quad(nb + 28.0, nav_iy, icon_s, &icons[1], fwd_c));

    // Full-width address bar (modern glassmorphism pill)
    let ax = SIDEBAR_W + 60.0;
    let ay = ty + 7.0;
    let ah = TOOLBAR_H - 14.0;
    let right_icons_w = 120.0;
    let aw = viewport_w - ax - right_icons_w - 10.0;
    let bar_radius = ah / 2.0;

    if focused {
        // Focused: elevated glass with blue accent glow
        quads.push(rq(ax - 1.0, ay + 2.0, aw + 2.0, ah + 1.0, bar_radius + 1.0, c(99, 132, 255, 25)));
        quads.push(rq(ax - 1.5, ay - 1.5, aw + 3.0, ah + 3.0, bar_radius + 2.0, c(99, 132, 255, 120)));
        quads.push(rq(ax, ay, aw, ah, bar_radius, c(255, 255, 255, 252)));
    } else {
        // Unfocused: frosted glass pill
        quads.push(rq(ax, ay + 1.0, aw, ah, bar_radius, c(0, 0, 0, 6)));
        quads.push(rq(ax, ay, aw, ah, bar_radius, c(235, 238, 245, 220)));
        // Inner highlight at top
        quads.push(NativeGpuCompositor::solid_quad(ax + 8.0, ay + 1.0, aw - 16.0, 1.0, c(255, 255, 255, 100)));
    }

    // Search/lock icon (home icon in URL bar)
    let icon_y = ay + ah / 2.0 + 5.0;
    let url_icon_s = 14.0;
    let url_icon_y = ay + (ah - url_icon_s) / 2.0;
    quads.push(NativeGpuCompositor::icon_quad(ax + 10.0, url_icon_y, url_icon_s, &icons[2], c(130, 135, 150, 255)));

    let is_placeholder = (address_text == "about:home" || address_text == "about:settings") && !focused;
    let display = if is_placeholder { "Search or type a URL" } else { address_text };
    let dtc = if is_placeholder { c(150, 155, 168, 255) } else { c(40, 42, 50, 255) };
    let end_x = render_text(compositor, &mut quads, display, ax + 34.0, icon_y, 13.0, dtc, ax + aw - 14.0);

    if focused {
        quads.push(NativeGpuCompositor::solid_quad(end_x + 1.0, ay + 6.0, 1.5, ah - 12.0, c(99, 132, 255, 200)));
    }

    // Right toolbar icons (modern, compact)
    let icons_start = viewport_w - right_icons_w;
    let iy = ty + 10.0;
    let ih = TOOLBAR_H - 20.0;
    let icy = iy + ih / 2.0 + 4.0;

    // Profile avatar (glass circle)
    let prof_x = icons_start + 8.0;
    quads.push(rq(prof_x, iy + 1.0, ih, ih, ih / 2.0, c(99, 132, 255, 180)));
    render_text(compositor, &mut quads, "S", prof_x + 6.0, icy, 11.0, white, viewport_w);

    // Three-dot menu button (icon)
    let menu_x = icons_start + 46.0;
    let menu_icon_s = 18.0;
    let menu_icon_y = ty + (TOOLBAR_H - menu_icon_s) / 2.0;
    quads.push(NativeGpuCompositor::icon_quad(menu_x, menu_icon_y, menu_icon_s, &icons[3], c(80, 85, 100, 255)));

    quads
}

fn build_dropdown_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    hover_menu: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();
    let text_primary = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let dm_w = 240.0;
    let dm_x = viewport_w - dm_w - 20.0;
    let dm_y = CHROME_TOP + 4.0;
    let menu_items = [
        ("P", "Profile Management"),
        ("D", "Dark Theme"),
        ("L", "Light Theme"),
        ("F", "Font Size +"),
        ("f", "Font Size -"),
        ("C", "Clear Memory"),
        ("T", "Theme Management"),
        ("S", "Settings"),
    ];
    let dm_h = 8.0 + menu_items.len() as f32 * 38.0 + 8.0;
    quads.push(rq(dm_x + 3.0, dm_y + 3.0, dm_w, dm_h, 12.0, c(0, 0, 0, 40)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(255, 255, 255, 255)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(218, 220, 224, 30)));

    for (i, (icon, label)) in menu_items.iter().enumerate() {
        let iy = dm_y + 8.0 + i as f32 * 38.0;
        if hover_menu == Some(i) {
            quads.push(rq(dm_x + 4.0, iy, dm_w - 8.0, 36.0, 8.0, c(232, 234, 237, 255)));
        }
        render_text(compositor, &mut quads, icon, dm_x + 16.0, iy + 24.0, 14.0, text_secondary, dm_x + 36.0);
        render_text(compositor, &mut quads, label, dm_x + 40.0, iy + 24.0, 13.0, text_primary, dm_x + dm_w - 10.0);
        if i < menu_items.len() - 1 {
            quads.push(NativeGpuCompositor::solid_quad(dm_x + 12.0, iy + 36.0, dm_w - 24.0, 1.0, c(218, 220, 224, 60)));
        }
    }
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
    search_focused: bool,
    search_text: &str,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let blue = c(26, 115, 232, 255);

    // Background image (tea garden photo)
    quads.push(NativeGpuCompositor::bg_image_quad(0.0, 0.0, content_w, content_h));
    // Dark overlay for readability
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, c(0, 0, 0, 140)));

    // ---- Centered content ----
    let cx = content_w / 2.0;
    let cy = content_h / 2.0 - 60.0;

    // Logo icon
    let logo_size = 56.0;
    let logo_x = cx - logo_size / 2.0;
    quads.push(rq(logo_x, cy, logo_size, logo_size, 14.0, c(255, 255, 255, 40)));
    quads.push(rq(logo_x + 2.0, cy + 2.0, logo_size - 4.0, logo_size - 4.0, 12.0, blue));
    render_text_centered(compositor, &mut quads, "A", cx, cy + 42.0, 28.0, white);

    // Title
    render_text_centered(compositor, &mut quads, "Axomai Browser", cx, cy + 85.0, 26.0, white);
    render_text_centered(compositor, &mut quads, "Fast. Private. AI-Powered. Built for Everyone.", cx, cy + 112.0, 13.0, c(200, 210, 220, 200));

    // ---- Search bar (centered, Chrome-style pill with glass effect) ----
    let search_w = 540.0f32.min(content_w - 80.0);
    let search_x = cx - search_w / 2.0;
    let search_y = cy + 135.0;
    // Glass background (brighter border when focused)
    if search_focused {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(100, 160, 255, 80)));
    } else {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(255, 255, 255, 25)));
    }
    quads.push(rq(search_x + 1.0, search_y + 1.0, search_w - 2.0, 42.0, 21.0, c(30, 30, 30, 180)));
    // Search icon
    render_text(compositor, &mut quads, "G", search_x + 16.0, search_y + 30.0, 16.0, c(130, 180, 255, 255), search_x + 36.0);
    if search_focused && !search_text.is_empty() {
        render_text(compositor, &mut quads, search_text, search_x + 42.0, search_y + 28.0, 14.0, white, search_x + search_w - 40.0);
        // Cursor
        let cursor_x = search_x + 42.0 + search_text.len() as f32 * 8.0;
        quads.push(NativeGpuCompositor::solid_quad(cursor_x, search_y + 10.0, 2.0, 24.0, white));
    } else if search_focused {
        // Cursor only
        quads.push(NativeGpuCompositor::solid_quad(search_x + 42.0, search_y + 10.0, 2.0, 24.0, white));
    } else {
        render_text(compositor, &mut quads, "Search the web with Axomai AI...", search_x + 42.0, search_y + 28.0, 14.0, c(180, 185, 195, 200), search_x + search_w - 40.0);
    }
    render_text(compositor, &mut quads, "Q", search_x + search_w - 32.0, search_y + 28.0, 14.0, c(160, 165, 175, 200), search_x + search_w);

    quads
}
