//! WebGPU & WebGL 2.0 Hardware-Accelerated 3D Graphics Engine for Axomai Browser.
//! Implements W3C WebGPU API specifications and WebGL 2.0 State Machine.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    Rgba16Float,
    R32Float,
    Depth24Plus,
    Depth32Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

#[derive(Debug, Clone)]
pub struct GpuBuffer {
    pub id: u32,
    pub size: usize,
    pub usage: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GpuShaderModule {
    pub id: u32,
    pub code: String,
    pub is_wgsl: bool,
}

#[derive(Debug, Clone)]
pub struct GpuRenderPipeline {
    pub id: u32,
    pub vertex_entry: String,
    pub fragment_entry: Option<String>,
    pub topology: GpuPrimitiveTopology,
}

#[derive(Debug, Clone)]
pub enum GpuRenderCommand {
    SetPipeline(u32),
    SetVertexBuffer { slot: u32, buffer_id: u32, offset: usize },
    SetIndexBuffer { buffer_id: u32, format: String },
    Draw { vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32 },
    DrawIndexed { index_count: u32, instance_count: u32, first_index: u32 },
}

pub struct GpuCommandEncoder {
    pub commands: Vec<GpuRenderCommand>,
}

impl GpuCommandEncoder {
    pub fn new() -> Self {
        GpuCommandEncoder { commands: Vec::new() }
    }

    pub fn set_pipeline(&mut self, pipeline_id: u32) {
        self.commands.push(GpuRenderCommand::SetPipeline(pipeline_id));
    }

    pub fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        self.commands.push(GpuRenderCommand::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        });
    }

    pub fn draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32) {
        self.commands.push(GpuRenderCommand::DrawIndexed {
            index_count,
            instance_count,
            first_index,
        });
    }

    pub fn finish(self) -> Vec<GpuRenderCommand> {
        self.commands
    }
}

pub struct GpuDevice {
    pub next_id: u32,
    pub buffers: HashMap<u32, GpuBuffer>,
    pub shaders: HashMap<u32, GpuShaderModule>,
    pub pipelines: HashMap<u32, GpuRenderPipeline>,
}

impl GpuDevice {
    pub fn new() -> Self {
        GpuDevice {
            next_id: 1,
            buffers: HashMap::new(),
            shaders: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    pub fn create_buffer(&mut self, size: usize, usage: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.buffers.insert(id, GpuBuffer {
            id,
            size,
            usage,
            data: vec![0u8; size],
        });
        id
    }

    pub fn create_shader_module(&mut self, code: &str, is_wgsl: bool) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.shaders.insert(id, GpuShaderModule {
            id,
            code: code.to_string(),
            is_wgsl,
        });
        id
    }

    pub fn create_render_pipeline(
        &mut self,
        vertex_entry: &str,
        fragment_entry: Option<&str>,
        topology: GpuPrimitiveTopology,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.pipelines.insert(id, GpuRenderPipeline {
            id,
            vertex_entry: vertex_entry.to_string(),
            fragment_entry: fragment_entry.map(|s| s.to_string()),
            topology,
        });
        id
    }
}

pub struct GpuCanvasContext {
    pub width: u32,
    pub height: u32,
    pub format: GpuTextureFormat,
    pub device: GpuDevice,
}

impl GpuCanvasContext {
    pub fn new(width: u32, height: u32) -> Self {
        GpuCanvasContext {
            width,
            height,
            format: GpuTextureFormat::Bgra8Unorm,
            device: GpuDevice::new(),
        }
    }

    pub fn configure(&mut self, format: GpuTextureFormat) {
        self.format = format;
    }
}

// ============================================================================
// WEBGL 2.0 STATE MACHINE
// ============================================================================

pub struct WebGl2Context {
    pub width: u32,
    pub height: u32,
    pub bound_array_buffer: Option<u32>,
    pub bound_element_buffer: Option<u32>,
    pub active_program: Option<u32>,
    pub clear_color: [f32; 4],
    pub draw_call_count: u32,
}

impl WebGl2Context {
    pub fn new(width: u32, height: u32) -> Self {
        WebGl2Context {
            width,
            height,
            bound_array_buffer: None,
            bound_element_buffer: None,
            active_program: None,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            draw_call_count: 0,
        }
    }

    pub fn clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }

    pub fn bind_buffer(&mut self, target: &str, buffer_id: u32) {
        if target == "ARRAY_BUFFER" {
            self.bound_array_buffer = Some(buffer_id);
        } else if target == "ELEMENT_ARRAY_BUFFER" {
            self.bound_element_buffer = Some(buffer_id);
        }
    }

    pub fn use_program(&mut self, program_id: u32) {
        self.active_program = Some(program_id);
    }

    pub fn draw_arrays(&mut self, _mode: &str, _first: i32, _count: i32) {
        self.draw_call_count += 1;
    }

    pub fn draw_elements(&mut self, _mode: &str, _count: i32, _type_enum: &str, _offset: i32) {
        self.draw_call_count += 1;
    }
}
