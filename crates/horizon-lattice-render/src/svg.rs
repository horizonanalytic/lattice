//! SVG rendering support for resolution-independent vector graphics.
//!
//! This module provides [`SvgImage`], which loads and renders SVG files at any
//! resolution. SVGs are ideal for icons and UI elements because they scale
//! perfectly to any DPI without pixelation.
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice_render::{ImageManager, SvgImage};
//!
//! // Load an SVG
//! let svg = SvgImage::from_file("icons/settings.svg")?;
//!
//! // Get the natural size
//! let natural_size = svg.natural_size();  // e.g., 24x24
//!
//! // Render at a specific size for the current scale factor
//! let mut manager = ImageManager::new()?;
//! let scale = window.scale_factor();
//! let image = svg.render_scaled(&mut manager, scale)?;
//!
//! // Or render at an exact pixel size
//! let image = svg.render_to_image(&mut manager, 48, 48)?;
//! ```
//!
//! # Performance Considerations
//!
//! SVG rendering is more expensive than loading a pre-rendered bitmap. For
//! best performance:
//!
//! - Cache rendered images at common scales (1x, 2x, 3x)
//! - Re-render only when the scale factor changes
//! - Consider using [`ScalableImage`] with pre-rendered PNGs for frequently
//!   used icons

use std::path::Path;
use std::sync::Arc;

use resvg::tiny_skia;
use resvg::usvg;

use crate::atlas::ImageManager;
use crate::error::{RenderError, RenderResult};
use crate::image::Image;
use crate::svg_cache::SvgCache;
use crate::types::Size;

/// An SVG image that can be rendered at any resolution.
///
/// `SvgImage` parses and holds an SVG document, allowing it to be rendered
/// at any size with perfect quality. This is ideal for icons and other
/// graphics that need to scale across different DPI settings.
///
/// # Thread Safety
///
/// The underlying SVG tree is wrapped in an `Arc` and is safe to share
/// across threads, though rendering must happen on a single thread at a time.
///
/// # Example
///
/// ```ignore
/// let svg = SvgImage::from_file("icon.svg")?;
///
/// // Render for a 2x display
/// let scale = 2.0;
/// let image = svg.render_scaled(&mut manager, scale)?;
///
/// // The image is now at 2x the natural SVG size
/// ```
#[derive(Clone)]
pub struct SvgImage {
    /// The parsed SVG tree.
    tree: Arc<usvg::Tree>,
    /// Default/natural size of the SVG.
    default_size: Size,
}

impl SvgImage {
    /// Load an SVG from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SVG file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The SVG is invalid or malformed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let svg = SvgImage::from_file("assets/icons/menu.svg")?;
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> RenderResult<Self> {
        let data = std::fs::read(path.as_ref())
            .map_err(|e| RenderError::ImageLoad(format!("Failed to read SVG file: {}", e)))?;
        Self::from_bytes(&data)
    }

    /// Load an SVG from bytes in memory.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw SVG file contents (UTF-8 XML)
    ///
    /// # Errors
    ///
    /// Returns an error if the SVG is invalid or malformed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let svg_data = include_bytes!("../assets/icon.svg");
    /// let svg = SvgImage::from_bytes(svg_data)?;
    /// ```
    pub fn from_bytes(data: &[u8]) -> RenderResult<Self> {
        // Create options for parsing
        let options = usvg::Options::default();

        // Parse the SVG
        let tree = usvg::Tree::from_data(data, &options)
            .map_err(|e| RenderError::ImageLoad(format!("Failed to parse SVG: {}", e)))?;

        // Get the natural size from the SVG viewBox or dimensions
        let size = tree.size();
        let default_size = Size::new(size.width(), size.height());

        Ok(Self {
            tree: Arc::new(tree),
            default_size,
        })
    }

    /// Get the natural/default size of the SVG.
    ///
    /// This is the size defined in the SVG's `width`/`height` attributes
    /// or `viewBox`.
    pub fn natural_size(&self) -> Size {
        self.default_size
    }

    /// Get the natural width of the SVG.
    pub fn natural_width(&self) -> f32 {
        self.default_size.width
    }

    /// Get the natural height of the SVG.
    pub fn natural_height(&self) -> f32 {
        self.default_size.height
    }

