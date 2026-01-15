//! Icon support for widgets.
//!
//! This module provides the [`Icon`] type for displaying icons in widgets like buttons,
//! menus, and tabs. Icons can be created from images or loaded lazily from file paths.
//!
//! # Features
//!
//! - **Multiple sizes**: Icons can have variants for different sizes (16x16, 24x24, etc.)
//! - **State variants**: Dedicated images for normal, disabled, active, selected, focused states
//! - **Theme variants**: Different icons for light/dark/high-contrast themes
//! - **Lazy loading**: Icons can be loaded on-demand from file paths
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{Icon, Image, Color, IconSize, IconState};
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
//!
//! // Create with multiple size variants
//! let icon = Icon::from_path("icons/save_16.png")
//!     .with_size_variant(IconSize::Size24, IconSource::Path("icons/save_24.png".into()))
//!     .with_size_variant(IconSize::Size32, IconSource::Path("icons/save_32.png".into()));
//!
//! // Create with theme variants (light/dark)
//! let icon = Icon::from_path("icons/save_light.png")
//!     .with_dark_variant(IconSource::Path("icons/save_dark.png".into()));
//! ```

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::image::Image;
use crate::types::{Color, Size};

// ============================================================================
// Icon Size Support
// ============================================================================

/// Standard icon sizes following desktop conventions.
///
/// These sizes are commonly used across desktop environments and provide
/// good coverage for different UI contexts (toolbars, menus, dialogs, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u32)]
pub enum IconSize {
    /// 16x16 - Small icons for menus and compact toolbars
    Size16 = 16,
    /// 22x22 - Common toolbar size
    Size22 = 22,
    /// 24x24 - Standard toolbar icons
    Size24 = 24,
    /// 32x32 - Large toolbar icons
    Size32 = 32,
    /// 48x48 - Dialog icons and medium tiles
    Size48 = 48,
    /// 64x64 - Large icons
    Size64 = 64,
    /// 128x128 - Extra large icons
    Size128 = 128,
    /// 256x256 - High-resolution icons
    Size256 = 256,
}

impl IconSize {
    /// Get the size in pixels.
    pub fn as_pixels(self) -> u32 {
        self as u32
    }

    /// Get the size as a floating point value.
    pub fn as_f32(self) -> f32 {
        self as u32 as f32
    }

    /// Convert a pixel value to the nearest standard icon size.
    ///
    /// Returns `None` if the value doesn't match any standard size.
    pub fn from_pixels(pixels: u32) -> Option<Self> {
        match pixels {
            16 => Some(IconSize::Size16),
            22 => Some(IconSize::Size22),
            24 => Some(IconSize::Size24),
            32 => Some(IconSize::Size32),
            48 => Some(IconSize::Size48),
            64 => Some(IconSize::Size64),
            128 => Some(IconSize::Size128),
            256 => Some(IconSize::Size256),
            _ => None,
        }
    }

    /// Find the best standard icon size for a target pixel size.
    ///
    /// Prefers larger sizes over smaller ones to avoid scaling artifacts.
    /// For example, if target is 20, this returns Size22 (not Size16).
    pub fn best_fit(target: u32) -> Self {
        // Standard sizes in order
        const SIZES: [IconSize; 8] = [
            IconSize::Size16,
            IconSize::Size22,
            IconSize::Size24,
            IconSize::Size32,
            IconSize::Size48,
            IconSize::Size64,
            IconSize::Size128,
            IconSize::Size256,
        ];

        // Find the smallest size >= target, or the largest available
        for size in SIZES {
            if size.as_pixels() >= target {
                return size;
            }
        }
        IconSize::Size256
    }

    /// Get all standard icon sizes.
    pub fn all() -> &'static [IconSize] {
        &[
            IconSize::Size16,
            IconSize::Size22,
            IconSize::Size24,
            IconSize::Size32,
            IconSize::Size48,
            IconSize::Size64,
            IconSize::Size128,
            IconSize::Size256,
        ]
    }
}

impl Default for IconSize {
    fn default() -> Self {
        IconSize::Size16
    }
}

