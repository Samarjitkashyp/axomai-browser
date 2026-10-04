pub mod actions;
pub mod ai;
pub mod app;
pub mod app_commands;
pub mod app_input;
pub mod app_notes;
pub mod app_suggest;
pub mod app_tabs;
pub mod blocklist;
pub mod passwords;
pub mod permissions;
pub mod secret;
pub mod bookmarks_io;
pub mod app_bookmarks;
pub mod downloads;
pub mod ext_scripts;
pub mod extensions;
pub mod favicons;
pub mod i18n;
pub mod launch;
pub mod i18n_data;
pub mod news;
pub mod omnibox;
pub mod overlays;
pub mod pages;
pub mod rendering;
pub mod settings;
pub mod settings_page;
pub mod site_icons;
pub mod splitview;
pub mod storage;
pub mod suggest;
pub mod sys;
pub mod tabgroups;
pub mod tabs;
pub mod theme;
pub mod toolbar;
pub mod viewctl;
pub mod types;
pub mod ui_shell;
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
    if args.iter().any(|a| a == "--register-default") {
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        println!("{}", if launch::register(&exe) { "Axomai is registered as a web browser for this user." } else { "Registration failed." });
        return Ok(());
    }
    if args.iter().any(|a| a == "--unregister-default") {
        launch::unregister();
        println!("Axomai's browser registration was removed.");
        return Ok(());
    }
    // Addresses given on the command line ("Open with", links from other apps). A window that is already running
    // takes them; this process then has nothing to do.
    let urls = launch::addresses_from_args(&args[1..]);
    if !private_window && launch::hand_over(&storage::data_dir(), &urls) {
        return Ok(());
    }

    let storage = match storage::BrowserStorage::new() {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("[Axomai] Storage init failed (non-fatal): {}", e);
            None
        }
    };
    // The cloud-AI settings were removed from the browser: wipe the keys and switches an earlier build may have left.
    if let Some(st) = storage.as_ref() {
        for key in ["ai_key_enc", "ai_key_openai_enc", "ai_llm", "ai_model", "ai_provider"] {
            let _ = st.delete_setting(key);
        }
    }
    // Downloads that were still running when the last window closed can never finish (unless another window is up).
    if !launch::is_running(&storage::data_dir()) {
        if let Some(st) = storage.as_ref() {
            let _ = st.fail_stale_downloads();
        }
    }
    let setting = |key: &str| storage.as_ref().and_then(|s| s.get_setting(key).ok().flatten());
    let settings = settings::Settings::load(storage.as_ref());
    let search_engine = match setting("search_engine").as_deref() {
        Some("Bing") => SearchEngine::Bing,
        Some("Yahoo") => SearchEngine::Yahoo,
        Some("DuckDuckGo") => SearchEngine::DuckDuckGo,
        _ => SearchEngine::Google,
    };

    let event_loop = EventLoop::new();
    let proxy = event_loop.create_proxy();
    let icon = {
        let path = sys::asset_path("icons/axomai_logo.png");
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

    let download_dir = if settings.download_dir.is_empty() { dirs_download() } else { PathBuf::from(&settings.download_dir) };
    let _ = std::fs::create_dir_all(&download_dir);
    let ui_dir = sys::ui_dir();
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
    // What the first window opens, according to "On startup".
    let mut saved_tabs: Vec<storage::SavedTab> = match settings.startup {
        settings::Startup::Restore if !private_window => storage.as_ref().and_then(|s| s.get_tabs().ok()).unwrap_or_default(),
        settings::Startup::HomePage if !settings.home_url.is_empty() => vec![storage::SavedTab { position: 0, url: settings.home_url.clone(), title: String::new(), is_active: true, group: String::new() }],
        _ => Vec::new(),
    };
    if !urls.is_empty() {
        for t in saved_tabs.iter_mut() {
            t.is_active = false;
        }
        let first = saved_tabs.len() as i32;
        for (i, u) in urls.iter().enumerate() {
            saved_tabs.push(storage::SavedTab { position: first + i as i32, url: u.clone(), title: String::new(), is_active: i + 1 == urls.len(), group: String::new() });
        }
    }
    // AXOMAI_UI_SCALE exists so the layout can be checked at other display scales on a 100% screen.
    let scale = std::env::var("AXOMAI_UI_SCALE").ok().and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.75..=4.0).contains(v)).unwrap_or(window.scale_factor() as f32);
    rendering::set_ui_scale(scale);

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
        settings,
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
        suggestions: Vec::new(),
        sugg_sel: 0,
        sugg_shown: false,
        sugg_hide_at: None,
        bar_marks: Vec::new(),
        dls: Vec::new(),
        dl_push_at: std::time::Instant::now(),
        pw_pending: None,
        pw_offer: None,
        perm_queue: Vec::new(),
        perm_session: Default::default(),
        infobar_on: false,
        groups: Vec::new(),
        next_group_id: 1,
        host_slots: Default::default(),
        icon_pending: Default::default(),
        news_cache: Default::default(),
        news_pending: Default::default(),
        split: None,
        last_beat: std::time::Instant::now() - std::time::Duration::from_secs(10),
        last_handoff_poll: std::time::Instant::now(),
        html_fullscreen: false,
        https_warn: Vec::new(),
        safe: false,
        proxy,
        _instance: instance,
    };
    app.core.passwords = app.settings.password_manager;
    app.core.gpc = app.settings.gpc;
    app.apply_privacy_settings();
    app.restore_or_start(saved_tabs);
    app.refresh_bar();

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
