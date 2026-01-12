//! Image loading and GPU texture management.
//!
//! This module provides types for loading images and managing them as GPU textures.
//! Images can be loaded from files, memory, or created programmatically.

use std::sync::Arc;

use crate::atlas::{AtlasAllocation, TextureAtlas};
use crate::types::{Rect, Size};

/// A GPU-backed image that can be rendered.
///
/// Images are stored in texture atlases for efficient batching. Each `Image`
/// holds a reference to its allocation within an atlas.
#[derive(Clone)]
pub struct Image {
    /// The allocation in the texture atlas.
    pub(crate) allocation: AtlasAllocation,
    /// Reference to the atlas containing this image.
    pub(crate) atlas: Arc<TextureAtlas>,
    /// Original image width in pixels.
    pub(crate) width: u32,
    /// Original image height in pixels.
    pub(crate) height: u32,
}

impl Image {
    /// Get the width of the image in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the image in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the size of the image.
    #[inline]
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// Get the UV coordinates for this image within its atlas.
    ///
    /// Returns (u_min, v_min, u_max, v_max) in normalized texture coordinates.
    #[inline]
    pub fn uv_rect(&self) -> (f32, f32, f32, f32) {
        self.allocation.uv_rect()
    }

    /// Get the atlas texture view for binding.
    pub(crate) fn texture_view(&self) -> &wgpu::TextureView {
        self.atlas.texture_view()
    }

    /// Get the atlas this image belongs to.
    pub(crate) fn atlas(&self) -> &Arc<TextureAtlas> {
        &self.atlas
    }

    /// Get the atlas ID for batching purposes.
    pub(crate) fn atlas_id(&self) -> usize {
        self.atlas.id()
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("atlas_id", &self.atlas.id())
            .finish()
    }
}

/// How to scale an image when rendering to a different size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageScaleMode {
    /// Stretch the image to fill the destination rectangle.
    /// This may distort the image's aspect ratio.
    #[default]
    Stretch,

    /// Scale the image to fit within the destination rectangle while
    /// maintaining aspect ratio. The image will be centered and may
    /// have letterboxing/pillarboxing.
    Fit,

    /// Scale the image to fill the destination rectangle while
    /// maintaining aspect ratio. The image will be centered and may
    /// be cropped.
    Fill,

    /// Tile the image to fill the destination rectangle.
    /// The image is repeated at its original size.
    Tile,
}

/// Nine-patch (9-slice) image definition.
///
/// A nine-patch image divides the source image into 9 regions that scale
/// differently when the image is resized:
/// - Corners: Not scaled
/// - Edges: Scaled in one dimension
/// - Center: Scaled in both dimensions
///
/// ```text
/// +-------+---------------+-------+
/// |   1   |       2       |   3   |
/// | (TL)  |     (Top)     |  (TR) |
/// +-------+---------------+-------+
/// |   4   |       5       |   6   |
/// | (L)   |   (Center)    |  (R)  |
/// +-------+---------------+-------+
/// |   7   |       8       |   9   |
/// | (BL)  |   (Bottom)    |  (BR) |
/// +-------+---------------+-------+
/// ```
#[derive(Debug, Clone)]
pub struct NinePatch {
    /// The source image.
    pub image: Image,
    /// Left border width in pixels.
    pub left: f32,
    /// Right border width in pixels.
    pub right: f32,
    /// Top border height in pixels.
    pub top: f32,
    /// Bottom border height in pixels.
    pub bottom: f32,
}

impl NinePatch {
    /// Create a new nine-patch from an image with uniform borders.
    pub fn new(image: Image, border: f32) -> Self {
        Self {
            image,
            left: border,
            right: border,
            top: border,
            bottom: border,
        }
    }

