//! Layer compositing system for rendering complex UI hierarchies.
//!
//! This module provides [`Layer`] and [`Compositor`] for rendering UI elements
//! to separate textures and compositing them together. This enables:
//!
//! - Group opacity (apply opacity to a group of elements as a whole)
//! - Caching of complex widget subtrees
//! - Blur and other post-processing effects
//! - Efficient scrolling with cached content
//!
//! # Architecture
//!
//! The compositing system follows a hierarchical layer model:
//!
//! 1. Content is rendered to individual [`Layer`] textures
//! 2. Layers are composited together using the [`Compositor`]
//! 3. Final result is presented to the screen
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::{
//!     GraphicsContext, GraphicsConfig, GpuRenderer, Renderer,
//!     Color, Rect, Size,
//! };
//! use horizon_lattice_render::layer::{Layer, LayerConfig, Compositor};
//!
//! // Initialize graphics
//! GraphicsContext::init(GraphicsConfig::default()).unwrap();
//!
//! // Create a compositor
//! let mut compositor = Compositor::new(800, 600).unwrap();
//!
//! // Create a layer for rendering some content
//! let layer = compositor.create_layer(LayerConfig {
//!     width: 200,
//!     height: 200,
//!     opacity: 0.8,
//!     ..Default::default()
//! }).unwrap();
//!
//! // Render content to the layer using the layer's renderer
//! // (in actual usage, you'd draw widgets here)
//! ```

use std::collections::HashMap;

use tracing::debug;

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};
use crate::paint::BlendMode;
use crate::types::{Color, Point, Rect, Size};

/// Returns the wgpu BlendState for the given blend mode when compositing layers.
/// Complex blend modes (Overlay, SoftLight, etc.) that require shader-based
/// blending return Normal blend state with a warning logged.
fn blend_state_for_mode(mode: BlendMode) -> wgpu::BlendState {
    use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState};

    match mode {
        // Normal: src_over blending (premultiplied alpha)
        BlendMode::Normal => BlendState::PREMULTIPLIED_ALPHA_BLENDING,

        // Multiply: dst * src (darkens the image)
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

        // Complex blend modes require fragment shader destination reads.
        // Fall back to Normal blending as these cannot be implemented
        // with wgpu's hardware blend states.
        BlendMode::Overlay
        | BlendMode::ColorDodge
        | BlendMode::ColorBurn
        | BlendMode::HardLight
        | BlendMode::SoftLight
        | BlendMode::Difference
        | BlendMode::Exclusion => {
            tracing::debug!(
                mode = ?mode,
                "Complex blend mode not supported in layer compositing, using Normal"
            );
            BlendState::PREMULTIPLIED_ALPHA_BLENDING
        }
    }
}

/// Create a composite pipeline for a specific blend mode.
fn create_composite_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    blend_mode: BlendMode,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("composite_pipeline_{:?}", blend_mode)),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<CompositeVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, // position
                    1 => Float32x2, // uv
                ],
            }],
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

/// Configuration for creating a layer.
#[derive(Debug, Clone)]
pub struct LayerConfig {
    /// Width of the layer in pixels.
    pub width: u32,
    /// Height of the layer in pixels.
    pub height: u32,
    /// Layer opacity (0.0 = fully transparent, 1.0 = fully opaque).
    pub opacity: f32,
    /// Blend mode for compositing this layer.
    pub blend_mode: BlendMode,
    /// Clear color for the layer (usually transparent).
    pub clear_color: Color,
    /// Position offset when compositing.
    pub position: Point,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            clear_color: Color::TRANSPARENT,
            position: Point::ZERO,
        }
    }
}

impl LayerConfig {
    /// Create a new layer config with the specified dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Set the layer opacity.
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the blend mode.
    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Set the clear color.
    pub fn with_clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    /// Set the position offset.
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Point::new(x, y);
        self
    }
}

/// A unique identifier for a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(u32);

