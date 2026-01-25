//! CPU-side image manipulation buffer.
//!
//! This module provides [`ImageBuffer`], a wrapper around `image::DynamicImage`
//! that provides convenient methods for image manipulation operations like
//! resizing, cropping, rotating, color adjustments, and composition.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{ImageBuffer, ResizeFilter, Color};
//!
//! // Load and manipulate an image
//! let image = ImageBuffer::from_file("photo.jpg")?
//!     .resize(800, 600, ResizeFilter::Lanczos3)
//!     .adjust_brightness(0.1)
//!     .adjust_contrast(1.2);
//!
//! // Save the result
//! image.save("output.jpg")?;
//!
//! // Or upload to GPU for rendering
//! let gpu_image = image.upload(&mut image_manager)?;
//! ```

use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat as ImgFormat, Rgba, RgbaImage};

use crate::atlas::ImageManager;
use crate::error::{RenderError, RenderResult};
use crate::image::Image;
use crate::types::Color;

/// Resampling filter for resize operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeFilter {
    /// Nearest neighbor interpolation. Fast but pixelated.
    Nearest,
    /// Bilinear interpolation. Balanced speed and quality.
    #[default]
    Triangle,
    /// Catmull-Rom bicubic interpolation. Good quality.
    CatmullRom,
    /// Gaussian blur interpolation. Smooth results.
    Gaussian,
    /// Lanczos interpolation with window size 3. High quality.
    Lanczos3,
}

impl ResizeFilter {
    fn to_image_filter(self) -> image::imageops::FilterType {
        match self {
            ResizeFilter::Nearest => image::imageops::FilterType::Nearest,
            ResizeFilter::Triangle => image::imageops::FilterType::Triangle,
            ResizeFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
            ResizeFilter::Gaussian => image::imageops::FilterType::Gaussian,
            ResizeFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

/// Blend mode for image composition operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageBlendMode {
    /// Standard alpha blending.
    #[default]
    Normal,
    /// Additive blending (source + destination).
    Add,
    /// Multiplicative blending (source * destination).
    Multiply,
    /// Screen blending (1 - (1 - source) * (1 - destination)).
    Screen,
    /// Direct replacement, ignoring destination.
    Replace,
}

/// Output format for image encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// PNG format (lossless).
    Png,
    /// JPEG format (lossy).
    Jpeg,
    /// WebP format.
    WebP,
    /// BMP format.
    Bmp,
    /// GIF format.
    Gif,
    /// TIFF format.
    Tiff,
}

impl OutputFormat {
    fn to_image_format(self) -> ImgFormat {
        match self {
            OutputFormat::Png => ImgFormat::Png,
            OutputFormat::Jpeg => ImgFormat::Jpeg,
            OutputFormat::WebP => ImgFormat::WebP,
            OutputFormat::Bmp => ImgFormat::Bmp,
            OutputFormat::Gif => ImgFormat::Gif,
            OutputFormat::Tiff => ImgFormat::Tiff,
        }
    }
}

/// A CPU-side image buffer for manipulation operations.
///
/// `ImageBuffer` wraps `image::DynamicImage` and provides a fluent API for
/// common image operations. All geometric and color operations return a new
/// `ImageBuffer`, making them chainable.
///
/// # Construction
///
/// Images can be loaded from files, bytes, or created programmatically:
///
/// ```ignore
/// // From file
/// let img = ImageBuffer::from_file("photo.png")?;
///
/// // From bytes
/// let img = ImageBuffer::from_bytes(&png_data)?;
///
/// // Solid color
/// let red = ImageBuffer::from_color(100, 100, Color::RED);
///
/// // Transparent
/// let blank = ImageBuffer::new(100, 100);
/// ```
///
/// # Chainable Operations
///
/// Most operations return a new `ImageBuffer`, allowing method chaining:
///
/// ```ignore
/// let result = ImageBuffer::from_file("input.jpg")?
///     .resize(800, 600, ResizeFilter::Lanczos3)
///     .rotate90()
///     .adjust_brightness(0.1)
///     .to_grayscale();
/// ```
#[derive(Clone)]
pub struct ImageBuffer {
    inner: DynamicImage,
}

impl ImageBuffer {
    // ========================================================================
    // CONSTRUCTION
    // ========================================================================

