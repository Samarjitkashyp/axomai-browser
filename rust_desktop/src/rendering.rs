use axomai_engine::{GpuQuad, NativeGpuCompositor, WgpuRenderer};

pub fn w_of(r: &WgpuRenderer) -> f32 {
    r.surface_config.width as f32
}

pub fn h_of(r: &WgpuRenderer) -> f32 {
    r.surface_config.height as f32
}

/// The swap-chain is sRGB, so colours written by the shader are treated as linear light. Convert from the sRGB
/// values used in CSS/design tokens, otherwise every colour comes out lighter than the web UI it mirrors.
fn srgb_to_linear(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

pub fn c(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a as f32 / 255.0]
}

/// Draw `text` starting at `x` (baseline `y`) and return the x where it ended. Stops once `max_x` is passed.
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
        if x > max_x {
            break;
        }
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
    let total_w = text_width(compositor, text, size);
    render_text(compositor, quads, text, center_x - total_w / 2.0, y, size, color, center_x + total_w);
}

pub fn text_width(compositor: &mut NativeGpuCompositor, text: &str, size: f32) -> f32 {
    text.chars().map(|ch| compositor.glyph_atlas.rasterize(ch, size).advance_width).sum()
}

/// `text` shortened with an ellipsis so that it fits in `max_w` pixels.
pub fn fit_text(compositor: &mut NativeGpuCompositor, text: &str, size: f32, max_w: f32) -> String {
    if max_w <= 0.0 {
        return String::new();
    }
    if text_width(compositor, text, size) <= max_w {
        return text.to_string();
    }
    let ellipsis = compositor.glyph_atlas.rasterize('\u{2026}', size).advance_width;
    let mut out = String::new();
    let mut w = 0.0;
    for ch in text.chars() {
        let cw = compositor.glyph_atlas.rasterize(ch, size).advance_width;
        if w + cw + ellipsis > max_w {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('\u{2026}');
    out
}
