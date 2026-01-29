//! Print dialog implementation.
//!
//! This module provides [`PrintDialog`] and [`PrintPreviewDialog`], modal dialogs for
//! configuring print settings and previewing print output.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{PrintDialog, PrintSettings};
//!
//! // Create a print dialog
//! let mut dialog = PrintDialog::new();
//!
//! dialog.accepted.connect(|settings| {
//!     println!("Printing with settings: {:?}", settings);
//! });
//!
//! dialog.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Size, Stroke};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, PaintContext, SizeHint,
    WheelEvent, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;

// ============================================================================
// Page Orientation
// ============================================================================

/// Page orientation for printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageOrientation {
    /// Portrait orientation (taller than wide).
    #[default]
    Portrait,

    /// Landscape orientation (wider than tall).
    Landscape,
}

impl PageOrientation {
    /// Returns the display name for this orientation.
    pub fn display_name(&self) -> &'static str {
        match self {
            PageOrientation::Portrait => "Portrait",
            PageOrientation::Landscape => "Landscape",
        }
    }
}

// ============================================================================
// Color Mode
// ============================================================================

/// Color mode for printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Full color printing.
    #[default]
    Color,

    /// Grayscale printing.
    Grayscale,
}

impl ColorMode {
    /// Returns the display name for this color mode.
    pub fn display_name(&self) -> &'static str {
        match self {
            ColorMode::Color => "Color",
            ColorMode::Grayscale => "Grayscale",
        }
    }
}

// ============================================================================
// Duplex Mode
// ============================================================================

/// Duplex (double-sided) printing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplexMode {
    /// Single-sided printing.
    #[default]
    None,

    /// Double-sided, flip on long edge (like a book).
    LongEdge,

    /// Double-sided, flip on short edge (like a notepad).
    ShortEdge,
}

impl DuplexMode {
    /// Returns the display name for this duplex mode.
    pub fn display_name(&self) -> &'static str {
        match self {
            DuplexMode::None => "Off",
            DuplexMode::LongEdge => "Long Edge",
            DuplexMode::ShortEdge => "Short Edge",
        }
    }
}

// ============================================================================
// Paper Size
// ============================================================================

/// Standard paper sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    /// US Letter (8.5 x 11 inches)
    #[default]
    Letter,

    /// US Legal (8.5 x 14 inches)
    Legal,

    /// A4 (210 x 297 mm)
    A4,

    /// A3 (297 x 420 mm)
    A3,

    /// A5 (148 x 210 mm)
    A5,

    /// B5 (176 x 250 mm)
    B5,

    /// Tabloid (11 x 17 inches)
    Tabloid,

    /// Executive (7.25 x 10.5 inches)
    Executive,

    /// Custom size
    Custom,
}

impl PaperSize {
    /// Returns the display name for this paper size.
    pub fn display_name(&self) -> &'static str {
        match self {
            PaperSize::Letter => "Letter (8.5\" × 11\")",
            PaperSize::Legal => "Legal (8.5\" × 14\")",
            PaperSize::A4 => "A4 (210 × 297 mm)",
            PaperSize::A3 => "A3 (297 × 420 mm)",
            PaperSize::A5 => "A5 (148 × 210 mm)",
            PaperSize::B5 => "B5 (176 × 250 mm)",
            PaperSize::Tabloid => "Tabloid (11\" × 17\")",
            PaperSize::Executive => "Executive (7.25\" × 10.5\")",
            PaperSize::Custom => "Custom",
        }
    }

    /// Returns all standard paper sizes.
    pub fn all() -> &'static [PaperSize] {
        &[
            PaperSize::Letter,
            PaperSize::Legal,
            PaperSize::A4,
            PaperSize::A3,
            PaperSize::A5,
            PaperSize::B5,
            PaperSize::Tabloid,
            PaperSize::Executive,
        ]
    }

    /// Returns the size in points (1 point = 1/72 inch).
    pub fn size_in_points(&self) -> (f32, f32) {
        match self {
            PaperSize::Letter => (612.0, 792.0),    // 8.5 x 11 inches
            PaperSize::Legal => (612.0, 1008.0),    // 8.5 x 14 inches
            PaperSize::A4 => (595.0, 842.0),        // 210 x 297 mm
            PaperSize::A3 => (842.0, 1191.0),       // 297 x 420 mm
            PaperSize::A5 => (420.0, 595.0),        // 148 x 210 mm
            PaperSize::B5 => (499.0, 709.0),        // 176 x 250 mm
            PaperSize::Tabloid => (792.0, 1224.0),  // 11 x 17 inches
            PaperSize::Executive => (522.0, 756.0), // 7.25 x 10.5 inches
            PaperSize::Custom => (612.0, 792.0),    // Default to letter
        }
    }
}

// ============================================================================
// Page Range
// ============================================================================

/// Specifies which pages to print.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PageRange {
    /// Print all pages.
    #[default]
    All,

    /// Print the current page only.
    CurrentPage,

    /// Print selected content only.
    Selection,

    /// Print a specific range of pages.
    Range {
        /// Starting page (1-indexed).
        from: u32,
        /// Ending page (1-indexed, inclusive).
        to: u32,
    },

    /// Print specific pages (e.g., "1,3,5-7").
    Pages(Vec<u32>),
}

impl PageRange {
    /// Create a page range from a string like "1-5" or "1,3,5-7".
    pub fn from_string(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        let mut pages = Vec::new();

        for part in s.split(',') {
            let part = part.trim();
            if part.contains('-') {
                let mut iter = part.split('-');
                let from: u32 = iter.next()?.trim().parse().ok()?;
                let to: u32 = iter.next()?.trim().parse().ok()?;
                if from > 0 && to >= from {
                    for p in from..=to {
                        if !pages.contains(&p) {
                            pages.push(p);
                        }
                    }
                }
            } else {
                let page: u32 = part.parse().ok()?;
                if page > 0 && !pages.contains(&page) {
                    pages.push(page);
                }
            }
        }

        if pages.is_empty() {
            None
        } else {
            pages.sort_unstable();
            Some(PageRange::Pages(pages))
        }
    }

    /// Returns a display string for this page range.
    pub fn display_string(&self) -> String {
        match self {
            PageRange::All => "All".to_string(),
            PageRange::CurrentPage => "Current Page".to_string(),
            PageRange::Selection => "Selection".to_string(),
            PageRange::Range { from, to } => format!("{}-{}", from, to),
            PageRange::Pages(pages) => {
                // Compress consecutive pages into ranges
                if pages.is_empty() {
                    return String::new();
                }
                let mut result = String::new();
                let mut i = 0;
                while i < pages.len() {
                    let start = pages[i];
                    let mut end = start;
                    while i + 1 < pages.len() && pages[i + 1] == end + 1 {
                        end = pages[i + 1];
                        i += 1;
                    }
                    if !result.is_empty() {
                        result.push_str(", ");
                    }
                    if start == end {
                        result.push_str(&start.to_string());
                    } else {
                        result.push_str(&format!("{}-{}", start, end));
                    }
                    i += 1;
                }
                result
            }
        }
    }
}

// ============================================================================
// Printer Info
// ============================================================================

/// Information about an available printer.
#[derive(Debug, Clone)]
pub struct PrinterInfo {
    /// Unique identifier for the printer.
    pub id: String,

    /// Display name of the printer.
    pub name: String,

    /// Description or location.
    pub description: String,

    /// Whether this is the default printer.
    pub is_default: bool,

    /// Whether the printer supports color.
    pub supports_color: bool,

    /// Whether the printer supports duplex.
    pub supports_duplex: bool,

    /// Whether the printer is currently available.
    pub is_available: bool,

    /// Supported paper sizes.
    pub paper_sizes: Vec<PaperSize>,
}

