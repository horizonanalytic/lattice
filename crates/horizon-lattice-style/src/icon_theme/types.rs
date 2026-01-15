//! Core types for the icon theme system.
//!
//! This module provides the fundamental types for icon theming, including:
//! - Standard icon names following freedesktop naming conventions
//! - Icon context categories (Actions, Places, etc.)
//! - Theme metadata structures
//! - Icon lookup parameters

use std::path::PathBuf;

use horizon_lattice_render::IconSize;

/// Standard icon name following freedesktop naming conventions.
///
/// Icon names use lowercase with hyphens, organized by semantic meaning
/// rather than visual appearance. For example, "document-new" represents
/// the action of creating a new document.
///
/// # Examples
///
/// ```
/// use horizon_lattice_style::icon_theme::IconName;
///
/// let icon = IconName::new("document-save");
/// assert_eq!(icon.as_str(), "document-save");
///
/// // Using a standard constant
/// let icon = IconName::new(IconName::EDIT_COPY);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconName(String);

impl IconName {
    // ========================================================================
    // Standard Action Icons
    // ========================================================================

    /// Create a new document
    pub const DOCUMENT_NEW: &'static str = "document-new";
    /// Open a document
    pub const DOCUMENT_OPEN: &'static str = "document-open";
    /// Save the current document
    pub const DOCUMENT_SAVE: &'static str = "document-save";
    /// Save document with a new name
    pub const DOCUMENT_SAVE_AS: &'static str = "document-save-as";
    /// Print the document
    pub const DOCUMENT_PRINT: &'static str = "document-print";
    /// Preview before printing
    pub const DOCUMENT_PRINT_PREVIEW: &'static str = "document-print-preview";
    /// Document properties
    pub const DOCUMENT_PROPERTIES: &'static str = "document-properties";
    /// Close the document
    pub const DOCUMENT_CLOSE: &'static str = "document-close";
    /// Revert to saved version
    pub const DOCUMENT_REVERT: &'static str = "document-revert";

    /// Copy selection
    pub const EDIT_COPY: &'static str = "edit-copy";
    /// Cut selection
    pub const EDIT_CUT: &'static str = "edit-cut";
    /// Paste from clipboard
    pub const EDIT_PASTE: &'static str = "edit-paste";
    /// Undo last action
    pub const EDIT_UNDO: &'static str = "edit-undo";
    /// Redo last undone action
    pub const EDIT_REDO: &'static str = "edit-redo";
    /// Delete selection
    pub const EDIT_DELETE: &'static str = "edit-delete";
    /// Find in document
    pub const EDIT_FIND: &'static str = "edit-find";
    /// Find and replace
    pub const EDIT_FIND_REPLACE: &'static str = "edit-find-replace";
    /// Select all
    pub const EDIT_SELECT_ALL: &'static str = "edit-select-all";
    /// Clear selection
    pub const EDIT_CLEAR: &'static str = "edit-clear";

    /// Navigate to home
    pub const GO_HOME: &'static str = "go-home";
    /// Navigate up one level
    pub const GO_UP: &'static str = "go-up";
    /// Navigate down
    pub const GO_DOWN: &'static str = "go-down";
    /// Navigate to first
    pub const GO_FIRST: &'static str = "go-first";
    /// Navigate to last
    pub const GO_LAST: &'static str = "go-last";
    /// Navigate to next
    pub const GO_NEXT: &'static str = "go-next";
    /// Navigate to previous
    pub const GO_PREVIOUS: &'static str = "go-previous";
    /// Jump forward
    pub const GO_JUMP: &'static str = "go-jump";

    /// Refresh/reload view
    pub const VIEW_REFRESH: &'static str = "view-refresh";
    /// Enter fullscreen
    pub const VIEW_FULLSCREEN: &'static str = "view-fullscreen";
    /// Sort ascending
    pub const VIEW_SORT_ASCENDING: &'static str = "view-sort-ascending";
    /// Sort descending
    pub const VIEW_SORT_DESCENDING: &'static str = "view-sort-descending";

    /// Zoom in
    pub const ZOOM_IN: &'static str = "zoom-in";
    /// Zoom out
    pub const ZOOM_OUT: &'static str = "zoom-out";
    /// Fit to window
    pub const ZOOM_FIT_BEST: &'static str = "zoom-fit-best";
    /// Original/100% size
    pub const ZOOM_ORIGINAL: &'static str = "zoom-original";

    /// Bold text
    pub const FORMAT_TEXT_BOLD: &'static str = "format-text-bold";
    /// Italic text
    pub const FORMAT_TEXT_ITALIC: &'static str = "format-text-italic";
    /// Underline text
    pub const FORMAT_TEXT_UNDERLINE: &'static str = "format-text-underline";
    /// Strikethrough text
    pub const FORMAT_TEXT_STRIKETHROUGH: &'static str = "format-text-strikethrough";

