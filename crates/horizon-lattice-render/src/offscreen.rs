//! Offscreen rendering surfaces for headless rendering and testing.
//!
//! This module provides [`OffscreenSurface`] for rendering without a window,
//! useful for testing, server-side rendering, and screenshot capture.

use tracing::{debug, info};

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};

/// Configuration for offscreen surface creation.
#[derive(Debug, Clone)]
pub struct OffscreenConfig {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Texture format. Defaults to Rgba8UnormSrgb for PNG-compatible output.
    pub format: Option<wgpu::TextureFormat>,
}

impl OffscreenConfig {
    /// Create a new offscreen config with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: None,
        }
    }

    /// Set a specific texture format.
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = Some(format);
        self
    }
}

impl Default for OffscreenConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            format: None,
        }
    }
}

/// An offscreen render surface for headless rendering.
///
/// Unlike [`RenderSurface`](crate::RenderSurface) which renders to a window,
/// `OffscreenSurface` renders to a GPU texture. This is useful for:
///
/// - Unit testing renderers without a display
/// - Server-side rendering
/// - Screenshot capture
/// - Render-to-texture for post-processing
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::{
///     GraphicsContext, GraphicsConfig, OffscreenSurface, OffscreenConfig,
///     GpuRenderer, Renderer, Color, Rect, Size,
/// };
///
/// // Initialize graphics context
/// GraphicsContext::init(GraphicsConfig::default()).unwrap();
///
/// // Create an offscreen surface (no window needed)
/// let mut surface = OffscreenSurface::new(OffscreenConfig::new(800, 600)).unwrap();
///
/// // Create a renderer and draw
/// let mut renderer = GpuRenderer::new_offscreen(&surface).unwrap();
/// renderer.begin_frame(Color::WHITE, Size::new(800.0, 600.0));
/// renderer.fill_rect(Rect::new(10.0, 10.0, 100.0, 50.0), Color::RED);
/// renderer.end_frame();
/// renderer.render_to_offscreen(&mut surface).unwrap();
///
/// // Export to image
/// let pixels = surface.read_pixels().unwrap();
/// ```
pub struct OffscreenSurface {
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
}

impl OffscreenSurface {
    /// Create a new offscreen surface.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration specifying dimensions and format.
    ///
    /// # Errors
    ///
    /// Returns an error if the graphics context has not been initialized
    /// or if dimensions are invalid.
    pub fn new(config: OffscreenConfig) -> RenderResult<Self> {
        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        if config.width == 0 || config.height == 0 {
            return Err(RenderError::InvalidDimensions {
                width: config.width,
                height: config.height,
            });
        }

        // Default to Rgba8UnormSrgb for PNG-compatible output
        let format = config.format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);

        let texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_render_target"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // RENDER_ATTACHMENT for rendering, COPY_SRC for readback
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        info!(
            target: "horizon_lattice_render::offscreen",
            width = config.width,
            height = config.height,
            format = ?format,
            "created offscreen surface"
        );

        Ok(Self {
            texture,
            view,
            width: config.width,
            height: config.height,
            format,
        })
    }

    /// Resize the offscreen surface.
    ///
    /// This recreates the underlying texture with new dimensions.
    pub fn resize(&mut self, width: u32, height: u32) -> RenderResult<()> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidDimensions { width, height });
        }

        if self.width == width && self.height == height {
            return Ok(());
        }

        let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;

        self.texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.width = width;
        self.height = height;

        debug!(
            target: "horizon_lattice_render::offscreen",
            width,
            height,
            "resized offscreen surface"
        );

        Ok(())
    }

    /// Get the texture view for rendering.
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

    /// Get the surface width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the surface height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the surface dimensions as (width, height).
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Read the rendered pixels back from the GPU.
    ///
    /// Returns RGBA pixel data as a flat Vec<u8> in row-major order.
    /// Each pixel is 4 bytes (R, G, B, A).
    ///
    /// # Note
    ///
    /// This operation is synchronous and will block until the GPU finishes.
    /// For better performance with many captures, consider using the
    /// async [`capture`](crate::capture) module directly.
    pub fn read_pixels(&self) -> RenderResult<Vec<u8>> {
        crate::capture::read_texture_pixels(&self.texture, self.width, self.height)
    }

    /// Save the rendered content to an image file.
    ///
    /// Supported formats: PNG, JPEG, BMP, etc. (determined by file extension).
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path. Extension determines format.
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> RenderResult<()> {
        crate::capture::save_texture_to_file(&self.texture, self.width, self.height, path)
    }
}

impl std::fmt::Debug for OffscreenSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OffscreenSurface")
            .field("size", &(self.width, self.height))
            .field("format", &self.format)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offscreen_config_default() {
        let config = OffscreenConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.format.is_none());
    }

    #[test]
    fn test_offscreen_config_new() {
        let config = OffscreenConfig::new(1920, 1080);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
    }

    #[test]
    fn test_offscreen_config_with_format() {
        let config = OffscreenConfig::new(100, 100).with_format(wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(config.format, Some(wgpu::TextureFormat::Bgra8Unorm));
    }
}
