//! Image metadata and data structures for image loading.
//!
//! This module provides types for representing image metadata including
//! dimensions, format, color type, and optionally EXIF data.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::image_data::{ImageMetadata, read_metadata};
//!
//! // Read metadata without fully decoding the image
//! let metadata = read_metadata("photo.jpg")?;
//! println!("Image: {}x{}", metadata.width, metadata.height);
//! println!("Format: {:?}", metadata.format);
//! println!("Color: {:?}", metadata.color_type);
//!
//! # Ok::<(), horizon_lattice_render::RenderError>(())
//! ```

use std::io::{BufRead, BufReader, Cursor, Read, Seek};
use std::path::Path;

use crate::embedded_icon::ImageFormat;
use crate::error::{RenderError, RenderResult};

/// Color type of an image, representing pixel format and bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// 8-bit grayscale (L8)
    L8,
    /// 8-bit grayscale with alpha (LA8)
    La8,
    /// 24-bit RGB (8 bits per channel)
    Rgb8,
    /// 32-bit RGBA (8 bits per channel)
    Rgba8,
    /// 16-bit grayscale (L16)
    L16,
    /// 16-bit grayscale with alpha (LA16)
    La16,
    /// 48-bit RGB (16 bits per channel)
    Rgb16,
    /// 64-bit RGBA (16 bits per channel)
    Rgba16,
    /// 96-bit RGB (32-bit float per channel, HDR)
    Rgb32F,
    /// 128-bit RGBA (32-bit float per channel, HDR)
    Rgba32F,
    /// Unknown color type
    Unknown,
}

impl ColorType {
    /// Get the number of bits per pixel.
    pub fn bits_per_pixel(&self) -> u32 {
        match self {
            ColorType::L8 => 8,
            ColorType::La8 => 16,
            ColorType::Rgb8 => 24,
            ColorType::Rgba8 => 32,
            ColorType::L16 => 16,
            ColorType::La16 => 32,
            ColorType::Rgb16 => 48,
            ColorType::Rgba16 => 64,
            ColorType::Rgb32F => 96,
            ColorType::Rgba32F => 128,
            ColorType::Unknown => 0,
        }
    }

    /// Get the number of bytes per pixel.
    pub fn bytes_per_pixel(&self) -> u32 {
        self.bits_per_pixel() / 8
    }

    /// Get the number of channels.
    pub fn channels(&self) -> u32 {
        match self {
            ColorType::L8 | ColorType::L16 => 1,
            ColorType::La8 | ColorType::La16 => 2,
            ColorType::Rgb8 | ColorType::Rgb16 | ColorType::Rgb32F => 3,
            ColorType::Rgba8 | ColorType::Rgba16 | ColorType::Rgba32F => 4,
            ColorType::Unknown => 0,
        }
    }

    /// Returns true if this color type has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            ColorType::La8
                | ColorType::Rgba8
                | ColorType::La16
                | ColorType::Rgba16
                | ColorType::Rgba32F
        )
    }

    /// Returns true if this is an HDR (floating point) format.
    pub fn is_hdr(&self) -> bool {
        matches!(self, ColorType::Rgb32F | ColorType::Rgba32F)
    }

    /// Returns true if this is a 16-bit format.
    pub fn is_16bit(&self) -> bool {
        matches!(
            self,
            ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16
        )
    }
}

impl From<image::ColorType> for ColorType {
    fn from(ct: image::ColorType) -> Self {
        match ct {
            image::ColorType::L8 => ColorType::L8,
            image::ColorType::La8 => ColorType::La8,
            image::ColorType::Rgb8 => ColorType::Rgb8,
            image::ColorType::Rgba8 => ColorType::Rgba8,
            image::ColorType::L16 => ColorType::L16,
            image::ColorType::La16 => ColorType::La16,
            image::ColorType::Rgb16 => ColorType::Rgb16,
            image::ColorType::Rgba16 => ColorType::Rgba16,
            image::ColorType::Rgb32F => ColorType::Rgb32F,
            image::ColorType::Rgba32F => ColorType::Rgba32F,
            _ => ColorType::Unknown,
        }
    }
}