impl From<IconSize> for Size {
    fn from(size: IconSize) -> Self {
        let px = size.as_f32();
        Size::new(px, px)
    }
}

// ============================================================================
// Icon State Support
// ============================================================================

/// Complete set of icon states for widget interaction.
///
/// Icons can have dedicated variants for different states, or fall back
/// to color tinting of the normal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconState {
    /// Normal/default state
    #[default]
    Normal,
    /// Disabled/inactive state - widget is not interactive
    Disabled,
    /// Active/pressed state - user is clicking
    Active,
    /// Selected/checked state - item is selected
    Selected,
    /// Focused state - keyboard navigation focus
    Focused,
}

impl IconState {
    /// Check if this is the normal state.
    pub fn is_normal(self) -> bool {
        matches!(self, IconState::Normal)
    }

    /// Check if this is a state that should reduce interactivity appearance.
    pub fn is_disabled(self) -> bool {
        matches!(self, IconState::Disabled)
    }

    /// Check if this is an interactive/active state.
    pub fn is_interactive(self) -> bool {
        matches!(self, IconState::Active | IconState::Selected | IconState::Focused)
    }
}

// ============================================================================
// Icon Theme Mode Support
// ============================================================================

/// Theme mode for icon variants.
///
/// This mirrors the style system's ThemeMode but is defined here to avoid
/// a dependency from the render crate to the style crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconThemeMode {
    /// Light theme (default)
    #[default]
    Light,
    /// Dark theme
    Dark,
    /// High contrast theme for accessibility
    HighContrast,
}

// ============================================================================
// Icon Variant Collections
// ============================================================================

/// Collection of icon variants for different sizes.
///
/// Allows an icon to have different images for different display sizes,
/// which is important for crisp rendering at various scales.
#[derive(Clone, Debug, Default)]
pub struct SizedIconSet {
    /// Map from size to icon source
    variants: BTreeMap<IconSize, IconSource>,
}

impl SizedIconSet {
    /// Create a new empty sized icon set.
    pub fn new() -> Self {
        Self {
            variants: BTreeMap::new(),
        }
    }

    /// Create a sized icon set with a single size variant.
    pub fn with_size(size: IconSize, source: IconSource) -> Self {
        let mut set = Self::new();
        set.variants.insert(size, source);
        set
    }

    /// Add a size variant.
    pub fn add(&mut self, size: IconSize, source: IconSource) {
        self.variants.insert(size, source);
    }

    /// Add a size variant (builder pattern).
    pub fn with(mut self, size: IconSize, source: IconSource) -> Self {
        self.variants.insert(size, source);
        self
    }

    /// Get the icon source for a specific size.
    pub fn get(&self, size: IconSize) -> Option<&IconSource> {
        self.variants.get(&size)
    }

    /// Get the best available source for a target pixel size.
    ///
    /// Prefers exact matches, then larger sizes (to scale down), then smaller sizes.
    pub fn best_for_pixels(&self, target: u32) -> Option<(IconSize, &IconSource)> {
        if self.variants.is_empty() {
            return None;
        }

        // Try exact match first
        if let Some(size) = IconSize::from_pixels(target) {
            if let Some(source) = self.variants.get(&size) {
                return Some((size, source));
            }
        }

        // Find smallest size >= target (prefer scaling down)
        for (&size, source) in &self.variants {
            if size.as_pixels() >= target {
                return Some((size, source));
            }
        }

        // Fall back to largest available (will need to scale up)
        self.variants.iter().next_back().map(|(&s, src)| (s, src))
    }

    /// Get available sizes.
    pub fn available_sizes(&self) -> impl Iterator<Item = IconSize> + '_ {
        self.variants.keys().copied()
    }

    /// Check if any sizes are available.
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    /// Get the number of size variants.
    pub fn len(&self) -> usize {
        self.variants.len()
    }
}

/// Collection of icon variants for different states.
///
/// Allows an icon to have different images for different widget states
/// (normal, disabled, active, selected, focused).
#[derive(Clone, Debug)]
pub struct StatefulIconSet {
    /// The normal state source (always present)
    normal: IconSource,
    /// Optional state variants
    variants: HashMap<IconState, IconSource>,
}