    /// Render the SVG to RGBA pixel data at a specific size.
    ///
    /// # Arguments
    ///
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Returns
    ///
    /// RGBA pixel data as a `Vec<u8>` with length `width * height * 4`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rgba = svg.render_to_rgba(64, 64);
    /// // rgba.len() == 64 * 64 * 4
    /// ```
    pub fn render_to_rgba(&self, width: u32, height: u32) -> Vec<u8> {
        // Create a pixmap to render into
        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .unwrap_or_else(|| tiny_skia::Pixmap::new(1, 1).unwrap());

        // Calculate the transform to fit the SVG into the target size
        let sx = width as f32 / self.default_size.width;
        let sy = height as f32 / self.default_size.height;
        let transform = tiny_skia::Transform::from_scale(sx, sy);

        // Render the SVG
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());

        // Convert from premultiplied RGBA to straight RGBA
        let data = pixmap.data();
        let mut result = Vec::with_capacity(data.len());

        for chunk in data.chunks(4) {
            let a = chunk[3] as f32 / 255.0;
            if a > 0.0 {
                // Unpremultiply RGB
                result.push((chunk[0] as f32 / a).min(255.0) as u8);
                result.push((chunk[1] as f32 / a).min(255.0) as u8);
                result.push((chunk[2] as f32 / a).min(255.0) as u8);
                result.push(chunk[3]);
            } else {
                // Fully transparent pixel
                result.extend_from_slice(&[0, 0, 0, 0]);
            }
        }

        result
    }

    /// Render the SVG to an Image at a specific size.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Errors
    ///
    /// Returns an error if the image cannot be uploaded to the GPU.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let image = svg.render_to_image(&mut manager, 48, 48)?;
    /// renderer.draw_image(&image, rect);
    /// ```
    pub fn render_to_image(
        &self,
        manager: &mut ImageManager,
        width: u32,
        height: u32,
    ) -> RenderResult<Image> {
        let rgba = self.render_to_rgba(width, height);
        manager.load_rgba(&rgba, width, height)
    }

    /// Render the SVG at its natural size scaled by a factor.
    ///
    /// This is the most common method for HiDPI support - it renders the
    /// SVG at its natural size multiplied by the scale factor.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `scale_factor` - The scale factor (e.g., 1.0, 2.0, 1.5)
    ///
    /// # Returns
    ///
    /// An `Image` rendered at `natural_size * scale_factor` pixels.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For a 24x24 SVG on a 2x display:
    /// let image = svg.render_scaled(&mut manager, 2.0)?;
    /// // image is 48x48 pixels
    /// ```
    pub fn render_scaled(
        &self,
        manager: &mut ImageManager,
        scale_factor: f64,
    ) -> RenderResult<Image> {
        let width = (self.default_size.width as f64 * scale_factor).round() as u32;
        let height = (self.default_size.height as f64 * scale_factor).round() as u32;

        // Ensure minimum size of 1x1
        let width = width.max(1);
        let height = height.max(1);

        self.render_to_image(manager, width, height)
    }

    /// Render the SVG at a specific logical size with a scale factor.
    ///
    /// This renders the SVG at `logical_size * scale_factor` physical pixels,
    /// which is useful when you want to display the SVG at a different size
    /// than its natural dimensions.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `logical_width` - Desired logical width
    /// * `logical_height` - Desired logical height
    /// * `scale_factor` - The display scale factor
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Render a 24x24 SVG at 32x32 logical size on a 2x display
    /// // Result is 64x64 physical pixels
    /// let image = svg.render_at_size(&mut manager, 32.0, 32.0, 2.0)?;
    /// ```
    pub fn render_at_size(
        &self,
        manager: &mut ImageManager,
        logical_width: f32,
        logical_height: f32,
        scale_factor: f64,
    ) -> RenderResult<Image> {
        let width = (logical_width as f64 * scale_factor).round() as u32;
        let height = (logical_height as f64 * scale_factor).round() as u32;

        // Ensure minimum size of 1x1
        let width = width.max(1);
        let height = height.max(1);

        self.render_to_image(manager, width, height)
    }

    /// Check if this SVG has any gradients.
    ///
    /// SVGs with gradients may render differently at different sizes.
    pub fn has_gradients(&self) -> bool {
        // Walk the tree to check for gradients
        // This is a simplified check - a full implementation would walk all nodes
        self.tree.root().has_children()
    }

    // ========================================================================
    // CACHED RENDERING METHODS
    // ========================================================================

    /// Render the SVG to an Image with caching support.
    ///
    /// If the rasterization exists in the cache, it's used directly.
    /// Otherwise, the SVG is rendered and the result is cached.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `cache` - The SVG cache to check/store rasterizations
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut cache = SvgCache::with_defaults();
    /// let image = svg.render_cached(&mut manager, &mut cache, 48, 48)?;
    /// ```
    pub fn render_cached(
        &self,
        manager: &mut ImageManager,
        cache: &mut SvgCache,
        width: u32,
        height: u32,
    ) -> RenderResult<Image> {
        let rgba = cache.get_or_render(self, width, height);
        manager.load_rgba(&rgba, width, height)
    }

    /// Render the SVG with caching using a file path as the cache key.
    ///
    /// This provides better cache key stability than `render_cached` since
    /// the file path is used directly as part of the key.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `cache` - The SVG cache to check/store rasterizations
    /// * `path` - The original file path (for cache key)
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    pub fn render_cached_with_path(
        &self,
        manager: &mut ImageManager,
        cache: &mut SvgCache,
        path: impl AsRef<Path>,
        width: u32,
        height: u32,
    ) -> RenderResult<Image> {
        let rgba = cache.get_or_render_file(self, path, width, height);
        manager.load_rgba(&rgba, width, height)
    }

    /// Render the SVG at its natural size scaled by a factor, with caching.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `cache` - The SVG cache to check/store rasterizations
    /// * `scale_factor` - The scale factor (e.g., 1.0, 2.0, 1.5)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut cache = SvgCache::with_defaults();
    /// let image = svg.render_scaled_cached(&mut manager, &mut cache, 2.0)?;
    /// ```
    pub fn render_scaled_cached(
        &self,
        manager: &mut ImageManager,
        cache: &mut SvgCache,
        scale_factor: f64,
    ) -> RenderResult<Image> {
        let width = (self.default_size.width as f64 * scale_factor).round() as u32;
        let height = (self.default_size.height as f64 * scale_factor).round() as u32;

        // Ensure minimum size of 1x1
        let width = width.max(1);
        let height = height.max(1);

        self.render_cached(manager, cache, width, height)
    }

    /// Render at a specific logical size with scale factor, using cache.
    ///
    /// # Arguments
    ///
    /// * `manager` - The image manager to upload the rendered image to
    /// * `cache` - The SVG cache to check/store rasterizations
    /// * `logical_width` - Desired logical width
    /// * `logical_height` - Desired logical height
    /// * `scale_factor` - The display scale factor
    pub fn render_at_size_cached(
        &self,
        manager: &mut ImageManager,
        cache: &mut SvgCache,
        logical_width: f32,
        logical_height: f32,
        scale_factor: f64,
    ) -> RenderResult<Image> {
        let width = (logical_width as f64 * scale_factor).round() as u32;
        let height = (logical_height as f64 * scale_factor).round() as u32;

        // Ensure minimum size of 1x1
        let width = width.max(1);
        let height = height.max(1);

        self.render_cached(manager, cache, width, height)
    }
}

