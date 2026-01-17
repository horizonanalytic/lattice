//! Native dialog abstraction layer.
//!
//! This module provides platform-specific implementations of native file dialogs,
//! message boxes, color pickers, and font selection dialogs.
//!
//! The module automatically selects the appropriate backend based on the target platform:
//! - macOS: Uses AppKit (NSOpenPanel, NSAlert, NSColorPanel, NSFontPanel)
//! - Windows: Uses Common Dialogs (IFileDialog, TaskDialog, ChooseColor, ChooseFont)
//! - Linux: Uses XDG Desktop Portal (with GTK fallback)
//!
//! When native dialogs are not available or fail, callers should fall back to
//! the custom implementations provided by the framework.

use std::path::PathBuf;

use horizon_lattice_render::Color;

// Platform-specific implementations
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod stub;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use stub as platform;

// ============================================================================
// Native File Dialog Types
// ============================================================================

/// Filter specification for file dialogs.
#[derive(Debug, Clone)]
pub struct NativeFileFilter {
    /// Display name for the filter (e.g., "Image Files").
    pub name: String,
    /// File extensions to match (without leading dot, e.g., "png", "jpg").
    pub extensions: Vec<String>,
}

impl NativeFileFilter {
    /// Create a new file filter.
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Create an "All Files" filter.
    pub fn all_files() -> Self {
        Self::new("All Files", &["*"])
    }
}

/// Options for native file dialogs.
#[derive(Debug, Clone, Default)]
pub struct NativeFileDialogOptions {
    /// Dialog title.
    pub title: Option<String>,
    /// Initial directory.
    pub directory: Option<PathBuf>,
    /// File filters.
    pub filters: Vec<NativeFileFilter>,
    /// Allow multiple file selection (for open dialogs).
    pub multiple: bool,
    /// Default filename (for save dialogs).
    pub default_name: Option<String>,
}

impl NativeFileDialogOptions {
    /// Create new options with a title.
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    /// Set the initial directory.
    pub fn directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.directory = Some(path.into());
        self
    }

    /// Add a file filter.
    pub fn filter(mut self, filter: NativeFileFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Allow multiple file selection.
    pub fn multiple(mut self, allow: bool) -> Self {
        self.multiple = allow;
        self
    }

    /// Set the default filename for save dialogs.
    pub fn default_name(mut self, name: impl Into<String>) -> Self {
        self.default_name = Some(name.into());
        self
    }
}

// ============================================================================
// Native Message Dialog Types
// ============================================================================

/// Icon type for message dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeMessageLevel {
    /// Informational message.
    #[default]
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Button configuration for message dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeMessageButtons {
    /// Single OK button.
    #[default]
    Ok,
    /// OK and Cancel buttons.
    OkCancel,
    /// Yes and No buttons.
    YesNo,
    /// Yes, No, and Cancel buttons.
    YesNoCancel,
}

/// Result from a message dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeMessageResult {
    /// OK/Yes was clicked.
    Ok,
    /// Cancel was clicked or dialog was closed.
    Cancel,
    /// Yes was clicked.
    Yes,
    /// No was clicked.
    No,
}

/// Options for native message dialogs.
#[derive(Debug, Clone, Default)]
pub struct NativeMessageOptions {
    /// Dialog title.
    pub title: Option<String>,
    /// Main message text.
    pub message: String,
    /// Additional detail text.
    pub detail: Option<String>,
    /// Message level/icon.
    pub level: NativeMessageLevel,
    /// Button configuration.
    pub buttons: NativeMessageButtons,
}

impl NativeMessageOptions {
    /// Create new message options.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Default::default()
        }
    }

    /// Set the dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the detail text.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the message level/icon.
    pub fn level(mut self, level: NativeMessageLevel) -> Self {
        self.level = level;
        self
    }

    /// Set the button configuration.
    pub fn buttons(mut self, buttons: NativeMessageButtons) -> Self {
        self.buttons = buttons;
        self
    }
}