impl LayerId {
    /// Create a new layer ID.
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// A compositing layer that renders to an offscreen texture.
///
/// Layers allow rendering content independently and then compositing
/// it together with other layers. This is useful for:
///
/// - Applying opacity to a group of elements
/// - Caching expensive-to-render content
/// - Implementing effects like blur or drop shadows
pub struct Layer {
    /// Unique identifier for this layer.
    id: LayerId,
    /// The render target texture.
    texture: wgpu::Texture,
    /// View for rendering.
    view: wgpu::TextureView,
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// Texture format.
    format: wgpu::TextureFormat,
    /// Layer opacity.
    opacity: f32,
    /// Blend mode for compositing.
    blend_mode: BlendMode,
    /// Clear color.
    clear_color: Color,
    /// Position offset when compositing.
    position: Point,
    /// Whether the layer content has been invalidated.
    dirty: bool,
    /// Bind group for sampling this layer's texture.
    bind_group: wgpu::BindGroup,
}

impl Layer {
    /// Create a new layer.
    pub(crate) fn new(
        id: LayerId,
        config: &LayerConfig,
        format: wgpu::TextureFormat,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> RenderResult<Self> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        if config.width == 0 || config.height == 0 {
            return Err(RenderError::InvalidDimensions {
                width: config.width,
                height: config.height,
            });
        }

        let texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("layer_{}", id.0)),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler for this layer
        let sampler = ctx.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("layer_{}_sampler", id.0)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for sampling
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("layer_{}_bind_group", id.0)),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        debug!(
            target: "horizon_lattice_render::layer",
            id = id.0,
            width = config.width,
            height = config.height,
            opacity = config.opacity,
            "created layer"
        );

        Ok(Self {
            id,
            texture,
            view,
            width: config.width,
            height: config.height,
            format,
            opacity: config.opacity.clamp(0.0, 1.0),
            blend_mode: config.blend_mode,
            clear_color: config.clear_color,
            position: config.position,
            dirty: true,
            bind_group,
        })
    }

    /// Get the layer ID.
    pub fn id(&self) -> LayerId {
        self.id
    }

    /// Get the texture view for rendering to this layer.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Get the underlying texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Get the texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Get the layer width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the layer height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the layer dimensions as a Size.
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// Get the layer bounds as a Rect (at position).
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.position.x,
            self.position.y,
            self.width as f32,
            self.height as f32,
        )
    }

    /// Get the layer opacity.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Set the layer opacity.
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Get the blend mode.
    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// Set the blend mode.
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
    }

    /// Get the clear color.
    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    /// Set the clear color.
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Get the position offset.
    pub fn position(&self) -> Point {
        self.position
    }

    /// Set the position offset.
    pub fn set_position(&mut self, position: Point) {
        self.position = position;
    }

    /// Check if the layer is dirty (needs re-rendering).
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the layer as dirty (needs re-rendering).
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Mark the layer as clean (has been rendered).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Get the bind group for sampling this layer.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Resize the layer.
    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> RenderResult<()> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidDimensions { width, height });
        }

        if self.width == width && self.height == height {
            return Ok(());
        }

        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        self.texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("layer_{}", self.id.0)),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate sampler and bind group
        let sampler = ctx.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("layer_{}_sampler", self.id.0)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        self.bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("layer_{}_bind_group", self.id.0)),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.width = width;
        self.height = height;
        self.dirty = true;

        debug!(
            target: "horizon_lattice_render::layer",
            id = self.id.0,
            width,
            height,
            "resized layer"
        );

        Ok(())
    }
}

impl std::fmt::Debug for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layer")
            .field("id", &self.id)
            .field("size", &(self.width, self.height))
            .field("opacity", &self.opacity)
            .field("blend_mode", &self.blend_mode)
            .field("position", &self.position)
            .field("dirty", &self.dirty)
            .finish()
    }
}

/// Manages layer creation and compositing.
///
/// The compositor maintains a collection of layers and handles
/// compositing them together into a final output.
pub struct Compositor {
    /// All layers managed by this compositor.
    layers: Vec<Layer>,
    /// Next layer ID.
    next_id: u32,
    /// Output width.
    output_width: u32,
    /// Output height.
    output_height: u32,
    /// Texture format for layers.
    format: wgpu::TextureFormat,
    /// Bind group layout for layer textures.
    layer_bind_group_layout: wgpu::BindGroupLayout,
    /// Compositing shader module.
    composite_shader: wgpu::ShaderModule,
    /// Pipeline layout for compositing.
    pipeline_layout: wgpu::PipelineLayout,
    /// Compositing pipeline cache by blend mode.
    composite_pipelines: HashMap<BlendMode, wgpu::RenderPipeline>,
    /// Uniform buffer for compositing.
    uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms.
    uniform_bind_group: wgpu::BindGroup,
    /// Vertex buffer for fullscreen quad.
    vertex_buffer: wgpu::Buffer,
}

