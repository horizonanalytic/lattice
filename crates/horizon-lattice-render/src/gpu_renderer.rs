//! GPU-accelerated renderer implementation using wgpu.
//!
//! This module provides the [`GpuRenderer`] which implements the [`Renderer`] trait
//! using wgpu for hardware-accelerated 2D rendering.

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use tracing::debug;

use crate::atlas::TextureAtlas;
use crate::context::GraphicsContext;
use crate::damage::DamageTracker;
use crate::error::RenderResult;
use crate::gradient::{create_gradient_bind_group_layout, GradientAtlas};
use crate::image::{Image, ImageScaleMode, NinePatch};
use crate::layer::Layer;
use crate::offscreen::OffscreenSurface;
use crate::paint::{BlendMode, BoxShadow, Paint, Stroke};
use crate::renderer::{FrameStats, RenderStateStack, Renderer};
use crate::stencil::{ClipShape, ClipStack, StencilTexture};
use crate::surface::RenderSurface;
use crate::transform::Transform2D;
use crate::types::{Color, CornerRadii, Point, Rect, RoundedRect, Size};

/// Paint type constants for shader.
const PAINT_TYPE_SOLID: u32 = 0;
const PAINT_TYPE_LINEAR_GRADIENT: u32 = 1;
const PAINT_TYPE_RADIAL_GRADIENT: u32 = 2;
const PAINT_TYPE_LINEAR_GRADIENT_TEX: u32 = 3;
const PAINT_TYPE_RADIAL_GRADIENT_TEX: u32 = 4;

/// Blend modes that can be implemented with hardware blending.
/// Returns the wgpu BlendState for the given blend mode.
/// Complex blend modes (Overlay, SoftLight, etc.) that require shader-based
/// blending return the same state as Normal with a warning logged.
fn blend_state_for_mode(mode: BlendMode) -> wgpu::BlendState {
    use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState};

    match mode {
        // Normal: src_over blending (premultiplied alpha)
        BlendMode::Normal => BlendState::PREMULTIPLIED_ALPHA_BLENDING,

        // Multiply: dst * src (darkens the image)
        // Result = Src * Dst
        // For premultiplied alpha: out.rgb = src.rgb * dst.rgb, out.a = src.a * (1 - dst.a) + dst.a
        BlendMode::Multiply => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        },

        // Screen: 1 - (1-src) * (1-dst) = src + dst - src*dst
        // For premultiplied: out = src + dst * (1 - src)
        BlendMode::Screen => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrc,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Add: src + dst (clamped to 1)
        BlendMode::Add => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        },

        // Darken: min(src, dst)
        BlendMode::Darken => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Min,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Lighten: max(src, dst)
        BlendMode::Lighten => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Max,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Source: replace destination completely
        BlendMode::Source => BlendState::REPLACE,

        // Destination: keep destination, ignore source
        BlendMode::Destination => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        },

        // Source In: source where destination has alpha
        BlendMode::SourceIn => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::DstAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::DstAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
        },

        // Destination In: destination where source has alpha
        BlendMode::DestinationIn => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::SrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::SrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Source Out: source where destination is transparent
        BlendMode::SourceOut => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
        },

        // Destination Out: destination where source is transparent
        BlendMode::DestinationOut => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Source Atop: source over destination, only where destination exists
        BlendMode::SourceAtop => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::DstAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::DstAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Destination Atop: destination over source, only where source exists
        BlendMode::DestinationAtop => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::SrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::SrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Xor: source or destination but not both
        BlendMode::Xor => BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        },

        // Complex blend modes that require shader-based implementation.
        // For now, fall back to Normal blending.
        // TODO: Implement shader-based blending for these modes.
        BlendMode::Overlay
        | BlendMode::ColorDodge
        | BlendMode::ColorBurn
        | BlendMode::HardLight
        | BlendMode::SoftLight
        | BlendMode::Difference
        | BlendMode::Exclusion => {
            tracing::debug!(
                mode = ?mode,
                "Complex blend mode not yet implemented, falling back to Normal"
            );
            BlendState::PREMULTIPLIED_ALPHA_BLENDING
        }
    }
}

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

    /// Create a vertex for texture-based linear gradient rendering.
    /// Used when gradient has more than 2 stops.
    fn linear_gradient_tex(
        position: [f32; 2],
        rect_pos: [f32; 2],
        rect_size: [f32; 2],
        corner_radii: [f32; 4],
        start: [f32; 2],
        end: [f32; 2],
        tex_v: f32,
        opacity: f32,
    ) -> Self {
        Self {
            position,
            // Store opacity in color0.a for shader to apply
            color0: [1.0, 1.0, 1.0, opacity],
            rect_pos,
            rect_size,
            corner_radii,
            gradient_info: [PAINT_TYPE_LINEAR_GRADIENT_TEX as f32, start[0], start[1], end[0]],
            gradient_end_stops: [end[1], 0.0, 0.0, tex_v],
            color1: [0.0; 4],
        }
    }

    /// Create a vertex for texture-based radial gradient rendering.
    /// Used when gradient has more than 2 stops.
    fn radial_gradient_tex(
        position: [f32; 2],
        rect_pos: [f32; 2],
        rect_size: [f32; 2],
        corner_radii: [f32; 4],
        center: [f32; 2],
        radius: f32,
        tex_v: f32,
        opacity: f32,
    ) -> Self {
        Self {
            position,
            // Store opacity in color0.a for shader to apply
            color0: [1.0, 1.0, 1.0, opacity],
            rect_pos,
            rect_size,
            corner_radii,
            gradient_info: [PAINT_TYPE_RADIAL_GRADIENT_TEX as f32, center[0], center[1], radius],
            gradient_end_stops: [0.0, 0.0, 0.0, tex_v],
            color1: [0.0; 4],
        }
    }
}

/// Vertex data for textured quads (images).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ImageVertex {
    /// Position in pixels.
    position: [f32; 2],
    /// Texture coordinates.
    uv: [f32; 2],
    /// Tint color (premultiplied alpha).
    tint: [f32; 4],
}

impl ImageVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // uv
        2 => Float32x4, // tint
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    /// Create a vertex with position, UV, and tint.
    fn new(position: [f32; 2], uv: [f32; 2], tint: Color) -> Self {
        Self {
            position,
            uv,
            tint: tint.to_array(),
        }
    }
}

/// Vertex data for box shadows.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ShadowVertex {
    /// Position in pixels.
    position: [f32; 2],
    /// Shadow color (premultiplied alpha).
    color: [f32; 4],
    /// Center of the shadow-casting rectangle.
    rect_center: [f32; 2],
    /// Half-size of the rectangle (after spread applied).
    rect_half_size: [f32; 2],
    /// Shadow params: [sigma, corner_radius, offset_x, offset_y]
    shadow_params: [f32; 4],
    /// Flags: [inset, unused, unused, unused]
    flags: [f32; 4],
}

