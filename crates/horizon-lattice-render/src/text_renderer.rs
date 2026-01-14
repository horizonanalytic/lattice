//! Text rendering integration for GPU-accelerated text display.
//!
//! This module provides the [`TextRenderer`] which combines glyph rasterization,
//! caching, and GPU rendering into a unified text rendering system.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::{
//!     GpuRenderer, Renderer, Color, Size, Point,
//!     TextRenderer, FontSystem, Font, FontFamily, TextLayout,
//! };
//!
//! // Setup
//! let mut font_system = FontSystem::new();
//! let mut text_renderer = TextRenderer::new().unwrap();
//! // let mut gpu_renderer = GpuRenderer::new(&surface)?;
//!
//! // Create text layout
//! let font = Font::new(FontFamily::SansSerif, 16.0);
//! let layout = TextLayout::new(&mut font_system, "Hello, World!", &font);
//!
//! // Render
//! // gpu_renderer.begin_frame(Color::WHITE, Size::new(800.0, 600.0));
//! // text_renderer.draw_layout(
//! //     &mut gpu_renderer,
//! //     &mut font_system,
//! //     &layout,
//! //     Point::new(100.0, 100.0),
//! //     Color::BLACK,
//! // );
//! // gpu_renderer.end_frame();
//! ```

use crate::error::RenderResult;
use crate::text::{
    FontSystem, GlyphAllocation, GlyphAtlas, GlyphCache, GlyphRenderMode, LayoutGlyph, TextLayout,
};
use crate::types::{Color, Point, Rect};

/// Text rendering configuration options.
#[derive(Debug, Clone)]
pub struct TextRendererConfig {
    /// Size of the glyph atlas texture.
    pub atlas_size: u32,
    /// Glyph rendering mode (grayscale or subpixel).
    pub render_mode: GlyphRenderMode,
}

impl Default for TextRendererConfig {
    fn default() -> Self {
        Self {
            atlas_size: 2048,
            render_mode: GlyphRenderMode::detect_platform(),
        }
    }
}

/// A positioned glyph ready for rendering.
#[derive(Debug, Clone)]
pub struct PreparedGlyph {
    /// Screen position (x, y) where the glyph should be rendered.
    pub position: Point,
    /// Atlas allocation with UV coordinates.
    pub allocation: GlyphAllocation,
    /// Text color for this glyph.
    pub color: Color,
}

impl PreparedGlyph {
    /// Get the destination rectangle for this glyph in screen coordinates.
    pub fn dest_rect(&self) -> Rect {
        Rect::new(
            self.position.x + self.allocation.offset_x as f32,
            self.position.y - self.allocation.offset_y as f32,
            self.allocation.width as f32,
            self.allocation.height as f32,
        )
    }

    /// Get the UV rectangle in the atlas.
    pub fn uv_rect(&self, atlas_size: u32) -> (f32, f32, f32, f32) {
        self.allocation.uv_rect(atlas_size)
    }
}

/// GPU-accelerated text renderer.
///
/// The TextRenderer manages glyph rasterization, caching, and atlas storage,
/// providing high-performance text rendering for GUI applications.
pub struct TextRenderer {
    /// Glyph rasterization cache.
    glyph_cache: GlyphCache,
    /// GPU texture atlas for glyph storage.
    glyph_atlas: GlyphAtlas,
    /// Configuration.
    config: TextRendererConfig,
}

impl TextRenderer {
    /// Create a new text renderer with default settings.
    pub fn new() -> RenderResult<Self> {
        Self::with_config(TextRendererConfig::default())
    }

    /// Create a new text renderer with custom configuration.
    pub fn with_config(config: TextRendererConfig) -> RenderResult<Self> {
        let glyph_atlas = GlyphAtlas::new(config.atlas_size)?;
        let glyph_cache = GlyphCache::with_render_mode(config.render_mode);

        Ok(Self {
            glyph_cache,
            glyph_atlas,
            config,
        })
    }

    /// Get the glyph atlas for binding in render passes.
    pub fn glyph_atlas(&self) -> &GlyphAtlas {
        &self.glyph_atlas
    }

    /// Get the render mode.
    pub fn render_mode(&self) -> GlyphRenderMode {
        self.config.render_mode
    }

    /// Set the render mode.
    pub fn set_render_mode(&mut self, mode: GlyphRenderMode) {
        self.config.render_mode = mode;
        self.glyph_cache.set_render_mode(mode);
    }

    /// Prepare a text layout for rendering.
    ///
    /// This rasterizes and caches all glyphs in the layout, returning a list
    /// of prepared glyphs ready to be rendered as textured quads.
    ///
    /// # Arguments
    ///
    /// * `font_system` - The font system for glyph rasterization
    /// * `layout` - The text layout to prepare
    /// * `position` - The position to render the layout at
    /// * `default_color` - Default color for glyphs without a specific color
    pub fn prepare_layout(
        &mut self,
        font_system: &mut FontSystem,
        layout: &TextLayout,
        position: Point,
        default_color: Color,
    ) -> RenderResult<Vec<PreparedGlyph>> {
        let mut prepared = Vec::new();

        for line in layout.lines() {
            for glyph in &line.glyphs {
                // Skip inline elements (widgets, images)
                if glyph.is_inline_element() {
                    continue;
                }

                // Prepare this glyph
                if let Some(prepared_glyph) = self.prepare_glyph(
                    font_system,
                    glyph,
                    position,
                    line.baseline_y,
                    default_color,
                )? {
                    prepared.push(prepared_glyph);
                }
            }
        }

        Ok(prepared)
    }

