//! Standard widgets for Horizon Lattice.
//!
//! This module provides common UI widgets:
//!
//! - [`Label`]: Text display widget
//! - [`PushButton`]: Standard clickable button
//! - [`ToolButton`]: Icon-focused button for toolbars with menu support
//! - [`ButtonVariant`]: Visual variants for buttons (Primary, Secondary, Danger, Flat, Outlined)
//! - [`CheckBox`]: Checkbox for boolean/tri-state selection
//! - [`RadioButton`]: Radio button for exclusive selection
//! - [`RadioGroup`]: Visual container for radio buttons with automatic exclusivity
//! - [`ButtonGroup`]: Non-visual container for exclusive button groups
//! - [`AbstractButton`]: Base for all button widgets
//! - [`LineEdit`]: Single-line text input
//! - [`TextEdit`]: Multi-line text editor with scrolling and word wrap
//! - [`PlainTextEdit`]: Plain text editor optimized for large documents with syntax highlighting
//! - [`SpinBox`]: Integer input with increment/decrement buttons
//! - [`DoubleSpinBox`]: Floating-point input with increment/decrement buttons
//! - [`ProgressBar`]: Progress indicator widget
//! - [`ImageWidget`]: Image display widget with animation and scaling support
//! - [`ContainerWidget`]: Generic container widget with layout support
//! - [`Frame`]: Container widget with border decoration
//! - [`GroupBox`]: Titled container with optional checkbox mode
//! - [`Separator`]: Visual dividing line
//! - [`Spacer`]: Invisible widget for layout spacing
//! - [`ScrollBar`]: Standalone scrollbar widget
//! - [`ScrollArea`]: Scrollable container widget
//! - [`Splitter`]: Resizable pane container
//! - [`TabBar`]: Standalone tab bar widget
//! - [`TabWidget`]: Tabbed page container
//! - [`StackedWidget`]: Container showing one child at a time
//! - [`ToolBox`]: Accordion-style container with expandable pages
//! - [`DockWidget`]: Dockable panel widget
//! - [`MainWindow`]: Main application window with dock areas
//! - [`Popup`]: Temporary floating container widget
//! - [`Window`]: Top-level window widget
//! - [`Dialog`]: Modal dialog with accept/reject semantics
//! - [`DialogButtonBox`]: Container for standard dialog buttons
//! - [`MessageBox`]: Modal dialog for displaying messages with icons
//! - [`ColorButton`]: Button that displays a color swatch
//! - [`ColorPicker`]: Inline HSV color picker with saturation/value square and hue bar
//! - [`ColorDialog`]: Modal dialog for color selection with HSV picker and palettes
//! - [`FontDialog`]: Modal dialog for font selection with family, style, size, and preview
//! - [`InputDialog`]: Modal dialog for simple input (text, numbers, item selection)
//! - [`ProgressDialog`]: Modal dialog showing operation progress with cancel option
//! - [`AboutDialog`]: Modal dialog for displaying application information
//! - [`PrintDialog`]: Modal dialog for configuring print settings
//! - [`PrintPreviewDialog`]: Modal dialog for previewing print output before printing
//! - [`FontComboBox`]: Dropdown for selecting font families with preview
//! - [`KeySequenceEdit`]: Keyboard shortcut capture and editing widget
//! - [`StatusBar`]: Status bar with temporary messages and permanent widgets
//! - [`SystemTrayIcon`]: System tray (notification area) icon with context menu support
//! - [`TrayMenu`]: Context menu adapter for system tray icons
//! - [`TrayIconImage`]: Image wrapper for tray icons
//! - [`ActivationReason`]: Enum indicating how a tray icon was activated

mod about_dialog;
mod abstract_button;
mod action;
mod button_group;
mod calendar;
mod checkbox;
mod color_button;
mod color_dialog;
mod color_palette_popup;
mod color_picker;
mod combo_box;
mod container;
mod date_edit;
mod date_time_edit;
mod dial;
mod dialog;
mod dialog_button_box;
mod dock_widget;
mod double_spin_box;
mod file_dialog;
mod find_replace;
mod font_combo_box;
mod font_dialog;
mod frame;
mod group_box;
mod header_view;
mod image_widget;
mod input_dialog;
mod key_sequence_edit;
mod label;
mod line_edit;
mod list_view;
mod list_widget;
mod main_window;
mod menu;
mod menu_bar;
mod message_box;
pub mod native_dialogs;
mod plain_text_edit;
mod popup;
mod print_dialog;
mod progress_bar;
mod progress_dialog;
mod push_button;
mod radio_button;
mod radio_group;
mod recent_colors_palette;
mod scroll_area;
mod scroll_bar;
mod separator;
mod slider;
mod spacer;
mod spin_box;
mod splitter;
mod stacked_widget;
mod status_bar;
mod styled_document;
mod system_tray;
mod tab_bar;
mod tab_widget;
mod table_view;
mod table_widget;
mod text_edit;
mod text_edit_toolbar;
mod time_edit;
mod timezone;
mod tool_bar;
mod tool_box;
mod tool_button;
mod tree_view;
mod tree_widget;
mod window;
mod wizard;