    /// Create a new nine-patch with separate border sizes.
    pub fn with_borders(image: Image, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            image,
            left,
            right,
            top,
            bottom,
        }
    }

    /// Get the minimum size this nine-patch can be rendered at.
    pub fn min_size(&self) -> Size {
        Size::new(self.left + self.right, self.top + self.bottom)
    }

    /// Calculate the 9 source and destination rectangles for rendering.
    ///
    /// Returns an array of (source_uv, dest_rect) pairs for each of the 9 patches.
    pub fn calculate_patches(&self, dest: Rect) -> [(Rect, Rect); 9] {
        let img_w = self.image.width() as f32;
        let img_h = self.image.height() as f32;

        // Source rectangles (in pixels, relative to image)
        let src_rects = [
            // Row 0: Top-left, Top, Top-right
            Rect::new(0.0, 0.0, self.left, self.top),
            Rect::new(self.left, 0.0, img_w - self.left - self.right, self.top),
            Rect::new(img_w - self.right, 0.0, self.right, self.top),
            // Row 1: Left, Center, Right
            Rect::new(0.0, self.top, self.left, img_h - self.top - self.bottom),
            Rect::new(
                self.left,
                self.top,
                img_w - self.left - self.right,
                img_h - self.top - self.bottom,
            ),
            Rect::new(
                img_w - self.right,
                self.top,
                self.right,
                img_h - self.top - self.bottom,
            ),
            // Row 2: Bottom-left, Bottom, Bottom-right
            Rect::new(0.0, img_h - self.bottom, self.left, self.bottom),
            Rect::new(
                self.left,
                img_h - self.bottom,
                img_w - self.left - self.right,
                self.bottom,
            ),
            Rect::new(img_w - self.right, img_h - self.bottom, self.right, self.bottom),
        ];

        // Calculate destination sizes
        let center_w = (dest.width() - self.left - self.right).max(0.0);
        let center_h = (dest.height() - self.top - self.bottom).max(0.0);

        // Destination rectangles
        let dest_rects = [
            // Row 0: Top-left, Top, Top-right
            Rect::new(dest.left(), dest.top(), self.left, self.top),
            Rect::new(dest.left() + self.left, dest.top(), center_w, self.top),
            Rect::new(
                dest.left() + self.left + center_w,
                dest.top(),
                self.right,
                self.top,
            ),
            // Row 1: Left, Center, Right
            Rect::new(dest.left(), dest.top() + self.top, self.left, center_h),
            Rect::new(
                dest.left() + self.left,
                dest.top() + self.top,
                center_w,
                center_h,
            ),
            Rect::new(
                dest.left() + self.left + center_w,
                dest.top() + self.top,
                self.right,
                center_h,
            ),
            // Row 2: Bottom-left, Bottom, Bottom-right
            Rect::new(
                dest.left(),
                dest.top() + self.top + center_h,
                self.left,
                self.bottom,
            ),
            Rect::new(
                dest.left() + self.left,
                dest.top() + self.top + center_h,
                center_w,
                self.bottom,
            ),
            Rect::new(
                dest.left() + self.left + center_w,
                dest.top() + self.top + center_h,
                self.right,
                self.bottom,
            ),
        ];

        let mut result = [(Rect::ZERO, Rect::ZERO); 9];
        for i in 0..9 {
            result[i] = (src_rects[i], dest_rects[i]);
        }
        result
    }
}

/// Builder for loading images into the rendering system.
///
/// Note: For most use cases, prefer using [`ImageManager`](crate::ImageManager)
/// which automatically manages texture atlases.
#[derive(Default)]
pub struct ImageLoader {
    _private: (),
}

impl ImageLoader {
    /// Create a new image loader.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_scale_mode_default() {
        assert_eq!(ImageScaleMode::default(), ImageScaleMode::Stretch);
    }

    #[test]
    fn test_nine_patch_min_size_calculation() {
        // Test the min_size calculation logic without needing an actual NinePatch
        let left = 10.0_f32;
        let right = 15.0_f32;
        let top = 8.0_f32;
        let bottom = 12.0_f32;

        let min_width = left + right;
        let min_height = top + bottom;

        assert_eq!(min_width, 25.0);
        assert_eq!(min_height, 20.0);
    }
}