/// Vertex for compositing shader.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

/// Uniforms for compositing shader.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeUniforms {
    /// Output viewport size.
    viewport_size: [f32; 2],
    /// Layer position offset.
    layer_offset: [f32; 2],
    /// Layer size.
    layer_size: [f32; 2],
    /// Layer opacity.
    opacity: f32,
    /// Padding for alignment.
    _padding: f32,
}

impl Compositor {
    /// Create a new compositor with the specified output dimensions.
    pub fn new(output_width: u32, output_height: u32) -> RenderResult<Self> {
        Self::new_with_format(output_width, output_height, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Create a new compositor with the specified output dimensions and format.
    pub fn new_with_format(
        output_width: u32,
        output_height: u32,
        format: wgpu::TextureFormat,
    ) -> RenderResult<Self> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;
        let device = ctx.device();

        // Create bind group layout for layer textures
        let layer_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layer_bind_group_layout"),
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

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compositor_uniform_buffer"),
            size: std::mem::size_of::<CompositeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform bind group layout
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compositor_uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            label: Some("compositor_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create compositing shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/composite.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compositor_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &layer_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create initial compositing pipeline for Normal blend mode
        let normal_pipeline = create_composite_pipeline(device, &shader, &pipeline_layout, format, BlendMode::Normal);
        let mut composite_pipelines = HashMap::new();
        composite_pipelines.insert(BlendMode::Normal, normal_pipeline);

        // Create vertex buffer for fullscreen quad
        let vertices = [
            CompositeVertex { position: [0.0, 0.0], uv: [0.0, 0.0] },
            CompositeVertex { position: [1.0, 0.0], uv: [1.0, 0.0] },
            CompositeVertex { position: [1.0, 1.0], uv: [1.0, 1.0] },
            CompositeVertex { position: [0.0, 0.0], uv: [0.0, 0.0] },
            CompositeVertex { position: [1.0, 1.0], uv: [1.0, 1.0] },
            CompositeVertex { position: [0.0, 1.0], uv: [0.0, 1.0] },
        ];

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compositor_vertex_buffer"),
            size: (std::mem::size_of::<CompositeVertex>() * 6) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        ctx.queue().write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        debug!(
            target: "horizon_lattice_render::layer",
            output_width,
            output_height,
            "created compositor"
        );

        Ok(Self {
            layers: Vec::new(),
            next_id: 0,
            output_width,
            output_height,
            format,
            layer_bind_group_layout,
            composite_shader: shader,
            pipeline_layout,
            composite_pipelines,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
        })
    }

    /// Create a new layer.
    pub fn create_layer(&mut self, config: LayerConfig) -> RenderResult<LayerId> {
        let id = LayerId::new(self.next_id);
        self.next_id += 1;

        let layer = Layer::new(id, &config, self.format, &self.layer_bind_group_layout)?;
        self.layers.push(layer);

        Ok(id)
    }

    /// Get a reference to a layer by ID.
    pub fn get_layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Get a mutable reference to a layer by ID.
    pub fn get_layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Remove a layer by ID.
    pub fn remove_layer(&mut self, id: LayerId) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            self.layers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all layers.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Get all layers mutably.
    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    /// Get the number of layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get the output dimensions.
    pub fn output_size(&self) -> (u32, u32) {
        (self.output_width, self.output_height)
    }

    /// Resize the output.
    pub fn resize_output(&mut self, width: u32, height: u32) {
        self.output_width = width;
        self.output_height = height;
    }

    /// Get the texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Get the bind group layout for layer textures.
    pub fn layer_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layer_bind_group_layout
    }

    /// Get or create a composite pipeline for the given blend mode.
    fn get_or_create_pipeline(&mut self, blend_mode: BlendMode) -> &wgpu::RenderPipeline {
        if !self.composite_pipelines.contains_key(&blend_mode) {
            let ctx = GraphicsContext::get();
            let pipeline = create_composite_pipeline(
                ctx.device(),
                &self.composite_shader,
                &self.pipeline_layout,
                self.format,
                blend_mode,
            );
            self.composite_pipelines.insert(blend_mode, pipeline);
            debug!(
                target: "horizon_lattice_render::layer",
                ?blend_mode,
                "created composite pipeline for blend mode"
            );
        }
        self.composite_pipelines.get(&blend_mode).unwrap()
    }

    /// Composite all layers to the target view.
    ///
    /// Layers are composited in order (first layer is at the bottom).
    /// Each layer uses its configured blend mode for compositing.
    pub fn composite_to(
        &mut self,
        target_view: &wgpu::TextureView,
        clear_color: Color,
    ) -> RenderResult<()> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;
        let device = ctx.device();
        let queue = ctx.queue();

        // Ensure all needed blend mode pipelines exist
        let blend_modes: Vec<BlendMode> = self.layers.iter()
            .filter(|l| l.opacity > 0.0)
            .map(|l| l.blend_mode)
            .collect();
        for blend_mode in blend_modes {
            self.get_or_create_pipeline(blend_mode);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compositor_encoder"),
        });