impl ShadowVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x4, // color
        2 => Float32x2, // rect_center
        3 => Float32x2, // rect_half_size
        4 => Float32x4, // shadow_params
        5 => Float32x4, // flags
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ShadowVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    /// Create a shadow vertex.
    fn new(
        position: [f32; 2],
        color: Color,
        rect_center: [f32; 2],
        rect_half_size: [f32; 2],
        sigma: f32,
        corner_radius: f32,
        offset: [f32; 2],
        inset: bool,
    ) -> Self {
        Self {
            position,
            color: color.to_array(),
            rect_center,
            rect_half_size,
            shadow_params: [sigma, corner_radius, offset[0], offset[1]],
            flags: [if inset { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
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

/// A batch of image draw commands for a single atlas.
struct ImageBatch {
    /// The atlas this batch uses.
    atlas: Arc<TextureAtlas>,
    /// Vertices for this batch.
    vertices: Vec<ImageVertex>,
    /// Indices for this batch.
    indices: Vec<u32>,
}

/// Maximum number of vertices per batch.
const MAX_VERTICES: usize = 65536;

/// Maximum number of indices per batch.
const MAX_INDICES: usize = MAX_VERTICES * 6 / 4; // 6 indices per 4 vertices (quad)

/// Create a rect pipeline for a specific blend mode.
fn create_rect_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    blend_mode: BlendMode,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("rect_pipeline_{:?}", blend_mode)),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[RectVertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend_state_for_mode(blend_mode)),
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
    })
}