/// EXIF orientation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    /// Normal orientation (1)
    #[default]
    Normal,
    /// Flipped horizontally (2)
    FlipHorizontal,
    /// Rotated 180 degrees (3)
    Rotate180,
    /// Flipped vertically (4)
    FlipVertical,
    /// Rotated 90 CW then flipped horizontally (5)
    Rotate90FlipH,
    /// Rotated 90 degrees clockwise (6)
    Rotate90,
    /// Rotated 90 CCW then flipped horizontally (7)
    Rotate270FlipH,
    /// Rotated 90 degrees counter-clockwise (270 CW) (8)
    Rotate270,
}

impl Orientation {
    /// Create from EXIF orientation value (1-8).
    pub fn from_exif(value: u32) -> Self {
        match value {
            1 => Orientation::Normal,
            2 => Orientation::FlipHorizontal,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipVertical,
            5 => Orientation::Rotate90FlipH,
            6 => Orientation::Rotate90,
            7 => Orientation::Rotate270FlipH,
            8 => Orientation::Rotate270,
            _ => Orientation::Normal,
        }
    }

    /// Returns true if this orientation requires dimension swap.
    pub fn swaps_dimensions(&self) -> bool {
        matches!(
            self,
            Orientation::Rotate90FlipH
                | Orientation::Rotate90
                | Orientation::Rotate270FlipH
                | Orientation::Rotate270
        )
    }
}

/// EXIF metadata extracted from an image.
#[derive(Debug, Clone, Default)]
pub struct ExifData {
    /// Camera make (manufacturer)
    pub make: Option<String>,
    /// Camera model
    pub model: Option<String>,
    /// Image orientation
    pub orientation: Orientation,
    /// Date/time the image was taken (as string)
    pub date_time: Option<String>,
    /// Exposure time in seconds (e.g., "1/125")
    pub exposure_time: Option<String>,
    /// F-number (aperture)
    pub f_number: Option<f64>,
    /// ISO speed
    pub iso: Option<u32>,
    /// Focal length in mm
    pub focal_length: Option<f64>,
    /// GPS latitude (decimal degrees, positive = North)
    pub gps_latitude: Option<f64>,
    /// GPS longitude (decimal degrees, positive = East)
    pub gps_longitude: Option<f64>,
    /// GPS altitude in meters
    pub gps_altitude: Option<f64>,
    /// Image width from EXIF (may differ from actual)
    pub exif_width: Option<u32>,
    /// Image height from EXIF (may differ from actual)
    pub exif_height: Option<u32>,
    /// Software used to create the image
    pub software: Option<String>,
    /// Image description
    pub description: Option<String>,
    /// Artist/author
    pub artist: Option<String>,
    /// Copyright notice
    pub copyright: Option<String>,
}

impl ExifData {
    /// Returns true if this EXIF data has GPS coordinates.
    pub fn has_gps(&self) -> bool {
        self.gps_latitude.is_some() && self.gps_longitude.is_some()
    }

    /// Get GPS coordinates as (latitude, longitude) if available.
    pub fn gps_coordinates(&self) -> Option<(f64, f64)> {
        match (self.gps_latitude, self.gps_longitude) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        }
    }
}

/// Metadata about an image.
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Detected image format.
    pub format: ImageFormat,
    /// Color type (pixel format).
    pub color_type: ColorType,
    /// Whether the image has an alpha channel.
    pub has_alpha: bool,
    /// EXIF data if available (JPEG, TIFF).
    pub exif: Option<ExifData>,
}

impl ImageMetadata {
    /// Get the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Get the total number of pixels.
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Estimate the uncompressed size in bytes (for RGBA8).
    pub fn estimated_rgba_size(&self) -> u64 {
        self.pixel_count() * 4
    }

    /// Get the corrected dimensions based on EXIF orientation.
    /// Returns (width, height) after applying orientation correction.
    pub fn corrected_dimensions(&self) -> (u32, u32) {
        if let Some(ref exif) = self.exif {
            if exif.orientation.swaps_dimensions() {
                return (self.height, self.width);
            }
        }
        (self.width, self.height)
    }
}

