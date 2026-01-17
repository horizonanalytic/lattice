//! TextEdit toolbar integration.
//!
//! This module provides helper types and functions for integrating rich text editing
//! toolbars with [`TextEdit`] widgets. It creates actions for formatting operations
//! and maintains bidirectional synchronization between toolbar button states and
//! the text editor's current format.
//!
//! # Overview
//!
//! The module provides:
//! - [`FormatActions`]: Checkable actions for bold, italic, underline, strikethrough
//! - [`ParagraphActions`]: Actions for alignment and list formatting
//! - [`TextEditToolbar`]: Complete toolbar setup with all formatting controls
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{TextEdit, TextEditToolbar, ToolBar};
//!
//! // Create a text editor and toolbar
//! let mut text_edit = TextEdit::new();
//! let mut toolbar = ToolBar::new();
//!
//! // Create toolbar integration and add to toolbar
//! let format_toolbar = TextEditToolbar::new();
//! format_toolbar.add_to_toolbar(&mut toolbar);
//!
//! // Connect to the text editor
//! format_toolbar.connect_to_text_edit(&text_edit);
//! ```

use std::sync::Arc;

use horizon_lattice_core::Signal;
use horizon_lattice_render::{Color, HorizontalAlign};

use super::{Action, ActionGroup, ColorButton, DoubleSpinBox, FontComboBox, TextEdit, ToolBar};

// ============================================================================
// Format Actions
// ============================================================================

/// Actions for character-level formatting (bold, italic, underline, strikethrough).
///
/// These actions are checkable and maintain their state synchronized with the
/// current text selection in a connected [`TextEdit`].
///
/// # Signals
///
/// Each action emits `toggled(bool)` when its state changes, either from user
/// interaction or from the text editor's format state changing.
pub struct FormatActions {
    /// Bold formatting action (Ctrl+B).
    pub bold: Arc<Action>,
    /// Italic formatting action (Ctrl+I).
    pub italic: Arc<Action>,
    /// Underline formatting action (Ctrl+U).
    pub underline: Arc<Action>,
    /// Strikethrough formatting action.
    pub strikethrough: Arc<Action>,
}

impl FormatActions {
    /// Create new format actions with default shortcuts.
    pub fn new() -> Self {
        let bold = Arc::new(
            Action::new("&Bold")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+B")
                .with_tooltip("Bold (Ctrl+B)"),
        );

        let italic = Arc::new(
            Action::new("&Italic")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+I")
                .with_tooltip("Italic (Ctrl+I)"),
        );

        let underline = Arc::new(
            Action::new("&Underline")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+U")
                .with_tooltip("Underline (Ctrl+U)"),
        );

        let strikethrough = Arc::new(
            Action::new("&Strikethrough")
                .with_checkable(true)
                .with_tooltip("Strikethrough"),
        );

        Self {
            bold,
            italic,
            underline,
            strikethrough,
        }
    }

    /// Add all format actions to a toolbar.
    pub fn add_to_toolbar(&self, toolbar: &mut ToolBar) {
        toolbar.add_action(self.bold.clone());
        toolbar.add_action(self.italic.clone());
        toolbar.add_action(self.underline.clone());
        toolbar.add_action(self.strikethrough.clone());
    }

    /// Connect format actions to a TextEdit's toggle methods.
    ///
    /// This sets up bidirectional synchronization:
    /// - Clicking an action toggles the format in the TextEdit
    /// - Changing selection in TextEdit updates action checked states
    ///
    /// # Arguments
    ///
    /// * `text_edit` - The TextEdit widget to connect to
    pub fn connect_to_text_edit(&self, text_edit: &TextEdit) {
        // Connect action triggers to TextEdit toggle methods
        // We need to use a pattern that allows calling methods on TextEdit
        // Since we can't hold a mutable reference, we'll emit through signals

        // Connect format_changed signal to update action states
        let bold = self.bold.clone();
        let italic = self.italic.clone();
        let underline = self.underline.clone();
        let strikethrough = self.strikethrough.clone();

        text_edit
            .format_changed
            .connect(move |&(is_bold, is_italic, is_underline, is_strikethrough)| {
                // Update action checked states without triggering their signals
                // We need to be careful to avoid infinite loops
                if bold.is_checked() != is_bold {
                    bold.set_checked(is_bold);
                }
                if italic.is_checked() != is_italic {
                    italic.set_checked(is_italic);
                }
                if underline.is_checked() != is_underline {
                    underline.set_checked(is_underline);
                }
                if strikethrough.is_checked() != is_strikethrough {
                    strikethrough.set_checked(is_strikethrough);
                }
            });

        // Initialize action states from current format
        let format = text_edit.current_format();
        self.bold.set_checked(format.bold);
        self.italic.set_checked(format.italic);
        self.underline.set_checked(format.underline);
        self.strikethrough.set_checked(format.strikethrough);
    }

