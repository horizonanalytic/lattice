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

mod context;
mod error;
mod gpu_renderer;
mod paint;
mod renderer;
mod surface;
mod transform;
mod types;

// Core infrastructure
pub use context::{GraphicsConfig, GraphicsContext, GpuResources};
pub use error::{RenderError, RenderResult};
pub use surface::{PresentMode, RenderSurface, SurfaceConfig, SurfaceFrame};

// Renderer API
pub use gpu_renderer::GpuRenderer;
pub use renderer::{FrameStats, RenderState, RenderStateStack, Renderer};

// Drawing types
pub use paint::{
    BlendMode, DashPattern, GradientStop, LineCap, LineJoin, LinearGradient, Paint,
    RadialGradient, Stroke,
};
pub use transform::{Transform2D, TransformStack};
pub use types::{Color, CornerRadii, Point, Rect, RoundedRect, Size};

// Re-export wgpu types that users commonly need
pub use wgpu;
