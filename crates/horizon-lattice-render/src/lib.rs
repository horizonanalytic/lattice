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
//!
//! // Render loop
//! if let Some(frame) = surface.get_current_frame()? {
//!     let ctx = GraphicsContext::get();
//!
//!     // Create a command encoder
//!     let mut encoder = ctx.device().create_command_encoder(
//!         &wgpu::CommandEncoderDescriptor { label: Some("render") }
//!     );
//!
//!     // Create a render pass
//!     {
//!         let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//!             label: Some("clear"),
//!             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//!                 view: &frame.view,
//!                 resolve_target: None,
//!                 ops: wgpu::Operations {
//!                     load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
//!                     store: wgpu::StoreOp::Store,
//!                 },
//!             })],
//!             depth_stencil_attachment: None,
//!             timestamp_writes: None,
//!             occlusion_query_set: None,
//!         });
//!     }
//!
//!     // Submit and present
//!     ctx.queue().submit(std::iter::once(encoder.finish()));
//!     frame.present();
//! }
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
mod surface;

pub use context::{GraphicsConfig, GraphicsContext, GpuResources};
pub use error::{RenderError, RenderResult};
pub use surface::{PresentMode, RenderSurface, SurfaceConfig, SurfaceFrame};

// Re-export wgpu types that users commonly need
pub use wgpu;