/// Create an image pipeline for a specific blend mode.
fn create_image_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    blend_mode: BlendMode,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("image_pipeline_{:?}", blend_mode)),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[ImageVertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend_state_for_mode(blend_mode)),
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
    })
}

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
    /// Rect pipeline cache by blend mode.
    rect_pipelines: HashMap<BlendMode, wgpu::RenderPipeline>,
    /// Pipeline layout for rect pipelines.
    rect_pipeline_layout: wgpu::PipelineLayout,
    /// Rect shader module.
    rect_shader: wgpu::ShaderModule,
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
    /// Blend mode of the current batch.
    batch_blend_mode: BlendMode,

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

    // Image rendering resources
    /// Image pipeline cache by blend mode.
    image_pipelines: HashMap<BlendMode, wgpu::RenderPipeline>,
    /// Pipeline layout for image pipelines.
    image_pipeline_layout: wgpu::PipelineLayout,
    /// Image shader module.
    image_shader: wgpu::ShaderModule,
    /// Vertex buffer for images.
    image_vertex_buffer: wgpu::Buffer,
    /// Index buffer for images.
    image_index_buffer: wgpu::Buffer,
    /// Bind group layout for image textures.
    #[allow(dead_code)]
    image_bind_group_layout: wgpu::BindGroupLayout,

    /// Image batches (one per atlas used this frame).
    image_batches: Vec<ImageBatch>,

    /// Damage tracker for dirty region optimization.
    damage_tracker: DamageTracker,

    // === Stencil clipping support ===
    /// Stencil texture for advanced clipping.
    stencil_texture: Option<StencilTexture>,
    /// Pipeline for pushing clips (incrementing stencil).
    push_clip_pipeline: wgpu::RenderPipeline,
    /// Pipeline for popping clips (decrementing stencil).
    pop_clip_pipeline: wgpu::RenderPipeline,
    /// Pipeline for rendering content with stencil testing.
    stencil_rect_pipeline: wgpu::RenderPipeline,
    /// Clip stack for managing nested stencil clips.
    clip_stack: ClipStack,
    /// Pending clip shapes to be rendered this frame.
    pending_clips: Vec<(ClipShape, bool)>, // (shape, is_push)

    // === Box shadow support ===
    /// Render pipeline for box shadows.
    shadow_pipeline: wgpu::RenderPipeline,
    /// Vertex buffer for shadows.
    shadow_vertex_buffer: wgpu::Buffer,
    /// Index buffer for shadows.
    shadow_index_buffer: wgpu::Buffer,
    /// Shadow vertices waiting to be rendered.
    shadow_vertices: Vec<ShadowVertex>,
    /// Shadow indices waiting to be rendered.
    shadow_indices: Vec<u32>,

    // === Multi-stop gradient support ===
    /// Gradient texture atlas for gradients with >2 stops.
    gradient_atlas: GradientAtlas,
    /// Bind group layout for gradient textures.
    #[allow(dead_code)]
    gradient_bind_group_layout: wgpu::BindGroupLayout,
    /// Pipeline for texture-based gradient rendering.
    gradient_tex_pipeline: wgpu::RenderPipeline,
    /// Vertices for texture-based gradients (separate batch).
    gradient_tex_vertices: Vec<RectVertex>,
    /// Indices for texture-based gradients.
    gradient_tex_indices: Vec<u32>,
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

        // Create gradient bind group layout early (needed for rect pipeline layout)
        // The rect shader expects gradient texture bindings at group 1
        let gradient_bind_group_layout = create_gradient_bind_group_layout(device);

        // Create pipeline layout with both uniform and gradient bind groups
        // The rect shader requires group 1 bindings even for solid colors
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout, &gradient_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create initial rect pipeline for Normal blend mode
        let rect_pipeline_normal = create_rect_pipeline(
            device,
            &shader,
            &pipeline_layout,
            format,
            BlendMode::Normal,
        );

        // Initialize rect pipeline cache with Normal blend mode
        let mut rect_pipelines = HashMap::new();
        rect_pipelines.insert(BlendMode::Normal, rect_pipeline_normal);

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

        // === Image pipeline setup ===

        // Create image shader module
        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
        });

        // Create image texture bind group layout
        let image_bind_group_layout = TextureAtlas::bind_group_layout(device);

        // Create image pipeline layout
        let image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout, &image_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create initial image pipeline for Normal blend mode
        let image_pipeline_normal = create_image_pipeline(
            device,
            &image_shader,
            &image_pipeline_layout,
            format,
            BlendMode::Normal,
        );

        // Initialize image pipeline cache with Normal blend mode
        let mut image_pipelines = HashMap::new();
        image_pipelines.insert(BlendMode::Normal, image_pipeline_normal);

        // Create image vertex buffer
        let image_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<ImageVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create image index buffer
        let image_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_index_buffer"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // === Stencil clipping pipelines ===

        // Pipeline for pushing clips (increments stencil)
        let push_clip_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("push_clip_pipeline"),
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
                    blend: None, // Don't write to color buffer when pushing clips
                    write_mask: wgpu::ColorWrites::empty(),
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
            depth_stencil: Some(crate::stencil::push_clip_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pipeline for popping clips (decrements stencil)
        let pop_clip_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pop_clip_pipeline"),
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
                    blend: None, // Don't write to color buffer when popping clips
                    write_mask: wgpu::ColorWrites::empty(),
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
            depth_stencil: Some(crate::stencil::pop_clip_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pipeline for rendering content with stencil testing
        let stencil_rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stencil_rect_pipeline"),
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
            depth_stencil: Some(crate::stencil::content_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // === Box shadow pipeline ===

        // Create shadow shader module
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shadow.wgsl").into()),
        });

        // Create shadow render pipeline
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                buffers: &[ShadowVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shadow_shader,
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

        // Create shadow vertex buffer
        let shadow_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<ShadowVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create shadow index buffer
        let shadow_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_index_buffer"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // === Multi-stop gradient support ===
        // gradient_bind_group_layout was created earlier (needed for rect pipeline layout)
        let gradient_atlas = GradientAtlas::new(device, &gradient_bind_group_layout);

        // Create texture-based gradient pipeline (uses same layout as rect pipeline)
        let gradient_tex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gradient_tex_pipeline"),
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

        debug!(
            target: "horizon_lattice_render::gpu_renderer",
            format = ?format,
            max_vertices = MAX_VERTICES,
            max_indices = MAX_INDICES,
            "created GPU renderer with stencil clipping, box shadow, and multi-stop gradient support"
        );

        Ok(Self {
            state: RenderStateStack::new(),
            viewport_size: Size::ZERO,
            clear_color: Color::BLACK,
            in_frame: false,

            rect_pipelines,
            rect_pipeline_layout: pipeline_layout,
            rect_shader: shader,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            bind_group,

            vertices: Vec::with_capacity(MAX_VERTICES),
            indices: Vec::with_capacity(MAX_INDICES),
            batch_blend_mode: BlendMode::Normal,

            draw_calls: 0,
            vertex_count: 0,
            state_changes: 0,

            scissor_rect: None,
            current_blend_mode: BlendMode::Normal,
            current_opacity: 1.0,

            surface_format: format,

            image_pipelines,
            image_pipeline_layout,
            image_shader,
            image_vertex_buffer,
            image_index_buffer,
            image_bind_group_layout,
            image_batches: Vec::new(),

            damage_tracker: DamageTracker::new(),

            // Stencil clipping
            stencil_texture: None, // Created lazily when needed
            push_clip_pipeline,
            pop_clip_pipeline,
            stencil_rect_pipeline,
            clip_stack: ClipStack::new(),
            pending_clips: Vec::new(),

            // Box shadows
            shadow_pipeline,
            shadow_vertex_buffer,
            shadow_index_buffer,
            shadow_vertices: Vec::with_capacity(MAX_VERTICES),
            shadow_indices: Vec::with_capacity(MAX_INDICES),

            // Multi-stop gradients
            gradient_atlas,
            gradient_bind_group_layout,
            gradient_tex_pipeline,
            gradient_tex_vertices: Vec::with_capacity(MAX_VERTICES / 4),
            gradient_tex_indices: Vec::with_capacity(MAX_INDICES / 4),
        })
    }

    /// Get or create a rect pipeline for the given blend mode.
    fn get_rect_pipeline(&mut self, blend_mode: BlendMode) -> &wgpu::RenderPipeline {
        // Check if we already have this pipeline
        if !self.rect_pipelines.contains_key(&blend_mode) {
            let ctx = GraphicsContext::get();
            let pipeline = create_rect_pipeline(
                ctx.device(),
                &self.rect_shader,
                &self.rect_pipeline_layout,
                self.surface_format,
                blend_mode,
            );
            self.rect_pipelines.insert(blend_mode, pipeline);
            debug!(
                target: "horizon_lattice_render::gpu_renderer",
                ?blend_mode,
                "created rect pipeline for blend mode"
            );
        }
        self.rect_pipelines.get(&blend_mode).unwrap()
    }

    /// Get or create an image pipeline for the given blend mode.
    fn get_image_pipeline(&mut self, blend_mode: BlendMode) -> &wgpu::RenderPipeline {
        // Check if we already have this pipeline
        if !self.image_pipelines.contains_key(&blend_mode) {
            let ctx = GraphicsContext::get();
            let pipeline = create_image_pipeline(
                ctx.device(),
                &self.image_shader,
                &self.image_pipeline_layout,
                self.surface_format,
                blend_mode,
            );
            self.image_pipelines.insert(blend_mode, pipeline);
            debug!(
                target: "horizon_lattice_render::gpu_renderer",
                ?blend_mode,
                "created image pipeline for blend mode"
            );
        }
        self.image_pipelines.get(&blend_mode).unwrap()
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

        // Check if we need stencil clipping
        let use_stencil = self.clip_stack.has_clips() || !self.pending_clips.is_empty();

        // Ensure stencil texture if needed
        if use_stencil {
            let width = self.viewport_size.width as u32;
            let height = self.viewport_size.height as u32;
            self.ensure_stencil_texture(device, width, height);
        }

        // Update uniform buffer
        let uniforms = Uniforms {
            transform: self.state.transform().to_mat4().to_cols_array_2d(),
            viewport_size: [self.viewport_size.width, self.viewport_size.height],
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Upload vertex and index data for rectangles
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }

        // Upload gradient atlas if needed
        if !self.gradient_tex_vertices.is_empty() {
            self.gradient_atlas.upload(queue);
        }

        // Upload vertex and index data for shadows
        if !self.shadow_vertices.is_empty() {
            queue.write_buffer(&self.shadow_vertex_buffer, 0, bytemuck::cast_slice(&self.shadow_vertices));
            queue.write_buffer(&self.shadow_index_buffer, 0, bytemuck::cast_slice(&self.shadow_indices));
        }

        // Ensure we have the rect pipeline for the current blend mode
        let batch_blend_mode = self.batch_blend_mode;
        let _ = self.get_rect_pipeline(batch_blend_mode);
        let _ = self.get_image_pipeline(batch_blend_mode);

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Get stencil attachment if needed
        let depth_stencil_attachment = if use_stencil {
            self.stencil_texture.as_ref().map(|tex| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: tex.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }
            })
        } else {
            None
        };

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
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Apply scissor if set
            if let Some(scissor) = &self.scissor_rect {
                render_pass.set_scissor_rect(
                    scissor.left().max(0.0) as u32,
                    scissor.top().max(0.0) as u32,
                    scissor.width().max(0.0) as u32,
                    scissor.height().max(0.0) as u32,
                );
            }

            // Process pending clip operations
            if !self.pending_clips.is_empty() {
                let clips: Vec<_> = self.pending_clips.drain(..).collect();
                let mut current_stencil_ref = 0u32;

                for (shape, is_push) in clips {
                    let (clip_vertices, clip_indices) = self.clip_shape_to_vertices(&shape);

                    // Upload clip geometry
                    queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&clip_vertices));
                    queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&clip_indices));

                    if is_push {
                        // Push clip: increment stencil where shape is drawn
                        render_pass.set_pipeline(&self.push_clip_pipeline);
                        render_pass.set_stencil_reference(current_stencil_ref);
                        current_stencil_ref += 1;
                    } else {
                        // Pop clip: decrement stencil where shape is drawn
                        render_pass.set_pipeline(&self.pop_clip_pipeline);
                        render_pass.set_stencil_reference(current_stencil_ref);
                        current_stencil_ref = current_stencil_ref.saturating_sub(1);
                    }

                    render_pass.set_bind_group(0, &self.bind_group, &[]);
                    render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..clip_indices.len() as u32, 0, 0..1);
                    self.draw_calls += 1;
                }

                // Re-upload content geometry after clip operations
                if !self.vertices.is_empty() {
                    queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
                    queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
                }
            }

            // Render shadows (before rectangles so they appear behind)
            if !self.shadow_indices.is_empty() {
                render_pass.set_pipeline(&self.shadow_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.shadow_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.shadow_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.shadow_indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render rectangles
            if !self.indices.is_empty() {
                // Use stencil pipeline if we have active clips
                if self.clip_stack.has_clips() {
                    render_pass.set_pipeline(&self.stencil_rect_pipeline);
                    render_pass.set_stencil_reference(self.clip_stack.depth());
                } else {
                    // Get pipeline for current blend mode (already ensured to exist)
                    let rect_pipeline = self.rect_pipelines.get(&batch_blend_mode).unwrap();
                    render_pass.set_pipeline(rect_pipeline);
                }
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render texture-based gradients (multi-stop gradients)
            if !self.gradient_tex_indices.is_empty() {
                // Upload gradient texture vertices
                queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.gradient_tex_vertices));
                queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.gradient_tex_indices));

                render_pass.set_pipeline(&self.gradient_tex_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.gradient_tex_indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render images (one draw call per atlas)
            if !self.image_batches.is_empty() {
                // Get pipeline for current blend mode (already ensured to exist)
                let image_pipeline = self.image_pipelines.get(&batch_blend_mode).unwrap();
                render_pass.set_pipeline(image_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);

                for batch in &self.image_batches {
                    if batch.indices.is_empty() {
                        continue;
                    }

                    // Upload batch vertices and indices
                    queue.write_buffer(&self.image_vertex_buffer, 0, bytemuck::cast_slice(&batch.vertices));
                    queue.write_buffer(&self.image_index_buffer, 0, bytemuck::cast_slice(&batch.indices));

                    render_pass.set_bind_group(1, batch.atlas.bind_group(), &[]);
                    render_pass.set_vertex_buffer(0, self.image_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.image_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..batch.indices.len() as u32, 0, 0..1);
                    self.draw_calls += 1;
                }
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
        self.image_batches.clear();
        self.gradient_tex_vertices.clear();
        self.gradient_tex_indices.clear();
        self.gradient_atlas.clear();
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

        // Upload shadow vertex and index data
        if !self.shadow_vertices.is_empty() {
            queue.write_buffer(&self.shadow_vertex_buffer, 0, bytemuck::cast_slice(&self.shadow_vertices));
            queue.write_buffer(&self.shadow_index_buffer, 0, bytemuck::cast_slice(&self.shadow_indices));
        }

        // Ensure we have the pipelines for the current blend mode
        let batch_blend_mode = self.batch_blend_mode;
        let _ = self.get_rect_pipeline(batch_blend_mode);
        let _ = self.get_image_pipeline(batch_blend_mode);

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

            // Apply scissor if set
            if let Some(scissor) = &self.scissor_rect {
                render_pass.set_scissor_rect(
                    scissor.left().max(0.0) as u32,
                    scissor.top().max(0.0) as u32,
                    scissor.width().max(0.0) as u32,
                    scissor.height().max(0.0) as u32,
                );
            }

            // Render shadows (before rectangles so they appear behind)
            if !self.shadow_indices.is_empty() {
                render_pass.set_pipeline(&self.shadow_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.shadow_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.shadow_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.shadow_indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render rectangles
            if !self.indices.is_empty() {
                let rect_pipeline = self.rect_pipelines.get(&batch_blend_mode).unwrap();
                render_pass.set_pipeline(rect_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render texture-based gradients (multi-stop gradients)
            if !self.gradient_tex_indices.is_empty() {
                // Upload gradient atlas
                self.gradient_atlas.upload(queue);

                // Upload gradient texture vertices
                queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.gradient_tex_vertices));
                queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.gradient_tex_indices));

                render_pass.set_pipeline(&self.gradient_tex_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.gradient_tex_indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render images (one draw call per atlas)
            if !self.image_batches.is_empty() {
                let image_pipeline = self.image_pipelines.get(&batch_blend_mode).unwrap();
                render_pass.set_pipeline(image_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);

                for batch in &self.image_batches {
                    if batch.indices.is_empty() {
                        continue;
                    }

                    // Upload batch vertices and indices
                    queue.write_buffer(&self.image_vertex_buffer, 0, bytemuck::cast_slice(&batch.vertices));
                    queue.write_buffer(&self.image_index_buffer, 0, bytemuck::cast_slice(&batch.indices));

                    render_pass.set_bind_group(1, batch.atlas.bind_group(), &[]);
                    render_pass.set_vertex_buffer(0, self.image_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.image_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..batch.indices.len() as u32, 0, 0..1);
                    self.draw_calls += 1;
                }
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
        self.shadow_vertices.clear();
        self.shadow_indices.clear();
        self.image_batches.clear();
        self.gradient_tex_vertices.clear();
        self.gradient_tex_indices.clear();
        self.gradient_atlas.clear();
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

        let positions = [
            [rect.left(), rect.top()],
            [rect.right(), rect.top()],
            [rect.right(), rect.bottom()],
            [rect.left(), rect.bottom()],
        ];

        // Use texture-based gradient for >2 stops
        if gradient.stops.len() > 2 {
            if let Some(gradient_id) = self.gradient_atlas.get_or_create(&gradient.stops) {
                let tex_v = gradient_id.tex_v();
                let opacity = self.current_opacity;
                let base_index = self.gradient_tex_vertices.len() as u32;

                for pos in positions {
                    self.gradient_tex_vertices.push(RectVertex::linear_gradient_tex(
                        pos,
                        rect_pos,
                        rect_size,
                        corner_radii,
                        start,
                        end,
                        tex_v,
                        opacity,
                    ));
                }

                self.gradient_tex_indices.extend_from_slice(&[
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index,
                    base_index + 2,
                    base_index + 3,
                ]);

                self.vertex_count += 4;
                return;
            }
            // Fall through to 2-stop path if atlas is full
        }

        // Use the 2-stop gradient path
        let base_index = self.vertices.len() as u32;
        let (stop0_offset, stop0_color, stop1_offset, stop1_color) = self.extract_two_stops(&gradient.stops);
        let stop0_color = self.apply_opacity(stop0_color);
        let stop1_color = self.apply_opacity(stop1_color);

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

        let positions = [
            [rect.left(), rect.top()],
            [rect.right(), rect.top()],
            [rect.right(), rect.bottom()],
            [rect.left(), rect.bottom()],
        ];

        // Use texture-based gradient for >2 stops
        if gradient.stops.len() > 2 {
            if let Some(gradient_id) = self.gradient_atlas.get_or_create(&gradient.stops) {
                let tex_v = gradient_id.tex_v();
                let opacity = self.current_opacity;
                let base_index = self.gradient_tex_vertices.len() as u32;

                for pos in positions {
                    self.gradient_tex_vertices.push(RectVertex::radial_gradient_tex(
                        pos,
                        rect_pos,
                        rect_size,
                        corner_radii,
                        center,
                        normalized_radius,
                        tex_v,
                        opacity,
                    ));
                }

                self.gradient_tex_indices.extend_from_slice(&[
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index,
                    base_index + 2,
                    base_index + 3,
                ]);

                self.vertex_count += 4;
                return;
            }
            // Fall through to 2-stop path if atlas is full
        }

        // Use the 2-stop gradient path
        let base_index = self.vertices.len() as u32;
        let (stop0_offset, stop0_color, stop1_offset, stop1_color) = self.extract_two_stops(&gradient.stops);
        let stop0_color = self.apply_opacity(stop0_color);
        let stop1_color = self.apply_opacity(stop1_color);

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

    /// Ensure stencil texture exists and is the correct size.
    fn ensure_stencil_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let needs_creation = match &self.stencil_texture {
            None => true,
            Some(tex) => tex.size().width as u32 != width || tex.size().height as u32 != height,
        };

        if needs_creation {
            self.stencil_texture = Some(StencilTexture::new(device, width, height));
        }
    }

    /// Generate vertices for a clip shape.
    fn clip_shape_to_vertices(&self, shape: &ClipShape) -> (Vec<RectVertex>, Vec<u32>) {
        // White color for stencil mask (color doesn't matter, we're not writing to color buffer)
        let color = Color::WHITE;

        match shape {
            ClipShape::RoundedRect(rr) => {
                let (rect, radii) = (rr.rect, rr.radii);
                let base_index = 0u32;
                let rect_pos = [rect.left(), rect.top()];
                let rect_size = [rect.width(), rect.height()];
                let corner_radii = [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left];

                let positions = [
                    [rect.left(), rect.top()],
                    [rect.right(), rect.top()],
                    [rect.right(), rect.bottom()],
                    [rect.left(), rect.bottom()],
                ];

                let vertices: Vec<RectVertex> = positions
                    .iter()
                    .map(|&pos| RectVertex::solid(pos, color, rect_pos, rect_size, corner_radii))
                    .collect();

                let indices = vec![
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index,
                    base_index + 2,
                    base_index + 3,
                ];

                (vertices, indices)
            }
            ClipShape::Rect(r) => {
                let rect = *r;
                let base_index = 0u32;
                let rect_pos = [rect.left(), rect.top()];
                let rect_size = [rect.width(), rect.height()];
                let corner_radii = [0.0; 4];

                let positions = [
                    [rect.left(), rect.top()],
                    [rect.right(), rect.top()],
                    [rect.right(), rect.bottom()],
                    [rect.left(), rect.bottom()],
                ];

                let vertices: Vec<RectVertex> = positions
                    .iter()
                    .map(|&pos| RectVertex::solid(pos, color, rect_pos, rect_size, corner_radii))
                    .collect();

                let indices = vec![
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index,
                    base_index + 2,
                    base_index + 3,
                ];

                (vertices, indices)
            }
            ClipShape::Path(path) => {
                // Tessellate the path for stencil clipping
                let tessellated = crate::path::tessellate_fill(
                    path,
                    crate::paint::FillRule::NonZero,
                    crate::path::DEFAULT_TOLERANCE,
                );

                if tessellated.is_empty() {
                    return (Vec::new(), Vec::new());
                }

                // Use minimal rect data that won't affect stencil
                let rect_pos = [0.0, 0.0];
                let rect_size = [1.0, 1.0];
                let corner_radii = [0.0; 4];

                let vertices: Vec<RectVertex> = tessellated.vertices
                    .iter()
                    .map(|pos| RectVertex::solid(*pos, color, rect_pos, rect_size, corner_radii))
                    .collect();

                (vertices, tessellated.indices)
            }
        }
    }

    // =========================================================================
    // Damage Tracking
    // =========================================================================

    /// Get a reference to the damage tracker.
    pub fn damage_tracker(&self) -> &DamageTracker {
        &self.damage_tracker
    }

    /// Get a mutable reference to the damage tracker.
    pub fn damage_tracker_mut(&mut self) -> &mut DamageTracker {
        &mut self.damage_tracker
    }

    /// Mark a region as damaged (needs repainting).
    ///
    /// Call this when content in the specified region has changed.
    /// The damage will be used to optimize rendering by only updating
    /// the affected areas.
    pub fn add_damage(&mut self, rect: Rect) {
        self.damage_tracker.add_damage(rect);
    }

    /// Request a full repaint of the entire viewport.
    pub fn invalidate_all(&mut self) {
        self.damage_tracker.invalidate_all();
    }

    /// Clear all recorded damage.
    ///
    /// Should be called after rendering the damaged regions.
    pub fn clear_damage(&mut self) {
        self.damage_tracker.clear();
    }

    /// Check if there is any damage that needs rendering.
    pub fn has_damage(&self) -> bool {
        self.damage_tracker.has_damage()
    }

    /// Get the current damage region.
    ///
    /// Returns `None` if no damage has been recorded.
    pub fn damage_region(&self) -> Option<Rect> {
        self.damage_tracker.damage_region()
    }

    // =========================================================================
    // Layer Rendering
    // =========================================================================

    /// Render to a layer.
    ///
    /// This should be called after `end_frame()` to submit the
    /// rendered content to the layer's texture.
    pub fn render_to_layer(&mut self, layer: &Layer) -> RenderResult<FrameStats> {
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

        // Upload shadow vertex and index data
        if !self.shadow_vertices.is_empty() {
            queue.write_buffer(&self.shadow_vertex_buffer, 0, bytemuck::cast_slice(&self.shadow_vertices));
            queue.write_buffer(&self.shadow_index_buffer, 0, bytemuck::cast_slice(&self.shadow_indices));
        }

        // Ensure we have the pipelines for the current blend mode
        let batch_blend_mode = self.batch_blend_mode;
        let _ = self.get_rect_pipeline(batch_blend_mode);
        let _ = self.get_image_pipeline(batch_blend_mode);

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("layer_render_encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("layer_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: layer.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(layer.clear_color().to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Apply scissor if set
            if let Some(scissor) = &self.scissor_rect {
                render_pass.set_scissor_rect(
                    scissor.left().max(0.0) as u32,
                    scissor.top().max(0.0) as u32,
                    scissor.width().max(0.0) as u32,
                    scissor.height().max(0.0) as u32,
                );
            }

            // Render shadows (before rectangles so they appear behind)
            if !self.shadow_indices.is_empty() {
                render_pass.set_pipeline(&self.shadow_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.shadow_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.shadow_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.shadow_indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render rectangles
            if !self.indices.is_empty() {
                let rect_pipeline = self.rect_pipelines.get(&batch_blend_mode).unwrap();
                render_pass.set_pipeline(rect_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_bind_group(1, self.gradient_atlas.bind_group(), &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
                self.draw_calls += 1;
            }

            // Render images (one draw call per atlas)
            if !self.image_batches.is_empty() {
                let image_pipeline = self.image_pipelines.get(&batch_blend_mode).unwrap();
                render_pass.set_pipeline(image_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);

                for batch in &self.image_batches {
                    if batch.indices.is_empty() {
                        continue;
                    }

                    // Upload batch vertices and indices
                    queue.write_buffer(&self.image_vertex_buffer, 0, bytemuck::cast_slice(&batch.vertices));
                    queue.write_buffer(&self.image_index_buffer, 0, bytemuck::cast_slice(&batch.indices));

                    render_pass.set_bind_group(1, batch.atlas.bind_group(), &[]);
                    render_pass.set_vertex_buffer(0, self.image_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.image_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..batch.indices.len() as u32, 0, 0..1);
                    self.draw_calls += 1;
                }
            }
        }

        // Submit (no present for layer)
        queue.submit(std::iter::once(encoder.finish()));

        let stats = FrameStats {
            draw_calls: self.draw_calls,
            vertices: self.vertex_count,
            state_changes: self.state_changes,
        };

        // Reset for next frame
        self.vertices.clear();
        self.indices.clear();
        self.shadow_vertices.clear();
        self.shadow_indices.clear();
        self.image_batches.clear();
        self.draw_calls = 0;
        self.vertex_count = 0;
        self.state_changes = 0;
        self.in_frame = false;

        Ok(stats)
    }

    /// Calculate the destination rectangle based on scale mode.
    fn calculate_scaled_dest(&self, image_size: Size, dest: Rect, scale_mode: ImageScaleMode) -> Rect {
        match scale_mode {
            ImageScaleMode::Stretch => dest,
            ImageScaleMode::Fit => {
                let scale_x = dest.width() / image_size.width;
                let scale_y = dest.height() / image_size.height;
                let scale = scale_x.min(scale_y);

                let new_width = image_size.width * scale;
                let new_height = image_size.height * scale;

                // Center the image
                let offset_x = (dest.width() - new_width) / 2.0;
                let offset_y = (dest.height() - new_height) / 2.0;

                Rect::new(
                    dest.left() + offset_x,
                    dest.top() + offset_y,
                    new_width,
                    new_height,
                )
            }
            ImageScaleMode::Fill => {
                let scale_x = dest.width() / image_size.width;
                let scale_y = dest.height() / image_size.height;
                let scale = scale_x.max(scale_y);

                let new_width = image_size.width * scale;
                let new_height = image_size.height * scale;

                // Center the image (it will be cropped by the clip rect)
                let offset_x = (dest.width() - new_width) / 2.0;
                let offset_y = (dest.height() - new_height) / 2.0;

                Rect::new(
                    dest.left() + offset_x,
                    dest.top() + offset_y,
                    new_width,
                    new_height,
                )
            }
            ImageScaleMode::Tile => {
                // For tiling, we just use the original dest and handle tiling in the shader
                // For now, just stretch
                dest
            }
        }
    }

    /// Add an image quad to the appropriate batch.
    fn add_image_quad(
        &mut self,
        atlas: &Arc<TextureAtlas>,
        dest: Rect,
        uvs: [f32; 4], // [u_min, v_min, u_max, v_max]
        tint: Color,
    ) {
        // Find or create batch for this atlas
        let batch_idx = self
            .image_batches
            .iter()
            .position(|b| Arc::ptr_eq(&b.atlas, atlas));

        let batch_idx = batch_idx.unwrap_or_else(|| {
            self.image_batches.push(ImageBatch {
                atlas: atlas.clone(),
                vertices: Vec::new(),
                indices: Vec::new(),
            });
            self.image_batches.len() - 1
        });

        let batch = &mut self.image_batches[batch_idx];
        let base_index = batch.vertices.len() as u32;

        // Unpack UVs
        let [u_min, v_min, u_max, v_max] = uvs;

        // Add four vertices for the quad
        batch.vertices.push(ImageVertex::new([dest.left(), dest.top()], [u_min, v_min], tint));
        batch.vertices.push(ImageVertex::new([dest.right(), dest.top()], [u_max, v_min], tint));
        batch.vertices.push(ImageVertex::new([dest.right(), dest.bottom()], [u_max, v_max], tint));
        batch.vertices.push(ImageVertex::new([dest.left(), dest.bottom()], [u_min, v_max], tint));

        // Add indices for two triangles
        batch.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
    }

    // === Shader hot-reload support ===

    /// Reload shaders from the given reload result.
    ///
    /// This method updates the shader modules and clears the pipeline caches,
    /// causing pipelines to be recreated on the next draw call.
    ///
    /// Only available when the `shader-hot-reload` feature is enabled.
    #[cfg(feature = "shader-hot-reload")]
    pub fn reload_shaders(&mut self, result: &crate::shader_watcher::ShaderReloadResult) {
        use crate::shader_watcher::ShaderKind;

        let ctx = GraphicsContext::get();
        let device = ctx.device();

        for (kind, source, _module) in &result.modules {
            match kind {
                ShaderKind::Rect => {
                    // Update rect shader and clear pipeline cache
                    self.rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("rect_shader"),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });
                    self.rect_pipelines.clear();

                    // Re-insert the Normal blend mode pipeline immediately
                    let pipeline = create_rect_pipeline(
                        device,
                        &self.rect_shader,
                        &self.rect_pipeline_layout,
                        self.surface_format,
                        BlendMode::Normal,
                    );
                    self.rect_pipelines.insert(BlendMode::Normal, pipeline);

                    // Recreate stencil pipelines that use the rect shader
                    self.recreate_stencil_pipelines();

                    tracing::info!("Reloaded rect shader and recreated {} pipelines", self.rect_pipelines.len() + 3);
                }
                ShaderKind::Image => {
                    // Update image shader and clear pipeline cache
                    self.image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("image_shader"),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });
                    self.image_pipelines.clear();

                    // Re-insert the Normal blend mode pipeline immediately
                    let pipeline = create_image_pipeline(
                        device,
                        &self.image_shader,
                        &self.image_pipeline_layout,
                        self.surface_format,
                        BlendMode::Normal,
                    );
                    self.image_pipelines.insert(BlendMode::Normal, pipeline);

                    tracing::info!("Reloaded image shader and recreated pipeline cache");
                }
                ShaderKind::Shadow => {
                    // Update shadow shader and recreate pipeline
                    let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("shadow_shader"),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });

                    self.shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("shadow_pipeline"),
                        layout: Some(&self.rect_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shadow_shader,
                            entry_point: Some("vs_main"),
                            buffers: &[ShadowVertex::desc()],
                            compilation_options: Default::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shadow_shader,
                            entry_point: Some("fs_main"),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: self.surface_format,
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

                    tracing::info!("Reloaded shadow shader and recreated pipeline");
                }
                ShaderKind::Composite => {
                    // Composite shader is used by the Compositor, not directly by GpuRenderer
                    tracing::info!("Composite shader changed - Compositor must be recreated to use new shader");
                }
            }
        }
    }

    /// Recreate stencil-related pipelines after shader reload.
    #[cfg(feature = "shader-hot-reload")]
    fn recreate_stencil_pipelines(&mut self) {
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        // Push clip pipeline (increments stencil)
        self.push_clip_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("push_clip_pipeline"),
            layout: Some(&self.rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
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
            depth_stencil: Some(crate::stencil::push_clip_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pop clip pipeline (decrements stencil)
        self.pop_clip_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pop_clip_pipeline"),
            layout: Some(&self.rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
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
            depth_stencil: Some(crate::stencil::pop_clip_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Stencil rect pipeline (for rendering with stencil testing)
        self.stencil_rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stencil_rect_pipeline"),
            layout: Some(&self.rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_format,
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
            depth_stencil: Some(crate::stencil::content_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
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
        self.image_batches.clear();
        self.scissor_rect = None;

        // Reset blend mode tracking
        self.current_blend_mode = BlendMode::Normal;
        self.batch_blend_mode = BlendMode::Normal;

        // Reset shadow buffers
        self.shadow_vertices.clear();
        self.shadow_indices.clear();

        // Reset stencil clipping state
        self.clip_stack.reset();
        self.pending_clips.clear();

        // Update damage tracker viewport
        self.damage_tracker.set_viewport(Rect::new(
            0.0,
            0.0,
            viewport_size.width,
            viewport_size.height,
        ));
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

    fn clip_rounded_rect(&mut self, rrect: RoundedRect) {
        // If no rounding, use simple scissor clip
        if rrect.radii.is_zero() {
            self.clip_rect(rrect.rect);
            return;
        }

        // Transform the clip rectangle
        let transformed_rect = self.state.transform().transform_rect(&rrect.rect);
        let shape = ClipShape::RoundedRect(RoundedRect {
            rect: transformed_rect,
            radii: rrect.radii,
        });

        // Push to clip stack and record for rendering
        if let Some(_depth) = self.clip_stack.push(shape.clone()) {
            self.pending_clips.push((shape, true)); // true = push
        }
    }

    fn clip_path(&mut self, path: &crate::types::Path) {
        if path.is_empty() {
            return;
        }

        // Transform the path
        let transformed_path = path.transformed(self.state.transform());
        let shape = ClipShape::Path(transformed_path);

        // Push to clip stack and record for rendering
        if let Some(_depth) = self.clip_stack.push(shape.clone()) {
            self.pending_clips.push((shape, true)); // true = push
        }
    }

    fn restore_clip(&mut self) {
        if let Some((shape, _depth)) = self.clip_stack.pop() {
            self.pending_clips.push((shape, false)); // false = pop
        }
    }

    fn clip_bounds(&self) -> Option<Rect> {
        self.state.clip_bounds()
    }

    fn has_stencil_clips(&self) -> bool {
        self.clip_stack.has_clips()
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

    fn draw_box_shadow(&mut self, rect: Rect, shadow: &BoxShadow) {
        self.draw_box_shadow_rounded(RoundedRect::new(rect, 0.0), shadow);
    }

    fn draw_box_shadow_rounded(&mut self, rrect: RoundedRect, shadow: &BoxShadow) {
        // Get the maximum corner radius for the shadow
        let max_radius = rrect.radii.max();

        // Calculate the expanded bounds for the shadow quad
        // The shadow extends beyond the shape by blur + spread
        let expand = shadow.blur_radius + shadow.spread_radius.max(0.0);

        // For inset shadows, we render within the original bounds
        // For outer shadows, we need to expand the rendering area
        let render_rect = if shadow.inset {
            rrect.rect
        } else {
            // Expand by blur + spread + some padding for smooth falloff
            // We add 3*sigma extra padding to ensure the Gaussian tail is captured
            let sigma = shadow.sigma();
            let total_expand = expand + sigma * 3.0;

            Rect::new(
                rrect.rect.left() - total_expand + shadow.offset_x.min(0.0),
                rrect.rect.top() - total_expand + shadow.offset_y.min(0.0),
                rrect.rect.width() + total_expand * 2.0 + shadow.offset_x.abs(),
                rrect.rect.height() + total_expand * 2.0 + shadow.offset_y.abs(),
            )
        };

        // Transform the render rect
        let transformed_render = self.state.transform().transform_rect(&render_rect);
        let transformed_shape = self.state.transform().transform_rect(&rrect.rect);

        // Calculate shadow rectangle center and half-size (with spread applied)
        let spread = shadow.spread_radius;
        let shadow_half_size = [
            (rrect.rect.width() / 2.0 + spread).max(0.0),
            (rrect.rect.height() / 2.0 + spread).max(0.0),
        ];
        let rect_center = [
            transformed_shape.left() + rrect.rect.width() / 2.0,
            transformed_shape.top() + rrect.rect.height() / 2.0,
        ];

        // Corner radius (adjusted for spread)
        let corner_radius = (max_radius + spread).max(0.0);

        // Apply opacity to shadow color
        let color = self.apply_opacity(shadow.color);

        // Create quad vertices for the shadow
        let sigma = shadow.sigma();
        let offset = [shadow.offset_x, shadow.offset_y];
        let inset = shadow.inset;

        let base_index = self.shadow_vertices.len() as u32;

        // Four corners of the render quad
        let positions = [
            [transformed_render.left(), transformed_render.top()],
            [transformed_render.right(), transformed_render.top()],
            [transformed_render.right(), transformed_render.bottom()],
            [transformed_render.left(), transformed_render.bottom()],
        ];

        for pos in positions {
            self.shadow_vertices.push(ShadowVertex::new(
                pos,
                color,
                rect_center,
                shadow_half_size,
                sigma,
                corner_radius,
                offset,
                inset,
            ));
        }

        // Two triangles for the quad
        self.shadow_indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);

        self.vertex_count += 4;
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

    fn fill_path(&mut self, path: &crate::types::Path, paint: impl Into<Paint>, fill_rule: crate::paint::FillRule) {
        if path.is_empty() {
            return;
        }

        let paint = paint.into();

        // Tessellate the path
        let tessellated = crate::path::tessellate_fill(path, fill_rule, crate::path::DEFAULT_TOLERANCE);

        if tessellated.is_empty() {
            return;
        }

        // Get color from paint (for now, only solid colors supported for paths)
        let color = match &paint {
            Paint::Solid(c) => self.apply_opacity(*c),
            Paint::LinearGradient(g) => {
                // Use first stop color as fallback
                if let Some(stop) = g.stops.first() {
                    self.apply_opacity(stop.color)
                } else {
                    Color::BLACK
                }
            }
            Paint::RadialGradient(g) => {
                if let Some(stop) = g.stops.first() {
                    self.apply_opacity(stop.color)
                } else {
                    Color::BLACK
                }
            }
        };

        let base_index = self.vertices.len() as u32;

        // For paths, we use a minimal rect that doesn't affect SDF calculations
        let rect_pos = [0.0, 0.0];
        let rect_size = [1.0, 1.0];
        let corner_radii = [0.0; 4];

        // Add vertices
        for pos in &tessellated.vertices {
            // Apply transform to each vertex
            let p = self.state.transform().transform_point(Point::new(pos[0], pos[1]));
            self.vertices.push(RectVertex::solid(
                [p.x, p.y],
                color,
                rect_pos,
                rect_size,
                corner_radii,
            ));
        }

        // Add indices (offset by base_index)
        for idx in &tessellated.indices {
            self.indices.push(base_index + idx);
        }

        self.vertex_count += tessellated.vertices.len() as u32;
    }

    fn stroke_path(&mut self, path: &crate::types::Path, stroke: &Stroke) {
        if path.is_empty() {
            return;
        }

        // Tessellate the stroke
        let tessellated = crate::path::tessellate_stroke(path, stroke, crate::path::DEFAULT_TOLERANCE);

        if tessellated.is_empty() {
            return;
        }

        // Get color from stroke paint
        let color = match &stroke.paint {
            Paint::Solid(c) => self.apply_opacity(*c),
            Paint::LinearGradient(g) => {
                if let Some(stop) = g.stops.first() {
                    self.apply_opacity(stop.color)
                } else {
                    Color::BLACK
                }
            }
            Paint::RadialGradient(g) => {
                if let Some(stop) = g.stops.first() {
                    self.apply_opacity(stop.color)
                } else {
                    Color::BLACK
                }
            }
        };

        let base_index = self.vertices.len() as u32;

        // For paths, we use a minimal rect that doesn't affect SDF calculations
        let rect_pos = [0.0, 0.0];
        let rect_size = [1.0, 1.0];
        let corner_radii = [0.0; 4];

        // Add vertices
        for pos in &tessellated.vertices {
            // Apply transform to each vertex
            let p = self.state.transform().transform_point(Point::new(pos[0], pos[1]));
            self.vertices.push(RectVertex::solid(
                [p.x, p.y],
                color,
                rect_pos,
                rect_size,
                corner_radii,
            ));
        }

        // Add indices (offset by base_index)
        for idx in &tessellated.indices {
            self.indices.push(base_index + idx);
        }

        self.vertex_count += tessellated.vertices.len() as u32;
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        if self.current_blend_mode != mode {
            self.current_blend_mode = mode;
            self.state_changes += 1;

            // If we haven't started drawing yet, update the batch blend mode
            // Otherwise, subsequent draws will use the current blend mode
            // Note: For proper multi-blend-mode support within a frame,
            // we would need to flush and render the current batch before switching.
            // For now, the batch uses the blend mode set before the first draw.
            if self.vertices.is_empty() && self.shadow_vertices.is_empty() {
                self.batch_blend_mode = mode;
            }
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

    fn draw_image(&mut self, image: &Image, dest: Rect, scale_mode: ImageScaleMode) {
        // Calculate source rect (entire image)
        let src = Rect::new(0.0, 0.0, image.width() as f32, image.height() as f32);

        // Calculate actual destination based on scale mode
        let actual_dest = self.calculate_scaled_dest(image.size(), dest, scale_mode);

        self.draw_image_rect(image, src, actual_dest);
    }

    fn draw_image_rect(&mut self, image: &Image, src: Rect, dest: Rect) {
        // Transform the destination rectangle
        let transformed_dest = self.state.transform().transform_rect(&dest);

        // Get UV coordinates from the atlas allocation
        let (u_min, v_min, u_max, v_max) = image.uv_rect();

        // Calculate UV coordinates for the source sub-rectangle
        let img_w = image.width() as f32;
        let img_h = image.height() as f32;

        // Map source rect to UV coordinates within the atlas allocation
        let u_range = u_max - u_min;
        let v_range = v_max - v_min;

        let src_u_min = u_min + (src.left() / img_w) * u_range;
        let src_v_min = v_min + (src.top() / img_h) * v_range;
        let src_u_max = u_min + (src.right() / img_w) * u_range;
        let src_v_max = v_min + (src.bottom() / img_h) * v_range;

        // Apply opacity as tint alpha
        let tint = Color::WHITE.with_alpha(self.current_opacity);

        // Add to appropriate batch
        self.add_image_quad(
            image.atlas(),
            transformed_dest,
            [src_u_min, src_v_min, src_u_max, src_v_max],
            tint,
        );
    }

    fn draw_nine_patch(&mut self, nine_patch: &NinePatch, dest: Rect) {
        let patches = nine_patch.calculate_patches(dest);
        let image = &nine_patch.image;

        let (u_min, v_min, u_max, v_max) = image.uv_rect();
        let img_w = image.width() as f32;
        let img_h = image.height() as f32;

        let u_range = u_max - u_min;
        let v_range = v_max - v_min;

        let tint = Color::WHITE.with_alpha(self.current_opacity);

        for (src, dest) in patches {
            // Skip patches with zero area
            if src.width() <= 0.0 || src.height() <= 0.0 || dest.width() <= 0.0 || dest.height() <= 0.0 {
                continue;
            }

            // Transform destination
            let transformed_dest = self.state.transform().transform_rect(&dest);

            // Calculate UVs for this patch
            let patch_u_min = u_min + (src.left() / img_w) * u_range;
            let patch_v_min = v_min + (src.top() / img_h) * v_range;
            let patch_u_max = u_min + (src.right() / img_w) * u_range;
            let patch_v_max = v_min + (src.bottom() / img_h) * v_range;

            self.add_image_quad(
                image.atlas(),
                transformed_dest,
                [patch_u_min, patch_v_min, patch_u_max, patch_v_max],
                tint,
            );
        }
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

    #[test]
    fn test_image_vertex_size() {
        // ImageVertex: position(2) + uv(2) + tint(4) = 8 floats * 4 = 32 bytes
        assert_eq!(std::mem::size_of::<ImageVertex>(), 32);
    }

    #[test]
    fn test_image_vertex_creation() {
        let vertex = ImageVertex::new([10.0, 20.0], [0.5, 0.5], Color::WHITE);
        assert_eq!(vertex.position, [10.0, 20.0]);
        assert_eq!(vertex.uv, [0.5, 0.5]);
        assert_eq!(vertex.tint, Color::WHITE.to_array());
    }

    #[test]
    fn test_blend_state_for_mode_normal() {
        let state = blend_state_for_mode(BlendMode::Normal);
        assert_eq!(state, wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
    }

    #[test]
    fn test_blend_state_for_mode_source() {
        let state = blend_state_for_mode(BlendMode::Source);
        assert_eq!(state, wgpu::BlendState::REPLACE);
    }

    #[test]
    fn test_blend_state_for_mode_multiply() {
        let state = blend_state_for_mode(BlendMode::Multiply);
        // Multiply uses Dst factor for src_factor
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::Dst);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::Zero);
        assert_eq!(state.color.operation, wgpu::BlendOperation::Add);
    }

    #[test]
    fn test_blend_state_for_mode_screen() {
        let state = blend_state_for_mode(BlendMode::Screen);
        // Screen: src + dst * (1 - src)
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::OneMinusSrc);
        assert_eq!(state.color.operation, wgpu::BlendOperation::Add);
    }

    #[test]
    fn test_blend_state_for_mode_add() {
        let state = blend_state_for_mode(BlendMode::Add);
        // Add: src + dst
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::One);
        assert_eq!(state.color.operation, wgpu::BlendOperation::Add);
    }

    #[test]
    fn test_blend_state_for_mode_darken() {
        let state = blend_state_for_mode(BlendMode::Darken);
        // Darken uses min operation
        assert_eq!(state.color.operation, wgpu::BlendOperation::Min);
    }

    #[test]
    fn test_blend_state_for_mode_lighten() {
        let state = blend_state_for_mode(BlendMode::Lighten);
        // Lighten uses max operation
        assert_eq!(state.color.operation, wgpu::BlendOperation::Max);
    }

    #[test]
    fn test_blend_state_for_mode_overlay_fallback() {
        // Overlay is a complex mode that falls back to Normal
        let state = blend_state_for_mode(BlendMode::Overlay);
        assert_eq!(state, wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
    }
}