        // Begin render pass with clear
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compositor_clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            let mut current_blend_mode: Option<BlendMode> = None;

            // Composite each layer
            for layer in &self.layers {
                if layer.opacity <= 0.0 {
                    continue;
                }

                // Set pipeline if blend mode changed
                if current_blend_mode != Some(layer.blend_mode) {
                    let pipeline = self.composite_pipelines.get(&layer.blend_mode).unwrap();
                    render_pass.set_pipeline(pipeline);
                    current_blend_mode = Some(layer.blend_mode);
                }

                // Update uniforms for this layer
                let uniforms = CompositeUniforms {
                    viewport_size: [self.output_width as f32, self.output_height as f32],
                    layer_offset: [layer.position.x, layer.position.y],
                    layer_size: [layer.width as f32, layer.height as f32],
                    opacity: layer.opacity,
                    _padding: 0.0,
                };
                queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

                render_pass.set_bind_group(1, &layer.bind_group, &[]);
                render_pass.draw(0..6, 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// Composite layers to the target view without clearing.
    ///
    /// This is useful when you want to composite layers onto existing content.
    /// Each layer uses its configured blend mode for compositing.
    pub fn composite_over(
        &mut self,
        target_view: &wgpu::TextureView,
    ) -> RenderResult<()> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;
        let device = ctx.device();
        let queue = ctx.queue();

        // Ensure all needed blend mode pipelines exist
        let blend_modes: Vec<BlendMode> = self.layers.iter()
            .filter(|l| l.opacity > 0.0)
            .map(|l| l.blend_mode)
            .collect();
        for blend_mode in blend_modes {
            self.get_or_create_pipeline(blend_mode);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compositor_over_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compositor_over_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            let mut current_blend_mode: Option<BlendMode> = None;

            for layer in &self.layers {
                if layer.opacity <= 0.0 {
                    continue;
                }

                // Set pipeline if blend mode changed
                if current_blend_mode != Some(layer.blend_mode) {
                    let pipeline = self.composite_pipelines.get(&layer.blend_mode).unwrap();
                    render_pass.set_pipeline(pipeline);
                    current_blend_mode = Some(layer.blend_mode);
                }

                let uniforms = CompositeUniforms {
                    viewport_size: [self.output_width as f32, self.output_height as f32],
                    layer_offset: [layer.position.x, layer.position.y],
                    layer_size: [layer.width as f32, layer.height as f32],
                    opacity: layer.opacity,
                    _padding: 0.0,
                };
                queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

                render_pass.set_bind_group(1, &layer.bind_group, &[]);
                render_pass.draw(0..6, 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

impl std::fmt::Debug for Compositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compositor")
            .field("output_size", &(self.output_width, self.output_height))
            .field("layer_count", &self.layers.len())
            .field("format", &self.format)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_config_default() {
        let config = LayerConfig::default();
        assert_eq!(config.width, 256);
        assert_eq!(config.height, 256);
        assert_eq!(config.opacity, 1.0);
        assert_eq!(config.blend_mode, BlendMode::Normal);
    }

    #[test]
    fn test_layer_config_builder() {
        let config = LayerConfig::new(512, 512)
            .with_opacity(0.5)
            .with_position(100.0, 200.0);

        assert_eq!(config.width, 512);
        assert_eq!(config.height, 512);
        assert_eq!(config.opacity, 0.5);
        assert_eq!(config.position.x, 100.0);
        assert_eq!(config.position.y, 200.0);
    }

    #[test]
    fn test_layer_id() {
        let id1 = LayerId::new(0);
        let id2 = LayerId::new(1);

        assert_ne!(id1, id2);
        assert_eq!(id1.id(), 0);
        assert_eq!(id2.id(), 1);
    }
}