impl PrinterInfo {
    /// Create a new printer info.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            is_default: false,
            supports_color: true,
            supports_duplex: false,
            is_available: true,
            paper_sizes: PaperSize::all().to_vec(),
        }
    }

    /// Builder: set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder: set as default printer.
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// Builder: set color support.
    pub fn with_color_support(mut self, supports: bool) -> Self {
        self.supports_color = supports;
        self
    }

    /// Builder: set duplex support.
    pub fn with_duplex_support(mut self, supports: bool) -> Self {
        self.supports_duplex = supports;
        self
    }
}

impl Default for PrinterInfo {
    fn default() -> Self {
        Self::new("default", "Default Printer").with_default(true)
    }
}

// ============================================================================
// Print Settings
// ============================================================================

/// Complete print settings configuration.
#[derive(Debug, Clone)]
pub struct PrintSettings {
    /// Selected printer ID.
    pub printer_id: String,

    /// Number of copies to print.
    pub copies: u32,

    /// Whether to collate multiple copies.
    pub collate: bool,

    /// Page range to print.
    pub page_range: PageRange,

    /// Page orientation.
    pub orientation: PageOrientation,

    /// Paper size.
    pub paper_size: PaperSize,

    /// Color mode.
    pub color_mode: ColorMode,

    /// Duplex mode.
    pub duplex: DuplexMode,

    /// Print to file instead of printer.
    pub print_to_file: bool,

    /// Output file path (when print_to_file is true).
    pub output_file: Option<String>,
}

impl PrintSettings {
    /// Create new print settings with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create print settings for a specific printer.
    pub fn for_printer(printer_id: impl Into<String>) -> Self {
        Self {
            printer_id: printer_id.into(),
            ..Default::default()
        }
    }
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            printer_id: "default".to_string(),
            copies: 1,
            collate: true,
            page_range: PageRange::All,
            orientation: PageOrientation::Portrait,
            paper_size: PaperSize::Letter,
            color_mode: ColorMode::Color,
            duplex: DuplexMode::None,
            print_to_file: false,
            output_file: None,
        }
    }
}

// ============================================================================
// Print Dialog Options
// ============================================================================

/// Options for configuring the print dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrintDialogOptions(u32);

impl PrintDialogOptions {
    /// No options.
    pub const NONE: Self = Self(0);

    /// Enable print to file option.
    pub const PRINT_TO_FILE: Self = Self(1 << 0);

    /// Enable print selection option.
    pub const PRINT_SELECTION: Self = Self(1 << 1);

    /// Enable page range selection.
    pub const PRINT_PAGE_RANGE: Self = Self(1 << 2);

    /// Show page size and margins options.
    pub const SHOW_PAGE_SIZE: Self = Self(1 << 3);

    /// Enable collate copies option.
    pub const PRINT_COLLATE_COPIES: Self = Self(1 << 4);

    /// Enable current page option.
    pub const PRINT_CURRENT_PAGE: Self = Self(1 << 5);

    /// All options enabled.
    pub const ALL: Self = Self(0xFF);

    /// Check if an option is set.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for PrintDialogOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PrintDialogOptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ============================================================================
// UI Section
// ============================================================================

/// Active section in the dialog for hover/click detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogSection {
    PrinterList,
    CopiesSpinner,
    CollateCheckbox,
    PageRangeAll,
    PageRangePages,
    PageRangeInput,
    OrientationPortrait,
    OrientationLandscape,
    PaperSizeDropdown,
    ColorModeDropdown,
    DuplexDropdown,
    PrintToFile,
}

// ============================================================================
// PrintDialog
// ============================================================================

/// A modal dialog for configuring print settings.
///
/// PrintDialog provides a standardized interface for selecting a printer
/// and configuring print options including:
///
/// - Printer selection from available printers
/// - Number of copies and collation
/// - Page range (all, current page, selection, or specific pages)
/// - Page orientation (portrait/landscape)
/// - Paper size selection
/// - Color mode (color/grayscale)
/// - Duplex (double-sided) printing
///
/// # Signals
///
/// - `print_requested`: Emitted when the user clicks Print with current settings
///
/// # Example
///
/// ```ignore
/// let mut dialog = PrintDialog::new();
///
/// dialog.print_requested.connect(|settings| {
///     // Handle printing with the provided settings
///     println!("Print {} copies", settings.copies);
/// });
///
/// dialog.open();
/// ```
#[allow(dead_code)]
pub struct PrintDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// Available printers.
    printers: Vec<PrinterInfo>,

    /// Currently selected printer index.
    selected_printer: usize,

    /// Current print settings.
    settings: PrintSettings,

    /// Dialog options.
    options: PrintDialogOptions,

    /// Whether selection printing is available (content is selected).
    has_selection: bool,

    /// Current page number (for current page printing).
    current_page: Option<u32>,

    /// Total page count (for validation).
    total_pages: Option<u32>,

    /// Page range input text.
    page_range_text: String,

    // UI state
    /// Which section is hovered.
    hovered_section: Option<DialogSection>,

    /// Printer list scroll position.
    printer_scroll: f32,

    /// Hovered printer index.
    hovered_printer: Option<usize>,

    // Visual styling
    /// Content padding.
    content_padding: f32,

    /// Section spacing.
    section_spacing: f32,

    /// Label width.
    label_width: f32,

    /// Row height.
    row_height: f32,

    /// Printer list height.
    printer_list_height: f32,

    /// Colors
    background_color: Color,
    section_color: Color,
    selection_color: Color,
    hover_color: Color,
    text_color: Color,
    secondary_text_color: Color,
    border_color: Color,
    input_background: Color,
    disabled_color: Color,

    // Signals
    /// Signal emitted when print is requested with settings.
    pub print_requested: Signal<PrintSettings>,
}

impl PrintDialog {
    /// Create a new print dialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Print")
            .with_size(500.0, 480.0)
            .with_standard_buttons(StandardButton::CANCEL);

        // Get available printers (placeholder - would use system API)
        let printers = Self::get_available_printers();
        let default_printer = printers.iter().position(|p| p.is_default).unwrap_or(0);

