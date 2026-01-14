//! Glyph rasterization and caching.
//!
//! This module provides glyph rasterization using cosmic-text's SwashCache,
//! with support for grayscale and subpixel antialiasing.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{FontSystem, GlyphCache, GlyphRenderMode};
//! use cosmic_text::CacheKey;
//!
//! let mut font_system = FontSystem::new();
//! let mut glyph_cache = GlyphCache::new();
//!
//! // Rasterize a glyph (CacheKey comes from TextLayout/ShapedText)
//! // let rasterized = glyph_cache.rasterize(&mut font_system, cache_key, GlyphRenderMode::Grayscale);
//! ```

use cosmic_text::{CacheKey, SwashCache, SwashContent};

use super::FontSystem;

/// Rendering mode for glyph rasterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GlyphRenderMode {
    /// Grayscale antialiasing (8-bit alpha mask).
    /// Works well on all displays and is the safest default.
    #[default]
    Grayscale,
    /// Subpixel (LCD) antialiasing using RGB subpixels.
    /// Provides sharper text on LCD displays but may show color fringing
    /// on non-LCD displays or at certain viewing angles.
    SubpixelHorizontalRgb,
    /// Subpixel antialiasing with BGR subpixel order.
    SubpixelHorizontalBgr,
    /// Vertical subpixel rendering (rare, for vertically-oriented LCD panels).
    SubpixelVerticalRgb,
    /// Vertical BGR subpixel rendering.
    SubpixelVerticalBgr,
}

impl GlyphRenderMode {
    /// Check if this mode uses subpixel rendering.
    pub fn is_subpixel(&self) -> bool {
        !matches!(self, GlyphRenderMode::Grayscale)
    }

    /// Detect the best rendering mode for the current platform.
    ///
    /// This attempts to determine the optimal subpixel rendering mode
    /// based on the operating system. Falls back to Grayscale if uncertain.
    pub fn detect_platform() -> Self {
        // On macOS, Apple recommends grayscale AA for Retina displays
        // and has removed subpixel AA in recent versions.
        #[cfg(target_os = "macos")]
        {
            GlyphRenderMode::Grayscale
        }

        // On Windows, horizontal RGB is most common.
        #[cfg(target_os = "windows")]
        {
            GlyphRenderMode::SubpixelHorizontalRgb
        }

        // On Linux, it varies by display configuration.
        // Default to grayscale for safety.
        #[cfg(target_os = "linux")]
        {
            GlyphRenderMode::Grayscale
        }

        // Other platforms: use grayscale
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            GlyphRenderMode::Grayscale
        }
    }
}

/// Pixel format of a rasterized glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphPixelFormat {
    /// 8-bit alpha mask (grayscale antialiasing).
    Alpha,
    /// 32-bit RGBA with subpixel coverage in RGB channels.
    SubpixelRgba,
    /// 32-bit RGBA color (for color emoji and bitmaps).
    ColorRgba,
}

impl GlyphPixelFormat {
    /// Bytes per pixel for this format.
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            GlyphPixelFormat::Alpha => 1,
            GlyphPixelFormat::SubpixelRgba | GlyphPixelFormat::ColorRgba => 4,
        }
    }
}

/// A rasterized glyph ready for upload to the GPU.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Pixel data of the rasterized glyph.
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// X offset from glyph origin to top-left of bitmap.
    pub offset_x: i32,
    /// Y offset from glyph origin to top-left of bitmap.
    pub offset_y: i32,
    /// Pixel format of the data.
    pub format: GlyphPixelFormat,
    /// Whether this glyph has color (emoji).
    pub is_color: bool,
}

impl RasterizedGlyph {
    /// Check if this glyph is empty (zero size).
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Get the size of the pixel data in bytes.
    pub fn data_size(&self) -> usize {
        self.width as usize * self.height as usize * self.format.bytes_per_pixel()
    }

    /// Convert alpha-only data to RGBA for uniform texture storage.
    ///
    /// This allows mixing grayscale and color glyphs in the same atlas.
    pub fn to_rgba(&self) -> Vec<u8> {
        match self.format {
            GlyphPixelFormat::Alpha => {
                let mut rgba = Vec::with_capacity(self.data.len() * 4);
                for &alpha in &self.data {
                    // White color with variable alpha
                    rgba.extend_from_slice(&[255, 255, 255, alpha]);
                }
                rgba
            }
            GlyphPixelFormat::SubpixelRgba | GlyphPixelFormat::ColorRgba => self.data.clone(),
        }
    }
}

