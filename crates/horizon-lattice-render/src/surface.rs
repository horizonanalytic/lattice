//! Window surface management for rendering.
//!
//! This module provides the [`RenderSurface`] type which manages a wgpu surface
//! attached to a window, handling configuration and resize events.

use std::sync::Arc;

use tracing::{debug, info, trace, warn};
use winit::window::Window;

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};

/// Configuration options for surface creation.
#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    /// Preferred present mode (VSync behavior).
    pub present_mode: PresentMode,
    /// Preferred texture format. If None, uses the surface's preferred format.
    pub format: Option<wgpu::TextureFormat>,
    /// Alpha compositing mode. If None, uses the first supported mode.
    pub alpha_mode: Option<wgpu::CompositeAlphaMode>,
    /// Maximum number of frames that can be queued for presentation.
    pub max_frame_latency: u32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            present_mode: PresentMode::AutoVsync,
            format: None,
            alpha_mode: None,
            max_frame_latency: 2,
        }
    }
}

/// Present mode (VSync behavior) preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentMode {
    /// VSync enabled, automatically falls back if unsupported.
    #[default]
    AutoVsync,
    /// VSync disabled, automatically falls back if unsupported.
    AutoNoVsync,
    /// Guaranteed VSync - waits for vertical blank.
    Fifo,
    /// Low-latency VSync - uses mailbox if available, falls back to Fifo.
    LowLatencyVsync,
    /// Immediate presentation (may tear).
    Immediate,
}

impl PresentMode {
    /// Convert to wgpu PresentMode, using capabilities to select the best option.
    fn to_wgpu(self, capabilities: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
        match self {
            PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
            PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
            PresentMode::Fifo => wgpu::PresentMode::Fifo,
            PresentMode::LowLatencyVsync => {
                if capabilities
                    .present_modes
                    .contains(&wgpu::PresentMode::Mailbox)
                {
                    wgpu::PresentMode::Mailbox
                } else {
                    wgpu::PresentMode::Fifo
                }
            }
            PresentMode::Immediate => {
                if capabilities
                    .present_modes
                    .contains(&wgpu::PresentMode::Immediate)
                {
                    wgpu::PresentMode::Immediate
                } else {
                    warn!(
                        target: "horizon_lattice_render::surface",
                        "Immediate present mode not supported, falling back to Fifo"
                    );
                    wgpu::PresentMode::Fifo
                }
            }
        }
    }
}

/// A render surface attached to a window.
///
/// This manages the wgpu surface lifecycle including configuration and resizing.
/// Use [`RenderSurface::new`] to create a surface for a window.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use horizon_lattice_render::{GraphicsContext, GraphicsConfig, RenderSurface, SurfaceConfig};
/// use winit::window::Window;
///
/// # fn example(window: Arc<Window>) -> horizon_lattice_render::RenderResult<()> {
/// // Initialize graphics context first
/// GraphicsContext::init(GraphicsConfig::default())?;
///
/// // Create a render surface for the window
/// let mut surface = RenderSurface::new(window, SurfaceConfig::default())?;
///
/// // Later, when the window is resized:
/// surface.resize(800, 600)?;
///
/// // Render a frame:
/// if let Some(frame) = surface.get_current_frame()? {
///     // ... render to frame.view ...
///     frame.present();
/// }
/// # Ok(())
/// # }
/// ```
pub struct RenderSurface {
    /// The window this surface is attached to (kept alive via Arc).
    window: Arc<Window>,
    /// The underlying wgpu surface.
    surface: wgpu::Surface<'static>,
    /// Current surface configuration.
    config: wgpu::SurfaceConfiguration,
    /// Whether the surface has been configured at least once.
    is_configured: bool,
    /// Surface capabilities from the adapter.
    capabilities: wgpu::SurfaceCapabilities,
}

impl RenderSurface {
    /// Create a new render surface for the given window.
    ///
    /// # Arguments
    ///
    /// * `window` - The window to create the surface for. Must be wrapped in `Arc`.
    /// * `config` - Configuration options for the surface.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The graphics context has not been initialized
    /// - Surface creation fails
    pub fn new(window: Arc<Window>, config: SurfaceConfig) -> RenderResult<Self> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        // Create the surface using the Arc<Window> for 'static lifetime
        let surface = ctx.instance().create_surface(Arc::clone(&window))?;

        // Get surface capabilities
        let capabilities = surface.get_capabilities(ctx.adapter());

        // Select the best texture format
        let format = config.format.unwrap_or_else(|| {
            // Prefer sRGB formats for correct color representation
            capabilities
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(capabilities.formats[0])
        });

        // Select alpha mode
        let alpha_mode = config
            .alpha_mode
            .filter(|m| capabilities.alpha_modes.contains(m))
            .unwrap_or(capabilities.alpha_modes[0]);

        // Select present mode
        let present_mode = config.present_mode.to_wgpu(&capabilities);

