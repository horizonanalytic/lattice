//! Window icon support.
//!
//! This module provides types for setting window icons from various sources.

use std::path::Path;

/// A window icon that can be set on a native window.
///
/// Icons can be created from raw RGBA pixel data or loaded from files.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::WindowIcon;
///
/// // Create from raw RGBA data (32 bits per pixel)
/// let rgba_data: Vec<u8> = vec![255, 0, 0, 255]; // 1x1 red pixel
/// let icon = WindowIcon::from_rgba(rgba_data, 1, 1)?;
///
/// // Load from a PNG file
/// let icon = WindowIcon::from_path("icon.png")?;
/// ```
#[derive(Clone)]
pub struct WindowIcon {
    /// RGBA pixel data (32 bits per pixel, row-major order).
    rgba: Vec<u8>,
    /// Icon width in pixels.
    width: u32,
    /// Icon height in pixels.
    height: u32,
}

/// Error type for icon operations.
#[derive(Debug)]
pub struct IconError {
    kind: IconErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconErrorKind {
    /// Invalid dimensions (zero or too large).
    InvalidDimensions,
    /// Data size doesn't match dimensions.
    DataSizeMismatch,
    /// Failed to load icon from file.
    LoadFailed,
    /// Unsupported image format.
    UnsupportedFormat,
}

impl IconError {
    fn invalid_dimensions(message: impl Into<String>) -> Self {
        Self {
            kind: IconErrorKind::InvalidDimensions,
            message: message.into(),
        }
    }

    fn data_size_mismatch(expected: usize, actual: usize) -> Self {
        Self {
            kind: IconErrorKind::DataSizeMismatch,
            message: format!("expected {} bytes, got {}", expected, actual),
        }
    }

    fn load_failed(message: impl Into<String>) -> Self {
        Self {
            kind: IconErrorKind::LoadFailed,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn unsupported_format(message: impl Into<String>) -> Self {
        Self {
            kind: IconErrorKind::UnsupportedFormat,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for IconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "icon error: {}", self.message)
    }
}

impl std::error::Error for IconError {}

impl WindowIcon {
    /// Create a window icon from raw RGBA pixel data.
    ///
    /// The data must be in row-major order with 4 bytes per pixel (RGBA).
    ///
    /// # Arguments
    ///
    /// * `rgba` - Raw pixel data in RGBA format (4 bytes per pixel)
    /// * `width` - Icon width in pixels
    /// * `height` - Icon height in pixels
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Width or height is zero
    /// - Data size doesn't match dimensions (width * height * 4)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create a 2x2 red square
    /// let rgba = vec![
    ///     255, 0, 0, 255,  255, 0, 0, 255,  // Row 1
    ///     255, 0, 0, 255,  255, 0, 0, 255,  // Row 2
    /// ];
    /// let icon = WindowIcon::from_rgba(rgba, 2, 2)?;
    /// ```
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, IconError> {
        if width == 0 || height == 0 {
            return Err(IconError::invalid_dimensions(
                "width and height must be non-zero",
            ));
        }

        let expected_size = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected_size {
            return Err(IconError::data_size_mismatch(expected_size, rgba.len()));
        }

        Ok(Self {
            rgba,
            width,
            height,
        })
    }

    /// Load a window icon from an image file.
    ///
    /// Supports PNG, JPEG, BMP, ICO, and other formats via the `image` crate.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or decoded.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, IconError> {
        let path = path.as_ref();

        let img = image::open(path)
            .map_err(|e| IconError::load_failed(format!("{}: {}", path.display(), e)))?;

        let rgba_image = img.to_rgba8();
        let width = rgba_image.width();
        let height = rgba_image.height();
        let rgba = rgba_image.into_raw();

        Ok(Self {
            rgba,
            width,
            height,
        })
    }

    /// Load a window icon from in-memory image data.
    ///
    /// The format is auto-detected from the data.
    ///
    /// # Arguments
    ///
    /// * `data` - Image file data (PNG, JPEG, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be decoded.
    pub fn from_memory(data: &[u8]) -> Result<Self, IconError> {
        let img = image::load_from_memory(data)
            .map_err(|e| IconError::load_failed(format!("failed to decode image: {}", e)))?;

        let rgba_image = img.to_rgba8();
        let width = rgba_image.width();
        let height = rgba_image.height();
        let rgba = rgba_image.into_raw();

        Ok(Self {
            rgba,
            width,
            height,
        })
    }

    /// Get the icon width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the icon height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the raw RGBA pixel data.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Convert to a winit Icon.
    ///
    /// This is used internally when setting the window icon.
    pub(crate) fn to_winit_icon(&self) -> Result<winit::window::Icon, IconError> {
        winit::window::Icon::from_rgba(self.rgba.clone(), self.width, self.height)
            .map_err(|e| IconError::invalid_dimensions(format!("winit icon error: {}", e)))
    }
}

impl std::fmt::Debug for WindowIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowIcon")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("data_len", &self.rgba.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_from_rgba_valid() {
        // 2x2 red square
        let rgba = vec![
            255, 0, 0, 255, 255, 0, 0, 255, // Row 1
            255, 0, 0, 255, 255, 0, 0, 255, // Row 2
        ];
        let icon = WindowIcon::from_rgba(rgba, 2, 2).unwrap();
        assert_eq!(icon.width(), 2);
        assert_eq!(icon.height(), 2);
        assert_eq!(icon.rgba().len(), 16);
    }

    #[test]
    fn test_icon_from_rgba_zero_dimensions() {
        let result = WindowIcon::from_rgba(vec![], 0, 10);
        assert!(result.is_err());

        let result = WindowIcon::from_rgba(vec![], 10, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_icon_from_rgba_size_mismatch() {
        // 2x2 needs 16 bytes, but we provide 8
        let rgba = vec![255; 8];
        let result = WindowIcon::from_rgba(rgba, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_icon_debug() {
        let rgba = vec![0; 16];
        let icon = WindowIcon::from_rgba(rgba, 2, 2).unwrap();
        let debug_str = format!("{:?}", icon);
        assert!(debug_str.contains("WindowIcon"));
        assert!(debug_str.contains("width: 2"));
        assert!(debug_str.contains("height: 2"));
    }
}
