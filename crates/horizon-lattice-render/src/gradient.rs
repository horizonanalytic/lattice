//! Gradient texture atlas for multi-stop gradient support.
//!
//! This module provides a [`GradientAtlas`] that stores gradient color ramps as
//! a 2D texture. Each gradient is stored as a single row of pixels, allowing
//! efficient GPU sampling for gradients with any number of color stops.

use std::collections::HashMap;

use crate::paint::GradientStop;
use crate::types::Color;

/// Width of each gradient row in pixels.
/// 256 pixels provides smooth gradients without excessive memory use.
const GRADIENT_WIDTH: u32 = 256;

/// Maximum number of gradients that can be stored in the atlas.
const MAX_GRADIENTS: u32 = 64;

/// Unique identifier for a gradient in the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GradientId(u32);

impl GradientId {
    /// Get the V texture coordinate for this gradient (0.0 to 1.0).
    #[inline]
    pub fn tex_v(&self) -> f32 {
        (self.0 as f32 + 0.5) / MAX_GRADIENTS as f32
    }
}

/// A hash key for gradient stop configurations.
/// Used to deduplicate identical gradients.
#[derive(Debug, Clone, PartialEq)]
struct GradientKey {
    stops: Vec<(u32, u32)>, // (offset_bits, color_bits)
}

impl GradientKey {
    fn from_stops(stops: &[GradientStop]) -> Self {
        Self {
            stops: stops
                .iter()
                .map(|s| {
                    let offset_bits = s.offset.to_bits();
                    let color_bits = s.color.to_u32();
                    (offset_bits, color_bits)
                })
                .collect(),
        }
    }
}

impl std::hash::Hash for GradientKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (offset, color) in &self.stops {
            offset.hash(state);
            color.hash(state);
        }
    }
}

impl Eq for GradientKey {}

/// Atlas that stores gradient color ramps as a 2D texture.
///
/// Each gradient is stored as a horizontal row of pixels. The shader can
/// sample from this texture using the gradient's V coordinate and the
/// interpolation parameter as the U coordinate.
pub struct GradientAtlas {
    /// The GPU texture storing all gradient ramps.
    texture: wgpu::Texture,
    /// Texture view for shader sampling.
    #[allow(dead_code)]
    texture_view: wgpu::TextureView,
    /// Sampler for the gradient texture.
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    /// Bind group for the gradient texture.
    bind_group: wgpu::BindGroup,
    /// Maps gradient configurations to their atlas IDs.
    gradient_cache: HashMap<GradientKey, GradientId>,
    /// Next available row index.
    next_row: u32,
    /// CPU-side pixel data for uploading.
    pixel_data: Vec<u8>,
    /// Whether the texture needs to be re-uploaded.
    dirty: bool,
}

