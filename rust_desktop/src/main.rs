mod internal_pages;
pub mod b64_assets;
pub mod types;
pub mod storage;
pub mod extensions;
pub mod pages;
pub mod chrome_js;
pub mod rendering;

use types::{SearchEngine, DesktopTab, SIDEBAR_ITEMS, SIDEBAR_W, TAB_BAR_H, TOOLBAR_H, CHROME_TOP};
use extensions::{create_extensions, build_extension_init_script};

use axomai_engine::AxomaiEngine;
use axomai_engine::NativeGpuCompositor;
use axomai_engine::WgpuRenderer;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::{CursorIcon, WindowBuilder},
};
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use wry::{Rect, WebViewBuilder};

use chrome_js::{CHROME_DROPDOWN_JS, CHROME_EXT_DROPDOWN_JS};
use rendering::{w_of, h_of, build_chrome_quads, build_dropdown_quads};


fn serde_json_mini(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn url_encode_mini(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn page_to_file_url(filename: &str, html: &str) -> String {
    let temp_path = std::env::temp_dir().join(filename);
    let _ = std::fs::write(&temp_path, html);
    format!("file:///{}", temp_path.to_string_lossy().replace('\\', "/"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--subprocess" {
        axomai_engine::run_subprocess(&args[2]);
        return Ok(());
    }

    let engine = Arc::new(Mutex::new(AxomaiEngine::new()));

    let browser_storage = match storage::BrowserStorage::new() {
        Ok(s) => {
            println!("[Axomai] SQLite storage initialized");
            Some(s)
        }
        Err(e) => {
            eprintln!("[Axomai] Storage init failed (non-fatal): {}", e);
            None
        }
    };

    let selected_search_engine_from_db = browser_storage.as_ref()
        .and_then(|s| s.get_setting("search_engine").ok().flatten())
        .and_then(|v| match v.as_str() {
            "Bing" => Some(SearchEngine::Bing),
            "Yahoo" => Some(SearchEngine::Yahoo),
            "DuckDuckGo" => Some(SearchEngine::DuckDuckGo),
            _ => Some(SearchEngine::Google),
        });

    let event_loop = EventLoop::new();

    let icon_data = {
        let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons").join("axomai_logo.png");
        if icon_path.exists() {
            if let Ok(img) = image::open(&icon_path) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                tao::window::Icon::from_rgba(rgba.into_raw(), w, h).ok()
            } else {
                None
            }
        } else {
            None
        }
    };

    let mut wb = WindowBuilder::new()
        .with_title("Axomai Browser")
        .with_inner_size(LogicalSize::new(1200.0, 700.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));
    if let Some(icon) = icon_data {
        wb = wb.with_window_icon(Some(icon));
    }
    let window = wb.build(&event_loop)?;

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
    let mut address_bar_cursor: usize = 0;
    let mut address_bar_focused = false;
    let mut home_search_focused = false;
    let mut home_search_text = String::new();
    let mut home_search_cursor: usize = 0;
    let mut _ime_active = false;
    let mut needs_chrome_redraw = true;
    let mut sidebar_active: usize = 0;
    let mut is_home_page = true;
    let mut home_scroll_y: f32 = 0.0;
    let mut hover_sidebar_idx: Option<usize> = None;
    let mut is_settings_page = false;
    let mut selected_search_engine = selected_search_engine_from_db.unwrap_or(SearchEngine::Google);
    let mut hover_engine_idx: Option<usize> = None;
    let mut menu_open = false;
    let mut hover_menu_idx: Option<usize> = None;
    let mut is_extensions_page = false;
    let mut extensions = create_extensions();
    let mut hover_ext_idx: Option<usize> = None;
    let mut ext_inject_time: Option<std::time::Instant> = None;

    let (mut tabs, mut active_tab_idx) = {
        let mut restored = false;
        let mut t: Vec<DesktopTab> = Vec::new();
        let mut idx: usize = 0;
        if let Some(ref s) = browser_storage {
            if let Ok(Some(val)) = s.get_setting("restore_session") {
                if val == "true" {
                    if let Ok(saved) = s.get_tabs() {
                        if !saved.is_empty() {
                            for st in &saved {
                                let is_home = st.url == "about:home";
                                t.push(DesktopTab {
                                    title: st.title.clone(),
                                    url: st.url.clone(),
                                    is_home,
                                    is_extensions: st.url == "axomai://extensions",
                                    is_settings: st.url == "axomai://settings",
                                });
                                if st.is_active {
                                    idx = t.len() - 1;
                                }
                            }
                            restored = true;
                        }
                    }
                }
            }
        }
        if !restored {
            t.push(DesktopTab {
                title: String::from("Axomai Browser"),
                url: String::from("about:home"),
                is_home: true,
                is_extensions: false,
                is_settings: false,
            });
        }
        (t, idx)
    };

    let scale_factor = window.scale_factor() as f32;

    let nav_url_shared: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let mut webview: Option<wry::WebView> = None;
    let mut webview_visible = false;
    let mut load_internal_page: Option<String> = Some("home".to_string());

    let mut browser_storage = browser_storage;
    let mut modifiers = tao::keyboard::ModifiersState::empty();

    let download_dir = {
        let d = dirs_download();
        let _ = std::fs::create_dir_all(&d);
        d
    };
    let download_signal: Arc<Mutex<Vec<(String, String, bool)>>> = Arc::new(Mutex::new(Vec::new()));

    let mut is_loading = false;
    let mut loading_progress: f32 = 0.0;

    let mut find_bar_open = false;
    let mut find_text = String::new();

    #[allow(unused_assignments)]
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        );

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Some(ref s) = browser_storage {
                    let saved: Vec<storage::SavedTab> = tabs.iter().enumerate().map(|(i, t)| {
                        storage::SavedTab {
                            position: i as i32,
                            url: t.url.clone(),
                            title: t.title.clone(),
                            is_active: i == active_tab_idx,
                        }
                    }).collect();
                    let _ = s.save_tabs(&saved);
                }
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
                // Extensions page hover tracking
                if is_extensions_page && mouse_x > SIDEBAR_W && mouse_y > CHROME_TOP {
                    let content_x = mouse_x - SIDEBAR_W;
                    let content_y = mouse_y - CHROME_TOP;
                    let content_w = gpu_renderer.surface_config.width as f32 - SIDEBAR_W;
                    let old_ext_hover = hover_ext_idx;
                    hover_ext_idx = None;
                    let padding = 24.0;
                    let gap = 16.0;
                    let cols = if content_w > 600.0 { 2usize } else { 1 };
                    let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
                    let card_h = 160.0;
                    let grid_start_y = 64.0 + 24.0 + 36.0;
                    for i in 0..extensions.len() {
                        let col = (i % cols) as f32;
                        let row = (i / cols) as f32;
                        let cx = padding + col * (card_w + gap);
                        let cy = grid_start_y + row * (card_h + gap);
                        if content_x >= cx && content_x <= cx + card_w
                            && content_y >= cy && content_y <= cy + card_h {
                            hover_ext_idx = Some(i);
                            break;
                        }
                    }
                    if old_ext_hover != hover_ext_idx { needs_chrome_redraw = true; }
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
                    let dm_w = 250.0;
                    let dm_x = gpu_renderer.surface_config.width as f32 - dm_w - 16.0;
                    let dm_y = CHROME_TOP + 4.0;
                    for i in 0..11 {
                        let iy = dm_y + 8.0 + i as f32 * 36.0;
                        if mouse_x >= dm_x && mouse_x <= dm_x + dm_w
                            && mouse_y >= iy && mouse_y <= iy + 34.0
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

                // Chrome-style cursor pointer effect (Hand tool / Text / Default)
                let mut cursor_icon = CursorIcon::Default;
                let w = gpu_renderer.surface_config.width as f32;

                if mouse_x < SIDEBAR_W {
                    if hover_sidebar_idx.is_some() || mouse_y > 450.0 {
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if mouse_y <= TAB_BAR_H {
                    // Over tab bar
                    let available_w = w - SIDEBAR_W - 80.0;
                    let tab_count = tabs.len().max(1);
                    let tab_w = ((available_w - 40.0) / tab_count as f32).clamp(110.0, 200.0);
                    let plus_x = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0) + 4.0;
                    let tab_bar_end = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0);
                    if mouse_x >= SIDEBAR_W + 8.0 && mouse_x <= tab_bar_end {
                        cursor_icon = CursorIcon::Hand;
                    } else if mouse_x >= plus_x - 4.0 && mouse_x <= plus_x + 36.0 {
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if mouse_y <= CHROME_TOP {
                    // Over toolbar
                    let nb = SIDEBAR_W + 10.0;
                    let n_enabled_ext = extensions.iter().filter(|e| e.enabled).count() as f32;
                    let right_icons_w = 120.0 + n_enabled_ext * 30.0;
                    let ax = SIDEBAR_W + 88.0;
                    let aw = w - ax - right_icons_w - 10.0;

                    if mouse_x >= nb && mouse_x <= nb + 84.0 {
                        // Back / Forward / Home buttons
                        cursor_icon = CursorIcon::Hand;
                    } else if mouse_x >= ax && mouse_x <= ax + aw {
                        // Address input
                        cursor_icon = CursorIcon::Text;
                    } else if mouse_x > ax + aw {
                        // Extensions / Menu icon
                        cursor_icon = CursorIcon::Hand;
                    }
                } else if menu_open && hover_menu_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                } else if is_settings_page && hover_engine_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                } else if is_extensions_page && hover_ext_idx.is_some() {
                    cursor_icon = CursorIcon::Hand;
                }

                window.set_cursor_icon(cursor_icon);
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let w = gpu_renderer.surface_config.width as f32;
                // Accurate client mouse coordinates via ScreenToClient
                #[cfg(target_os = "windows")]
                {
                    unsafe {
                        extern "system" {
                            fn GetCursorPos(point: *mut [i32; 2]) -> i32;
                            fn ScreenToClient(hwnd: *mut std::ffi::c_void, point: *mut [i32; 2]) -> i32;
                        }
                        let mut pt: [i32; 2] = [0, 0];
                        GetCursorPos(&mut pt);
                        ScreenToClient(window.hwnd() as _, &mut pt);
                        mouse_x = pt[0] as f32;
                        mouse_y = pt[1] as f32;
                    }
                }
                if state == ElementState::Pressed && button == MouseButton::Left {
                    if SIDEBAR_W > 0.0 && mouse_x < SIDEBAR_W {
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
                                        is_extensions_page = false;
                                        address_bar_text = String::from("about:home");
                                        home_scroll_y = 0.0;
                                        load_internal_page = Some("home".to_string());
                                    } else if i == 5 {
                                        is_extensions_page = true;
                                        is_home_page = false;
                                        is_settings_page = false;
                                        address_bar_text = String::from("axomai://extensions");
                                        load_internal_page = Some("extensions".to_string());
                                    } else if i == 7 {
                                        is_settings_page = true;
                                        is_home_page = false;
                                        is_extensions_page = false;
                                        address_bar_text = String::from("about:settings");
                                        load_internal_page = Some("settings".to_string());
                                    } else {
                                        is_settings_page = false;
                                        is_extensions_page = false;
                                    }
                                    needs_chrome_redraw = true;
                                    break;
                                }
                                item_y += h;
                            }
                        }
                    } else if menu_open {
                        // Check if click is on a menu item
                        let menu_x = w - 266.0;
                        let menu_y_start = CHROME_TOP + 4.0;
                        let menu_w = 250.0;
                        let menu_items = [
                            "New Tab", "Home", "Bookmarks", "History",
                            "Downloads", "Extensions", "Passwords",
                            "Heritage Themes", "Clear RAM & Cache", "Settings", "Exit"
                        ];
                        let mut clicked_item = None;
                        for (i, _item) in menu_items.iter().enumerate() {
                            let iy = menu_y_start + 8.0 + i as f32 * 36.0;
                            if mouse_x >= menu_x && mouse_x <= menu_x + menu_w
                                && mouse_y >= iy && mouse_y <= iy + 34.0
                            {
                                clicked_item = Some(i);
                                break;
                            }
                        }
                        menu_open = false;
                        if let Some(ref wv) = webview {
                            let _ = wv.set_visible(!is_home_page && !is_settings_page && !is_extensions_page);
                        }
                        if let Some(idx) = clicked_item {
                            match idx {
                                0 => {
                                    // New Tab
                                    tabs.push(DesktopTab {
                                        title: String::from("New Tab"),
                                        url: String::from("about:home"),
                                        is_home: true,
                                        is_extensions: false,
                                        is_settings: false,
                                    });
                                    active_tab_idx = tabs.len() - 1;
                                    is_home_page = true;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("about:home");
                                    load_internal_page = Some("home".to_string());
                                }
                                1 => {
                                    // Home
                                    is_home_page = true;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("about:home");
                                    load_internal_page = Some("home".to_string());
                                }
                                2 => {
                                    // Bookmarks
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("axomai://bookmarks");
                                    load_internal_page = Some("bookmarks".to_string());
                                }
                                3 => {
                                    // History
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("axomai://history");
                                    load_internal_page = Some("history".to_string());
                                }
                                4 => {
                                    // Downloads
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("axomai://downloads");
                                    load_internal_page = Some("downloads".to_string());
                                }
                                5 => {
                                    // Extensions
                                    is_extensions_page = true;
                                    is_home_page = false;
                                    is_settings_page = false;
                                    sidebar_active = 5;
                                    address_bar_text = String::from("axomai://extensions");
                                    load_internal_page = Some("extensions".to_string());
                                }
                                6 => {
                                    // Passwords
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    is_settings_page = false;
                                    address_bar_text = String::from("axomai://passwords");
                                    load_internal_page = Some("home".to_string());
                                }
                                7 => {
                                    // Themes
                                    is_settings_page = true;
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    address_bar_text = String::from("about:settings");
                                    load_internal_page = Some("settings".to_string());
                                }
                                8 => {
                                    // Clear RAM
                                    println!("[Axomai] RAM & Cache cleared");
                                }
                                9 => {
                                    // Settings
                                    is_settings_page = true;
                                    is_home_page = false;
                                    is_extensions_page = false;
                                    sidebar_active = 7;
                                    address_bar_text = String::from("about:settings");
                                    load_internal_page = Some("settings".to_string());
                                }
                                10 => {
                                    // Exit
                                    *control_flow = ControlFlow::Exit;
                                }
                                _ => {}
                            }
                        }
                        needs_chrome_redraw = true;
                    } else if mouse_y < CHROME_TOP {
                        let available_w = w - SIDEBAR_W - 80.0;
                        let tab_count = tabs.len().max(1);
                        let tab_w = ((available_w - 40.0) / tab_count as f32).clamp(110.0, 200.0);
                        let mut tab_action = None; // (index, is_close)

                        if mouse_y <= TAB_BAR_H {
                            for (i, _) in tabs.iter().enumerate() {
                                let tab_x = SIDEBAR_W + 8.0 + i as f32 * (tab_w + 4.0);
                                let close_x = tab_x + tab_w - 24.0;
                                if mouse_x >= close_x && mouse_x <= tab_x + tab_w + 2.0 {
                                    tab_action = Some((i, true));
                                    break;
                                } else if mouse_x >= tab_x && mouse_x < close_x {
                                    tab_action = Some((i, false));
                                    break;
                                }
                            }

                            let plus_x = SIDEBAR_W + 8.0 + tabs.len() as f32 * (tab_w + 4.0) + 4.0;
                            if mouse_x >= plus_x - 4.0 && mouse_x <= plus_x + 36.0 {
                                // + New Tab
                                tabs.push(DesktopTab {
                                    title: String::from("New Tab"),
                                    url: String::from("about:home"),
                                    is_home: true,
                                    is_extensions: false,
                                    is_settings: false,
                                });
                                active_tab_idx = tabs.len() - 1;
                                is_home_page = true;
                                is_extensions_page = false;
                                is_settings_page = false;
                                address_bar_text = String::from("about:home");
                                home_search_text.clear();
                                home_search_focused = false;
                                load_internal_page = Some("home".to_string());
                                needs_chrome_redraw = true;
                            } else if let Some((i, is_close)) = tab_action {
                                if is_close {
                                    if tabs.len() > 1 {
                                        tabs.remove(i);
                                        if active_tab_idx >= tabs.len() {
                                            active_tab_idx = tabs.len() - 1;
                                        } else if active_tab_idx > i {
                                            active_tab_idx -= 1;
                                        }
                                    } else {
                                        tabs[0] = DesktopTab {
                                            title: String::from("Axomai Browser"),
                                            url: String::from("about:home"),
                                            is_home: true,
                                            is_extensions: false,
                                            is_settings: false,
                                        };
                                        active_tab_idx = 0;
                                    }
                                    let cur = &tabs[active_tab_idx];
                                    is_home_page = cur.is_home;
                                    is_extensions_page = cur.is_extensions;
                                    is_settings_page = cur.is_settings;
                                    address_bar_text = cur.url.clone();
                                    if cur.is_home {
                                        load_internal_page = Some("home".to_string());
                                    } else if cur.is_extensions {
                                        load_internal_page = Some("extensions".to_string());
                                    } else if cur.is_settings {
                                        load_internal_page = Some("settings".to_string());
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&cur.url);
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                    needs_chrome_redraw = true;
                                } else {
                                    // Switch tab
                                    active_tab_idx = i;
                                    let cur = &tabs[active_tab_idx];
                                    is_home_page = cur.is_home;
                                    is_extensions_page = cur.is_extensions;
                                    is_settings_page = cur.is_settings;
                                    address_bar_text = cur.url.clone();
                                    if cur.is_home {
                                        load_internal_page = Some("home".to_string());
                                    } else if cur.is_extensions {
                                        load_internal_page = Some("extensions".to_string());
                                    } else if cur.is_settings {
                                        load_internal_page = Some("settings".to_string());
                                    } else if let Some(ref wv) = webview {
                                        let _ = wv.load_url(&cur.url);
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                    needs_chrome_redraw = true;
                                }
                            }
                        }
                        // Toolbar right controls (Profile + Extensions + 3-Dot Menu)
                        let n_ext = extensions.iter().filter(|e| e.enabled).count() as f32;
                        let right_w = 110.0 + n_ext * 30.0;
                        let icons_start_x = w - right_w;
                        let prof_end_x = icons_start_x + 8.0 + (TOOLBAR_H - 20.0) + 8.0;

                        let menu_btn_x = w - 46.0;
                        let puzzle_btn_x = w - 82.0;

                        // Extensions Puzzle Button click (top-right of toolbar)
                        if mouse_x >= puzzle_btn_x && mouse_x < menu_btn_x
                            && mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP
                        {
                            if let Some(ref wv) = webview {
                                let _ = wv.evaluate_script(CHROME_EXT_DROPDOWN_JS);
                            } else {
                                is_extensions_page = true;
                                is_home_page = false;
                                is_settings_page = false;
                                sidebar_active = 5;
                                address_bar_text = String::from("axomai://extensions");
                                load_internal_page = Some("extensions".to_string());
                            }
                            needs_chrome_redraw = true;
                        }

                        // 3-dot menu button area (top-right of toolbar)
                        if mouse_x >= menu_btn_x && mouse_x <= w
                            && mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP
                        {
                            if let Some(ref wv) = webview {
                                let _ = wv.evaluate_script(CHROME_DROPDOWN_JS);
                            } else {
                                menu_open = !menu_open;
                            }
                            needs_chrome_redraw = true;
                        }
                        // Click on extension icon in toolbar to toggle it off
                        if mouse_y >= TAB_BAR_H && mouse_y <= CHROME_TOP {
                            let mut ex = prof_end_x;
                            let ih = TOOLBAR_H - 20.0;
                            for i in 0..extensions.len() {
                                if extensions[i].enabled {
                                    if mouse_x >= ex && mouse_x <= ex + ih {
                                        extensions[i].enabled = false;
                                        if let Some(ref wv) = webview {
                                            let _ = wv.evaluate_script(extensions[i].disable_js);
                                        }
                                        needs_chrome_redraw = true;
                                        break;
                                    }
                                    ex += 30.0;
                                }
                            }
                        }
                        let addr_x = SIDEBAR_W + 88.0;
                        let addr_y = TAB_BAR_H + 7.0;
                        let addr_h = TOOLBAR_H - 14.0;
                        let addr_w = w - addr_x - right_w - 10.0;
                        if mouse_x >= addr_x
                            && mouse_x <= addr_x + addr_w
                            && mouse_y >= addr_y
                            && mouse_y <= addr_y + addr_h
                        {
                            address_bar_focused = true;
                            find_bar_open = false;
                            address_bar_text.clear();
                            address_bar_cursor = 0;
                            needs_chrome_redraw = true;
                            #[cfg(target_os = "windows")]
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
                            } else if mouse_x >= nav_base_x + 52.0 && mouse_x <= nav_base_x + 78.0 {
                                is_home_page = true;
                                is_settings_page = false;
                                is_extensions_page = false;
                                address_bar_text = String::from("about:home");
                                home_scroll_y = 0.0;
                                home_search_focused = false;
                                home_search_text.clear();
                                home_search_cursor = 0;
                                sidebar_active = 0;
                                if let Some(ref wv) = webview {
                                    let _ = wv.set_visible(false);
                                    webview_visible = false;
                                }
                                load_internal_page = Some("home".to_string());
                            }
                            if address_bar_focused {
                                address_bar_focused = false;
                                address_bar_text = tabs[active_tab_idx].url.clone();
                            }
                            needs_chrome_redraw = true;
                        }
                    } else if is_extensions_page {
                        address_bar_focused = false;
                        menu_open = false;
                        let content_x = mouse_x - SIDEBAR_W;
                        let content_y = mouse_y - CHROME_TOP;
                        let content_w = w - SIDEBAR_W;
                        let padding = 24.0;
                        let gap = 16.0;
                        let cols = if content_w > 600.0 { 2usize } else { 1 };
                        let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
                        let card_h = 160.0;
                        let grid_start_y = 64.0 + 24.0 + 36.0;
                        for i in 0..extensions.len() {
                            let col = (i % cols) as f32;
                            let row = (i / cols) as f32;
                            let cx = padding + col * (card_w + gap);
                            let cy = grid_start_y + row * (card_h + gap);
                            if content_x >= cx && content_x <= cx + card_w
                                && content_y >= cy && content_y <= cy + card_h
                            {
                                extensions[i].enabled = !extensions[i].enabled;
                                let is_on = extensions[i].enabled;
                                let js = if is_on { extensions[i].inject_js } else { extensions[i].disable_js };
                                if let Some(ref wv) = webview {
                                    let _ = wv.evaluate_script(js);
                                }
                                needs_chrome_redraw = true;
                                break;
                            }
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
                            #[cfg(target_os = "windows")]
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
                event: WindowEvent::ModifiersChanged(new_modifiers),
                ..
            } => {
                modifiers = new_modifiers;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event: key_event, .. },
                ..
            } => {
                if key_event.state == ElementState::Pressed {
                    let ctrl = modifiers.control_key();
                    if ctrl {
                        match key_event.logical_key {
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("t") => {
                                tabs.push(DesktopTab {
                                    title: String::from("New Tab"),
                                    url: String::from("about:home"),
                                    is_home: true,
                                    is_extensions: false,
                                    is_settings: false,
                                });
                                active_tab_idx = tabs.len() - 1;
                                address_bar_text = String::from("about:home");
                                is_home_page = true;
                                is_settings_page = false;
                                is_extensions_page = false;
                                home_search_focused = false;
                                home_search_text.clear();
                                address_bar_focused = false;
                                if let Some(ref wv) = webview {
                                    let _ = wv.set_visible(false);
                                    webview_visible = false;
                                }
                                load_internal_page = Some("home".to_string());
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("w") => {
                                if tabs.len() > 1 {
                                    tabs.remove(active_tab_idx);
                                    if active_tab_idx >= tabs.len() {
                                        active_tab_idx = tabs.len() - 1;
                                    }
                                } else {
                                    tabs[0] = DesktopTab {
                                        title: String::from("Axomai Browser"),
                                        url: String::from("about:home"),
                                        is_home: true,
                                        is_extensions: false,
                                        is_settings: false,
                                    };
                                    active_tab_idx = 0;
                                }
                                let cur = &tabs[active_tab_idx];
                                address_bar_text = cur.url.clone();
                                is_home_page = cur.is_home;
                                is_extensions_page = cur.is_extensions;
                                is_settings_page = cur.is_settings;
                                address_bar_focused = false;
                                home_search_focused = false;
                                if is_home_page || is_extensions_page || is_settings_page {
                                    if let Some(ref wv) = webview {
                                        let _ = wv.set_visible(false);
                                        webview_visible = false;
                                    }
                                    let page = if is_extensions_page { "extensions" } else if is_settings_page { "settings" } else { "home" };
                                    load_internal_page = Some(page.to_string());
                                } else if let Some(ref wv) = webview {
                                    let _ = wv.load_url(&cur.url);
                                    if !webview_visible {
                                        let _ = wv.set_visible(true);
                                        webview_visible = true;
                                    }
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("l") => {
                                address_bar_focused = true;
                                find_bar_open = false;
                                address_bar_text.clear();
                                address_bar_cursor = 0;
                                needs_chrome_redraw = true;
                                #[cfg(target_os = "windows")]
                                unsafe {
                                    extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                    SetFocus(window.hwnd() as _);
                                }
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("d") => {
                                if let Some(ref s) = browser_storage {
                                    if !is_home_page && !address_bar_text.is_empty()
                                        && !address_bar_text.starts_with("about:")
                                        && !address_bar_text.starts_with("axomai://")
                                    {
                                        let title = tabs[active_tab_idx].title.clone();
                                        let _ = s.add_bookmark(&address_bar_text, &title, "Unsorted");
                                    }
                                }
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("r") => {
                                if let Some(ref wv) = webview {
                                    let _ = wv.load_url(&address_bar_text);
                                }
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("h") => {
                                load_internal_page = Some("history".to_string());
                                address_bar_text = String::from("axomai://history");
                                is_home_page = false;
                                is_settings_page = false;
                                is_extensions_page = false;
                                if let Some(ref wv) = webview {
                                    let _ = wv.set_visible(false);
                                    webview_visible = false;
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("j") => {
                                load_internal_page = Some("downloads".to_string());
                                address_bar_text = String::from("axomai://downloads");
                                is_home_page = false;
                                is_settings_page = false;
                                is_extensions_page = false;
                                if let Some(ref wv) = webview {
                                    let _ = wv.set_visible(false);
                                    webview_visible = false;
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if ch.eq_ignore_ascii_case("f") => {
                                find_bar_open = !find_bar_open;
                                if !find_bar_open {
                                    find_text.clear();
                                    if let Some(ref wv) = webview {
                                        let _ = wv.evaluate_script("window.getSelection().removeAllRanges();");
                                    }
                                }
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else if find_bar_open {
                        match key_event.logical_key {
                            Key::Backspace => {
                                find_text.pop();
                                if let Some(ref wv) = webview {
                                    let escaped = find_text.replace('\\', "\\\\").replace('\'', "\\'");
                                    let _ = wv.evaluate_script(&format!("window.find('{}')", escaped));
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Enter => {
                                if let Some(ref wv) = webview {
                                    let escaped = find_text.replace('\\', "\\\\").replace('\'', "\\'");
                                    let _ = wv.evaluate_script(&format!("window.find('{}')", escaped));
                                }
                            }
                            Key::Escape => {
                                find_bar_open = false;
                                find_text.clear();
                                if let Some(ref wv) = webview {
                                    let _ = wv.evaluate_script("window.getSelection().removeAllRanges();");
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if !key_event.repeat => {
                                find_text.push_str(ch);
                                if let Some(ref wv) = webview {
                                    let escaped = find_text.replace('\\', "\\\\").replace('\'', "\\'");
                                    let _ = wv.evaluate_script(&format!("window.find('{}')", escaped));
                                }
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else if address_bar_focused {
                        match key_event.logical_key {
                            Key::Backspace => {
                                if address_bar_cursor > 0 {
                                    let byte_pos = address_bar_text.char_indices()
                                        .nth(address_bar_cursor - 1).map(|(i, _)| i);
                                    let byte_end = address_bar_text.char_indices()
                                        .nth(address_bar_cursor).map(|(i, _)| i)
                                        .unwrap_or(address_bar_text.len());
                                    if let Some(start) = byte_pos {
                                        address_bar_text.replace_range(start..byte_end, "");
                                        address_bar_cursor -= 1;
                                    }
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::ArrowLeft => {
                                if address_bar_cursor > 0 {
                                    address_bar_cursor -= 1;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::ArrowRight => {
                                let char_count = address_bar_text.chars().count();
                                if address_bar_cursor < char_count {
                                    address_bar_cursor += 1;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::Home => {
                                address_bar_cursor = 0;
                                needs_chrome_redraw = true;
                            }
                            Key::End => {
                                address_bar_cursor = address_bar_text.chars().count();
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
                                    let init_js = build_extension_init_script(&extensions);
                                    webview = WebViewBuilder::new()
                                        .with_url(&url)
                                        .with_devtools(false)
                                        .with_initialization_script(&init_js)
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
                                        .with_download_started_handler({
                                            let dl_dir = download_dir.clone();
                                            move |url, path| {
                                                let fname = url.rsplit('/').next().unwrap_or("download").split('?').next().unwrap_or("download");
                                                let fname = if fname.is_empty() { "download" } else { fname };
                                                *path = dl_dir.join(fname);
                                                true
                                            }
                                        })
                                        .with_download_completed_handler({
                                            let dl_sig = download_signal.clone();
                                            move |url, path, success| {
                                                let fname = path.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_default();
                                                if let Ok(mut sig) = dl_sig.lock() {
                                                    sig.push((url, fname, success));
                                                }
                                            }
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
                                is_extensions_page = false;
                                needs_chrome_redraw = true;
                            }
                            Key::Escape => {
                                address_bar_focused = false;
                                address_bar_text = tabs[active_tab_idx].url.clone();
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if !key_event.repeat => {
                                let byte_pos = address_bar_text.char_indices()
                                    .nth(address_bar_cursor).map(|(i, _)| i)
                                    .unwrap_or(address_bar_text.len());
                                address_bar_text.insert_str(byte_pos, ch);
                                address_bar_cursor += ch.chars().count();
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else if home_search_focused {
                        match key_event.logical_key {
                            Key::Backspace => {
                                if home_search_cursor > 0 {
                                    let byte_pos = home_search_text.char_indices()
                                        .nth(home_search_cursor - 1).map(|(i, _)| i);
                                    let byte_end = home_search_text.char_indices()
                                        .nth(home_search_cursor).map(|(i, _)| i)
                                        .unwrap_or(home_search_text.len());
                                    if let Some(start) = byte_pos {
                                        home_search_text.replace_range(start..byte_end, "");
                                        home_search_cursor -= 1;
                                    }
                                }
                                needs_chrome_redraw = true;
                            }
                            Key::ArrowLeft => {
                                if home_search_cursor > 0 {
                                    home_search_cursor -= 1;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::ArrowRight => {
                                let char_count = home_search_text.chars().count();
                                if home_search_cursor < char_count {
                                    home_search_cursor += 1;
                                    needs_chrome_redraw = true;
                                }
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
                                        let init_js = build_extension_init_script(&extensions);
                                        webview = WebViewBuilder::new()
                                            .with_url(&url)
                                            .with_devtools(false)
                                            .with_initialization_script(&init_js)
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
                                            .with_download_started_handler({
                                                let dl_dir = download_dir.clone();
                                                move |url, path| {
                                                    let fname = url.rsplit('/').next().unwrap_or("download").split('?').next().unwrap_or("download");
                                                    let fname = if fname.is_empty() { "download" } else { fname };
                                                    *path = dl_dir.join(fname);
                                                    true
                                                }
                                            })
                                            .with_download_completed_handler({
                                                let dl_sig = download_signal.clone();
                                                move |url, path, success| {
                                                    let fname = path.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_default();
                                                    if let Ok(mut sig) = dl_sig.lock() {
                                                        sig.push((url, fname, success));
                                                    }
                                                }
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
                                    is_extensions_page = false;
                                    needs_chrome_redraw = true;
                                }
                            }
                            Key::Escape => {
                                home_search_focused = false;
                                home_search_text.clear();
                                needs_chrome_redraw = true;
                            }
                            Key::Character(ref ch) if !key_event.repeat => {
                                let byte_pos = home_search_text.char_indices()
                                    .nth(home_search_cursor).map(|(i, _)| i)
                                    .unwrap_or(home_search_text.len());
                                home_search_text.insert_str(byte_pos, ch);
                                home_search_cursor += ch.chars().count();
                                needs_chrome_redraw = true;
                            }
                            _ => {}
                        }
                    } else {
                        if is_home_page {
                            if let Key::Character(ref ch) = key_event.logical_key {
                                if !key_event.repeat {
                                    home_search_focused = true;
                                    home_search_text.clear();
                                    home_search_text.push_str(ch);
                                    home_search_cursor = ch.chars().count();
                                    needs_chrome_redraw = true;
                                    #[cfg(target_os = "windows")]
                                    unsafe {
                                        extern "system" { fn SetFocus(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
                                        SetFocus(window.hwnd() as _);
                                    }
                                }
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
                // All text input handled by Key::Character in KeyboardInput
                let _ = text;
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
                        if url.starts_with("axomai://newtab") {
                            tabs.push(DesktopTab {
                                title: String::from("New Tab"),
                                url: String::from("about:home"),
                                is_home: true,
                                is_extensions: false,
                                is_settings: false,
                            });
                            active_tab_idx = tabs.len() - 1;
                            is_home_page = true;
                            is_extensions_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("about:home");
                            load_internal_page = Some("home".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://home") {
                            is_home_page = true;
                            is_extensions_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("about:home");
                            load_internal_page = Some("home".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://extensions") {
                            is_extensions_page = true;
                            is_home_page = false;
                            is_settings_page = false;
                            address_bar_text = String::from("axomai://extensions");
                            load_internal_page = Some("extensions".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://settings") || url.starts_with("axomai://themes") {
                            is_settings_page = true;
                            is_home_page = false;
                            is_extensions_page = false;
                            address_bar_text = String::from("about:settings");
                            load_internal_page = Some("settings".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://exit") {
                            *control_flow = ControlFlow::Exit;
                        } else if url.starts_with("axomai://ext-toggle/") {
                            if let Ok(idx) = url.trim_start_matches("axomai://ext-toggle/").parse::<usize>() {
                                if idx < extensions.len() {
                                    extensions[idx].enabled = !extensions[idx].enabled;
                                    if is_extensions_page {
                                        load_internal_page = Some("extensions".to_string());
                                    }
                                    needs_chrome_redraw = true;
                                }
                            }
                        } else if url.starts_with("axomai://set-engine/") {
                            let name = url.trim_start_matches("axomai://set-engine/");
                            selected_search_engine = match name {
                                "Bing" => SearchEngine::Bing,
                                "Yahoo" => SearchEngine::Yahoo,
                                "DuckDuckGo" => SearchEngine::DuckDuckGo,
                                _ => SearchEngine::Google,
                            };
                            if let Some(ref s) = browser_storage {
                                let _ = s.set_setting("search_engine", selected_search_engine.name());
                            }
                            if is_settings_page {
                                load_internal_page = Some("settings".to_string());
                            }
                        } else if url.starts_with("axomai://set-restore-session/") {
                            let val = url.trim_start_matches("axomai://set-restore-session/");
                            if let Some(ref s) = browser_storage {
                                let _ = s.set_setting("restore_session", val);
                            }
                            if is_settings_page {
                                load_internal_page = Some("settings".to_string());
                            }
                        } else if url.contains("axomai_home.html") || url == "about:home" || url == "axomai://home" || url == "axomai://newtab" {
                            address_bar_text = String::from("about:home");
                            is_home_page = true;
                            is_extensions_page = false;
                            is_settings_page = false;
                            needs_chrome_redraw = true;
                        } else if url.contains("axomai_history.html") {
                            address_bar_text = String::from("axomai://history");
                            needs_chrome_redraw = true;
                        } else if url.contains("axomai_bookmarks.html") {
                            address_bar_text = String::from("axomai://bookmarks");
                            needs_chrome_redraw = true;
                        } else if url.contains("axomai_downloads.html") {
                            address_bar_text = String::from("axomai://downloads");
                            needs_chrome_redraw = true;
                        } else if url.contains("axomai_extensions.html") || url == "axomai://extensions" {
                            address_bar_text = String::from("axomai://extensions");
                            is_extensions_page = true;
                            is_home_page = false;
                            is_settings_page = false;
                            needs_chrome_redraw = true;
                        } else if url.contains("axomai_settings.html") || url == "axomai://settings" || url == "about:settings" || url == "axomai://themes" {
                            address_bar_text = String::from("about:settings");
                            is_settings_page = true;
                            is_home_page = false;
                            is_extensions_page = false;
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://bookmarks") {
                            address_bar_text = String::from("axomai://bookmarks");
                            is_home_page = false;
                            is_extensions_page = false;
                            is_settings_page = false;
                            load_internal_page = Some("bookmarks".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://history") {
                            address_bar_text = String::from("axomai://history");
                            is_home_page = false;
                            is_extensions_page = false;
                            is_settings_page = false;
                            load_internal_page = Some("history".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://downloads") {
                            address_bar_text = String::from("axomai://downloads");
                            is_home_page = false;
                            is_extensions_page = false;
                            is_settings_page = false;
                            load_internal_page = Some("downloads".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://clear-history") {
                            if let Some(ref s) = browser_storage {
                                let _ = s.clear_history();
                                println!("[Axomai] History cleared");
                            }
                            load_internal_page = Some("history".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://clear-downloads") {
                            if let Some(ref s) = browser_storage {
                                let _ = s.clear_downloads();
                                println!("[Axomai] Downloads cleared");
                            }
                            load_internal_page = Some("downloads".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://remove-bookmark/") {
                            if let Ok(id) = url.trim_start_matches("axomai://remove-bookmark/").parse::<i64>() {
                                if let Some(ref s) = browser_storage {
                                    let _ = s.remove_bookmark(id);
                                    println!("[Axomai] Bookmark {} removed", id);
                                }
                            }
                            load_internal_page = Some("bookmarks".to_string());
                            needs_chrome_redraw = true;
                        } else if url.starts_with("axomai://add-bookmark") {
                            if let Some(ref s) = browser_storage {
                                let bookmark_url = &address_bar_text;
                                let title_str = if let Ok(eng) = engine.lock() {
                                    eng.current_title.clone()
                                } else {
                                    String::new()
                                };
                                let _ = s.add_bookmark(bookmark_url, &title_str, "Unsorted");
                                println!("[Axomai] Bookmarked: {}", bookmark_url);
                            }
                            needs_chrome_redraw = true;
                        } else if url.starts_with("data:") || (url.starts_with("file:") && url.contains("axomai_")) {
                            // Internal page temp file/data URL — don't update address bar
                        } else {
                            if let Some(ref s) = browser_storage {
                                let _ = s.add_history(&url, "");
                            }
                            address_bar_text = url;
                            is_home_page = false;
                            is_extensions_page = false;
                            is_settings_page = false;
                            is_loading = true;
                            loading_progress = 0.0;
                            needs_chrome_redraw = true;
                            let has_auto_ext = extensions.iter().any(|e| e.enabled && e.auto_inject);
                            if has_auto_ext {
                                ext_inject_time = Some(std::time::Instant::now() + std::time::Duration::from_millis(1200));
                            }
                        }
                    }
                }
                if let Some(page) = load_internal_page.take() {
                    let target_url = match page.as_str() {
                        "home" => {
                            let html = internal_pages::home_page_html_with_engine(selected_search_engine.js_search_template());
                            page_to_file_url("axomai_home.html", &html)
                        }
                        "extensions" => {
                            let html = internal_pages::extensions_page_html(&extensions);
                            page_to_file_url("axomai_extensions.html", &html)
                        }
                        "settings" => {
                            let restore = browser_storage.as_ref()
                                .and_then(|s| s.get_setting("restore_session").ok().flatten())
                                .map(|v| v == "true").unwrap_or(false);
                            let html = internal_pages::settings_page_html(selected_search_engine, restore);
                            page_to_file_url("axomai_settings.html", &html)
                        }
                        "history" => {
                            let entries = browser_storage.as_ref()
                                .and_then(|s| s.get_history(100).ok())
                                .unwrap_or_default();
                            let html = pages::history_page_html(&entries);
                            page_to_file_url("axomai_history.html", &html)
                        }
                        "bookmarks" => {
                            let entries = browser_storage.as_ref()
                                .and_then(|s| s.get_bookmarks().ok())
                                .unwrap_or_default();
                            let html = pages::bookmarks_page_html(&entries);
                            page_to_file_url("axomai_bookmarks.html", &html)
                        }
                        "downloads" => {
                            let entries = browser_storage.as_ref()
                                .and_then(|s| s.get_downloads(100).ok())
                                .unwrap_or_default();
                            let html = pages::downloads_page_html(&entries);
                            page_to_file_url("axomai_downloads.html", &html)
                        }
                        _ => {
                            let html = internal_pages::home_page_html_with_engine(selected_search_engine.js_search_template());
                            page_to_file_url("axomai_home.html", &html)
                        }
                    };

                    let cw = w_of(&gpu_renderer);
                    let ch = h_of(&gpu_renderer);
                    if webview.is_none() {
                        let nav_clone = nav_url_shared.clone();
                        webview = WebViewBuilder::new()
                            .with_url(&target_url)
                            .with_devtools(false)
                            .with_bounds(Rect {
                                position: wry::dpi::LogicalPosition::new(SIDEBAR_W as i32, CHROME_TOP as i32).into(),
                                size: wry::dpi::LogicalSize::new(
                                    (cw - SIDEBAR_W) as u32,
                                    (ch - CHROME_TOP) as u32,
                                ).into(),
                            })
                            .with_navigation_handler(move |nav_url: String| {
                                if nav_url.starts_with("axomai://") {
                                    if let Ok(mut nav) = nav_clone.lock() {
                                        *nav = Some(nav_url);
                                    }
                                    return false;
                                }
                                if let Ok(mut nav) = nav_clone.lock() {
                                    *nav = Some(nav_url);
                                }
                                true
                            })
                            .with_download_started_handler({
                                let dl_dir = download_dir.clone();
                                move |url, path| {
                                    let fname = url.rsplit('/').next().unwrap_or("download").split('?').next().unwrap_or("download");
                                    let fname = if fname.is_empty() { "download" } else { fname };
                                    *path = dl_dir.join(fname);
                                    true
                                }
                            })
                            .with_download_completed_handler({
                                let dl_sig = download_signal.clone();
                                move |url, path, success| {
                                    let fname = path.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_default();
                                    if let Ok(mut sig) = dl_sig.lock() {
                                        sig.push((url, fname, success));
                                    }
                                }
                            })
                            .build_as_child(&window)
                            .ok();
                        webview_visible = webview.is_some();
                    } else if let Some(ref wv) = webview {
                        let _ = wv.load_url(&target_url);
                        if !webview_visible {
                            let _ = wv.set_visible(true);
                            webview_visible = true;
                        }
                    }
                    needs_chrome_redraw = true;
                }
                if let Some(t) = ext_inject_time {
                    if std::time::Instant::now() >= t {
                        ext_inject_time = None;
                        if let Some(ref wv) = webview {
                            for ext in &extensions {
                                if ext.enabled && ext.auto_inject {
                                    let _ = wv.evaluate_script(ext.inject_js);
                                }
                            }
                        }
                    }
                }

                if let Ok(mut sigs) = download_signal.lock() {
                    for (url, fname, success) in sigs.drain(..) {
                        if let Some(ref s) = browser_storage {
                            let filepath = download_dir.join(&fname).to_string_lossy().to_string();
                            let status = if success { "completed" } else { "failed" };
                            let _ = s.add_download(&url, &fname, &filepath);
                            if let Ok(downloads) = s.get_downloads(1) {
                                if let Some(dl) = downloads.first() {
                                    let _ = s.update_download_status(dl.id, status, 0);
                                }
                            }
                        }
                        println!("[Axomai] Download {}: {} ({})", if success { "completed" } else { "failed" }, fname, url);
                    }
                }

                let w = w_of(&gpu_renderer);
                let h = h_of(&gpu_renderer);
                let content_w = w - SIDEBAR_W;
                let content_h = h - CHROME_TOP;

                let should_render = if is_home_page || is_settings_page || is_extensions_page {
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
                    } else if is_extensions_page {
                        "Extensions".to_string()
                    } else if is_settings_page {
                        "Settings".to_string()
                    } else if let Ok(eng) = engine.lock() {
                        let t = eng.current_title.clone();
                        if !t.is_empty() && is_loading {
                            is_loading = false;
                            loading_progress = 0.0;
                        }
                        t
                    } else {
                        String::new()
                    };

                    let history_back = if let Ok(eng) = engine.lock() { eng.history_index > 0 } else { false };
                    let history_fwd = if let Ok(eng) = engine.lock() { eng.history_index + 1 < eng.history.len() } else { false };

                    let toolbar_icons = [icon_back, icon_forward, icon_home, icon_menu];
                    if active_tab_idx < tabs.len() {
                        tabs[active_tab_idx].url = address_bar_text.clone();
                        tabs[active_tab_idx].is_home = is_home_page;
                        tabs[active_tab_idx].is_extensions = is_extensions_page;
                        tabs[active_tab_idx].is_settings = is_settings_page;
                        tabs[active_tab_idx].title = title.clone();
                    }
                    let mut quads = build_chrome_quads(
                        &mut compositor, w, h, &address_bar_text, address_bar_focused, address_bar_cursor,
                        &tabs, active_tab_idx, history_back, history_fwd, sidebar_active, hover_sidebar_idx,
                        menu_open, hover_menu_idx, &toolbar_icons, &extensions,
                    );

                    if is_home_page || is_extensions_page || is_settings_page {
                        // Internal pages rendered via WebView HTML — no GPU quads needed
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

                    if find_bar_open {
                        let fb_w = 320.0f32.min(w - SIDEBAR_W - 20.0);
                        let fb_x = w - fb_w - 16.0;
                        let fb_y = CHROME_TOP + 4.0;
                        let fb_h = 36.0;
                        quads.push(NativeGpuCompositor::solid_quad(fb_x + 2.0, fb_y + 2.0, fb_w, fb_h, rendering::c(0, 0, 0, 30)));
                        quads.push(NativeGpuCompositor::solid_quad(fb_x, fb_y, fb_w, fb_h, rendering::c(255, 255, 255, 250)));
                        quads.push(NativeGpuCompositor::solid_quad(fb_x, fb_y + fb_h - 2.0, fb_w, 2.0, rendering::c(26, 115, 232, 255)));
                        let display = if find_text.is_empty() { "Find in page..." } else { &find_text };
                        let tc = if find_text.is_empty() { rendering::c(150, 155, 168, 255) } else { rendering::c(32, 33, 36, 255) };
                        rendering::render_text(&mut compositor, &mut quads, display, fb_x + 12.0, fb_y + 24.0, 13.0, tc, fb_x + fb_w - 30.0);
                        rendering::render_text(&mut compositor, &mut quads, "x", fb_x + fb_w - 20.0, fb_y + 24.0, 13.0, rendering::c(95, 99, 104, 255), fb_x + fb_w);
                    }

                    if is_loading {
                        loading_progress = (loading_progress + 0.02).min(1.0);
                        if loading_progress >= 1.0 {
                            is_loading = false;
                            loading_progress = 0.0;
                        } else {
                            let bar_w = (w - SIDEBAR_W) * loading_progress;
                            quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, CHROME_TOP - 3.0, bar_w, 3.0, rendering::c(26, 115, 232, 200)));
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

fn dirs_download() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(home).join("Downloads")
    } else {
        PathBuf::from(".")
    }
}

