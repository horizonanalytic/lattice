//! Window configuration and builder.
//!
//! This module provides `WindowConfig`, a builder for configuring
//! native window creation options.

use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::window::{Window, WindowAttributes, WindowButtons, WindowLevel};

use super::native_window::NativeWindowId;
use super::window_icon::WindowIcon;
use super::window_type::WindowType;
use crate::widget::widgets::WindowFlags;

/// Configuration for creating a native window.
///
/// `WindowConfig` provides a builder pattern for specifying all window
/// creation options, which can then be converted to winit `WindowAttributes`.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{WindowConfig, WindowType, WindowFlags};
///
/// let config = WindowConfig::new("My Application")
///     .with_type(WindowType::Normal)
///     .with_size(1280, 720)
///     .with_resizable(true)
///     .with_icon(icon);
///
/// // Create the window
/// let window = config.build(event_loop)?;
/// ```
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title.
    title: String,
    /// Window type.
    window_type: WindowType,
    /// Window flags (override type defaults if set).
    flags: Option<WindowFlags>,
    /// Initial window size (width, height) in logical pixels.
    size: Option<(u32, u32)>,
    /// Minimum window size.
    min_size: Option<(u32, u32)>,
    /// Maximum window size.
    max_size: Option<(u32, u32)>,
    /// Aspect ratio constraint (width / height).
    ///
    /// When set, the window will maintain this aspect ratio during resize operations.
    /// For example, a ratio of 16.0/9.0 ≈ 1.78 maintains a 16:9 aspect ratio.
    aspect_ratio: Option<f32>,
    /// Initial window position.
    position: Option<(i32, i32)>,
    /// Whether the window is resizable.
    resizable: Option<bool>,
    /// Whether the window has decorations (title bar, borders).
    decorations: Option<bool>,
    /// Whether the window is transparent.
    transparent: Option<bool>,
    /// Whether the window is visible on creation.
    visible: bool,
    /// Whether the window is maximized on creation.
    maximized: bool,
    /// Window icon.
    icon: Option<WindowIcon>,
    /// Parent window for transient windows (dialogs, tool windows).
    ///
    /// Transient windows:
    /// - Float independently of parent (not embedded)
    /// - Stay logically associated with parent for z-ordering
    /// - Close automatically when parent closes
    parent: Option<NativeWindowId>,
    /// Window level (z-ordering).
    level: Option<WindowLevel>,
}

