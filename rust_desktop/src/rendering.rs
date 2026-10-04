use axomai_engine::{GpuQuad, NativeGpuCompositor, WgpuRenderer};
use crate::types::{DesktopTab, Extension, SearchEngine, SIDEBAR_ITEMS, SIDEBAR_W, CHROME_TOP};

pub fn w_of(r: &WgpuRenderer) -> f32 { r.surface_config.width as f32 }
pub fn h_of(r: &WgpuRenderer) -> f32 { r.surface_config.height as f32 }

/// The swap-chain is sRGB, so colours written by the shader are treated as linear light. Convert from the sRGB
/// values used in CSS/design tokens, otherwise every colour comes out lighter than the web UI it mirrors.
fn srgb_to_linear(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

pub fn c(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a as f32 / 255.0]
}

pub fn render_text(
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

pub fn render_text_centered(
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

fn rq(x: f32, y: f32, w: f32, h: f32, _r: f32, color: [f32; 4]) -> GpuQuad {
    NativeGpuCompositor::solid_quad(x, y, w, h, color)
}

pub fn build_chrome_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    viewport_h: f32,
    address_text: &str,
    focused: bool,
    address_cursor: usize,
    tabs: &[DesktopTab],
    active_tab_idx: usize,
    history_back: bool,
    history_fwd: bool,
    sidebar_active: usize,
    hover_sidebar: Option<usize>,
    _menu_open: bool,
    _hover_menu: Option<usize>,
    icons: &crate::toolbar::ToolbarIcons,
    chrome: &crate::toolbar::ChromeState,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();


    if SIDEBAR_W > 0.0 {
        let bands = 8;
        for i in 0..bands {
            let t = i as f32 / bands as f32;
            let r = (15.0 + t * 10.0) as u8;
            let g = (20.0 + t * 30.0) as u8;
            let b = (50.0 + t * 30.0) as u8;
            let band_h = viewport_h / bands as f32;
            quads.push(NativeGpuCompositor::solid_quad(0.0, i as f32 * band_h, SIDEBAR_W, band_h + 1.0, c(r, g, b, 255)));
        }
        quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, SIDEBAR_W, viewport_h, c(255, 255, 255, 18)));
        quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W - 1.0, 0.0, 1.0, viewport_h, c(100, 140, 200, 80)));

        let sb_text = c(220, 225, 235, 255);
        let sb_text_dim = c(140, 155, 180, 255);
        let sb_accent = c(100, 180, 255, 255);
        let sb_divider = c(255, 255, 255, 20);
        let sb_hover = c(255, 255, 255, 20);
        let sb_active = c(100, 180, 255, 30);

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
    }

    quads.extend(crate::toolbar::build_toolbar_quads(
        compositor, viewport_w, address_text, focused, address_cursor, tabs, active_tab_idx,
        history_back, history_fwd, icons, chrome,
    ));
    quads
}

pub fn build_dropdown_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    hover_menu: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();
    let text_primary = c(32, 33, 36, 255);
    let dm_w = 250.0;
    let dm_x = viewport_w - dm_w - 16.0;
    let dm_y = CHROME_TOP + 4.0;
    let menu_items: &[(&str, &str)] = &[
        ("+", "New Tab            Ctrl+T"),
        ("H", "Home Page"),
        ("B", "Bookmarks"),
        ("h", "History"),
        ("D", "Downloads"),
        ("E", "Extensions"),
        ("P", "Passwords"),
        ("T", "Heritage Themes"),
        ("C", "Clear RAM & Cache"),
        ("S", "Settings"),
        ("A", "About"),
        ("X", "Exit Axomai"),
    ];
    let dm_h = 10.0 + menu_items.len() as f32 * 36.0 + 8.0;
    quads.push(rq(dm_x + 3.0, dm_y + 3.0, dm_w, dm_h, 12.0, c(0, 0, 0, 40)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(255, 255, 255, 255)));
    quads.push(rq(dm_x, dm_y, dm_w, dm_h, 12.0, c(218, 220, 224, 60)));

    for (i, (icon, label)) in menu_items.iter().enumerate() {
        let iy = dm_y + 8.0 + i as f32 * 36.0;
        if hover_menu == Some(i) {
            quads.push(rq(dm_x + 6.0, iy, dm_w - 12.0, 34.0, 8.0, c(235, 240, 248, 255)));
        }
        render_text(compositor, &mut quads, icon, dm_x + 16.0, iy + 22.0, 13.0, c(16, 185, 129, 255), dm_x + 36.0);
        render_text(compositor, &mut quads, label, dm_x + 38.0, iy + 22.0, 12.5, text_primary, dm_x + dm_w - 12.0);
    }

    quads
}

