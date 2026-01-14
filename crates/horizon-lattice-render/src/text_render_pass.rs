//! GPU text rendering pass.
//!
//! This module provides a helper for rendering text glyphs that works
//! with the existing GpuRenderer by sharing its frame setup.

use bytemuck::{Pod, Zeroable};

use crate::context::GraphicsContext;
use crate::error::RenderResult;
use crate::text::{GlyphAtlas, TextDecorationStyle, TextLayout};
use crate::text_renderer::PreparedGlyph;
use crate::types::{Color, Point, Size};

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

/// Vertex for solid color rendering (backgrounds, decorations).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SolidVertex {
    /// Position in pixels.
    position: [f32; 2],
    /// Color (premultiplied alpha).
    color: [f32; 4],
}

impl SolidVertex {
    fn new(position: [f32; 2], color: Color) -> Self {
        Self {
            position,
            color: color.to_array(),
        }
    }

    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x4, // color
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
    /// Render pipeline for text glyphs.
    pipeline: wgpu::RenderPipeline,
    /// Render pipeline for solid colors (backgrounds, decorations).
    solid_pipeline: wgpu::RenderPipeline,
    /// Vertex buffer for glyphs.
    vertex_buffer: wgpu::Buffer,
    /// Vertex buffer for solid shapes.
    solid_vertex_buffer: wgpu::Buffer,
    /// Index buffer.
    index_buffer: wgpu::Buffer,
    /// Solid index buffer.
    solid_index_buffer: wgpu::Buffer,
    /// Uniform buffer.
    uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms.
    uniform_bind_group: wgpu::BindGroup,
    /// Texture bind group layout (for atlas).
    texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Current glyph vertex data.
    vertices: Vec<TextVertex>,
    /// Current glyph index data.
    indices: Vec<u32>,
    /// Current background vertex data.
    background_vertices: Vec<SolidVertex>,
    /// Current background index data.
    background_indices: Vec<u32>,
    /// Current decoration vertex data.
    decoration_vertices: Vec<SolidVertex>,
    /// Current decoration index data.
    decoration_indices: Vec<u32>,
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

