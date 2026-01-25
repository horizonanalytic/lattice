//! Graphics rendering backend for Horizon Lattice.
//!
//! This crate provides the GPU-accelerated rendering layer built on wgpu.
//! It handles surface management, rendering primitives, and GPU resource management.
//!
//! # Getting Started
//!
//! Before any rendering can occur, you must initialize the [`GraphicsContext`]:
//!
//! ```no_run
//! use horizon_lattice_render::{GraphicsContext, GraphicsConfig};
//!
//! // Initialize with default configuration
//! let ctx = GraphicsContext::init(GraphicsConfig::default())
//!     .expect("Failed to initialize graphics");
//!
//! // Get adapter information
//! let info = ctx.adapter_info();
//! println!("Using GPU: {} ({:?})", info.name, info.backend);
//! ```
//!
//! # Creating Render Surfaces
//!
//! Each window needs a [`RenderSurface`] to render to:
//!
//! ```no_run
//! use std::sync::Arc;
//! use horizon_lattice_render::{GraphicsContext, GraphicsConfig, RenderSurface, SurfaceConfig};
//! use winit::event_loop::ActiveEventLoop;
//! use winit::window::Window;
//!
//! # fn example(event_loop: &ActiveEventLoop) -> horizon_lattice_render::RenderResult<()> {
//! // Initialize graphics first
//! GraphicsContext::init(GraphicsConfig::default())?;
//!
//! // Create a window (must be Arc for surface lifetime)
//! let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
//!
//! // Create a render surface
//! let mut surface = RenderSurface::new(window, SurfaceConfig::default())?;
//! # Ok(())
//! # }
//! ```
//!
//! # Using the Renderer
//!
//! The [`GpuRenderer`] provides a high-level 2D drawing API:
//!
//! ```no_run
//! use horizon_lattice_render::{
//!     GraphicsContext, GraphicsConfig, RenderSurface, SurfaceConfig,
//!     GpuRenderer, Renderer, Color, Rect, Size,
//! };
//! use std::sync::Arc;
//! use winit::window::Window;
//!
//! # fn example(window: Arc<Window>) -> horizon_lattice_render::RenderResult<()> {
//! GraphicsContext::init(GraphicsConfig::default())?;
//! let mut surface = RenderSurface::new(window, SurfaceConfig::default())?;
//! let mut renderer = GpuRenderer::new(&surface)?;
//!
//! // Begin a frame
//! renderer.begin_frame(Color::WHITE, Size::new(800.0, 600.0));
//!
//! // Draw some shapes
//! renderer.fill_rect(Rect::new(10.0, 10.0, 100.0, 50.0), Color::RED);
//!
//! renderer.save();
//! renderer.translate(200.0, 100.0);
//! renderer.fill_rect(Rect::new(0.0, 0.0, 80.0, 80.0), Color::BLUE);
//! renderer.restore();
//!
//! // End frame and render to surface
//! renderer.end_frame();
//! renderer.render_to_surface(&mut surface)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Handling Window Events
//!
//! The surface needs to be resized when the window is resized:
//!
//! ```no_run
//! # use horizon_lattice_render::RenderSurface;
//! # fn example(surface: &mut RenderSurface, width: u32, height: u32) {
//! // In your window event handler:
//! // WindowEvent::Resized(size) => {
//! surface.resize(width, height).ok();
//! // }
//! # }
//! ```

mod async_image;
mod atlas;
mod context;
pub mod damage;
mod error;
mod gpu_renderer;
mod embedded_icon;
mod icon;
mod image;
mod image_buffer;
pub mod image_data;
pub mod layer;
mod paint;
mod path;
mod renderer;
mod scalable_image;
pub mod stencil;
mod svg;
mod surface;
pub mod text;
mod text_render_pass;
mod text_renderer;
mod transform;
mod types;

pub mod capture;
mod offscreen;

// Shader hot-reload support (optional)
#[cfg(feature = "shader-hot-reload")]
mod shader_watcher;

// Core infrastructure
pub use context::{GraphicsConfig, GraphicsContext, GpuResources};
pub use error::{RenderError, RenderResult};
pub use offscreen::{OffscreenConfig, OffscreenSurface};
pub use surface::{PresentMode, RenderSurface, SurfaceConfig, SurfaceFrame};

// Renderer API
pub use gpu_renderer::GpuRenderer;
pub use renderer::{FrameStats, RenderState, RenderStateStack, Renderer};

// Drawing types
pub use paint::{
    BlendMode, BoxShadow, BoxShadowParams, DashPattern, FillRule, GradientStop, LineCap, LineJoin,
    LinearGradient, Paint, RadialGradient, Stroke,
};
pub use path::{tessellate_fill, tessellate_stroke, TessellatedPath, DEFAULT_TOLERANCE};
pub use transform::{Transform2D, TransformStack};
pub use types::{Color, CornerRadii, Path, PathCommand, Point, Rect, RoundedRect, Size};

// Image types
pub use async_image::{AsyncImageHandle, AsyncImageLoader, AsyncImageLoaderConfig, LoadingState};
pub use atlas::{ImageManager, TextureAtlas};
pub use image::{Image, ImageLoader, ImageScaleMode, NinePatch};
pub use image_buffer::{ImageBlendMode, ImageBuffer, OutputFormat, ResizeFilter};
pub use scalable_image::ScalableImage;
pub use svg::SvgImage;

// Icon types
pub use icon::{
    icon_tint_for_state, icon_tint_for_state_full, icon_tint_for_state_with_hover, Icon, IconMode,
    IconPosition, IconSize, IconSource, IconState, IconThemeMode, SizedIconSet, StatefulIconSet,
    ThemedIconSet,
};

// Embedded icon support
pub use embedded_icon::{EmbeddedIconData, EmbeddedIconSet, ImageFormat};

// Image metadata
pub use image_data::{
    read_dimensions, read_dimensions_from_bytes, read_metadata, read_metadata_from_bytes,
    ColorType, ExifData, ImageMetadata, Orientation,
};

// Damage tracking
pub use damage::DamageTracker;

// Layer compositing
pub use layer::{Compositor, Layer, LayerConfig, LayerId};

// Stencil clipping
pub use stencil::{ClipShape, ClipStack};

// Text rendering
pub use text::{
    Font, FontBuilder, FontFaceId, FontFaceInfo, FontFamily, FontFeature, FontLoadError,
    FontMetrics, FontQuery, FontStretch, FontStyle, FontSystem, FontSystemConfig, FontWeight,
    // Text shaping
    GlyphCacheKey, GlyphId, ShapedGlyph, ShapedText, ShapingOptions, TextShaper,
    // Text layout
    BackgroundRect, DecorationLine, HorizontalAlign, LayoutGlyph, LayoutLine, TextLayout,
    TextLayoutOptions, TextSpan, VerticalAlign, WrapMode,
    // Rich text
    RichText, RichTextSpan,
    // Text decoration
    TextDecoration, TextDecorationStyle, TextDecorationType,
    // Glyph rendering
    GlyphAllocation, GlyphAtlas, GlyphAtlasStats, GlyphCache, GlyphCacheStats, GlyphPixelFormat,
    GlyphRenderMode, RasterizedGlyph,
};

// Text renderer
pub use text_render_pass::TextRenderPass;
pub use text_renderer::{PreparedGlyph, TextRenderer, TextRendererConfig, TextRendererStats};

// Shader hot-reload support
#[cfg(feature = "shader-hot-reload")]
pub use shader_watcher::{load_shader_source, ShaderKind, ShaderReloadResult, ShaderWatcher};

// Re-export wgpu types that users commonly need
pub use wgpu;
