//! GPU-accelerated renderer implementation using wgpu.
//!
//! This module provides the [`GpuRenderer`] which implements the [`Renderer`] trait
//! using wgpu for hardware-accelerated 2D rendering.

use bytemuck::{Pod, Zeroable};
use tracing::debug;

use crate::context::GraphicsContext;
use crate::error::RenderResult;
use crate::offscreen::OffscreenSurface;
use crate::paint::{BlendMode, Paint, Stroke};
use crate::renderer::{FrameStats, RenderStateStack, Renderer};
use crate::surface::RenderSurface;
use crate::transform::Transform2D;
use crate::types::{Color, CornerRadii, Point, Rect, RoundedRect, Size};

/// Paint type constants for shader.
const PAINT_TYPE_SOLID: u32 = 0;
const PAINT_TYPE_LINEAR_GRADIENT: u32 = 1;
const PAINT_TYPE_RADIAL_GRADIENT: u32 = 2;

/// Vertex data for rectangles with gradient support.
///
/// Supports solid colors, linear gradients, and radial gradients with 2 color stops.
/// For gradients, coordinates are in normalized local space (0-1 within the rect).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RectVertex {
    /// Position in pixels.
    position: [f32; 2],
    /// RGBA color for stop 0 (premultiplied alpha).
    color0: [f32; 4],
    /// Rectangle top-left position.
    rect_pos: [f32; 2],
    /// Rectangle size.
    rect_size: [f32; 2],
    /// Corner radii (TL, TR, BR, BL).
    corner_radii: [f32; 4],
    /// Gradient info: [paint_type, gradient_start_x, gradient_start_y, gradient_end_x]
    /// paint_type: 0=solid, 1=linear, 2=radial
    /// For linear: start/end are normalized local coords (0-1)
    /// For radial: start is center, end.x is radius (in normalized coords)
    gradient_info: [f32; 4],
    /// Gradient end and stops: [gradient_end_y, stop0_offset, stop1_offset, _unused]
    gradient_end_stops: [f32; 4],
    /// RGBA color for stop 1 (premultiplied alpha).
    color1: [f32; 4],
}

impl RectVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x4, // color0
        2 => Float32x2, // rect_pos
        3 => Float32x2, // rect_size
        4 => Float32x4, // corner_radii
        5 => Float32x4, // gradient_info
        6 => Float32x4, // gradient_end_stops
        7 => Float32x4, // color1
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    /// Create a vertex for solid color rendering.
    fn solid(position: [f32; 2], color: Color, rect_pos: [f32; 2], rect_size: [f32; 2], corner_radii: [f32; 4]) -> Self {
        Self {
            position,
            color0: color.to_array(),
            rect_pos,
            rect_size,
            corner_radii,
            gradient_info: [PAINT_TYPE_SOLID as f32, 0.0, 0.0, 0.0],
            gradient_end_stops: [0.0, 0.0, 1.0, 0.0],
            color1: [0.0; 4],
        }
    }

    /// Create a vertex for linear gradient rendering.
    fn linear_gradient(
        position: [f32; 2],
        rect_pos: [f32; 2],
        rect_size: [f32; 2],
        corner_radii: [f32; 4],
        start: [f32; 2],
        end: [f32; 2],
        stop0_offset: f32,
        stop0_color: Color,
        stop1_offset: f32,
        stop1_color: Color,
    ) -> Self {
        Self {
            position,
            color0: stop0_color.to_array(),
            rect_pos,
            rect_size,
            corner_radii,
            gradient_info: [PAINT_TYPE_LINEAR_GRADIENT as f32, start[0], start[1], end[0]],
            gradient_end_stops: [end[1], stop0_offset, stop1_offset, 0.0],
            color1: stop1_color.to_array(),
        }
    }

    /// Create a vertex for radial gradient rendering.
    fn radial_gradient(
        position: [f32; 2],
        rect_pos: [f32; 2],
        rect_size: [f32; 2],
        corner_radii: [f32; 4],
        center: [f32; 2],
        radius: f32,
        stop0_offset: f32,
        stop0_color: Color,
        stop1_offset: f32,
        stop1_color: Color,
    ) -> Self {
        Self {
            position,
            color0: stop0_color.to_array(),
            rect_pos,
            rect_size,
            corner_radii,
            gradient_info: [PAINT_TYPE_RADIAL_GRADIENT as f32, center[0], center[1], radius],
            gradient_end_stops: [0.0, stop0_offset, stop1_offset, 0.0],
            color1: stop1_color.to_array(),
        }
    }
}

