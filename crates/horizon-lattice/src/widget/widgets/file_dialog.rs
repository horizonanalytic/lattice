//! FileDialog widget for file and directory selection.
//!
//! This module provides [`FileDialog`], a modal dialog for selecting files and
//! directories with full navigation capabilities.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{FileDialog, FileDialogMode, FileFilter};
//!
//! // Open a file dialog
//! let mut dialog = FileDialog::for_open()
//!     .with_title("Open File")
//!     .with_filter(FileFilter::new("Rust Files", &["*.rs"]))
//!     .with_filter(FileFilter::new("All Files", &["*"]));
//!
//! dialog.file_selected.connect(|path| {
//!     println!("Selected: {:?}", path);
//! });
//!
//! dialog.open();
//!
//! // Using static helpers
//! let path = FileDialog::get_open_file_name(
//!     "Select File",
//!     "/home/user",
//!     &[FileFilter::new("Text Files", &["*.txt"])],
//! );
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Size, Stroke};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, WheelEvent, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;

// ============================================================================
// FileDialogMode
// ============================================================================

/// The mode of operation for a FileDialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileDialogMode {
    /// Select a single file to open.
    #[default]
    OpenFile,

    /// Select multiple files to open.
    OpenFiles,

    /// Select a file to save to.
    SaveFile,

    /// Select a directory.
    Directory,
}

impl FileDialogMode {
    /// Check if this mode allows selecting multiple items.
    pub fn is_multi_select(&self) -> bool {
        matches!(self, FileDialogMode::OpenFiles)
    }

    /// Check if this mode is for opening (vs saving).
    pub fn is_open_mode(&self) -> bool {
        matches!(
            self,
            FileDialogMode::OpenFile | FileDialogMode::OpenFiles | FileDialogMode::Directory
        )
    }

    /// Check if this mode selects directories.
    pub fn is_directory_mode(&self) -> bool {
        matches!(self, FileDialogMode::Directory)
    }

    /// Get the appropriate accept button text for this mode.
    pub fn accept_button_text(&self) -> &'static str {
        match self {
            FileDialogMode::OpenFile | FileDialogMode::OpenFiles => "Open",
            FileDialogMode::SaveFile => "Save",
            FileDialogMode::Directory => "Select Folder",
        }
    }
}

// ============================================================================
// FileFilter
// ============================================================================

/// A file filter for restricting visible files in the dialog.
///
/// # Example
///
/// ```ignore
/// let filter = FileFilter::new("Images", &["*.png", "*.jpg", "*.gif"]);
/// let all_files = FileFilter::all_files();
/// ```
#[derive(Debug, Clone)]
pub struct FileFilter {
    /// Display name for the filter (e.g., "Image Files").
    pub name: String,

    /// Glob patterns for matching files (e.g., ["*.png", "*.jpg"]).
    pub patterns: Vec<String>,
}

impl FileFilter {
    /// Create a new file filter with a name and patterns.
    pub fn new(name: impl Into<String>, patterns: &[&str]) -> Self {
        Self {
            name: name.into(),
            patterns: patterns.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Create an "All Files" filter that matches everything.
    pub fn all_files() -> Self {
        Self::new("All Files", &["*"])
    }

    /// Create a filter for Rust source files.
    pub fn rust_files() -> Self {
        Self::new("Rust Files", &["*.rs"])
    }

    /// Create a filter for text files.
    pub fn text_files() -> Self {
        Self::new("Text Files", &["*.txt"])
    }

    /// Create a filter for image files.
    pub fn image_files() -> Self {
        Self::new(
            "Images",
            &["*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp"],
        )
    }

    /// Check if a filename matches this filter.
    pub fn matches(&self, filename: &str) -> bool {
        let filename_lower = filename.to_lowercase();

        for pattern in &self.patterns {
            if pattern == "*" {
                return true;
            }

            // Handle simple extension patterns like "*.rs"
            if let Some(ext_pattern) = pattern.strip_prefix("*.")
                && filename_lower.ends_with(&format!(".{}", ext_pattern.to_lowercase()))
            {
                return true;
            }
        }

        false
    }

    /// Get the display text for this filter (name + patterns).
    pub fn display_text(&self) -> String {
        format!("{} ({})", self.name, self.patterns.join(", "))
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self::all_files()
    }
}

// ============================================================================
// FileEntry
// ============================================================================

/// Represents a file or directory entry in the dialog.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The name of the file or directory.
    pub name: String,

    /// The full path to the file or directory.
    pub path: PathBuf,

    /// Whether this is a directory.
    pub is_dir: bool,

    /// File size in bytes (0 for directories).
    pub size: u64,

    /// Last modified time.
    pub modified: Option<SystemTime>,

    /// Whether this entry is hidden.
    pub is_hidden: bool,

    /// Whether this is a symlink.
    pub is_symlink: bool,
}

impl FileEntry {
    /// Create a new file entry.
    pub fn new(path: PathBuf, is_dir: bool) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_hidden = name.starts_with('.');

        Self {
            name,
            path,
            is_dir,
            size: 0,
            modified: None,
            is_hidden,
            is_symlink: false,
        }
    }

    /// Create a file entry from a path, reading metadata.
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_hidden = name.starts_with('.');
        let is_symlink = std::fs::symlink_metadata(&path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);

        Ok(Self {
            name,
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
            is_hidden,
            is_symlink,
        })
    }

    /// Get a human-readable size string.
    pub fn size_string(&self) -> String {
        if self.is_dir {
            return String::new();
        }

        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.1} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }

    /// Get the file extension.
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }
}

// ============================================================================
// BookmarkEntry
// ============================================================================

/// A bookmark entry for quick navigation.
#[derive(Debug, Clone)]
pub struct BookmarkEntry {
    /// Display name for the bookmark.
    pub name: String,

    /// Path to the bookmarked location.
    pub path: PathBuf,

    /// Optional icon identifier.
    pub icon_type: BookmarkIcon,
}

/// Icon types for bookmarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BookmarkIcon {
    /// Generic folder icon.
    #[default]
    Folder,
    /// Home directory icon.
    Home,
    /// Desktop directory icon.
    Desktop,
    /// Documents directory icon.
    Documents,
    /// Downloads directory icon.
    Downloads,
    /// Pictures directory icon.
    Pictures,
    /// Music directory icon.
    Music,
    /// Videos directory icon.
    Videos,
    /// Disk/drive icon.
    Drive,
    /// Network location icon.
    Network,
    /// Trash/recycle bin icon.
    Trash,
    /// Custom user-defined icon.
    Custom,
}

impl BookmarkEntry {
    /// Create a new bookmark entry.
    pub fn new(name: impl Into<String>, path: PathBuf, icon_type: BookmarkIcon) -> Self {
        Self {
            name: name.into(),
            path,
            icon_type,
        }
    }

    /// Create a bookmark for the user's home directory.
    pub fn home() -> Option<Self> {
        dirs_path::home_dir().map(|p| Self::new("Home", p, BookmarkIcon::Home))
    }

    /// Create a bookmark for the user's desktop.
    pub fn desktop() -> Option<Self> {
        dirs_path::desktop_dir().map(|p| Self::new("Desktop", p, BookmarkIcon::Desktop))
    }

    /// Create a bookmark for the user's documents folder.
    pub fn documents() -> Option<Self> {
        dirs_path::document_dir().map(|p| Self::new("Documents", p, BookmarkIcon::Documents))
    }

    /// Create a bookmark for the user's downloads folder.
    pub fn downloads() -> Option<Self> {
        dirs_path::download_dir().map(|p| Self::new("Downloads", p, BookmarkIcon::Downloads))
    }

