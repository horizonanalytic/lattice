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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ImageFormat {
    /// PNG format (recommended for icons)
    Png,
    /// WebP format
    Webp,
    /// Windows ICO format
    Ico,
    /// BMP format
    Bmp,
    /// GIF format (supports animation)
    Gif,
    /// JPEG format (not recommended for icons with transparency)
    Jpeg,
    /// TIFF format
    Tiff,
    /// AVIF format (AV1 Image File Format)
    Avif,
    /// TGA (Truevision) format
    Tga,
    /// DDS (DirectDraw Surface) format
    Dds,
    /// HDR (Radiance RGBE) format
    Hdr,
    /// OpenEXR format
    OpenExr,
    /// PNM (Portable Any Map) format
    Pnm,
    /// QOI (Quite OK Image) format
    Qoi,
    /// Farbfeld format
    Farbfeld,
    /// Unknown format - will try to auto-detect
    #[default]
    Unknown,
}

impl ImageFormat {
    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "webp" => ImageFormat::Webp,
            "ico" => ImageFormat::Ico,
            "bmp" | "dib" => ImageFormat::Bmp,
            "gif" => ImageFormat::Gif,
            "jpg" | "jpeg" | "jpe" | "jif" | "jfif" => ImageFormat::Jpeg,
            "tiff" | "tif" => ImageFormat::Tiff,
            "avif" | "avifs" => ImageFormat::Avif,
            "tga" | "icb" | "vda" | "vst" => ImageFormat::Tga,
            "dds" => ImageFormat::Dds,
            "hdr" => ImageFormat::Hdr,
            "exr" => ImageFormat::OpenExr,
            "pbm" | "pgm" | "ppm" | "pam" | "pnm" => ImageFormat::Pnm,
            "qoi" => ImageFormat::Qoi,
            "ff" | "farbfeld" => ImageFormat::Farbfeld,
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

        // TIFF: II (little-endian) or MM (big-endian)
        if data.starts_with(&[0x49, 0x49, 0x2A, 0x00])
            || data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
        {
            return ImageFormat::Tiff;
        }

        // DDS: DDS
        if data.starts_with(b"DDS ") {
            return ImageFormat::Dds;
        }

        // OpenEXR: 76 2F 31 01
        if data.starts_with(&[0x76, 0x2F, 0x31, 0x01]) {
            return ImageFormat::OpenExr;
        }

        // HDR (Radiance): #?RADIANCE or #?RGBE
        if data.starts_with(b"#?RADIANCE") || data.starts_with(b"#?RGBE") {
            return ImageFormat::Hdr;
        }

        // QOI: qoif
        if data.starts_with(b"qoif") {
            return ImageFormat::Qoi;
        }

        // Farbfeld: farbfeld
        if data.starts_with(b"farbfeld") {
            return ImageFormat::Farbfeld;
        }

        // AVIF/HEIF: check for ftyp box with avif/heic brands
        if data.len() >= 12 {
            if &data[4..8] == b"ftyp" {
                let brand = &data[8..12];
                if brand == b"avif" || brand == b"avis" || brand == b"mif1" {
                    return ImageFormat::Avif;
                }
            }
        }