impl StatefulIconSet {
    /// Create a new stateful icon set with the normal state.
    pub fn new(normal: IconSource) -> Self {
        Self {
            normal,
            variants: HashMap::new(),
        }
    }

    /// Add a state variant.
    pub fn add(&mut self, state: IconState, source: IconSource) {
        if state == IconState::Normal {
            self.normal = source;
        } else {
            self.variants.insert(state, source);
        }
    }

    /// Add a state variant (builder pattern).
    pub fn with_state(mut self, state: IconState, source: IconSource) -> Self {
        self.add(state, source);
        self
    }

    /// Get the icon source for a specific state.
    ///
    /// Falls back to normal state if the requested state isn't available.
    pub fn get(&self, state: IconState) -> &IconSource {
        if state == IconState::Normal {
            &self.normal
        } else {
            self.variants.get(&state).unwrap_or(&self.normal)
        }
    }

    /// Get the icon source for a state, returning None if not available (no fallback).
    pub fn get_exact(&self, state: IconState) -> Option<&IconSource> {
        if state == IconState::Normal {
            Some(&self.normal)
        } else {
            self.variants.get(&state)
        }
    }

    /// Check if a dedicated variant exists for a state.
    pub fn has_state(&self, state: IconState) -> bool {
        state == IconState::Normal || self.variants.contains_key(&state)
    }

    /// Get the normal state source.
    pub fn normal(&self) -> &IconSource {
        &self.normal
    }
}

/// Collection of icon variants for different theme modes.
///
/// Allows an icon to have different images for light, dark, and high contrast themes.
#[derive(Clone, Debug)]
pub struct ThemedIconSet {
    /// Light theme icon (always present, used as default)
    light: IconSource,
    /// Dark theme variant
    dark: Option<IconSource>,
    /// High contrast variant
    high_contrast: Option<IconSource>,
}

impl ThemedIconSet {
    /// Create a new themed icon set with the light theme variant.
    pub fn new(light: IconSource) -> Self {
        Self {
            light,
            dark: None,
            high_contrast: None,
        }
    }

    /// Set the dark theme variant (builder pattern).
    pub fn with_dark(mut self, source: IconSource) -> Self {
        self.dark = Some(source);
        self
    }

    /// Set the high contrast variant (builder pattern).
    pub fn with_high_contrast(mut self, source: IconSource) -> Self {
        self.high_contrast = Some(source);
        self
    }

    /// Set the dark theme variant (mutable).
    pub fn set_dark(&mut self, source: IconSource) {
        self.dark = Some(source);
    }

    /// Set the high contrast variant (mutable).
    pub fn set_high_contrast(&mut self, source: IconSource) {
        self.high_contrast = Some(source);
    }

    /// Get the icon source for a theme mode.
    ///
    /// Falls back to light theme if the requested mode isn't available.
    pub fn for_mode(&self, mode: IconThemeMode) -> &IconSource {
        match mode {
            IconThemeMode::Light => &self.light,
            IconThemeMode::Dark => self.dark.as_ref().unwrap_or(&self.light),
            IconThemeMode::HighContrast => self
                .high_contrast
                .as_ref()
                .or(self.dark.as_ref())
                .unwrap_or(&self.light),
        }
    }

    /// Check if a dedicated variant exists for a theme mode.
    pub fn has_mode(&self, mode: IconThemeMode) -> bool {
        match mode {
            IconThemeMode::Light => true,
            IconThemeMode::Dark => self.dark.is_some(),
            IconThemeMode::HighContrast => self.high_contrast.is_some(),
        }
    }

    /// Get the light theme source.
    pub fn light(&self) -> &IconSource {
        &self.light
    }

    /// Get the dark theme source, if set.
    pub fn dark(&self) -> Option<&IconSource> {
        self.dark.as_ref()
    }

    /// Get the high contrast source, if set.
    pub fn high_contrast(&self) -> Option<&IconSource> {
        self.high_contrast.as_ref()
    }
}

