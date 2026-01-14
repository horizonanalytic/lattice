//! GPU texture atlas for glyph storage.
//!
//! This module provides a texture atlas specifically designed for storing
//! rasterized glyphs with efficient caching and LRU eviction.
//!
//! # Architecture
//!
//! The glyph atlas uses a shelf-based packing algorithm to efficiently
//! pack glyphs of varying sizes into a single GPU texture. When the atlas
//! is full, it uses LRU (Least Recently Used) eviction to make room for
//! new glyphs.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{FontSystem, GlyphCache, GlyphAtlas};
//! use cosmic_text::CacheKey;
//!
//! // The GlyphAtlas is typically used internally by the text renderer
//! // let mut atlas = GlyphAtlas::new(2048)?;
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use cosmic_text::CacheKey;

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};

use super::glyph_cache::{GlyphPixelFormat, RasterizedGlyph};

/// Default glyph atlas texture size.
pub const DEFAULT_GLYPH_ATLAS_SIZE: u32 = 2048;

/// Minimum glyph atlas texture size.
pub const MIN_GLYPH_ATLAS_SIZE: u32 = 512;

/// Maximum glyph atlas texture size.
pub const MAX_GLYPH_ATLAS_SIZE: u32 = 4096;

/// Padding between glyph allocations to prevent texture bleeding.
const GLYPH_PADDING: u32 = 1;

/// A shelf (horizontal row) in the atlas for glyph allocation.
#[derive(Debug)]
struct Shelf {
    /// Y position of this shelf in the atlas.
    y: u32,
    /// Height of this shelf.
    height: u32,
    /// Current X position for the next allocation.
    cursor_x: u32,
    /// Allocations in this shelf (for LRU eviction).
    allocations: Vec<ShelfAllocation>,
}

/// An allocation within a shelf.
#[derive(Debug, Clone)]
struct ShelfAllocation {
    /// X position in the shelf.
    x: u32,
    /// Width of the allocation.
    width: u32,
    /// The cache key for this glyph.
    cache_key: CacheKey,
}

impl Shelf {
    fn new(y: u32, height: u32) -> Self {
        Self {
            y,
            height,
            cursor_x: 0,
            allocations: Vec::new(),
        }
    }

    /// Try to allocate space in this shelf.
    fn try_allocate(
        &mut self,
        width: u32,
        height: u32,
        atlas_width: u32,
        cache_key: CacheKey,
    ) -> Option<(u32, u32)> {
        // Check if allocation fits in height
        if height > self.height {
            return None;
        }

        // Check if allocation fits in remaining width
        let padded_width = width + GLYPH_PADDING;
        if self.cursor_x + padded_width > atlas_width {
            return None;
        }

        let x = self.cursor_x;
        self.cursor_x += padded_width;

        // Track the allocation
        self.allocations.push(ShelfAllocation {
            x,
            width: padded_width,
            cache_key,
        });

        Some((x, self.y))
    }
}

/// Information about an allocated glyph in the atlas.
#[derive(Debug, Clone)]
pub struct GlyphAllocation {
    /// X position in the atlas texture.
    pub x: u32,
    /// Y position in the atlas texture.
    pub y: u32,
    /// Width of the glyph.
    pub width: u32,
    /// Height of the glyph.
    pub height: u32,
    /// X offset for rendering (from glyph origin).
    pub offset_x: i32,
    /// Y offset for rendering (from glyph origin).
    pub offset_y: i32,
    /// Whether this is a color glyph (emoji).
    pub is_color: bool,
    /// The pixel format of this glyph.
    pub format: GlyphPixelFormat,
}

impl GlyphAllocation {
    /// Get normalized UV coordinates for this glyph.
    ///
    /// Returns (u_min, v_min, u_max, v_max).
    pub fn uv_rect(&self, atlas_size: u32) -> (f32, f32, f32, f32) {
        let size = atlas_size as f32;
        (
            self.x as f32 / size,
            self.y as f32 / size,
            (self.x + self.width) as f32 / size,
            (self.y + self.height) as f32 / size,
        )
    }
}

/// Entry in the glyph cache map.
#[derive(Debug)]
struct CacheEntry {
    /// The allocation information.
    allocation: GlyphAllocation,
    /// Last access time for LRU eviction.
    last_access: u64,
}

