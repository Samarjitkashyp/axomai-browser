mod internal_pages;
pub mod actions;
pub mod ai;
pub mod app;
pub mod app_commands;
pub mod app_input;
pub mod app_tabs;
pub mod blocklist;
pub mod ext_scripts;
pub mod extensions;
pub mod favicons;
pub mod omnibox;
pub mod overlays;
pub mod pages;
pub mod rendering;
pub mod storage;
pub mod sys;
pub mod tabs;
pub mod theme;
pub mod toolbar;
pub mod types;
pub mod viewsource;
pub mod web;

use app::App;
use axomai_engine::{NativeGpuCompositor, WgpuRenderer};
use extensions::create_extensions;
use std::path::PathBuf;
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use types::SearchEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--subprocess" {
        axomai_engine::run_subprocess(&args[2]);
        return Ok(());
    }
    let private_window = args.iter().any(|a| a == "--incognito");

    let storage = match storage::BrowserStorage::new() {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("[Axomai] Storage init failed (non-fatal): {}", e);
            None
        }
    };
    let setting = |key: &str| storage.as_ref().and_then(|s| s.get_setting(key).ok().flatten());
    let search_engine = match setting("search_engine").as_deref() {
        Some("Bing") => SearchEngine::Bing,
        Some("Yahoo") => SearchEngine::Yahoo,
        Some("DuckDuckGo") => SearchEngine::DuckDuckGo,
        _ => SearchEngine::Google,
    };

    let event_loop = EventLoop::new();
    let proxy = event_loop.create_proxy();
    let icon = {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons").join("axomai_logo.png");
        image::open(&path)
            .ok()
            .and_then(|img| {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                tao::window::Icon::from_rgba(rgba.into_raw(), w, h).ok()
            })
    };
    let mut wb = WindowBuilder::new()
        .with_title(if private_window { "Axomai Browser (Incognito)" } else { "Axomai Browser" })
        .with_inner_size(LogicalSize::new(1200.0, 700.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));
    if let Some(icon) = icon {
        wb = wb.with_window_icon(Some(icon));
    }
    let window = wb.build(&event_loop)?;
    let size: PhysicalSize<u32> = window.inner_size();

    let backends: &[(&str, wgpu::Backends)] = if cfg!(target_os = "windows") {
        &[
            ("All", wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL),
            ("GL", wgpu::Backends::GL),
            ("Vulkan", wgpu::Backends::VULKAN),
            ("DX12", wgpu::Backends::DX12),
        ]
    } else {
        &[("All", wgpu::Backends::all())]
    };
    let mut gpu: Option<WgpuRenderer> = None;
    let mut instance: Option<wgpu::Instance> = None;
    for (name, backend) in backends {
        let inst = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: *backend, ..Default::default() });
        let surface = match unsafe { inst.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).expect("surface target")) } {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Axomai] {} surface creation failed: {}", name, e);
                continue;
            }
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| WgpuRenderer::new(&inst, surface, size.width, size.height))) {
            Ok(r) => {
                println!("[Axomai] {} backend succeeded", name);
                gpu = Some(r);
                instance = Some(inst);
                break;
            }
            Err(_) => eprintln!("[Axomai] {} backend failed, trying next...", name),
        }
    }
    let mut gpu = gpu.expect("Failed to initialize the GPU with any backend. Ensure GPU drivers are installed.");

    let mut compositor = NativeGpuCompositor::new(size.width, size.height);
    let icons = toolbar::load_icons(&mut compositor);
    let favicons = favicons::Favicons::new();
    // The chrome's image texture is the favicon atlas; it must exist before the first frame.
    gpu.upload_bg_image(favicons::ATLAS, favicons::ATLAS, favicons.pixels());

    let download_dir = dirs_download();
    let _ = std::fs::create_dir_all(&download_dir);
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or(std::path::Path::new(".")).join("ui");
    let hub = web::WebShared::new(download_dir, &ui_dir);
    let mut extensions = create_extensions();
    let mut core = actions::Core::new(storage.as_ref(), &mut extensions);
    if private_window {
        // A private window always uses the dark theme so it is obvious at a glance.
        core.theme = theme::by_id("cyber-dark");
    }
    if let Some(saved) = setting("blocked_total").and_then(|v| v.parse::<u64>().ok()) {
        hub.shield.total_blocked.store(saved, std::sync::atomic::Ordering::Relaxed);
    }
    let restore = !private_window && setting("restore_session").as_deref() == Some("true");
    let saved_tabs = if restore { storage.as_ref().and_then(|s| s.get_tabs().ok()).unwrap_or_default() } else { Vec::new() };
    let scale = window.scale_factor() as f32;

    let mut app = App {
        window,
        gpu,
        compositor,
        icons,
        favicons,
        hub,
        core,
        extensions,
        storage,
        search_engine,
        tabs: Vec::new(),
        active: 0,
        webview: None,
        closed: Vec::new(),
        next_tab_id: 1,
        addr_text: String::new(),
        addr_cursor: 0,
        addr_focused: false,
        addr_selected: false,
        addr_focus_at: std::time::Instant::now(),
        mouse: (0.0, 0.0),
        left_down: false,
        mods: Default::default(),
        scale,
        drag: None,
        redraw: true,
        loading_progress: 0.0,
        last_shield_count: 0,
        fullscreen: false,
        private_window,
        exit: false,
        view_pending: false,
        safe: false,
        proxy,
        _instance: instance,
    };
    app.restore_or_start(saved_tabs);

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(16));
        app.handle(event, flow);
    });
}

fn dirs_download() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(home).join("Downloads")
    } else {
        PathBuf::from(".")
    }
}