#[allow(dead_code)]
pub fn build_settings_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    selected: SearchEngine,
    hover_idx: Option<usize>,
) -> Vec<GpuQuad> {
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

    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));
    render_text(compositor, &mut quads, "Settings", 40.0, 40.0, 24.0, text_dark, content_w);
    quads.push(NativeGpuCompositor::solid_quad(40.0, 55.0, content_w - 80.0, 1.0, border));
    render_text(compositor, &mut quads, "Search Engine", 40.0, 88.0, 16.0, text_dark, content_w);
    render_text(compositor, &mut quads, "Choose the search engine used in the address bar", 40.0, 108.0, 12.0, text_secondary, content_w);

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

        let bg = if is_selected { selected_bg } else if is_hovered { hover_bg } else { white };
        quads.push(rq(card_x, ey, card_w, engine_h, 10.0, bg));

        let radio_x = card_x + 20.0;
        let radio_y = ey + engine_h / 2.0;
        quads.push(rq(radio_x - 9.0, radio_y - 9.0, 18.0, 18.0, 9.0, if is_selected { blue } else { border }));
        quads.push(rq(radio_x - 7.0, radio_y - 7.0, 14.0, 14.0, 7.0, if is_selected { blue } else { white }));
        if is_selected {
            quads.push(rq(radio_x - 4.0, radio_y - 4.0, 8.0, 8.0, 4.0, white));
        }

        let name_x = card_x + 48.0;
        render_text(compositor, &mut quads, eng.name(), name_x, ey + 24.0, 14.0, text_dark, card_x + card_w);
        render_text(compositor, &mut quads, engine_descriptions[i], name_x, ey + 42.0, 11.0, text_secondary, card_x + card_w - 10.0);

        if is_selected {
            let badge_x = card_x + card_w - 80.0;
            render_text(compositor, &mut quads, "Default", badge_x, ey + 32.0, 11.0, green, card_x + card_w);
        }
    }

    let info_y = start_y + 4.0 * (engine_h + 8.0) + 10.0;
    render_text(compositor, &mut quads, "Click on a search engine to set it as default.", 40.0, info_y + 14.0, 11.0, text_secondary, content_w);
    render_text(compositor, &mut quads, "The selected engine is used when you type in the address bar.", 40.0, info_y + 30.0, 11.0, text_secondary, content_w);

    quads
}