/// Glyph rasterization cache using cosmic-text's SwashCache.
///
/// This struct manages glyph rasterization and provides a cache for
/// recently rasterized glyphs to avoid redundant work.
pub struct GlyphCache {
    /// The underlying swash cache from cosmic-text.
    swash_cache: SwashCache,
    /// Default render mode.
    render_mode: GlyphRenderMode,
    /// Statistics for debugging/monitoring.
    stats: GlyphCacheStats,
}

/// Statistics about glyph cache usage.
#[derive(Debug, Clone, Default)]
pub struct GlyphCacheStats {
    /// Number of rasterization requests.
    pub rasterize_calls: u64,
    /// Number of glyphs successfully rasterized.
    pub glyphs_rasterized: u64,
    /// Number of empty/missing glyphs.
    pub empty_glyphs: u64,
    /// Number of color glyphs (emoji).
    pub color_glyphs: u64,
}

impl GlyphCache {
    /// Create a new glyph cache with default settings.
    pub fn new() -> Self {
        Self {
            swash_cache: SwashCache::new(),
            render_mode: GlyphRenderMode::detect_platform(),
            stats: GlyphCacheStats::default(),
        }
    }

    /// Create a new glyph cache with a specific render mode.
    pub fn with_render_mode(render_mode: GlyphRenderMode) -> Self {
        Self {
            swash_cache: SwashCache::new(),
            render_mode,
            stats: GlyphCacheStats::default(),
        }
    }

    /// Get the current render mode.
    pub fn render_mode(&self) -> GlyphRenderMode {
        self.render_mode
    }