impl WindowConfig {
    /// Create a new window configuration with the given title.
    ///
    /// The window type defaults to `WindowType::Normal`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            window_type: WindowType::Normal,
            flags: None,
            size: None,
            min_size: None,
            max_size: None,
            aspect_ratio: None,
            position: None,
            resizable: None,
            decorations: None,
            transparent: None,
            visible: true,
            maximized: false,
            icon: None,
            parent: None,
            level: None,
        }
    }

    /// Set the window type.
    ///
    /// The window type affects default behaviors like decorations,
    /// z-ordering, and taskbar presence.
    pub fn with_type(mut self, window_type: WindowType) -> Self {
        self.window_type = window_type;
        self
    }

    /// Set explicit window flags.
    ///
    /// If set, these override the default flags for the window type.
    pub fn with_flags(mut self, flags: WindowFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    /// Set the initial window size in logical pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = Some((width, height));
        self
    }

    /// Set the minimum window size in logical pixels.
    pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    /// Set the maximum window size in logical pixels.
    pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }

    /// Set the aspect ratio constraint (width / height).
    ///
    /// When set, the window will maintain this aspect ratio during user resize operations.
    /// The ratio should be positive (width divided by height).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 16:9 aspect ratio
    /// let config = WindowConfig::new("Video Player")
    ///     .with_aspect_ratio(16.0 / 9.0);
    ///
    /// // 4:3 aspect ratio
    /// let config = WindowConfig::new("Classic App")
    ///     .with_aspect_ratio(4.0 / 3.0);
    ///
    /// // Square window
    /// let config = WindowConfig::new("Square")
    ///     .with_aspect_ratio(1.0);
    /// ```
    pub fn with_aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    /// Set the initial window position.
    ///
    /// The position is relative to the top-left corner of the primary monitor.
    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.position = Some((x, y));
        self
    }

    /// Set whether the window is resizable.
    ///
    /// If not set, the default is determined by the window type.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    /// Set whether the window has decorations (title bar, borders).
    ///
    /// If not set, the default is determined by the window type.
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    /// Set whether the window has a transparent background.
    ///
    /// Transparent windows allow content behind them to show through
    /// where the window content is not opaque.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// Set whether the window is visible when created.
    ///
    /// Defaults to `true`. Set to `false` to create a hidden window
    /// that can be shown later.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set whether the window starts maximized.
    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Set the window icon.
    pub fn with_icon(mut self, icon: WindowIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the window level (z-ordering).
    ///
    /// - `WindowLevel::AlwaysOnBottom` - Below normal windows
    /// - `WindowLevel::Normal` - Normal z-order
    /// - `WindowLevel::AlwaysOnTop` - Above normal windows
    pub fn with_level(mut self, level: WindowLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Set the parent window for transient relationship.
    ///
    /// Transient windows:
    /// - Float independently of parent (not embedded inside parent)
    /// - Are logically associated with parent for z-ordering coordination
    /// - Close automatically when parent closes
    ///
    /// Use this for dialog windows, tool palettes, and other secondary windows
    /// that belong to a main window.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let dialog_config = WindowConfig::new("Save As")
    ///     .with_type(WindowType::Dialog)
    ///     .with_parent(main_window_id)
    ///     .with_size(400, 300);
    /// ```
    pub fn with_parent(mut self, parent: NativeWindowId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Get the window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the window type.
    pub fn window_type(&self) -> WindowType {
        self.window_type
    }

    /// Get the effective window flags.
    ///
    /// If explicit flags are set, returns those. Otherwise, returns
    /// the default flags for the window type.
    pub fn effective_flags(&self) -> WindowFlags {
        self.flags
            .unwrap_or_else(|| self.window_type.default_flags())
    }

    /// Get the aspect ratio constraint, if set.
    pub fn aspect_ratio(&self) -> Option<f32> {
        self.aspect_ratio
    }

    /// Get the parent window, if set.
    pub fn parent(&self) -> Option<NativeWindowId> {
        self.parent
    }

    /// Check if the window will have decorations (title bar, borders).
    ///
    /// This returns the effective decorations setting, considering:
    /// - Explicit `with_decorations()` setting
    /// - Window type default
    /// - Frameless flag
    pub fn has_decorations(&self) -> bool {
        let flags = self.effective_flags();
        self.decorations
            .unwrap_or_else(|| self.window_type.has_decorations() && !flags.is_frameless())
    }

    /// Convert to winit `WindowAttributes`.
    ///
    /// This creates the attributes needed to create a winit window.
    pub fn to_window_attributes(&self) -> WindowAttributes {
        let flags = self.effective_flags();
        let mut attrs = Window::default_attributes().with_title(&self.title);

        // Size
        if let Some((w, h)) = self.size {
            attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }

        if let Some((w, h)) = self.min_size {
            attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }

        if let Some((w, h)) = self.max_size {
            attrs = attrs.with_max_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }

        // Position
        if let Some((x, y)) = self.position {
            attrs =
                attrs.with_position(Position::Logical(LogicalPosition::new(x as f64, y as f64)));
        }

        // Resizable
        let resizable = self
            .resizable
            .unwrap_or_else(|| self.window_type.is_resizable() && flags.is_resizable());
        attrs = attrs.with_resizable(resizable);

        // Decorations
        let decorations = self
            .decorations
            .unwrap_or_else(|| self.window_type.has_decorations() && !flags.is_frameless());
        attrs = attrs.with_decorations(decorations);

        // Transparency
        let transparent = self.transparent.unwrap_or_else(|| flags.is_transparent());
        attrs = attrs.with_transparent(transparent);

        // Visibility
        attrs = attrs.with_visible(self.visible);

        // Maximized
        attrs = attrs.with_maximized(self.maximized);

        // Window level (z-ordering)
        let level = self.level.unwrap_or_else(|| {
            if flags.stays_on_top() || self.window_type.stays_on_top() {
                WindowLevel::AlwaysOnTop
            } else if flags.stays_on_bottom() {
                WindowLevel::AlwaysOnBottom
            } else {
                WindowLevel::Normal
            }
        });
        attrs = attrs.with_window_level(level);

        // Window buttons
        let mut buttons = WindowButtons::empty();
        if flags.has_close_button() {
            buttons |= WindowButtons::CLOSE;
        }
        if flags.has_minimize_button() {
            buttons |= WindowButtons::MINIMIZE;
        }
        if flags.has_maximize_button() {
            buttons |= WindowButtons::MAXIMIZE;
        }
        attrs = attrs.with_enabled_buttons(buttons);

        // Icon
        if let Some(ref icon) = self.icon
            && let Ok(winit_icon) = icon.to_winit_icon() {
                attrs = attrs.with_window_icon(Some(winit_icon));
            }

        attrs
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new("Horizon Lattice Window")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_defaults() {
        let config = WindowConfig::new("Test Window");
        assert_eq!(config.title(), "Test Window");
        assert_eq!(config.window_type(), WindowType::Normal);
        assert!(config.visible);
        assert!(!config.maximized);
    }

    #[test]
    fn test_window_config_builder() {
        let config = WindowConfig::new("Test")
            .with_type(WindowType::Dialog)
            .with_size(800, 600)
            .with_position(100, 100)
            .with_resizable(false)
            .with_decorations(true)
            .with_visible(false);

        assert_eq!(config.window_type(), WindowType::Dialog);
        assert_eq!(config.size, Some((800, 600)));
        assert_eq!(config.position, Some((100, 100)));
        assert_eq!(config.resizable, Some(false));
        assert_eq!(config.decorations, Some(true));
        assert!(!config.visible);
    }

    #[test]
    fn test_effective_flags() {
        // Default flags from window type
        let config = WindowConfig::new("Test").with_type(WindowType::Dialog);
        let flags = config.effective_flags();
        assert!(flags.has_close_button());
        assert!(!flags.has_minimize_button());
        assert!(!flags.has_maximize_button());

        // Explicit flags override type defaults
        let config = WindowConfig::new("Test")
            .with_type(WindowType::Dialog)
            .with_flags(WindowFlags::DEFAULT);
        let flags = config.effective_flags();
        assert!(flags.has_minimize_button());
        assert!(flags.has_maximize_button());
    }

    #[test]
    fn test_window_config_with_all_options() {
        let config = WindowConfig::new("Full Test")
            .with_type(WindowType::Tool)
            .with_size(400, 300)
            .with_min_size(200, 150)
            .with_max_size(800, 600)
            .with_position(50, 50)
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false)
            .with_visible(true)
            .with_maximized(false)
            .with_level(WindowLevel::AlwaysOnTop);

        assert_eq!(config.title(), "Full Test");
        assert_eq!(config.window_type(), WindowType::Tool);
        assert_eq!(config.size, Some((400, 300)));
        assert_eq!(config.min_size, Some((200, 150)));
        assert_eq!(config.max_size, Some((800, 600)));
        assert_eq!(config.position, Some((50, 50)));
        assert_eq!(config.level, Some(WindowLevel::AlwaysOnTop));
    }

    #[test]
    fn test_window_config_aspect_ratio() {
        // No aspect ratio by default
        let config = WindowConfig::new("Test");
        assert_eq!(config.aspect_ratio(), None);

        // Set 16:9 aspect ratio
        let config = WindowConfig::new("Video Player").with_aspect_ratio(16.0 / 9.0);
        let ratio = config.aspect_ratio().unwrap();
        assert!((ratio - 1.777).abs() < 0.01);

        // Set 4:3 aspect ratio
        let config = WindowConfig::new("Classic").with_aspect_ratio(4.0 / 3.0);
        let ratio = config.aspect_ratio().unwrap();
        assert!((ratio - 1.333).abs() < 0.01);

        // Set square aspect ratio
        let config = WindowConfig::new("Square").with_aspect_ratio(1.0);
        assert_eq!(config.aspect_ratio(), Some(1.0));
    }

    #[test]
    fn test_window_config_parent() {
        use std::mem::transmute;
        use winit::window::WindowId;

        // Helper to create a fake NativeWindowId for testing
        fn fake_id(n: u64) -> NativeWindowId {
            let fake_winit_id: WindowId = unsafe { transmute(n) };
            NativeWindowId::from_winit(fake_winit_id)
        }

        // No parent by default
        let config = WindowConfig::new("Test");
        assert_eq!(config.parent(), None);

        // Set parent for dialog
        let parent_id = fake_id(42);
        let config = WindowConfig::new("Dialog")
            .with_type(WindowType::Dialog)
            .with_parent(parent_id)
            .with_size(400, 300);

        assert_eq!(config.parent(), Some(parent_id));
        assert_eq!(config.window_type(), WindowType::Dialog);
        assert_eq!(config.size, Some((400, 300)));
    }
}
