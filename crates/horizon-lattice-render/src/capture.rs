//! Screenshot capture and texture readback utilities.
//!
//! This module provides functions for reading GPU texture data back to the CPU
//! and saving rendered content to image files.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::{
//!     GraphicsContext, GraphicsConfig, OffscreenSurface, OffscreenConfig,
//!     GpuRenderer, Renderer, Color, Rect, Size, capture,
//! };
//! use std::path::Path;
//!
//! // After rendering to an offscreen surface...
//! # fn example(surface: &OffscreenSurface) -> horizon_lattice_render::RenderResult<()> {
//! // Read pixels directly
//! let pixels = surface.read_pixels()?;
//!
//! // Or save to a file
//! surface.save_to_file("screenshot.png")?;
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use tracing::debug;

use crate::context::GraphicsContext;
use crate::error::{RenderError, RenderResult};

/// Bytes per pixel for RGBA8 format.
const BYTES_PER_PIXEL: u32 = 4;

/// Helper for calculating buffer dimensions with proper row alignment.
///
/// WebGPU requires `bytes_per_row` to be a multiple of 256 bytes for texture copies.
#[derive(Debug, Clone, Copy)]
pub struct BufferDimensions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bytes per row without padding.
    pub unpadded_bytes_per_row: u32,
    /// Bytes per row with padding (aligned to 256).
    pub padded_bytes_per_row: u32,
}

impl BufferDimensions {
    /// Create buffer dimensions for the given pixel dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let unpadded_bytes_per_row = width * BYTES_PER_PIXEL;

        // WebGPU requires bytes_per_row to be a multiple of COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }

    /// Total buffer size in bytes.
    pub fn buffer_size(&self) -> u64 {
        (self.padded_bytes_per_row * self.height) as u64
    }
}

/// Read pixel data from a texture.
///
/// Returns RGBA pixel data as a flat `Vec<u8>` in row-major order.
/// Each pixel is 4 bytes (R, G, B, A).
///
/// # Arguments
///
/// * `texture` - The GPU texture to read from.
/// * `width` - Width of the texture in pixels.
/// * `height` - Height of the texture in pixels.
///
/// # Note
///
/// This operation is synchronous and blocks until the GPU completes the transfer.
pub fn read_texture_pixels(
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> RenderResult<Vec<u8>> {
    let ctx = GraphicsContext::try_get().ok_or(RenderError::NotInitialized)?;
    let device = ctx.device();
    let queue = ctx.queue();

    let dims = BufferDimensions::new(width, height);

    // Create a buffer for reading back the texture
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture_readback_buffer"),
        size: dims.buffer_size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture to buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("texture_readback_encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(dims.padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map the buffer for reading
    let buffer_slice = output_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    // Wait for the GPU to finish
    device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|_| RenderError::Surface(wgpu::SurfaceError::Lost))?
        .map_err(|_| RenderError::Surface(wgpu::SurfaceError::Lost))?;

    // Extract pixel data, removing row padding
    let data = buffer_slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * BYTES_PER_PIXEL) as usize);

    for row in 0..height {
        let start = (row * dims.padded_bytes_per_row) as usize;
        let end = start + dims.unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&data[start..end]);
    }

    drop(data);
    output_buffer.unmap();

    debug!(
        target: "horizon_lattice_render::capture",
        width,
        height,
        pixels_len = pixels.len(),
        "read texture pixels"
    );

    Ok(pixels)
}

/// Save texture content to an image file.
///
/// Supported formats are determined by the file extension:
/// - `.png` - PNG format (recommended for screenshots)
/// - `.jpg` / `.jpeg` - JPEG format
/// - `.bmp` - BMP format
/// - `.tga` - TGA format
///
/// # Arguments
///
/// * `texture` - The GPU texture to save.
/// * `width` - Width of the texture in pixels.
/// * `height` - Height of the texture in pixels.
/// * `path` - Output file path. Extension determines format.
pub fn save_texture_to_file(
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> RenderResult<()> {
    let pixels = read_texture_pixels(texture, width, height)?;
    save_pixels_to_file(&pixels, width, height, path)
}

/// Save raw pixel data to an image file.
///
/// # Arguments
///
/// * `pixels` - RGBA pixel data in row-major order.
/// * `width` - Width in pixels.
/// * `height` - Height in pixels.
/// * `path` - Output file path.
pub fn save_pixels_to_file(
    pixels: &[u8],
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> RenderResult<()> {
    let path = path.as_ref();

    let image_buffer: image::ImageBuffer<image::Rgba<u8>, _> =
        image::ImageBuffer::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| RenderError::InvalidDimensions { width, height })?;

    image_buffer
        .save(path)
        .map_err(|e| RenderError::ImageSaveError(e.to_string()))?;

    debug!(
        target: "horizon_lattice_render::capture",
        path = %path.display(),
        width,
        height,
        "saved image to file"
    );

    Ok(())
}

/// Create an image from pixel data without saving.
///
/// Returns an `image::RgbaImage` that can be further manipulated
/// or saved in various formats.
pub fn pixels_to_image(pixels: &[u8], width: u32, height: u32) -> Option<image::RgbaImage> {
    image::ImageBuffer::from_raw(width, height, pixels.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_dimensions_no_padding() {
        // 64 pixels wide * 4 bytes = 256, which is already aligned
        let dims = BufferDimensions::new(64, 100);
        assert_eq!(dims.unpadded_bytes_per_row, 256);
        assert_eq!(dims.padded_bytes_per_row, 256);
        assert_eq!(dims.buffer_size(), 256 * 100);
    }

    #[test]
    fn test_buffer_dimensions_with_padding() {
        // 100 pixels wide * 4 bytes = 400, needs padding to 512
        let dims = BufferDimensions::new(100, 100);
        assert_eq!(dims.unpadded_bytes_per_row, 400);
        assert_eq!(dims.padded_bytes_per_row, 512);
        assert_eq!(dims.buffer_size(), 512 * 100);
    }

    #[test]
    fn test_buffer_dimensions_small() {
        // 10 pixels wide * 4 bytes = 40, needs padding to 256
        let dims = BufferDimensions::new(10, 10);
        assert_eq!(dims.unpadded_bytes_per_row, 40);
        assert_eq!(dims.padded_bytes_per_row, 256);
    }
}
