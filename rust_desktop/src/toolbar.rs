//! Native browser chrome (tab strip + navigation toolbar) laid out to mirror `ui/index.html` / `ui/style.css`.
//! `toolbar_layout` / `tab_strip` are the single source of truth for geometry: the renderer draws from
//! them and `main.rs` hit-tests against them, so what is drawn is what is clickable.

use crate::rendering::{c, fit_text, render_text, render_text_centered, text_width};
use crate::theme::Theme;
use crate::favicons::Favicons;
use crate::types::{SIDEBAR_W, TAB_BAR_H, TOOLBAR_H};
use axomai_engine::glyph_atlas::GlyphInfo;
use axomai_engine::{GpuQuad, NativeGpuCompositor};
use resvg::{tiny_skia, usvg};

// ---------------------------------------------------------------- geometry

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }
}

pub const BRAND_W: f32 = 168.0;
pub const TAB_START_X: f32 = SIDEBAR_W + BRAND_W;
pub const TAB_GAP: f32 = 5.0;
pub const TAB_H: f32 = 36.0;
const TAB_MIN_W: f32 = 44.0;
const TAB_MAX_W: f32 = 220.0;
pub const PINNED_W: f32 = 44.0;

/// What the tab strip needs to draw one tab.
#[derive(Clone, Debug, Default)]
pub struct TabItem {
    pub title: String,
    pub pinned: bool,
    pub private: bool,
    pub favicon: Option<usize>,
    pub audio: bool,
    pub muted: bool,
    pub loading: bool,
    pub sleeping: bool,
}

pub struct TabStrip {
    pub rects: Vec<Rect>,
    pub plus: Rect,
}

impl TabStrip {
    pub fn tab_rect(&self, i: usize) -> Rect {
        self.rects.get(i).copied().unwrap_or(Rect::new(TAB_START_X, TAB_BAR_H - TAB_H, 0.0, TAB_H))
    }
    pub fn close_rect(&self, i: usize) -> Rect {
        let t = self.tab_rect(i);
        Rect::new(t.right() - 28.0, t.y, 28.0, t.h)
    }
    /// Speaker / mute button, just left of the close button.
    pub fn speaker_rect(&self, i: usize) -> Rect {
        let t = self.tab_rect(i);
        Rect::new(t.right() - 52.0, t.y, 24.0, t.h)
    }
    pub fn end_x(&self) -> f32 {
        self.rects.last().map(|r| r.right()).unwrap_or(TAB_START_X)
    }
    pub fn index_at(&self, x: f32, y: f32) -> Option<usize> {
        self.rects.iter().position(|r| r.contains(x, y))
    }
}

/// Whether a tab shows its close button (narrow tabs hide it; pinned tabs never have one).
pub fn close_visible(item: &TabItem, r: &Rect, active: bool) -> bool {
    !item.pinned && (if active { r.w >= 64.0 } else { r.w >= 96.0 })
}

/// Whether a tab shows the speaker / mute button.
pub fn speaker_visible(item: &TabItem, r: &Rect) -> bool {
    item.audio && !item.pinned && r.w >= 84.0
}