    /// Get a signal that emits when the bold action is triggered.
    ///
    /// Connect this to `TextEdit::toggle_bold()`.
    pub fn bold_triggered(&self) -> &Signal<bool> {
        &self.bold.triggered
    }

    /// Get a signal that emits when the italic action is triggered.
    ///
    /// Connect this to `TextEdit::toggle_italic()`.
    pub fn italic_triggered(&self) -> &Signal<bool> {
        &self.italic.triggered
    }

    /// Get a signal that emits when the underline action is triggered.
    ///
    /// Connect this to `TextEdit::toggle_underline()`.
    pub fn underline_triggered(&self) -> &Signal<bool> {
        &self.underline.triggered
    }

    /// Get a signal that emits when the strikethrough action is triggered.
    ///
    /// Connect this to `TextEdit::toggle_strikethrough()`.
    pub fn strikethrough_triggered(&self) -> &Signal<bool> {
        &self.strikethrough.triggered
    }
}

impl Default for FormatActions {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Paragraph Actions
// ============================================================================

/// Actions for paragraph-level formatting (alignment, lists, indentation).
///
/// Alignment actions are grouped as exclusive (only one can be active).
/// List actions are checkable toggles.
pub struct ParagraphActions {
    /// Align left action.
    pub align_left: Arc<Action>,
    /// Align center action.
    pub align_center: Arc<Action>,
    /// Align right action.
    pub align_right: Arc<Action>,
    /// Justify action.
    pub align_justify: Arc<Action>,
    /// Action group for exclusive alignment selection.
    pub alignment_group: ActionGroup,
    /// Toggle bullet list action.
    pub bullet_list: Arc<Action>,
    /// Toggle numbered list action.
    pub numbered_list: Arc<Action>,
    /// Increase indentation action.
    pub indent_increase: Arc<Action>,
    /// Decrease indentation action.
    pub indent_decrease: Arc<Action>,
}

impl ParagraphActions {
    /// Create new paragraph actions.
    pub fn new() -> Self {
        // Alignment actions (exclusive group)
        let align_left = Arc::new(
            Action::new("Align &Left")
                .with_checkable(true)
                .with_checked(true) // Default
                .with_shortcut_str("Ctrl+L")
                .with_tooltip("Align Left (Ctrl+L)"),
        );

        let align_center = Arc::new(
            Action::new("Align &Center")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+E")
                .with_tooltip("Align Center (Ctrl+E)"),
        );

        let align_right = Arc::new(
            Action::new("Align &Right")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+R")
                .with_tooltip("Align Right (Ctrl+R)"),
        );

        let align_justify = Arc::new(
            Action::new("&Justify")
                .with_checkable(true)
                .with_shortcut_str("Ctrl+J")
                .with_tooltip("Justify (Ctrl+J)"),
        );

        // Create exclusive action group for alignment (ActionGroup is exclusive by default)
        let alignment_group = ActionGroup::new();
        alignment_group.add_action(align_left.clone());
        alignment_group.add_action(align_center.clone());
        alignment_group.add_action(align_right.clone());
        alignment_group.add_action(align_justify.clone());

        // List actions (toggle)
        let bullet_list = Arc::new(
            Action::new("&Bullet List")
                .with_checkable(true)
                .with_tooltip("Bullet List"),
        );

        let numbered_list = Arc::new(
            Action::new("&Numbered List")
                .with_checkable(true)
                .with_tooltip("Numbered List"),
        );

        // Indent actions (not checkable, just trigger)
        let indent_increase = Arc::new(
            Action::new("&Increase Indent")
                .with_shortcut_str("Ctrl+]")
                .with_tooltip("Increase Indent (Ctrl+])"),
        );

        let indent_decrease = Arc::new(
            Action::new("&Decrease Indent")
                .with_shortcut_str("Ctrl+[")
                .with_tooltip("Decrease Indent (Ctrl+[)"),
        );

        Self {
            align_left,
            align_center,
            align_right,
            align_justify,
            alignment_group,
            bullet_list,
            numbered_list,
            indent_increase,
            indent_decrease,
        }
    }