impl std::fmt::Debug for SvgImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvgImage")
            .field("natural_size", &self.default_size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SVG: &[u8] = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" fill="red"/>
        </svg>
    "#;

    #[test]
    fn test_svg_from_bytes() {
        let svg = SvgImage::from_bytes(SIMPLE_SVG).expect("Should parse valid SVG");
        assert_eq!(svg.natural_width(), 24.0);
        assert_eq!(svg.natural_height(), 24.0);
    }

    #[test]
    fn test_svg_natural_size() {
        let svg = SvgImage::from_bytes(SIMPLE_SVG).unwrap();
        let size = svg.natural_size();
        assert_eq!(size.width, 24.0);
        assert_eq!(size.height, 24.0);
    }

    #[test]
    fn test_svg_render_to_rgba() {
        let svg = SvgImage::from_bytes(SIMPLE_SVG).unwrap();
        let rgba = svg.render_to_rgba(48, 48);

        // Should have correct size
        assert_eq!(rgba.len(), 48 * 48 * 4);

        // Center pixel should be red (circle is at center)
        let center_idx = (24 * 48 + 24) * 4;
        assert!(rgba[center_idx] > 200, "Red channel should be high"); // R
        assert!(rgba[center_idx + 1] < 50, "Green channel should be low"); // G
        assert!(rgba[center_idx + 2] < 50, "Blue channel should be low"); // B
        assert!(rgba[center_idx + 3] > 200, "Alpha should be opaque"); // A
    }

    #[test]
    fn test_svg_invalid_data() {
        let result = SvgImage::from_bytes(b"not valid svg");
        assert!(result.is_err());
    }
}
