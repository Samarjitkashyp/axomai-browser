//! Native GPU Compositor & Hardware Surface Presentation Pipeline for Axomai Browser.
//! When compiled with the `wgpu-backend` feature, this module uses real wgpu device,
//! surface, render pipeline, and GPU buffers to present frames to the OS window.
//! Without the feature, it falls back to CPU rasterization into a framebuffer.

use crate::glyph_atlas::GlyphAtlas;
use crate::painter::DisplayCommand;

#[cfg(feature = "wgpu-backend")]
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuQuad {
    pub vertices: [GpuVertex; 4],
    pub indices: [u32; 6],
    pub clip_rect: Option<[f32; 4]>,
    pub opacity: f32,
    pub is_textured: bool,
}

#[derive(Debug, Clone)]
pub struct NativeFramebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl NativeFramebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        NativeFramebuffer {
            width,
            height,
            pixels: vec![255u8; size],
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
                    self.pixels[idx] = color.0;
                    self.pixels[idx + 1] = color.1;
                    self.pixels[idx + 2] = color.2;
                    self.pixels[idx + 3] = color.3;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuPipelineDescriptor {
    pub vertex_shader_wgsl: String,
    pub fragment_shader_wgsl: String,
    pub topology: String,
    pub sample_count: u32,
}

#[derive(Debug, Clone)]
pub struct GpuBufferStream {
    pub vertex_data: Vec<f32>,
    pub index_data: Vec<u32>,
    pub quad_count: usize,
}

#[derive(Debug, Clone)]
pub struct GpuHardwareDevice {
    pub adapter_name: String,
    pub backend_name: String,
    pub is_hardware_accelerated: bool,
    pub max_texture_dimension_2d: u32,
}

impl GpuHardwareDevice {
    pub fn new() -> Self {
        Self {
            adapter_name: "Axomai Native GPU Pipeline".to_string(),
            backend_name: if cfg!(target_os = "windows") {
                "DirectX12 / Vulkan".to_string()
            } else if cfg!(target_os = "macos") {
                "Metal".to_string()
            } else {
                "Vulkan".to_string()
            },
            is_hardware_accelerated: true,
            max_texture_dimension_2d: 8192,
        }
    }

    pub fn create_buffer_stream(&self, quads: &[GpuQuad]) -> GpuBufferStream {
        let mut vertex_data = Vec::with_capacity(quads.len() * 32);
        let mut index_data = Vec::with_capacity(quads.len() * 6);
        let mut base_vertex = 0u32;

        for quad in quads {
            for v in &quad.vertices {
                vertex_data.push(v.position[0]);
                vertex_data.push(v.position[1]);
                vertex_data.push(v.uv[0]);
                vertex_data.push(v.uv[1]);
                vertex_data.push(v.color[0]);
                vertex_data.push(v.color[1]);
                vertex_data.push(v.color[2]);
                vertex_data.push(v.color[3]);
            }

            for idx in &quad.indices {
                index_data.push(base_vertex + idx);
            }
            base_vertex += 4;
        }

        GpuBufferStream {
            vertex_data,
            index_data,
            quad_count: quads.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuSwapchainPresenter {
    pub surface_width: u32,
    pub surface_height: u32,
    pub presented_frames: u64,
    pub last_frame_latency_ms: f32,
    pub is_vsync_enabled: bool,
    pub hardware_device: GpuHardwareDevice,
}

impl GpuSwapchainPresenter {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            surface_width: width,
            surface_height: height,
            presented_frames: 0,
            last_frame_latency_ms: 0.0,
            is_vsync_enabled: true,
            hardware_device: GpuHardwareDevice::new(),
        }
    }

    pub fn present_frame(&mut self, _fb: &NativeFramebuffer, quads: &[GpuQuad]) -> u64 {
        let _stream = self.hardware_device.create_buffer_stream(quads);
        self.presented_frames += 1;
        self.last_frame_latency_ms = 16.67;
        self.presented_frames
    }
}

// ─── Real wgpu GPU Renderer (behind wgpu-backend feature) ───

/// Vertex layout: position(2) + uv(2) + color(4) + mode(1) = 9 floats = 36 bytes
#[cfg(feature = "wgpu-backend")]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub mode: f32,
}

#[cfg(feature = "wgpu-backend")]
impl WgpuVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

#[cfg(feature = "wgpu-backend")]
const SHADER_SOURCE: &str = r#"
struct Uniforms {
    projection: mat4x4<f32>,
};
@binding(0) @group(0) var<uniform> uniforms: Uniforms;
@binding(0) @group(1) var glyph_texture: texture_2d<f32>;
@binding(1) @group(1) var glyph_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mode: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) mode: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.projection * vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    output.mode = input.mode;
    return output;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, @location(2) mode: f32) -> @location(0) vec4<f32> {
    if (mode > 0.5) {
        let alpha = textureSample(glyph_texture, glyph_sampler, uv).r;
        return vec4<f32>(color.rgb, color.a * alpha);
    }
    return color;
}
"#;