// ============================================================================
// Icon Source
// ============================================================================

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
/// - Multiple size variants for crisp rendering at different scales
/// - State variants (normal, disabled, active, selected, focused)
/// - Theme variants (light, dark, high contrast)
/// - Preferred size specification
///
/// # State Handling
///
/// - **Normal**: Uses the primary icon image
/// - **Disabled**: Uses the disabled variant if provided, otherwise tints the normal icon
/// - **Active/Selected/Focused**: Uses dedicated variant if available, otherwise tints
/// - **Pressed/Hovered**: Uses color tinting of the current state's icon
///
/// # Size Selection
///
/// When multiple sizes are available, the icon automatically selects the best
/// variant for the requested display size, preferring to scale down rather than up.
#[derive(Clone, Debug)]
pub struct Icon {
    /// The primary icon source (normal state).
    source: IconSource,

    /// Optional disabled icon source (legacy support).
    disabled_source: Option<IconSource>,

    /// Preferred display size. If None, uses the natural image size.
    preferred_size: Option<Size>,

    /// Loaded image cache for the normal state (when using path source).
    loaded_image: Option<Image>,

    /// Loaded image cache for the disabled state (when using path source).
    loaded_disabled_image: Option<Image>,

    // Advanced variant support

    /// Size variants for different display sizes.
    sized_variants: Option<SizedIconSet>,

    /// State variants for different widget states.
    state_variants: Option<StatefulIconSet>,