impl GradientAtlas {
    /// Create a new gradient atlas.
    pub fn new(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        // Create the texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gradient_atlas"),
            size: wgpu::Extent3d {
                width: GRADIENT_WIDTH,
                height: MAX_GRADIENTS,
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gradient_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gradient_bind_group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Initialize pixel data to transparent black
        let pixel_data = vec![0u8; (GRADIENT_WIDTH * MAX_GRADIENTS * 4) as usize];

        Self {
            texture,
            texture_view,
            sampler,
            bind_group,
            gradient_cache: HashMap::new(),
            next_row: 0,
            pixel_data,
            dirty: false,
        }
    }

    /// Get or create a gradient ID for the given stops.
    ///
    /// Returns `None` if the atlas is full and the gradient is not already cached.
    ///
    /// # Panics
    ///
    /// Panics if any `GradientStop` has a NaN offset value, as NaN values
    /// cannot be compared for sorting.
    pub fn get_or_create(&mut self, stops: &[GradientStop]) -> Option<GradientId> {
        let key = GradientKey::from_stops(stops);

        // Check if already cached
        if let Some(&id) = self.gradient_cache.get(&key) {
            return Some(id);
        }

        // Check if we have space
        if self.next_row >= MAX_GRADIENTS {
            return None;
        }

        // Allocate new row
        let id = GradientId(self.next_row);
        self.next_row += 1;

        // Rasterize the gradient to the pixel data
        self.rasterize_gradient(id, stops);

        // Cache it
        self.gradient_cache.insert(key, id);
        self.dirty = true;

        Some(id)
    }

    /// Rasterize a gradient to the pixel data buffer.
    fn rasterize_gradient(&mut self, id: GradientId, stops: &[GradientStop]) {
        let row_offset = (id.0 * GRADIENT_WIDTH * 4) as usize;

        // Handle edge cases
        if stops.is_empty() {
            // Fill with black
            for x in 0..GRADIENT_WIDTH as usize {
                let offset = row_offset + x * 4;
                self.pixel_data[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
            return;
        }

        if stops.len() == 1 {
            // Fill with single color
            let color = stops[0].color.to_rgba8();
            for x in 0..GRADIENT_WIDTH as usize {
                let offset = row_offset + x * 4;
                self.pixel_data[offset..offset + 4].copy_from_slice(&color);
            }
            return;
        }

        // Sort stops by offset
        let mut sorted_stops: Vec<_> = stops.to_vec();
        sorted_stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());

        // Rasterize each pixel
        for x in 0..GRADIENT_WIDTH {
            let t = x as f32 / (GRADIENT_WIDTH - 1) as f32;
            let color = Self::sample_gradient(&sorted_stops, t);
            let rgba = color.to_rgba8();
            let offset = row_offset + (x as usize) * 4;
            self.pixel_data[offset..offset + 4].copy_from_slice(&rgba);
        }
    }

    /// Sample a gradient at position t (0.0 to 1.0).
    fn sample_gradient(stops: &[GradientStop], t: f32) -> Color {
        // Clamp t to valid range
        let t = t.clamp(0.0, 1.0);

        // Find the two stops that bracket t
        let mut prev_stop = &stops[0];
        let mut next_stop = &stops[stops.len() - 1];

        for i in 0..stops.len() - 1 {
            if stops[i].offset <= t && stops[i + 1].offset >= t {
                prev_stop = &stops[i];
                next_stop = &stops[i + 1];
                break;
            }
        }

        // Handle case where t is before first stop
        if t <= prev_stop.offset {
            return prev_stop.color;
        }

        // Handle case where t is after last stop
        if t >= next_stop.offset {
            return next_stop.color;
        }

        // Interpolate between the two stops
        let range = next_stop.offset - prev_stop.offset;
        if range < 0.0001 {
            return prev_stop.color;
        }

        let factor = (t - prev_stop.offset) / range;
        prev_stop.color.lerp(next_stop.color, factor)
    }

    /// Upload any dirty data to the GPU.
    pub fn upload(&mut self, queue: &wgpu::Queue) {
        if !self.dirty {
            return;
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixel_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(GRADIENT_WIDTH * 4),
                rows_per_image: Some(MAX_GRADIENTS),
            },
            wgpu::Extent3d {
                width: GRADIENT_WIDTH,
                height: MAX_GRADIENTS,
                depth_or_array_layers: 1,
            },
        );

        self.dirty = false;
    }

    /// Get the bind group for shader use.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the texture view.
    #[allow(dead_code)]
    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    /// Clear the atlas for a new frame.
    ///
    /// This resets the cache but keeps the allocated texture.
    /// Call this at the start of each frame to allow gradient reuse.
    pub fn clear(&mut self) {
        self.gradient_cache.clear();
        self.next_row = 0;
        // Don't clear pixel_data - it will be overwritten as needed
    }

    /// Get the number of gradients currently stored.
    #[allow(dead_code)]
    pub fn gradient_count(&self) -> u32 {
        self.next_row
    }

    /// Check if the atlas is full.
    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        self.next_row >= MAX_GRADIENTS
    }
}

/// Create the bind group layout for gradient textures.
pub fn create_gradient_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gradient_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_id_tex_v() {
        let id0 = GradientId(0);
        let id1 = GradientId(1);
        let id63 = GradientId(63);

        // First row should be near 0
        assert!(id0.tex_v() > 0.0 && id0.tex_v() < 0.02);
        // Second row should be offset
        assert!(id1.tex_v() > id0.tex_v());
        // Last row should be near 1
        assert!(id63.tex_v() > 0.98 && id63.tex_v() < 1.0);
    }

    #[test]
    fn test_gradient_key_equality() {
        let stops1 = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(1.0, Color::BLUE),
        ];
        let stops2 = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(1.0, Color::BLUE),
        ];
        let stops3 = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(0.5, Color::GREEN),
            GradientStop::new(1.0, Color::BLUE),
        ];

        let key1 = GradientKey::from_stops(&stops1);
        let key2 = GradientKey::from_stops(&stops2);
        let key3 = GradientKey::from_stops(&stops3);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_sample_gradient_two_stops() {
        let stops = vec![
            GradientStop::new(0.0, Color::BLACK),
            GradientStop::new(1.0, Color::WHITE),
        ];

        let at_0 = GradientAtlas::sample_gradient(&stops, 0.0);
        let at_1 = GradientAtlas::sample_gradient(&stops, 1.0);
        let at_half = GradientAtlas::sample_gradient(&stops, 0.5);

        assert_eq!(at_0, Color::BLACK);
        assert_eq!(at_1, Color::WHITE);
        // Middle should be gray
        assert!((at_half.r - 0.5).abs() < 0.02);
    }

    #[test]
    fn test_sample_gradient_multi_stops() {
        let stops = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(0.5, Color::GREEN),
            GradientStop::new(1.0, Color::BLUE),
        ];

        let at_0 = GradientAtlas::sample_gradient(&stops, 0.0);
        let at_half = GradientAtlas::sample_gradient(&stops, 0.5);
        let at_1 = GradientAtlas::sample_gradient(&stops, 1.0);

        assert_eq!(at_0, Color::RED);
        assert_eq!(at_half, Color::GREEN);
        assert_eq!(at_1, Color::BLUE);
    }

    #[test]
    fn test_sample_gradient_uneven_stops() {
        let stops = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(0.25, Color::GREEN),
            GradientStop::new(1.0, Color::BLUE),
        ];

        // At 0.25, should be exactly green
        let at_quarter = GradientAtlas::sample_gradient(&stops, 0.25);
        assert_eq!(at_quarter, Color::GREEN);
    }
}
