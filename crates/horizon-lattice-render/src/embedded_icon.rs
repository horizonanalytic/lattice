//! Embedded icon support for compile-time icon data.
//!
//! This module provides [`EmbeddedIconData`] for icons that are compiled directly
//! into the binary using `include_bytes!`. This is useful for essential application
//! icons that should always be available regardless of the filesystem.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{EmbeddedIconData, ImageFormat};
//!
//! // Define an embedded icon at compile time
//! const SAVE_ICON: EmbeddedIconData = EmbeddedIconData::new(
//!     include_bytes!("../assets/icons/save.png"),
//!     ImageFormat::Png,
//!     "save",
//! );
//!
//! // Later, convert to an Icon when you have access to ImageManager
//! let icon = SAVE_ICON.to_icon(&mut image_manager)?;
//! ```

use crate::atlas::ImageManager;
use crate::icon::{Icon, IconSource};
use crate::image::Image;
use crate::RenderError;

/// Image format for embedded icon data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// PNG format (recommended for icons)
    Png,
    /// WebP format
    Webp,
    /// Windows ICO format
    Ico,
    /// BMP format
    Bmp,
    /// GIF format (single frame only)
    Gif,
    /// JPEG format (not recommended for icons with transparency)
    Jpeg,
    /// Unknown format - will try to auto-detect
    Unknown,
}

impl ImageFormat {
    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "webp" => ImageFormat::Webp,
            "ico" => ImageFormat::Ico,
            "bmp" => ImageFormat::Bmp,
            "gif" => ImageFormat::Gif,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            _ => ImageFormat::Unknown,
        }
    }

    /// Detect format from file magic bytes.
    pub fn from_magic_bytes(data: &[u8]) -> Self {
        if data.len() < 4 {
            return ImageFormat::Unknown;
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return ImageFormat::Png;
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return ImageFormat::Jpeg;
        }

        // GIF: GIF87a or GIF89a
        if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            return ImageFormat::Gif;
        }

        // BMP: BM
        if data.starts_with(b"BM") {
            return ImageFormat::Bmp;
        }

        // WebP: RIFF....WEBP
        if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
            return ImageFormat::Webp;
        }

        // ICO: 00 00 01 00
        if data.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return ImageFormat::Ico;
        }

        ImageFormat::Unknown
    }

    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Unknown => "application/octet-stream",
        }
    }

    /// Check if this format supports transparency.
    pub fn supports_transparency(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png | ImageFormat::Webp | ImageFormat::Ico | ImageFormat::Gif
        )
    }
}

/// Embedded icon data that is compiled into the binary.
///
/// This struct holds a reference to static icon data that can be decoded
/// at runtime into an [`Icon`]. The data itself is stored in the binary's
/// read-only data segment.
///
/// # Usage
///
/// Define embedded icons as constants:
///
/// ```ignore
/// const MY_ICON: EmbeddedIconData = EmbeddedIconData::new(
///     include_bytes!("icons/my_icon.png"),
///     ImageFormat::Png,
///     "my_icon",
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedIconData {
    /// Raw image bytes (PNG, WebP, etc.)
    data: &'static [u8],
    /// Image format hint
    format: ImageFormat,
    /// Icon name for debugging/identification
    name: &'static str,
}

impl EmbeddedIconData {
    /// Create new embedded icon data.
    ///
    /// This is a const function, allowing use in static/const contexts.
    pub const fn new(data: &'static [u8], format: ImageFormat, name: &'static str) -> Self {
        Self { data, format, name }
    }

    /// Create embedded icon data with auto-detected format.
    pub fn with_auto_format(data: &'static [u8], name: &'static str) -> Self {
        let format = ImageFormat::from_magic_bytes(data);
        Self { data, format, name }
    }

    /// Get the raw image data.
    pub const fn data(&self) -> &'static [u8] {
        self.data
    }

    /// Get the image format.
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// Get the icon name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Get the size of the embedded data in bytes.
    pub const fn size(&self) -> usize {
        self.data.len()
    }

    /// Decode the embedded data and load it into the GPU.
    ///
    /// This decodes the image data and uploads it to a texture atlas.
    pub fn load(&self, image_manager: &mut ImageManager) -> Result<Image, RenderError> {
        image_manager.load_bytes(self.data)
    }

    /// Convert to an [`Icon`] by loading the image.
    ///
    /// This is a convenience method that loads the embedded data and
    /// wraps it in an [`Icon`].
    pub fn to_icon(&self, image_manager: &mut ImageManager) -> Result<Icon, RenderError> {
        let image = self.load(image_manager)?;
        Ok(Icon::from_image(image))
    }

    /// Convert to an [`IconSource::Image`] by loading the image.
    pub fn to_icon_source(&self, image_manager: &mut ImageManager) -> Result<IconSource, RenderError> {
        let image = self.load(image_manager)?;
        Ok(IconSource::Image(image))
    }
}