// ============================================================================
// Native Color Dialog Types
// ============================================================================

/// Options for native color dialogs.
#[derive(Debug, Clone, Default)]
pub struct NativeColorOptions {
    /// Initial color.
    pub initial_color: Option<Color>,
    /// Whether to show alpha channel.
    pub show_alpha: bool,
    /// Dialog title.
    pub title: Option<String>,
}

impl NativeColorOptions {
    /// Create new color dialog options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial color.
    pub fn initial_color(mut self, color: Color) -> Self {
        self.initial_color = Some(color);
        self
    }

    /// Enable or disable alpha channel.
    pub fn show_alpha(mut self, show: bool) -> Self {
        self.show_alpha = show;
        self
    }

    /// Set the dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

// ============================================================================
// Native Font Dialog Types
// ============================================================================

/// Simplified font description for native dialogs.
#[derive(Debug, Clone)]
pub struct NativeFontDesc {
    /// Font family name.
    pub family: String,
    /// Font size in points.
    pub size: f32,
    /// Whether the font is bold.
    pub bold: bool,
    /// Whether the font is italic.
    pub italic: bool,
}

impl Default for NativeFontDesc {
    fn default() -> Self {
        Self {
            family: "sans-serif".to_string(),
            size: 12.0,
            bold: false,
            italic: false,
        }
    }
}

impl NativeFontDesc {
    /// Create a new font description.
    pub fn new(family: impl Into<String>, size: f32) -> Self {
        Self {
            family: family.into(),
            size,
            bold: false,
            italic: false,
        }
    }

    /// Set whether the font is bold.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Set whether the font is italic.
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
}

/// Options for native font dialogs.
#[derive(Debug, Clone, Default)]
pub struct NativeFontOptions {
    /// Initial font.
    pub initial_font: Option<NativeFontDesc>,
    /// Dialog title.
    pub title: Option<String>,
    /// Whether to show only monospace fonts.
    pub monospace_only: bool,
}

impl NativeFontOptions {
    /// Create new font dialog options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial font.
    pub fn initial_font(mut self, font: NativeFontDesc) -> Self {
        self.initial_font = Some(font);
        self
    }