    /// Create a new transparent image with the specified dimensions.
    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: DynamicImage::ImageRgba8(RgbaImage::new(width, height)),
        }
    }

    /// Create an image filled with a solid color.
    pub fn from_color(width: u32, height: u32, color: Color) -> Self {
        let mut rgba = RgbaImage::new(width, height);
        let pixel = Self::color_to_rgba(color);
        for p in rgba.pixels_mut() {
            *p = pixel;
        }
        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    /// Load an image from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> RenderResult<Self> {
        let img = image::open(path.as_ref()).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to load image: {}", e))
        })?;
        Ok(Self { inner: img })
    }

    /// Load an image from bytes in memory.
    pub fn from_bytes(bytes: &[u8]) -> RenderResult<Self> {
        let img = image::load_from_memory(bytes).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to decode image: {}", e))
        })?;
        Ok(Self { inner: img })
    }

    /// Create an image from raw RGBA pixel data.
    ///
    /// The data must be exactly `width * height * 4` bytes, with pixels in
    /// row-major order, 4 bytes per pixel (R, G, B, A).
    pub fn from_rgba(data: &[u8], width: u32, height: u32) -> RenderResult<Self> {
        let expected = (width * height * 4) as usize;
        if data.len() != expected {
            return Err(RenderError::ImageLoad(format!(
                "Invalid data size: expected {} bytes, got {}",
                expected,
                data.len()
            )));
        }
        let rgba = RgbaImage::from_raw(width, height, data.to_vec()).ok_or_else(|| {
            RenderError::ImageLoad("Failed to create image from raw data".to_string())
        })?;
        Ok(Self {
            inner: DynamicImage::ImageRgba8(rgba),
        })
    }

    /// Create an image from raw RGB pixel data.
    ///
    /// The data must be exactly `width * height * 3` bytes, with pixels in
    /// row-major order, 3 bytes per pixel (R, G, B).
    pub fn from_rgb(data: &[u8], width: u32, height: u32) -> RenderResult<Self> {
        let expected = (width * height * 3) as usize;
        if data.len() != expected {
            return Err(RenderError::ImageLoad(format!(
                "Invalid data size: expected {} bytes, got {}",
                expected,
                data.len()
            )));
        }
        let rgb = image::RgbImage::from_raw(width, height, data.to_vec()).ok_or_else(|| {
            RenderError::ImageLoad("Failed to create image from raw data".to_string())
        })?;
        Ok(Self {
            inner: DynamicImage::ImageRgb8(rgb),
        })
    }

    /// Create from an existing `DynamicImage`.
    #[inline]
    pub fn from_dynamic_image(img: DynamicImage) -> Self {
        Self { inner: img }
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Get the width of the image in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Get the height of the image in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Get the dimensions as a (width, height) tuple.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    /// Get the size as a `Size` struct.
    #[inline]
    pub fn size(&self) -> crate::types::Size {
        crate::types::Size::new(self.width() as f32, self.height() as f32)
    }

    /// Get the color type of the underlying image.
    #[inline]
    pub fn color_type(&self) -> image::ColorType {
        self.inner.color()
    }

    /// Check if the image has an alpha channel.
    #[inline]
    pub fn has_alpha(&self) -> bool {
        self.inner.color().has_alpha()
    }

    // ========================================================================
    // GEOMETRIC TRANSFORMS
    // ========================================================================

    /// Resize the image to exact dimensions.
    ///
    /// This may change the aspect ratio. Use [`resize_to_fit`](Self::resize_to_fit)
    /// or [`resize_to_fill`](Self::resize_to_fill) to preserve aspect ratio.
    #[must_use]
    pub fn resize(&self, width: u32, height: u32, filter: ResizeFilter) -> Self {
        Self {
            inner: self.inner.resize_exact(width, height, filter.to_image_filter()),
        }
    }

    /// Resize to fit within the given dimensions while preserving aspect ratio.
    ///
    /// The resulting image will be at most `max_width` x `max_height`, but may
    /// be smaller in one dimension to maintain the original aspect ratio.
    #[must_use]
    pub fn resize_to_fit(&self, max_width: u32, max_height: u32, filter: ResizeFilter) -> Self {
        Self {
            inner: self.inner.resize(max_width, max_height, filter.to_image_filter()),
        }
    }

    /// Resize to fill the given dimensions while preserving aspect ratio.
    ///
    /// The resulting image will completely fill `width` x `height`, cropping
    /// the source as necessary to maintain aspect ratio.
    #[must_use]
    pub fn resize_to_fill(&self, width: u32, height: u32, filter: ResizeFilter) -> Self {
        Self {
            inner: self.inner.resize_to_fill(width, height, filter.to_image_filter()),
        }
    }

    /// Scale the image by a factor.
    ///
    /// A factor of 2.0 doubles the size, 0.5 halves it.
    #[must_use]
    pub fn scale(&self, factor: f32, filter: ResizeFilter) -> Self {
        let new_width = ((self.width() as f32) * factor).max(1.0) as u32;
        let new_height = ((self.height() as f32) * factor).max(1.0) as u32;
        self.resize(new_width, new_height, filter)
    }

    /// Crop a rectangular region from the image.
    ///
    /// The region is specified by its top-left corner (x, y) and dimensions.
    /// Coordinates are clamped to the image bounds.
    #[must_use]
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        // Clamp to valid bounds
        let x = x.min(self.width().saturating_sub(1));
        let y = y.min(self.height().saturating_sub(1));
        let width = width.min(self.width().saturating_sub(x));
        let height = height.min(self.height().saturating_sub(y));

        Self {
            inner: self.inner.crop_imm(x, y, width, height),
        }
    }

    /// Rotate the image 90 degrees clockwise.
    #[must_use]
    pub fn rotate90(&self) -> Self {
        Self {
            inner: self.inner.rotate90(),
        }
    }

    /// Rotate the image 180 degrees.
    #[must_use]
    pub fn rotate180(&self) -> Self {
        Self {
            inner: self.inner.rotate180(),
        }
    }

    /// Rotate the image 270 degrees clockwise (90 degrees counter-clockwise).
    #[must_use]
    pub fn rotate270(&self) -> Self {
        Self {
            inner: self.inner.rotate270(),
        }
    }

    /// Flip the image horizontally (mirror along vertical axis).
    #[must_use]
    pub fn flip_horizontal(&self) -> Self {
        Self {
            inner: self.inner.fliph(),
        }
    }

    /// Flip the image vertically (mirror along horizontal axis).
    #[must_use]
    pub fn flip_vertical(&self) -> Self {
        Self {
            inner: self.inner.flipv(),
        }
    }

    // ========================================================================
    // COLOR ADJUSTMENTS
    // ========================================================================

    /// Convert the image to grayscale.
    #[must_use]
    pub fn to_grayscale(&self) -> Self {
        Self {
            inner: DynamicImage::ImageLuma8(self.inner.to_luma8()),
        }
    }

    /// Adjust the brightness of the image.
    ///
    /// Positive values increase brightness, negative values decrease it.
    /// A value of 0.1 increases brightness by 10%.
    #[must_use]
    pub fn adjust_brightness(&self, value: f32) -> Self {
        let rgba = self.inner.to_rgba8();
        let adjusted = image::imageops::brighten(&rgba, (value * 255.0) as i32);
        Self {
            inner: DynamicImage::ImageRgba8(adjusted),
        }
    }

    /// Adjust the contrast of the image.
    ///
    /// A value of 1.0 keeps the original contrast. Values > 1.0 increase
    /// contrast, values < 1.0 decrease it.
    #[must_use]
    pub fn adjust_contrast(&self, factor: f32) -> Self {
        let rgba = self.inner.to_rgba8();
        let adjusted = image::imageops::contrast(&rgba, factor);
        Self {
            inner: DynamicImage::ImageRgba8(adjusted),
        }
    }

    /// Adjust the hue of the image.
    ///
    /// The value is in degrees (-180 to 180). Positive values shift toward
    /// yellow/green, negative values shift toward magenta/blue.
    #[must_use]
    pub fn adjust_hue(&self, degrees: f32) -> Self {
        let mut rgba = self.inner.to_rgba8();
        let hue_shift = degrees;

        for pixel in rgba.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            // Convert to HSV
            let (h, s, v) = rgb_to_hsv(r, g, b);
            // Shift hue
            let new_h = (h + hue_shift).rem_euclid(360.0);
            // Convert back to RGB
            let (nr, ng, nb) = hsv_to_rgb(new_h, s, v);
            pixel.0 = [nr, ng, nb, a];
        }

        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    /// Adjust the saturation of the image.
    ///
    /// A value of 1.0 keeps the original saturation. Values > 1.0 increase
    /// saturation, values < 1.0 decrease it. A value of 0.0 produces grayscale.
    #[must_use]
    pub fn adjust_saturation(&self, factor: f32) -> Self {
        let mut rgba = self.inner.to_rgba8();

        for pixel in rgba.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            // Convert to HSV
            let (h, s, v) = rgb_to_hsv(r, g, b);
            // Adjust saturation
            let new_s = (s * factor).clamp(0.0, 1.0);
            // Convert back to RGB
            let (nr, ng, nb) = hsv_to_rgb(h, new_s, v);
            pixel.0 = [nr, ng, nb, a];
        }

        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    /// Invert the colors of the image.
    #[must_use]
    pub fn invert(&self) -> Self {
        let mut img = self.inner.clone();
        img.invert();
        Self { inner: img }
    }

    /// Apply a color tint to the image.
    ///
    /// The tint color is multiplied with each pixel.
    #[must_use]
    pub fn tint(&self, color: Color) -> Self {
        let mut rgba = self.inner.to_rgba8();
        let tint_r = color.r;
        let tint_g = color.g;
        let tint_b = color.b;

        for pixel in rgba.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            pixel.0 = [
                ((r as f32 / 255.0) * tint_r * 255.0).clamp(0.0, 255.0) as u8,
                ((g as f32 / 255.0) * tint_g * 255.0).clamp(0.0, 255.0) as u8,
                ((b as f32 / 255.0) * tint_b * 255.0).clamp(0.0, 255.0) as u8,
                a,
            ];
        }

        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    /// Apply a sepia tone effect.
    #[must_use]
    pub fn sepia(&self) -> Self {
        let mut rgba = self.inner.to_rgba8();

        for pixel in rgba.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let rf = r as f32;
            let gf = g as f32;
            let bf = b as f32;

            // Standard sepia transform matrix
            let new_r = (0.393 * rf + 0.769 * gf + 0.189 * bf).clamp(0.0, 255.0) as u8;
            let new_g = (0.349 * rf + 0.686 * gf + 0.168 * bf).clamp(0.0, 255.0) as u8;
            let new_b = (0.272 * rf + 0.534 * gf + 0.131 * bf).clamp(0.0, 255.0) as u8;

            pixel.0 = [new_r, new_g, new_b, a];
        }

        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    // ========================================================================
    // PIXEL ACCESS
    // ========================================================================

    /// Get the color of a pixel at the specified coordinates.
    ///
    /// Returns `None` if the coordinates are out of bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let pixel = self.inner.get_pixel(x, y);
        Some(Color::from_rgba8(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]))
    }

    /// Set the color of a pixel at the specified coordinates.
    ///
    /// This modifies the image in place. Does nothing if coordinates are
    /// out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width() || y >= self.height() {
            return;
        }
        let pixel = Self::color_to_rgba(color);
        // We need a mutable reference, so convert to RGBA8 if needed
        if let Some(rgba) = self.inner.as_mut_rgba8() {
            rgba.put_pixel(x, y, pixel);
        } else {
            // Convert to RGBA8 first
            let mut rgba = self.inner.to_rgba8();
            rgba.put_pixel(x, y, pixel);
            self.inner = DynamicImage::ImageRgba8(rgba);
        }
    }

    /// Iterate over all pixels in the image.
    ///
    /// Returns an iterator of ((x, y), Color) tuples.
    pub fn pixels(&self) -> impl Iterator<Item = ((u32, u32), Color)> + '_ {
        self.inner.pixels().map(|(x, y, pixel)| {
            let color = Color::from_rgba8(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
            ((x, y), color)
        })
    }

    /// Transform each pixel in the image using a function.
    ///
    /// The function receives the (x, y) coordinates and current color,
    /// and returns the new color.
    #[must_use]
    pub fn map_pixels<F>(&self, mut f: F) -> Self
    where
        F: FnMut(u32, u32, Color) -> Color,
    {
        let mut rgba = self.inner.to_rgba8();

        for y in 0..self.height() {
            for x in 0..self.width() {
                let pixel = rgba.get_pixel(x, y);
                let color = Color::from_rgba8(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                let new_color = f(x, y, color);
                rgba.put_pixel(x, y, Self::color_to_rgba(new_color));
            }
        }

        Self {
            inner: DynamicImage::ImageRgba8(rgba),
        }
    }

    // ========================================================================
    // FORMAT CONVERSION
    // ========================================================================

    /// Convert to RGBA8 format.
    #[must_use]
    pub fn to_rgba8(&self) -> Self {
        Self {
            inner: DynamicImage::ImageRgba8(self.inner.to_rgba8()),
        }
    }

    /// Convert to RGB8 format (discarding alpha).
    #[must_use]
    pub fn to_rgb8(&self) -> Self {
        Self {
            inner: DynamicImage::ImageRgb8(self.inner.to_rgb8()),
        }
    }

    /// Convert to grayscale (8-bit luminance).
    #[must_use]
    pub fn to_luma8(&self) -> Self {
        Self {
            inner: DynamicImage::ImageLuma8(self.inner.to_luma8()),
        }
    }

    /// Get the raw RGBA8 pixel data as bytes.
    ///
    /// The data is in row-major order, 4 bytes per pixel (R, G, B, A).
    pub fn as_rgba8_bytes(&self) -> Vec<u8> {
        self.inner.to_rgba8().into_raw()
    }

    /// Get the raw RGB8 pixel data as bytes.
    ///
    /// The data is in row-major order, 3 bytes per pixel (R, G, B).
    pub fn as_rgb8_bytes(&self) -> Vec<u8> {
        self.inner.to_rgb8().into_raw()
    }

    /// Get a reference to the underlying `DynamicImage`.
    #[inline]
    pub fn as_inner(&self) -> &DynamicImage {
        &self.inner
    }

    /// Consume this buffer and return the underlying `DynamicImage`.
    #[inline]
    pub fn into_inner(self) -> DynamicImage {
        self.inner
    }

    // ========================================================================
    // COMPOSITION
    // ========================================================================

    /// Overlay another image on top of this one.
    ///
    /// The overlay is placed at the specified (x, y) position. Standard
    /// alpha blending is used.
    #[must_use]
    pub fn overlay(&self, other: &ImageBuffer, x: i64, y: i64) -> Self {
        self.blend(other, x, y, ImageBlendMode::Normal)
    }

    /// Blend another image onto this one with a specified blend mode.
    #[must_use]
    pub fn blend(&self, other: &ImageBuffer, x: i64, y: i64, mode: ImageBlendMode) -> Self {
        let mut result = self.inner.to_rgba8();
        let other_rgba = other.inner.to_rgba8();

        for oy in 0..other.height() {
            for ox in 0..other.width() {
                let dx = x + ox as i64;
                let dy = y + oy as i64;

                // Skip if out of bounds
                if dx < 0 || dy < 0 || dx >= result.width() as i64 || dy >= result.height() as i64 {
                    continue;
                }

                let dx = dx as u32;
                let dy = dy as u32;

                let src = other_rgba.get_pixel(ox, oy);
                let dst = result.get_pixel(dx, dy);

                let blended = blend_pixels(*dst, *src, mode);
                result.put_pixel(dx, dy, blended);
            }
        }

        Self {
            inner: DynamicImage::ImageRgba8(result),
        }
    }

    /// Blend another image with a specified opacity.
    ///
    /// The opacity value (0.0 to 1.0) is multiplied with the source alpha.
    #[must_use]
    pub fn blend_with_opacity(&self, other: &ImageBuffer, x: i64, y: i64, opacity: f32) -> Self {
        let mut result = self.inner.to_rgba8();
        let other_rgba = other.inner.to_rgba8();
        let opacity = opacity.clamp(0.0, 1.0);

        for oy in 0..other.height() {
            for ox in 0..other.width() {
                let dx = x + ox as i64;
                let dy = y + oy as i64;

                // Skip if out of bounds
                if dx < 0 || dy < 0 || dx >= result.width() as i64 || dy >= result.height() as i64 {
                    continue;
                }

                let dx = dx as u32;
                let dy = dy as u32;

                let mut src = *other_rgba.get_pixel(ox, oy);
                // Multiply source alpha by opacity
                src.0[3] = ((src.0[3] as f32) * opacity) as u8;

                let dst = *result.get_pixel(dx, dy);
                let blended = blend_pixels(dst, src, ImageBlendMode::Normal);
                result.put_pixel(dx, dy, blended);
            }
        }

        Self {
            inner: DynamicImage::ImageRgba8(result),
        }
    }

    // ========================================================================
    // EXPORT
    // ========================================================================

    /// Save the image to a file.
    ///
    /// The format is determined by the file extension.
    pub fn save(&self, path: impl AsRef<Path>) -> RenderResult<()> {
        self.inner.save(path.as_ref()).map_err(|e| {
            RenderError::ImageSaveError(format!("Failed to save image: {}", e))
        })
    }

    /// Save the image to a file with a specific format.
    pub fn save_with_format(&self, path: impl AsRef<Path>, format: OutputFormat) -> RenderResult<()> {
        self.inner.save_with_format(path.as_ref(), format.to_image_format()).map_err(|e| {
            RenderError::ImageSaveError(format!("Failed to save image: {}", e))
        })
    }

    /// Encode the image to bytes in the specified format.
    pub fn encode(&self, format: OutputFormat) -> RenderResult<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        self.inner.write_to(&mut buffer, format.to_image_format()).map_err(|e| {
            RenderError::ImageSaveError(format!("Failed to encode image: {}", e))
        })?;
        Ok(buffer.into_inner())
    }

    /// Encode the image as PNG.
    pub fn to_png(&self) -> RenderResult<Vec<u8>> {
        self.encode(OutputFormat::Png)
    }

    /// Encode the image as JPEG with the specified quality (1-100).
    pub fn to_jpeg(&self, quality: u8) -> RenderResult<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        let rgb = self.inner.to_rgb8();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        encoder.encode(&rgb, self.width(), self.height(), image::ExtendedColorType::Rgb8)
            .map_err(|e| RenderError::ImageSaveError(format!("Failed to encode JPEG: {}", e)))?;
        Ok(buffer.into_inner())
    }

    // ========================================================================
    // GPU UPLOAD
    // ========================================================================

    /// Upload this image to the GPU for rendering.
    ///
    /// Returns an [`Image`] that can be used with the renderer.
    pub fn upload(&self, manager: &mut ImageManager) -> RenderResult<Image> {
        let rgba = self.inner.to_rgba8();
        let (width, height) = rgba.dimensions();
        manager.load_rgba(rgba.as_raw(), width, height)
    }

    // ========================================================================
    // HELPERS
    // ========================================================================

    fn color_to_rgba(color: Color) -> Rgba<u8> {
        // Unpremultiply alpha for storage
        let (r, g, b) = if color.a > 0.0 {
            (
                ((color.r / color.a) * 255.0).clamp(0.0, 255.0) as u8,
                ((color.g / color.a) * 255.0).clamp(0.0, 255.0) as u8,
                ((color.b / color.a) * 255.0).clamp(0.0, 255.0) as u8,
            )
        } else {
            (0, 0, 0)
        };
        let a = (color.a * 255.0).clamp(0.0, 255.0) as u8;
        Rgba([r, g, b, a])
    }
}