/// A collection of embedded icons.
///
/// This struct allows organizing multiple embedded icons together,
/// typically for a set of related icons (e.g., all toolbar icons).
#[derive(Debug, Clone)]
pub struct EmbeddedIconSet {
    /// Map from icon name to embedded data
    icons: std::collections::HashMap<&'static str, EmbeddedIconData>,
}

impl EmbeddedIconSet {
    /// Create a new empty icon set.
    pub fn new() -> Self {
        Self {
            icons: std::collections::HashMap::new(),
        }
    }

    /// Add an embedded icon to the set.
    pub fn add(&mut self, icon: EmbeddedIconData) {
        self.icons.insert(icon.name, icon);
    }

    /// Add an embedded icon to the set (builder pattern).
    pub fn with(mut self, icon: EmbeddedIconData) -> Self {
        self.add(icon);
        self
    }

    /// Get an embedded icon by name.
    pub fn get(&self, name: &str) -> Option<&EmbeddedIconData> {
        self.icons.get(name)
    }

    /// Check if an icon exists in the set.
    pub fn contains(&self, name: &str) -> bool {
        self.icons.contains_key(name)
    }

    /// Get the number of icons in the set.
    pub fn len(&self) -> usize {
        self.icons.len()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }

    /// Iterate over all icon names.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.icons.keys().copied()
    }

    /// Iterate over all embedded icon data.
    pub fn icons(&self) -> impl Iterator<Item = &EmbeddedIconData> + '_ {
        self.icons.values()
    }

    /// Load all icons in the set and return a map of names to Images.
    pub fn load_all(
        &self,
        image_manager: &mut ImageManager,
    ) -> Result<std::collections::HashMap<&'static str, Image>, RenderError> {
        let mut loaded = std::collections::HashMap::new();
        for (name, icon) in &self.icons {
            let image = icon.load(image_manager)?;
            loaded.insert(*name, image);
        }
        Ok(loaded)
    }
}

impl Default for EmbeddedIconSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("png"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("PNG"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("jpg"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("jpeg"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("webp"), ImageFormat::Webp);
        assert_eq!(ImageFormat::from_extension("ico"), ImageFormat::Ico);
        assert_eq!(ImageFormat::from_extension("xyz"), ImageFormat::Unknown);
    }

    #[test]
    fn test_image_format_from_magic_bytes() {
        // PNG magic bytes
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::from_magic_bytes(&png_data), ImageFormat::Png);

        // JPEG magic bytes
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(ImageFormat::from_magic_bytes(&jpeg_data), ImageFormat::Jpeg);

        // GIF magic bytes
        let gif_data = b"GIF89a";
        assert_eq!(ImageFormat::from_magic_bytes(gif_data), ImageFormat::Gif);

        // Unknown/short data
        let short_data = [0x00, 0x01];
        assert_eq!(
            ImageFormat::from_magic_bytes(&short_data),
            ImageFormat::Unknown
        );
    }

    #[test]
    fn test_image_format_mime_type() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Unknown.mime_type(), "application/octet-stream");
    }

    #[test]
    fn test_image_format_transparency() {
        assert!(ImageFormat::Png.supports_transparency());
        assert!(ImageFormat::Webp.supports_transparency());
        assert!(!ImageFormat::Jpeg.supports_transparency());
        assert!(!ImageFormat::Bmp.supports_transparency());
    }

    #[test]
    fn test_embedded_icon_data_const() {
        // Test that EmbeddedIconData can be created in const context
        const TEST_DATA: &[u8] = &[0x89, 0x50, 0x4E, 0x47];
        const TEST_ICON: EmbeddedIconData =
            EmbeddedIconData::new(TEST_DATA, ImageFormat::Png, "test");

        assert_eq!(TEST_ICON.name(), "test");
        assert_eq!(TEST_ICON.format(), ImageFormat::Png);
        assert_eq!(TEST_ICON.size(), 4);
    }

    #[test]
    fn test_embedded_icon_set() {
        const ICON1: EmbeddedIconData =
            EmbeddedIconData::new(&[0x89, 0x50], ImageFormat::Png, "icon1");
        const ICON2: EmbeddedIconData =
            EmbeddedIconData::new(&[0xFF, 0xD8], ImageFormat::Jpeg, "icon2");

        let set = EmbeddedIconSet::new().with(ICON1).with(ICON2);

        assert_eq!(set.len(), 2);
        assert!(set.contains("icon1"));
        assert!(set.contains("icon2"));
        assert!(!set.contains("icon3"));

        let icon1 = set.get("icon1").unwrap();
        assert_eq!(icon1.format(), ImageFormat::Png);
    }
}