/// Read image dimensions quickly without decoding the full image.
///
/// This is much faster than loading the entire image when you only need
/// the dimensions.
///
/// # Arguments
///
/// * `path` - Path to the image file
///
/// # Returns
///
/// A tuple of (width, height) in pixels.
pub fn read_dimensions(path: impl AsRef<Path>) -> RenderResult<(u32, u32)> {
    image::image_dimensions(path.as_ref())
        .map_err(|e| RenderError::ImageLoad(format!("Failed to read image dimensions: {}", e)))
}

/// Read image dimensions from bytes without decoding the full image.
///
/// # Arguments
///
/// * `bytes` - Raw image data
///
/// # Returns
///
/// A tuple of (width, height) in pixels.
pub fn read_dimensions_from_bytes(bytes: &[u8]) -> RenderResult<(u32, u32)> {
    let cursor = Cursor::new(bytes);
    let reader = image::ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| RenderError::ImageLoad(format!("Failed to detect image format: {}", e)))?;

    // Try to get dimensions without full decode
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| RenderError::ImageLoad(format!("Failed to read dimensions: {}", e)))?;

    Ok((width, height))
}

/// Read full image metadata from a file.
///
/// This reads the image header to extract dimensions, format, and color type.
/// It also attempts to read EXIF data if present (JPEG, TIFF).
///
/// # Arguments
///
/// * `path` - Path to the image file
///
/// # Returns
///
/// Complete [`ImageMetadata`] including EXIF if available.
pub fn read_metadata(path: impl AsRef<Path>) -> RenderResult<ImageMetadata> {
    let path = path.as_ref();

    // Detect format from extension and magic bytes
    let format = detect_format(path)?;

    // Read the file
    let file = std::fs::File::open(path)
        .map_err(|e| RenderError::ImageLoad(format!("Failed to open file: {}", e)))?;
    let mut reader = BufReader::new(file);

    read_metadata_from_reader(&mut reader, format)
}

/// Read full image metadata from bytes.
///
/// # Arguments
///
/// * `bytes` - Raw image data
///
/// # Returns
///
/// Complete [`ImageMetadata`] including EXIF if available.
pub fn read_metadata_from_bytes(bytes: &[u8]) -> RenderResult<ImageMetadata> {
    let format = ImageFormat::from_magic_bytes(bytes);
    let mut cursor = Cursor::new(bytes);
    read_metadata_from_reader(&mut cursor, format)
}

/// Read metadata from a reader.
fn read_metadata_from_reader<R: BufRead + Seek>(
    reader: &mut R,
    format: ImageFormat,
) -> RenderResult<ImageMetadata> {
    // Read bytes for EXIF parsing
    let start_pos = reader
        .stream_position()
        .map_err(|e| RenderError::ImageLoad(format!("Seek error: {}", e)))?;

    // Read enough bytes for EXIF (usually in first 64KB)
    let mut header_bytes = vec![0u8; 65536];
    let bytes_read = reader
        .read(&mut header_bytes)
        .map_err(|e| RenderError::ImageLoad(format!("Read error: {}", e)))?;
    header_bytes.truncate(bytes_read);

    // Try to parse EXIF
    let exif = parse_exif(&header_bytes);

    // Reset reader position
    reader
        .seek(std::io::SeekFrom::Start(start_pos))
        .map_err(|e| RenderError::ImageLoad(format!("Seek error: {}", e)))?;

    // Use image crate to get dimensions and color type
    let img_reader = image::ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|e| RenderError::ImageLoad(format!("Failed to detect format: {}", e)))?;

    // Get dimensions without full decode
    let (width, height) = img_reader
        .into_dimensions()
        .map_err(|e| RenderError::ImageLoad(format!("Failed to read dimensions: {}", e)))?;

    // Determine color type from format (rough approximation without full decode)
    let (color_type, has_alpha) = estimate_color_type(format);

    Ok(ImageMetadata {
        width,
        height,
        format,
        color_type,
        has_alpha,
        exif,
    })
}