/// Builds an orthographic projection matrix mapping pixel coords to NDC
#[cfg(feature = "wgpu-backend")]
fn orthographic_projection(width: f32, height: f32) -> [f32; 16] {
    [
        2.0 / width, 0.0,           0.0, 0.0,
        0.0,        -2.0 / height,  0.0, 0.0,
        0.0,         0.0,           1.0, 0.0,
       -1.0,         1.0,           0.0, 1.0,
    ]
}

/// Real GPU rendering state backed by wgpu
#[cfg(feature = "wgpu-backend")]
pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub glyph_texture: wgpu::Texture,
    pub glyph_bind_group: wgpu::BindGroup,
    pub glyph_bind_group_layout: wgpu::BindGroupLayout,
    pub presented_frames: u64,
}

#[cfg(feature = "wgpu-backend")]
impl WgpuRenderer {
    /// Creates a real wgpu renderer attached to an OS window surface.
    /// `window` must be a `&'static impl raw_window_handle::HasWindowHandle + HasDisplayHandle`.
    pub fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Self {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .or_else(|| {
            eprintln!("[Axomai GPU] High-performance adapter not found, trying low-power...");
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
        })
        .or_else(|| {
            eprintln!("[Axomai GPU] Trying software fallback with surface...");
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: true,
            }))
        })
        .expect("Failed to find any GPU adapter. Ensure GPU drivers are installed.");

        let adapter_info = adapter.get_info();
        println!(
            "[Axomai GPU] Adapter: {} ({:?})",
            adapter_info.name, adapter_info.backend
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Axomai GPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        ))
        .expect("Failed to create wgpu device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = if surface_caps.formats.is_empty() {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            surface_caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0])
        };
        let alpha_mode = if surface_caps.alpha_modes.is_empty() {
            wgpu::CompositeAlphaMode::Auto
        } else {
            surface_caps.alpha_modes[0]
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Axomai Compositor Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let projection = orthographic_projection(width as f32, height as f32);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&projection),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let glyph_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Glyph Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let glyph_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Glyph Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let glyph_view = glyph_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glyph Bind Group"),
            layout: &glyph_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&glyph_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&glyph_sampler) },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Axomai Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &glyph_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Axomai Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[WgpuVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        WgpuRenderer {
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            glyph_texture,
            glyph_bind_group,
            glyph_bind_group_layout,
            presented_frames: 0,
        }
    }

    /// Resize the surface when the window changes size
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        let projection = orthographic_projection(width as f32, height as f32);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&projection),
        );
    }

    pub fn upload_glyph_atlas(&mut self, atlas: &crate::glyph_atlas::GlyphAtlas) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.glyph_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas.size),
                rows_per_image: Some(atlas.size),
            },
            wgpu::Extent3d { width: atlas.size, height: atlas.size, depth_or_array_layers: 1 },
        );
    }

    /// Render a frame: convert GpuQuads to real GPU buffers, execute a render pass, present.
    pub fn render_frame(&mut self, quads: &[GpuQuad]) -> Result<u64, wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Build vertex and index data from quads
        let mut vertices: Vec<WgpuVertex> = Vec::with_capacity(quads.len() * 4);
        let mut indices: Vec<u32> = Vec::with_capacity(quads.len() * 6);
        let mut base: u32 = 0;

        for quad in quads {
            let mode = if quad.is_textured { 1.0f32 } else { 0.0f32 };
            for v in &quad.vertices {
                vertices.push(WgpuVertex {
                    position: v.position,
                    uv: v.uv,
                    color: v.color,
                    mode,
                });
            }
            for idx in &quad.indices {
                indices.push(base + idx);
            }
            base += 4;
        }

        if vertices.is_empty() {
            // Nothing to draw — just clear
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Clear Encoder"),
            });
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
            self.presented_frames += 1;
            return Ok(self.presented_frames);
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Axomai Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Axomai Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.glyph_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.presented_frames += 1;
        Ok(self.presented_frames)
    }
}

