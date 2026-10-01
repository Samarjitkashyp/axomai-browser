//! Native GPU Compositor & Direct Pixel Surface Rasterizer for Axomai Browser.
//! Renders DisplayLists directly to RGBA hardware surface framebuffers without external browser engines.

use crate::painter::DisplayCommand;

#[derive(Debug, Clone)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct GpuQuad {
    pub vertices: [GpuVertex; 4],
    pub indices: [u32; 6],
    pub clip_rect: Option<[f32; 4]>,
    pub opacity: f32,
}

pub struct NativeFramebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA 32-bit pixel buffer
}

impl NativeFramebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        NativeFramebuffer {
            width,
            height,
            pixels: vec![255u8; size], // White background default
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }

    pub fn draw_solid_rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: (u8, u8, u8, u8)) {
        let start_x = (x1.max(0.0) as u32).min(self.width);
        let end_x = (x2.max(0.0) as u32).min(self.width);
        let start_y = (y1.max(0.0) as u32).min(self.height);
        let end_y = (y2.max(0.0) as u32).min(self.height);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = ((y * self.width + x) * 4) as usize;
                if idx + 3 < self.pixels.len() {
                    self.pixels[idx] = color.0;     // R
                    self.pixels[idx + 1] = color.1; // G
                    self.pixels[idx + 2] = color.2; // B
                    self.pixels[idx + 3] = color.3; // A
                }
            }
        }
    }
}

pub struct NativeGpuCompositor {
    pub width: u32,
    pub height: u32,
    pub framebuffer: NativeFramebuffer,
    pub quads: Vec<GpuQuad>,
}

impl NativeGpuCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        NativeGpuCompositor {
            width,
            height,
            framebuffer: NativeFramebuffer::new(width, height),
            quads: Vec::new(),
        }
    }

    /// Convert DisplayCommand list into GPU hardware quad primitives
    pub fn composite_display_list(&mut self, display_list: &[DisplayCommand], scroll_y: f32) {
        self.quads.clear();
        self.framebuffer.clear(255, 255, 255, 255);

        for cmd in display_list {
            match cmd {
                DisplayCommand::DrawRect { x1, y1, x2, y2, color, .. } => {
                    let (r, g, b, a) = Self::parse_color_hex(color);
                    let y1_adj = y1 - scroll_y;
                    let y2_adj = y2 - scroll_y;

                    // Rasterize directly to pixel framebuffer
                    self.framebuffer.draw_solid_rect(*x1, y1_adj, *x2, y2_adj, (r, g, b, a));

                    // And record GPU vertex quad for hardware GPU pipelines (wgpu / DirectX / Vulkan)
                    let quad = GpuQuad {
                        vertices: [
                            GpuVertex { position: [*x1, y1_adj], uv: [0.0, 0.0], color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0] },
                            GpuVertex { position: [*x2, y1_adj], uv: [1.0, 0.0], color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0] },
                            GpuVertex { position: [*x2, y2_adj], uv: [1.0, 1.0], color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0] },
                            GpuVertex { position: [*x1, y2_adj], uv: [0.0, 1.0], color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0] },
                        ],
                        indices: [0, 1, 2, 0, 2, 3],
                        clip_rect: None,
                        opacity: 1.0,
                    };
                    self.quads.push(quad);
                }
                _ => {}
            }
        }
    }

    fn parse_color_hex(hex: &str) -> (u8, u8, u8, u8) {
        let clean = hex.trim().trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            (r, g, b, 255)
        } else if clean.len() == 8 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255);
            (r, g, b, a)
        } else {
            (15, 23, 42, 255) // Default slate dark
        }
    }
}