/// Parse EXIF data from image bytes.
fn parse_exif(bytes: &[u8]) -> Option<ExifData> {
    let exif_reader = exif::Reader::new();
    let mut cursor = Cursor::new(bytes);

    let exif_data = exif_reader.read_from_container(&mut cursor).ok()?;

    let mut data = ExifData::default();

    // Helper to get string field
    let get_string = |tag: exif::Tag| -> Option<String> {
        exif_data
            .get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif_data).to_string())
    };

    // Helper to get u32 field
    let get_u32 = |tag: exif::Tag| -> Option<u32> {
        exif_data.get_field(tag, exif::In::PRIMARY).and_then(|f| {
            if let exif::Value::Long(ref v) = f.value {
                v.first().copied()
            } else if let exif::Value::Short(ref v) = f.value {
                v.first().map(|&x| x as u32)
            } else {
                None
            }
        })
    };

    // Helper to get rational as f64
    let get_rational = |tag: exif::Tag| -> Option<f64> {
        exif_data.get_field(tag, exif::In::PRIMARY).and_then(|f| {
            if let exif::Value::Rational(ref v) = f.value {
                v.first().map(|r| r.num as f64 / r.denom as f64)
            } else {
                None
            }
        })
    };

    // Basic info
    data.make = get_string(exif::Tag::Make);
    data.model = get_string(exif::Tag::Model);
    data.software = get_string(exif::Tag::Software);
    data.date_time = get_string(exif::Tag::DateTime);
    data.description = get_string(exif::Tag::ImageDescription);
    data.artist = get_string(exif::Tag::Artist);
    data.copyright = get_string(exif::Tag::Copyright);

    // Orientation
    if let Some(orient) = get_u32(exif::Tag::Orientation) {
        data.orientation = Orientation::from_exif(orient);
    }

    // Camera settings
    data.exposure_time = get_string(exif::Tag::ExposureTime);
    data.f_number = get_rational(exif::Tag::FNumber);
    data.iso = get_u32(exif::Tag::PhotographicSensitivity);
    data.focal_length = get_rational(exif::Tag::FocalLength);

    // Dimensions from EXIF
    data.exif_width = get_u32(exif::Tag::ImageWidth)
        .or_else(|| get_u32(exif::Tag::PixelXDimension));
    data.exif_height = get_u32(exif::Tag::ImageLength)
        .or_else(|| get_u32(exif::Tag::PixelYDimension));

    // GPS coordinates
    data.gps_latitude = parse_gps_coordinate(&exif_data, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
    data.gps_longitude = parse_gps_coordinate(&exif_data, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);
    data.gps_altitude = get_rational(exif::Tag::GPSAltitude);

    Some(data)
}

/// Parse a GPS coordinate from EXIF data.
fn parse_gps_coordinate(
    exif: &exif::Exif,
    coord_tag: exif::Tag,
    ref_tag: exif::Tag,
) -> Option<f64> {
    let coord_field = exif.get_field(coord_tag, exif::In::PRIMARY)?;
    let ref_field = exif.get_field(ref_tag, exif::In::PRIMARY)?;

    // Get the reference (N/S or E/W)
    let ref_str = ref_field.display_value().to_string();
    let is_negative = ref_str.starts_with('S') || ref_str.starts_with('W');

    // Parse degrees, minutes, seconds
    if let exif::Value::Rational(ref rationals) = coord_field.value {
        if rationals.len() >= 3 {
            let degrees = rationals[0].num as f64 / rationals[0].denom as f64;
            let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
            let seconds = rationals[2].num as f64 / rationals[2].denom as f64;

            let decimal = degrees + minutes / 60.0 + seconds / 3600.0;
            return Some(if is_negative { -decimal } else { decimal });
        }
    }

    None
}

/// Detect image format from file path.
fn detect_format(path: &Path) -> RenderResult<ImageFormat> {
    // Try extension first
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let format = ImageFormat::from_extension(ext);
        if format != ImageFormat::Unknown {
            return Ok(format);
        }
    }

    // Try magic bytes
    let mut file = std::fs::File::open(path)
        .map_err(|e| RenderError::ImageLoad(format!("Failed to open file: {}", e)))?;
    let mut header = [0u8; 16];
    let bytes_read = file
        .read(&mut header)
        .map_err(|e| RenderError::ImageLoad(format!("Failed to read file: {}", e)))?;

    Ok(ImageFormat::from_magic_bytes(&header[..bytes_read]))
}

