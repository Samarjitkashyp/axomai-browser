use std::collections::HashMap;

const ATLAS_SIZE: u32 = 1024;
const BUILTIN_FONT: &[u8] = include_bytes!("fonts/DejaVuSans.ttf");

#[derive(Clone, Copy)]
pub struct GlyphInfo {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub width: f32,
    pub height: f32,
    pub advance_width: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

pub struct GlyphAtlas {
    font: fontdue::Font,
    pub pixels: Vec<u8>,
    pub size: u32,
    cache: HashMap<(char, u32), GlyphInfo>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    pub dirty: bool,
}

impl GlyphAtlas {
    pub fn new() -> Self {
        let font = fontdue::Font::from_bytes(BUILTIN_FONT, fontdue::FontSettings::default())
            .expect("Failed to load built-in font");

        GlyphAtlas {
            font,
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            size: ATLAS_SIZE,
            cache: HashMap::new(),
            cursor_x: 1,
            cursor_y: 1,
            row_height: 0,
            dirty: true,
        }
    }

    pub fn rasterize(&mut self, ch: char, font_size_px: f32) -> GlyphInfo {
        let key = (ch, (font_size_px * 4.0) as u32);
        if let Some(info) = self.cache.get(&key) {
            return *info;
        }

        let (metrics, bitmap) = self.font.rasterize(ch, font_size_px);
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        if gw == 0 || gh == 0 {
            let info = GlyphInfo {
                u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                width: 0.0, height: 0.0,
                advance_width: metrics.advance_width,
                offset_x: metrics.xmin as f32,
                offset_y: metrics.ymin as f32,
            };
            self.cache.insert(key, info);
            return info;
        }

        if self.cursor_x + gw + 1 > self.size {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }

        if self.cursor_y + gh + 1 > self.size {
            self.cursor_x = 1;
            self.cursor_y = 1;
            self.row_height = 0;
            self.cache.clear();
        }

        let ox = self.cursor_x;
        let oy = self.cursor_y;
        for row in 0..gh {
            for col in 0..gw {
                let src = (row * gw + col) as usize;
                let dst = ((oy + row) * self.size + ox + col) as usize;
                if src < bitmap.len() && dst < self.pixels.len() {
                    self.pixels[dst] = bitmap[src];
                }
            }
        }

        let info = GlyphInfo {
            u0: ox as f32 / self.size as f32,
            v0: oy as f32 / self.size as f32,
            u1: (ox + gw) as f32 / self.size as f32,
            v1: (oy + gh) as f32 / self.size as f32,
            width: gw as f32,
            height: gh as f32,
            advance_width: metrics.advance_width,
            offset_x: metrics.xmin as f32,
            offset_y: metrics.ymin as f32,
        };

        self.cursor_x += gw + 1;
        if gh > self.row_height {
            self.row_height = gh;
        }

        self.cache.insert(key, info);
        self.dirty = true;
        info
    }

    /// Blit an RGBA icon into the atlas, converting to grayscale alpha.
    /// Returns a GlyphInfo with UV coordinates for the icon.
    pub fn blit_icon(&mut self, name: &str, rgba: &[u8], width: u32, height: u32, target_size: u32) -> GlyphInfo {
        let key_char = match name {
            "back" => '\u{E000}',
            "forward" => '\u{E001}',
            "home" => '\u{E002}',
            "menu" => '\u{E003}',
            _ => '\u{E010}',
        };
        let key = (key_char, target_size);
        if let Some(info) = self.cache.get(&key) {
            return *info;
        }

        let ts = target_size;
        if self.cursor_x + ts + 1 > self.size {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }

        let ox = self.cursor_x;
        let oy = self.cursor_y;

        for row in 0..ts {
            for col in 0..ts {
                let src_x = (col as f32 / ts as f32 * width as f32) as u32;
                let src_y = (row as f32 / ts as f32 * height as f32) as u32;
                let src_idx = ((src_y * width + src_x) * 4) as usize;
                if src_idx + 3 < rgba.len() {
                    let r = rgba[src_idx] as f32;
                    let g = rgba[src_idx + 1] as f32;
                    let b = rgba[src_idx + 2] as f32;
                    let a = rgba[src_idx + 3] as f32 / 255.0;
                    // Invert: dark pixels → high alpha, white → transparent
                    let luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255.0;
                    let coverage = (1.0 - luminance) * a;
                    let dst = ((oy + row) * self.size + ox + col) as usize;
                    if dst < self.pixels.len() {
                        self.pixels[dst] = (coverage * 255.0) as u8;
                    }
                }
            }
        }

        let info = GlyphInfo {
            u0: ox as f32 / self.size as f32,
            v0: oy as f32 / self.size as f32,
            u1: (ox + ts) as f32 / self.size as f32,
            v1: (oy + ts) as f32 / self.size as f32,
            width: ts as f32,
            height: ts as f32,
            advance_width: ts as f32,
            offset_x: 0.0,
            offset_y: 0.0,
        };

        self.cursor_x += ts + 1;
        if ts > self.row_height {
            self.row_height = ts;
        }

        self.cache.insert(key, info);
        self.dirty = true;
        info
    }

    /// Blit a ready-made `size`x`size` alpha mask (one byte per pixel) into the atlas.
    /// `id` picks a private-use slot so differently drawn icons never share a cache entry.
    pub fn blit_mask(&mut self, id: u32, alpha: &[u8], size: u32) -> GlyphInfo {
        let key = (char::from_u32(0xE100 + id).unwrap_or('\u{E100}'), size);
        if let Some(info) = self.cache.get(&key) {
            return *info;
        }
        if self.cursor_x + size + 1 > self.size {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }
        let ox = self.cursor_x;
        let oy = self.cursor_y;
        for row in 0..size {
            for col in 0..size {
                let dst = ((oy + row) * self.size + ox + col) as usize;
                if let (Some(px), Some(a)) = (self.pixels.get_mut(dst), alpha.get((row * size + col) as usize)) {
                    *px = *a;
                }
            }
        }
        let info = GlyphInfo {
            u0: ox as f32 / self.size as f32,
            v0: oy as f32 / self.size as f32,
            u1: (ox + size) as f32 / self.size as f32,
            v1: (oy + size) as f32 / self.size as f32,
            width: size as f32,
            height: size as f32,
            advance_width: size as f32,
            offset_x: 0.0,
            offset_y: 0.0,
        };
        self.cursor_x += size + 1;
        if size > self.row_height {
            self.row_height = size;
        }
        self.cache.insert(key, info);
        self.dirty = true;
        info
    }
}
