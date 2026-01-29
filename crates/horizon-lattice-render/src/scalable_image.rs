//! Multi-resolution image support for HiDPI displays.
//!
//! This module provides [`ScalableImage`], which holds multiple resolution
//! variants of the same image (e.g., @1x, @2x, @3x) and automatically selects
//! the best variant for a given scale factor.
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice_render::{ImageManager, ScalableImage};
//!
//! let mut manager = ImageManager::new()?;
//!
//! // Load with automatic @2x/@3x variant discovery
//! let icon = manager.load_scalable("icons/menu.png")?;
//! // This will also look for "icons/menu@2x.png" and "icons/menu@3x.png"
//!
//! // Get the best variant for the current scale factor
//! let scale_factor = window.scale_factor();
//! let image = icon.best_for_scale(scale_factor);
//!
//! // Render at logical size
//! let logical_size = icon.logical_size();
//! renderer.draw_image(image, Rect::from_size(logical_size));
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use crate::image::Image;
use crate::types::Size;

/// An image with multiple resolution variants for HiDPI support.
///
/// `ScalableImage` holds @1x, @2x, and @3x variants of the same image and
/// automatically selects the best variant based on the target scale factor.
///
/// # Resolution Variants
///
/// - **@1x**: Base resolution, used for standard DPI displays (scale factor 1.0)
/// - **@2x**: Double resolution, used for Retina/HiDPI displays (scale factor 2.0)
/// - **@3x**: Triple resolution, used for very high DPI displays (scale factor 3.0)
///
/// # File Naming Convention
///
/// When loading images with [`ImageManager::load_scalable()`], the following
/// naming convention is used:
///
/// - `icon.png` - Base @1x image
/// - `icon@2x.png` - @2x variant
/// - `icon@3x.png` - @3x variant
///
/// # Variant Selection
///
/// The [`best_for_scale()`](ScalableImage::best_for_scale) method selects the
/// optimal variant using these rules:
///
/// 1. For scale factors ≤ 1.0, use @1x
/// 2. For scale factors ≤ 2.0, use @2x if available, otherwise @1x
/// 3. For scale factors > 2.0, use @3x if available, otherwise @2x, otherwise @1x
///
/// This ensures crisp rendering without wasting memory on unnecessary high-resolution
/// textures.
#[derive(Clone)]
pub struct ScalableImage {
    /// Resolution variants: scale multiplier (1, 2, 3) -> Image
    variants: BTreeMap<u8, Image>,
    /// Logical width at 1x scale (in logical pixels)
    base_width: u32,
    /// Logical height at 1x scale (in logical pixels)
    base_height: u32,
}

impl ScalableImage {
    /// Create a new ScalableImage from a @1x base image.
    ///
    /// Additional variants can be added with [`with_variant()`](Self::with_variant).
    ///
    /// # Arguments
    ///
    /// * `image_1x` - The base @1x resolution image
    ///
    /// # Example
    ///
    /// ```ignore
    /// let base_image = manager.load_file("icon.png")?;
    /// let scalable = ScalableImage::new(base_image);
    /// ```
    pub fn new(image_1x: Image) -> Self {
        let base_width = image_1x.width();
        let base_height = image_1x.height();

        let mut variants = BTreeMap::new();
        variants.insert(1, image_1x);

        Self {
            variants,
            base_width,
            base_height,
        }
    }

    /// Create a new ScalableImage from a @2x image.
    ///
    /// The logical size is calculated by halving the @2x dimensions.
    /// Use this when you only have a @2x asset and want to use it as the base.
    ///
    /// # Arguments
    ///
    /// * `image_2x` - The @2x resolution image
    pub fn from_2x(image_2x: Image) -> Self {
        let base_width = image_2x.width() / 2;
        let base_height = image_2x.height() / 2;

        let mut variants = BTreeMap::new();
        variants.insert(2, image_2x);

        Self {
            variants,
            base_width,
            base_height,
        }
    }

    /// Add a resolution variant to this scalable image.
    ///
    /// # Arguments
    ///
    /// * `scale` - The scale multiplier (1, 2, or 3)
    /// * `image` - The image at that scale
    ///
    /// # Panics
    ///
    /// Panics if scale is 0 or greater than 3.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let base = manager.load_file("icon.png")?;
    /// let retina = manager.load_file("icon@2x.png")?;
    ///
    /// let scalable = ScalableImage::new(base)
    ///     .with_variant(2, retina);
    /// ```
    pub fn with_variant(mut self, scale: u8, image: Image) -> Self {
        assert!(
            scale > 0 && scale <= 3,
            "Scale must be 1, 2, or 3 (got {})",
            scale
        );
        self.variants.insert(scale, image);
        self
    }

    /// Add a resolution variant to this scalable image (mutable version).
    ///
    /// # Arguments
    ///
    /// * `scale` - The scale multiplier (1, 2, or 3)
    /// * `image` - The image at that scale
    ///
    /// # Panics
    ///
    /// Panics if scale is 0 or greater than 3.
    pub fn add_variant(&mut self, scale: u8, image: Image) {
        assert!(
            scale > 0 && scale <= 3,
            "Scale must be 1, 2, or 3 (got {})",
            scale
        );
        self.variants.insert(scale, image);
    }