        // Get initial window size
        let size = window.inner_size();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: config.max_frame_latency,
        };

        info!(
            target: "horizon_lattice_render::surface",
            format = ?format,
            present_mode = ?present_mode,
            alpha_mode = ?alpha_mode,
            width = size.width,
            height = size.height,
            "created render surface"
        );

        Ok(Self {
            window,
            surface,
            config: surface_config,
            is_configured: false,
            capabilities,
        })
    }

    /// Configure the surface for rendering.
    ///
    /// This must be called before the first render and after any resize.
    /// It's automatically called by [`resize`](Self::resize) and
    /// [`get_current_frame`](Self::get_current_frame).
    pub fn configure(&mut self) -> RenderResult<()> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        if self.config.width == 0 || self.config.height == 0 {
            return Err(RenderError::InvalidDimensions {
                width: self.config.width,
                height: self.config.height,
            });
        }

        self.surface.configure(ctx.device(), &self.config);
        self.is_configured = true;

        trace!(
            target: "horizon_lattice_render::surface",
            width = self.config.width,
            height = self.config.height,
            "surface configured"
        );

        Ok(())
    }

    /// Resize the surface to new dimensions.
    ///
    /// This should be called when the window is resized. Zero dimensions
    /// are handled gracefully by skipping the resize (surface remains valid
    /// but won't render).
    ///
    /// # Arguments
    ///
    /// * `width` - New width in physical pixels
    /// * `height` - New height in physical pixels
    pub fn resize(&mut self, width: u32, height: u32) -> RenderResult<()> {
        // Skip zero-sized resizes (window minimized, etc.)
        if width == 0 || height == 0 {
            debug!(
                target: "horizon_lattice_render::surface",
                width,
                height,
                "skipping zero-sized resize"
            );
            return Ok(());
        }

        // Skip if dimensions haven't changed
        if self.config.width == width && self.config.height == height {
            return Ok(());
        }

        self.config.width = width;
        self.config.height = height;

        debug!(
            target: "horizon_lattice_render::surface",
            width,
            height,
            "resizing surface"
        );

        self.configure()
    }

    /// Get the current frame for rendering.
    ///
    /// Returns a [`SurfaceFrame`] that can be used for rendering. The frame
    /// is automatically presented when dropped, or you can call
    /// [`present`](SurfaceFrame::present) explicitly.
    ///
    /// Returns `Ok(None)` if the surface has zero dimensions (e.g., window minimized).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Getting the surface texture fails (lost, outdated, timeout, OOM)
    pub fn get_current_frame(&mut self) -> RenderResult<Option<SurfaceFrame>> {
        // Ensure surface is configured
        if !self.is_configured {
            self.configure()?;
        }

        // Skip rendering if surface has zero dimensions
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(None);
        }

        match self.surface.get_current_texture() {
            Ok(texture) => {
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                Ok(Some(SurfaceFrame { texture, view }))
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Reconfigure and try again
                debug!(
                    target: "horizon_lattice_render::surface",
                    "surface lost or outdated, reconfiguring"
                );
                self.configure()?;
                let texture = self.surface.get_current_texture()?;
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                Ok(Some(SurfaceFrame { texture, view }))
            }
            Err(wgpu::SurfaceError::Timeout) => {
                warn!(
                    target: "horizon_lattice_render::surface",
                    "surface timeout"
                );
                Err(RenderError::Surface(wgpu::SurfaceError::Timeout))
            }
            Err(e) => Err(RenderError::Surface(e)),
        }
    }

    /// Get the surface texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Get the current surface dimensions.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Get the current surface width.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Get the current surface height.
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// Get the present mode currently in use.
    pub fn present_mode(&self) -> wgpu::PresentMode {
        self.config.present_mode
    }

    /// Get the alpha compositing mode.
    pub fn alpha_mode(&self) -> wgpu::CompositeAlphaMode {
        self.config.alpha_mode
    }

    /// Check if the surface has been configured.
    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    /// Get the surface capabilities.
    pub fn capabilities(&self) -> &wgpu::SurfaceCapabilities {
        &self.capabilities
    }

    /// Get a reference to the window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get the Arc to the window.
    pub fn window_arc(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }

    /// Request a redraw for the window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Update the present mode.
    ///
    /// Takes effect on the next [`configure`](Self::configure) call.
    pub fn set_present_mode(&mut self, mode: PresentMode) {
        let wgpu_mode = mode.to_wgpu(&self.capabilities);
        if self.config.present_mode != wgpu_mode {
            self.config.present_mode = wgpu_mode;
            self.is_configured = false; // Force reconfigure
        }
    }
}

impl std::fmt::Debug for RenderSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderSurface")
            .field("size", &(self.config.width, self.config.height))
            .field("format", &self.config.format)
            .field("present_mode", &self.config.present_mode)
            .field("is_configured", &self.is_configured)
            .finish()
    }
}

/// A frame ready for rendering.
///
/// This wraps a surface texture and its view. The texture is presented
/// when this struct is dropped, or you can call [`present`](Self::present)
/// explicitly for more control.
pub struct SurfaceFrame {
    /// The surface texture to render to.
    texture: wgpu::SurfaceTexture,
    /// A view of the texture for use in render passes.
    pub view: wgpu::TextureView,
}

impl SurfaceFrame {
    /// Present the frame to the screen.
    ///
    /// This consumes the frame. If you don't call this explicitly,
    /// the frame will be presented when dropped.
    pub fn present(self) {
        self.texture.present();
    }

    /// Get the underlying surface texture.
    pub fn texture(&self) -> &wgpu::SurfaceTexture {
        &self.texture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_config_default() {
        let config = SurfaceConfig::default();
        assert_eq!(config.present_mode, PresentMode::AutoVsync);
        assert!(config.format.is_none());
        assert!(config.alpha_mode.is_none());
        assert_eq!(config.max_frame_latency, 2);
    }

    #[test]
    fn test_present_mode_default() {
        assert_eq!(PresentMode::default(), PresentMode::AutoVsync);
    }
}