        // PNM formats
        if data.len() >= 2 && data[0] == b'P' {
            match data[1] {
                b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' => {
                    return ImageFormat::Pnm;
                }
                _ => {}
            }
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
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Tga => "image/x-tga",
            ImageFormat::Dds => "image/vnd-ms.dds",
            ImageFormat::Hdr => "image/vnd.radiance",
            ImageFormat::OpenExr => "image/x-exr",
            ImageFormat::Pnm => "image/x-portable-anymap",
            ImageFormat::Qoi => "image/x-qoi",
            ImageFormat::Farbfeld => "image/x-farbfeld",
            ImageFormat::Unknown => "application/octet-stream",
        }
    }

    /// Check if this format supports transparency.
    pub fn supports_transparency(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png
                | ImageFormat::Webp
                | ImageFormat::Ico
                | ImageFormat::Gif
                | ImageFormat::Tiff
                | ImageFormat::Avif
                | ImageFormat::Tga
                | ImageFormat::Dds
                | ImageFormat::OpenExr
                | ImageFormat::Qoi
                | ImageFormat::Farbfeld
        )
    }

    /// Check if this format supports animation.
    pub fn supports_animation(&self) -> bool {
        matches!(
            self,
            ImageFormat::Gif | ImageFormat::Webp | ImageFormat::Png | ImageFormat::Avif
        )
    }

    /// Check if this format is HDR (high dynamic range).
    pub fn is_hdr(&self) -> bool {
        matches!(self, ImageFormat::Hdr | ImageFormat::OpenExr)
    }

    /// Check if the image crate can decode this format.
    pub fn can_decode(&self) -> bool {
        !matches!(self, ImageFormat::Unknown)
    }

    /// Check if the image crate can encode this format.
    pub fn can_encode(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png
                | ImageFormat::Jpeg
                | ImageFormat::Gif
                | ImageFormat::Bmp
                | ImageFormat::Ico
                | ImageFormat::Tiff
                | ImageFormat::Avif
                | ImageFormat::Webp
                | ImageFormat::Tga
                | ImageFormat::Hdr
                | ImageFormat::OpenExr
                | ImageFormat::Pnm
                | ImageFormat::Qoi
                | ImageFormat::Farbfeld
        )
    }

    /// Get all common file extensions for this format.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ImageFormat::Png => &["png"],
            ImageFormat::Webp => &["webp"],
            ImageFormat::Ico => &["ico"],
            ImageFormat::Bmp => &["bmp", "dib"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe", "jif", "jfif"],
            ImageFormat::Tiff => &["tiff", "tif"],
            ImageFormat::Avif => &["avif", "avifs"],
            ImageFormat::Tga => &["tga", "icb", "vda", "vst"],
            ImageFormat::Dds => &["dds"],
            ImageFormat::Hdr => &["hdr"],
            ImageFormat::OpenExr => &["exr"],
            ImageFormat::Pnm => &["pbm", "pgm", "ppm", "pam", "pnm"],
            ImageFormat::Qoi => &["qoi"],
            ImageFormat::Farbfeld => &["ff", "farbfeld"],
            ImageFormat::Unknown => &[],
        }
    }

    /// Convert to image crate's ImageFormat.
    pub fn to_image_format(&self) -> Option<image::ImageFormat> {
        match self {
            ImageFormat::Png => Some(image::ImageFormat::Png),
            ImageFormat::Webp => Some(image::ImageFormat::WebP),
            ImageFormat::Ico => Some(image::ImageFormat::Ico),
            ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
            ImageFormat::Gif => Some(image::ImageFormat::Gif),
            ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormat::Avif => Some(image::ImageFormat::Avif),
            ImageFormat::Tga => Some(image::ImageFormat::Tga),
            ImageFormat::Dds => Some(image::ImageFormat::Dds),
            ImageFormat::Hdr => Some(image::ImageFormat::Hdr),
            ImageFormat::OpenExr => Some(image::ImageFormat::OpenExr),
            ImageFormat::Pnm => Some(image::ImageFormat::Pnm),
            ImageFormat::Qoi => Some(image::ImageFormat::Qoi),
            ImageFormat::Farbfeld => Some(image::ImageFormat::Farbfeld),
            ImageFormat::Unknown => None,
        }
    }

    /// Convert from image crate's ImageFormat.
    pub fn from_image_format(format: image::ImageFormat) -> Self {
        match format {
            image::ImageFormat::Png => ImageFormat::Png,
            image::ImageFormat::WebP => ImageFormat::Webp,
            image::ImageFormat::Ico => ImageFormat::Ico,
            image::ImageFormat::Bmp => ImageFormat::Bmp,
            image::ImageFormat::Gif => ImageFormat::Gif,
            image::ImageFormat::Jpeg => ImageFormat::Jpeg,
            image::ImageFormat::Tiff => ImageFormat::Tiff,
            image::ImageFormat::Avif => ImageFormat::Avif,
            image::ImageFormat::Tga => ImageFormat::Tga,
            image::ImageFormat::Dds => ImageFormat::Dds,
            image::ImageFormat::Hdr => ImageFormat::Hdr,
            image::ImageFormat::OpenExr => ImageFormat::OpenExr,
            image::ImageFormat::Pnm => ImageFormat::Pnm,
            image::ImageFormat::Qoi => ImageFormat::Qoi,
            image::ImageFormat::Farbfeld => ImageFormat::Farbfeld,
            _ => ImageFormat::Unknown,
        }
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
        assert_eq!(ImageFormat::from_extension("tiff"), ImageFormat::Tiff);
        assert_eq!(ImageFormat::from_extension("tif"), ImageFormat::Tiff);
        assert_eq!(ImageFormat::from_extension("avif"), ImageFormat::Avif);
        assert_eq!(ImageFormat::from_extension("tga"), ImageFormat::Tga);
        assert_eq!(ImageFormat::from_extension("dds"), ImageFormat::Dds);
        assert_eq!(ImageFormat::from_extension("hdr"), ImageFormat::Hdr);
        assert_eq!(ImageFormat::from_extension("exr"), ImageFormat::OpenExr);
        assert_eq!(ImageFormat::from_extension("qoi"), ImageFormat::Qoi);
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
        let gif87_data = b"GIF87a";
        assert_eq!(ImageFormat::from_magic_bytes(gif87_data), ImageFormat::Gif);

        // BMP magic bytes
        let bmp_data = b"BM\x00\x00";
        assert_eq!(ImageFormat::from_magic_bytes(bmp_data), ImageFormat::Bmp);

        // WebP magic bytes
        let webp_data = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(ImageFormat::from_magic_bytes(webp_data), ImageFormat::Webp);

        // TIFF magic bytes (little-endian)
        let tiff_le = [0x49, 0x49, 0x2A, 0x00];
        assert_eq!(ImageFormat::from_magic_bytes(&tiff_le), ImageFormat::Tiff);
        // TIFF magic bytes (big-endian)
        let tiff_be = [0x4D, 0x4D, 0x00, 0x2A];
        assert_eq!(ImageFormat::from_magic_bytes(&tiff_be), ImageFormat::Tiff);

        // QOI magic bytes
        let qoi_data = b"qoif";
        assert_eq!(ImageFormat::from_magic_bytes(qoi_data), ImageFormat::Qoi);

        // HDR magic bytes
        let hdr_data = b"#?RADIANCE\n";
        assert_eq!(ImageFormat::from_magic_bytes(hdr_data), ImageFormat::Hdr);

        // DDS magic bytes
        let dds_data = b"DDS ";
        assert_eq!(ImageFormat::from_magic_bytes(dds_data), ImageFormat::Dds);

        // Farbfeld magic bytes
        let ff_data = b"farbfeld";
        assert_eq!(ImageFormat::from_magic_bytes(ff_data), ImageFormat::Farbfeld);

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
        assert!(ImageFormat::Tiff.supports_transparency());
        assert!(ImageFormat::Avif.supports_transparency());
        assert!(!ImageFormat::Jpeg.supports_transparency());
        assert!(!ImageFormat::Bmp.supports_transparency());
        assert!(!ImageFormat::Hdr.supports_transparency());
    }

    #[test]
    fn test_image_format_animation() {
        assert!(ImageFormat::Gif.supports_animation());
        assert!(ImageFormat::Webp.supports_animation());
        assert!(ImageFormat::Png.supports_animation());
        assert!(ImageFormat::Avif.supports_animation());
        assert!(!ImageFormat::Jpeg.supports_animation());
        assert!(!ImageFormat::Bmp.supports_animation());
    }

    #[test]
    fn test_image_format_hdr() {
        assert!(ImageFormat::Hdr.is_hdr());
        assert!(ImageFormat::OpenExr.is_hdr());
        assert!(!ImageFormat::Png.is_hdr());
        assert!(!ImageFormat::Jpeg.is_hdr());
    }

    #[test]
    fn test_image_format_extensions() {
        assert_eq!(ImageFormat::Png.extensions(), &["png"]);
        assert_eq!(ImageFormat::Jpeg.extensions(), &["jpg", "jpeg", "jpe", "jif", "jfif"]);
        assert_eq!(ImageFormat::Tiff.extensions(), &["tiff", "tif"]);
        assert!(ImageFormat::Unknown.extensions().is_empty());
    }

    #[test]
    fn test_image_format_conversion() {
        // Round-trip conversion
        let format = ImageFormat::Png;
        let image_format = format.to_image_format().unwrap();
        let back = ImageFormat::from_image_format(image_format);
        assert_eq!(format, back);

        // Unknown has no image format
        assert!(ImageFormat::Unknown.to_image_format().is_none());
    }

    #[test]
    fn test_image_format_can_decode_encode() {
        assert!(ImageFormat::Png.can_decode());
        assert!(ImageFormat::Png.can_encode());
        assert!(ImageFormat::Jpeg.can_decode());
        assert!(ImageFormat::Jpeg.can_encode());
        assert!(ImageFormat::Dds.can_decode());
        assert!(!ImageFormat::Dds.can_encode()); // DDS is read-only
        assert!(!ImageFormat::Unknown.can_decode());
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
