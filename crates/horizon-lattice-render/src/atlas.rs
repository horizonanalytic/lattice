//! Texture atlas management using shelf-based packing.
//!
//! This module implements a texture atlas system that efficiently packs
//! multiple images into a single GPU texture. It uses a shelf-based
//! allocation algorithm inspired by the `etagere` crate.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use std::path::Path;

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};
use crate::image::Image;
use crate::scalable_image::{scaled_path, ScalableImage};
use crate::svg::SvgImage;

/// Global atlas ID counter.
static ATLAS_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Default atlas texture size.
pub const DEFAULT_ATLAS_SIZE: u32 = 2048;

/// Minimum atlas texture size.
pub const MIN_ATLAS_SIZE: u32 = 256;

/// Maximum atlas texture size.
pub const MAX_ATLAS_SIZE: u32 = 8192;

/// Padding between allocations to prevent texture bleeding.
const ALLOCATION_PADDING: u32 = 1;

/// A shelf in the atlas (horizontal row of allocations).
#[derive(Debug)]
struct Shelf {
    /// Y position of this shelf in the atlas.
    y: u32,
    /// Height of this shelf.
    height: u32,
    /// Current X position for the next allocation.
    cursor_x: u32,
}

impl Shelf {
    fn new(y: u32, height: u32) -> Self {
        Self {
            y,
            height,
            cursor_x: 0,
        }
    }

    /// Try to allocate space in this shelf.
    fn try_allocate(&mut self, width: u32, height: u32, atlas_width: u32) -> Option<(u32, u32)> {
        // Check if allocation fits in height
        if height > self.height {
            return None;
        }

        // Check if allocation fits in remaining width
        let padded_width = width + ALLOCATION_PADDING;
        if self.cursor_x + padded_width > atlas_width {
            return None;
        }

        let x = self.cursor_x;
        self.cursor_x += padded_width;
        Some((x, self.y))
    }

    /// Get remaining width in this shelf.
    fn remaining_width(&self, atlas_width: u32) -> u32 {
        atlas_width.saturating_sub(self.cursor_x)
    }
}

/// An allocation within a texture atlas.
#[derive(Debug, Clone)]
pub struct AtlasAllocation {
    /// X position in the atlas texture.
    pub x: u32,
    /// Y position in the atlas texture.
    pub y: u32,
    /// Width of the allocation.
    pub width: u32,
    /// Height of the allocation.
    pub height: u32,
    /// Atlas texture size for UV calculation.
    pub(crate) atlas_size: u32,
}

impl AtlasAllocation {
    /// Get normalized UV coordinates for this allocation.
    ///
    /// Returns (u_min, v_min, u_max, v_max).
    pub fn uv_rect(&self) -> (f32, f32, f32, f32) {
        let size = self.atlas_size as f32;
        (
            self.x as f32 / size,
            self.y as f32 / size,
            (self.x + self.width) as f32 / size,
            (self.y + self.height) as f32 / size,
        )
    }
}

/// Internal mutable state for texture atlas allocation.
struct AtlasState {
    /// Shelves in the atlas.
    shelves: Vec<Shelf>,
    /// Y position for the next shelf.
    next_shelf_y: u32,
}

/// A texture atlas that packs multiple images into a single GPU texture.
///
/// Uses shelf-based packing for efficient allocation. Each shelf is a
/// horizontal row of allocations with the same height.
pub struct TextureAtlas {
    /// Unique ID for this atlas.
    id: usize,
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
    /// Mutable allocation state.
    state: Mutex<AtlasState>,
}

impl TextureAtlas {
    /// Create a new texture atlas with the specified size.
    pub fn new(size: u32) -> RenderResult<Self> {
        let size = size.clamp(MIN_ATLAS_SIZE, MAX_ATLAS_SIZE);
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        // Create the atlas texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture_atlas"),
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

        // Create sampler with linear filtering
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout and bind group
        let bind_group = Self::create_bind_group(device, &texture_view, &sampler);

        let id = ATLAS_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Ok(Self {
            id,
            texture,
            texture_view,
            sampler,
            bind_group,
            size,
            state: Mutex::new(AtlasState {
                shelves: Vec::new(),
                next_shelf_y: 0,
            }),
        })
    }

    /// Create with default size.
    pub fn with_default_size() -> RenderResult<Self> {
        Self::new(DEFAULT_ATLAS_SIZE)
    }