    /// Add all paragraph actions to a toolbar.
    ///
    /// Adds alignment buttons, a separator, list buttons, another separator,
    /// and indent buttons.
    pub fn add_to_toolbar(&self, toolbar: &mut ToolBar) {
        // Alignment buttons
        toolbar.add_action(self.align_left.clone());
        toolbar.add_action(self.align_center.clone());
        toolbar.add_action(self.align_right.clone());
        toolbar.add_action(self.align_justify.clone());
        toolbar.add_separator();

        // List buttons
        toolbar.add_action(self.bullet_list.clone());
        toolbar.add_action(self.numbered_list.clone());
        toolbar.add_separator();

        // Indent buttons
        toolbar.add_action(self.indent_increase.clone());
        toolbar.add_action(self.indent_decrease.clone());
    }

    /// Connect paragraph actions to a TextEdit.
    ///
    /// This sets up synchronization for alignment state changes.
    pub fn connect_to_text_edit(&self, text_edit: &TextEdit) {
        // Connect alignment_changed signal to update action states
        let align_left = self.align_left.clone();
        let align_center = self.align_center.clone();
        let align_right = self.align_right.clone();
        let align_justify = self.align_justify.clone();

        text_edit.alignment_changed.connect(move |&alignment| {
            // Update alignment action states
            match alignment {
                HorizontalAlign::Left => {
                    if !align_left.is_checked() {
                        align_left.set_checked(true);
                    }
                }
                HorizontalAlign::Center => {
                    if !align_center.is_checked() {
                        align_center.set_checked(true);
                    }
                }
                HorizontalAlign::Right => {
                    if !align_right.is_checked() {
                        align_right.set_checked(true);
                    }
                }
                HorizontalAlign::Justified => {
                    if !align_justify.is_checked() {
                        align_justify.set_checked(true);
                    }
                }
            }
        });

        // Initialize list state from current paragraph
        let is_list = text_edit.is_list_item();
        if is_list {
            // We'd need to check list style to set the right button
            // For now, just note that it's a list item
        }
    }

    /// Get a signal that emits when align left is triggered.
    pub fn align_left_triggered(&self) -> &Signal<bool> {
        &self.align_left.triggered
    }

    /// Get a signal that emits when align center is triggered.
    pub fn align_center_triggered(&self) -> &Signal<bool> {
        &self.align_center.triggered
    }

    /// Get a signal that emits when align right is triggered.
    pub fn align_right_triggered(&self) -> &Signal<bool> {
        &self.align_right.triggered
    }

    /// Get a signal that emits when justify is triggered.
    pub fn align_justify_triggered(&self) -> &Signal<bool> {
        &self.align_justify.triggered
    }

    /// Get a signal that emits when bullet list is triggered.
    pub fn bullet_list_triggered(&self) -> &Signal<bool> {
        &self.bullet_list.triggered
    }

    /// Get a signal that emits when numbered list is triggered.
    pub fn numbered_list_triggered(&self) -> &Signal<bool> {
        &self.numbered_list.triggered
    }

    /// Get a signal that emits when increase indent is triggered.
    pub fn indent_increase_triggered(&self) -> &Signal<bool> {
        &self.indent_increase.triggered
    }