    /// Left align
    pub const FORMAT_JUSTIFY_LEFT: &'static str = "format-justify-left";
    /// Center align
    pub const FORMAT_JUSTIFY_CENTER: &'static str = "format-justify-center";
    /// Right align
    pub const FORMAT_JUSTIFY_RIGHT: &'static str = "format-justify-right";
    /// Full justify
    pub const FORMAT_JUSTIFY_FILL: &'static str = "format-justify-fill";

    /// Add to list
    pub const LIST_ADD: &'static str = "list-add";
    /// Remove from list
    pub const LIST_REMOVE: &'static str = "list-remove";

    /// Generic close action
    pub const WINDOW_CLOSE: &'static str = "window-close";
    /// Maximize window
    pub const WINDOW_MAXIMIZE: &'static str = "window-maximize";
    /// Minimize window
    pub const WINDOW_MINIMIZE: &'static str = "window-minimize";
    /// New window
    pub const WINDOW_NEW: &'static str = "window-new";

    /// Help/documentation
    pub const HELP_ABOUT: &'static str = "help-about";
    /// Help contents
    pub const HELP_CONTENTS: &'static str = "help-contents";

    /// Application quit
    pub const APPLICATION_EXIT: &'static str = "application-exit";

    // ========================================================================
    // Standard Status Icons
    // ========================================================================

    /// Error dialog
    pub const DIALOG_ERROR: &'static str = "dialog-error";
    /// Information dialog
    pub const DIALOG_INFORMATION: &'static str = "dialog-information";
    /// Warning dialog
    pub const DIALOG_WARNING: &'static str = "dialog-warning";
    /// Question dialog
    pub const DIALOG_QUESTION: &'static str = "dialog-question";
    /// Password/authentication
    pub const DIALOG_PASSWORD: &'static str = "dialog-password";

    // ========================================================================
    // Standard Place Icons
    // ========================================================================

    /// Generic folder
    pub const FOLDER: &'static str = "folder";
    /// Open folder
    pub const FOLDER_OPEN: &'static str = "folder-open";
    /// User's home directory
    pub const USER_HOME: &'static str = "user-home";
    /// Trash/recycle bin
    pub const USER_TRASH: &'static str = "user-trash";
    /// Desktop folder
    pub const USER_DESKTOP: &'static str = "user-desktop";
    /// Network location
    pub const NETWORK_WORKGROUP: &'static str = "network-workgroup";

    // ========================================================================
    // Standard Device Icons
    // ========================================================================

    /// Generic computer
    pub const COMPUTER: &'static str = "computer";
    /// Hard disk drive
    pub const DRIVE_HARDDISK: &'static str = "drive-harddisk";
    /// Optical drive (CD/DVD)
    pub const DRIVE_OPTICAL: &'static str = "drive-optical";
    /// Removable media
    pub const DRIVE_REMOVABLE_MEDIA: &'static str = "drive-removable-media";
    /// Printer device
    pub const PRINTER: &'static str = "printer";

    // ========================================================================
    // Methods
    // ========================================================================

    /// Create a new icon name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the icon name as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the context category for this icon name.
    ///
    /// Determines the context based on the name prefix.
    pub fn context(&self) -> IconContext {
        let name = self.as_str();
        if name.starts_with("document-")
            || name.starts_with("edit-")
            || name.starts_with("go-")
            || name.starts_with("view-")
            || name.starts_with("zoom-")
            || name.starts_with("format-")
            || name.starts_with("list-")
            || name.starts_with("window-")
            || name.starts_with("help-")
            || name.starts_with("application-")
        {
            IconContext::Actions
        } else if name.starts_with("dialog-") {
            IconContext::Status
        } else if name.starts_with("folder") || name.starts_with("user-") || name.starts_with("network-") {
            IconContext::Places
        } else if name.starts_with("drive-") || name.starts_with("computer") || name.starts_with("printer") {
            IconContext::Devices
        } else if name.starts_with("emblem-") {
            IconContext::Emblems
        } else if name.contains('/') || name.starts_with("application-") || name.starts_with("text-") {
            IconContext::MimeTypes
        } else {
            IconContext::Actions // Default
        }
    }
}

impl From<&str> for IconName {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for IconName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for IconName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Icon context/category following freedesktop specification.
///
/// Contexts help organize icons by their purpose and determine
/// where to search for icons within a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconContext {
    /// Icons for user actions (copy, save, etc.)
    Actions,
    /// Loading and processing animations
    Animations,
    /// Application icons
    Applications,
    /// Program menu categories
    Categories,
    /// Hardware devices
    Devices,
    /// File/folder emblems and tags
    Emblems,
    /// Emoticons for chat
    Emotes,
    /// Country flags
    International,
    /// File type icons (MIME types)
    MimeTypes,
    /// Filesystem locations
    Places,
    /// System status indicators
    Status,
}