    /// Create a bookmark for the user's pictures folder.
    pub fn pictures() -> Option<Self> {
        dirs_path::picture_dir().map(|p| Self::new("Pictures", p, BookmarkIcon::Pictures))
    }
}

// Simple cross-platform directory path helpers
mod dirs_path {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(PathBuf::from)
        }
    }

    pub fn desktop_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Desktop"))
    }

    pub fn document_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Documents"))
    }

    pub fn download_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Downloads"))
    }

    pub fn picture_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Pictures"))
    }
}

// ============================================================================
// PathSegment (for breadcrumb)
// ============================================================================

/// A segment in the path breadcrumb.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PathSegment {
    /// Display name for this segment.
    name: String,
    /// Full path up to and including this segment.
    path: PathBuf,
    /// Visual rectangle for click detection.
    rect: Rect,
    /// Whether this segment is hovered.
    hovered: bool,
}

// ============================================================================
// DirectoryNode (for tree sidebar)
// ============================================================================

/// A node in the directory tree sidebar.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DirectoryNode {
    /// Path to this directory.
    path: PathBuf,
    /// Display name.
    name: String,
    /// Whether this node is expanded.
    expanded: bool,
    /// Whether children have been loaded.
    children_loaded: bool,
    /// Depth in the tree.
    depth: usize,
    /// Visual rectangle.
    rect: Rect,
    /// Whether this node is hovered.
    hovered: bool,
    /// Whether this node is selected.
    selected: bool,
}

// ============================================================================
// ViewMode
// ============================================================================

/// Display mode for the file list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileViewMode {
    /// List view with details (name, size, date).
    #[default]
    List,
    /// Icon grid view.
    Icons,
    /// Compact list view.
    Compact,
}

// ============================================================================
// FileDialog
// ============================================================================

/// A modal dialog for file and directory selection.
///
/// FileDialog provides a comprehensive file browser with:
/// - Open/Save/Directory selection modes
/// - Multiple file selection support
/// - File filtering by extension
/// - Directory tree navigation sidebar
/// - Path breadcrumb navigation
/// - Recent locations and bookmarks
/// - Keyboard navigation
///
/// # Static Helpers
///
/// For simple use cases, use the static helper methods:
/// - [`FileDialog::get_open_file_name()`]: Select a single file
/// - [`FileDialog::get_open_file_names()`]: Select multiple files
/// - [`FileDialog::get_save_file_name()`]: Select a save location
/// - [`FileDialog::get_existing_directory()`]: Select a directory
///
/// # Signals
///
/// - `file_selected`: Emitted when a single file is selected
/// - `files_selected`: Emitted when multiple files are selected
/// - `directory_selected`: Emitted when a directory is selected
/// - `current_changed`: Emitted when the current directory changes
/// - `filter_changed`: Emitted when the current filter changes
#[allow(dead_code)]
pub struct FileDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// The dialog mode.
    mode: FileDialogMode,

    /// Current directory being viewed.
    current_dir: PathBuf,

    /// Available file filters.
    filters: Vec<FileFilter>,

    /// Currently selected filter index.
    current_filter_index: usize,

    /// Files/directories in the current view.
    entries: Vec<FileEntry>,

    /// Currently selected entries (indices into entries).
    selected_indices: HashSet<usize>,

    /// Whether to show hidden files.
    show_hidden: bool,

    /// The text in the filename input field.
    filename_text: String,

    // Navigation UI state
    /// Path segments for breadcrumb.
    path_segments: Vec<PathSegment>,

    /// Directory tree nodes for sidebar.
    tree_nodes: Vec<DirectoryNode>,

    /// Sidebar width.
    sidebar_width: f32,

    /// Whether the sidebar is visible.
    sidebar_visible: bool,

    /// Recent locations (most recent first).
    recent_locations: Vec<PathBuf>,

    /// Maximum recent locations to remember.
    max_recent: usize,

    /// User bookmarks.
    bookmarks: Vec<BookmarkEntry>,

    /// System bookmarks (Home, Desktop, etc.).
    system_bookmarks: Vec<BookmarkEntry>,

    // View state
    /// Current view mode.
    view_mode: FileViewMode,

    /// Scroll position in the file list.
    scroll_y: f32,

    /// Scroll position in the sidebar tree.
    tree_scroll_y: f32,

    /// Hovered entry index in file list.
    hovered_entry: Option<usize>,

    /// Last click time for double-click detection.
    last_click_time: Option<Instant>,

    /// Last clicked entry for double-click detection.
    last_click_entry: Option<usize>,

    /// Whether a directory is being loaded.
    loading: bool,

    // Visual configuration
    /// Row height in list view.
    row_height: f32,

    /// Icon size in icon view.
    icon_size: f32,

    /// Content padding.
    content_padding: f32,

    /// Path bar height.
    path_bar_height: f32,

    /// Filter bar height.
    filter_bar_height: f32,

    /// Tree item height.
    tree_item_height: f32,

    /// Tree indentation per level.
    tree_indent: f32,

    // Colors
    background_color: Color,
    sidebar_color: Color,
    path_bar_color: Color,
    selection_color: Color,
    hover_color: Color,
    text_color: Color,
    secondary_text_color: Color,
    border_color: Color,
    folder_icon_color: Color,
    file_icon_color: Color,

    // Native dialog settings
    /// Whether to prefer native dialogs when available.
    use_native_dialog: bool,

    // Signals
    /// Emitted when a single file is selected and dialog is accepted.
    pub file_selected: Signal<PathBuf>,

    /// Emitted when multiple files are selected and dialog is accepted.
    pub files_selected: Signal<Vec<PathBuf>>,

    /// Emitted when a directory is selected and dialog is accepted.
    pub directory_selected: Signal<PathBuf>,

    /// Emitted when the current directory changes.
    pub current_changed: Signal<PathBuf>,

    /// Emitted when the filter selection changes.
    pub filter_changed: Signal<usize>,
}