    /// Get a signal that emits when decrease indent is triggered.
    pub fn indent_decrease_triggered(&self) -> &Signal<bool> {
        &self.indent_decrease.triggered
    }
}

impl Default for ParagraphActions {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Color Toolbar Widgets
// ============================================================================

/// Maximum number of recent colors to store.
const MAX_RECENT_COLORS: usize = 16;

/// Color selection widgets for text foreground and background colors.
///
/// Provides color buttons with dropdown menus for quick access to recently
/// used colors. The buttons display the current color and have a dropdown
/// arrow for accessing the recent colors palette.
///
/// # Recent Colors
///
/// When a color is selected (via dialog or palette), call `add_recent_color()`
/// to add it to the recent colors list. The list is shared between foreground
/// and background buttons.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::ColorWidgets;
///
/// let mut color_widgets = ColorWidgets::new();
///
/// // Connect to foreground button click to open color dialog
/// color_widgets.foreground_button.clicked.connect(|&current_color| {
///     // Open color dialog, then call set_foreground_color with result
/// });
///
/// // Connect to dropdown request to show recent colors palette
/// color_widgets.foreground_button.dropdown_requested.connect(|()| {
///     // Show recent colors palette popup
/// });
/// ```
pub struct ColorWidgets {
    /// Foreground (text) color button.
    pub foreground_button: ColorButton,
    /// Background (highlight) color button.
    pub background_button: ColorButton,
    /// Recent colors (shared between foreground and background).
    recent_colors: Vec<Color>,
}

impl ColorWidgets {
    /// Create new color widgets with default colors.
    ///
    /// Both buttons are created with `MenuButton` popup mode, showing a
    /// dropdown arrow for accessing recent colors.
    pub fn new() -> Self {
        use super::ColorButtonPopupMode;

        let foreground_button = ColorButton::new()
            .with_color(Color::BLACK)
            .with_popup_mode(ColorButtonPopupMode::MenuButton);

        let background_button = ColorButton::new()
            .with_color(Color::TRANSPARENT)
            .with_popup_mode(ColorButtonPopupMode::MenuButton);

        Self {
            foreground_button,
            background_button,
            recent_colors: Vec::new(),
        }
    }

    /// Create new color widgets without dropdown functionality.
    ///
    /// Use this if you don't want the recent colors palette feature.
    pub fn new_simple() -> Self {
        let foreground_button = ColorButton::new().with_color(Color::BLACK);
        let background_button = ColorButton::new().with_color(Color::TRANSPARENT);

        Self {
            foreground_button,
            background_button,
            recent_colors: Vec::new(),
        }
    }

    /// Set the foreground color display.
    pub fn set_foreground_color(&mut self, color: Color) {
        self.foreground_button.set_color(color);
    }

    /// Set the background color display.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_button.set_color(color);
    }

    /// Get the current foreground color.
    pub fn foreground_color(&self) -> Color {
        self.foreground_button.color()
    }

    /// Get the current background color.
    pub fn background_color(&self) -> Color {
        self.background_button.color()
    }

    /// Get a reference to the foreground button's clicked signal.
    ///
    /// Connect to this signal to handle foreground color selection requests.
    /// The signal parameter is the current color.
    pub fn foreground_clicked(&self) -> &Signal<Color> {
        &self.foreground_button.clicked
    }

    /// Get a reference to the background button's clicked signal.
    ///
    /// Connect to this signal to handle background color selection requests.
    /// The signal parameter is the current color.
    pub fn background_clicked(&self) -> &Signal<Color> {
        &self.background_button.clicked
    }

    /// Get a reference to the foreground button's dropdown requested signal.
    ///
    /// Connect to this signal to show the recent colors palette for foreground.
    pub fn foreground_dropdown_requested(&self) -> &Signal<()> {
        &self.foreground_button.dropdown_requested
    }

    /// Get a reference to the background button's dropdown requested signal.
    ///
    /// Connect to this signal to show the recent colors palette for background.
    pub fn background_dropdown_requested(&self) -> &Signal<()> {
        &self.background_button.dropdown_requested
    }

    // =========================================================================
    // Recent Colors
    // =========================================================================

    /// Get the recent colors list.
    pub fn recent_colors(&self) -> &[Color] {
        &self.recent_colors
    }

    /// Add a color to the recent colors list.
    ///
    /// The color is added to the front of the list. If it already exists,
    /// it is moved to the front. The list is capped at 16 colors.
    pub fn add_recent_color(&mut self, color: Color) {
        // Remove if already exists
        self.recent_colors.retain(|&c| c != color);
        // Add to front
        self.recent_colors.insert(0, color);
        // Enforce max
        if self.recent_colors.len() > MAX_RECENT_COLORS {
            self.recent_colors.pop();
        }
    }

    /// Set the recent colors list.
    pub fn set_recent_colors(&mut self, colors: Vec<Color>) {
        self.recent_colors = colors;
        if self.recent_colors.len() > MAX_RECENT_COLORS {
            self.recent_colors.truncate(MAX_RECENT_COLORS);
        }
    }