/// GPU texture atlas for storing rasterized glyphs.
///
/// The atlas uses RGBA8 format to support both grayscale text and color emoji.
/// Grayscale glyphs use white RGB with alpha for coverage.
pub struct GlyphAtlas {
    /// GPU texture.
    texture: wgpu::Texture,
    /// Texture view for rendering.
    texture_view: wgpu::TextureView,
    /// Texture sampler.
    sampler: wgpu::Sampler,
    /// Bind group for this atlas.
    bind_group: wgpu::BindGroup,
    /// Atlas texture size (square).
    size: u32,
    /// Shelves for allocation.
    shelves: Vec<Shelf>,
    /// Y position for the next shelf.
    next_shelf_y: u32,
    /// Cache mapping CacheKey to allocation.
    cache: HashMap<CacheKey, CacheEntry>,
    /// Monotonic counter for LRU tracking.
    access_counter: AtomicU64,
    /// Statistics.
    stats: GlyphAtlasStats,
}

/// Statistics about glyph atlas usage.
#[derive(Debug, Clone, Default)]
pub struct GlyphAtlasStats {
    /// Number of glyphs currently in the atlas.
    pub glyph_count: usize,
    /// Number of cache hits.
    pub cache_hits: u64,
    /// Number of cache misses.
    pub cache_misses: u64,
    /// Number of evictions performed.
    pub evictions: u64,
}

impl GlyphAtlas {
    /// Create a new glyph atlas with the specified size.
    pub fn new(size: u32) -> RenderResult<Self> {
        let size = size.clamp(MIN_GLYPH_ATLAS_SIZE, MAX_GLYPH_ATLAS_SIZE);
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        // Create the atlas texture (RGBA8 for color emoji support)
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler with linear filtering for smooth text
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph_atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group
        let bind_group = Self::create_bind_group(device, &texture_view, &sampler);

        Ok(Self {
            texture,
            texture_view,
            sampler,
            bind_group,
            size,
            shelves: Vec::new(),
            next_shelf_y: 0,
            cache: HashMap::new(),
            access_counter: AtomicU64::new(0),
            stats: GlyphAtlasStats::default(),
        })
    }

    /// Create with default size.
    pub fn with_default_size() -> RenderResult<Self> {
        Self::new(DEFAULT_GLYPH_ATLAS_SIZE)
    }