#[allow(dead_code)]
pub fn build_extensions_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    extensions: &[Extension],
    hover_idx: Option<usize>,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let page_bg = c(241, 243, 244, 255);
    let text_dark = c(32, 33, 36, 255);
    let text_secondary = c(95, 99, 104, 255);
    let text_hint = c(154, 160, 166, 255);
    let blue = c(26, 115, 232, 255);
    let green = c(34, 168, 83, 255);
    let gray_track = c(189, 193, 198, 255);
    let card_border = c(218, 220, 224, 255);

    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, page_bg));

    let header_h = 64.0;
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, header_h, white));
    quads.push(NativeGpuCompositor::solid_quad(0.0, header_h - 1.0, content_w, 1.0, card_border));

    let icon_s = 28.0;
    quads.push(rq(24.0, (header_h - icon_s) / 2.0, icon_s, icon_s, 6.0, blue));
    render_text(compositor, &mut quads, "E", 31.0, header_h / 2.0 + 6.0, 16.0, white, 60.0);
    render_text(compositor, &mut quads, "Extensions", 64.0, header_h / 2.0 + 7.0, 20.0, text_dark, content_w);

    let search_w = 260.0f32.min(content_w - 300.0);
    if search_w > 100.0 {
        let sx = content_w - search_w - 24.0;
        let sy = (header_h - 36.0) / 2.0;
        quads.push(rq(sx, sy, search_w, 36.0, 18.0, c(241, 243, 244, 255)));
        render_text(compositor, &mut quads, "Search extensions", sx + 16.0, sy + 23.0, 13.0, text_hint, sx + search_w - 8.0);
    }

    let section_y = header_h + 24.0;
    render_text(compositor, &mut quads, "All Extensions", 32.0, section_y + 16.0, 14.0, text_dark, content_w);

    let grid_start_y = section_y + 36.0;
    let padding = 24.0;
    let gap = 16.0;
    let cols = if content_w > 600.0 { 2 } else { 1 };
    let card_w = if cols == 2 { (content_w - padding * 2.0 - gap) / 2.0 } else { content_w - padding * 2.0 };
    let card_h = 160.0;

    for (i, ext) in extensions.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let cx = padding + col * (card_w + gap);
        let cy = grid_start_y + row * (card_h + gap);
        let is_hovered = hover_idx == Some(i);

        if is_hovered {
            quads.push(rq(cx + 1.0, cy + 3.0, card_w - 2.0, card_h, 12.0, c(0, 0, 0, 20)));
        }

        quads.push(rq(cx, cy, card_w, card_h, 12.0, white));
        quads.push(rq(cx, cy, card_w, 1.0, 0.0, c(218, 220, 224, 60)));
        quads.push(rq(cx, cy + card_h - 1.0, card_w, 1.0, 0.0, c(218, 220, 224, 60)));
        quads.push(rq(cx, cy, 1.0, card_h, 0.0, c(218, 220, 224, 40)));
        quads.push(rq(cx + card_w - 1.0, cy, 1.0, card_h, 0.0, c(218, 220, 224, 40)));

        let icon_size = 44.0;
        let icon_x = cx + 20.0;
        let icon_y = cy + 20.0;
        let ic = c(ext.icon_color[0], ext.icon_color[1], ext.icon_color[2], 255);
        quads.push(rq(icon_x, icon_y, icon_size, icon_size, icon_size / 2.0, ic));
        render_text(compositor, &mut quads, ext.icon_letter, icon_x + 13.0, icon_y + 30.0, 20.0, white, icon_x + icon_size);

        let name_x = icon_x + icon_size + 14.0;
        let name_end = cx + card_w - 80.0;
        render_text(compositor, &mut quads, ext.name, name_x, icon_y + 18.0, 15.0, text_dark, name_end);
        let ver_label = format!("  {}", ext.version);
        let name_w = ext.name.len() as f32 * 8.5;
        render_text(compositor, &mut quads, &ver_label, name_x + name_w, icon_y + 18.0, 11.0, text_hint, name_end + 60.0);
        render_text(compositor, &mut quads, ext.description, name_x, icon_y + 38.0, 11.0, text_secondary, cx + card_w - 20.0);

        let toggle_x = cx + card_w - 64.0;
        let toggle_y = cy + 24.0;
        let track_w = 44.0;
        let track_h = 22.0;
        let thumb_r = 9.0;

        if ext.enabled {
            quads.push(rq(toggle_x, toggle_y, track_w, track_h, track_h / 2.0, blue));
            quads.push(rq(toggle_x + track_w - track_h + 2.0, toggle_y + 2.0, thumb_r * 2.0, thumb_r * 2.0, thumb_r, white));
        } else {
            quads.push(rq(toggle_x, toggle_y, track_w, track_h, track_h / 2.0, gray_track));
            quads.push(rq(toggle_x + 2.0, toggle_y + 2.0, thumb_r * 2.0, thumb_r * 2.0, thumb_r, white));
        }

        let bottom_y = cy + card_h - 44.0;
        quads.push(NativeGpuCompositor::solid_quad(cx + 16.0, bottom_y, card_w - 32.0, 1.0, c(218, 220, 224, 120)));

        let btn_y = bottom_y + 10.0;
        let details_x = cx + 20.0;
        quads.push(rq(details_x, btn_y, 68.0, 26.0, 13.0, c(232, 240, 254, 255)));
        render_text(compositor, &mut quads, "Details", details_x + 10.0, btn_y + 17.0, 11.0, blue, details_x + 66.0);

        let remove_x = details_x + 78.0;
        quads.push(rq(remove_x, btn_y, 72.0, 26.0, 13.0, c(232, 240, 254, 255)));
        render_text(compositor, &mut quads, "Remove", remove_x + 10.0, btn_y + 17.0, 11.0, blue, remove_x + 70.0);

        let status_text = if ext.enabled { "Active" } else { "Inactive" };
        let status_c = if ext.enabled { green } else { text_hint };
        render_text(compositor, &mut quads, status_text, cx + card_w - 70.0, btn_y + 17.0, 10.0, status_c, cx + card_w);
    }

    quads
}