        Self {
            dialog,
            printers,
            selected_printer: default_printer,
            settings: PrintSettings::default(),
            options: PrintDialogOptions::PRINT_PAGE_RANGE
                | PrintDialogOptions::PRINT_COLLATE_COPIES
                | PrintDialogOptions::PRINT_CURRENT_PAGE,
            has_selection: false,
            current_page: None,
            total_pages: None,
            page_range_text: String::new(),
            hovered_section: None,
            printer_scroll: 0.0,
            hovered_printer: None,
            content_padding: 16.0,
            section_spacing: 16.0,
            label_width: 100.0,
            row_height: 28.0,
            printer_list_height: 80.0,
            background_color: Color::WHITE,
            section_color: Color::from_rgb8(248, 248, 248),
            selection_color: Color::from_rgba8(0, 120, 215, 80),
            hover_color: Color::from_rgba8(0, 0, 0, 20),
            text_color: Color::from_rgb8(32, 32, 32),
            secondary_text_color: Color::from_rgb8(128, 128, 128),
            border_color: Color::from_rgb8(200, 200, 200),
            input_background: Color::WHITE,
            disabled_color: Color::from_rgb8(180, 180, 180),
            print_requested: Signal::new(),
        }
    }

    /// Get available printers from the system.
    ///
    /// Note: This is a placeholder implementation. A real implementation would
    /// query the operating system for available printers.
    fn get_available_printers() -> Vec<PrinterInfo> {
        // Placeholder printers for demonstration
        vec![
            PrinterInfo::new("default", "Default Printer")
                .with_description("Local printer")
                .with_default(true)
                .with_color_support(true)
                .with_duplex_support(true),
            PrinterInfo::new("pdf", "Save as PDF")
                .with_description("Save document as PDF file")
                .with_color_support(true),
            PrinterInfo::new("network1", "Office Printer")
                .with_description("Network printer - Building A")
                .with_color_support(true)
                .with_duplex_support(true),
        ]
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the available printers using builder pattern.
    pub fn with_printers(mut self, printers: Vec<PrinterInfo>) -> Self {
        self.printers = printers;
        self.selected_printer = self.printers.iter().position(|p| p.is_default).unwrap_or(0);
        self
    }

    /// Set dialog options using builder pattern.
    pub fn with_options(mut self, options: PrintDialogOptions) -> Self {
        self.options = options;
        self
    }

    /// Set initial print settings using builder pattern.
    pub fn with_settings(mut self, settings: PrintSettings) -> Self {
        self.settings = settings;
        // Find the printer index
        if let Some(idx) = self
            .printers
            .iter()
            .position(|p| p.id == self.settings.printer_id)
        {
            self.selected_printer = idx;
        }
        self
    }

    /// Set whether there is selected content using builder pattern.
    pub fn with_selection(mut self, has_selection: bool) -> Self {
        self.has_selection = has_selection;
        if has_selection {
            self.options |= PrintDialogOptions::PRINT_SELECTION;
        }
        self
    }

    /// Set the current page number using builder pattern.
    pub fn with_current_page(mut self, page: u32) -> Self {
        self.current_page = Some(page);
        self.options |= PrintDialogOptions::PRINT_CURRENT_PAGE;
        self
    }

    /// Set the total page count using builder pattern.
    pub fn with_total_pages(mut self, total: u32) -> Self {
        self.total_pages = Some(total);
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the current print settings.
    pub fn settings(&self) -> &PrintSettings {
        &self.settings
    }

    /// Get mutable access to the print settings.
    pub fn settings_mut(&mut self) -> &mut PrintSettings {
        &mut self.settings
    }

    /// Get the selected printer.
    pub fn selected_printer(&self) -> Option<&PrinterInfo> {
        self.printers.get(self.selected_printer)
    }

    /// Set the selected printer by index.
    pub fn set_selected_printer(&mut self, index: usize) {
        if index < self.printers.len() {
            self.selected_printer = index;
            if let Some(printer) = self.printers.get(index) {
                self.settings.printer_id = printer.id.clone();
            }
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the number of copies.
    pub fn copies(&self) -> u32 {
        self.settings.copies
    }

    /// Set the number of copies.
    pub fn set_copies(&mut self, copies: u32) {
        self.settings.copies = copies.max(1);
        self.dialog.widget_base_mut().update();
    }

    /// Get whether collation is enabled.
    pub fn collate(&self) -> bool {
        self.settings.collate
    }

    /// Set whether to collate copies.
    pub fn set_collate(&mut self, collate: bool) {
        self.settings.collate = collate;
        self.dialog.widget_base_mut().update();
    }

    /// Get the page range.
    pub fn page_range(&self) -> &PageRange {
        &self.settings.page_range
    }

    /// Set the page range.
    pub fn set_page_range(&mut self, range: PageRange) {
        self.settings.page_range = range;
        self.dialog.widget_base_mut().update();
    }

    /// Get the page orientation.
    pub fn orientation(&self) -> PageOrientation {
        self.settings.orientation
    }

    /// Set the page orientation.
    pub fn set_orientation(&mut self, orientation: PageOrientation) {
        self.settings.orientation = orientation;
        self.dialog.widget_base_mut().update();
    }

    /// Get the paper size.
    pub fn paper_size(&self) -> PaperSize {
        self.settings.paper_size
    }

    /// Set the paper size.
    pub fn set_paper_size(&mut self, size: PaperSize) {
        self.settings.paper_size = size;
        self.dialog.widget_base_mut().update();
    }

    /// Get the color mode.
    pub fn color_mode(&self) -> ColorMode {
        self.settings.color_mode
    }

    /// Set the color mode.
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.settings.color_mode = mode;
        self.dialog.widget_base_mut().update();
    }

    /// Get the duplex mode.
    pub fn duplex(&self) -> DuplexMode {
        self.settings.duplex
    }

    /// Set the duplex mode.
    pub fn set_duplex(&mut self, mode: DuplexMode) {
        self.settings.duplex = mode;
        self.dialog.widget_base_mut().update();
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the print dialog.
    pub fn open(&mut self) {
        self.dialog.open();
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

    /// Accept the dialog and emit print request.
    fn accept(&mut self) {
        // Update settings from UI state
        self.settings.printer_id = self
            .printers
            .get(self.selected_printer)
            .map(|p| p.id.clone())
            .unwrap_or_default();

        // Emit signal
        self.print_requested.emit(self.settings.clone());
        self.dialog.accept();
    }

    /// Reject/cancel the dialog.
    fn reject(&mut self) {
        self.dialog.reject();
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
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

    /// Get the content area rectangle.
    fn content_rect(&self) -> Rect {
        let rect = self.dialog.widget_base().rect();
        Rect::new(
            self.content_padding,
            self.title_bar_height() + self.content_padding,
            rect.width() - self.content_padding * 2.0,
            rect.height()
                - self.title_bar_height()
                - self.button_box_height()
                - self.content_padding * 2.0,
        )
    }

    /// Get the printer list rectangle.
    fn printer_list_rect(&self) -> Rect {
        let content = self.content_rect();
        Rect::new(
            content.origin.x,
            content.origin.y + self.row_height,
            content.width(),
            self.printer_list_height,
        )
    }

    /// Get a printer item rectangle within the list.
    fn printer_item_rect(&self, index: usize) -> Rect {
        let list = self.printer_list_rect();
        Rect::new(
            list.origin.x + 2.0,
            list.origin.y + 2.0 + (index as f32 * self.row_height) - self.printer_scroll,
            list.width() - 4.0,
            self.row_height - 2.0,
        )
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let content = self.content_rect();
        let mut y = content.origin.y;

        // Skip "Printer:" label row
        y += self.row_height;

        // Printer list
        let list_rect = self.printer_list_rect();
        if list_rect.contains(pos) {
            for (i, _) in self.printers.iter().enumerate() {
                let item_rect = self.printer_item_rect(i);
                if item_rect.contains(pos) && item_rect.origin.y >= list_rect.origin.y {
                    self.selected_printer = i;
                    self.settings.printer_id = self.printers[i].id.clone();
                    self.dialog.widget_base_mut().update();
                    return true;
                }
            }
        }

        y += self.printer_list_height + self.section_spacing;

        // Copies section
        let copies_row = Rect::new(content.origin.x, y, content.width(), self.row_height);
        if copies_row.contains(pos) {
            // Increment/decrement based on click position
            let control_width = 80.0;
            let control_x = content.origin.x + self.label_width;

            if pos.x > control_x + control_width - 20.0 {
                // Increment button
                self.settings.copies += 1;
            } else if pos.x > control_x && pos.x < control_x + 20.0 && self.settings.copies > 1 {
                // Decrement button
                self.settings.copies -= 1;
            }
            self.dialog.widget_base_mut().update();
            return true;
        }

        y += self.row_height;

        // Collate checkbox (if copies > 1)
        if self.settings.copies > 1
            && self
                .options
                .contains(PrintDialogOptions::PRINT_COLLATE_COPIES)
        {
            let collate_row = Rect::new(content.origin.x, y, content.width(), self.row_height);
            if collate_row.contains(pos) {
                self.settings.collate = !self.settings.collate;
                self.dialog.widget_base_mut().update();
                return true;
            }
            y += self.row_height;
        }

        y += self.section_spacing;

        // Page range section
        if self.options.contains(PrintDialogOptions::PRINT_PAGE_RANGE) {
            // "All" option
            let all_row = Rect::new(content.origin.x, y, content.width(), self.row_height);
            if all_row.contains(pos) {
                self.settings.page_range = PageRange::All;
                self.dialog.widget_base_mut().update();
                return true;
            }
            y += self.row_height;

            // "Current Page" option
            if self
                .options
                .contains(PrintDialogOptions::PRINT_CURRENT_PAGE)
                && self.current_page.is_some()
            {
                let current_row = Rect::new(content.origin.x, y, content.width(), self.row_height);
                if current_row.contains(pos) {
                    self.settings.page_range = PageRange::CurrentPage;
                    self.dialog.widget_base_mut().update();
                    return true;
                }
                y += self.row_height;
            }

            // "Selection" option
            if self.options.contains(PrintDialogOptions::PRINT_SELECTION) && self.has_selection {
                let selection_row =
                    Rect::new(content.origin.x, y, content.width(), self.row_height);
                if selection_row.contains(pos) {
                    self.settings.page_range = PageRange::Selection;
                    self.dialog.widget_base_mut().update();
                    return true;
                }
                y += self.row_height;
            }

            // "Pages" option with text input
            let pages_row = Rect::new(content.origin.x, y, content.width(), self.row_height);
            if pages_row.contains(pos) {
                // If clicking on the input field area, just select the option
                if let Some(range) = PageRange::from_string(&self.page_range_text) {
                    self.settings.page_range = range;
                } else {
                    self.settings.page_range = PageRange::Pages(vec![]);
                }
                self.dialog.widget_base_mut().update();
                return true;
            }
            y += self.row_height;
        }

        y += self.section_spacing;

        // Orientation section
        let portrait_row = Rect::new(content.origin.x, y, content.width() / 2.0, self.row_height);
        let landscape_row = Rect::new(
            content.origin.x + content.width() / 2.0,
            y,
            content.width() / 2.0,
            self.row_height,
        );

        if portrait_row.contains(pos) {
            self.settings.orientation = PageOrientation::Portrait;
            self.dialog.widget_base_mut().update();
            return true;
        }
        if landscape_row.contains(pos) {
            self.settings.orientation = PageOrientation::Landscape;
            self.dialog.widget_base_mut().update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;
        let mut needs_update = false;

        // Check printer list hover
        let list_rect = self.printer_list_rect();
        if list_rect.contains(pos) {
            let mut new_hover = None;
            for (i, _) in self.printers.iter().enumerate() {
                let item_rect = self.printer_item_rect(i);
                if item_rect.contains(pos) && item_rect.origin.y >= list_rect.origin.y {
                    new_hover = Some(i);
                    break;
                }
            }
            if self.hovered_printer != new_hover {
                self.hovered_printer = new_hover;
                needs_update = true;
            }
        } else if self.hovered_printer.is_some() {
            self.hovered_printer = None;
            needs_update = true;
        }

        if needs_update {
            self.dialog.widget_base_mut().update();
        }

        needs_update
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let list_rect = self.printer_list_rect();
        if list_rect.contains(event.local_pos) {
            let delta = event.delta_y * 3.0;
            let max_scroll =
                ((self.printers.len() as f32 * self.row_height) - list_rect.height()).max(0.0);
            self.printer_scroll = (self.printer_scroll - delta).clamp(0.0, max_scroll);
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Enter => {
                self.accept();
                return true;
            }
            Key::Escape => {
                self.reject();
                return true;
            }
            _ => {}
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_content(&self, ctx: &mut PaintContext<'_>) {
        let content = self.content_rect();
        let mut y = content.origin.y;

        // Section: Printer
        self.paint_section_header(ctx, "Printer", content.origin.x, y);
        y += self.row_height;

        // Printer list
        self.paint_printer_list(ctx);
        y += self.printer_list_height + self.section_spacing;

        // Section: Copies
        self.paint_section_header(ctx, "Copies", content.origin.x, y);
        self.paint_spinner(
            ctx,
            content.origin.x + self.label_width,
            y,
            80.0,
            self.settings.copies,
        );
        y += self.row_height;

        // Collate checkbox
        if self.settings.copies > 1
            && self
                .options
                .contains(PrintDialogOptions::PRINT_COLLATE_COPIES)
        {
            self.paint_checkbox(
                ctx,
                content.origin.x + 20.0,
                y,
                "Collate",
                self.settings.collate,
            );
            y += self.row_height;
        }

        y += self.section_spacing;

        // Section: Pages
        if self.options.contains(PrintDialogOptions::PRINT_PAGE_RANGE) {
            self.paint_section_header(ctx, "Pages", content.origin.x, y);
            y += self.row_height;

            // Radio: All
            self.paint_radio(
                ctx,
                content.origin.x + 20.0,
                y,
                "All",
                matches!(self.settings.page_range, PageRange::All),
            );
            y += self.row_height;

            // Radio: Current Page
            if self
                .options
                .contains(PrintDialogOptions::PRINT_CURRENT_PAGE)
                && self.current_page.is_some()
            {
                let label = format!("Current page ({})", self.current_page.unwrap());
                self.paint_radio(
                    ctx,
                    content.origin.x + 20.0,
                    y,
                    &label,
                    matches!(self.settings.page_range, PageRange::CurrentPage),
                );
                y += self.row_height;
            }

            // Radio: Selection
            if self.options.contains(PrintDialogOptions::PRINT_SELECTION) && self.has_selection {
                self.paint_radio(
                    ctx,
                    content.origin.x + 20.0,
                    y,
                    "Selection",
                    matches!(self.settings.page_range, PageRange::Selection),
                );
                y += self.row_height;
            }

            // Radio: Pages with input
            let is_pages = matches!(
                self.settings.page_range,
                PageRange::Pages(_) | PageRange::Range { .. }
            );
            self.paint_radio(ctx, content.origin.x + 20.0, y, "Pages:", is_pages);

            // Page range input field
            let input_rect = Rect::new(
                content.origin.x + 100.0,
                y + 2.0,
                content.width() - 120.0,
                self.row_height - 4.0,
            );
            ctx.renderer()
                .fill_rounded_rect(RoundedRect::new(input_rect, 3.0), self.input_background);
            ctx.renderer().stroke_rounded_rect(
                RoundedRect::new(input_rect, 3.0),
                &Stroke::new(self.border_color, 1.0),
            );
            // Text would be rendered here
            y += self.row_height;
        }

        y += self.section_spacing;

        // Section: Layout
        self.paint_section_header(ctx, "Layout", content.origin.x, y);
        y += self.row_height;

        // Orientation radio buttons (side by side)
        let half_width = (content.width() - 20.0) / 2.0;
        self.paint_radio(
            ctx,
            content.origin.x + 20.0,
            y,
            "Portrait",
            self.settings.orientation == PageOrientation::Portrait,
        );
        self.paint_radio(
            ctx,
            content.origin.x + 20.0 + half_width,
            y,
            "Landscape",
            self.settings.orientation == PageOrientation::Landscape,
        );
        y += self.row_height;

        // Paper size
        self.paint_label(ctx, "Paper size:", content.origin.x + 20.0, y);
        self.paint_dropdown(
            ctx,
            content.origin.x + self.label_width,
            y,
            content.width() - self.label_width - 20.0,
            self.settings.paper_size.display_name(),
        );
        y += self.row_height;

        // Color mode (if printer supports it)
        if let Some(printer) = self.printers.get(self.selected_printer) {
            if printer.supports_color {
                self.paint_label(ctx, "Color:", content.origin.x + 20.0, y);
                self.paint_dropdown(
                    ctx,
                    content.origin.x + self.label_width,
                    y,
                    content.width() - self.label_width - 20.0,
                    self.settings.color_mode.display_name(),
                );
                y += self.row_height;
            }

            // Duplex (if printer supports it)
            if printer.supports_duplex {
                self.paint_label(ctx, "Two-sided:", content.origin.x + 20.0, y);
                self.paint_dropdown(
                    ctx,
                    content.origin.x + self.label_width,
                    y,
                    content.width() - self.label_width - 20.0,
                    self.settings.duplex.display_name(),
                );
            }
        }
    }

    fn paint_section_header(&self, ctx: &mut PaintContext<'_>, text: &str, x: f32, y: f32) {
        // Draw section label (text rendering would happen here)
        // For now, just draw a visual indicator
        let _ = (ctx, text, x, y);
    }

    fn paint_label(&self, ctx: &mut PaintContext<'_>, text: &str, x: f32, y: f32) {
        let _ = (ctx, text, x, y);
        // Text rendering would happen here
    }

    fn paint_printer_list(&self, ctx: &mut PaintContext<'_>) {
        let list_rect = self.printer_list_rect();

        // Background
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(list_rect, 3.0), self.section_color);
        ctx.renderer().stroke_rounded_rect(
            RoundedRect::new(list_rect, 3.0),
            &Stroke::new(self.border_color, 1.0),
        );

        // Draw printer items
        for (i, printer) in self.printers.iter().enumerate() {
            let item_rect = self.printer_item_rect(i);

            // Skip if outside visible area
            if item_rect.origin.y + item_rect.height() < list_rect.origin.y {
                continue;
            }
            if item_rect.origin.y > list_rect.origin.y + list_rect.height() {
                break;
            }

            // Selection/hover background
            if i == self.selected_printer {
                ctx.renderer()
                    .fill_rounded_rect(RoundedRect::new(item_rect, 2.0), self.selection_color);
            } else if self.hovered_printer == Some(i) {
                ctx.renderer()
                    .fill_rounded_rect(RoundedRect::new(item_rect, 2.0), self.hover_color);
            }

            // Printer icon
            self.paint_printer_icon(
                ctx,
                Point::new(item_rect.origin.x + 4.0, item_rect.origin.y + 4.0),
                self.row_height - 8.0,
            );

            // Default indicator
            if printer.is_default {
                // Draw a small checkmark or star
                self.paint_default_indicator(
                    ctx,
                    Point::new(
                        item_rect.origin.x + item_rect.width() - 20.0,
                        item_rect.origin.y + 8.0,
                    ),
                );
            }
        }
    }

    fn paint_printer_icon(&self, ctx: &mut PaintContext<'_>, pos: Point, size: f32) {
        // Simple printer icon
        let body_rect = Rect::new(pos.x + 2.0, pos.y + size * 0.3, size - 4.0, size * 0.5);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(body_rect, 2.0), self.secondary_text_color);

        // Paper tray
        let tray_rect = Rect::new(pos.x + 4.0, pos.y + size * 0.1, size - 8.0, size * 0.25);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(tray_rect, 1.0), self.input_background);
    }

    fn paint_default_indicator(&self, ctx: &mut PaintContext<'_>, pos: Point) {
        // Draw a small star or checkmark
        let stroke = Stroke::new(Color::from_rgb8(0, 150, 0), 2.0);
        ctx.renderer().draw_line(
            Point::new(pos.x, pos.y + 4.0),
            Point::new(pos.x + 4.0, pos.y + 8.0),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(pos.x + 4.0, pos.y + 8.0),
            Point::new(pos.x + 10.0, pos.y),
            &stroke,
        );
    }

    fn paint_spinner(&self, ctx: &mut PaintContext<'_>, x: f32, y: f32, width: f32, value: u32) {
        let rect = Rect::new(x, y + 2.0, width, self.row_height - 4.0);

        // Background
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(rect, 3.0), self.input_background);
        ctx.renderer().stroke_rounded_rect(
            RoundedRect::new(rect, 3.0),
            &Stroke::new(self.border_color, 1.0),
        );

        // Decrement button
        let dec_rect = Rect::new(x + 2.0, y + 4.0, 16.0, self.row_height - 8.0);
        ctx.renderer().fill_rect(dec_rect, self.section_color);
        let stroke = Stroke::new(self.text_color, 1.5);
        ctx.renderer().draw_line(
            Point::new(
                dec_rect.origin.x + 4.0,
                dec_rect.origin.y + dec_rect.height() / 2.0,
            ),
            Point::new(
                dec_rect.origin.x + 12.0,
                dec_rect.origin.y + dec_rect.height() / 2.0,
            ),
            &stroke,
        );

        // Increment button
        let inc_rect = Rect::new(x + width - 18.0, y + 4.0, 16.0, self.row_height - 8.0);
        ctx.renderer().fill_rect(inc_rect, self.section_color);
        ctx.renderer().draw_line(
            Point::new(
                inc_rect.origin.x + 4.0,
                inc_rect.origin.y + inc_rect.height() / 2.0,
            ),
            Point::new(
                inc_rect.origin.x + 12.0,
                inc_rect.origin.y + inc_rect.height() / 2.0,
            ),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(inc_rect.origin.x + 8.0, inc_rect.origin.y + 4.0),
            Point::new(
                inc_rect.origin.x + 8.0,
                inc_rect.origin.y + inc_rect.height() - 4.0,
            ),
            &stroke,
        );

        // Value would be rendered as text in the center
        let _ = value;
    }

    fn paint_checkbox(
        &self,
        ctx: &mut PaintContext<'_>,
        x: f32,
        y: f32,
        _label: &str,
        checked: bool,
    ) {
        let box_size = 14.0;
        let box_rect = Rect::new(
            x,
            y + (self.row_height - box_size) / 2.0,
            box_size,
            box_size,
        );

        // Box background
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(box_rect, 2.0), self.input_background);
        ctx.renderer().stroke_rounded_rect(
            RoundedRect::new(box_rect, 2.0),
            &Stroke::new(self.border_color, 1.0),
        );

        // Checkmark
        if checked {
            let stroke = Stroke::new(self.text_color, 2.0);
            ctx.renderer().draw_line(
                Point::new(box_rect.origin.x + 3.0, box_rect.origin.y + box_size / 2.0),
                Point::new(box_rect.origin.x + 6.0, box_rect.origin.y + box_size - 3.0),
                &stroke,
            );
            ctx.renderer().draw_line(
                Point::new(box_rect.origin.x + 6.0, box_rect.origin.y + box_size - 3.0),
                Point::new(box_rect.origin.x + box_size - 3.0, box_rect.origin.y + 3.0),
                &stroke,
            );
        }

        // Label text would be rendered here
    }

    fn paint_radio(
        &self,
        ctx: &mut PaintContext<'_>,
        x: f32,
        y: f32,
        _label: &str,
        selected: bool,
    ) {
        let circle_size = 14.0;
        let center_x = x + circle_size / 2.0;
        let center_y = y + self.row_height / 2.0;

        // Outer circle
        ctx.renderer().stroke_circle(
            Point::new(center_x, center_y),
            circle_size / 2.0,
            &Stroke::new(self.border_color, 1.5),
        );

        // Inner dot when selected
        if selected {
            ctx.renderer().fill_circle(
                Point::new(center_x, center_y),
                circle_size / 2.0 - 3.0,
                self.text_color,
            );
        }

        // Label text would be rendered here
    }

    fn paint_dropdown(&self, ctx: &mut PaintContext<'_>, x: f32, y: f32, width: f32, _value: &str) {
        let rect = Rect::new(x, y + 2.0, width, self.row_height - 4.0);

        // Background
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(rect, 3.0), self.input_background);
        ctx.renderer().stroke_rounded_rect(
            RoundedRect::new(rect, 3.0),
            &Stroke::new(self.border_color, 1.0),
        );

        // Dropdown arrow
        let arrow_x = rect.origin.x + rect.width() - 16.0;
        let arrow_y = rect.origin.y + rect.height() / 2.0;
        let stroke = Stroke::new(self.secondary_text_color, 1.5);
        ctx.renderer().draw_line(
            Point::new(arrow_x, arrow_y - 2.0),
            Point::new(arrow_x + 5.0, arrow_y + 2.0),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(arrow_x + 5.0, arrow_y + 2.0),
            Point::new(arrow_x + 10.0, arrow_y - 2.0),
            &stroke,
        );

        // Value text would be rendered here
    }

    /// Paint the Print button in the button box area.
    fn paint_print_button(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.dialog.widget_base().rect();
        let button_rect = Rect::new(
            rect.width() - 16.0 - 80.0 - 8.0 - 80.0,
            rect.height() - self.button_box_height() + 8.0,
            80.0,
            32.0,
        );

        // Button background
        ctx.renderer().fill_rounded_rect(
            RoundedRect::new(button_rect, 4.0),
            Color::from_rgb8(0, 120, 215),
        );

        // "Print" text would be rendered here
    }
}

impl Object for PrintDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for PrintDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::new(Size::new(500.0, 480.0)).with_minimum(Size::new(400.0, 400.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint PrintDialog-specific content
        self.paint_content(ctx);
        self.paint_print_button(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
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

impl Default for PrintDialog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PrintPreviewDialog
// ============================================================================

/// A modal dialog for previewing print output before printing.
///
/// PrintPreviewDialog provides a visual preview of how the document will
/// appear when printed. It supports:
///
/// - Page-by-page preview
/// - Zoom controls
/// - Navigation between pages
/// - Direct printing from preview
///
/// # Signals
///
/// - `paint_requested`: Emitted when the preview needs to be rendered.
///   Connect to this signal to provide your document rendering logic.
/// - `print_requested`: Emitted when the user clicks Print.
///
/// # Example
///
/// ```ignore
/// let mut preview = PrintPreviewDialog::new(&print_settings);
///
/// preview.paint_requested.connect(|printer| {
///     // Render your document to the printer
///     render_document(printer);
/// });
///
/// preview.open();
/// ```
#[allow(dead_code)]
pub struct PrintPreviewDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// Print settings.
    settings: PrintSettings,

    /// Current page being previewed.
    current_page: u32,

    /// Total pages.
    total_pages: u32,

    /// Zoom level (1.0 = 100%).
    zoom: f32,

    /// Available zoom levels.
    zoom_levels: Vec<f32>,

    /// Scroll position for the preview area.
    scroll_x: f32,
    scroll_y: f32,

    /// Whether we're dragging the preview.
    is_panning: bool,
    pan_start: Point,

    // Visual styling
    content_padding: f32,
    toolbar_height: f32,
    page_margin: f32,
    background_color: Color,
    toolbar_color: Color,
    page_color: Color,
    page_shadow_color: Color,
    border_color: Color,
    text_color: Color,

    // Signals
    /// Signal emitted when the preview needs to be painted.
    /// The signal provides a reference to the current print settings.
    pub paint_requested: Signal<PrintSettings>,

    /// Signal emitted when the user clicks Print.
    pub print_requested: Signal<PrintSettings>,
}

impl PrintPreviewDialog {
    /// Create a new print preview dialog.
    pub fn new(settings: PrintSettings) -> Self {
        let dialog = Dialog::new("Print Preview")
            .with_size(800.0, 600.0)
            .with_standard_buttons(StandardButton::CANCEL);

        Self {
            dialog,
            settings,
            current_page: 1,
            total_pages: 1,
            zoom: 1.0,
            zoom_levels: vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0],
            scroll_x: 0.0,
            scroll_y: 0.0,
            is_panning: false,
            pan_start: Point::ZERO,
            content_padding: 16.0,
            toolbar_height: 40.0,
            page_margin: 20.0,
            background_color: Color::from_rgb8(80, 80, 80),
            toolbar_color: Color::from_rgb8(240, 240, 240),
            page_color: Color::WHITE,
            page_shadow_color: Color::from_rgba8(0, 0, 0, 80),
            border_color: Color::from_rgb8(200, 200, 200),
            text_color: Color::from_rgb8(32, 32, 32),
            paint_requested: Signal::new(),
            print_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the total page count using builder pattern.
    pub fn with_total_pages(mut self, total: u32) -> Self {
        self.total_pages = total.max(1);
        self
    }

    /// Set the initial zoom level using builder pattern.
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom.clamp(0.1, 10.0);
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the current page number.
    pub fn current_page(&self) -> u32 {
        self.current_page
    }

    /// Set the current page number.
    pub fn set_current_page(&mut self, page: u32) {
        if page >= 1 && page <= self.total_pages {
            self.current_page = page;
            self.request_paint();
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the total page count.
    pub fn total_pages(&self) -> u32 {
        self.total_pages
    }

    /// Set the total page count.
    pub fn set_total_pages(&mut self, total: u32) {
        self.total_pages = total.max(1);
        if self.current_page > self.total_pages {
            self.current_page = self.total_pages;
        }
        self.dialog.widget_base_mut().update();
    }

    /// Get the current zoom level.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Set the zoom level.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 10.0);
        self.dialog.widget_base_mut().update();
    }

    /// Zoom in to the next level.
    pub fn zoom_in(&mut self) {
        if let Some(&next) = self.zoom_levels.iter().find(|&&z| z > self.zoom) {
            self.set_zoom(next);
        }
    }

    /// Zoom out to the previous level.
    pub fn zoom_out(&mut self) {
        if let Some(&prev) = self.zoom_levels.iter().rev().find(|&&z| z < self.zoom) {
            self.set_zoom(prev);
        }
    }

    /// Fit the page to the window width.
    pub fn fit_width(&mut self) {
        let preview_rect = self.preview_area_rect();
        let (page_width, _) = self.settings.paper_size.size_in_points();
        let fit_zoom = (preview_rect.width() - self.page_margin * 2.0) / page_width;
        self.set_zoom(fit_zoom);
    }

    /// Fit the entire page in the window.
    pub fn fit_page(&mut self) {
        let preview_rect = self.preview_area_rect();
        let (page_width, page_height) = self.settings.paper_size.size_in_points();

        let width_zoom = (preview_rect.width() - self.page_margin * 2.0) / page_width;
        let height_zoom = (preview_rect.height() - self.page_margin * 2.0) / page_height;

        self.set_zoom(width_zoom.min(height_zoom));
    }

    /// Navigate to the next page.
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages {
            self.set_current_page(self.current_page + 1);
        }
    }

    /// Navigate to the previous page.
    pub fn previous_page(&mut self) {
        if self.current_page > 1 {
            self.set_current_page(self.current_page - 1);
        }
    }

    /// Navigate to the first page.
    pub fn first_page(&mut self) {
        self.set_current_page(1);
    }

    /// Navigate to the last page.
    pub fn last_page(&mut self) {
        self.set_current_page(self.total_pages);
    }

    /// Get the print settings.
    pub fn settings(&self) -> &PrintSettings {
        &self.settings
    }

    /// Request a repaint of the preview.
    fn request_paint(&self) {
        self.paint_requested.emit(self.settings.clone());
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the preview dialog.
    pub fn open(&mut self) {
        self.request_paint();
        self.dialog.open();
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

    /// Accept and print.
    fn print(&mut self) {
        self.print_requested.emit(self.settings.clone());
        self.dialog.accept();
    }

    /// Reject/cancel the dialog.
    fn reject(&mut self) {
        self.dialog.reject();
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Get the title bar height.
    fn title_bar_height(&self) -> f32 {
        28.0
    }

    /// Get the button box height.
    fn button_box_height(&self) -> f32 {
        48.0
    }

    /// Get the toolbar rectangle.
    fn toolbar_rect(&self) -> Rect {
        let rect = self.dialog.widget_base().rect();
        Rect::new(
            0.0,
            self.title_bar_height(),
            rect.width(),
            self.toolbar_height,
        )
    }

    /// Get the preview area rectangle.
    fn preview_area_rect(&self) -> Rect {
        let rect = self.dialog.widget_base().rect();
        Rect::new(
            0.0,
            self.title_bar_height() + self.toolbar_height,
            rect.width(),
            rect.height()
                - self.title_bar_height()
                - self.toolbar_height
                - self.button_box_height(),
        )
    }

    /// Get the page rectangle at current zoom level.
    fn page_rect(&self) -> Rect {
        let preview = self.preview_area_rect();
        let (page_width, page_height) = self.settings.paper_size.size_in_points();

        // Apply orientation
        let (w, h) = if self.settings.orientation == PageOrientation::Landscape {
            (page_height, page_width)
        } else {
            (page_width, page_height)
        };

        let scaled_width = w * self.zoom;
        let scaled_height = h * self.zoom;

        // Center in preview area
        let x = preview.origin.x + (preview.width() - scaled_width) / 2.0 - self.scroll_x;
        let y = preview.origin.y + (preview.height() - scaled_height) / 2.0 - self.scroll_y;

        Rect::new(x, y, scaled_width, scaled_height)
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let toolbar = self.toolbar_rect();
        let preview = self.preview_area_rect();

        // Check toolbar button clicks
        if toolbar.contains(pos) {
            let button_width = 32.0;
            let spacing = 8.0;
            let mut x = toolbar.origin.x + spacing;

            // First/Previous/Next/Last buttons
            let first_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if first_rect.contains(pos) {
                self.first_page();
                return true;
            }
            x += button_width + spacing;

            let prev_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if prev_rect.contains(pos) {
                self.previous_page();
                return true;
            }
            x += button_width + 60.0 + spacing; // Skip page indicator

            let next_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if next_rect.contains(pos) {
                self.next_page();
                return true;
            }
            x += button_width + spacing;

            let last_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if last_rect.contains(pos) {
                self.last_page();
                return true;
            }
            x += button_width + spacing * 2.0;

            // Zoom buttons
            let zoom_out_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if zoom_out_rect.contains(pos) {
                self.zoom_out();
                return true;
            }
            x += button_width + 50.0 + spacing; // Skip zoom indicator

            let zoom_in_rect = Rect::new(x, toolbar.origin.y + 4.0, button_width, button_width);
            if zoom_in_rect.contains(pos) {
                self.zoom_in();
                return true;
            }

            return true;
        }

        // Start panning in preview area
        if preview.contains(pos) {
            self.is_panning = true;
            self.pan_start = pos;
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, _event: &crate::widget::MouseReleaseEvent) -> bool {
        if self.is_panning {
            self.is_panning = false;
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if self.is_panning {
            let delta_x = event.local_pos.x - self.pan_start.x;
            let delta_y = event.local_pos.y - self.pan_start.y;
            self.scroll_x -= delta_x;
            self.scroll_y -= delta_y;
            self.pan_start = event.local_pos;
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let preview = self.preview_area_rect();
        if preview.contains(event.local_pos) {
            if event.modifiers.control {
                // Zoom with Ctrl+wheel
                if event.delta_y > 0.0 {
                    self.zoom_in();
                } else {
                    self.zoom_out();
                }
            } else {
                // Scroll
                self.scroll_y -= event.delta_y * 3.0;
            }
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Enter if event.modifiers.control => {
                self.print();
                return true;
            }
            Key::Escape => {
                self.reject();
                return true;
            }
            Key::PageUp | Key::ArrowLeft => {
                self.previous_page();
                return true;
            }
            Key::PageDown | Key::ArrowRight => {
                self.next_page();
                return true;
            }
            Key::Home => {
                self.first_page();
                return true;
            }
            Key::End => {
                self.last_page();
                return true;
            }
            _ => {}
        }

        // Zoom shortcuts
        if event.modifiers.control {
            match event.key {
                Key::Equal | Key::NumpadAdd => {
                    self.zoom_in();
                    return true;
                }
                Key::Minus | Key::NumpadSubtract => {
                    self.zoom_out();
                    return true;
                }
                Key::Digit0 | Key::Numpad0 => {
                    self.set_zoom(1.0);
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_content(&self, ctx: &mut PaintContext<'_>) {
        self.paint_toolbar(ctx);
        self.paint_preview_area(ctx);
        self.paint_print_button(ctx);
    }

    fn paint_toolbar(&self, ctx: &mut PaintContext<'_>) {
        let toolbar = self.toolbar_rect();

        // Background
        ctx.renderer().fill_rect(toolbar, self.toolbar_color);

        // Bottom border
        ctx.renderer().draw_line(
            Point::new(toolbar.origin.x, toolbar.origin.y + toolbar.height()),
            Point::new(
                toolbar.origin.x + toolbar.width(),
                toolbar.origin.y + toolbar.height(),
            ),
            &Stroke::new(self.border_color, 1.0),
        );

        let button_size = 32.0;
        let spacing = 8.0;
        let mut x = toolbar.origin.x + spacing;
        let button_y = toolbar.origin.y + (toolbar.height() - button_size) / 2.0;

        // Navigation buttons
        self.paint_nav_button(ctx, x, button_y, button_size, "|<", self.current_page > 1);
        x += button_size + spacing;

        self.paint_nav_button(ctx, x, button_y, button_size, "<", self.current_page > 1);
        x += button_size + spacing;

        // Page indicator space (would render text)
        x += 60.0 + spacing;

        self.paint_nav_button(
            ctx,
            x,
            button_y,
            button_size,
            ">",
            self.current_page < self.total_pages,
        );
        x += button_size + spacing;

        self.paint_nav_button(
            ctx,
            x,
            button_y,
            button_size,
            ">|",
            self.current_page < self.total_pages,
        );
        x += button_size + spacing * 2.0;

        // Separator
        ctx.renderer().draw_line(
            Point::new(x, toolbar.origin.y + 8.0),
            Point::new(x, toolbar.origin.y + toolbar.height() - 8.0),
            &Stroke::new(self.border_color, 1.0),
        );
        x += spacing * 2.0;

        // Zoom buttons
        self.paint_nav_button(ctx, x, button_y, button_size, "-", self.zoom > 0.25);
        x += button_size + spacing;

        // Zoom indicator space
        x += 50.0 + spacing;

        self.paint_nav_button(ctx, x, button_y, button_size, "+", self.zoom < 4.0);
    }

    fn paint_nav_button(
        &self,
        ctx: &mut PaintContext<'_>,
        x: f32,
        y: f32,
        size: f32,
        _icon: &str,
        enabled: bool,
    ) {
        let rect = Rect::new(x, y, size, size);
        let color = if enabled {
            self.text_color
        } else {
            Color::from_rgb8(180, 180, 180)
        };

        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(rect, 4.0), Color::from_rgba8(0, 0, 0, 10));

        // Icon text would be rendered here
        let _ = color;
    }

    fn paint_preview_area(&self, ctx: &mut PaintContext<'_>) {
        let preview = self.preview_area_rect();

        // Dark background
        ctx.renderer().fill_rect(preview, self.background_color);

        // Draw page
        let page = self.page_rect();

        // Page shadow
        let shadow_offset = 4.0;
        let shadow_rect = Rect::new(
            page.origin.x + shadow_offset,
            page.origin.y + shadow_offset,
            page.width(),
            page.height(),
        );
        ctx.renderer()
            .fill_rect(shadow_rect, self.page_shadow_color);

        // Page
        ctx.renderer().fill_rect(page, self.page_color);
        ctx.renderer()
            .stroke_rect(page, &Stroke::new(self.border_color, 1.0));

        // Page content would be rendered by the paint_requested signal handler
    }

    fn paint_print_button(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.dialog.widget_base().rect();
        let button_rect = Rect::new(
            rect.width() - 16.0 - 80.0 - 8.0 - 80.0,
            rect.height() - self.button_box_height() + 8.0,
            80.0,
            32.0,
        );

        ctx.renderer().fill_rounded_rect(
            RoundedRect::new(button_rect, 4.0),
            Color::from_rgb8(0, 120, 215),
        );

        // "Print" text would be rendered here
    }
}

impl Object for PrintPreviewDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for PrintPreviewDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::new(Size::new(800.0, 600.0)).with_minimum(Size::new(600.0, 400.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        self.paint_content(ctx);
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

impl Default for PrintPreviewDialog {
    fn default() -> Self {
        Self::new(PrintSettings::default())
    }
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

    // =========================================================================
    // PageRange Tests
    // =========================================================================

    #[test]
    fn test_page_range_from_string() {
        // Single page
        assert_eq!(PageRange::from_string("5"), Some(PageRange::Pages(vec![5])));

        // Range
        assert_eq!(
            PageRange::from_string("1-5"),
            Some(PageRange::Pages(vec![1, 2, 3, 4, 5]))
        );

        // Multiple pages
        assert_eq!(
            PageRange::from_string("1,3,5"),
            Some(PageRange::Pages(vec![1, 3, 5]))
        );

        // Mixed
        assert_eq!(
            PageRange::from_string("1,3-5,7"),
            Some(PageRange::Pages(vec![1, 3, 4, 5, 7]))
        );

        // Invalid
        assert_eq!(PageRange::from_string(""), None);
        assert_eq!(PageRange::from_string("abc"), None);
        assert_eq!(PageRange::from_string("0"), None);
    }

    #[test]
    fn test_page_range_display_string() {
        assert_eq!(PageRange::All.display_string(), "All");
        assert_eq!(PageRange::CurrentPage.display_string(), "Current Page");
        assert_eq!(PageRange::Selection.display_string(), "Selection");
        assert_eq!(PageRange::Range { from: 1, to: 5 }.display_string(), "1-5");
        assert_eq!(
            PageRange::Pages(vec![1, 2, 3, 5, 7, 8, 9]).display_string(),
            "1-3, 5, 7-9"
        );
    }

    // =========================================================================
    // Paper Size Tests
    // =========================================================================

    #[test]
    fn test_paper_size_points() {
        let (w, h) = PaperSize::Letter.size_in_points();
        assert!((w - 612.0).abs() < 1.0);
        assert!((h - 792.0).abs() < 1.0);

        let (w, h) = PaperSize::A4.size_in_points();
        assert!((w - 595.0).abs() < 1.0);
        assert!((h - 842.0).abs() < 1.0);
    }

    #[test]
    fn test_paper_size_all() {
        let sizes = PaperSize::all();
        assert!(!sizes.is_empty());
        assert!(sizes.contains(&PaperSize::Letter));
        assert!(sizes.contains(&PaperSize::A4));
    }

    // =========================================================================
    // Print Settings Tests
    // =========================================================================

    #[test]
    fn test_print_settings_default() {
        let settings = PrintSettings::default();
        assert_eq!(settings.copies, 1);
        assert!(settings.collate);
        assert_eq!(settings.page_range, PageRange::All);
        assert_eq!(settings.orientation, PageOrientation::Portrait);
        assert_eq!(settings.paper_size, PaperSize::Letter);
        assert_eq!(settings.color_mode, ColorMode::Color);
        assert_eq!(settings.duplex, DuplexMode::None);
        assert!(!settings.print_to_file);
    }

    #[test]
    fn test_print_settings_for_printer() {
        let settings = PrintSettings::for_printer("my_printer");
        assert_eq!(settings.printer_id, "my_printer");
    }

    // =========================================================================
    // PrintDialogOptions Tests
    // =========================================================================

    #[test]
    fn test_print_dialog_options() {
        let options = PrintDialogOptions::PRINT_PAGE_RANGE | PrintDialogOptions::PRINT_SELECTION;
        assert!(options.contains(PrintDialogOptions::PRINT_PAGE_RANGE));
        assert!(options.contains(PrintDialogOptions::PRINT_SELECTION));
        assert!(!options.contains(PrintDialogOptions::PRINT_TO_FILE));
    }

    // =========================================================================
    // PrinterInfo Tests
    // =========================================================================

    #[test]
    fn test_printer_info() {
        let printer = PrinterInfo::new("test", "Test Printer")
            .with_description("A test printer")
            .with_default(true)
            .with_color_support(true)
            .with_duplex_support(true);

        assert_eq!(printer.id, "test");
        assert_eq!(printer.name, "Test Printer");
        assert_eq!(printer.description, "A test printer");
        assert!(printer.is_default);
        assert!(printer.supports_color);
        assert!(printer.supports_duplex);
    }

    // =========================================================================
    // PrintDialog Tests
    // =========================================================================

    #[test]
    fn test_print_dialog_creation() {
        setup();
        let dialog = PrintDialog::new();

        assert!(!dialog.is_open());
        assert_eq!(dialog.copies(), 1);
        assert!(dialog.collate());
        assert_eq!(*dialog.page_range(), PageRange::All);
    }

    #[test]
    fn test_print_dialog_builder() {
        setup();
        let settings = PrintSettings {
            copies: 3,
            collate: false,
            page_range: PageRange::Range { from: 1, to: 5 },
            ..Default::default()
        };

        let dialog = PrintDialog::new()
            .with_title("My Print Dialog")
            .with_settings(settings.clone())
            .with_current_page(2)
            .with_total_pages(10);

        assert_eq!(dialog.copies(), 3);
        assert!(!dialog.collate());
    }

    #[test]
    fn test_print_dialog_properties() {
        setup();
        let mut dialog = PrintDialog::new();

        dialog.set_copies(5);
        assert_eq!(dialog.copies(), 5);

        dialog.set_collate(false);
        assert!(!dialog.collate());

        dialog.set_orientation(PageOrientation::Landscape);
        assert_eq!(dialog.orientation(), PageOrientation::Landscape);

        dialog.set_paper_size(PaperSize::A4);
        assert_eq!(dialog.paper_size(), PaperSize::A4);

        dialog.set_color_mode(ColorMode::Grayscale);
        assert_eq!(dialog.color_mode(), ColorMode::Grayscale);

        dialog.set_duplex(DuplexMode::LongEdge);
        assert_eq!(dialog.duplex(), DuplexMode::LongEdge);
    }

    // =========================================================================
    // PrintPreviewDialog Tests
    // =========================================================================

    #[test]
    fn test_print_preview_creation() {
        setup();
        let preview = PrintPreviewDialog::new(PrintSettings::default());

        assert!(!preview.is_open());
        assert_eq!(preview.current_page(), 1);
        assert_eq!(preview.total_pages(), 1);
        assert!((preview.zoom() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_print_preview_navigation() {
        setup();
        let mut preview = PrintPreviewDialog::new(PrintSettings::default()).with_total_pages(10);

        assert_eq!(preview.current_page(), 1);

        preview.next_page();
        assert_eq!(preview.current_page(), 2);

        preview.last_page();
        assert_eq!(preview.current_page(), 10);

        preview.previous_page();
        assert_eq!(preview.current_page(), 9);

        preview.first_page();
        assert_eq!(preview.current_page(), 1);
    }

    #[test]
    fn test_print_preview_zoom() {
        setup();
        let mut preview = PrintPreviewDialog::new(PrintSettings::default());

        preview.set_zoom(2.0);
        assert!((preview.zoom() - 2.0).abs() < 0.001);

        preview.zoom_in();
        assert!(preview.zoom() > 2.0);

        preview.set_zoom(0.5);
        preview.zoom_out();
        assert!(preview.zoom() < 0.5);
    }

    #[test]
    fn test_print_preview_zoom_clamp() {
        setup();
        let mut preview = PrintPreviewDialog::new(PrintSettings::default());

        preview.set_zoom(100.0);
        assert!(preview.zoom() <= 10.0);

        preview.set_zoom(0.001);
        assert!(preview.zoom() >= 0.1);
    }
}