impl FileDialog {
    /// Create a new FileDialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Open")
            .with_size(800.0, 500.0)
            .with_standard_buttons(StandardButton::OPEN | StandardButton::CANCEL);

        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| dirs_path::home_dir().unwrap_or_else(|| PathBuf::from("/")));

        let mut file_dialog = Self {
            dialog,
            mode: FileDialogMode::OpenFile,
            current_dir: current_dir.clone(),
            filters: vec![FileFilter::all_files()],
            current_filter_index: 0,
            entries: Vec::new(),
            selected_indices: HashSet::new(),
            show_hidden: false,
            filename_text: String::new(),
            path_segments: Vec::new(),
            tree_nodes: Vec::new(),
            sidebar_width: 180.0,
            sidebar_visible: true,
            recent_locations: Vec::new(),
            max_recent: 10,
            bookmarks: Vec::new(),
            system_bookmarks: Vec::new(),
            view_mode: FileViewMode::List,
            scroll_y: 0.0,
            tree_scroll_y: 0.0,
            hovered_entry: None,
            last_click_time: None,
            last_click_entry: None,
            loading: false,
            row_height: 24.0,
            icon_size: 48.0,
            content_padding: 8.0,
            path_bar_height: 32.0,
            filter_bar_height: 36.0,
            tree_item_height: 24.0,
            tree_indent: 16.0,
            background_color: Color::WHITE,
            sidebar_color: Color::from_rgb8(245, 245, 245),
            path_bar_color: Color::from_rgb8(250, 250, 250),
            selection_color: Color::from_rgba8(0, 120, 215, 80),
            hover_color: Color::from_rgba8(0, 0, 0, 20),
            text_color: Color::from_rgb8(32, 32, 32),
            secondary_text_color: Color::from_rgb8(128, 128, 128),
            border_color: Color::from_rgb8(200, 200, 200),
            folder_icon_color: Color::from_rgb8(255, 200, 87),
            file_icon_color: Color::from_rgb8(180, 180, 180),
            use_native_dialog: false,
            file_selected: Signal::new(),
            files_selected: Signal::new(),
            directory_selected: Signal::new(),
            current_changed: Signal::new(),
            filter_changed: Signal::new(),
        };

        // Initialize system bookmarks
        file_dialog.init_system_bookmarks();

        // Initialize tree with root nodes
        file_dialog.init_tree_roots();

        // Load current directory
        file_dialog.navigate_to(&current_dir);

        file_dialog
    }

    // =========================================================================
    // Factory Methods
    // =========================================================================

    /// Create a FileDialog configured for opening a single file.
    pub fn for_open() -> Self {
        Self::new()
            .with_mode(FileDialogMode::OpenFile)
            .with_title("Open File")
    }

    /// Create a FileDialog configured for opening multiple files.
    pub fn for_open_multiple() -> Self {
        Self::new()
            .with_mode(FileDialogMode::OpenFiles)
            .with_title("Open Files")
    }

    /// Create a FileDialog configured for saving a file.
    pub fn for_save() -> Self {
        let mut dialog = Self::new()
            .with_mode(FileDialogMode::SaveFile)
            .with_title("Save File");
        dialog
            .dialog
            .set_standard_buttons(StandardButton::SAVE | StandardButton::CANCEL);
        dialog
    }

    /// Create a FileDialog configured for selecting a directory.
    pub fn for_directory() -> Self {
        Self::new()
            .with_mode(FileDialogMode::Directory)
            .with_title("Select Folder")
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog mode using builder pattern.
    pub fn with_mode(mut self, mode: FileDialogMode) -> Self {
        self.mode = mode;
        self.update_buttons_for_mode();
        self
    }

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the starting directory using builder pattern.
    pub fn with_directory(mut self, dir: impl AsRef<Path>) -> Self {
        self.navigate_to(dir.as_ref());
        self
    }

    /// Add a file filter using builder pattern.
    pub fn with_filter(mut self, filter: FileFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set all filters using builder pattern.
    pub fn with_filters(mut self, filters: Vec<FileFilter>) -> Self {
        self.filters = filters;
        if self.current_filter_index >= self.filters.len() {
            self.current_filter_index = 0;
        }
        self.refresh_entries();
        self
    }

    /// Set whether to show hidden files using builder pattern.
    pub fn with_show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self.refresh_entries();
        self
    }

    /// Set the default filename using builder pattern.
    pub fn with_default_filename(mut self, name: impl Into<String>) -> Self {
        self.filename_text = name.into();
        self
    }

    /// Set whether to prefer native dialogs using builder pattern.
    pub fn with_native_dialog(mut self, use_native: bool) -> Self {
        self.use_native_dialog = use_native;
        self
    }

    /// Set the sidebar visibility using builder pattern.
    pub fn with_sidebar(mut self, visible: bool) -> Self {
        self.sidebar_visible = visible;
        self
    }

    /// Set the view mode using builder pattern.
    pub fn with_view_mode(mut self, mode: FileViewMode) -> Self {
        self.view_mode = mode;
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the dialog mode.
    pub fn mode(&self) -> FileDialogMode {
        self.mode
    }

    /// Set the dialog mode.
    pub fn set_mode(&mut self, mode: FileDialogMode) {
        if self.mode != mode {
            self.mode = mode;
            self.update_buttons_for_mode();
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the current directory.
    pub fn current_directory(&self) -> &Path {
        &self.current_dir
    }

    /// Set the current directory.
    pub fn set_current_directory(&mut self, dir: impl AsRef<Path>) {
        self.navigate_to(dir.as_ref());
    }

    /// Get the filters.
    pub fn filters(&self) -> &[FileFilter] {
        &self.filters
    }

    /// Set the filters.
    pub fn set_filters(&mut self, filters: Vec<FileFilter>) {
        self.filters = filters;
        if self.current_filter_index >= self.filters.len() {
            self.current_filter_index = 0;
        }
        self.refresh_entries();
    }

    /// Add a filter.
    pub fn add_filter(&mut self, filter: FileFilter) {
        self.filters.push(filter);
    }

    /// Get the current filter index.
    pub fn current_filter_index(&self) -> usize {
        self.current_filter_index
    }

    /// Set the current filter index.
    pub fn set_current_filter_index(&mut self, index: usize) {
        if index < self.filters.len() && self.current_filter_index != index {
            self.current_filter_index = index;
            self.refresh_entries();
            self.filter_changed.emit(index);
        }
    }

    /// Get the filename text.
    pub fn filename_text(&self) -> &str {
        &self.filename_text
    }

    /// Set the filename text.
    pub fn set_filename_text(&mut self, text: impl Into<String>) {
        self.filename_text = text.into();
        self.dialog.widget_base_mut().update();
    }

    /// Get whether hidden files are shown.
    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    /// Set whether hidden files are shown.
    pub fn set_show_hidden(&mut self, show: bool) {
        if self.show_hidden != show {
            self.show_hidden = show;
            self.refresh_entries();
        }
    }

    /// Get the selected paths.
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.path.clone())
            .collect()
    }

    /// Get the selected path (for single selection modes).
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_indices
            .iter()
            .next()
            .and_then(|&i| self.entries.get(i))
            .map(|e| e.path.clone())
    }

    /// Get the view mode.
    pub fn view_mode(&self) -> FileViewMode {
        self.view_mode
    }

    /// Set the view mode.
    pub fn set_view_mode(&mut self, mode: FileViewMode) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the recent locations.
    pub fn recent_locations(&self) -> &[PathBuf] {
        &self.recent_locations
    }

    /// Add a recent location.
    pub fn add_recent_location(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_locations.retain(|p| p != &path);
        // Add at front
        self.recent_locations.insert(0, path);
        // Trim to max
        self.recent_locations.truncate(self.max_recent);
    }

    /// Get the user bookmarks.
    pub fn bookmarks(&self) -> &[BookmarkEntry] {
        &self.bookmarks
    }

    /// Add a bookmark.
    pub fn add_bookmark(&mut self, bookmark: BookmarkEntry) {
        self.bookmarks.push(bookmark);
    }

    /// Remove a bookmark by path.
    pub fn remove_bookmark(&mut self, path: &Path) {
        self.bookmarks.retain(|b| b.path != path);
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Open a dialog to select a single file and return the path.
    ///
    /// Returns `None` if the user cancels the dialog.
    ///
    /// Note: This creates the dialog but the actual selection happens via signals.
    /// For synchronous behavior, use native dialogs on platforms that support them.
    pub fn get_open_file_name(
        title: impl Into<String>,
        directory: impl AsRef<Path>,
        filters: &[FileFilter],
    ) -> FileDialog {
        let mut dialog = Self::for_open().with_title(title).with_directory(directory);

        for filter in filters {
            dialog = dialog.with_filter(filter.clone());
        }

        dialog
    }

    /// Open a dialog to select multiple files.
    pub fn get_open_file_names(
        title: impl Into<String>,
        directory: impl AsRef<Path>,
        filters: &[FileFilter],
    ) -> FileDialog {
        let mut dialog = Self::for_open_multiple()
            .with_title(title)
            .with_directory(directory);

        for filter in filters {
            dialog = dialog.with_filter(filter.clone());
        }

        dialog
    }

    /// Open a dialog to select a save file location.
    pub fn get_save_file_name(
        title: impl Into<String>,
        directory: impl AsRef<Path>,
        filters: &[FileFilter],
    ) -> FileDialog {
        let mut dialog = Self::for_save().with_title(title).with_directory(directory);

        for filter in filters {
            dialog = dialog.with_filter(filter.clone());
        }

        dialog
    }

    /// Open a dialog to select an existing directory.
    pub fn get_existing_directory(
        title: impl Into<String>,
        directory: impl AsRef<Path>,
    ) -> FileDialog {
        Self::for_directory()
            .with_title(title)
            .with_directory(directory)
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the dialog.
    ///
    /// If `use_native_dialog` is true and native dialogs are available,
    /// this will show a native file dialog instead of the custom one.
    pub fn open(&mut self) {
        // Try native dialog if preferred and available
        if self.use_native_dialog && native::is_available() {
            let title = self.dialog.title();
            let dir = &self.current_dir;
            let filters = &self.filters;

            match self.mode {
                FileDialogMode::OpenFile => {
                    if let Some(path) = native::open_file_dialog(title, dir, filters) {
                        self.file_selected.emit(path.clone());
                        self.add_recent_location(path.parent().unwrap_or(dir).to_path_buf());
                        return;
                    }
                }
                FileDialogMode::OpenFiles => {
                    if let Some(paths) = native::open_files_dialog(title, dir, filters) {
                        if let Some(first_path) = paths.first() {
                            self.add_recent_location(
                                first_path.parent().unwrap_or(dir).to_path_buf(),
                            );
                        }
                        self.files_selected.emit(paths);
                        return;
                    }
                }
                FileDialogMode::SaveFile => {
                    if let Some(path) = native::save_file_dialog(title, dir, filters) {
                        self.file_selected.emit(path.clone());
                        self.add_recent_location(path.parent().unwrap_or(dir).to_path_buf());
                        return;
                    }
                }
                FileDialogMode::Directory => {
                    if let Some(path) = native::directory_dialog(title, dir) {
                        self.directory_selected.emit(path.clone());
                        self.add_recent_location(path);
                        return;
                    }
                }
            }
            // Native dialog was cancelled or not available, don't fall through
            // to custom dialog - just return without showing anything
            return;
        }

        // Use custom dialog
        self.refresh_entries();
        self.dialog.open();
    }

    /// Accept the dialog with current selection.
    pub fn accept(&mut self) {
        // Emit appropriate signal based on mode
        match self.mode {
            FileDialogMode::OpenFile | FileDialogMode::SaveFile => {
                if let Some(path) = self.get_final_path() {
                    self.file_selected.emit(path);
                }
            }
            FileDialogMode::OpenFiles => {
                let paths = self.get_final_paths();
                if !paths.is_empty() {
                    self.files_selected.emit(paths);
                }
            }
            FileDialogMode::Directory => {
                if let Some(path) = self.get_final_path() {
                    self.directory_selected.emit(path);
                }
            }
        }

        // Add to recent locations
        self.add_recent_location(self.current_dir.clone());

        self.dialog.accept();
    }

    /// Reject/cancel the dialog.
    pub fn reject(&mut self) {
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    /// Check if the dialog is open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Navigate to a directory.
    pub fn navigate_to(&mut self, path: &Path) {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.current_dir.join(path)
        };

        // Canonicalize the path
        let canonical = path.canonicalize().unwrap_or(path);

        if canonical.is_dir() {
            self.current_dir = canonical.clone();
            self.selected_indices.clear();
            self.scroll_y = 0.0;
            self.update_path_segments();
            self.refresh_entries();
            self.current_changed.emit(canonical);
            self.dialog.widget_base_mut().update();
        }
    }

    /// Navigate up to the parent directory.
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(&parent);
        }
    }

    /// Navigate back in history (if implemented).
    pub fn navigate_back(&mut self) {
        // For now, just go to parent
        self.navigate_up();
    }

    /// Refresh the current directory listing.
    pub fn refresh(&mut self) {
        self.refresh_entries();
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    fn update_buttons_for_mode(&mut self) {
        let buttons = match self.mode {
            FileDialogMode::OpenFile | FileDialogMode::OpenFiles => {
                StandardButton::OPEN | StandardButton::CANCEL
            }
            FileDialogMode::SaveFile => StandardButton::SAVE | StandardButton::CANCEL,
            FileDialogMode::Directory => StandardButton::OPEN | StandardButton::CANCEL,
        };
        self.dialog.set_standard_buttons(buttons);
    }

    fn init_system_bookmarks(&mut self) {
        self.system_bookmarks.clear();

        // Add system bookmarks
        if let Some(home) = BookmarkEntry::home() {
            self.system_bookmarks.push(home);
        }
        if let Some(desktop) = BookmarkEntry::desktop()
            && desktop.path.exists()
        {
            self.system_bookmarks.push(desktop);
        }
        if let Some(docs) = BookmarkEntry::documents()
            && docs.path.exists()
        {
            self.system_bookmarks.push(docs);
        }
        if let Some(downloads) = BookmarkEntry::downloads()
            && downloads.path.exists()
        {
            self.system_bookmarks.push(downloads);
        }
        if let Some(pictures) = BookmarkEntry::pictures()
            && pictures.path.exists()
        {
            self.system_bookmarks.push(pictures);
        }

        // Add root filesystem
        #[cfg(not(target_os = "windows"))]
        {
            self.system_bookmarks.push(BookmarkEntry::new(
                "File System",
                PathBuf::from("/"),
                BookmarkIcon::Drive,
            ));
        }

        // On Windows, add drives
        #[cfg(target_os = "windows")]
        {
            for letter in b'A'..=b'Z' {
                let drive_path = PathBuf::from(format!("{}:\\", letter as char));
                if drive_path.exists() {
                    self.system_bookmarks.push(BookmarkEntry::new(
                        format!("Drive ({}:)", letter as char),
                        drive_path,
                        BookmarkIcon::Drive,
                    ));
                }
            }
        }
    }

    fn init_tree_roots(&mut self) {
        self.tree_nodes.clear();

        // Add system bookmarks as root nodes
        for bookmark in &self.system_bookmarks {
            self.tree_nodes.push(DirectoryNode {
                path: bookmark.path.clone(),
                name: bookmark.name.clone(),
                expanded: false,
                children_loaded: false,
                depth: 0,
                rect: Rect::ZERO,
                hovered: false,
                selected: false,
            });
        }
    }

    fn update_path_segments(&mut self) {
        self.path_segments.clear();

        let mut accumulated_path = PathBuf::new();

        // Handle root
        #[cfg(not(target_os = "windows"))]
        {
            accumulated_path.push("/");
            self.path_segments.push(PathSegment {
                name: "/".to_string(),
                path: PathBuf::from("/"),
                rect: Rect::ZERO,
                hovered: false,
            });
        }

        // Add each component
        for component in self.current_dir.components() {
            let name = component.as_os_str().to_string_lossy().to_string();
            if name == "/" || name.is_empty() {
                continue;
            }

            accumulated_path.push(&name);
            self.path_segments.push(PathSegment {
                name,
                path: accumulated_path.clone(),
                rect: Rect::ZERO,
                hovered: false,
            });
        }
    }

    fn refresh_entries(&mut self) {
        self.entries.clear();
        self.loading = true;

        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<FileEntry> = Vec::new();
            let mut files: Vec<FileEntry> = Vec::new();

            for entry in read_dir.flatten() {
                if let Ok(file_entry) = FileEntry::from_path(entry.path()) {
                    // Skip hidden files if not showing them
                    if file_entry.is_hidden && !self.show_hidden {
                        continue;
                    }

                    if file_entry.is_dir {
                        dirs.push(file_entry);
                    } else {
                        // Apply filter for files
                        let current_filter = self.filters.get(self.current_filter_index);
                        let matches = current_filter
                            .map(|f| f.matches(&file_entry.name))
                            .unwrap_or(true);

                        // In directory mode, only show directories
                        if self.mode.is_directory_mode() {
                            continue;
                        }

                        if matches {
                            files.push(file_entry);
                        }
                    }
                }
            }

            // Sort directories and files alphabetically
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // Directories first, then files
            self.entries.extend(dirs);
            self.entries.extend(files);
        }

        self.loading = false;
        self.dialog.widget_base_mut().update();
    }

    fn get_final_path(&self) -> Option<PathBuf> {
        // If in directory mode, return current directory
        if self.mode.is_directory_mode() {
            return Some(self.current_dir.clone());
        }

        // If there's text in the filename field, use that
        if !self.filename_text.is_empty() {
            let path = if Path::new(&self.filename_text).is_absolute() {
                PathBuf::from(&self.filename_text)
            } else {
                self.current_dir.join(&self.filename_text)
            };
            return Some(path);
        }

        // Otherwise, use the selected item
        self.selected_path()
    }

    fn get_final_paths(&self) -> Vec<PathBuf> {
        if !self.filename_text.is_empty() {
            // Parse multiple files from filename field (space or semicolon separated)
            return self
                .filename_text
                .split([';', '\n'])
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let s = s.trim();
                    if Path::new(s).is_absolute() {
                        PathBuf::from(s)
                    } else {
                        self.current_dir.join(s)
                    }
                })
                .collect();
        }

        self.selected_paths()
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the title bar height.
    fn title_bar_height(&self) -> f32 {
        28.0
    }

    /// Get the button box height.
    fn button_box_height(&self) -> f32 {
        48.0
    }

    /// Get the sidebar rectangle.
    fn sidebar_rect(&self) -> Rect {
        if !self.sidebar_visible {
            return Rect::ZERO;
        }

        let dialog_rect = self.dialog.widget_base().rect();
        Rect::new(
            0.0,
            self.title_bar_height() + self.path_bar_height,
            self.sidebar_width,
            dialog_rect.height()
                - self.title_bar_height()
                - self.path_bar_height
                - self.filter_bar_height
                - self.button_box_height(),
        )
    }

    /// Get the path bar rectangle.
    fn path_bar_rect(&self) -> Rect {
        let dialog_rect = self.dialog.widget_base().rect();
        Rect::new(
            0.0,
            self.title_bar_height(),
            dialog_rect.width(),
            self.path_bar_height,
        )
    }

    /// Get the file list rectangle.
    fn file_list_rect(&self) -> Rect {
        let dialog_rect = self.dialog.widget_base().rect();
        let sidebar_rect = self.sidebar_rect();
        let left = if self.sidebar_visible {
            sidebar_rect.width() + 1.0 // 1px border
        } else {
            0.0
        };

        Rect::new(
            left,
            self.title_bar_height() + self.path_bar_height,
            dialog_rect.width() - left,
            dialog_rect.height()
                - self.title_bar_height()
                - self.path_bar_height
                - self.filter_bar_height
                - self.button_box_height(),
        )
    }

    /// Get the filter bar rectangle.
    fn filter_bar_rect(&self) -> Rect {
        let dialog_rect = self.dialog.widget_base().rect();
        Rect::new(
            0.0,
            dialog_rect.height() - self.filter_bar_height - self.button_box_height(),
            dialog_rect.width(),
            self.filter_bar_height,
        )
    }

    /// Get the entry rectangle for a given index.
    fn entry_rect(&self, index: usize) -> Rect {
        let list_rect = self.file_list_rect();

        match self.view_mode {
            FileViewMode::List | FileViewMode::Compact => {
                let y = list_rect.origin.y + (index as f32 * self.row_height) - self.scroll_y;
                Rect::new(list_rect.origin.x, y, list_rect.width(), self.row_height)
            }
            FileViewMode::Icons => {
                let cols = (list_rect.width() / (self.icon_size + self.content_padding * 2.0))
                    .max(1.0) as usize;
                let row = index / cols;
                let col = index % cols;
                let cell_width = list_rect.width() / cols as f32;
                let cell_height = self.icon_size + 24.0; // Icon + text

                Rect::new(
                    list_rect.origin.x + col as f32 * cell_width,
                    list_rect.origin.y + row as f32 * cell_height - self.scroll_y,
                    cell_width,
                    cell_height,
                )
            }
        }
    }

    /// Find which entry is at the given position.
    fn entry_at_pos(&self, pos: Point) -> Option<usize> {
        let list_rect = self.file_list_rect();
        if !list_rect.contains(pos) {
            return None;
        }

        for (i, _entry) in self.entries.iter().enumerate() {
            let rect = self.entry_rect(i);
            if rect.contains(pos) && rect.origin.y >= list_rect.origin.y {
                return Some(i);
            }
        }

        None
    }

    // =========================================================================
    // Selection
    // =========================================================================

    fn select_entry(&mut self, index: usize, extend: bool) {
        if index >= self.entries.len() {
            return;
        }

        if !extend || !self.mode.is_multi_select() {
            self.selected_indices.clear();
        }

        self.selected_indices.insert(index);

        // Update filename text
        if let Some(entry) = self.entries.get(index)
            && (!entry.is_dir || self.mode.is_directory_mode())
        {
            self.filename_text = entry.name.clone();
        }

        self.dialog.widget_base_mut().update();
    }

    fn toggle_entry_selection(&mut self, index: usize) {
        if !self.mode.is_multi_select() {
            self.select_entry(index, false);
            return;
        }

        if self.selected_indices.contains(&index) {
            self.selected_indices.remove(&index);
        } else {
            self.selected_indices.insert(index);
        }

        // Update filename text with all selected items
        let names: Vec<_> = self
            .selected_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.name.clone())
            .collect();
        self.filename_text = names.join("; ");

        self.dialog.widget_base_mut().update();
    }

    fn activate_entry(&mut self, index: usize) {
        if let Some(entry) = self.entries.get(index) {
            if entry.is_dir {
                // Navigate into directory
                self.navigate_to(&entry.path.clone());
            } else {
                // Select file and accept
                self.select_entry(index, false);
                self.accept();
            }
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check path bar clicks
        for segment in self.path_segments.iter() {
            if segment.rect.contains(pos) {
                let path = segment.path.clone();
                self.navigate_to(&path);
                return true;
            }
        }

        // Check sidebar clicks
        if self.sidebar_visible {
            let sidebar_rect = self.sidebar_rect();
            if sidebar_rect.contains(pos) {
                // Check bookmark clicks
                for _bookmark in &self.system_bookmarks {
                    // Calculate bookmark rect (simplified)
                    // In a full implementation, we'd track these rects
                }

                // Check tree node clicks
                for node in &self.tree_nodes {
                    if node.rect.contains(pos) {
                        let path = node.path.clone();
                        self.navigate_to(&path);
                        return true;
                    }
                }
            }
        }

        // Check file list clicks
        if let Some(index) = self.entry_at_pos(pos) {
            let now = Instant::now();
            let is_double_click = self
                .last_click_time
                .map(|t| now.duration_since(t).as_millis() < 400)
                .unwrap_or(false)
                && self.last_click_entry == Some(index);

            if is_double_click {
                self.activate_entry(index);
                self.last_click_time = None;
                self.last_click_entry = None;
            } else {
                let extend = event.modifiers.control || event.modifiers.shift;
                if event.modifiers.control {
                    self.toggle_entry_selection(index);
                } else {
                    self.select_entry(index, extend);
                }
                self.last_click_time = Some(now);
                self.last_click_entry = Some(index);
            }
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, _event: &MouseReleaseEvent) -> bool {
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;
        let mut needs_update = false;

        // Update path segment hover states
        for segment in &mut self.path_segments {
            let new_hover = segment.rect.contains(pos);
            if segment.hovered != new_hover {
                segment.hovered = new_hover;
                needs_update = true;
            }
        }

        // Update entry hover state
        let new_hover = self.entry_at_pos(pos);
        if self.hovered_entry != new_hover {
            self.hovered_entry = new_hover;
            needs_update = true;
        }

        if needs_update {
            self.dialog.widget_base_mut().update();
        }

        needs_update
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let list_rect = self.file_list_rect();
        if list_rect.contains(event.local_pos) {
            let delta = event.delta_y * 3.0;
            let max_scroll =
                (self.entries.len() as f32 * self.row_height - list_rect.height()).max(0.0);
            self.scroll_y = (self.scroll_y - delta).clamp(0.0, max_scroll);
            self.dialog.widget_base_mut().update();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Enter => {
                // Activate selected or accept
                if let Some(&index) = self.selected_indices.iter().next() {
                    if let Some(entry) = self.entries.get(index) {
                        if entry.is_dir && !self.mode.is_directory_mode() {
                            self.navigate_to(&entry.path.clone());
                        } else {
                            self.accept();
                        }
                    }
                } else {
                    self.accept();
                }
                return true;
            }
            Key::Escape => {
                self.reject();
                return true;
            }
            Key::Backspace if event.modifiers.alt => {
                self.navigate_up();
                return true;
            }
            Key::ArrowUp => {
                if let Some(&current) = self.selected_indices.iter().next() {
                    if current > 0 {
                        self.select_entry(current - 1, event.modifiers.shift);
                    }
                } else if !self.entries.is_empty() {
                    self.select_entry(0, false);
                }
                return true;
            }
            Key::ArrowDown => {
                if let Some(&current) = self.selected_indices.iter().next() {
                    if current + 1 < self.entries.len() {
                        self.select_entry(current + 1, event.modifiers.shift);
                    }
                } else if !self.entries.is_empty() {
                    self.select_entry(0, false);
                }
                return true;
            }
            Key::Home => {
                if !self.entries.is_empty() {
                    self.select_entry(0, event.modifiers.shift);
                }
                return true;
            }
            Key::End => {
                if !self.entries.is_empty() {
                    self.select_entry(self.entries.len() - 1, event.modifiers.shift);
                }
                return true;
            }
            _ => {}
        }

        // Handle typing to filter/search
        if let Some(ch) = event.key.to_ascii_char()
            && ch.is_alphanumeric()
        {
            // Find first entry starting with this character
            let ch_lower = ch.to_ascii_lowercase();
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.name.to_lowercase().starts_with(ch_lower) {
                    self.select_entry(i, false);
                    break;
                }
            }
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_path_bar(&mut self, ctx: &mut PaintContext<'_>) {
        let rect = self.path_bar_rect();

        // Background
        ctx.renderer().fill_rect(rect, self.path_bar_color);

        // Border at bottom
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(rect.origin.x, rect.origin.y + rect.height()),
            Point::new(rect.origin.x + rect.width(), rect.origin.y + rect.height()),
            &stroke,
        );

        // Draw path segments
        let mut x = rect.origin.x + self.content_padding;
        let y = rect.origin.y + (rect.height() - 16.0) / 2.0;

        for segment in &mut self.path_segments {
            let text_width = segment.name.len() as f32 * 8.0; // Approximate width

            segment.rect = Rect::new(x, rect.origin.y, text_width + 16.0, rect.height());

            // Background on hover
            if segment.hovered {
                ctx.renderer().fill_rect(segment.rect, self.hover_color);
            }

            // Draw separator
            if x > rect.origin.x + self.content_padding {
                let arrow_x = x;
                ctx.renderer().draw_line(
                    Point::new(arrow_x, y + 4.0),
                    Point::new(arrow_x + 6.0, y + 8.0),
                    &Stroke::new(self.secondary_text_color, 1.0),
                );
                ctx.renderer().draw_line(
                    Point::new(arrow_x + 6.0, y + 8.0),
                    Point::new(arrow_x, y + 12.0),
                    &Stroke::new(self.secondary_text_color, 1.0),
                );
                x += 12.0;
            }

            // Text would be drawn here with text renderer
            // For now, we just advance x
            x += text_width + self.content_padding;
        }
    }

    fn paint_sidebar(&self, ctx: &mut PaintContext<'_>) {
        if !self.sidebar_visible {
            return;
        }

        let rect = self.sidebar_rect();

        // Background
        ctx.renderer().fill_rect(rect, self.sidebar_color);

        // Right border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(rect.origin.x + rect.width(), rect.origin.y),
            Point::new(rect.origin.x + rect.width(), rect.origin.y + rect.height()),
            &stroke,
        );

        // Draw bookmarks section
        let mut y = rect.origin.y + self.content_padding;

        // Section header: "Places"
        // (Text rendering would go here)
        y += 20.0;

        // Draw system bookmarks
        for bookmark in &self.system_bookmarks {
            let item_rect = Rect::new(rect.origin.x, y, rect.width(), self.tree_item_height);

            // Highlight if this is the current directory
            if bookmark.path == self.current_dir {
                ctx.renderer().fill_rect(item_rect, self.selection_color);
            }

            // Draw folder icon
            self.draw_folder_icon(
                ctx,
                Point::new(rect.origin.x + self.content_padding, y + 4.0),
                16.0,
            );

            // Name would be drawn with text renderer
            y += self.tree_item_height;
        }

        // Separator
        y += self.content_padding;
        ctx.renderer().draw_line(
            Point::new(rect.origin.x + self.content_padding, y),
            Point::new(rect.origin.x + rect.width() - self.content_padding, y),
            &Stroke::new(self.border_color, 1.0),
        );

        // User bookmarks section
        // (Similar to system bookmarks)
    }

    fn paint_file_list(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.file_list_rect();

        // Background
        ctx.renderer().fill_rect(rect, self.background_color);

        // Clip to list area
        // (In a full implementation, we'd use a clip rect)

        match self.view_mode {
            FileViewMode::List | FileViewMode::Compact => {
                self.paint_list_view(ctx, &rect);
            }
            FileViewMode::Icons => {
                self.paint_icon_view(ctx, &rect);
            }
        }
    }

    fn paint_list_view(&self, ctx: &mut PaintContext<'_>, list_rect: &Rect) {
        for (i, entry) in self.entries.iter().enumerate() {
            let entry_rect = self.entry_rect(i);

            // Skip if outside visible area
            if entry_rect.origin.y + entry_rect.height() < list_rect.origin.y {
                continue;
            }
            if entry_rect.origin.y > list_rect.origin.y + list_rect.height() {
                break;
            }

            // Selection background
            if self.selected_indices.contains(&i) {
                ctx.renderer().fill_rect(entry_rect, self.selection_color);
            } else if self.hovered_entry == Some(i) {
                ctx.renderer().fill_rect(entry_rect, self.hover_color);
            }

            // Icon
            let icon_x = entry_rect.origin.x + self.content_padding;
            let icon_y = entry_rect.origin.y + (entry_rect.height() - 16.0) / 2.0;

            if entry.is_dir {
                self.draw_folder_icon(ctx, Point::new(icon_x, icon_y), 16.0);
            } else {
                self.draw_file_icon(ctx, Point::new(icon_x, icon_y), 16.0);
            }

            // Name, size, date columns would be drawn with text renderer
            // For now we just draw the visual structure
        }
    }

    fn paint_icon_view(&self, ctx: &mut PaintContext<'_>, list_rect: &Rect) {
        for (i, entry) in self.entries.iter().enumerate() {
            let entry_rect = self.entry_rect(i);

            // Skip if outside visible area
            if entry_rect.origin.y + entry_rect.height() < list_rect.origin.y {
                continue;
            }
            if entry_rect.origin.y > list_rect.origin.y + list_rect.height() {
                break;
            }

            // Selection background
            if self.selected_indices.contains(&i) {
                let sel_rect = Rect::new(
                    entry_rect.origin.x + 4.0,
                    entry_rect.origin.y + 4.0,
                    entry_rect.width() - 8.0,
                    entry_rect.height() - 8.0,
                );
                ctx.renderer()
                    .fill_rounded_rect(RoundedRect::new(sel_rect, 4.0), self.selection_color);
            } else if self.hovered_entry == Some(i) {
                let hover_rect = Rect::new(
                    entry_rect.origin.x + 4.0,
                    entry_rect.origin.y + 4.0,
                    entry_rect.width() - 8.0,
                    entry_rect.height() - 8.0,
                );
                ctx.renderer()
                    .fill_rounded_rect(RoundedRect::new(hover_rect, 4.0), self.hover_color);
            }

            // Icon (centered)
            let icon_x = entry_rect.origin.x + (entry_rect.width() - self.icon_size) / 2.0;
            let icon_y = entry_rect.origin.y + 4.0;

            if entry.is_dir {
                self.draw_folder_icon(ctx, Point::new(icon_x, icon_y), self.icon_size);
            } else {
                self.draw_file_icon(ctx, Point::new(icon_x, icon_y), self.icon_size);
            }

            // Name would be drawn below icon with text renderer
        }
    }

    fn paint_filter_bar(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.filter_bar_rect();

        // Background
        ctx.renderer().fill_rect(rect, self.path_bar_color);

        // Top border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(rect.origin.x, rect.origin.y),
            Point::new(rect.origin.x + rect.width(), rect.origin.y),
            &stroke,
        );

        // Filename label and input would be drawn here
        // Filter dropdown would be drawn here
    }

    fn draw_folder_icon(&self, ctx: &mut PaintContext<'_>, pos: Point, size: f32) {
        // Simple folder icon
        let w = size;
        let h = size * 0.8;
        let tab_w = w * 0.4;
        let tab_h = h * 0.15;

        // Folder tab
        let tab_rect = Rect::new(pos.x, pos.y, tab_w, tab_h);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(tab_rect, 2.0), self.folder_icon_color);

        // Folder body
        let body_rect = Rect::new(pos.x, pos.y + tab_h - 1.0, w, h - tab_h + 1.0);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(body_rect, 2.0), self.folder_icon_color);
    }

    fn draw_file_icon(&self, ctx: &mut PaintContext<'_>, pos: Point, size: f32) {
        // Simple file icon
        let w = size * 0.75;
        let h = size;
        let corner = size * 0.2;

        // Main body
        let body_rect = Rect::new(pos.x, pos.y, w, h);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(body_rect, 1.0), Color::WHITE);
        ctx.renderer().stroke_rounded_rect(
            RoundedRect::new(body_rect, 1.0),
            &Stroke::new(self.file_icon_color, 1.0),
        );

        // Folded corner
        let stroke = Stroke::new(self.file_icon_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(pos.x + w - corner, pos.y),
            Point::new(pos.x + w - corner, pos.y + corner),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(pos.x + w - corner, pos.y + corner),
            Point::new(pos.x + w, pos.y + corner),
            &stroke,
        );
    }
}