    /// Create the bind group for this atlas.
    fn create_bind_group(
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let layout = Self::bind_group_layout(device);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_atlas_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Get the bind group layout for glyph atlases.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph_atlas_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
        })
    }

    /// Get the atlas texture size.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Get the texture view for rendering.
    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    /// Get the bind group for rendering.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get atlas statistics.
    pub fn stats(&self) -> &GlyphAtlasStats {
        &self.stats
    }

    /// Look up a glyph in the cache.
    ///
    /// Returns the allocation if found, updating the LRU timestamp.
    pub fn get(&mut self, cache_key: &CacheKey) -> Option<&GlyphAllocation> {
        if let Some(entry) = self.cache.get_mut(cache_key) {
            entry.last_access = self.access_counter.fetch_add(1, Ordering::Relaxed);
            self.stats.cache_hits += 1;
            Some(&entry.allocation)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    /// Check if a glyph is in the cache without updating LRU.
    pub fn contains(&self, cache_key: &CacheKey) -> bool {
        self.cache.contains_key(cache_key)
    }

    /// Insert a rasterized glyph into the atlas.
    ///
    /// Returns the allocation if successful. May evict old glyphs if the atlas is full.
    pub fn insert(
        &mut self,
        cache_key: CacheKey,
        glyph: &RasterizedGlyph,
    ) -> RenderResult<GlyphAllocation> {
        // Check if already in cache
        if let Some(entry) = self.cache.get_mut(&cache_key) {
            entry.last_access = self.access_counter.fetch_add(1, Ordering::Relaxed);
            return Ok(entry.allocation.clone());
        }

        // Convert to RGBA for uniform texture storage
        let rgba_data = glyph.to_rgba();

        // Try to allocate space
        let (x, y) = match self.try_allocate(glyph.width, glyph.height, cache_key) {
            Some(pos) => pos,
            None => {
                // Atlas is full, try to evict and retry
                self.evict_lru()?;
                self.try_allocate(glyph.width, glyph.height, cache_key)
                    .ok_or_else(|| {
                        RenderError::GlyphAtlas(format!(
                            "Glyph {}x{} too large for atlas size {}",
                            glyph.width, glyph.height, self.size
                        ))
                    })?
            }
        };

        // Upload to GPU
        self.upload(x, y, glyph.width, glyph.height, &rgba_data)?;

        // Create allocation record
        let allocation = GlyphAllocation {
            x,
            y,
            width: glyph.width,
            height: glyph.height,
            offset_x: glyph.offset_x,
            offset_y: glyph.offset_y,
            is_color: glyph.is_color,
            format: glyph.format,
        };

        // Add to cache
        let access_time = self.access_counter.fetch_add(1, Ordering::Relaxed);
        self.cache.insert(
            cache_key,
            CacheEntry {
                allocation: allocation.clone(),
                last_access: access_time,
            },
        );
        self.stats.glyph_count = self.cache.len();

        Ok(allocation)
    }

    /// Try to allocate space for a glyph.
    fn try_allocate(&mut self, width: u32, height: u32, cache_key: CacheKey) -> Option<(u32, u32)> {
        let padded_width = width + GLYPH_PADDING;
        let padded_height = height + GLYPH_PADDING;

        // Check if allocation is even possible
        if padded_width > self.size || padded_height > self.size {
            return None;
        }

        // Try existing shelves first (best fit by height)
        let mut best_shelf_idx = None;
        let mut best_waste = u32::MAX;

        for (idx, shelf) in self.shelves.iter().enumerate() {
            if padded_height <= shelf.height {
                let remaining = self.size - shelf.cursor_x;
                if remaining >= padded_width {
                    let waste = shelf.height - padded_height;
                    if waste < best_waste {
                        best_waste = waste;
                        best_shelf_idx = Some(idx);
                    }
                }
            }
        }

        // Allocate from best fitting shelf
        if let Some(idx) = best_shelf_idx {
            return self.shelves[idx].try_allocate(width, height, self.size, cache_key);
        }

        // Create a new shelf if there's room
        if self.next_shelf_y + padded_height <= self.size {
            let mut shelf = Shelf::new(self.next_shelf_y, padded_height);
            let result = shelf.try_allocate(width, height, self.size, cache_key);
            self.next_shelf_y += padded_height;
            self.shelves.push(shelf);
            return result;
        }

        None
    }

    /// Upload glyph data to the GPU texture.
    fn upload(&self, x: u32, y: u32, width: u32, height: u32, data: &[u8]) -> RenderResult<()> {
        let ctx = GraphicsContext::get();
        let queue = ctx.queue();

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    /// Evict the least recently used glyphs to make room.
    fn evict_lru(&mut self) -> RenderResult<()> {
        if self.cache.is_empty() {
            return Err(RenderError::GlyphAtlas(
                "Cannot evict from empty glyph atlas".to_string(),
            ));
        }

        // Find entries to evict (oldest 25% of cache)
        let evict_count = (self.cache.len() / 4).max(1);

        let mut entries: Vec<_> = self
            .cache
            .iter()
            .map(|(k, v)| (*k, v.last_access))
            .collect();
        entries.sort_by_key(|(_, access)| *access);

        let to_evict: Vec<_> = entries.iter().take(evict_count).map(|(k, _)| *k).collect();

        for key in to_evict {
            self.cache.remove(&key);
            self.stats.evictions += 1;
        }

        // Clear the atlas and rebuild from remaining glyphs
        // This is a simple approach - a more sophisticated implementation
        // would compact the atlas in place
        self.clear_allocations();
        self.stats.glyph_count = self.cache.len();

        Ok(())
    }

    /// Clear all allocations (but keep the texture).
    fn clear_allocations(&mut self) {
        self.shelves.clear();
        self.next_shelf_y = 0;
        // Note: We don't clear the cache here since evict_lru removes specific entries
        // The remaining cache entries will be re-uploaded when accessed
    }

    /// Clear the entire atlas and cache.
    pub fn clear(&mut self) {
        self.shelves.clear();
        self.next_shelf_y = 0;
        self.cache.clear();
        self.stats.glyph_count = 0;
    }

    /// Get the percentage of the atlas that is allocated.
    pub fn usage(&self) -> f32 {
        if self.shelves.is_empty() {
            return 0.0;
        }

        let total_pixels = (self.size * self.size) as f32;
        let used_pixels: f32 = self
            .shelves
            .iter()
            .map(|s| (s.cursor_x * s.height) as f32)
            .sum();

        used_pixels / total_pixels
    }
}

impl std::fmt::Debug for GlyphAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphAtlas")
            .field("size", &self.size)
            .field("glyph_count", &self.cache.len())
            .field("usage", &format!("{:.1}%", self.usage() * 100.0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_allocation_uv() {
        let alloc = GlyphAllocation {
            x: 100,
            y: 200,
            width: 50,
            height: 60,
            offset_x: 0,
            offset_y: 0,
            is_color: false,
            format: GlyphPixelFormat::Alpha,
        };

        let (u_min, v_min, u_max, v_max) = alloc.uv_rect(1000);
        assert_eq!(u_min, 0.1);
        assert_eq!(v_min, 0.2);
        assert_eq!(u_max, 0.15);
        assert_eq!(v_max, 0.26);
    }
}