/// Estimate color type from image format (without full decode).
fn estimate_color_type(format: ImageFormat) -> (ColorType, bool) {
    match format {
        // Formats that typically have alpha
        ImageFormat::Png
        | ImageFormat::Webp
        | ImageFormat::Ico
        | ImageFormat::Tga
        | ImageFormat::Dds
        | ImageFormat::Qoi
        | ImageFormat::Farbfeld => (ColorType::Rgba8, true),

        // Formats that typically don't have alpha
        ImageFormat::Jpeg | ImageFormat::Pnm => (ColorType::Rgb8, false),

        // Formats that might have alpha (assume yes to be safe)
        ImageFormat::Gif | ImageFormat::Bmp | ImageFormat::Tiff | ImageFormat::Avif => {
            (ColorType::Rgba8, true)
        }

        // HDR formats
        ImageFormat::Hdr | ImageFormat::OpenExr => (ColorType::Rgb32F, false),

        // Unknown - assume RGBA
        ImageFormat::Unknown => (ColorType::Rgba8, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_type_properties() {
        assert_eq!(ColorType::Rgba8.bits_per_pixel(), 32);
        assert_eq!(ColorType::Rgba8.bytes_per_pixel(), 4);
        assert_eq!(ColorType::Rgba8.channels(), 4);
        assert!(ColorType::Rgba8.has_alpha());
        assert!(!ColorType::Rgba8.is_hdr());

        assert_eq!(ColorType::Rgb8.bits_per_pixel(), 24);
        assert_eq!(ColorType::Rgb8.channels(), 3);
        assert!(!ColorType::Rgb8.has_alpha());

        assert!(ColorType::Rgb32F.is_hdr());
        assert!(ColorType::Rgba32F.is_hdr());

        assert!(ColorType::Rgb16.is_16bit());
        assert!(ColorType::Rgba16.is_16bit());
        assert!(!ColorType::Rgb8.is_16bit());
    }

    #[test]
    fn test_orientation() {
        assert!(!Orientation::Normal.swaps_dimensions());
        assert!(!Orientation::Rotate180.swaps_dimensions());
        assert!(Orientation::Rotate90.swaps_dimensions());
        assert!(Orientation::Rotate270.swaps_dimensions());

        assert_eq!(Orientation::from_exif(1), Orientation::Normal);
        assert_eq!(Orientation::from_exif(6), Orientation::Rotate90);
        assert_eq!(Orientation::from_exif(8), Orientation::Rotate270);
        assert_eq!(Orientation::from_exif(99), Orientation::Normal); // Invalid defaults to Normal
    }

    #[test]
    fn test_exif_data_gps() {
        let mut exif = ExifData::default();
        assert!(!exif.has_gps());
        assert!(exif.gps_coordinates().is_none());

        exif.gps_latitude = Some(40.7128);
        assert!(!exif.has_gps()); // Still missing longitude

        exif.gps_longitude = Some(-74.0060);
        assert!(exif.has_gps());
        assert_eq!(exif.gps_coordinates(), Some((40.7128, -74.0060)));
    }

    #[test]
    fn test_image_metadata_calculations() {
        let metadata = ImageMetadata {
            width: 1920,
            height: 1080,
            format: ImageFormat::Jpeg,
            color_type: ColorType::Rgb8,
            has_alpha: false,
            exif: None,
        };

        assert!((metadata.aspect_ratio() - 1.777).abs() < 0.01);
        assert_eq!(metadata.pixel_count(), 1920 * 1080);
        assert_eq!(metadata.estimated_rgba_size(), 1920 * 1080 * 4);
        assert_eq!(metadata.corrected_dimensions(), (1920, 1080));
    }

    #[test]
    fn test_corrected_dimensions_with_rotation() {
        let mut metadata = ImageMetadata {
            width: 1920,
            height: 1080,
            format: ImageFormat::Jpeg,
            color_type: ColorType::Rgb8,
            has_alpha: false,
            exif: Some(ExifData {
                orientation: Orientation::Rotate90,
                ..Default::default()
            }),
        };

        // Rotation swaps dimensions
        assert_eq!(metadata.corrected_dimensions(), (1080, 1920));

        // No rotation
        metadata.exif.as_mut().unwrap().orientation = Orientation::Normal;
        assert_eq!(metadata.corrected_dimensions(), (1920, 1080));
    }
}