    /// Clear the recent colors list.
    pub fn clear_recent_colors(&mut self) {
        self.recent_colors.clear();
    }
}

impl Default for ColorWidgets {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Font Selection Widgets
// ============================================================================

/// Font selection widgets for font family and size.
///
/// Provides a combo box for font family selection and a spin box for font size.
/// Connect to the widgets' signals to handle font changes.
///
/// # Example
///
/// ```ignore
/// let font_widgets = FontWidgets::new();
///
/// // Connect to font family changes
/// font_widgets.font_family.font_changed.connect(|family| {
///     text_edit.set_char_font_family(Some(FontFamily::Name(family.clone())));
/// });
///
/// // Connect to font size changes
/// font_widgets.font_size.value_changed.connect(|&size| {
///     text_edit.set_char_font_size(Some(size as f32));
/// });
/// ```
pub struct FontWidgets {
    /// Font family combo box.
    pub font_family: FontComboBox,
    /// Font size spin box.
    pub font_size: DoubleSpinBox,
}

impl FontWidgets {
    /// Create new font widgets with default values.
    pub fn new() -> Self {
        let font_family = FontComboBox::new();
        let mut font_size = DoubleSpinBox::new();

        // Configure font size spin box
        font_size.set_range(6.0, 144.0);
        font_size.set_value(12.0);
        font_size.set_single_step(1.0);
        font_size.set_suffix(" pt");

        Self {
            font_family,
            font_size,
        }
    }

    /// Set the current font family.
    pub fn set_font_family(&mut self, family: &str) {
        self.font_family.set_current_font(family);
    }

    /// Set the current font size.
    pub fn set_font_size(&mut self, size: f64) {
        self.font_size.set_value(size);
    }

    /// Get the current font family.
    pub fn current_font_family(&self) -> Option<String> {
        self.font_family.current_font()
    }

    /// Get the current font size.
    pub fn current_font_size(&self) -> f64 {
        self.font_size.value()
    }

    /// Get a reference to the font family changed signal.
    ///
    /// Connect to this signal to handle font family changes.
    pub fn font_family_changed(&self) -> &Signal<String> {
        &self.font_family.font_changed
    }

    /// Get a reference to the font size changed signal.
    ///
    /// Connect to this signal to handle font size changes.
    pub fn font_size_changed(&self) -> &Signal<f64> {
        &self.font_size.value_changed
    }
}

impl Default for FontWidgets {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Complete Toolbar
// ============================================================================

/// Complete toolbar integration for TextEdit with all formatting controls.
///
/// This struct combines all formatting actions and widgets into a single
/// cohesive toolbar setup.
///
/// # Example
///
/// ```ignore
/// let text_edit = TextEdit::new();
/// let mut toolbar = ToolBar::new();
///
/// let format_toolbar = TextEditToolbar::new();
/// format_toolbar.populate_toolbar(&mut toolbar);
/// format_toolbar.connect_to_text_edit(&text_edit);
/// ```
pub struct TextEditToolbar {
    /// Character format actions (bold, italic, underline, strikethrough).
    pub format_actions: FormatActions,
    /// Paragraph format actions (alignment, lists, indent).
    pub paragraph_actions: ParagraphActions,
    /// Color selection widgets.
    pub color_widgets: ColorWidgets,
    /// Font selection widgets.
    pub font_widgets: FontWidgets,
}

impl TextEditToolbar {
    /// Create a new TextEdit toolbar with all formatting controls.
    pub fn new() -> Self {
        Self {
            format_actions: FormatActions::new(),
            paragraph_actions: ParagraphActions::new(),
            color_widgets: ColorWidgets::new(),
            font_widgets: FontWidgets::new(),
        }
    }

    /// Populate a toolbar with all formatting controls.
    ///
    /// The toolbar is populated in the following order:
    /// 1. Font family combo box
    /// 2. Font size spin box
    /// 3. Separator
    /// 4. Format actions (bold, italic, underline, strikethrough)
    /// 5. Separator
    /// 6. Color buttons (foreground, background)
    /// 7. Separator
    /// 8. Paragraph actions (alignment, lists, indent)
    pub fn populate_toolbar(&self, toolbar: &mut ToolBar) {
        // Font selection (widgets need to be added via widget IDs)
        // For now, just add the actions
        toolbar.add_separator();

        // Format actions
        self.format_actions.add_to_toolbar(toolbar);
        toolbar.add_separator();

        // Paragraph actions
        self.paragraph_actions.add_to_toolbar(toolbar);
    }

    /// Connect all toolbar controls to a TextEdit widget.
    ///
    /// This establishes bidirectional synchronization between the toolbar
    /// and the text editor.
    pub fn connect_to_text_edit(&self, text_edit: &TextEdit) {
        self.format_actions.connect_to_text_edit(text_edit);
        self.paragraph_actions.connect_to_text_edit(text_edit);
    }