    /// Theme variants for different theme modes.
    themed_variants: Option<ThemedIconSet>,
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
            sized_variants: None,
            state_variants: None,
            themed_variants: None,
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
            sized_variants: None,
            state_variants: None,
            themed_variants: None,
        }
    }

    /// Create an icon from a sized icon set.
    ///
    /// The primary source will be the smallest available size variant.
    pub fn from_sized_set(set: SizedIconSet) -> Self {
        // Use the smallest available size as the primary source
        let source = set
            .variants
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| IconSource::Path(PathBuf::new()));

        Self {
            source,
            disabled_source: None,
            preferred_size: None,
            loaded_image: None,
            loaded_disabled_image: None,
            sized_variants: Some(set),
            state_variants: None,
            themed_variants: None,
        }
    }

    /// Create an icon from a stateful icon set.
    pub fn from_stateful_set(set: StatefulIconSet) -> Self {
        let source = set.normal.clone();
        Self {
            source,
            disabled_source: None,
            preferred_size: None,
            loaded_image: None,
            loaded_disabled_image: None,
            sized_variants: None,
            state_variants: Some(set),
            themed_variants: None,
        }
    }

    /// Create an icon from a themed icon set.
    pub fn from_themed_set(set: ThemedIconSet) -> Self {
        let source = set.light.clone();
        Self {
            source,
            disabled_source: None,
            preferred_size: None,
            loaded_image: None,
            loaded_disabled_image: None,
            sized_variants: None,
            state_variants: None,
            themed_variants: Some(set),
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

    // ========================================================================
    // Size Variant Methods
    // ========================================================================

    /// Add a size variant for this icon.
    ///
    /// Size variants allow the icon to be rendered crisply at different sizes
    /// by using purpose-built images rather than scaling a single image.
    pub fn with_size_variant(mut self, size: IconSize, source: IconSource) -> Self {
        if self.sized_variants.is_none() {
            self.sized_variants = Some(SizedIconSet::new());
        }
        if let Some(ref mut set) = self.sized_variants {
            set.add(size, source);
        }
        self
    }

    /// Set a complete sized icon set.
    pub fn with_sized_variants(mut self, set: SizedIconSet) -> Self {
        self.sized_variants = Some(set);
        self
    }

    /// Get the sized variants, if any.
    pub fn sized_variants(&self) -> Option<&SizedIconSet> {
        self.sized_variants.as_ref()
    }

    /// Check if this icon has size variants.
    pub fn has_size_variants(&self) -> bool {
        self.sized_variants.as_ref().map_or(false, |s| !s.is_empty())
    }

    /// Get the best image for a target pixel size.
    ///
    /// If size variants are available, returns the best match for the target size.
    /// Otherwise, returns the primary image.
    pub fn image_for_size(&self, target_pixels: u32) -> Option<&Image> {
        // Try sized variants first
        if let Some(set) = &self.sized_variants {
            if let Some((_size, source)) = set.best_for_pixels(target_pixels) {
                return source.image();
            }
        }
        // Fall back to primary image
        self.image()
    }

    // ========================================================================
    // State Variant Methods
    // ========================================================================

    /// Add a state variant for this icon.
    ///
    /// State variants provide dedicated images for different widget states,
    /// rather than relying on color tinting.
    pub fn with_state_variant(mut self, state: IconState, source: IconSource) -> Self {
        if self.state_variants.is_none() {
            // Initialize with the current source as normal state
            self.state_variants = Some(StatefulIconSet::new(self.source.clone()));
        }
        if let Some(ref mut set) = self.state_variants {
            set.add(state, source);
        }
        self
    }

    /// Set a complete stateful icon set.
    pub fn with_state_variants(mut self, set: StatefulIconSet) -> Self {
        self.state_variants = Some(set);
        self
    }

    /// Get the state variants, if any.
    pub fn state_variants(&self) -> Option<&StatefulIconSet> {
        self.state_variants.as_ref()
    }

    /// Check if this icon has a dedicated variant for a state.
    pub fn has_state_variant(&self, state: IconState) -> bool {
        self.state_variants
            .as_ref()
            .map_or(false, |s| s.has_state(state))
    }

    /// Get the image for a specific state.
    ///
    /// If state variants are available, returns the image for that state.
    /// Otherwise, returns the primary image (or disabled image for Disabled state).
    pub fn image_for_state(&self, state: IconState) -> Option<&Image> {
        // Try state variants first
        if let Some(set) = &self.state_variants {
            return set.get(state).image();
        }
        // Fall back to legacy disabled handling or primary
        match state {
            IconState::Disabled => self.disabled_image(),
            _ => self.image(),
        }
    }

    // ========================================================================
    // Theme Variant Methods
    // ========================================================================

    /// Add a dark theme variant for this icon.
    pub fn with_dark_variant(mut self, source: IconSource) -> Self {
        if let Some(ref mut set) = self.themed_variants {
            set.set_dark(source);
        } else {
            self.themed_variants = Some(ThemedIconSet::new(self.source.clone()).with_dark(source));
        }
        self
    }

    /// Add a high contrast variant for this icon.
    pub fn with_high_contrast_variant(mut self, source: IconSource) -> Self {
        if let Some(ref mut set) = self.themed_variants {
            set.set_high_contrast(source);
        } else {
            self.themed_variants =
                Some(ThemedIconSet::new(self.source.clone()).with_high_contrast(source));
        }
        self
    }

    /// Set a complete themed icon set.
    pub fn with_themed_variants(mut self, set: ThemedIconSet) -> Self {
        self.themed_variants = Some(set);
        self
    }

    /// Get the themed variants, if any.
    pub fn themed_variants(&self) -> Option<&ThemedIconSet> {
        self.themed_variants.as_ref()
    }

    /// Check if this icon has a variant for a theme mode.
    pub fn has_theme_variant(&self, mode: IconThemeMode) -> bool {
        self.themed_variants
            .as_ref()
            .map_or(false, |s| s.has_mode(mode))
    }

    /// Get the image for a specific theme mode.
    ///
    /// If theme variants are available, returns the image for that mode.
    /// Otherwise, returns the primary image.
    pub fn image_for_theme(&self, mode: IconThemeMode) -> Option<&Image> {
        if let Some(set) = &self.themed_variants {
            return set.for_mode(mode).image();
        }
        self.image()
    }

    // ========================================================================
    // Core Query Methods
    // ========================================================================

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

/// Calculate the tint color for an icon based on [`IconState`].
///
/// This is an enhanced version of [`icon_tint_for_state`] that supports
/// all icon states defined in the enum.
///
/// # Tinting Behavior
///
/// - **Normal**: Returns the base tint unchanged
/// - **Disabled**: Reduces opacity to 40%
/// - **Active**: Darkens by 30%
/// - **Selected**: Adds a slight blue tint for selection indication
/// - **Focused**: Returns base tint (focus is typically shown via outline)
pub fn icon_tint_for_state_full(base_tint: Color, state: IconState) -> Color {
    match state {
        IconState::Normal => base_tint,
        IconState::Disabled => {
            // Reduce opacity for disabled state
            Color::new(base_tint.r, base_tint.g, base_tint.b, base_tint.a * 0.4)
        }
        IconState::Active => {
            // Darken for active/pressed state
            Color::new(
                base_tint.r * 0.7,
                base_tint.g * 0.7,
                base_tint.b * 0.7,
                base_tint.a,
            )
        }
        IconState::Selected => {
            // Add slight blue tint for selection
            Color::new(
                base_tint.r * 0.9,
                base_tint.g * 0.9,
                (base_tint.b * 1.1).min(1.0),
                base_tint.a,
            )
        }
        IconState::Focused => {
            // Focused state typically shown via outline, not tint
            base_tint
        }
    }
}

/// Calculate the tint color combining [`IconState`] with hover effects.
///
/// This allows applying hover brightening on top of any base state.
pub fn icon_tint_for_state_with_hover(
    base_tint: Color,
    state: IconState,
    is_hovered: bool,
) -> Color {
    let state_tint = icon_tint_for_state_full(base_tint, state);

    // Don't apply hover effect to disabled icons
    if state == IconState::Disabled {
        return state_tint;
    }

    if is_hovered {
        // Slightly brighten for hover
        Color::new(
            (state_tint.r * 1.1).min(1.0),
            (state_tint.g * 1.1).min(1.0),
            (state_tint.b * 1.1).min(1.0),
            state_tint.a,
        )
    } else {
        state_tint
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

    // ========================================================================
    // IconSize Tests
    // ========================================================================

    #[test]
    fn test_icon_size_as_pixels() {
        assert_eq!(IconSize::Size16.as_pixels(), 16);
        assert_eq!(IconSize::Size24.as_pixels(), 24);
        assert_eq!(IconSize::Size48.as_pixels(), 48);
        assert_eq!(IconSize::Size256.as_pixels(), 256);
    }

    #[test]
    fn test_icon_size_from_pixels() {
        assert_eq!(IconSize::from_pixels(16), Some(IconSize::Size16));
        assert_eq!(IconSize::from_pixels(24), Some(IconSize::Size24));
        assert_eq!(IconSize::from_pixels(48), Some(IconSize::Size48));
        assert_eq!(IconSize::from_pixels(20), None); // Not a standard size
    }

    #[test]
    fn test_icon_size_best_fit() {
        // Exact matches
        assert_eq!(IconSize::best_fit(16), IconSize::Size16);
        assert_eq!(IconSize::best_fit(24), IconSize::Size24);

        // Prefer next larger size
        assert_eq!(IconSize::best_fit(17), IconSize::Size22);
        assert_eq!(IconSize::best_fit(20), IconSize::Size22);
        assert_eq!(IconSize::best_fit(23), IconSize::Size24);

        // Fall back to largest for huge values
        assert_eq!(IconSize::best_fit(512), IconSize::Size256);
    }

    #[test]
    fn test_icon_size_default() {
        assert_eq!(IconSize::default(), IconSize::Size16);
    }

    #[test]
    fn test_icon_size_ordering() {
        assert!(IconSize::Size16 < IconSize::Size24);
        assert!(IconSize::Size24 < IconSize::Size48);
        assert!(IconSize::Size128 < IconSize::Size256);
    }

    // ========================================================================
    // IconState Tests
    // ========================================================================

    #[test]
    fn test_icon_state_default() {
        assert_eq!(IconState::default(), IconState::Normal);
    }

    #[test]
    fn test_icon_state_is_normal() {
        assert!(IconState::Normal.is_normal());
        assert!(!IconState::Disabled.is_normal());
        assert!(!IconState::Active.is_normal());
    }

    #[test]
    fn test_icon_state_is_disabled() {
        assert!(IconState::Disabled.is_disabled());
        assert!(!IconState::Normal.is_disabled());
        assert!(!IconState::Active.is_disabled());
    }

    #[test]
    fn test_icon_state_is_interactive() {
        assert!(IconState::Active.is_interactive());
        assert!(IconState::Selected.is_interactive());
        assert!(IconState::Focused.is_interactive());
        assert!(!IconState::Normal.is_interactive());
        assert!(!IconState::Disabled.is_interactive());
    }

    // ========================================================================
    // SizedIconSet Tests
    // ========================================================================

    #[test]
    fn test_sized_icon_set_empty() {
        let set = SizedIconSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_sized_icon_set_add() {
        let mut set = SizedIconSet::new();
        set.add(IconSize::Size16, IconSource::Path("icon16.png".into()));
        set.add(IconSize::Size24, IconSource::Path("icon24.png".into()));

        assert!(!set.is_empty());
        assert_eq!(set.len(), 2);
        assert!(set.get(IconSize::Size16).is_some());
        assert!(set.get(IconSize::Size24).is_some());
        assert!(set.get(IconSize::Size48).is_none());
    }

    #[test]
    fn test_sized_icon_set_best_for_pixels() {
        let set = SizedIconSet::new()
            .with(IconSize::Size16, IconSource::Path("16.png".into()))
            .with(IconSize::Size32, IconSource::Path("32.png".into()))
            .with(IconSize::Size64, IconSource::Path("64.png".into()));

        // Exact match
        let (size, _) = set.best_for_pixels(16).unwrap();
        assert_eq!(size, IconSize::Size16);

        // Prefer larger (scale down)
        let (size, _) = set.best_for_pixels(20).unwrap();
        assert_eq!(size, IconSize::Size32);

        // Fall back to largest
        let (size, _) = set.best_for_pixels(100).unwrap();
        assert_eq!(size, IconSize::Size64);
    }

    // ========================================================================
    // StatefulIconSet Tests
    // ========================================================================

    #[test]
    fn test_stateful_icon_set_normal() {
        let set = StatefulIconSet::new(IconSource::Path("normal.png".into()));
        assert!(set.has_state(IconState::Normal));
        assert!(!set.has_state(IconState::Disabled));
    }

    #[test]
    fn test_stateful_icon_set_fallback() {
        let set = StatefulIconSet::new(IconSource::Path("normal.png".into()));

        // Should fall back to normal for any state
        assert!(set.get(IconState::Normal).path().is_some());
        assert!(set.get(IconState::Disabled).path().is_some()); // Falls back
        assert!(set.get(IconState::Active).path().is_some()); // Falls back
    }

    #[test]
    fn test_stateful_icon_set_with_variants() {
        let set = StatefulIconSet::new(IconSource::Path("normal.png".into()))
            .with_state(IconState::Disabled, IconSource::Path("disabled.png".into()))
            .with_state(IconState::Active, IconSource::Path("active.png".into()));

        assert!(set.has_state(IconState::Disabled));
        assert!(set.has_state(IconState::Active));
        assert!(!set.has_state(IconState::Selected));

        // Exact lookup
        assert!(set.get_exact(IconState::Disabled).is_some());
        assert!(set.get_exact(IconState::Selected).is_none());
    }

    // ========================================================================
    // ThemedIconSet Tests
    // ========================================================================

    #[test]
    fn test_themed_icon_set_light_only() {
        let set = ThemedIconSet::new(IconSource::Path("light.png".into()));

        assert!(set.has_mode(IconThemeMode::Light));
        assert!(!set.has_mode(IconThemeMode::Dark));
        assert!(!set.has_mode(IconThemeMode::HighContrast));

        // All modes fall back to light
        assert!(set.for_mode(IconThemeMode::Light).path().is_some());
        assert!(set.for_mode(IconThemeMode::Dark).path().is_some());
        assert!(set.for_mode(IconThemeMode::HighContrast).path().is_some());
    }

    #[test]
    fn test_themed_icon_set_with_dark() {
        let set = ThemedIconSet::new(IconSource::Path("light.png".into()))
            .with_dark(IconSource::Path("dark.png".into()));

        assert!(set.has_mode(IconThemeMode::Dark));
        assert!(set.dark().is_some());
    }

    #[test]
    fn test_themed_icon_set_high_contrast_fallback() {
        // High contrast falls back to dark if available
        let set = ThemedIconSet::new(IconSource::Path("light.png".into()))
            .with_dark(IconSource::Path("dark.png".into()));

        // High contrast should use dark since we don't have specific high contrast
        let hc_source = set.for_mode(IconThemeMode::HighContrast);
        let dark_source = set.for_mode(IconThemeMode::Dark);
        // Both should point to the same source (dark)
        assert_eq!(hc_source.path(), dark_source.path());
    }

    // ========================================================================
    // New Tint Function Tests
    // ========================================================================

    #[test]
    fn test_icon_tint_for_state_full_normal() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state_full(base, IconState::Normal);
        assert_eq!(tinted, base);
    }

    #[test]
    fn test_icon_tint_for_state_full_disabled() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state_full(base, IconState::Disabled);
        assert!(tinted.a < base.a); // Should be more transparent
        assert!((tinted.a - 0.4).abs() < 0.001); // 40% opacity
    }

    #[test]
    fn test_icon_tint_for_state_full_active() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state_full(base, IconState::Active);
        assert!(tinted.r < base.r); // Should be darker
        assert!((tinted.r - 0.7).abs() < 0.001); // 70% brightness
    }

    #[test]
    fn test_icon_tint_for_state_full_selected() {
        let base = Color::WHITE;
        let tinted = icon_tint_for_state_full(base, IconState::Selected);
        // Red and green should be slightly reduced, blue slightly increased
        assert!(tinted.r < base.r);
        assert!(tinted.g < base.g);
    }

    #[test]
    fn test_icon_tint_with_hover() {
        let base = Color::new(0.5, 0.5, 0.5, 1.0);
        let normal_no_hover = icon_tint_for_state_with_hover(base, IconState::Normal, false);
        let normal_with_hover = icon_tint_for_state_with_hover(base, IconState::Normal, true);

        assert_eq!(normal_no_hover, base);
        assert!(normal_with_hover.r > base.r); // Should be brighter

        // Disabled should not change with hover
        let disabled_no_hover = icon_tint_for_state_with_hover(base, IconState::Disabled, false);
        let disabled_with_hover = icon_tint_for_state_with_hover(base, IconState::Disabled, true);
        assert_eq!(disabled_no_hover, disabled_with_hover);
    }

    // ========================================================================
    // Icon Variant Builder Tests
    // ========================================================================

    #[test]
    fn test_icon_with_size_variants() {
        let icon = Icon::from_path("base.png")
            .with_size_variant(IconSize::Size24, IconSource::Path("24.png".into()))
            .with_size_variant(IconSize::Size32, IconSource::Path("32.png".into()));

        assert!(icon.has_size_variants());
        assert!(icon.sized_variants().is_some());
        assert_eq!(icon.sized_variants().unwrap().len(), 2);
    }

    #[test]
    fn test_icon_with_state_variants() {
        let icon = Icon::from_path("normal.png")
            .with_state_variant(IconState::Disabled, IconSource::Path("disabled.png".into()));

        assert!(icon.has_state_variant(IconState::Disabled));
        assert!(icon.has_state_variant(IconState::Normal)); // Always has normal
        assert!(!icon.has_state_variant(IconState::Active));
    }

    #[test]
    fn test_icon_with_themed_variants() {
        let icon = Icon::from_path("light.png")
            .with_dark_variant(IconSource::Path("dark.png".into()));

        assert!(icon.has_theme_variant(IconThemeMode::Light));
        assert!(icon.has_theme_variant(IconThemeMode::Dark));
        assert!(!icon.has_theme_variant(IconThemeMode::HighContrast));
    }
}