        // Create solid color shader module
        let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("solid_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/solid.wgsl").into()),
        });

        // Create solid pipeline layout (uses same uniform bind group, no texture)
        let solid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solid_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create solid pipeline
        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solid_pipeline"),
            layout: Some(&solid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some("vs_main"),
                buffers: &[SolidVertex::buffer_layout()],
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
                module: &solid_shader,
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

        // Create solid vertex buffer
        let solid_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid_vertex_buffer"),
            size: (MAX_TEXT_VERTICES * std::mem::size_of::<SolidVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create solid index buffer
        let solid_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid_index_buffer"),
            size: (MAX_TEXT_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            solid_pipeline,
            vertex_buffer,
            solid_vertex_buffer,
            index_buffer,
            solid_index_buffer,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            vertices: Vec::new(),
            indices: Vec::new(),
            background_vertices: Vec::new(),
            background_indices: Vec::new(),
            decoration_vertices: Vec::new(),
            decoration_indices: Vec::new(),
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

    /// Clear all pending data.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.background_vertices.clear();
        self.background_indices.clear();
        self.decoration_vertices.clear();
        self.decoration_indices.clear();
    }

    /// Add backgrounds and decorations from a text layout.
    ///
    /// This extracts background rectangles and decoration lines from the layout
    /// and adds them to the appropriate render batches.
    pub fn add_layout_styling(&mut self, layout: &TextLayout, position: Point) {
        // Add background rectangles
        for bg in layout.background_rects() {
            self.add_background_rect(
                bg.x + position.x,
                bg.y + position.y,
                bg.width,
                bg.height,
                Color::from_rgba8(bg.color[0], bg.color[1], bg.color[2], bg.color[3]),
            );
        }

        // Add decoration lines
        for decoration in layout.decoration_lines() {
            self.add_decoration_line(
                decoration.x_start + position.x,
                decoration.x_end + position.x,
                decoration.y + position.y,
                decoration.thickness,
                Color::from_rgba8(
                    decoration.color[0],
                    decoration.color[1],
                    decoration.color[2],
                    decoration.color[3],
                ),
                decoration.style,
            );
        }
    }

    /// Add a background rectangle.
    pub fn add_background_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        let base_index = self.background_vertices.len() as u32;

        // Add four vertices for the quad
        self.background_vertices.push(SolidVertex::new([x, y], color));
        self.background_vertices.push(SolidVertex::new([x + width, y], color));
        self.background_vertices.push(SolidVertex::new([x + width, y + height], color));
        self.background_vertices.push(SolidVertex::new([x, y + height], color));

        // Add indices for two triangles
        self.background_indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    /// Add a decoration line with the specified style.
    pub fn add_decoration_line(
        &mut self,
        x_start: f32,
        x_end: f32,
        y: f32,
        thickness: f32,
        color: Color,
        style: TextDecorationStyle,
    ) {
        match style {
            TextDecorationStyle::Solid => {
                self.add_solid_line(x_start, x_end, y, thickness, color);
            }
            TextDecorationStyle::Dotted => {
                self.add_dotted_line(x_start, x_end, y, thickness, color);
            }
            TextDecorationStyle::Dashed => {
                self.add_dashed_line(x_start, x_end, y, thickness, color);
            }
            TextDecorationStyle::Wavy => {
                self.add_wavy_line(x_start, x_end, y, thickness, color);
            }
        }
    }

    /// Add a solid line as a rectangle.
    fn add_solid_line(&mut self, x_start: f32, x_end: f32, y: f32, thickness: f32, color: Color) {
        let half_thick = thickness / 2.0;
        let base_index = self.decoration_vertices.len() as u32;

        self.decoration_vertices.push(SolidVertex::new([x_start, y - half_thick], color));
        self.decoration_vertices.push(SolidVertex::new([x_end, y - half_thick], color));
        self.decoration_vertices.push(SolidVertex::new([x_end, y + half_thick], color));
        self.decoration_vertices.push(SolidVertex::new([x_start, y + half_thick], color));

        self.decoration_indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    /// Add a dotted line (circular dots).
    fn add_dotted_line(&mut self, x_start: f32, x_end: f32, y: f32, thickness: f32, color: Color) {
        // Dot spacing proportional to thickness (2x diameter)
        let dot_size = thickness;
        let spacing = dot_size * 2.0;
        let width = x_end - x_start;
        let num_dots = ((width / spacing) as usize).max(1);

        for i in 0..num_dots {
            let cx = x_start + (i as f32 * spacing) + (dot_size / 2.0);
            if cx > x_end {
                break;
            }
            // Approximate circle with a small square (for simplicity)
            // A proper implementation would use more vertices for circular dots
            let half = dot_size / 2.0;
            let base_index = self.decoration_vertices.len() as u32;

            self.decoration_vertices.push(SolidVertex::new([cx - half, y - half], color));
            self.decoration_vertices.push(SolidVertex::new([cx + half, y - half], color));
            self.decoration_vertices.push(SolidVertex::new([cx + half, y + half], color));
            self.decoration_vertices.push(SolidVertex::new([cx - half, y + half], color));

            self.decoration_indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);
        }
    }

    /// Add a dashed line.
    fn add_dashed_line(&mut self, x_start: f32, x_end: f32, y: f32, thickness: f32, color: Color) {
        // Dash length and gap proportional to thickness
        let dash_length = thickness * 4.0;
        let gap_length = thickness * 2.0;
        let cycle = dash_length + gap_length;
        let half_thick = thickness / 2.0;

        let mut x = x_start;
        while x < x_end {
            let dash_end = (x + dash_length).min(x_end);
            let base_index = self.decoration_vertices.len() as u32;

            self.decoration_vertices.push(SolidVertex::new([x, y - half_thick], color));
            self.decoration_vertices.push(SolidVertex::new([dash_end, y - half_thick], color));
            self.decoration_vertices.push(SolidVertex::new([dash_end, y + half_thick], color));
            self.decoration_vertices.push(SolidVertex::new([x, y + half_thick], color));

            self.decoration_indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);

            x += cycle;
        }
    }

    /// Add a wavy line (sinusoidal).
    fn add_wavy_line(&mut self, x_start: f32, x_end: f32, y: f32, thickness: f32, color: Color) {
        // Wave parameters proportional to thickness
        let amplitude = thickness * 1.5;
        let wavelength = thickness * 6.0;
        let half_thick = thickness / 2.0;

        // Generate triangle strip for wavy line
        let step = 2.0; // Step in pixels
        let num_segments = ((x_end - x_start) / step).ceil() as usize;

        if num_segments < 2 {
            // Fall back to solid line for very short lines
            self.add_solid_line(x_start, x_end, y, thickness, color);
            return;
        }

        // Generate vertices along the wave
        for i in 0..num_segments {
            let x = x_start + (i as f32 * step).min(x_end - x_start);
            let t = x / wavelength;
            let wave_y = y + amplitude * (t * std::f32::consts::TAU).sin();

            let base_index = self.decoration_vertices.len() as u32;

            // Top and bottom of the line at this x position
            self.decoration_vertices.push(SolidVertex::new([x, wave_y - half_thick], color));
            self.decoration_vertices.push(SolidVertex::new([x, wave_y + half_thick], color));

            // Add triangles connecting to previous segment
            if i > 0 {
                let prev_base = base_index - 2;
                // Two triangles forming a quad
                self.decoration_indices.extend_from_slice(&[
                    prev_base,
                    prev_base + 1,
                    base_index,
                    prev_base + 1,
                    base_index + 1,
                    base_index,
                ]);
            }
        }
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
        let has_glyphs = !self.vertices.is_empty();
        let has_backgrounds = !self.background_vertices.is_empty();
        let has_decorations = !self.decoration_vertices.is_empty();

        if !has_glyphs && !has_backgrounds && !has_decorations {
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

        // Upload glyph vertex data
        if has_glyphs {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.vertices),
            );
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }

        // Upload solid vertex data (backgrounds + decorations)
        let solid_vertices: Vec<SolidVertex> = self.background_vertices.iter()
            .chain(self.decoration_vertices.iter())
            .cloned()
            .collect();

        let bg_index_offset = self.background_vertices.len() as u32;
        let solid_indices: Vec<u32> = self.background_indices.iter()
            .cloned()
            .chain(self.decoration_indices.iter().map(|i| i + bg_index_offset))
            .collect();

        if has_backgrounds || has_decorations {
            queue.write_buffer(
                &self.solid_vertex_buffer,
                0,
                bytemuck::cast_slice(&solid_vertices),
            );
            queue.write_buffer(&self.solid_index_buffer, 0, bytemuck::cast_slice(&solid_indices));
        }

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

            // 1. Render backgrounds first (behind text)
            if has_backgrounds {
                render_pass.set_pipeline(&self.solid_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.solid_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.solid_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.background_indices.len() as u32, 0, 0..1);
            }

            // 2. Render glyphs
            if has_glyphs {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            }

            // 3. Render decorations (on top of text)
            if has_decorations {
                render_pass.set_pipeline(&self.solid_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.solid_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.solid_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                let start = self.background_indices.len() as u32;
                let end = solid_indices.len() as u32;
                render_pass.draw_indexed(start..end, 0, 0..1);
            }
        }

        // Submit
        queue.submit(std::iter::once(encoder.finish()));

        // Clear for next frame
        self.clear();

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