    /// Set the render mode.
    pub fn set_render_mode(&mut self, mode: GlyphRenderMode) {
        self.render_mode = mode;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &GlyphCacheStats {
        &self.stats
    }

    /// Reset cache statistics.
    pub fn reset_stats(&mut self) {
        self.stats = GlyphCacheStats::default();
    }

    /// Rasterize a glyph using the default render mode.
    ///
    /// Returns `None` if the glyph cannot be rasterized (e.g., whitespace).
    pub fn rasterize(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> Option<RasterizedGlyph> {
        self.rasterize_with_mode(font_system, cache_key, self.render_mode)
    }

    /// Rasterize a glyph with a specific render mode.
    ///
    /// Returns `None` if the glyph cannot be rasterized (e.g., whitespace).
    pub fn rasterize_with_mode(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
        mode: GlyphRenderMode,
    ) -> Option<RasterizedGlyph> {
        self.stats.rasterize_calls += 1;

        // Get the image from cosmic-text's swash cache
        // We need to extract the data we need to avoid borrow issues
        let image_data = {
            let image_ref = self
                .swash_cache
                .get_image(font_system.inner_mut(), cache_key);

            match image_ref {
                Some(img) if img.placement.width > 0 && img.placement.height > 0 => {
                    Some((
                        img.content,
                        img.placement.width,
                        img.placement.height,
                        img.placement.left,
                        img.placement.top,
                        img.data.clone(),
                    ))
                }
                _ => None,
            }
        };

        let (content, width, height, left, top, data) = match image_data {
            Some(data) => data,
            None => {
                self.stats.empty_glyphs += 1;
                return None;
            }
        };

        let result = self.convert_image_data(content, width, height, left, top, &data, mode);

        if result.is_color {
            self.stats.color_glyphs += 1;
        }
        self.stats.glyphs_rasterized += 1;

        Some(result)
    }

    /// Convert image data to our RasterizedGlyph format.
    fn convert_image_data(
        &self,
        content: SwashContent,
        width: u32,
        height: u32,
        offset_x: i32,
        offset_y: i32,
        data: &[u8],
        mode: GlyphRenderMode,
    ) -> RasterizedGlyph {
        match content {
            SwashContent::Mask => {
                // 8-bit alpha mask (grayscale AA)
                RasterizedGlyph {
                    data: data.to_vec(),
                    width,
                    height,
                    offset_x,
                    offset_y,
                    format: GlyphPixelFormat::Alpha,
                    is_color: false,
                }
            }
            SwashContent::SubpixelMask => {
                // 32-bit RGBA subpixel mask
                // The RGB channels contain per-channel coverage
                let processed_data = if mode.is_subpixel() {
                    self.process_subpixel_data(data, mode)
                } else {
                    // Convert subpixel to grayscale if grayscale mode requested
                    self.subpixel_to_grayscale(data)
                };

                let format = if mode.is_subpixel() {
                    GlyphPixelFormat::SubpixelRgba
                } else {
                    GlyphPixelFormat::Alpha
                };

                RasterizedGlyph {
                    data: processed_data,
                    width,
                    height,
                    offset_x,
                    offset_y,
                    format,
                    is_color: false,
                }
            }
            SwashContent::Color => {
                // 32-bit RGBA color (emoji, etc.)
                RasterizedGlyph {
                    data: data.to_vec(),
                    width,
                    height,
                    offset_x,
                    offset_y,
                    format: GlyphPixelFormat::ColorRgba,
                    is_color: true,
                }
            }
        }
    }

    /// Process subpixel data, potentially swapping channel order.
    fn process_subpixel_data(&self, data: &[u8], mode: GlyphRenderMode) -> Vec<u8> {
        match mode {
            GlyphRenderMode::SubpixelHorizontalRgb | GlyphRenderMode::SubpixelVerticalRgb => {
                // Data is already in RGB order
                data.to_vec()
            }
            GlyphRenderMode::SubpixelHorizontalBgr | GlyphRenderMode::SubpixelVerticalBgr => {
                // Swap R and B channels
                let mut result = data.to_vec();
                for chunk in result.chunks_exact_mut(4) {
                    chunk.swap(0, 2); // Swap R and B
                }
                result
            }
            GlyphRenderMode::Grayscale => {
                // Should not reach here, but handle gracefully
                self.subpixel_to_grayscale(data)
            }
        }
    }

    /// Convert subpixel RGBA data to grayscale alpha.
    fn subpixel_to_grayscale(&self, data: &[u8]) -> Vec<u8> {
        data.chunks_exact(4)
            .map(|rgba| {
                // Average the RGB channels for a grayscale value
                let r = rgba[0] as u32;
                let g = rgba[1] as u32;
                let b = rgba[2] as u32;
                ((r + g + b) / 3) as u8
            })
            .collect()
    }

    /// Create a CacheKey from layout glyph data.
    ///
    /// This is a helper to create cache keys from the data stored in LayoutGlyph.
    /// Returns the CacheKey and the integer pixel offsets (x, y) for rendering.
    ///
    /// # Arguments
    ///
    /// * `font_id` - The font face ID
    /// * `glyph_id` - The glyph ID within the font
    /// * `font_size` - The font size in pixels
    /// * `position` - The glyph position (x, y) including fractional part
    /// * `flags` - Cache key flags from the layout
    pub fn make_cache_key(
        font_id: fontdb::ID,
        glyph_id: u16,
        font_size: f32,
        position: (f32, f32),
        flags: cosmic_text::CacheKeyFlags,
    ) -> (CacheKey, i32, i32) {
        // cosmic-text's CacheKey::new handles subpixel binning and returns pixel offsets
        CacheKey::new(font_id, glyph_id, font_size, position, flags)
    }

    /// Create a CacheKey from a LayoutGlyph.
    ///
    /// Returns the CacheKey and the integer pixel offsets (x, y) for rendering.
    pub fn cache_key_from_layout_glyph(
        glyph: &super::LayoutGlyph,
    ) -> (CacheKey, i32, i32) {
        Self::make_cache_key(
            glyph.font_id,
            glyph.glyph_id,
            glyph.font_size,
            (glyph.x + glyph.x_offset, glyph.y + glyph.y_offset),
            glyph.cache_key_flags,
        )
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for GlyphCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphCache")
            .field("render_mode", &self.render_mode)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_detection() {
        let mode = GlyphRenderMode::detect_platform();
        // Just verify it returns something without panicking
        let _ = mode.is_subpixel();
    }

    #[test]
    fn test_pixel_format_bytes() {
        assert_eq!(GlyphPixelFormat::Alpha.bytes_per_pixel(), 1);
        assert_eq!(GlyphPixelFormat::SubpixelRgba.bytes_per_pixel(), 4);
        assert_eq!(GlyphPixelFormat::ColorRgba.bytes_per_pixel(), 4);
    }

    #[test]
    fn test_rasterized_glyph_to_rgba() {
        // Test alpha to RGBA conversion
        let glyph = RasterizedGlyph {
            data: vec![128, 255, 0],
            width: 3,
            height: 1,
            offset_x: 0,
            offset_y: 0,
            format: GlyphPixelFormat::Alpha,
            is_color: false,
        };

        let rgba = glyph.to_rgba();
        assert_eq!(rgba.len(), 12); // 3 pixels * 4 bytes
        assert_eq!(&rgba[0..4], &[255, 255, 255, 128]);
        assert_eq!(&rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&rgba[8..12], &[255, 255, 255, 0]);
    }

    #[test]
    fn test_subpixel_to_grayscale() {
        let cache = GlyphCache::new();
        let subpixel_data = vec![100, 150, 200, 255]; // RGBA
        let grayscale = cache.subpixel_to_grayscale(&subpixel_data);
        assert_eq!(grayscale.len(), 1);
        assert_eq!(grayscale[0], 150); // (100 + 150 + 200) / 3 = 150
    }
}
