//! Icon support for widgets.
//!
//! This module provides the [`Icon`] type for displaying icons in widgets like buttons,
//! menus, and tabs. Icons can be created from images or loaded lazily from file paths.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{Icon, Image, Color};
//!
//! // Create from an already-loaded image
//! let icon = Icon::from_image(my_image);
//!
//! // Create with a lazy-loaded path
//! let icon = Icon::from_path("icons/save.png");
//!
//! // Create with a disabled variant
//! let icon = Icon::from_image(normal_image)
//!     .with_disabled_image(disabled_image);
//! ```

use std::path::{Path, PathBuf};

use crate::image::Image;
use crate::types::{Color, Size};

/// Source for an icon - either a pre-loaded image or a path for lazy loading.
#[derive(Clone, Debug)]
pub enum IconSource {
    /// A pre-loaded image.
    Image(Image),
    /// A path to load the image from lazily.
    Path(PathBuf),
}

impl IconSource {
    /// Check if this source has been loaded.
    pub fn is_loaded(&self) -> bool {
        matches!(self, IconSource::Image(_))
    }

    /// Get the image if loaded.
    pub fn image(&self) -> Option<&Image> {
        match self {
            IconSource::Image(img) => Some(img),
            IconSource::Path(_) => None,
        }
    }

    /// Get the path if this is a path source.
    pub fn path(&self) -> Option<&Path> {
        match self {
            IconSource::Image(_) => None,
            IconSource::Path(p) => Some(p),
        }
    }
}

/// An icon that can be displayed in widgets.
///
/// Icons support:
/// - Pre-loaded images or lazy loading from paths
/// - Optional disabled variant (other states use color tinting)
/// - Preferred size specification
///
/// # State Handling
///
/// - **Normal**: Uses the primary icon image
/// - **Disabled**: Uses the disabled variant if provided, otherwise tints the normal icon
/// - **Pressed/Hovered/Checked**: Uses color tinting of the normal icon
#[derive(Clone, Debug)]
pub struct Icon {
    /// The primary icon source (normal state).
    source: IconSource,

    /// Optional disabled icon source.
    disabled_source: Option<IconSource>,

    /// Preferred display size. If None, uses the natural image size.
    preferred_size: Option<Size>,

    /// Loaded image cache for the normal state (when using path source).
    loaded_image: Option<Image>,

    /// Loaded image cache for the disabled state (when using path source).
    loaded_disabled_image: Option<Image>,
}

impl Icon {
    /// Create an icon from a pre-loaded image.
    pub fn from_image(image: Image) -> Self {
        Self {
            source: IconSource::Image(image),
            disabled_source: None,
            preferred_size: None,
            loaded_image: None,
            loaded_disabled_image: None,
        }
    }

    /// Create an icon from a file path (lazy loading).
    ///
    /// The image will be loaded when first needed for rendering.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            source: IconSource::Path(path.as_ref().to_path_buf()),
            disabled_source: None,
            preferred_size: None,
            loaded_image: None,
            loaded_disabled_image: None,
        }
    }

    /// Set a disabled variant image.
    pub fn with_disabled_image(mut self, image: Image) -> Self {
        self.disabled_source = Some(IconSource::Image(image));
        self
    }

    /// Set a disabled variant from a file path.
    pub fn with_disabled_path(mut self, path: impl AsRef<Path>) -> Self {
        self.disabled_source = Some(IconSource::Path(path.as_ref().to_path_buf()));
        self
    }

    /// Set the preferred display size.
    ///
    /// If not set, the icon will be displayed at its natural size.
    pub fn with_size(mut self, size: Size) -> Self {
        self.preferred_size = Some(size);
        self
    }

    /// Set the preferred display size with width and height.
    pub fn with_dimensions(mut self, width: f32, height: f32) -> Self {
        self.preferred_size = Some(Size::new(width, height));
        self
    }

    /// Get the icon source.
    pub fn source(&self) -> &IconSource {
        &self.source
    }

    /// Get the disabled icon source, if any.
    pub fn disabled_source(&self) -> Option<&IconSource> {
        self.disabled_source.as_ref()
    }

    /// Get the preferred size, if set.
    pub fn preferred_size(&self) -> Option<Size> {
        self.preferred_size
    }

    /// Check if the icon has a dedicated disabled variant.
    pub fn has_disabled_variant(&self) -> bool {
        self.disabled_source.is_some()
    }

    /// Get the image for the normal state.
    ///
    /// Returns None if the icon uses a path source that hasn't been loaded yet.
    pub fn image(&self) -> Option<&Image> {
        match &self.source {
            IconSource::Image(img) => Some(img),
            IconSource::Path(_) => self.loaded_image.as_ref(),
        }
    }

    /// Get the image for the disabled state.
    ///
    /// Returns the disabled variant if available, otherwise returns the normal image.
    pub fn disabled_image(&self) -> Option<&Image> {
        // First try the explicit disabled source
        if let Some(disabled) = &self.disabled_source {
            match disabled {
                IconSource::Image(img) => return Some(img),
                IconSource::Path(_) => {
                    if let Some(img) = &self.loaded_disabled_image {
                        return Some(img);
                    }
                }
            }
        }
        // Fall back to normal image
        self.image()
    }

    /// Get the natural size of the icon.
    ///
    /// Returns the preferred size if set, otherwise the image's natural size.
    /// Returns None if the icon uses a path source that hasn't been loaded yet.
    pub fn size(&self) -> Option<Size> {
        if let Some(preferred) = self.preferred_size {
            return Some(preferred);
        }
        self.image().map(|img| img.size())
    }

    /// Get the display size for this icon.
    ///
    /// This is the size that should be used for layout calculations.
    /// Returns the preferred size if set, otherwise the natural image size,
    /// or a default size if the image isn't loaded.
    pub fn display_size(&self) -> Size {
        self.size().unwrap_or_else(|| Size::new(16.0, 16.0))
    }

    /// Check if this icon's image(s) are loaded and ready for rendering.
    pub fn is_loaded(&self) -> bool {
        self.image().is_some()
    }

    /// Get the path for lazy loading, if this icon uses a path source.
    pub fn path(&self) -> Option<&Path> {
        self.source.path()
    }

    /// Get the disabled path for lazy loading, if using a path source.
    pub fn disabled_path(&self) -> Option<&Path> {
        self.disabled_source.as_ref().and_then(|s| s.path())
    }

    /// Set the loaded image (used by the image loading system).
    pub fn set_loaded_image(&mut self, image: Image) {
        self.loaded_image = Some(image);
    }

    /// Set the loaded disabled image (used by the image loading system).
    pub fn set_loaded_disabled_image(&mut self, image: Image) {
        self.loaded_disabled_image = Some(image);
    }
}