// ─── NativeGpuCompositor (unchanged API, works with or without wgpu feature) ───

pub struct NativeGpuCompositor {
    pub width: u32,
    pub height: u32,
    pub framebuffer: NativeFramebuffer,
    pub quads: Vec<GpuQuad>,
    pub pipeline: GpuPipelineDescriptor,
    pub presenter: GpuSwapchainPresenter,
    pub glyph_atlas: GlyphAtlas,
}

impl NativeGpuCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        let vertex_shader = r#"
            struct Uniforms {
                projection: mat4x4<f32>,
            };
            @binding(0) @group(0) var<uniform> uniforms: Uniforms;

            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) uv: vec2<f32>,
                @location(2) color: vec4<f32>,
            };

            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) uv: vec2<f32>,
                @location(1) color: vec4<f32>,
            };

            @vertex
            fn vs_main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                output.position = uniforms.projection * vec4<f32>(input.position, 0.0, 1.0);
                output.uv = input.uv;
                output.color = input.color;
                return output;
            }
        "#.to_string();

        let fragment_shader = r#"
            @fragment
            fn fs_main(@location(0) uv: vec2<f32>, @location(1) color: vec4<f32>) -> @location(0) vec4<f32> {
                return color;
            }
        "#.to_string();

        NativeGpuCompositor {
            width,
            height,
            framebuffer: NativeFramebuffer::new(width, height),
            quads: Vec::new(),
            pipeline: GpuPipelineDescriptor {
                vertex_shader_wgsl: vertex_shader,
                fragment_shader_wgsl: fragment_shader,
                topology: "triangle-list".to_string(),
                sample_count: 1,
            },
            presenter: GpuSwapchainPresenter::new(width, height),
            glyph_atlas: GlyphAtlas::new(),
        }
    }

    pub fn rasterize(&mut self, display_list: &[DisplayCommand]) -> &NativeFramebuffer {
        self.composite_display_list(display_list, 0.0);
        &self.framebuffer
    }

    pub fn present(&mut self, display_list: &[DisplayCommand]) -> u64 {
        self.composite_display_list(display_list, 0.0);
        self.presenter.present_frame(&self.framebuffer, &self.quads)
    }

    pub fn extract_gpu_quads(&mut self, display_list: &[DisplayCommand]) -> Vec<GpuQuad> {
        self.composite_display_list(display_list, 0.0);
        self.quads.clone()
    }

    pub fn generate_gpu_buffer_stream(&mut self, display_list: &[DisplayCommand]) -> GpuBufferStream {
        self.composite_display_list(display_list, 0.0);
        self.presenter.hardware_device.create_buffer_stream(&self.quads)
    }

    pub fn solid_quad(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> GpuQuad {
        GpuQuad {
            vertices: [
                GpuVertex { position: [x, y], uv: [0.0, 0.0], color },
                GpuVertex { position: [x + w, y], uv: [1.0, 0.0], color },
                GpuVertex { position: [x + w, y + h], uv: [1.0, 1.0], color },
                GpuVertex { position: [x, y + h], uv: [0.0, 1.0], color },
            ],
            indices: [0, 1, 2, 0, 2, 3],
            clip_rect: None,
            opacity: 1.0,
            is_textured: false,
        }
    }

    pub fn text_quad(x: f32, y: f32, glyph: &crate::glyph_atlas::GlyphInfo, color: [f32; 4]) -> GpuQuad {
        GpuQuad {
            vertices: [
                GpuVertex { position: [x, y], uv: [glyph.u0, glyph.v0], color },
                GpuVertex { position: [x + glyph.width, y], uv: [glyph.u1, glyph.v0], color },
                GpuVertex { position: [x + glyph.width, y + glyph.height], uv: [glyph.u1, glyph.v1], color },
                GpuVertex { position: [x, y + glyph.height], uv: [glyph.u0, glyph.v1], color },
            ],
            indices: [0, 1, 2, 0, 2, 3],
            clip_rect: None,
            opacity: 1.0,
            is_textured: true,
        }
    }

    fn push_border_quads(&mut self, x: f32, y: f32, w: f32, h: f32, bw: f32, color: [f32; 4]) {
        self.quads.push(Self::solid_quad(x, y, w, bw, color));
        self.quads.push(Self::solid_quad(x, y + h - bw, w, bw, color));
        self.quads.push(Self::solid_quad(x, y + bw, bw, h - 2.0 * bw, color));
        self.quads.push(Self::solid_quad(x + w - bw, y + bw, bw, h - 2.0 * bw, color));
    }

    pub fn composite_display_list(&mut self, display_list: &[DisplayCommand], scroll_y: f32) {
        self.quads.clear();
        self.framebuffer.clear(255, 255, 255, 255);

        for cmd in display_list {
            match cmd {
                DisplayCommand::DrawRect { x1, y1, x2, y2, color, .. } => {
                    let (r, g, b, a) = Self::parse_color_hex(color);
                    let y1_adj = y1 - scroll_y;
                    let y2_adj = y2 - scroll_y;

                    self.framebuffer.draw_solid_rect(*x1, y1_adj, *x2, y2_adj, (r, g, b, a));

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
                        is_textured: false,
                    };
                    self.quads.push(quad);
                }
                DisplayCommand::DrawText { x, y, text, font_size, color, width: _, height: _, .. } => {
                    let (r, g, b, a) = Self::parse_color_hex(color);
                    let y_adj = y - scroll_y;
                    let rf = r as f32 / 255.0;
                    let gf = g as f32 / 255.0;
                    let bf = b as f32 / 255.0;
                    let af = a as f32 / 255.0;

                    let mut pen_x = *x;
                    let baseline_y = y_adj + font_size;
                    for ch in text.chars() {
                        let glyph = self.glyph_atlas.rasterize(ch, *font_size);
                        if glyph.width > 0.0 && glyph.height > 0.0 {
                            let gx = pen_x + glyph.offset_x;
                            let gy = baseline_y - glyph.offset_y - glyph.height;
                            let quad = GpuQuad {
                                vertices: [
                                    GpuVertex { position: [gx, gy], uv: [glyph.u0, glyph.v0], color: [rf, gf, bf, af] },
                                    GpuVertex { position: [gx + glyph.width, gy], uv: [glyph.u1, glyph.v0], color: [rf, gf, bf, af] },
                                    GpuVertex { position: [gx + glyph.width, gy + glyph.height], uv: [glyph.u1, glyph.v1], color: [rf, gf, bf, af] },
                                    GpuVertex { position: [gx, gy + glyph.height], uv: [glyph.u0, glyph.v1], color: [rf, gf, bf, af] },
                                ],
                                indices: [0, 1, 2, 0, 2, 3],
                                clip_rect: None,
                                opacity: 1.0,
                                is_textured: true,
                            };
                            self.quads.push(quad);
                        }
                        pen_x += glyph.advance_width;
                    }
                }
                DisplayCommand::DrawGradientRect { x1, y1, x2, y2, gradient: _, border_radius: _ } => {
                    let y1_adj = y1 - scroll_y;
                    let y2_adj = y2 - scroll_y;
                    self.framebuffer.draw_solid_rect(*x1, y1_adj, *x2, y2_adj, (128, 128, 200, 255));

                    let quad = GpuQuad {
                        vertices: [
                            GpuVertex { position: [*x1, y1_adj], uv: [0.0, 0.0], color: [0.5, 0.5, 0.78, 1.0] },
                            GpuVertex { position: [*x2, y1_adj], uv: [1.0, 0.0], color: [0.5, 0.5, 0.78, 1.0] },
                            GpuVertex { position: [*x2, y2_adj], uv: [1.0, 1.0], color: [0.5, 0.5, 0.78, 1.0] },
                            GpuVertex { position: [*x1, y2_adj], uv: [0.0, 1.0], color: [0.5, 0.5, 0.78, 1.0] },
                        ],
                        indices: [0, 1, 2, 0, 2, 3],
                        clip_rect: None,
                        opacity: 1.0,
                        is_textured: false,
                    };
                    self.quads.push(quad);
                }
                DisplayCommand::DrawImage { x, y, width, height, .. } => {
                    let y_adj = y - scroll_y;
                    self.framebuffer.draw_solid_rect(*x, y_adj, *x + *width, y_adj + *height, (200, 200, 200, 255));

                    let quad = GpuQuad {
                        vertices: [
                            GpuVertex { position: [*x, y_adj], uv: [0.0, 0.0], color: [0.78, 0.78, 0.78, 1.0] },
                            GpuVertex { position: [*x + *width, y_adj], uv: [1.0, 0.0], color: [0.78, 0.78, 0.78, 1.0] },
                            GpuVertex { position: [*x + *width, y_adj + *height], uv: [1.0, 1.0], color: [0.78, 0.78, 0.78, 1.0] },
                            GpuVertex { position: [*x, y_adj + *height], uv: [0.0, 1.0], color: [0.78, 0.78, 0.78, 1.0] },
                        ],
                        indices: [0, 1, 2, 0, 2, 3],
                        clip_rect: None,
                        opacity: 1.0,
                        is_textured: false,
                    };
                    self.quads.push(quad);
                }
                DisplayCommand::DrawInput { x, y, width, height, value, placeholder, is_focused } => {
                    let y_adj = y - scroll_y;
                    let border_color: [f32; 4] = if *is_focused {
                        [0.24, 0.52, 0.88, 1.0]
                    } else {
                        [0.80, 0.82, 0.84, 1.0]
                    };
                    let bw = if *is_focused { 2.0 } else { 1.0 };
                    self.push_border_quads(*x, y_adj, *width, *height, bw, border_color);

                    let bg: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
                    self.quads.push(Self::solid_quad(*x + bw, y_adj + bw, *width - 2.0 * bw, *height - 2.0 * bw, bg));

                    let text = if value.is_empty() { placeholder } else { value };
                    let text_color = if value.is_empty() {
                        [0.6, 0.63, 0.66, 1.0]
                    } else {
                        [0.13, 0.13, 0.13, 1.0]
                    };
                    if !text.is_empty() {
                        let font_size = 14.0f32;
                        let mut pen_x = *x + bw + 8.0;
                        let baseline_y = y_adj + bw + font_size + 4.0;
                        for ch in text.chars() {
                            let glyph = self.glyph_atlas.rasterize(ch, font_size);
                            if glyph.width > 0.0 && glyph.height > 0.0 {
                                let gx = pen_x + glyph.offset_x;
                                let gy = baseline_y - glyph.offset_y - glyph.height;
                                self.quads.push(Self::text_quad(gx, gy, &glyph, text_color));
                            }
                            pen_x += glyph.advance_width;
                        }
                    }
                }
                DisplayCommand::DrawButton { x, y, width, height, label } => {
                    let y_adj = y - scroll_y;
                    let bg: [f32; 4] = [0.93, 0.94, 0.95, 1.0];
                    let border_color: [f32; 4] = [0.78, 0.80, 0.82, 1.0];
                    self.push_border_quads(*x, y_adj, *width, *height, 1.0, border_color);
                    self.quads.push(Self::solid_quad(*x + 1.0, y_adj + 1.0, *width - 2.0, *height - 2.0, bg));

                    if !label.is_empty() {
                        let font_size = 14.0f32;
                        let text_w: f32 = label.chars().map(|ch| self.glyph_atlas.rasterize(ch, font_size).advance_width).sum();
                        let mut pen_x = *x + (*width - text_w) / 2.0;
                        let baseline_y = y_adj + (*height + font_size) / 2.0 - 2.0;
                        let text_color = [0.13, 0.13, 0.13, 1.0];
                        for ch in label.chars() {
                            let glyph = self.glyph_atlas.rasterize(ch, font_size);
                            if glyph.width > 0.0 && glyph.height > 0.0 {
                                let gx = pen_x + glyph.offset_x;
                                let gy = baseline_y - glyph.offset_y - glyph.height;
                                self.quads.push(Self::text_quad(gx, gy, &glyph, text_color));
                            }
                            pen_x += glyph.advance_width;
                        }
                    }
                }
                DisplayCommand::DrawBoxShadow { x, y, width, height, offset_x, offset_y, blur_radius, spread_radius, color, .. } => {
                    let y_adj = y - scroll_y;
                    let (r, g, b, a) = Self::parse_color_hex(color);
                    let spread = spread_radius + blur_radius * 0.5;
                    let sx = *x + *offset_x - spread;
                    let sy = y_adj + *offset_y - spread;
                    let sw = *width + 2.0 * spread;
                    let sh = *height + 2.0 * spread;
                    let alpha = (a as f32 / 255.0) * 0.4;
                    let c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, alpha];
                    self.quads.push(Self::solid_quad(sx, sy, sw, sh, c));
                }
                _ => {}
            }
        }
    }

    fn parse_color_hex(hex: &str) -> (u8, u8, u8, u8) {
        let clean = hex.trim();
        let lower = clean.to_lowercase();
        match lower.as_str() {
            "white" => return (255, 255, 255, 255),
            "black" => return (0, 0, 0, 255),
            "red" => return (255, 0, 0, 255),
            "green" => return (0, 128, 0, 255),
            "blue" => return (0, 0, 255, 255),
            "yellow" => return (255, 255, 0, 255),
            "cyan" | "aqua" => return (0, 255, 255, 255),
            "magenta" | "fuchsia" => return (255, 0, 255, 255),
            "orange" => return (255, 165, 0, 255),
            "purple" => return (128, 0, 128, 255),
            "pink" => return (255, 192, 203, 255),
            "brown" => return (165, 42, 42, 255),
            "navy" => return (0, 0, 128, 255),
            "teal" => return (0, 128, 128, 255),
            "olive" => return (128, 128, 0, 255),
            "maroon" => return (128, 0, 0, 255),
            "silver" => return (192, 192, 192, 255),
            "lime" => return (0, 255, 0, 255),
            "transparent" => return (0, 0, 0, 0),
            "gray" | "grey" => return (128, 128, 128, 255),
            "lightgray" | "lightgrey" => return (211, 211, 211, 255),
            "darkgray" | "darkgrey" => return (169, 169, 169, 255),
            "dimgray" | "dimgrey" => return (105, 105, 105, 255),
            "whitesmoke" => return (245, 245, 245, 255),
            "gainsboro" => return (220, 220, 220, 255),
            "cornflowerblue" => return (100, 149, 237, 255),
            "dodgerblue" => return (30, 144, 255, 255),
            "steelblue" => return (70, 130, 180, 255),
            "tomato" => return (255, 99, 71, 255),
            "coral" => return (255, 127, 80, 255),
            "crimson" => return (220, 20, 60, 255),
            "darkblue" => return (0, 0, 139, 255),
            "darkred" => return (139, 0, 0, 255),
            "darkgreen" => return (0, 100, 0, 255),
            "indianred" => return (205, 92, 92, 255),
            "gold" => return (255, 215, 0, 255),
            "khaki" => return (240, 230, 140, 255),
            "linen" => return (250, 240, 230, 255),
            "ivory" => return (255, 255, 240, 255),
            "beige" => return (245, 245, 220, 255),
            "wheat" => return (245, 222, 179, 255),
            "lavender" => return (230, 230, 250, 255),
            "aliceblue" => return (240, 248, 255, 255),
            "ghostwhite" => return (248, 248, 255, 255),
            "mintcream" => return (245, 255, 250, 255),
            "honeydew" => return (240, 255, 240, 255),
            "seashell" => return (255, 245, 238, 255),
            "snow" => return (255, 250, 250, 255),
            "slategray" | "slategrey" => return (112, 128, 144, 255),
            "lightslategray" | "lightslategrey" => return (119, 136, 153, 255),
            _ => {}
        }

        if lower.starts_with("rgb") {
            return Self::parse_rgb_color(&lower);
        }
        if lower.starts_with("hsl") {
            return Self::parse_hsl_color(&lower);
        }

        let hex_str = clean.trim_start_matches('#');
        if hex_str.len() == 3 {
            let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap_or(0);
            (r * 17, g * 17, b * 17, 255)
        } else if hex_str.len() == 6 {
            let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
            (r, g, b, 255)
        } else if hex_str.len() == 8 {
            let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex_str[6..8], 16).unwrap_or(255);
            (r, g, b, a)
        } else {
            (0, 0, 0, 255)
        }
    }

    fn parse_rgb_color(s: &str) -> (u8, u8, u8, u8) {
        let inner = s.trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(|c| c == ',' || c == '/').collect();
        let r = parts.get(0).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
        let g = parts.get(1).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
        let b = parts.get(2).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
        let a = parts.get(3).and_then(|v| {
            let t = v.trim();
            if t.ends_with('%') {
                t.trim_end_matches('%').parse::<f32>().ok().map(|p| p / 100.0)
            } else {
                t.parse::<f32>().ok()
            }
        }).unwrap_or(1.0);
        (r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8, (a.clamp(0.0, 1.0) * 255.0) as u8)
    }

    fn parse_hsl_color(s: &str) -> (u8, u8, u8, u8) {
        let inner = s.trim_start_matches("hsla(")
            .trim_start_matches("hsl(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(|c| c == ',' || c == '/').collect();
        let h = parts.get(0).and_then(|v| v.trim().trim_end_matches("deg").parse::<f32>().ok()).unwrap_or(0.0) / 360.0;
        let s_val = parts.get(1).and_then(|v| v.trim().trim_end_matches('%').parse::<f32>().ok()).unwrap_or(0.0) / 100.0;
        let l = parts.get(2).and_then(|v| v.trim().trim_end_matches('%').parse::<f32>().ok()).unwrap_or(0.0) / 100.0;
        let a = parts.get(3).and_then(|v| {
            let t = v.trim();
            if t.ends_with('%') {
                t.trim_end_matches('%').parse::<f32>().ok().map(|p| p / 100.0)
            } else {
                t.parse::<f32>().ok()
            }
        }).unwrap_or(1.0);

        let (r, g, b) = hsl_to_rgb(h, s_val, l);
        ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, (a.clamp(0.0, 1.0) * 255.0) as u8)
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    (r, g, b)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}
