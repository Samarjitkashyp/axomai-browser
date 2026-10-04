//! Tab favicons. Decoded icons are packed into one RGBA atlas that is uploaded to the GPU as the chrome's
//! image texture, so a tab's icon is a single textured quad (the glyph atlas is alpha-only and cannot hold colour).

use axomai_engine::{GpuQuad, GpuVertex};
use std::collections::HashMap;

pub const CELL: u32 = 32;
pub const ATLAS: u32 = 512;
const PER_ROW: u32 = ATLAS / CELL;
const SLOTS: usize = (PER_ROW * PER_ROW) as usize;

pub struct Favicons {
    pixels: Vec<u8>,
    by_hash: HashMap<u64, usize>,
    next: usize,
    /// True when `pixels` changed since the last GPU upload.
    pub dirty: bool,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl Favicons {
    pub fn new() -> Self {
        Favicons { pixels: vec![0; (ATLAS * ATLAS * 4) as usize], by_hash: HashMap::new(), next: 0, dirty: true }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Decode PNG/JPEG bytes, scale to a cell and store it. Identical images share a slot.
    pub fn add(&mut self, bytes: &[u8]) -> Option<usize> {
        let key = fnv1a(bytes);
        if let Some(slot) = self.by_hash.get(&key) {
            return Some(*slot);
        }
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        if img.width() == 0 || img.height() == 0 {
            return None;
        }
        let img = image::imageops::resize(&img, CELL, CELL, image::imageops::FilterType::Triangle);
        let slot = self.next % SLOTS;
        self.next += 1;
        // When the atlas wraps around, forget whichever icon used the slot we are about to overwrite.
        self.by_hash.retain(|_, s| *s != slot);
        let (cx, cy) = ((slot as u32 % PER_ROW) * CELL, (slot as u32 / PER_ROW) * CELL);
        for y in 0..CELL {
            for x in 0..CELL {
                let p = img.get_pixel(x, y).0;
                let dst = (((cy + y) * ATLAS + cx + x) * 4) as usize;
                self.pixels[dst..dst + 4].copy_from_slice(&p);
            }
        }
        self.by_hash.insert(key, slot);
        self.dirty = true;
        Some(slot)
    }

    fn uv(slot: usize) -> [f32; 4] {
        let (cx, cy) = ((slot as u32 % PER_ROW) as f32, (slot as u32 / PER_ROW) as f32);
        let a = ATLAS as f32;
        // Half-texel inset so bilinear filtering never bleeds in the neighbouring icon.
        let e = 0.5 / a;
        [cx * CELL as f32 / a + e, cy * CELL as f32 / a + e, (cx + 1.0) * CELL as f32 / a - e, (cy + 1.0) * CELL as f32 / a - e]
    }

    /// A quad drawing `slot` at (x, y) with the given edge length.
    pub fn quad(slot: usize, x: f32, y: f32, size: f32, alpha: f32) -> GpuQuad {
        let [u0, v0, u1, v1] = Self::uv(slot);
        let c = [1.0, 1.0, 1.0, alpha];
        GpuQuad {
            vertices: [
                GpuVertex { position: [x, y], uv: [u0, v0], color: c },
                GpuVertex { position: [x + size, y], uv: [u1, v0], color: c },
                GpuVertex { position: [x + size, y + size], uv: [u1, v1], color: c },
                GpuVertex { position: [x, y + size], uv: [u0, v1], color: c },
            ],
            indices: [0, 1, 2, 0, 2, 3],
            clip_rect: None,
            opacity: 1.0,
            is_textured: true,
            texture_mode: 2,
            rect_bounds: None,
            corner_radius: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(color: [u8; 4], size: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(size, size, image::Rgba(color));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).write_to(&mut out, image::ImageOutputFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn identical_icons_share_a_slot_and_different_ones_do_not() {
        let mut f = Favicons::new();
        let a = f.add(&png([255, 0, 0, 255], 16)).unwrap();
        let a2 = f.add(&png([255, 0, 0, 255], 16)).unwrap();
        let b = f.add(&png([0, 0, 255, 255], 16)).unwrap();
        assert_eq!(a, a2);
        assert_ne!(a, b);
    }

    #[test]
    fn pixels_land_in_the_right_cell() {
        let mut f = Favicons::new();
        let slot = f.add(&png([10, 20, 30, 255], 8)).unwrap();
        let (cx, cy) = ((slot as u32 % PER_ROW) * CELL, (slot as u32 / PER_ROW) * CELL);
        let i = (((cy + 5) * ATLAS + cx + 5) * 4) as usize;
        assert_eq!(&f.pixels()[i..i + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(Favicons::new().add(b"not an image").is_none());
    }

    #[test]
    fn atlas_wraps_without_panicking() {
        let mut f = Favicons::new();
        for i in 0..(SLOTS as u32 + 3) {
            assert!(f.add(&png([(i % 250) as u8, (i / 250) as u8, 7, 255], 4)).is_some());
        }
    }

    #[test]
    fn uv_stays_inside_the_cell() {
        let [u0, v0, u1, v1] = Favicons::uv(0);
        assert!(u0 > 0.0 && v0 > 0.0 && u1 < CELL as f32 / ATLAS as f32 && v1 < CELL as f32 / ATLAS as f32);
    }
}