impl Object for FileDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for FileDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::new(Size::new(800.0, 500.0)).with_minimum(Size::new(400.0, 300.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint FileDialog-specific content
        // Note: paint_path_bar needs mut self, so we use a different approach
        // In a real implementation, we'd pre-calculate rects during layout
        let path_bar_rect = self.path_bar_rect();
        ctx.renderer().fill_rect(path_bar_rect, self.path_bar_color);

        self.paint_sidebar(ctx);
        self.paint_file_list(ctx);
        self.paint_filter_bar(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::Wheel(e) => self.handle_wheel(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            _ => false,
        };

        if handled {
            return true;
        }

        // Delegate to dialog
        self.dialog.event(event)
    }
}

impl Default for FileDialog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Native Dialog Integration
// ============================================================================

use super::native_dialogs::{self, NativeFileDialogOptions, NativeFileFilter};

/// Convert FileFilter to NativeFileFilter.
fn convert_filter(filter: &FileFilter) -> NativeFileFilter {
    // Extract extensions from patterns like "*.rs" -> "rs"
    let extensions: Vec<String> = filter
        .patterns
        .iter()
        .filter_map(|p| {
            if p == "*" {
                Some("*".to_string())
            } else {
                p.strip_prefix("*.").map(|ext| ext.to_string())
            }
        })
        .collect();

    NativeFileFilter {
        name: filter.name.clone(),
        extensions,
    }
}

/// Convert a slice of FileFilters to NativeFileFilters.
fn convert_filters(filters: &[FileFilter]) -> Vec<NativeFileFilter> {
    filters.iter().map(convert_filter).collect()
}

/// Module providing native dialog functions using the native_dialogs backend.
mod native {
    use super::*;

    /// Check if native dialogs are available on this platform.
    pub fn is_available() -> bool {
        native_dialogs::is_available()
    }

    /// Open a native file dialog for a single file.
    pub fn open_file_dialog(title: &str, dir: &Path, filters: &[FileFilter]) -> Option<PathBuf> {
        let options = NativeFileDialogOptions::with_title(title).directory(dir.to_path_buf());

        let mut options = options;
        for filter in convert_filters(filters) {
            options = options.filter(filter);
        }

        native_dialogs::open_file(options)
    }

    /// Open a native file dialog for multiple files.
    pub fn open_files_dialog(
        title: &str,
        dir: &Path,
        filters: &[FileFilter],
    ) -> Option<Vec<PathBuf>> {
        let options = NativeFileDialogOptions::with_title(title)
            .directory(dir.to_path_buf())
            .multiple(true);

        let mut options = options;
        for filter in convert_filters(filters) {
            options = options.filter(filter);
        }

        native_dialogs::open_files(options)
    }

    /// Open a native save dialog.
    pub fn save_file_dialog(title: &str, dir: &Path, filters: &[FileFilter]) -> Option<PathBuf> {
        let options = NativeFileDialogOptions::with_title(title).directory(dir.to_path_buf());

        let mut options = options;
        for filter in convert_filters(filters) {
            options = options.filter(filter);
        }

        native_dialogs::save_file(options)
    }

    /// Open a native directory dialog.
    pub fn directory_dialog(title: &str, dir: &Path) -> Option<PathBuf> {
        let options = NativeFileDialogOptions::with_title(title).directory(dir.to_path_buf());

        native_dialogs::select_directory(options)
    }
}

/// Check if native file dialogs are available.
pub fn native_dialog_available() -> bool {
    native::is_available()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_file_dialog_mode() {
        assert!(!FileDialogMode::OpenFile.is_multi_select());
        assert!(FileDialogMode::OpenFiles.is_multi_select());
        assert!(!FileDialogMode::SaveFile.is_multi_select());
        assert!(!FileDialogMode::Directory.is_multi_select());

        assert!(FileDialogMode::OpenFile.is_open_mode());
        assert!(FileDialogMode::OpenFiles.is_open_mode());
        assert!(!FileDialogMode::SaveFile.is_open_mode());
        assert!(FileDialogMode::Directory.is_open_mode());

        assert!(!FileDialogMode::OpenFile.is_directory_mode());
        assert!(FileDialogMode::Directory.is_directory_mode());
    }

    #[test]
    fn test_file_filter() {
        let rust_filter = FileFilter::rust_files();
        assert!(rust_filter.matches("main.rs"));
        assert!(rust_filter.matches("lib.RS")); // Case insensitive
        assert!(!rust_filter.matches("main.txt"));

        let all_filter = FileFilter::all_files();
        assert!(all_filter.matches("anything.xyz"));
        assert!(all_filter.matches("noextension"));

        let image_filter = FileFilter::image_files();
        assert!(image_filter.matches("photo.png"));
        assert!(image_filter.matches("photo.JPG"));
        assert!(!image_filter.matches("doc.pdf"));
    }

    #[test]
    fn test_file_filter_display() {
        let filter = FileFilter::new("Rust Files", &["*.rs"]);
        assert_eq!(filter.display_text(), "Rust Files (*.rs)");

        let filter2 = FileFilter::new("Images", &["*.png", "*.jpg"]);
        assert_eq!(filter2.display_text(), "Images (*.png, *.jpg)");
    }

    #[test]
    fn test_file_entry() {
        let entry = FileEntry::new(PathBuf::from("/home/user/test.txt"), false);
        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
        assert!(!entry.is_hidden);

        let hidden_entry = FileEntry::new(PathBuf::from("/home/user/.hidden"), false);
        assert!(hidden_entry.is_hidden);

        let dir_entry = FileEntry::new(PathBuf::from("/home/user/docs"), true);
        assert!(dir_entry.is_dir);
    }

    #[test]
    fn test_file_entry_size_string() {
        let mut entry = FileEntry::new(PathBuf::from("test.txt"), false);

        entry.size = 500;
        assert_eq!(entry.size_string(), "500 B");

        entry.size = 1024;
        assert_eq!(entry.size_string(), "1.0 KB");

        entry.size = 1024 * 1024;
        assert_eq!(entry.size_string(), "1.0 MB");

        entry.size = 1024 * 1024 * 1024;
        assert_eq!(entry.size_string(), "1.0 GB");

        entry.is_dir = true;
        assert_eq!(entry.size_string(), "");
    }

    #[test]
    fn test_bookmark_entry() {
        let bookmark = BookmarkEntry::new("Test", PathBuf::from("/test"), BookmarkIcon::Folder);
        assert_eq!(bookmark.name, "Test");
        assert_eq!(bookmark.path, PathBuf::from("/test"));
    }

    #[test]
    fn test_file_dialog_creation() {
        setup();

        let dialog = FileDialog::new();
        assert_eq!(dialog.mode(), FileDialogMode::OpenFile);
        assert!(!dialog.is_open());
        assert!(!dialog.filters().is_empty());
    }

    #[test]
    fn test_file_dialog_factory_methods() {
        setup();

        let open = FileDialog::for_open();
        assert_eq!(open.mode(), FileDialogMode::OpenFile);

        let open_multi = FileDialog::for_open_multiple();
        assert_eq!(open_multi.mode(), FileDialogMode::OpenFiles);

        let save = FileDialog::for_save();
        assert_eq!(save.mode(), FileDialogMode::SaveFile);

        let dir = FileDialog::for_directory();
        assert_eq!(dir.mode(), FileDialogMode::Directory);
    }

    #[test]
    fn test_file_dialog_builder() {
        setup();

        let dialog = FileDialog::new()
            .with_mode(FileDialogMode::SaveFile)
            .with_title("Save As")
            .with_filter(FileFilter::rust_files())
            .with_filter(FileFilter::all_files())
            .with_show_hidden(true)
            .with_default_filename("untitled.rs");

        assert_eq!(dialog.mode(), FileDialogMode::SaveFile);
        assert_eq!(dialog.filters().len(), 3); // Default + 2 added
        assert!(dialog.show_hidden());
        assert_eq!(dialog.filename_text(), "untitled.rs");
    }

    #[test]
    fn test_file_dialog_static_helpers() {
        setup();

        let dialog = FileDialog::get_open_file_name("Open", "/tmp", &[FileFilter::text_files()]);
        assert_eq!(dialog.mode(), FileDialogMode::OpenFile);

        let dialog =
            FileDialog::get_open_file_names("Open Multiple", "/tmp", &[FileFilter::all_files()]);
        assert_eq!(dialog.mode(), FileDialogMode::OpenFiles);

        let dialog = FileDialog::get_save_file_name("Save", "/tmp", &[FileFilter::text_files()]);
        assert_eq!(dialog.mode(), FileDialogMode::SaveFile);

        let dialog = FileDialog::get_existing_directory("Select Folder", "/tmp");
        assert_eq!(dialog.mode(), FileDialogMode::Directory);
    }

    #[test]
    fn test_file_dialog_filter_selection() {
        setup();

        let mut dialog = FileDialog::new()
            .with_filter(FileFilter::rust_files())
            .with_filter(FileFilter::text_files());

        assert_eq!(dialog.current_filter_index(), 0);

        dialog.set_current_filter_index(1);
        assert_eq!(dialog.current_filter_index(), 1);

        // Out of bounds should not change
        dialog.set_current_filter_index(999);
        assert_eq!(dialog.current_filter_index(), 1);
    }

    #[test]
    fn test_file_dialog_bookmarks() {
        setup();

        let mut dialog = FileDialog::new();

        // System bookmarks should be populated
        assert!(!dialog.system_bookmarks.is_empty());

        // Add user bookmark
        dialog.add_bookmark(BookmarkEntry::new(
            "Projects",
            PathBuf::from("/home/user/projects"),
            BookmarkIcon::Folder,
        ));
        assert_eq!(dialog.bookmarks().len(), 1);

        // Remove bookmark
        dialog.remove_bookmark(&PathBuf::from("/home/user/projects"));
        assert_eq!(dialog.bookmarks().len(), 0);
    }

    #[test]
    fn test_file_dialog_recent_locations() {
        setup();

        let mut dialog = FileDialog::new();
        assert!(dialog.recent_locations().is_empty());

        dialog.add_recent_location(PathBuf::from("/path1"));
        dialog.add_recent_location(PathBuf::from("/path2"));
        dialog.add_recent_location(PathBuf::from("/path1")); // Duplicate

        // Should have 2 unique, with path1 at front (most recent)
        assert_eq!(dialog.recent_locations().len(), 2);
        assert_eq!(dialog.recent_locations()[0], PathBuf::from("/path1"));
    }

    #[test]
    fn test_view_mode() {
        setup();

        let mut dialog = FileDialog::new();
        assert_eq!(dialog.view_mode(), FileViewMode::List);

        dialog.set_view_mode(FileViewMode::Icons);
        assert_eq!(dialog.view_mode(), FileViewMode::Icons);

        dialog.set_view_mode(FileViewMode::Compact);
        assert_eq!(dialog.view_mode(), FileViewMode::Compact);
    }
}