    /// Prepare a single glyph for rendering.
    fn prepare_glyph(
        &mut self,
        font_system: &mut FontSystem,
        glyph: &LayoutGlyph,
        layout_position: Point,
        baseline_y: f32,
        default_color: Color,
    ) -> RenderResult<Option<PreparedGlyph>> {
        // Create cache key
        let (cache_key, pixel_x, pixel_y) = GlyphCache::cache_key_from_layout_glyph(glyph);

        // Get or rasterize the glyph
        let allocation = if let Some(alloc) = self.glyph_atlas.get(&cache_key) {
            alloc.clone()
        } else {
            // Rasterize the glyph
            let rasterized = match self.glyph_cache.rasterize(font_system, cache_key) {
                Some(g) => g,
                None => return Ok(None), // Empty glyph (whitespace, etc.)
            };

            // Insert into atlas
            self.glyph_atlas.insert(cache_key, &rasterized)?
        };

        // Calculate screen position
        let screen_x = layout_position.x + pixel_x as f32;
        let screen_y = layout_position.y + baseline_y + pixel_y as f32;

        // Determine color
        let color = if let Some(c) = glyph.color {
            Color::from_rgba8(c[0], c[1], c[2], c[3])
        } else {
            default_color
        };

        Ok(Some(PreparedGlyph {
            position: Point::new(screen_x, screen_y),
            allocation,
            color,
        }))
    }

    /// Prepare glyphs for a simple string (single font, single color).
    ///
    /// This is a convenience method that creates a layout and prepares it.
    pub fn prepare_text(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        font: &crate::text::Font,
        position: Point,
        color: Color,
    ) -> RenderResult<Vec<PreparedGlyph>> {
        let layout = TextLayout::new(font_system, text, font);
        self.prepare_layout(font_system, &layout, position, color)
    }

    /// Get the bind group for the glyph atlas (for use in custom render passes).
    pub fn atlas_bind_group(&self) -> &wgpu::BindGroup {
        self.glyph_atlas.bind_group()
    }

    /// Get the atlas texture size.
    pub fn atlas_size(&self) -> u32 {
        self.glyph_atlas.size()
    }

    /// Clear the glyph atlas and cache.
    ///
    /// This should be called when fonts change or memory needs to be freed.
    pub fn clear(&mut self) {
        self.glyph_atlas.clear();
    }

    /// Get statistics about the text renderer.
    pub fn stats(&self) -> TextRendererStats {
        TextRendererStats {
            atlas_glyph_count: self.glyph_atlas.stats().glyph_count,
            atlas_usage: self.glyph_atlas.usage(),
            cache_hits: self.glyph_atlas.stats().cache_hits,
            cache_misses: self.glyph_atlas.stats().cache_misses,
            rasterize_calls: self.glyph_cache.stats().rasterize_calls,
            glyphs_rasterized: self.glyph_cache.stats().glyphs_rasterized,
        }
    }
}

impl std::fmt::Debug for TextRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextRenderer")
            .field("render_mode", &self.config.render_mode)
            .field("atlas_size", &self.glyph_atlas.size())
            .field("glyph_count", &self.glyph_atlas.stats().glyph_count)
            .finish()
    }
}

/// Statistics about text rendering performance.
#[derive(Debug, Clone, Default)]
pub struct TextRendererStats {
    /// Number of glyphs in the atlas.
    pub atlas_glyph_count: usize,
    /// Percentage of atlas space used.
    pub atlas_usage: f32,
    /// Number of atlas cache hits.
    pub cache_hits: u64,
    /// Number of atlas cache misses.
    pub cache_misses: u64,
    /// Total rasterization calls.
    pub rasterize_calls: u64,
    /// Total glyphs rasterized.
    pub glyphs_rasterized: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepared_glyph_dest_rect() {
        use crate::text::GlyphPixelFormat;

        let glyph = PreparedGlyph {
            position: Point::new(100.0, 200.0),
            allocation: GlyphAllocation {
                x: 0,
                y: 0,
                width: 10,
                height: 12,
                offset_x: 1,
                offset_y: 10,
                is_color: false,
                format: GlyphPixelFormat::Alpha,
            },
            color: Color::BLACK,
        };

        let rect = glyph.dest_rect();
        assert_eq!(rect.origin.x, 101.0); // 100 + 1 offset
        assert_eq!(rect.origin.y, 190.0); // 200 - 10 offset
        assert_eq!(rect.size.width, 10.0);
        assert_eq!(rect.size.height, 12.0);
    }
}