    /// Set the dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show only monospace fonts.
    pub fn monospace_only(mut self, only: bool) -> Self {
        self.monospace_only = only;
        self
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Check if native dialogs are available on this platform.
pub fn is_available() -> bool {
    platform::is_available()
}

/// Open a native file open dialog.
///
/// Returns `None` if the user cancels or if native dialogs are not available.
pub fn open_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    platform::open_file(options)
}

/// Open a native file open dialog for multiple files.
///
/// Returns `None` if the user cancels or if native dialogs are not available.
pub fn open_files(options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    platform::open_files(options)
}

/// Open a native file save dialog.
///
/// Returns `None` if the user cancels or if native dialogs are not available.
pub fn save_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    platform::save_file(options)
}

/// Open a native directory selection dialog.
///
/// Returns `None` if the user cancels or if native dialogs are not available.
pub fn select_directory(options: NativeFileDialogOptions) -> Option<PathBuf> {
    platform::select_directory(options)
}

/// Show a native message dialog.
///
/// Returns the user's response, or `None` if native dialogs are not available.
pub fn show_message(options: NativeMessageOptions) -> Option<NativeMessageResult> {
    platform::show_message(options)
}

/// Show a native color picker dialog.
///
/// Returns the selected color, or `None` if the user cancels or native dialogs
/// are not available.
pub fn pick_color(options: NativeColorOptions) -> Option<Color> {
    platform::pick_color(options)
}

/// Show a native font selection dialog.
///
/// Returns the selected font, or `None` if the user cancels or native dialogs
/// are not available.
pub fn pick_font(options: NativeFontOptions) -> Option<NativeFontDesc> {
    platform::pick_font(options)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_filter_creation() {
        let filter = NativeFileFilter::new("Images", &["png", "jpg", "gif"]);
        assert_eq!(filter.name, "Images");
        assert_eq!(filter.extensions, vec!["png", "jpg", "gif"]);
    }

    #[test]
    fn test_file_filter_all_files() {
        let filter = NativeFileFilter::all_files();
        assert_eq!(filter.name, "All Files");
        assert_eq!(filter.extensions, vec!["*"]);
    }

    #[test]
    fn test_file_dialog_options_builder() {
        let options = NativeFileDialogOptions::with_title("Open Image")
            .directory("/home/user")
            .filter(NativeFileFilter::new("Images", &["png", "jpg"]))
            .multiple(true);

        assert_eq!(options.title, Some("Open Image".to_string()));
        assert_eq!(options.directory, Some(PathBuf::from("/home/user")));
        assert!(options.multiple);
        assert_eq!(options.filters.len(), 1);
    }

    #[test]
    fn test_file_dialog_options_default_name() {
        let options = NativeFileDialogOptions::with_title("Save")
            .default_name("document.txt");

        assert_eq!(options.default_name, Some("document.txt".to_string()));
    }

    #[test]
    fn test_message_options_builder() {
        let options = NativeMessageOptions::new("Are you sure?")
            .title("Confirm")
            .detail("This action cannot be undone.")
            .level(NativeMessageLevel::Warning)
            .buttons(NativeMessageButtons::YesNo);

        assert_eq!(options.message, "Are you sure?");
        assert_eq!(options.title, Some("Confirm".to_string()));
        assert_eq!(options.level, NativeMessageLevel::Warning);
        assert_eq!(options.buttons, NativeMessageButtons::YesNo);
    }

    #[test]
    fn test_message_level_default() {
        let level: NativeMessageLevel = Default::default();
        assert_eq!(level, NativeMessageLevel::Info);
    }

    #[test]
    fn test_message_buttons_default() {
        let buttons: NativeMessageButtons = Default::default();
        assert_eq!(buttons, NativeMessageButtons::Ok);
    }

    #[test]
    fn test_color_options_builder() {
        let color = Color::from_rgb8(255, 128, 64);
        let options = NativeColorOptions::new()
            .initial_color(color)
            .show_alpha(true)
            .title("Pick a color");

        assert!(options.initial_color.is_some());
        assert!(options.show_alpha);
        assert_eq!(options.title, Some("Pick a color".to_string()));
    }

    #[test]
    fn test_font_desc_creation() {
        let font = NativeFontDesc::new("Arial", 14.0)
            .bold(true)
            .italic(false);

        assert_eq!(font.family, "Arial");
        assert!((font.size - 14.0).abs() < 0.01);
        assert!(font.bold);
        assert!(!font.italic);
    }

    #[test]
    fn test_font_desc_default() {
        let font = NativeFontDesc::default();
        assert_eq!(font.family, "sans-serif");
        assert!((font.size - 12.0).abs() < 0.01);
        assert!(!font.bold);
        assert!(!font.italic);
    }

    #[test]
    fn test_font_options_builder() {
        let font = NativeFontDesc::new("Courier New", 12.0);
        let options = NativeFontOptions::new()
            .initial_font(font)
            .title("Select Font")
            .monospace_only(true);

        assert!(options.initial_font.is_some());
        assert_eq!(options.title, Some("Select Font".to_string()));
        assert!(options.monospace_only);
    }

    #[test]
    fn test_availability_check() {
        // This should not panic regardless of platform
        let _ = is_available();
    }

    #[test]
    fn test_message_result_variants() {
        // Verify all result variants exist and can be compared
        assert_eq!(NativeMessageResult::Ok, NativeMessageResult::Ok);
        assert_eq!(NativeMessageResult::Cancel, NativeMessageResult::Cancel);
        assert_eq!(NativeMessageResult::Yes, NativeMessageResult::Yes);
        assert_eq!(NativeMessageResult::No, NativeMessageResult::No);
        assert_ne!(NativeMessageResult::Ok, NativeMessageResult::Cancel);
    }
}