#[allow(dead_code)]
pub fn build_home_page_quads(
    compositor: &mut NativeGpuCompositor,
    content_w: f32,
    content_h: f32,
    _scroll_y: f32,
    search_focused: bool,
    search_text: &str,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    let white = c(255, 255, 255, 255);
    let blue = c(26, 115, 232, 255);

    quads.push(NativeGpuCompositor::bg_image_quad(0.0, 0.0, content_w, content_h));
    quads.push(NativeGpuCompositor::solid_quad(0.0, 0.0, content_w, content_h, c(0, 0, 0, 140)));

    let cx = content_w / 2.0;
    let cy = content_h / 2.0 - 60.0;

    let logo_size = 56.0;
    let logo_x = cx - logo_size / 2.0;
    quads.push(rq(logo_x, cy, logo_size, logo_size, 14.0, c(255, 255, 255, 40)));
    quads.push(rq(logo_x + 2.0, cy + 2.0, logo_size - 4.0, logo_size - 4.0, 12.0, blue));
    render_text_centered(compositor, &mut quads, "A", cx, cy + 42.0, 28.0, white);

    render_text_centered(compositor, &mut quads, "Axomai Browser", cx, cy + 85.0, 26.0, white);
    render_text_centered(compositor, &mut quads, "Fast. Private. AI-Powered. Built for Everyone.", cx, cy + 112.0, 13.0, c(200, 210, 220, 200));

    let search_w = 540.0f32.min(content_w - 80.0);
    let search_x = cx - search_w / 2.0;
    let search_y = cy + 135.0;
    if search_focused {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(100, 160, 255, 80)));
    } else {
        quads.push(rq(search_x, search_y, search_w, 44.0, 22.0, c(255, 255, 255, 25)));
    }
    quads.push(rq(search_x + 1.0, search_y + 1.0, search_w - 2.0, 42.0, 21.0, c(30, 30, 30, 180)));
    render_text(compositor, &mut quads, "G", search_x + 16.0, search_y + 30.0, 16.0, c(130, 180, 255, 255), search_x + 36.0);
    if search_focused && !search_text.is_empty() {
        render_text(compositor, &mut quads, search_text, search_x + 42.0, search_y + 28.0, 14.0, white, search_x + search_w - 40.0);
        let cursor_x = search_x + 42.0 + search_text.len() as f32 * 8.0;
        quads.push(NativeGpuCompositor::solid_quad(cursor_x, search_y + 10.0, 2.0, 24.0, white));
    } else if search_focused {
        quads.push(NativeGpuCompositor::solid_quad(search_x + 42.0, search_y + 10.0, 2.0, 24.0, white));
    } else {
        render_text(compositor, &mut quads, "Search the web with Axomai AI...", search_x + 42.0, search_y + 28.0, 14.0, c(180, 185, 195, 200), search_x + search_w - 40.0);
    }
    render_text(compositor, &mut quads, "Q", search_x + search_w - 32.0, search_y + 28.0, 14.0, c(160, 165, 175, 200), search_x + search_w);

    quads
}