impl std::fmt::Debug for ImageBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageBuffer")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("color_type", &self.color_type())
            .finish()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert RGB (0-255) to HSV (h: 0-360, s: 0-1, v: 0-1).
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let v = max;
    let s = if max > 0.0 { delta / max } else { 0.0 };

    let h = if delta == 0.0 {
        0.0
    } else if max == rf {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if max == gf {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, v)
}

/// Convert HSV (h: 0-360, s: 0-1, v: 0-1) to RGB (0-255).
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let h = h.rem_euclid(360.0);

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// Blend two pixels according to the specified blend mode.
fn blend_pixels(dst: Rgba<u8>, src: Rgba<u8>, mode: ImageBlendMode) -> Rgba<u8> {
    let [sr, sg, sb, sa] = src.0;
    let [dr, dg, db, da] = dst.0;

    // Normalize to 0-1 range
    let src_a = sa as f32 / 255.0;
    let dst_a = da as f32 / 255.0;

    if src_a == 0.0 {
        return dst;
    }

    match mode {
        ImageBlendMode::Replace => src,

        ImageBlendMode::Normal => {
            // Standard alpha blending
            let out_a = src_a + dst_a * (1.0 - src_a);
            if out_a == 0.0 {
                return Rgba([0, 0, 0, 0]);
            }

            let blend = |s: u8, d: u8| -> u8 {
                let sf = s as f32 / 255.0;
                let df = d as f32 / 255.0;
                let result = (sf * src_a + df * dst_a * (1.0 - src_a)) / out_a;
                (result * 255.0).clamp(0.0, 255.0) as u8
            };

            Rgba([
                blend(sr, dr),
                blend(sg, dg),
                blend(sb, db),
                (out_a * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }

        ImageBlendMode::Add => {
            let blend = |s: u8, d: u8| -> u8 {
                let sf = s as f32 * src_a;
                let df = d as f32;
                (sf + df).clamp(0.0, 255.0) as u8
            };

            let out_a = (src_a + dst_a).min(1.0);
            Rgba([
                blend(sr, dr),
                blend(sg, dg),
                blend(sb, db),
                (out_a * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }

        ImageBlendMode::Multiply => {
            let blend = |s: u8, d: u8| -> u8 {
                let sf = s as f32 / 255.0;
                let df = d as f32 / 255.0;
                let result = sf * df * src_a + df * (1.0 - src_a);
                (result * 255.0).clamp(0.0, 255.0) as u8
            };

            let out_a = src_a + dst_a * (1.0 - src_a);
            Rgba([
                blend(sr, dr),
                blend(sg, dg),
                blend(sb, db),
                (out_a * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }

        ImageBlendMode::Screen => {
            let blend = |s: u8, d: u8| -> u8 {
                let sf = s as f32 / 255.0;
                let df = d as f32 / 255.0;
                // Screen: 1 - (1 - src) * (1 - dst)
                let screen = 1.0 - (1.0 - sf) * (1.0 - df);
                let result = screen * src_a + df * (1.0 - src_a);
                (result * 255.0).clamp(0.0, 255.0) as u8
            };

            let out_a = src_a + dst_a * (1.0 - src_a);
            Rgba([
                blend(sr, dr),
                blend(sg, dg),
                blend(sb, db),
                (out_a * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_buffer_new() {
        let img = ImageBuffer::new(100, 50);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 50);
        assert_eq!(img.dimensions(), (100, 50));
    }

    #[test]
    fn test_image_buffer_from_color() {
        let img = ImageBuffer::from_color(10, 10, Color::RED);
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 10);

        // Check that pixels are red
        let pixel = img.get_pixel(5, 5).unwrap();
        assert!((pixel.r - 1.0).abs() < 0.01);
        assert!(pixel.g.abs() < 0.01);
        assert!(pixel.b.abs() < 0.01);
        assert!((pixel.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resize() {
        let img = ImageBuffer::from_color(100, 100, Color::BLUE);
        let resized = img.resize(50, 25, ResizeFilter::Triangle);
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 25);
    }

    #[test]
    fn test_scale() {
        let img = ImageBuffer::from_color(100, 100, Color::GREEN);
        let scaled = img.scale(0.5, ResizeFilter::Nearest);
        assert_eq!(scaled.width(), 50);
        assert_eq!(scaled.height(), 50);

        let scaled_up = img.scale(2.0, ResizeFilter::Nearest);
        assert_eq!(scaled_up.width(), 200);
        assert_eq!(scaled_up.height(), 200);
    }

    #[test]
    fn test_crop() {
        let img = ImageBuffer::from_color(100, 100, Color::RED);
        let cropped = img.crop(10, 20, 30, 40);
        assert_eq!(cropped.width(), 30);
        assert_eq!(cropped.height(), 40);
    }

    #[test]
    fn test_rotate90() {
        let img = ImageBuffer::from_color(100, 50, Color::WHITE);
        let rotated = img.rotate90();
        // 90 degree rotation swaps dimensions
        assert_eq!(rotated.width(), 50);
        assert_eq!(rotated.height(), 100);
    }

    #[test]
    fn test_rotate180() {
        let img = ImageBuffer::from_color(100, 50, Color::WHITE);
        let rotated = img.rotate180();
        // 180 degree rotation keeps dimensions
        assert_eq!(rotated.width(), 100);
        assert_eq!(rotated.height(), 50);
    }

    #[test]
    fn test_rotate270() {
        let img = ImageBuffer::from_color(100, 50, Color::WHITE);
        let rotated = img.rotate270();
        // 270 degree rotation swaps dimensions
        assert_eq!(rotated.width(), 50);
        assert_eq!(rotated.height(), 100);
    }

    #[test]
    fn test_flip_horizontal() {
        // Create an asymmetric image
        let mut img = ImageBuffer::new(10, 5);
        img.set_pixel(0, 0, Color::RED);
        img.set_pixel(9, 0, Color::BLUE);

        let flipped = img.flip_horizontal();
        // Red should now be on the right
        let right_pixel = flipped.get_pixel(9, 0).unwrap();
        assert!(right_pixel.r > 0.9);
        // Blue should now be on the left
        let left_pixel = flipped.get_pixel(0, 0).unwrap();
        assert!(left_pixel.b > 0.9);
    }

    #[test]
    fn test_flip_vertical() {
        let mut img = ImageBuffer::new(5, 10);
        img.set_pixel(0, 0, Color::RED);
        img.set_pixel(0, 9, Color::BLUE);

        let flipped = img.flip_vertical();
        // Red should now be at bottom
        let bottom_pixel = flipped.get_pixel(0, 9).unwrap();
        assert!(bottom_pixel.r > 0.9);
        // Blue should now be at top
        let top_pixel = flipped.get_pixel(0, 0).unwrap();
        assert!(top_pixel.b > 0.9);
    }

    #[test]
    fn test_to_grayscale() {
        let img = ImageBuffer::from_color(10, 10, Color::RED);
        let gray = img.to_grayscale();
        // Should still have same dimensions
        assert_eq!(gray.width(), 10);
        assert_eq!(gray.height(), 10);
    }

    #[test]
    fn test_adjust_brightness() {
        let img = ImageBuffer::from_color(10, 10, Color::from_rgb(0.5, 0.5, 0.5));
        let brightened = img.adjust_brightness(0.2);
        let pixel = brightened.get_pixel(5, 5).unwrap();
        // Should be brighter
        assert!(pixel.r > 0.5);
    }

    #[test]
    fn test_invert() {
        let img = ImageBuffer::from_color(10, 10, Color::BLACK);
        let inverted = img.invert();
        let pixel = inverted.get_pixel(5, 5).unwrap();
        // Black inverted should be white
        assert!(pixel.r > 0.9);
        assert!(pixel.g > 0.9);
        assert!(pixel.b > 0.9);
    }

    #[test]
    fn test_pixel_access() {
        let mut img = ImageBuffer::new(10, 10);

        // Set a pixel
        img.set_pixel(5, 5, Color::MAGENTA);

        // Get it back
        let pixel = img.get_pixel(5, 5).unwrap();
        assert!(pixel.r > 0.9);
        assert!(pixel.g < 0.1);
        assert!(pixel.b > 0.9);

        // Out of bounds should return None
        assert!(img.get_pixel(100, 100).is_none());
    }

    #[test]
    fn test_map_pixels() {
        let img = ImageBuffer::from_color(10, 10, Color::WHITE);

        // Make all pixels red
        let mapped = img.map_pixels(|_x, _y, _color| Color::RED);

        let pixel = mapped.get_pixel(0, 0).unwrap();
        assert!(pixel.r > 0.9);
        assert!(pixel.g < 0.1);
    }

    #[test]
    fn test_overlay() {
        let base = ImageBuffer::from_color(20, 20, Color::BLUE);
        let overlay = ImageBuffer::from_color(10, 10, Color::RED);

        let result = base.overlay(&overlay, 5, 5);

        // Corner should still be blue
        let corner = result.get_pixel(0, 0).unwrap();
        assert!(corner.b > 0.9);

        // Center should be red (overlay)
        let center = result.get_pixel(10, 10).unwrap();
        assert!(center.r > 0.9);
    }

    #[test]
    fn test_chainable_api() {
        let result = ImageBuffer::from_color(100, 100, Color::WHITE)
            .resize(50, 50, ResizeFilter::Nearest)
            .rotate90()
            .flip_horizontal()
            .adjust_brightness(0.1);

        assert_eq!(result.width(), 50);
        assert_eq!(result.height(), 50);
    }

    #[test]
    fn test_format_conversion() {
        let img = ImageBuffer::from_color(10, 10, Color::RED);

        let rgba_bytes = img.as_rgba8_bytes();
        assert_eq!(rgba_bytes.len(), 10 * 10 * 4);

        let rgb_bytes = img.as_rgb8_bytes();
        assert_eq!(rgb_bytes.len(), 10 * 10 * 3);
    }

    #[test]
    fn test_from_rgba() {
        // Create a 2x2 red image
        let data = vec![
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
        ];
        let img = ImageBuffer::from_rgba(&data, 2, 2).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);

        let pixel = img.get_pixel(0, 0).unwrap();
        assert!(pixel.r > 0.9);
    }

    #[test]
    fn test_from_rgba_invalid_size() {
        let data = vec![255, 0, 0, 255]; // Only 1 pixel but claiming 2x2
        let result = ImageBuffer::from_rgba(&data, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_png() {
        let img = ImageBuffer::from_color(10, 10, Color::RED);
        let png_data = img.to_png().unwrap();
        // PNG should start with signature
        assert!(png_data.len() > 8);
        assert_eq!(&png_data[1..4], b"PNG");
    }

    #[test]
    fn test_encode_jpeg() {
        let img = ImageBuffer::from_color(10, 10, Color::RED);
        let jpeg_data = img.to_jpeg(80).unwrap();
        // JPEG should start with SOI marker
        assert!(jpeg_data.len() > 2);
        assert_eq!(&jpeg_data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_hsv_conversion_roundtrip() {
        let colors = [
            (255u8, 0u8, 0u8),   // Red
            (0, 255, 0),         // Green
            (0, 0, 255),         // Blue
            (255, 255, 0),       // Yellow
            (128, 128, 128),     // Gray
        ];

        for (r, g, b) in colors {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let (nr, ng, nb) = hsv_to_rgb(h, s, v);

            // Allow small rounding errors
            assert!((r as i32 - nr as i32).abs() <= 1, "Red mismatch for {:?}", (r, g, b));
            assert!((g as i32 - ng as i32).abs() <= 1, "Green mismatch for {:?}", (r, g, b));
            assert!((b as i32 - nb as i32).abs() <= 1, "Blue mismatch for {:?}", (r, g, b));
        }
    }

    #[test]
    fn test_blend_modes() {
        let base = ImageBuffer::from_color(10, 10, Color::from_rgb(0.5, 0.5, 0.5));
        let overlay = ImageBuffer::from_color(10, 10, Color::WHITE);

        // Test each blend mode doesn't crash
        let _ = base.blend(&overlay, 0, 0, ImageBlendMode::Normal);
        let _ = base.blend(&overlay, 0, 0, ImageBlendMode::Add);
        let _ = base.blend(&overlay, 0, 0, ImageBlendMode::Multiply);
        let _ = base.blend(&overlay, 0, 0, ImageBlendMode::Screen);
        let _ = base.blend(&overlay, 0, 0, ImageBlendMode::Replace);
    }

    #[test]
    fn test_sepia() {
        let img = ImageBuffer::from_color(10, 10, Color::WHITE);
        let sepia = img.sepia();
        let pixel = sepia.get_pixel(5, 5).unwrap();
        // Sepia tint should make it slightly brownish (red > green > blue)
        assert!(pixel.r >= pixel.g);
        assert!(pixel.g >= pixel.b);
    }

    #[test]
    fn test_has_alpha() {
        let img = ImageBuffer::new(10, 10);
        assert!(img.has_alpha());

        let grayscale = img.to_luma8();
        assert!(!grayscale.has_alpha());
    }
}
