//! GPU text rendering pass.
//!
//! This module provides a helper for rendering text glyphs that works
//! with the existing GpuRenderer by sharing its frame setup.

use bytemuck::{Pod, Zeroable};

use crate::context::GraphicsContext;
use crate::error::RenderResult;
use crate::text::GlyphAtlas;
use crate::text_renderer::PreparedGlyph;
use crate::types::{Color, Size};

/// Maximum vertices per text batch.
const MAX_TEXT_VERTICES: usize = 65536;
/// Maximum indices per text batch.
const MAX_TEXT_INDICES: usize = MAX_TEXT_VERTICES * 6 / 4;

/// Vertex for text rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TextVertex {
    /// Position in pixels.
    position: [f32; 2],
    /// Texture coordinates.
    uv: [f32; 2],
    /// Color (premultiplied alpha).
    color: [f32; 4],
}

impl TextVertex {
    fn new(position: [f32; 2], uv: [f32; 2], color: Color) -> Self {
        Self {
            position,
            uv,
            color: color.to_array(),
        }
    }

    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // uv
        2 => Float32x4, // color
    ];

    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniforms for text rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TextUniforms {
    /// Transform matrix.
    transform: [[f32; 4]; 4],
    /// Viewport size.
    viewport_size: [f32; 2],
    /// Padding.
    _padding: [f32; 2],
}

/// A pass for rendering text glyphs.
///
/// This is designed to be used alongside GpuRenderer, handling text
/// rendering separately from shape rendering.
pub struct TextRenderPass {
    /// Render pipeline for text.
    pipeline: wgpu::RenderPipeline,
    /// Vertex buffer.
    vertex_buffer: wgpu::Buffer,
    /// Index buffer.
    index_buffer: wgpu::Buffer,
    /// Uniform buffer.
    uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms.
    uniform_bind_group: wgpu::BindGroup,
    /// Texture bind group layout (for atlas).
    texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Current vertex data.
    vertices: Vec<TextVertex>,
    /// Current index data.
    indices: Vec<u32>,
}

impl TextRenderPass {
    /// Create a new text render pass.
    pub fn new(format: wgpu::TextureFormat) -> RenderResult<Self> {
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_uniform_buffer"),
            size: std::mem::size_of::<TextUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform bind group layout
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("text_uniform_bind_group_layout"),
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

        // Create uniform bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create texture bind group layout (matches GlyphAtlas bind group)
        let texture_bind_group_layout = GlyphAtlas::bind_group_layout(device);

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TextVertex::buffer_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
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
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        // Create vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_vertex_buffer"),
            size: (MAX_TEXT_VERTICES * std::mem::size_of::<TextVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_index_buffer"),
            size: (MAX_TEXT_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            vertices: Vec::new(),
            indices: Vec::new(),
        })
    }

    /// Add prepared glyphs to the batch.
    ///
    /// The glyphs will be rendered when `render()` is called.
    pub fn add_glyphs(&mut self, glyphs: &[PreparedGlyph], atlas_size: u32) {
        for glyph in glyphs {
            let dest = glyph.dest_rect();
            let (u_min, v_min, u_max, v_max) = glyph.uv_rect(atlas_size);

            let base_index = self.vertices.len() as u32;

            // Add four vertices for the quad
            self.vertices.push(TextVertex::new(
                [dest.left(), dest.top()],
                [u_min, v_min],
                glyph.color,
            ));
            self.vertices.push(TextVertex::new(
                [dest.right(), dest.top()],
                [u_max, v_min],
                glyph.color,
            ));
            self.vertices.push(TextVertex::new(
                [dest.right(), dest.bottom()],
                [u_max, v_max],
                glyph.color,
            ));
            self.vertices.push(TextVertex::new(
                [dest.left(), dest.bottom()],
                [u_min, v_max],
                glyph.color,
            ));

            // Add indices for two triangles
            self.indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);
        }
    }

    /// Clear all pending glyphs.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Render text to a texture view.
    ///
    /// # Arguments
    ///
    /// * `target` - The texture view to render to
    /// * `atlas` - The glyph atlas containing the rasterized glyphs
    /// * `viewport_size` - The size of the render target
    pub fn render(
        &mut self,
        target: &wgpu::TextureView,
        atlas: &GlyphAtlas,
        viewport_size: Size,
    ) -> RenderResult<()> {
        if self.vertices.is_empty() {
            return Ok(());
        }

        let ctx = GraphicsContext::get();
        let device = ctx.device();
        let queue = ctx.queue();

        // Update uniforms
        let uniforms = TextUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            viewport_size: [viewport_size.width, viewport_size.height],
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Upload vertex data
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.vertices),
        );
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("text_render_encoder"),
        });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load existing content
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, atlas.bind_group(), &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }

        // Submit
        queue.submit(std::iter::once(encoder.finish()));

        // Clear for next frame
        self.vertices.clear();
        self.indices.clear();

        Ok(())
    }

    /// Get the number of pending glyphs.
    pub fn glyph_count(&self) -> usize {
        self.vertices.len() / 4
    }
}

impl std::fmt::Debug for TextRenderPass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextRenderPass")
            .field("pending_glyphs", &self.glyph_count())
            .finish()
    }
}