/// Uniform buffer data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Uniforms {
    /// Transform matrix (4x4 for GPU compatibility).
    transform: [[f32; 4]; 4],
    /// Viewport size.
    viewport_size: [f32; 2],
    /// Padding for alignment.
    _padding: [f32; 2],
}

/// Maximum number of vertices per batch.
const MAX_VERTICES: usize = 65536;

/// Maximum number of indices per batch.
const MAX_INDICES: usize = MAX_VERTICES * 6 / 4; // 6 indices per 4 vertices (quad)

/// GPU-accelerated 2D renderer.
///
/// This renderer batches drawing operations and submits them to the GPU
/// efficiently. It supports basic 2D primitives with transforms and clipping.
pub struct GpuRenderer {
    /// Render state stack.
    state: RenderStateStack,
    /// Current viewport size.
    viewport_size: Size,
    /// Clear color for the current frame.
    clear_color: Color,
    /// Whether we're currently in a frame.
    in_frame: bool,

    // GPU resources
    /// Render pipeline for rectangles.
    rect_pipeline: wgpu::RenderPipeline,
    /// Vertex buffer.
    vertex_buffer: wgpu::Buffer,
    /// Index buffer.
    index_buffer: wgpu::Buffer,
    /// Uniform buffer.
    uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms.
    bind_group: wgpu::BindGroup,

    // Batching state
    /// Vertices waiting to be rendered.
    vertices: Vec<RectVertex>,
    /// Indices waiting to be rendered.
    indices: Vec<u32>,

    // Frame statistics
    /// Draw calls this frame.
    draw_calls: u32,
    /// Vertices this frame.
    vertex_count: u32,
    /// State changes this frame.
    state_changes: u32,

    // Current state
    /// Current scissor rect (in pixels).
    scissor_rect: Option<Rect>,
    /// Current blend mode.
    current_blend_mode: BlendMode,
    /// Current opacity.
    current_opacity: f32,

    /// The surface format this renderer was created for.
    surface_format: wgpu::TextureFormat,
}

impl GpuRenderer {
    /// Create a new GPU renderer for the given surface.
    pub fn new(surface: &RenderSurface) -> RenderResult<Self> {
        Self::new_with_format(surface.format())
    }

    /// Create a new GPU renderer for offscreen rendering.
    pub fn new_offscreen(surface: &OffscreenSurface) -> RenderResult<Self> {
        Self::new_with_format(surface.format())
    }