    /// Get the best image variant for a given scale factor.
    ///
    /// This selects the highest resolution variant that doesn't exceed
    /// what's needed for the given scale factor, with fallback to
    /// lower resolutions if the ideal variant isn't available.
    ///
    /// # Arguments
    ///
    /// * `scale_factor` - The target scale factor (e.g., 1.0, 2.0, 1.5)
    ///
    /// # Returns
    ///
    /// Reference to the best matching image variant.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let scale = window.scale_factor();
    /// let image = scalable.best_for_scale(scale);
    /// renderer.draw_image(image, rect);
    /// ```
    pub fn best_for_scale(&self, scale_factor: f64) -> &Image {
        // Determine ideal scale (round up for fractional scales like 1.5)
        let ideal_scale = if scale_factor <= 1.0 {
            1
        } else if scale_factor <= 2.0 {
            2
        } else {
            3
        };

        // Find the best available variant (prefer higher if available, fall back to lower)
        for scale in (1..=ideal_scale).rev() {
            if let Some(image) = self.variants.get(&scale) {
                return image;
            }
        }

        // Should never happen since we always have at least one variant
        self.variants
            .values()
            .next()
            .expect("ScalableImage has no variants")
    }

    /// Get a specific scale variant if it exists.
    ///
    /// # Arguments
    ///
    /// * `scale` - The scale multiplier (1, 2, or 3)
    ///
    /// # Returns
    ///
    /// `Some(&Image)` if the variant exists, `None` otherwise.
    pub fn get_variant(&self, scale: u8) -> Option<&Image> {
        self.variants.get(&scale)
    }

    /// Check if a specific scale variant is available.
    pub fn has_variant(&self, scale: u8) -> bool {
        self.variants.contains_key(&scale)
    }

    /// Get all available scale variants.
    ///
    /// Returns a slice of scale factors (e.g., [1, 2] for @1x and @2x).
    pub fn available_scales(&self) -> Vec<u8> {
        self.variants.keys().copied().collect()
    }

    /// Get the logical size of the image.
    ///
    /// This is the size the image should be drawn at in logical (UI) coordinates,
    /// regardless of which resolution variant is used for rendering.
    pub fn logical_size(&self) -> Size {
        Size::new(self.base_width as f32, self.base_height as f32)
    }

    /// Get the logical width of the image.
    pub fn logical_width(&self) -> u32 {
        self.base_width
    }

    /// Get the logical height of the image.
    pub fn logical_height(&self) -> u32 {
        self.base_height
    }

    /// Get the @1x (base) variant if available.
    pub fn image_1x(&self) -> Option<&Image> {
        self.variants.get(&1)
    }

    /// Get the @2x variant if available.
    pub fn image_2x(&self) -> Option<&Image> {
        self.variants.get(&2)
    }

    /// Get the @3x variant if available.
    pub fn image_3x(&self) -> Option<&Image> {
        self.variants.get(&3)
    }
}

impl std::fmt::Debug for ScalableImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalableImage")
            .field(
                "logical_size",
                &format!("{}x{}", self.base_width, self.base_height),
            )
            .field("variants", &self.available_scales())
            .finish()
    }
}

/// Helper to construct the path for a scaled variant.
///
/// Given "icon.png" and scale 2, returns "icon@2x.png".
pub(crate) fn scaled_path(path: &Path, scale: u8) -> Option<std::path::PathBuf> {
    if scale == 1 {
        return Some(path.to_path_buf());
    }

    let stem = path.file_stem()?.to_str()?;
    let extension = path.extension()?.to_str()?;
    let parent = path.parent();

    let filename = format!("{}@{}x.{}", stem, scale, extension);

    match parent {
        Some(p) if p.as_os_str().is_empty() => Some(std::path::PathBuf::from(filename)),
        Some(p) => Some(p.join(filename)),
        None => Some(std::path::PathBuf::from(filename)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaled_path() {
        // Simple filename
        assert_eq!(
            scaled_path(Path::new("icon.png"), 2),
            Some(std::path::PathBuf::from("icon@2x.png"))
        );
        assert_eq!(
            scaled_path(Path::new("icon.png"), 3),
            Some(std::path::PathBuf::from("icon@3x.png"))
        );
        assert_eq!(
            scaled_path(Path::new("icon.png"), 1),
            Some(std::path::PathBuf::from("icon.png"))
        );

        // With directory
        assert_eq!(
            scaled_path(Path::new("assets/icons/menu.png"), 2),
            Some(std::path::PathBuf::from("assets/icons/menu@2x.png"))
        );

        // Different extension
        assert_eq!(
            scaled_path(Path::new("logo.jpg"), 2),
            Some(std::path::PathBuf::from("logo@2x.jpg"))
        );
    }

    #[test]
    fn test_scaled_path_no_extension() {
        // Files without extension return None
        assert_eq!(scaled_path(Path::new("icon"), 2), None);
    }
}