    /// Create the bind group for this atlas.
    fn create_bind_group(
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let layout = Self::bind_group_layout(device);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bind_group"),
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

    /// Get the bind group layout for texture atlases.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas_bind_group_layout"),
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

    /// Get the unique ID of this atlas.
    #[inline]
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the size of this atlas texture.
    #[inline]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Get the texture view.
    #[inline]
    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    /// Get the bind group for rendering.
    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Try to allocate space for an image in this atlas.
    pub fn try_allocate(&self, width: u32, height: u32) -> Option<AtlasAllocation> {
        let mut state = self.state.lock();

        // Add padding
        let padded_width = width + ALLOCATION_PADDING;
        let padded_height = height + ALLOCATION_PADDING;

        // Check if allocation is even possible
        if padded_width > self.size || padded_height > self.size {
            return None;
        }

        // Try existing shelves first (best fit)
        let mut best_shelf_idx = None;
        let mut best_waste = u32::MAX;

        for (idx, shelf) in state.shelves.iter().enumerate() {
            // Check if height fits
            if padded_height <= shelf.height {
                // Check if width fits
                if shelf.remaining_width(self.size) >= padded_width {
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
            if let Some((x, y)) = state.shelves[idx].try_allocate(width, height, self.size) {
                return Some(AtlasAllocation {
                    x,
                    y,
                    width,
                    height,
                    atlas_size: self.size,
                });
            }
        }

        // Create a new shelf if there's room
        if state.next_shelf_y + padded_height <= self.size {
            let mut shelf = Shelf::new(state.next_shelf_y, padded_height);
            let (x, y) = shelf.try_allocate(width, height, self.size)?;
            state.next_shelf_y += padded_height;
            state.shelves.push(shelf);
            return Some(AtlasAllocation {
                x,
                y,
                width,
                height,
                atlas_size: self.size,
            });
        }

        None
    }

    /// Upload image data to a pre-allocated region.
    pub fn upload(&self, allocation: &AtlasAllocation, data: &[u8]) -> RenderResult<()> {
        let expected_size = (allocation.width * allocation.height * 4) as usize;
        if data.len() != expected_size {
            return Err(RenderError::ImageLoad(format!(
                "Invalid image data size: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        let ctx = GraphicsContext::get();
        let queue = ctx.queue();

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: allocation.x,
                    y: allocation.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * allocation.width),
                rows_per_image: Some(allocation.height),
            },
            wgpu::Extent3d {
                width: allocation.width,
                height: allocation.height,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    /// Get the percentage of the atlas that is used.
    pub fn usage(&self) -> f32 {
        let state = self.state.lock();
        if state.shelves.is_empty() {
            return 0.0;
        }

        let total_pixels = (self.size * self.size) as f32;
        let used_pixels: f32 = state
            .shelves
            .iter()
            .map(|s| (s.cursor_x * s.height) as f32)
            .sum();

        used_pixels / total_pixels
    }
}

impl std::fmt::Debug for TextureAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextureAtlas")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("usage", &format!("{:.1}%", self.usage() * 100.0))
            .finish()
    }
}

/// Manages multiple texture atlases for efficient image storage.
///
/// Automatically creates new atlases when existing ones are full.
pub struct ImageManager {
    /// All atlases managed by this manager.
    atlases: Vec<Arc<TextureAtlas>>,
    /// Default atlas size for new atlases.
    default_atlas_size: u32,
}

impl ImageManager {
    /// Create a new image manager.
    pub fn new() -> RenderResult<Self> {
        Self::with_atlas_size(DEFAULT_ATLAS_SIZE)
    }

    /// Create a new image manager with a specific atlas size.
    pub fn with_atlas_size(size: u32) -> RenderResult<Self> {
        Ok(Self {
            atlases: Vec::new(),
            default_atlas_size: size,
        })
    }

    /// Load an image from a file path.
    pub fn load_file(&mut self, path: impl AsRef<std::path::Path>) -> RenderResult<Image> {
        let img = image::open(path.as_ref()).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to load image: {}", e))
        })?;
        self.load_dynamic_image(img)
    }

    /// Load an image with automatic @2x/@3x variant discovery.
    ///
    /// Given a base path like "icon.png", this method will:
    /// 1. Load the base image as the @1x variant
    /// 2. Look for "icon@2x.png" and load it as the @2x variant (if it exists)
    /// 3. Look for "icon@3x.png" and load it as the @3x variant (if it exists)
    ///
    /// The returned [`ScalableImage`] can then select the best variant for
    /// any given scale factor.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the base (@1x) image file
    ///
    /// # Returns
    ///
    /// A `ScalableImage` containing all found resolution variants.
    ///
    /// # Errors
    ///
    /// Returns an error if the base image cannot be loaded. Missing @2x/@3x
    /// variants are silently ignored.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Given these files:
    /// //   assets/icon.png      (32x32)
    /// //   assets/icon@2x.png   (64x64)
    /// //   assets/icon@3x.png   (96x96)
    ///
    /// let mut manager = ImageManager::new()?;
    /// let icon = manager.load_scalable("assets/icon.png")?;
    ///
    /// // On a 2x display:
    /// let image = icon.best_for_scale(2.0);  // Returns the @2x variant
    /// ```
    pub fn load_scalable(&mut self, path: impl AsRef<Path>) -> RenderResult<ScalableImage> {
        let path = path.as_ref();

        // Load the base @1x image (required)
        let image_1x = self.load_file(path)?;
        let mut scalable = ScalableImage::new(image_1x);

        // Try to load @2x variant (optional)
        if let Some(path_2x) = scaled_path(path, 2) {
            if path_2x.exists() {
                if let Ok(image_2x) = self.load_file(path_2x.as_path()) {
                    scalable.add_variant(2, image_2x);
                }
            }
        }

        // Try to load @3x variant (optional)
        if let Some(path_3x) = scaled_path(path, 3) {
            if path_3x.exists() {
                if let Ok(image_3x) = self.load_file(path_3x.as_path()) {
                    scalable.add_variant(3, image_3x);
                }
            }
        }

        Ok(scalable)
    }

    /// Load an SVG and render it at a specific pixel size.
    ///
    /// This is a convenience method that loads an SVG file and renders it
    /// immediately to a GPU texture at the specified dimensions.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SVG file
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Returns
    ///
    /// An `Image` containing the rendered SVG.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Render a 24x24 SVG at 48x48 for a 2x display
    /// let icon = manager.load_svg("icons/settings.svg", 48, 48)?;
    /// ```
    pub fn load_svg(
        &mut self,
        path: impl AsRef<Path>,
        width: u32,
        height: u32,
    ) -> RenderResult<Image> {
        let svg = SvgImage::from_file(path)?;
        svg.render_to_image(self, width, height)
    }

    /// Load an SVG and render it at its natural size scaled by a factor.
    ///
    /// This is the most common way to load SVGs for HiDPI displays. The SVG
    /// is rendered at `natural_size * scale_factor` pixels.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SVG file
    /// * `scale_factor` - The scale factor to render at (e.g., 1.0, 2.0)
    ///
    /// # Returns
    ///
    /// An `Image` containing the rendered SVG.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For a 24x24 SVG on a 2x display, renders at 48x48 pixels
    /// let scale = window.scale_factor();
    /// let icon = manager.load_svg_scaled("icons/menu.svg", scale)?;
    /// ```
    pub fn load_svg_scaled(
        &mut self,
        path: impl AsRef<Path>,
        scale_factor: f64,
    ) -> RenderResult<Image> {
        let svg = SvgImage::from_file(path)?;
        svg.render_scaled(self, scale_factor)
    }

    /// Load an image from bytes in memory.
    pub fn load_bytes(&mut self, bytes: &[u8]) -> RenderResult<Image> {
        let img = image::load_from_memory(bytes).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to decode image: {}", e))
        })?;
        self.load_dynamic_image(img)
    }

    /// Load a DynamicImage.
    fn load_dynamic_image(&mut self, img: image::DynamicImage) -> RenderResult<Image> {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        self.load_rgba(rgba.as_raw(), width, height)
    }

    /// Load raw RGBA pixel data.
    pub fn load_rgba(&mut self, data: &[u8], width: u32, height: u32) -> RenderResult<Image> {
        // Validate data size
        let expected_size = (width * height * 4) as usize;
        if data.len() != expected_size {
            return Err(RenderError::ImageLoad(format!(
                "Invalid image data size: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        // Try existing atlases first
        for atlas in &self.atlases {
            if let Some(allocation) = atlas.try_allocate(width, height) {
                atlas.upload(&allocation, data)?;
                return Ok(Image {
                    allocation,
                    atlas: atlas.clone(),
                    width,
                    height,
                });
            }
        }

        // Create a new atlas
        let new_atlas = TextureAtlas::new(self.default_atlas_size)?;
        let allocation = new_atlas.try_allocate(width, height).ok_or_else(|| {
            RenderError::ImageLoad(format!(
                "Image {}x{} is too large for atlas size {}",
                width, height, self.default_atlas_size
            ))
        })?;
        new_atlas.upload(&allocation, data)?;

        let atlas = Arc::new(new_atlas);
        let image = Image {
            allocation,
            atlas: atlas.clone(),
            width,
            height,
        };
        self.atlases.push(atlas);

        Ok(image)
    }

    /// Create a solid color image (1x1 pixel).
    pub fn create_solid_color(&mut self, color: crate::types::Color) -> RenderResult<Image> {
        let data = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];
        self.load_rgba(&data, 1, 1)
    }

    /// Get all atlases.
    pub fn atlases(&self) -> &[Arc<TextureAtlas>] {
        &self.atlases
    }

    /// Get the total number of atlases.
    pub fn atlas_count(&self) -> usize {
        self.atlases.len()
    }
}

impl Default for ImageManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ImageManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_uv() {
        let alloc = AtlasAllocation {
            x: 100,
            y: 200,
            width: 50,
            height: 60,
            atlas_size: 1000,
        };

        let (u_min, v_min, u_max, v_max) = alloc.uv_rect();
        assert_eq!(u_min, 0.1);
        assert_eq!(v_min, 0.2);
        assert_eq!(u_max, 0.15);
        assert_eq!(v_max, 0.26);
    }

    #[test]
    fn test_shelf_allocation() {
        let mut shelf = Shelf::new(0, 100);

        // First allocation should succeed
        let result = shelf.try_allocate(50, 80, 500);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), (0, 0));

        // Second allocation
        let result = shelf.try_allocate(30, 90, 500);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), (51, 0)); // 50 + 1 padding

        // Allocation that's too tall should fail
        let result = shelf.try_allocate(30, 110, 500);
        assert!(result.is_none());
    }
}