    /// Create a new GPU renderer with the specified texture format.
    fn new_with_format(format: wgpu::TextureFormat) -> RenderResult<Self> {
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
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

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            cache: None,
        });

        // Create vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<RectVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        debug!(
            target: "horizon_lattice_render::gpu_renderer",
            format = ?format,
            max_vertices = MAX_VERTICES,
            max_indices = MAX_INDICES,
            "created GPU renderer"
        );

        Ok(Self {
            state: RenderStateStack::new(),
            viewport_size: Size::ZERO,
            clear_color: Color::BLACK,
            in_frame: false,

            rect_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            bind_group,

            vertices: Vec::with_capacity(MAX_VERTICES),
            indices: Vec::with_capacity(MAX_INDICES),

            draw_calls: 0,
            vertex_count: 0,
            state_changes: 0,

            scissor_rect: None,
            current_blend_mode: BlendMode::Normal,
            current_opacity: 1.0,

            surface_format: format,
        })
    }

    /// Render to a surface frame.
    ///
    /// This should be called after `end_frame()` to actually submit the
    /// rendered content to the surface.
    pub fn render_to_surface(&mut self, surface: &mut RenderSurface) -> RenderResult<FrameStats> {
        let frame = surface.get_current_frame()?;
        let Some(frame) = frame else {
            return Ok(FrameStats::default());
        };

        let ctx = GraphicsContext::get();
        let device = ctx.device();
        let queue = ctx.queue();

        // Update uniform buffer
        let uniforms = Uniforms {
            transform: self.state.transform().to_mat4().to_cols_array_2d(),
            viewport_size: [self.viewport_size.width, self.viewport_size.height],
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Upload vertex and index data
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if !self.indices.is_empty() {
                render_pass.set_pipeline(&self.rect_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                // Apply scissor if set
                if let Some(scissor) = &self.scissor_rect {
                    render_pass.set_scissor_rect(
                        scissor.left().max(0.0) as u32,
                        scissor.top().max(0.0) as u32,
                        scissor.width().max(0.0) as u32,
                        scissor.height().max(0.0) as u32,
                    );
                }

                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }
        }

        // Submit
        queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        let stats = FrameStats {
            draw_calls: self.draw_calls,
            vertices: self.vertex_count,
            state_changes: self.state_changes,
        };

        // Reset for next frame
        self.vertices.clear();
        self.indices.clear();
        self.draw_calls = 0;
        self.vertex_count = 0;
        self.state_changes = 0;
        self.in_frame = false;

        Ok(stats)
    }

    /// Render to an offscreen surface.
    ///
    /// This should be called after `end_frame()` to actually submit the
    /// rendered content to the offscreen texture.
    pub fn render_to_offscreen(&mut self, surface: &OffscreenSurface) -> RenderResult<FrameStats> {
        let ctx = GraphicsContext::get();
        let device = ctx.device();
        let queue = ctx.queue();

        // Update uniform buffer
        let uniforms = Uniforms {
            transform: self.state.transform().to_mat4().to_cols_array_2d(),
            viewport_size: [self.viewport_size.width, self.viewport_size.height],
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Upload vertex and index data
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("offscreen_render_encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("offscreen_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if !self.indices.is_empty() {
                render_pass.set_pipeline(&self.rect_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                // Apply scissor if set
                if let Some(scissor) = &self.scissor_rect {
                    render_pass.set_scissor_rect(
                        scissor.left().max(0.0) as u32,
                        scissor.top().max(0.0) as u32,
                        scissor.width().max(0.0) as u32,
                        scissor.height().max(0.0) as u32,
                    );
                }

                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }
        }

        // Submit (no present for offscreen)
        queue.submit(std::iter::once(encoder.finish()));

        let stats = FrameStats {
            draw_calls: self.draw_calls,
            vertices: self.vertex_count,
            state_changes: self.state_changes,
        };

        // Reset for next frame
        self.vertices.clear();
        self.indices.clear();
        self.draw_calls = 0;
        self.vertex_count = 0;
        self.state_changes = 0;
        self.in_frame = false;

        Ok(stats)
    }

    /// Add a filled quad to the batch with solid color.
    fn add_filled_quad(&mut self, rect: Rect, radii: CornerRadii, color: Color) {
        let base_index = self.vertices.len() as u32;

        // Apply opacity
        let color = if self.current_opacity < 1.0 {
            color.with_alpha(color.a * self.current_opacity)
        } else {
            color
        };

        let rect_pos = [rect.left(), rect.top()];
        let rect_size = [rect.width(), rect.height()];
        let corner_radii = [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left];

        // Add four vertices for the quad
        let positions = [
            [rect.left(), rect.top()],
            [rect.right(), rect.top()],
            [rect.right(), rect.bottom()],
            [rect.left(), rect.bottom()],
        ];

        for pos in positions {
            self.vertices.push(RectVertex::solid(pos, color, rect_pos, rect_size, corner_radii));
        }

        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    /// Add a filled quad with a paint (solid, linear gradient, or radial gradient).
    fn add_filled_quad_paint(&mut self, rect: Rect, radii: CornerRadii, paint: &Paint) {
        match paint {
            Paint::Solid(color) => {
                self.add_filled_quad(rect, radii, *color);
            }
            Paint::LinearGradient(gradient) => {
                self.add_linear_gradient_quad(rect, radii, gradient);
            }
            Paint::RadialGradient(gradient) => {
                self.add_radial_gradient_quad(rect, radii, gradient);
            }
        }
    }

    /// Add a filled quad with a linear gradient.
    fn add_linear_gradient_quad(&mut self, rect: Rect, radii: CornerRadii, gradient: &crate::paint::LinearGradient) {
        let base_index = self.vertices.len() as u32;

        let rect_pos = [rect.left(), rect.top()];
        let rect_size = [rect.width(), rect.height()];
        let corner_radii = [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left];

        // Convert gradient start/end from absolute coords to normalized local coords (0-1)
        let start = [
            (gradient.start.x - rect.left()) / rect.width(),
            (gradient.start.y - rect.top()) / rect.height(),
        ];
        let end = [
            (gradient.end.x - rect.left()) / rect.width(),
            (gradient.end.y - rect.top()) / rect.height(),
        ];

        // Get the first two stops (we support 2 stops in the current implementation)
        let (stop0_offset, stop0_color, stop1_offset, stop1_color) = self.extract_two_stops(&gradient.stops);

        // Apply opacity to colors
        let stop0_color = self.apply_opacity(stop0_color);
        let stop1_color = self.apply_opacity(stop1_color);

        // Add four vertices for the quad
        let positions = [
            [rect.left(), rect.top()],
            [rect.right(), rect.top()],
            [rect.right(), rect.bottom()],
            [rect.left(), rect.bottom()],
        ];

        for pos in positions {
            self.vertices.push(RectVertex::linear_gradient(
                pos,
                rect_pos,
                rect_size,
                corner_radii,
                start,
                end,
                stop0_offset,
                stop0_color,
                stop1_offset,
                stop1_color,
            ));
        }

        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    /// Add a filled quad with a radial gradient.
    fn add_radial_gradient_quad(&mut self, rect: Rect, radii: CornerRadii, gradient: &crate::paint::RadialGradient) {
        let base_index = self.vertices.len() as u32;

        let rect_pos = [rect.left(), rect.top()];
        let rect_size = [rect.width(), rect.height()];
        let corner_radii = [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left];

        // Convert gradient center from absolute coords to normalized local coords (0-1)
        let center = [
            (gradient.center.x - rect.left()) / rect.width(),
            (gradient.center.y - rect.top()) / rect.height(),
        ];

        // Normalize radius relative to the rect size (use average of width/height)
        let avg_size = (rect.width() + rect.height()) / 2.0;
        let normalized_radius = gradient.radius / avg_size;

        // Get the first two stops
        let (stop0_offset, stop0_color, stop1_offset, stop1_color) = self.extract_two_stops(&gradient.stops);

        // Apply opacity to colors
        let stop0_color = self.apply_opacity(stop0_color);
        let stop1_color = self.apply_opacity(stop1_color);

        // Add four vertices for the quad
        let positions = [
            [rect.left(), rect.top()],
            [rect.right(), rect.top()],
            [rect.right(), rect.bottom()],
            [rect.left(), rect.bottom()],
        ];

        for pos in positions {
            self.vertices.push(RectVertex::radial_gradient(
                pos,
                rect_pos,
                rect_size,
                corner_radii,
                center,
                normalized_radius,
                stop0_offset,
                stop0_color,
                stop1_offset,
                stop1_color,
            ));
        }

        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    /// Extract the first two stops from a gradient stop list.
    /// If there's only one stop, duplicates it. If there are more than two,
    /// uses the first and last.
    fn extract_two_stops(&self, stops: &[crate::paint::GradientStop]) -> (f32, Color, f32, Color) {
        match stops.len() {
            0 => (0.0, Color::BLACK, 1.0, Color::BLACK),
            1 => (stops[0].offset, stops[0].color, stops[0].offset, stops[0].color),
            2 => (stops[0].offset, stops[0].color, stops[1].offset, stops[1].color),
            _ => {
                // For more than 2 stops, use first and last
                // TODO: Support more stops with multi-pass or texture-based approach
                let first = &stops[0];
                let last = &stops[stops.len() - 1];
                (first.offset, first.color, last.offset, last.color)
            }
        }
    }

    /// Apply current opacity to a color.
    fn apply_opacity(&self, color: Color) -> Color {
        if self.current_opacity < 1.0 {
            color.with_alpha(color.a * self.current_opacity)
        } else {
            color
        }
    }

    /// Transform a paint's gradient coordinates from original rect space to transformed rect space.
    fn transform_paint(&self, paint: Paint, original_rect: &Rect, transformed_rect: &Rect) -> Paint {
        match paint {
            Paint::Solid(_) => paint, // No transformation needed for solid colors
            Paint::LinearGradient(mut gradient) => {
                // Transform gradient start/end points
                gradient.start = self.transform_gradient_point(
                    gradient.start,
                    original_rect,
                    transformed_rect,
                );
                gradient.end = self.transform_gradient_point(
                    gradient.end,
                    original_rect,
                    transformed_rect,
                );
                Paint::LinearGradient(gradient)
            }
            Paint::RadialGradient(mut gradient) => {
                // Transform gradient center
                gradient.center = self.transform_gradient_point(
                    gradient.center,
                    original_rect,
                    transformed_rect,
                );
                // Scale radius proportionally (use average scale factor)
                let scale_x = transformed_rect.width() / original_rect.width();
                let scale_y = transformed_rect.height() / original_rect.height();
                gradient.radius *= (scale_x + scale_y) / 2.0;
                Paint::RadialGradient(gradient)
            }
        }
    }

    /// Transform a point from original rect space to transformed rect space.
    fn transform_gradient_point(&self, point: Point, original_rect: &Rect, transformed_rect: &Rect) -> Point {
        // Calculate normalized position within original rect
        let norm_x = (point.x - original_rect.left()) / original_rect.width();
        let norm_y = (point.y - original_rect.top()) / original_rect.height();
        // Apply to transformed rect
        Point::new(
            transformed_rect.left() + norm_x * transformed_rect.width(),
            transformed_rect.top() + norm_y * transformed_rect.height(),
        )
    }

    /// Add a stroked quad to the batch (as four separate quads for each edge).
    fn add_stroked_quad(&mut self, rect: Rect, radii: CornerRadii, stroke: &Stroke) {
        let half_width = stroke.width / 2.0;
        let color = stroke.paint.as_solid().unwrap_or(Color::BLACK);

        if radii.is_zero() {
            // Simple stroked rectangle - draw as 4 edge rectangles
            // Top edge
            self.add_filled_quad(
                Rect::new(
                    rect.left() - half_width,
                    rect.top() - half_width,
                    rect.width() + stroke.width,
                    stroke.width,
                ),
                CornerRadii::ZERO,
                color,
            );
            // Bottom edge
            self.add_filled_quad(
                Rect::new(
                    rect.left() - half_width,
                    rect.bottom() - half_width,
                    rect.width() + stroke.width,
                    stroke.width,
                ),
                CornerRadii::ZERO,
                color,
            );
            // Left edge
            self.add_filled_quad(
                Rect::new(
                    rect.left() - half_width,
                    rect.top() + half_width,
                    stroke.width,
                    rect.height() - stroke.width,
                ),
                CornerRadii::ZERO,
                color,
            );
            // Right edge
            self.add_filled_quad(
                Rect::new(
                    rect.right() - half_width,
                    rect.top() + half_width,
                    stroke.width,
                    rect.height() - stroke.width,
                ),
                CornerRadii::ZERO,
                color,
            );
        } else {
            // For rounded strokes, draw outer and inner (will need more sophisticated approach)
            // For now, approximate with filled rounded rect minus inner
            let outer = rect.inflate(half_width);
            let outer_radii = CornerRadii {
                top_left: radii.top_left + half_width,
                top_right: radii.top_right + half_width,
                bottom_right: radii.bottom_right + half_width,
                bottom_left: radii.bottom_left + half_width,
            };
            self.add_filled_quad(outer, outer_radii, color);

            // Inner (transparent to cut out)
            // This is a simplified approach - proper strokes need more work
        }
    }

    /// Flush any pending draw commands.
    fn flush(&mut self) {
        // For now, we batch everything until render_to_surface
    }

    /// Get the surface format this renderer was created for.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }
}

impl Renderer for GpuRenderer {
    fn begin_frame(&mut self, clear_color: Color, viewport_size: Size) {
        self.clear_color = clear_color;
        self.viewport_size = viewport_size;
        self.in_frame = true;
        self.state.reset();
        self.vertices.clear();
        self.indices.clear();
        self.scissor_rect = None;
    }

    fn end_frame(&mut self) -> FrameStats {
        self.flush();
        // Stats will be returned by render_to_surface
        FrameStats {
            draw_calls: self.draw_calls,
            vertices: self.vertex_count,
            state_changes: self.state_changes,
        }
    }

    fn save(&mut self) {
        self.state.save();
    }

    fn restore(&mut self) {
        self.state.restore();
        // Update scissor
        self.scissor_rect = self.state.clip_bounds();
    }

    fn reset(&mut self) {
        self.state.reset();
        self.scissor_rect = None;
    }

    fn transform(&self) -> &Transform2D {
        self.state.transform()
    }

    fn set_transform(&mut self, transform: Transform2D) {
        self.state.set_transform(transform);
    }

    fn concat_transform(&mut self, transform: &Transform2D) {
        self.state.concat_transform(transform);
    }

    fn translate(&mut self, tx: f32, ty: f32) {
        self.state.translate(tx, ty);
    }

    fn scale(&mut self, sx: f32, sy: f32) {
        self.state.scale(sx, sy);
    }

    fn rotate(&mut self, angle: f32) {
        self.state.rotate(angle);
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.state.clip_rect(rect);
        self.scissor_rect = self.state.clip_bounds();
    }

    fn clip_bounds(&self) -> Option<Rect> {
        self.state.clip_bounds()
    }

    fn fill_rect(&mut self, rect: Rect, paint: impl Into<Paint>) {
        let paint = paint.into();

        // Transform the rectangle
        let transformed_rect = self.state.transform().transform_rect(&rect);

        // For gradients, we need to transform the gradient coordinates too
        let paint = self.transform_paint(paint, &rect, &transformed_rect);
        self.add_filled_quad_paint(transformed_rect, CornerRadii::ZERO, &paint);
    }

    fn fill_rounded_rect(&mut self, rrect: RoundedRect, paint: impl Into<Paint>) {
        let paint = paint.into();

        // Transform the rectangle
        let transformed_rect = self.state.transform().transform_rect(&rrect.rect);

        // For gradients, we need to transform the gradient coordinates too
        let paint = self.transform_paint(paint, &rrect.rect, &transformed_rect);
        self.add_filled_quad_paint(transformed_rect, rrect.radii, &paint);
    }

    fn stroke_rect(&mut self, rect: Rect, stroke: &Stroke) {
        let transformed_rect = self.state.transform().transform_rect(&rect);
        self.add_stroked_quad(transformed_rect, CornerRadii::ZERO, stroke);
    }

    fn stroke_rounded_rect(&mut self, rrect: RoundedRect, stroke: &Stroke) {
        let transformed_rect = self.state.transform().transform_rect(&rrect.rect);
        self.add_stroked_quad(transformed_rect, rrect.radii, stroke);
    }

    fn draw_line(&mut self, from: Point, to: Point, stroke: &Stroke) {
        // Transform points
        let from = self.state.transform().transform_point(from);
        let to = self.state.transform().transform_point(to);

        // Calculate line as a thin rectangle
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let length = (dx * dx + dy * dy).sqrt();

        if length < 0.001 {
            return;
        }

        // Perpendicular direction
        let nx = -dy / length;
        let ny = dx / length;

        let half_width = stroke.width / 2.0;
        let color = self.apply_opacity(stroke.paint.as_solid().unwrap_or(Color::BLACK));

        let base_index = self.vertices.len() as u32;

        // For lines, we use a minimal rect that doesn't affect SDF calculations
        let rect_pos = [0.0, 0.0];
        let rect_size = [1.0, 1.0];
        let corner_radii = [0.0; 4];

        // Four corners of the line quad
        let positions = [
            [from.x + nx * half_width, from.y + ny * half_width],
            [from.x - nx * half_width, from.y - ny * half_width],
            [to.x - nx * half_width, to.y - ny * half_width],
            [to.x + nx * half_width, to.y + ny * half_width],
        ];

        for pos in positions {
            self.vertices.push(RectVertex::solid(pos, color, rect_pos, rect_size, corner_radii));
        }

        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    fn draw_polyline(&mut self, points: &[Point], stroke: &Stroke) {
        if points.len() < 2 {
            return;
        }

        for window in points.windows(2) {
            self.draw_line(window[0], window[1], stroke);
        }
    }

    fn fill_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, paint: impl Into<Paint>) {
        // Approximate ellipse as a rounded rect with full radius
        let rect = Rect::from_center(center, Size::new(radius_x * 2.0, radius_y * 2.0));
        let radius = radius_x.min(radius_y);
        let rrect = RoundedRect::new(rect, radius);
        self.fill_rounded_rect(rrect, paint);
    }

    fn stroke_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, stroke: &Stroke) {
        // Approximate ellipse as a rounded rect with full radius
        let rect = Rect::from_center(center, Size::new(radius_x * 2.0, radius_y * 2.0));
        let radius = radius_x.min(radius_y);
        let rrect = RoundedRect::new(rect, radius);
        self.stroke_rounded_rect(rrect, stroke);
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        if self.current_blend_mode != mode {
            self.current_blend_mode = mode;
            self.state_changes += 1;
            // Note: Would need to flush and change pipeline for different blend modes
        }
    }

    fn blend_mode(&self) -> BlendMode {
        self.current_blend_mode
    }

    fn set_opacity(&mut self, opacity: f32) {
        self.current_opacity = opacity.clamp(0.0, 1.0);
    }

    fn opacity(&self) -> f32 {
        self.current_opacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_vertex_size() {
        // Ensure vertex is properly sized for GPU
        // New size: position(2) + color0(4) + rect_pos(2) + rect_size(2) + corner_radii(4)
        //         + gradient_info(4) + gradient_end_stops(4) + color1(4) = 26 floats * 4 = 104 bytes
        assert_eq!(std::mem::size_of::<RectVertex>(), 104);
    }

    #[test]
    fn test_uniforms_size() {
        // Ensure uniforms are properly aligned
        assert_eq!(std::mem::size_of::<Uniforms>(), 80); // 4x4 mat + 2 vec2s
    }

    #[test]
    fn test_solid_vertex_creation() {
        let vertex = RectVertex::solid(
            [10.0, 20.0],
            Color::RED,
            [0.0, 0.0],
            [100.0, 100.0],
            [5.0, 5.0, 5.0, 5.0],
        );
        assert_eq!(vertex.gradient_info[0], PAINT_TYPE_SOLID as f32);
    }

    #[test]
    fn test_linear_gradient_vertex_creation() {
        let vertex = RectVertex::linear_gradient(
            [10.0, 20.0],
            [0.0, 0.0],
            [100.0, 100.0],
            [0.0; 4],
            [0.0, 0.0],   // start
            [1.0, 0.0],   // end
            0.0,          // stop0 offset
            Color::RED,
            1.0,          // stop1 offset
            Color::BLUE,
        );
        assert_eq!(vertex.gradient_info[0], PAINT_TYPE_LINEAR_GRADIENT as f32);
    }

    #[test]
    fn test_radial_gradient_vertex_creation() {
        let vertex = RectVertex::radial_gradient(
            [10.0, 20.0],
            [0.0, 0.0],
            [100.0, 100.0],
            [0.0; 4],
            [0.5, 0.5],   // center
            0.5,          // radius
            0.0,          // stop0 offset
            Color::WHITE,
            1.0,          // stop1 offset
            Color::BLACK,
        );
        assert_eq!(vertex.gradient_info[0], PAINT_TYPE_RADIAL_GRADIENT as f32);
    }
}