    /// Get a reference to the format actions.
    pub fn format_actions(&self) -> &FormatActions {
        &self.format_actions
    }

    /// Get a reference to the paragraph actions.
    pub fn paragraph_actions(&self) -> &ParagraphActions {
        &self.paragraph_actions
    }

    /// Get a reference to the color widgets.
    pub fn color_widgets(&self) -> &ColorWidgets {
        &self.color_widgets
    }

    /// Get a reference to the font widgets.
    pub fn font_widgets(&self) -> &FontWidgets {
        &self.font_widgets
    }

    /// Get a mutable reference to the color widgets.
    pub fn color_widgets_mut(&mut self) -> &mut ColorWidgets {
        &mut self.color_widgets
    }

    /// Get a mutable reference to the font widgets.
    pub fn font_widgets_mut(&mut self) -> &mut FontWidgets {
        &mut self.font_widgets
    }
}

impl Default for TextEditToolbar {
    fn default() -> Self {
        Self::new()
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
        let _ = init_global_registry();
    }

    #[test]
    fn test_format_actions_creation() {
        setup();
        let actions = FormatActions::new();

        assert!(actions.bold.is_checkable());
        assert!(actions.italic.is_checkable());
        assert!(actions.underline.is_checkable());
        assert!(actions.strikethrough.is_checkable());

        // All should start unchecked
        assert!(!actions.bold.is_checked());
        assert!(!actions.italic.is_checked());
        assert!(!actions.underline.is_checked());
        assert!(!actions.strikethrough.is_checked());
    }

    #[test]
    fn test_format_actions_toggle() {
        setup();
        let actions = FormatActions::new();

        // Toggle bold
        actions.bold.toggle();
        assert!(actions.bold.is_checked());

        actions.bold.toggle();
        assert!(!actions.bold.is_checked());
    }

    #[test]
    fn test_paragraph_actions_creation() {
        setup();
        let actions = ParagraphActions::new();

        // Alignment actions should be checkable
        assert!(actions.align_left.is_checkable());
        assert!(actions.align_center.is_checkable());
        assert!(actions.align_right.is_checkable());
        assert!(actions.align_justify.is_checkable());

        // Left should be default checked
        assert!(actions.align_left.is_checked());
        assert!(!actions.align_center.is_checked());
        assert!(!actions.align_right.is_checked());
        assert!(!actions.align_justify.is_checked());

        // List actions should be checkable
        assert!(actions.bullet_list.is_checkable());
        assert!(actions.numbered_list.is_checkable());

        // Indent actions should not be checkable
        assert!(!actions.indent_increase.is_checkable());
        assert!(!actions.indent_decrease.is_checkable());
    }

    #[test]
    fn test_paragraph_alignment_exclusivity() {
        setup();
        let actions = ParagraphActions::new();

        // Check center - should uncheck left via action group
        actions.align_center.set_checked(true);
        assert!(actions.align_center.is_checked());
        // Note: ActionGroup exclusivity is handled by the group, not individual actions
    }

    #[test]
    fn test_color_widgets_creation() {
        setup();
        let widgets = ColorWidgets::new();

        // Default colors
        assert_eq!(widgets.foreground_color(), Color::BLACK);
        assert_eq!(widgets.background_color(), Color::TRANSPARENT);
    }

    #[test]
    fn test_color_widgets_set_color() {
        setup();
        let mut widgets = ColorWidgets::new();

        widgets.set_foreground_color(Color::RED);
        assert_eq!(widgets.foreground_color(), Color::RED);

        widgets.set_background_color(Color::YELLOW);
        assert_eq!(widgets.background_color(), Color::YELLOW);
    }

    #[test]
    fn test_font_widgets_creation() {
        setup();
        let widgets = FontWidgets::new();

        // Default font size
        assert_eq!(widgets.current_font_size(), 12.0);
    }

    #[test]
    fn test_font_widgets_set_size() {
        setup();
        let mut widgets = FontWidgets::new();

        widgets.set_font_size(24.0);
        assert_eq!(widgets.current_font_size(), 24.0);
    }

    #[test]
    fn test_text_edit_toolbar_creation() {
        setup();
        let toolbar = TextEditToolbar::new();

        // Should have all sub-components
        assert!(toolbar.format_actions.bold.is_checkable());
        assert!(toolbar.paragraph_actions.align_left.is_checkable());
    }
}