/// Position of an icon relative to text in a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPosition {
    /// Icon appears to the left of text (default).
    #[default]
    Left,
    /// Icon appears to the right of text.
    Right,
    /// Icon appears above text.
    Top,
    /// Icon appears below text.
    Bottom,
}

impl IconPosition {
    /// Check if this position is horizontal (left or right of text).
    pub fn is_horizontal(&self) -> bool {
        matches!(self, IconPosition::Left | IconPosition::Right)
    }

    /// Check if this position is vertical (above or below text).
    pub fn is_vertical(&self) -> bool {
        matches!(self, IconPosition::Top | IconPosition::Bottom)
    }
}

/// Mode for displaying an icon in a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// Show icon alongside text (if text is present).
    #[default]
    IconAndText,
    /// Show icon only, hide text even if present.
    IconOnly,
    /// Show text only, hide icon even if present.
    TextOnly,
}

/// Calculate the tint color for an icon based on widget state.
///
/// This provides visual feedback for interactive states by adjusting
/// the icon's appearance.
pub fn icon_tint_for_state(
    base_tint: Color,
    is_disabled: bool,
    is_pressed: bool,
    is_hovered: bool,
) -> Color {
    if is_disabled {
        // Reduce opacity for disabled state
        Color::new(base_tint.r, base_tint.g, base_tint.b, base_tint.a * 0.4)
    } else if is_pressed {
        // Darken for pressed state
        Color::new(
            base_tint.r * 0.7,
            base_tint.g * 0.7,
            base_tint.b * 0.7,
            base_tint.a,
        )
    } else if is_hovered {
        // Slightly brighten for hover
        Color::new(
            (base_tint.r * 1.1).min(1.0),
            (base_tint.g * 1.1).min(1.0),
            (base_tint.b * 1.1).min(1.0),
            base_tint.a,
        )
    } else {
        base_tint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_position_is_horizontal() {
        assert!(IconPosition::Left.is_horizontal());
        assert!(IconPosition::Right.is_horizontal());
        assert!(!IconPosition::Top.is_horizontal());
        assert!(!IconPosition::Bottom.is_horizontal());
    }

    #[test]
    fn test_icon_position_is_vertical() {
        assert!(!IconPosition::Left.is_vertical());
        assert!(!IconPosition::Right.is_vertical());
        assert!(IconPosition::Top.is_vertical());
        assert!(IconPosition::Bottom.is_vertical());
    }

    #[test]
    fn test_icon_mode_default() {
        assert_eq!(IconMode::default(), IconMode::IconAndText);
    }

    #[test]
    fn test_icon_position_default() {
        assert_eq!(IconPosition::default(), IconPosition::Left);
    }

    #[test]
    fn test_icon_tint_disabled() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state(base, true, false, false);
        assert!(tinted.a < base.a); // Should be more transparent
    }

    #[test]
    fn test_icon_tint_pressed() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state(base, false, true, false);
        assert!(tinted.r < base.r); // Should be darker
    }

    #[test]
    fn test_icon_from_path() {
        let icon = Icon::from_path("test/icon.png");
        assert!(icon.path().is_some());
        assert!(!icon.is_loaded());
        assert_eq!(icon.display_size(), Size::new(16.0, 16.0)); // Default size
    }

    #[test]
    fn test_icon_with_size() {
        let icon = Icon::from_path("test/icon.png").with_dimensions(24.0, 24.0);
        assert_eq!(icon.display_size(), Size::new(24.0, 24.0));
    }
}
