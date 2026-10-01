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
}