pub use abstract_button::{AbstractButton, ButtonVariant};
pub use action::{Action, ActionGroup, ActionPriority, MenuRole, ShortcutContext};
pub use button_group::ButtonGroup;
pub use calendar::{
    CalendarWidget, CompositeDayFormatter, DateRangeHighlightFormatter, DayCellInfo, DayFormat,
    DayFormatter, DefaultDayFormatter, WeekendHighlightFormatter,
};
pub use checkbox::{CheckBox, CheckState};
pub use color_button::{ColorButton, ColorButtonPopupMode};
pub use color_dialog::ColorDialog;
pub use color_palette_popup::ColorPalettePopup;
pub use color_picker::ColorPicker;
pub use combo_box::{
    ComboBox, ComboBoxItem, ComboBoxItemDelegate, ComboBoxModel, DefaultComboBoxDelegate,
    IconListComboModel, StringListComboModel,
};
pub use container::ContainerWidget;
pub use date_edit::{DateEdit, DateFormat};
pub use date_time_edit::{DateTimeEdit, TimezoneDisplay};
pub use dial::Dial;
pub use dock_widget::{DockArea, DockAreas, DockWidget, DockWidgetFeatures};
pub use frame::{Frame, FrameShadow, FrameShape};
pub use group_box::GroupBox;
pub use header_view::{HeaderView, ResizeMode, SortOrder};
pub use image_widget::{ImageSource, ImageWidget, ImageWidgetState};
pub use label::{ElideMode, Label};
pub use line_edit::{EchoMode, LineEdit};
pub use list_view::{Flow, ListView, ListViewMode};
pub use recent_colors_palette::RecentColorsPalette;
pub use timezone::{
    COMMON_TIMEZONES, TimezoneComboModel, TimezoneDisplayFormat, format_timezone,
    format_utc_offset, get_timezone_abbreviation, get_utc_offset_seconds, local_timezone,
};
// Re-export validation types for convenience
pub use super::validator::{
    CustomValidator, DoubleValidator, HexColorValidator, HexFormat, IntValidator, RegexValidator,
    ValidationState, Validator,
};
// Re-export completer types for convenience
pub use super::completer::{CaseSensitivity, Completer, CompleterModel, StringListModel};
pub use main_window::MainWindow;
pub use plain_text_edit::{HighlightSpan, LineNumberConfig, PlainTextEdit, SyntaxHighlighter};
pub use popup::{Popup, PopupFlags, PopupPlacement};
pub use progress_bar::{Orientation, ProgressBar};
pub use push_button::PushButton;
pub use radio_button::RadioButton;
pub use radio_group::RadioGroup;
pub use scroll_area::{ScrollArea, ScrollBarPolicy};
pub use scroll_bar::ScrollBar;
pub use separator::{Separator, SeparatorOrientation};
pub use slider::{Slider, TickPosition};
pub use spacer::Spacer;
pub use splitter::Splitter;
pub use stacked_widget::StackedWidget;
pub use styled_document::{
    BlockFormat, BlockRun, CharFormat, FormatRun, LineSpacing, ListFormat, ListStyle,
    StyledDocument,
};
pub use tab_bar::{TabBar, TabPosition};
pub use tab_widget::TabWidget;
pub use table_view::{GridStyle, TableContextMenuLocation, TableView};
pub use text_edit::{TextEdit, TextWrapMode};
pub use tree_view::{IndentationStyle, TreeView};
// Re-export font and text types for rich text formatting
pub use about_dialog::AboutDialog;
pub use dialog::{Dialog, DialogResult};
pub use dialog_button_box::{
    ButtonBoxOrientation, ButtonOrder, ButtonRole, DialogButtonBox, StandardButton,
};
pub use double_spin_box::{DoubleSpinBox, NotationMode, NotationStyle};
pub use file_dialog::{
    BookmarkEntry, BookmarkIcon, FileDialog, FileDialogMode, FileEntry, FileFilter, FileViewMode,
    native_dialog_available,
};
pub use find_replace::{FindOptions, FindReplaceBar, FindReplaceMode, SearchMatch, Searchable};
pub use font_combo_box::{FontComboBox, FontFilter};
pub use font_dialog::{FontDialog, FontDialogOptions};
pub use horizon_lattice_render::text::{FontFamily, FontWeight, HorizontalAlign};
pub use input_dialog::{InputDialog, InputEchoMode, InputMode};
pub use key_sequence_edit::KeySequenceEdit;
pub use list_widget::{ListWidget, ListWidgetItem, MatchFlags};
pub use menu::{Menu, MenuItem, MenuStyle};
pub use menu_bar::{MenuBar, MenuBarStyle};
pub use message_box::{CustomButtonInfo, MessageBox, MessageIcon};
pub use print_dialog::{
    ColorMode, DuplexMode, PageOrientation, PageRange, PaperSize, PrintDialog, PrintDialogOptions,
    PrintPreviewDialog, PrintSettings, PrinterInfo,
};
pub use progress_dialog::ProgressDialog;
pub use spin_box::SpinBox;
pub use status_bar::{MessagePriority, StatusBar, StatusBarStyle};
pub use system_tray::{ActivationReason, SystemTrayIcon, TrayError, TrayIconImage, TrayMenu};
pub use table_widget::{TableWidget, TableWidgetItem};
pub use text_edit_toolbar::{
    ColorWidgets, FontWidgets, FormatActions, ParagraphActions, TextEditToolbar,
};
pub use time_edit::{TimeEdit, TimeFormat};
pub use tool_bar::{
    ToolBar, ToolBarArea, ToolBarAreas, ToolBarFeatures, ToolBarItem, ToolBarStyle,
};
pub use tool_box::ToolBox;
pub use tool_button::{ToolButton, ToolButtonPopupMode, ToolButtonStyle};
pub use tree_widget::{TreeIndentationStyle, TreeWidget, TreeWidgetItem};
pub use window::{Window, WindowFlags, WindowModality, WindowState};
pub use wizard::{
    PageCondition, PageValidator, ValidationError, ValidationResult, Wizard, WizardButton,
    WizardPage, WizardStyle,
};