impl IconContext {
    /// Get the freedesktop context name.
    pub fn as_str(&self) -> &'static str {
        match self {
            IconContext::Actions => "actions",
            IconContext::Animations => "animations",
            IconContext::Applications => "apps",
            IconContext::Categories => "categories",
            IconContext::Devices => "devices",
            IconContext::Emblems => "emblems",
            IconContext::Emotes => "emotes",
            IconContext::International => "intl",
            IconContext::MimeTypes => "mimetypes",
            IconContext::Places => "places",
            IconContext::Status => "status",
        }
    }

    /// Parse a context from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "actions" => Some(IconContext::Actions),
            "animations" => Some(IconContext::Animations),
            "apps" | "applications" => Some(IconContext::Applications),
            "categories" => Some(IconContext::Categories),
            "devices" => Some(IconContext::Devices),
            "emblems" => Some(IconContext::Emblems),
            "emotes" => Some(IconContext::Emotes),
            "intl" | "international" => Some(IconContext::International),
            "mimetypes" | "mime-types" => Some(IconContext::MimeTypes),
            "places" => Some(IconContext::Places),
            "status" => Some(IconContext::Status),
            _ => None,
        }
    }
}

/// Size type for icon theme directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSizeType {
    /// Fixed size icons - must match exactly
    Fixed,
    /// Scalable icons (typically SVG)
    Scalable,
    /// Threshold-based sizing - matches within a range
    Threshold,
}

impl Default for IconSizeType {
    fn default() -> Self {
        IconSizeType::Threshold
    }
}

/// Information about an icon theme directory.
#[derive(Debug, Clone)]
pub struct IconThemeDirectory {
    /// Directory path relative to theme root
    pub path: String,
    /// Nominal icon size
    pub size: u32,
    /// Scale factor (1 for normal, 2 for HiDPI, etc.)
    pub scale: u32,
    /// Icon context
    pub context: Option<IconContext>,
    /// Size type
    pub size_type: IconSizeType,
    /// Minimum size (for Scalable)
    pub min_size: Option<u32>,
    /// Maximum size (for Scalable)
    pub max_size: Option<u32>,
    /// Size threshold (for Threshold type)
    pub threshold: u32,
}

impl IconThemeDirectory {
    /// Check if this directory matches a target size.
    pub fn matches_size(&self, target: u32, scale: u32) -> bool {
        if self.scale != scale {
            return false;
        }

        match self.size_type {
            IconSizeType::Fixed => self.size == target,
            IconSizeType::Scalable => {
                let min = self.min_size.unwrap_or(self.size);
                let max = self.max_size.unwrap_or(self.size);
                target >= min && target <= max
            }
            IconSizeType::Threshold => {
                let diff = (self.size as i32 - target as i32).unsigned_abs();
                diff <= self.threshold
            }
        }
    }

    /// Calculate size distance (for finding best match).
    pub fn size_distance(&self, target: u32) -> u32 {
        match self.size_type {
            IconSizeType::Fixed => {
                if self.size == target {
                    0
                } else {
                    u32::MAX
                }
            }
            IconSizeType::Scalable => {
                let min = self.min_size.unwrap_or(self.size);
                let max = self.max_size.unwrap_or(self.size);
                if target < min {
                    min - target
                } else if target > max {
                    target - max
                } else {
                    0
                }
            }
            IconSizeType::Threshold => (self.size as i32 - target as i32).unsigned_abs(),
        }
    }
}

/// Icon theme metadata.
#[derive(Debug, Clone)]
pub struct IconThemeInfo {
    /// Unique theme identifier (directory name)
    pub id: String,
    /// Human-readable theme name
    pub name: String,
    /// Theme description
    pub comment: Option<String>,
    /// Parent themes for inheritance (fallback chain)
    pub inherits: Vec<String>,
    /// Whether to hide from theme selection UI
    pub hidden: bool,
    /// Theme example icon name
    pub example: Option<String>,
    /// Theme directories
    pub directories: Vec<IconThemeDirectory>,
    /// Theme base paths (where theme was found)
    pub base_paths: Vec<PathBuf>,
}