pub fn tab_strip(w: f32, items: &[TabItem]) -> TabStrip {
    let pinned = items.iter().filter(|t| t.pinned).count();
    let others = items.len() - pinned;
    let reserved = 76.0; // the "+" button and a right margin
    let available = w - TAB_START_X - reserved - pinned as f32 * (PINNED_W + TAB_GAP);
    let tab_w = if others == 0 { TAB_MAX_W } else { (available / others as f32 - TAB_GAP).clamp(TAB_MIN_W, TAB_MAX_W) };
    let mut x = TAB_START_X;
    let mut rects = Vec::with_capacity(items.len());
    for t in items {
        let width = if t.pinned { PINNED_W } else { tab_w };
        rects.push(Rect::new(x, TAB_BAR_H - TAB_H, width, TAB_H));
        x += width + TAB_GAP;
    }
    TabStrip { rects, plus: Rect::new(x, TAB_BAR_H - 3.0 - 30.0, 30.0, 30.0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarHit {
    Back,
    Forward,
    Reload,
    Home,
    Omnibox,
    Secure,
    Bookmark,
    Reader,
    Qr,
    Shield,
    Extensions,
    Downloads,
    Theme,
    Ai,
    Profile,
    Menu,
}

pub struct ToolbarLayout {
    pub back: Rect,
    pub forward: Rect,
    pub reload: Rect,
    pub home: Rect,
    pub omnibox: Rect,
    pub secure_badge: Rect,
    pub engine_icon: Rect,
    pub text_x: f32,
    pub text_max_x: f32,
    pub bookmark: Rect,
    /// Zoom indicator, only drawn (and clickable) when the page is not at 100%.
    pub zoom: Rect,
    pub reader: Rect,
    pub qr: Rect,
    pub shield: Rect,
    pub extensions: Rect,
    pub downloads: Rect,
    pub theme: Rect,
    pub ai: Rect,
    pub profile: Rect,
    pub menu: Rect,
}

impl ToolbarLayout {
    pub fn hit(&self, x: f32, y: f32) -> Option<ToolbarHit> {
        if y < TAB_BAR_H || y > TAB_BAR_H + TOOLBAR_H {
            return None;
        }
        let table = [
            (self.secure_badge, ToolbarHit::Secure),
            (self.back, ToolbarHit::Back),
            (self.forward, ToolbarHit::Forward),
            (self.reload, ToolbarHit::Reload),
            (self.home, ToolbarHit::Home),
            (self.bookmark, ToolbarHit::Bookmark),
            (self.reader, ToolbarHit::Reader),
            (self.qr, ToolbarHit::Qr),
            (self.shield, ToolbarHit::Shield),
            (self.extensions, ToolbarHit::Extensions),
            (self.downloads, ToolbarHit::Downloads),
            (self.theme, ToolbarHit::Theme),
            (self.ai, ToolbarHit::Ai),
            (self.profile, ToolbarHit::Profile),
            (self.menu, ToolbarHit::Menu),
            (self.omnibox, ToolbarHit::Omnibox),
        ];
        table.iter().find(|(r, _)| r.contains(x, y)).map(|(_, h)| *h)
    }
}

pub fn toolbar_layout(w: f32) -> ToolbarLayout {
    let ty = TAB_BAR_H;
    let btn = 32.0;
    let by = ty + (TOOLBAR_H - btn) / 2.0;

    let mut x = SIDEBAR_W + 14.0;
    let nav = |x: &mut f32| {
        let r = Rect::new(*x, by, btn, btn);
        *x += btn + 4.0;
        r
    };
    let back = nav(&mut x);
    let forward = nav(&mut x);
    let reload = nav(&mut x);
    let home = nav(&mut x);
    let omni_x = x - 4.0 + 12.0;

    let (shield_w, ai_w, gap) = (58.0, 54.0, 6.0);
    let actions_w = shield_w + btn * 5.0 + ai_w + gap * 6.0;
    let omni_w = (w - omni_x - 12.0 - actions_w - 14.0).clamp(200.0, 820.0);
    let omnibox = Rect::new(omni_x, ty + (TOOLBAR_H - 36.0) / 2.0, omni_w, 36.0);

    let secure_badge = Rect::new(omnibox.x + 12.0, omnibox.cy() - 11.0, 76.0, 22.0);
    let engine_icon = Rect::new(secure_badge.right() + 10.0, omnibox.cy() - 8.0, 16.0, 16.0);
    let text_x = engine_icon.right() + 10.0;
    let qr = Rect::new(omnibox.right() - 12.0 - 28.0, omnibox.cy() - 14.0, 28.0, 28.0);
    let reader = Rect::new(qr.x - 2.0 - 28.0, qr.y, 28.0, 28.0);
    let bookmark = Rect::new(reader.x - 2.0 - 28.0, qr.y, 28.0, 28.0);
    let zoom = Rect::new(bookmark.x - 6.0 - 52.0, omnibox.cy() - 11.0, 52.0, 22.0);
    let text_max_x = bookmark.x - 8.0;

    let mut ax = omnibox.right() + 12.0;
    let mut place = |wd: f32, ht: f32| {
        let r = Rect::new(ax, ty + (TOOLBAR_H - ht) / 2.0, wd, ht);
        ax += wd + gap;
        r
    };
    let shield = place(shield_w, 26.0);
    let extensions = place(btn, btn);
    let downloads = place(btn, btn);
    let theme = place(btn, btn);
    let ai = place(ai_w, 28.0);
    let profile = place(btn, btn);
    let menu = place(btn, btn);

    ToolbarLayout {
        back, forward, reload, home, omnibox, secure_badge, engine_icon, text_x, text_max_x,
        bookmark, zoom, reader, qr, shield, extensions, downloads, theme, ai, profile, menu,
    }
}

// ------------------------------------------------------------------- icons

/// Icon masks rasterised from the same SVG paths `ui/index.html` uses.
#[derive(Clone, Copy)]
pub struct ToolbarIcons {
    pub back: GlyphInfo,
    pub forward: GlyphInfo,
    pub reload: GlyphInfo,
    pub home: GlyphInfo,
    pub menu: GlyphInfo,
    pub shield_outline: GlyphInfo,
    pub shield_filled: GlyphInfo,
    pub puzzle: GlyphInfo,
    pub download: GlyphInfo,
    pub sun: GlyphInfo,
    pub sparkle: GlyphInfo,
    pub star: GlyphInfo,
    pub star_filled: GlyphInfo,
    pub book: GlyphInfo,
    pub qr: GlyphInfo,
    pub close: GlyphInfo,
    pub plus: GlyphInfo,
    pub speaker: GlyphInfo,
    pub speaker_off: GlyphInfo,
}

fn stroke_svg(width: f32, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
    )
}

fn fill_svg(body: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white">{body}</svg>"#)
}

fn raster(svg: &str, px: u32) -> Vec<u8> {
    let tree = match usvg::Tree::from_str(svg, &usvg::Options::default()) {
        Ok(t) => t,
        Err(_) => return vec![0; (px * px) as usize],
    };
    let mut pixmap = tiny_skia::Pixmap::new(px, px).expect("pixmap");
    let s = px as f32 / tree.size().width();
    resvg::render(&tree, tiny_skia::Transform::from_scale(s, s), &mut pixmap.as_mut());
    pixmap.pixels().iter().map(|p| p.alpha()).collect()
}

pub fn load_icons(compositor: &mut NativeGpuCompositor) -> ToolbarIcons {
    let mut id = 0u32;
    let mut put = |svg: String, px: u32| {
        id += 1;
        compositor.glyph_atlas.blit_mask(id, &raster(&svg, px), px)
    };
    ToolbarIcons {
        back: put(stroke_svg(2.2, r#"<path d="M19 12H5M12 19l-7-7 7-7"/>"#), 16),
        forward: put(stroke_svg(2.2, r#"<path d="M5 12h14M12 5l7 7-7 7"/>"#), 16),
        reload: put(stroke_svg(2.2, r#"<path d="M23 4v6h-6M1 20v-6h6"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>"#), 16),
        home: put(fill_svg(r#"<path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/>"#), 16),
        menu: put(fill_svg(r#"<circle cx="12" cy="5" r="2.2"/><circle cx="12" cy="12" r="2.2"/><circle cx="12" cy="19" r="2.2"/>"#), 18),
        shield_outline: put(stroke_svg(2.2, r#"<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>"#), 14),
        shield_filled: put(fill_svg(r#"<path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z"/>"#), 14),
        puzzle: put(stroke_svg(2.0, r#"<path d="M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5a2.5 2.5 0 0 0-5 0V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7s2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5a2.5 2.5 0 0 0 0-5z"/>"#), 16),
        download: put(stroke_svg(2.0, r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>"#), 16),
        sun: put(stroke_svg(2.0, r#"<circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>"#), 16),
        sparkle: put(fill_svg(r#"<path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z"/>"#), 13),
        star: put(stroke_svg(2.0, r#"<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>"#), 16),
        star_filled: put(fill_svg(r#"<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>"#), 16),
        book: put(stroke_svg(2.0, r#"<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>"#), 16),
        qr: put(stroke_svg(2.0, r#"<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>"#), 16),
        close: put(stroke_svg(2.5, r#"<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>"#), 12),
        plus: put(stroke_svg(2.5, r#"<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>"#), 14),
        speaker: put(stroke_svg(2.0, r#"<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>"#), 14),
        speaker_off: put(stroke_svg(2.0, r#"<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/>"#), 14),
    }
}

// --------------------------------------------------------------- rendering

fn rr(x: f32, y: f32, w: f32, h: f32, r: f32, color: [f32; 4]) -> GpuQuad {
    NativeGpuCompositor::rounded_quad(x, y, w, h, r.min(w / 2.0).min(h / 2.0), color)
}

pub fn rr_rect(r: Rect, radius: f32, color: [f32; 4]) -> GpuQuad {
    rr(r.x, r.y, r.w, r.h, radius, color)
}

/// Horizontal two-colour gradient on a rounded quad (left -> right).
fn rr_gradient(r: Rect, radius: f32, left: [f32; 4], right: [f32; 4]) -> GpuQuad {
    let mut q = rr_rect(r, radius, left);
    q.vertices[1].color = right;
    q.vertices[2].color = right;
    q
}

fn icon_in(icon: &GlyphInfo, r: Rect, color: [f32; 4]) -> GpuQuad {
    NativeGpuCompositor::icon_quad(r.cx() - icon.width / 2.0, r.cy() - icon.height / 2.0, icon.width, icon, color)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Security {
    Secure,
    Insecure,
    Internal,
}

impl Security {
    pub fn from_address(address: &str) -> Self {
        if address.starts_with("https://") {
            Security::Secure
        } else if address.starts_with("http://") {
            Security::Insecure
        } else {
            Security::Internal
        }
    }
}

pub struct ChromeState<'a> {
    pub theme: &'a Theme,
    pub security: Security,
    pub shield_count: u32,
    pub bookmarked: bool,
    pub address_selected: bool,
    pub avatar: &'a str,
    /// A private (incognito) window.
    pub private: bool,
    /// Zoom percentage to show in the address bar (`None` at 100%).
    pub zoom: Option<u32>,
}

pub fn build_toolbar_quads(
    compositor: &mut NativeGpuCompositor,
    viewport_w: f32,
    address_text: &str,
    focused: bool,
    address_cursor: usize,
    tabs: &[TabItem],
    active_tab_idx: usize,
    history_back: bool,
    history_fwd: bool,
    icons: &ToolbarIcons,
    st: &ChromeState,
) -> Vec<GpuQuad> {
    let mut quads = Vec::new();

    // Palette comes from the active heritage theme (same values as ui/style.css).
    let th = st.theme;
    let rgb = |v: [u8; 3]| c(v[0], v[1], v[2], 255);
    let titlebar_bg = rgb(th.titlebar);
    let toolbar_bg = rgb(th.toolbar);
    let border = c(th.primary[0], th.primary[1], th.primary[2], if th.dark { 60 } else { 51 });
    let heading = rgb(th.heading);
    let main_text = rgb(th.text_main);
    let muted = rgb(th.muted);
    let primary = rgb(th.primary);
    let primary_light = c(th.primary_light[0], th.primary_light[1], th.primary_light[2], th.primary_light[3]);
    let accent = rgb(th.accent);
    let white = c(255, 255, 255, 255);
    let emerald = c(th.primary[0].saturating_add(11), th.primary[1].saturating_add(35), th.primary[2].saturating_add(24), 255);
    let field_bg = if th.dark { c(30, 41, 59, 255) } else { white };

    // ---- titlebar: brand + tabs
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, 0.0, viewport_w - SIDEBAR_W, TAB_BAR_H, titlebar_bg));
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, TAB_BAR_H - 1.0, viewport_w - SIDEBAR_W, 1.0, border));

    let badge = Rect::new(SIDEBAR_W + 12.0, (TAB_BAR_H - 24.0) / 2.0, 24.0, 24.0);
    quads.push(rr(badge.x, badge.y + 1.0, 24.0, 24.0, 6.0, c(0, 0, 0, 30)));
    quads.push(rr_gradient(badge, 6.0, primary, emerald));
    render_text(compositor, &mut quads, "A", badge.x + 7.0, badge.y + 17.5, 14.0, white, badge.right());
    let name_end = render_text(compositor, &mut quads, "Axomai", badge.right() + 8.0, 28.0, 14.0, heading, TAB_START_X);
    if st.private {
        render_text(compositor, &mut quads, "Incognito", name_end + 4.0, 28.0, 12.0, c(168, 85, 247, 255), TAB_START_X + 40.0);
    } else {
        render_text(compositor, &mut quads, "Browser", name_end + 4.0, 28.0, 12.0, primary, TAB_START_X);
    }

    let strip = tab_strip(viewport_w, tabs);
    for (i, t) in tabs.iter().enumerate() {
        let r = strip.tab_rect(i);
        let is_active = i == active_tab_idx;
        let title = if t.title.is_empty() { "New Tab" } else { &t.title };
        let wide = r.w >= 110.0;
        let close = strip.close_rect(i);
        let show_close = close_visible(t, &r, is_active);
        let show_speaker = speaker_visible(t, &r);
        let icon_box = if t.pinned || !wide { Rect::new(r.cx() - 8.0, r.cy() - 8.0, 16.0, 16.0) } else { Rect::new(r.x + 13.0, r.cy() - 8.0, 16.0, 16.0) };
        let text_x = icon_box.right() + 8.0;
        let text_max = (if show_speaker { strip.speaker_rect(i).x } else if show_close { close.x } else { r.right() - 10.0 }) - 2.0;
        let icon_alpha = if t.loading || t.sleeping { 0.55 } else { 1.0 };
        let text_color = if is_active { heading } else if t.sleeping { c(th.muted[0], th.muted[1], th.muted[2], 150) } else { muted };

        if is_active {
            // Rounded top corners only: the quad runs past the titlebar and the toolbar paints over its foot.
            quads.push(rr(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 14.0, 12.0, c(th.primary[0], th.primary[1], th.primary[2], 90)));
            quads.push(rr(r.x, r.y, r.w, r.h + 13.0, 12.0, toolbar_bg));
            quads.push(rr(r.x + 12.0_f32.min(r.w / 4.0), r.y, r.w - 2.0 * 12.0_f32.min(r.w / 4.0), 2.5, 1.25, if t.private { c(168, 85, 247, 255) } else { primary }));
        } else if t.pinned {
            quads.push(rr_rect(r, 10.0, c(th.primary[0], th.primary[1], th.primary[2], 28)));
        }

        // favicon, or the green "A" placeholder
        match t.favicon {
            Some(slot) => quads.push(Favicons::quad(slot, icon_box.x, icon_box.y, 16.0, icon_alpha)),
            None => {
                let base = if t.private { c(168, 85, 247, 255) } else if is_active { emerald } else { c(th.primary[0], th.primary[1], th.primary[2], 150) };
                quads.push(rr_rect(Rect::new(icon_box.x + 0.5, icon_box.y + 0.5, 15.0, 15.0), 3.5, base));
                render_text(compositor, &mut quads, if t.private { "P" } else { "A" }, icon_box.x + 4.0, icon_box.y + 12.5, 10.5, white, icon_box.right());
            }
        }
        if t.pinned && t.audio {
            // Pinned tabs have no room for a speaker button; a dot marks that this tab is playing sound.
            quads.push(rr(r.right() - 11.0, r.y + 7.0, 6.0, 6.0, 3.0, accent));
        }
        if wide && !t.pinned {
            let shown = fit_text(compositor, title, 12.5, text_max - text_x);
            render_text(compositor, &mut quads, &shown, text_x, r.cy() + 4.5, 12.5, text_color, text_max);
        }
        if show_speaker {
            let icon = if t.muted { &icons.speaker_off } else { &icons.speaker };
            quads.push(icon_in(icon, strip.speaker_rect(i), if t.muted { muted } else { primary }));
        }
        if show_close {
            quads.push(icon_in(&icons.close, close, if is_active { muted } else { c(100, 116, 139, 160) }));
        }
    }
    quads.push(icon_in(&icons.plus, strip.plus, muted));

    // ---- navigation toolbar
    let ty = TAB_BAR_H;
    let l = toolbar_layout(viewport_w);
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty, viewport_w - SIDEBAR_W, TOOLBAR_H, toolbar_bg));
    quads.push(NativeGpuCompositor::solid_quad(SIDEBAR_W, ty + TOOLBAR_H - 1.0, viewport_w - SIDEBAR_W, 1.0, border));

    let dim = c(th.text_main[0], th.text_main[1], th.text_main[2], 85);
    quads.push(icon_in(&icons.back, l.back, if history_back { main_text } else { dim }));
    quads.push(icon_in(&icons.forward, l.forward, if history_fwd { main_text } else { dim }));
    quads.push(icon_in(&icons.reload, l.reload, main_text));
    quads.push(icon_in(&icons.home, l.home, main_text));

    // omnibox
    let o = l.omnibox;
    if focused {
        quads.push(rr(o.x - 3.0, o.y - 3.0, o.w + 6.0, o.h + 6.0, 21.0, c(th.primary[0], th.primary[1], th.primary[2], 64)));
        quads.push(rr_rect(o, 18.0, primary));
    } else {
        quads.push(rr(o.x, o.y + 2.0, o.w, o.h + 1.0, 18.0, c(th.primary[0], th.primary[1], th.primary[2], 14)));
        quads.push(rr_rect(o, 18.0, border));
    }
    quads.push(rr(o.x + 1.0, o.y + 1.0, o.w - 2.0, o.h - 2.0, 17.0, field_bg));

    let sb = l.secure_badge;
    let (badge_bg, badge_fg, badge_label) = match st.security {
        Security::Secure => (primary_light, primary, "Secure"),
        Security::Internal => (primary_light, primary, "Axomai"),
        Security::Insecure => (c(254, 226, 226, 255), c(220, 38, 38, 255), "HTTP"),
    };
    quads.push(rr_rect(sb, 11.0, badge_bg));
    let shield_tint = if st.security == Security::Insecure { badge_fg } else { emerald };
    quads.push(NativeGpuCompositor::icon_quad(sb.x + 8.0, sb.cy() - 7.0, 14.0, &icons.shield_outline, shield_tint));
    render_text(compositor, &mut quads, badge_label, sb.x + 26.0, sb.cy() + 4.0, 11.5, badge_fg, sb.right());

    quads.push(rr_rect(l.engine_icon, 4.0, emerald));
    render_text(compositor, &mut quads, "A", l.engine_icon.x + 4.0, l.engine_icon.y + 12.5, 11.0, white, l.engine_icon.right());

    let is_home = address_text == "about:home" || address_text == "axomai://home" || address_text == "axomai://newtab"
        || address_text.contains("axomai_home.html") || address_text.is_empty();
    let placeholder = is_home && !focused;
    let display = if placeholder {
        "Search with Axomai or enter address..."
    } else if is_home && focused && address_text == "about:home" {
        ""
    } else {
        address_text
    };
    let text_color = if placeholder { muted } else { heading };
    let base_y = o.cy() + 4.5;
    let caret_h = 18.0;
    if focused && st.address_selected && !display.is_empty() {
        let w = text_width(compositor, display, 13.5).min(l.text_max_x - l.text_x);
        quads.push(rr(l.text_x - 2.0, o.cy() - 10.0, w + 4.0, 20.0, 4.0, c(th.primary[0], th.primary[1], th.primary[2], 90)));
        render_text(compositor, &mut quads, display, l.text_x, base_y, 13.5, text_color, l.text_max_x);
    } else if focused && !display.is_empty() {
        let before: String = display.chars().take(address_cursor).collect();
        let caret_x = render_text(compositor, &mut quads, &before, l.text_x, base_y, 13.5, text_color, l.text_max_x);
        let after: String = display.chars().skip(address_cursor).collect();
        if !after.is_empty() {
            render_text(compositor, &mut quads, &after, caret_x, base_y, 13.5, text_color, l.text_max_x);
        }
        quads.push(NativeGpuCompositor::solid_quad(caret_x + 1.0, o.cy() - caret_h / 2.0, 1.5, caret_h, primary));
    } else {
        let end_x = render_text(compositor, &mut quads, display, l.text_x, base_y, 13.5, text_color, l.text_max_x);
        if focused {
            quads.push(NativeGpuCompositor::solid_quad(end_x + 1.0, o.cy() - caret_h / 2.0, 1.5, caret_h, primary));
        }
    }

    if let Some(pct) = st.zoom {
        quads.push(rr_rect(l.zoom, 11.0, primary_light));
        render_text_centered(compositor, &mut quads, &format!("{}%", pct), l.zoom.cx(), l.zoom.cy() + 4.0, 11.5, primary);
    }
    if st.bookmarked {
        quads.push(icon_in(&icons.star_filled, l.bookmark, accent));
    } else {
        quads.push(icon_in(&icons.star, l.bookmark, muted));
    }
    quads.push(icon_in(&icons.book, l.reader, muted));
    quads.push(icon_in(&icons.qr, l.qr, muted));

    // right-hand actions
    let s = l.shield;
    quads.push(rr_rect(s, 13.0, border));
    quads.push(rr(s.x + 1.0, s.y + 1.0, s.w - 2.0, s.h - 2.0, 12.0, primary_light));
    quads.push(NativeGpuCompositor::icon_quad(s.x + 10.0, s.cy() - 7.0, 14.0, &icons.shield_filled, emerald));
    let count_text = if st.shield_count > 999 { "999+".to_string() } else { st.shield_count.to_string() };
    render_text(compositor, &mut quads, &count_text, s.x + 29.0, s.cy() + 4.5, 12.5, primary, s.right());

    quads.push(icon_in(&icons.puzzle, l.extensions, main_text));
    quads.push(icon_in(&icons.download, l.downloads, main_text));
    quads.push(icon_in(&icons.sun, l.theme, main_text));

    let ai = l.ai;
    quads.push(rr(ai.x, ai.y + 2.0, ai.w, ai.h, 14.0, c(th.primary[0], th.primary[1], th.primary[2], 50)));
    quads.push(rr_gradient(ai, 14.0, primary, accent));
    quads.push(NativeGpuCompositor::icon_quad(ai.x + 10.0, ai.cy() - 6.5, 13.0, &icons.sparkle, c(254, 240, 138, 255)));
    let ai_text_w = text_width(compositor, "AI", 12.5);
    render_text(compositor, &mut quads, "AI", ai.x + 27.0, ai.cy() + 4.5, 12.5, white, ai.x + 27.0 + ai_text_w + 4.0);

    let p = l.profile;
    quads.push(rr_rect(p, 16.0, white));
    quads.push(rr_gradient(Rect::new(p.x + 2.0, p.y + 2.0, 28.0, 28.0), 14.0, primary, emerald));
    render_text_centered(compositor, &mut quads, st.avatar, p.cx(), p.cy() + 5.0, 14.0, white);
    quads.push(rr(p.right() - 11.0, p.y + p.h - 11.0, 11.0, 11.0, 5.5, white));
    quads.push(rr(p.right() - 9.5, p.y + p.h - 9.5, 8.0, 8.0, 4.0, emerald));

    quads.push(icon_in(&icons.menu, l.menu, main_text));

    quads
}