impl IconThemeInfo {
    /// Create a new theme info with just an ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            comment: None,
            inherits: Vec::new(),
            hidden: false,
            example: None,
            directories: Vec::new(),
            base_paths: Vec::new(),
        }
    }

    /// Get available sizes in this theme.
    pub fn available_sizes(&self) -> Vec<IconSize> {
        let mut sizes: Vec<_> = self
            .directories
            .iter()
            .filter_map(|d| IconSize::from_pixels(d.size))
            .collect();
        sizes.sort();
        sizes.dedup();
        sizes
    }

    /// Check if this theme has scalable icons.
    pub fn has_scalable(&self) -> bool {
        self.directories
            .iter()
            .any(|d| d.size_type == IconSizeType::Scalable)
    }

    /// Find directories matching a size and context.
    pub fn find_directories(
        &self,
        size: u32,
        scale: u32,
        context: Option<IconContext>,
    ) -> Vec<&IconThemeDirectory> {
        self.directories
            .iter()
            .filter(|d| {
                d.matches_size(size, scale)
                    && (context.is_none() || d.context == context || d.context.is_none())
            })
            .collect()
    }
}

/// Parameters for icon lookup.
#[derive(Debug, Clone)]
pub struct IconLookup {
    /// Icon name to look up
    pub name: IconName,
    /// Desired icon size
    pub size: IconSize,
    /// Scale factor (1 for normal, 2 for HiDPI)
    pub scale: u32,
    /// Specific context to search
    pub context: Option<IconContext>,
    /// Force exact size match
    pub force_size: bool,
}

impl IconLookup {
    /// Create a new lookup for an icon name and size.
    pub fn new(name: impl Into<IconName>, size: IconSize) -> Self {
        Self {
            name: name.into(),
            size,
            scale: 1,
            context: None,
            force_size: false,
        }
    }

    /// Set the scale factor.
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale;
        self
    }

    /// Set the context.
    pub fn with_context(mut self, context: IconContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Force exact size match.
    pub fn with_exact_size(mut self) -> Self {
        self.force_size = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_name_constants() {
        let icon = IconName::new(IconName::DOCUMENT_SAVE);
        assert_eq!(icon.as_str(), "document-save");
    }

    #[test]
    fn test_icon_name_context() {
        assert_eq!(
            IconName::new("document-save").context(),
            IconContext::Actions
        );
        assert_eq!(IconName::new("dialog-error").context(), IconContext::Status);
        assert_eq!(IconName::new("folder").context(), IconContext::Places);
        assert_eq!(
            IconName::new("drive-harddisk").context(),
            IconContext::Devices
        );
    }

    #[test]
    fn test_icon_context_str() {
        assert_eq!(IconContext::Actions.as_str(), "actions");
        assert_eq!(IconContext::Applications.as_str(), "apps");
        assert_eq!(IconContext::MimeTypes.as_str(), "mimetypes");
    }

    #[test]
    fn test_icon_context_from_str() {
        assert_eq!(IconContext::from_str("actions"), Some(IconContext::Actions));
        assert_eq!(
            IconContext::from_str("apps"),
            Some(IconContext::Applications)
        );
        assert_eq!(
            IconContext::from_str("applications"),
            Some(IconContext::Applications)
        );
        assert_eq!(IconContext::from_str("unknown"), None);
    }

    #[test]
    fn test_theme_directory_matches_size() {
        let fixed = IconThemeDirectory {
            path: "16x16/actions".to_string(),
            size: 16,
            scale: 1,
            context: Some(IconContext::Actions),
            size_type: IconSizeType::Fixed,
            min_size: None,
            max_size: None,
            threshold: 2,
        };

        assert!(fixed.matches_size(16, 1));
        assert!(!fixed.matches_size(24, 1));
        assert!(!fixed.matches_size(16, 2)); // Wrong scale

        let scalable = IconThemeDirectory {
            path: "scalable/actions".to_string(),
            size: 48,
            scale: 1,
            context: Some(IconContext::Actions),
            size_type: IconSizeType::Scalable,
            min_size: Some(16),
            max_size: Some(256),
            threshold: 2,
        };

        assert!(scalable.matches_size(16, 1));
        assert!(scalable.matches_size(48, 1));
        assert!(scalable.matches_size(256, 1));
        assert!(!scalable.matches_size(512, 1));

        let threshold = IconThemeDirectory {
            path: "22x22/actions".to_string(),
            size: 22,
            scale: 1,
            context: Some(IconContext::Actions),
            size_type: IconSizeType::Threshold,
            min_size: None,
            max_size: None,
            threshold: 2,
        };

        assert!(threshold.matches_size(20, 1));
        assert!(threshold.matches_size(22, 1));
        assert!(threshold.matches_size(24, 1));
        assert!(!threshold.matches_size(16, 1));
    }

    #[test]
    fn test_icon_lookup() {
        let lookup = IconLookup::new("document-save", IconSize::Size24)
            .with_scale(2)
            .with_context(IconContext::Actions);

        assert_eq!(lookup.name.as_str(), "document-save");
        assert_eq!(lookup.size, IconSize::Size24);
        assert_eq!(lookup.scale, 2);
        assert_eq!(lookup.context, Some(IconContext::Actions));
    }
}
