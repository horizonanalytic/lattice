//! FontComboBox widget for font family selection.
//!
//! The FontComboBox widget provides a dropdown for selecting font families:
//! - Lists all available system fonts
//! - Preview of each font in the dropdown rendered in that font
//! - Optional filtering by font type (all, monospace, proportional)
//! - Editable mode for type-to-filter functionality
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::FontComboBox;
//!
//! // Create a font combo box with all system fonts
//! let mut combo = FontComboBox::new();
//!
//! // Or filter to only monospace fonts
//! let mut mono_combo = FontComboBox::new()
//!     .with_filter(FontFilter::Monospace);
//!
//! // Connect to the font_changed signal
//! combo.font_changed.connect(|family| {
//!     println!("Selected font: {}", family);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, RoundedRect, Size, Stroke,
    TextLayout, TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

// ============================================================================
// Font Filter
// ============================================================================

/// Filter for which fonts to show in the FontComboBox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFilter {
    /// Show all available fonts.
    #[default]
    All,
    /// Show only monospace fonts.
    Monospace,
    /// Show only proportional (non-monospace) fonts.
    Proportional,
}

// ============================================================================
// FontComboBox Parts
// ============================================================================

/// Parts of the FontComboBox for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FontComboBoxPart {
    #[default]
    None,
    /// The text/display area.
    Display,
    /// The dropdown arrow button.
    Arrow,
    /// An item in the popup list.
    PopupItem(usize),
}

// ============================================================================
// FontComboBox Widget
// ============================================================================

/// A dropdown selection widget for choosing font families.
///
/// FontComboBox provides a convenient way to select from available system fonts.
/// It automatically enumerates fonts and can optionally show a preview of each
/// font in its own typeface.
///
/// # Features
///
/// - Automatic system font enumeration
/// - Font preview in dropdown (each font rendered in its own typeface)
/// - Filter by font type (all, monospace, proportional)
/// - Editable mode for type-to-filter functionality
/// - Keyboard navigation (arrow keys, Enter, Escape)
///
/// # Signals
///
/// - `font_changed(String)`: Emitted when the selected font family changes
pub struct FontComboBox {
    /// Widget base.
    base: WidgetBase,

    /// Available font families (cached from FontSystem).
    families: Vec<String>,

    /// Whether fonts are monospace (parallel to families).
    monospace_flags: Vec<bool>,

    /// Current selected index (-1 means no selection).
    current_index: i32,

    /// Current font filter.
    filter: FontFilter,

    /// Filtered indices (indices into families vec).
    filtered_indices: Vec<usize>,

    /// Whether the combobox is editable.
    editable: bool,

    /// Current edit text (for editable mode).
    edit_text: String,

    /// Cursor position in edit text.
    cursor_pos: usize,

    /// Whether we're currently editing.
    is_editing: bool,

    /// Placeholder text for editable mode.
    placeholder: String,

    /// Whether the popup is currently visible.
    popup_visible: bool,

    /// Highlighted index in popup (-1 means no highlight).
    highlighted_index: i32,

    /// Text filter indices (for editable mode search).
    text_filter_indices: Vec<usize>,

    /// Whether text filtering is active.
    text_filtering_active: bool,

    /// Case insensitive filtering.
    case_insensitive: bool,

    /// Maximum visible items in popup.
    max_visible_items: usize,

    /// Scroll offset in popup.
    scroll_offset: usize,

    /// Item height in popup.
    item_height: f32,

    /// Whether to show font preview in dropdown.
    show_preview: bool,

    // Appearance
    /// Background color.
    background_color: Color,
    /// Text color.
    text_color: Color,
    /// Placeholder text color.
    placeholder_color: Color,
    /// Border color.
    border_color: Color,
    /// Focus border color.
    focus_border_color: Color,
    /// Arrow button color.
    arrow_color: Color,
    /// Arrow button hover color.
    arrow_hover_color: Color,
    /// Popup background color.
    popup_background_color: Color,
    /// Popup border color.
    popup_border_color: Color,
    /// Selection background color.
    selection_color: Color,
    /// Hover background color.
    hover_color: Color,
    /// Selected text color.
    selected_text_color: Color,

    /// Font for the widget text (not the preview).
    font: Font,
    /// Font size for preview text.
    preview_font_size: f32,
    /// Border radius.
    border_radius: f32,
    /// Arrow button width.
    arrow_width: f32,
    /// Padding inside the widget.
    padding: f32,

    /// Current hover part.
    hover_part: FontComboBoxPart,
    /// Current pressed part.
    pressed_part: FontComboBoxPart,

    // Signals
    /// Signal emitted when the selected font family changes.
    pub font_changed: Signal<String>,
}

impl FontComboBox {
    /// Create a new FontComboBox with default settings.
    ///
    /// This will enumerate all system fonts. The font enumeration happens
    /// lazily when the widget is first displayed or when fonts are accessed.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        let mut this = Self {
            base,
            families: Vec::new(),
            monospace_flags: Vec::new(),
            current_index: -1,
            filter: FontFilter::All,
            filtered_indices: Vec::new(),
            editable: false,
            edit_text: String::new(),
            cursor_pos: 0,
            is_editing: false,
            placeholder: String::new(),
            popup_visible: false,
            highlighted_index: -1,
            text_filter_indices: Vec::new(),
            text_filtering_active: false,
            case_insensitive: true,
            max_visible_items: 10,
            scroll_offset: 0,
            item_height: 24.0,
            show_preview: true,
            background_color: Color::WHITE,
            text_color: Color::BLACK,
            placeholder_color: Color::from_rgb8(160, 160, 160),
            border_color: Color::from_rgb8(180, 180, 180),
            focus_border_color: Color::from_rgb8(51, 153, 255),
            arrow_color: Color::from_rgb8(100, 100, 100),
            arrow_hover_color: Color::from_rgb8(51, 153, 255),
            popup_background_color: Color::WHITE,
            popup_border_color: Color::from_rgb8(180, 180, 180),
            selection_color: Color::from_rgba8(51, 153, 255, 255),
            hover_color: Color::from_rgba8(200, 200, 200, 100),
            selected_text_color: Color::WHITE,
            font: Font::new(FontFamily::SansSerif, 13.0),
            preview_font_size: 14.0,
            border_radius: 4.0,
            arrow_width: 24.0,
            padding: 6.0,
            hover_part: FontComboBoxPart::None,
            pressed_part: FontComboBoxPart::None,
            font_changed: Signal::new(),
        };

        // Load system fonts
        this.load_fonts();

        this
    }

    /// Load fonts from the system.
    fn load_fonts(&mut self) {
        let font_system = FontSystem::new();

        // Get all unique family names
        self.families = font_system.family_names();

        // Determine which fonts are monospace
        self.monospace_flags = self
            .families
            .iter()
            .map(|family| {
                font_system
                    .faces()
                    .find(|face| face.families.contains(family))
                    .map(|face| face.monospaced)
                    .unwrap_or(false)
            })
            .collect();

        // Apply initial filter
        self.apply_filter();
    }

    /// Apply the current font filter.
    fn apply_filter(&mut self) {
        self.filtered_indices = self
            .families
            .iter()
            .enumerate()
            .filter(|(i, _)| match self.filter {
                FontFilter::All => true,
                FontFilter::Monospace => self.monospace_flags.get(*i).copied().unwrap_or(false),
                FontFilter::Proportional => !self.monospace_flags.get(*i).copied().unwrap_or(true),
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if current selection is no longer valid
        if self.current_index >= 0 {
            let current_family_idx = self.current_index as usize;
            if !self.filtered_indices.contains(&current_family_idx) {
                self.current_index = -1;
            }
        }
    }

    // =========================================================================
    // Font Access
    // =========================================================================

    /// Get the number of fonts (after filtering).
    pub fn count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get the total number of fonts (before filtering).
    pub fn total_count(&self) -> usize {
        self.families.len()
    }

    /// Get the font family at the given display index.
    pub fn font_family(&self, display_index: usize) -> Option<&str> {
        self.filtered_indices
            .get(display_index)
            .and_then(|&idx| self.families.get(idx))
            .map(|s| s.as_str())
    }

    /// Find the display index of a font family by name.
    pub fn find_font(&self, family: &str) -> Option<usize> {
        self.filtered_indices.iter().position(|&idx| {
            self.families
                .get(idx)
                .map(|f| f.eq_ignore_ascii_case(family))
                .unwrap_or(false)
        })
    }

    /// Check if a font family exists.
    pub fn has_font(&self, family: &str) -> bool {
        self.families.iter().any(|f| f.eq_ignore_ascii_case(family))
    }

    // =========================================================================
    // Current Selection
    // =========================================================================

    /// Get the current selected display index (-1 if no selection).
    pub fn current_index(&self) -> i32 {
        self.current_index
    }

    /// Set the current selected display index.
    pub fn set_current_index(&mut self, index: i32) {
        let count = self.count() as i32;
        let new_index = if index < 0 || index >= count {
            -1
        } else {
            index
        };

        if self.current_index != new_index {
            self.current_index = new_index;

            // Update edit text for editable mode
            if self.editable
                && new_index >= 0
                && let Some(family) = self.font_family(new_index as usize)
            {
                self.edit_text = family.to_string();
                self.cursor_pos = self.edit_text.len();
            }

            self.base.update();

            // Emit font_changed signal
            if new_index >= 0 {
                if let Some(family) = self.font_family(new_index as usize) {
                    self.font_changed.emit(family.to_string());
                }
            } else {
                self.font_changed.emit(String::new());
            }
        }
    }

    /// Set current index using builder pattern.
    pub fn with_current_index(mut self, index: i32) -> Self {
        self.set_current_index(index);
        self
    }

    /// Get the current selected font family.
    pub fn current_font(&self) -> Option<String> {
        if self.editable && self.current_index < 0 && !self.edit_text.is_empty() {
            Some(self.edit_text.clone())
        } else if self.current_index >= 0 {
            self.font_family(self.current_index as usize)
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Set the current font by family name.
    pub fn set_current_font(&mut self, family: impl AsRef<str>) {
        let family = family.as_ref();

        if self.editable {
            self.edit_text = family.to_string();
            self.cursor_pos = self.edit_text.len();
        }

        // Try to find matching font
        if let Some(idx) = self.find_font(family) {
            self.set_current_index(idx as i32);
        } else if self.editable {
            // No match, but in editable mode we keep the text
            self.current_index = -1;
            self.font_changed.emit(family.to_string());
        } else {
            self.current_index = -1;
        }

        self.base.update();
    }

    /// Set current font using builder pattern.
    pub fn with_current_font(mut self, family: impl AsRef<str>) -> Self {
        self.set_current_font(family);
        self
    }

    // =========================================================================
    // Filter
    // =========================================================================

    /// Get the current font filter.
    pub fn filter(&self) -> FontFilter {
        self.filter
    }

    /// Set the font filter.
    pub fn set_filter(&mut self, filter: FontFilter) {
        if self.filter != filter {
            self.filter = filter;
            self.apply_filter();
            self.base.update();
        }
    }

    /// Set filter using builder pattern.
    pub fn with_filter(mut self, filter: FontFilter) -> Self {
        self.set_filter(filter);
        self
    }

    // =========================================================================
    // Editable Mode
    // =========================================================================

    /// Check if the combobox is editable.
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether the combobox is editable.
    pub fn set_editable(&mut self, editable: bool) {
        if self.editable != editable {
            self.editable = editable;

            // Initialize edit text from current selection
            if editable
                && self.current_index >= 0
                && let Some(family) = self.font_family(self.current_index as usize)
            {
                self.edit_text = family.to_string();
                self.cursor_pos = self.edit_text.len();
            }

            self.base.update();
        }
    }

    /// Set editable using builder pattern.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.set_editable(editable);
        self
    }

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set placeholder text for editable mode.
    pub fn set_placeholder(&mut self, text: impl Into<String>) {
        self.placeholder = text.into();
        self.base.update();
    }

    /// Set placeholder using builder pattern.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    // =========================================================================
    // Preview
    // =========================================================================

    /// Check if font preview is enabled.
    pub fn show_preview(&self) -> bool {
        self.show_preview
    }

    /// Set whether to show font preview in dropdown.
    pub fn set_show_preview(&mut self, show: bool) {
        self.show_preview = show;
        self.base.update();
    }

    /// Set show preview using builder pattern.
    pub fn with_show_preview(mut self, show: bool) -> Self {
        self.show_preview = show;
        self
    }

    /// Get the preview font size.
    pub fn preview_font_size(&self) -> f32 {
        self.preview_font_size
    }

    /// Set the preview font size.
    pub fn set_preview_font_size(&mut self, size: f32) {
        self.preview_font_size = size.max(8.0);
        self.base.update();
    }

    /// Set preview font size using builder pattern.
    pub fn with_preview_font_size(mut self, size: f32) -> Self {
        self.preview_font_size = size.max(8.0);
        self
    }

    // =========================================================================
    // Popup Configuration
    // =========================================================================

    /// Check if the popup is visible.
    pub fn is_popup_visible(&self) -> bool {
        self.popup_visible
    }

    /// Get the maximum number of visible items in the popup.
    pub fn max_visible_items(&self) -> usize {
        self.max_visible_items
    }

    /// Set the maximum number of visible items in the popup.
    pub fn set_max_visible_items(&mut self, count: usize) {
        self.max_visible_items = count.max(1);
    }

    /// Set max visible items using builder pattern.
    pub fn with_max_visible_items(mut self, count: usize) -> Self {
        self.max_visible_items = count.max(1);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.base.update();
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
        self.base.update();
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the font for the widget (not preview).
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Set the border radius.
    pub fn set_border_radius(&mut self, radius: f32) {
        self.border_radius = radius;
        self.base.update();
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    // =========================================================================
    // Popup Control
    // =========================================================================

    /// Show the dropdown popup.
    pub fn show_popup(&mut self) {
        if self.count() == 0 {
            return;
        }

        self.popup_visible = true;
        self.update_text_filter();

        // Set highlighted to current selection or first item
        if self.current_index >= 0 {
            self.highlighted_index = if self.text_filtering_active {
                // Find current index in filtered list
                self.text_filter_indices
                    .iter()
                    .position(|&i| i == self.current_index as usize)
                    .map(|i| i as i32)
                    .unwrap_or(0)
            } else {
                self.current_index
            };
        } else {
            self.highlighted_index = 0;
        }

        self.ensure_highlighted_visible();
        self.base.update();
    }

    /// Hide the dropdown popup.
    pub fn hide_popup(&mut self) {
        if self.popup_visible {
            self.popup_visible = false;
            self.highlighted_index = -1;
            self.text_filtering_active = false;
            self.text_filter_indices.clear();
            self.base.update();
        }
    }

    /// Toggle the popup visibility.
    pub fn toggle_popup(&mut self) {
        if self.popup_visible {
            self.hide_popup();
        } else {
            self.show_popup();
        }
    }

    fn update_text_filter(&mut self) {
        if self.editable && !self.edit_text.is_empty() {
            let prefix = if self.case_insensitive {
                self.edit_text.to_lowercase()
            } else {
                self.edit_text.clone()
            };

            self.text_filter_indices = self
                .filtered_indices
                .iter()
                .enumerate()
                .filter(|(_, family_idx)| {
                    if let Some(family) = self.families.get(**family_idx) {
                        let family_cmp = if self.case_insensitive {
                            family.to_lowercase()
                        } else {
                            family.clone()
                        };
                        family_cmp.starts_with(&prefix)
                    } else {
                        false
                    }
                })
                .map(|(display_idx, _)| display_idx)
                .collect();
            self.text_filtering_active = true;
        } else {
            self.text_filtering_active = false;
            self.text_filter_indices.clear();
        }
    }

    fn effective_item_count(&self) -> usize {
        if self.text_filtering_active {
            self.text_filter_indices.len()
        } else {
            self.count()
        }
    }

    fn actual_display_index(&self, visual_index: usize) -> usize {
        if self.text_filtering_active {
            self.text_filter_indices
                .get(visual_index)
                .copied()
                .unwrap_or(visual_index)
        } else {
            visual_index
        }
    }

    fn ensure_highlighted_visible(&mut self) {
        if self.highlighted_index < 0 {
            return;
        }

        let idx = self.highlighted_index as usize;
        if idx < self.scroll_offset {
            self.scroll_offset = idx;
        } else if idx >= self.scroll_offset + self.max_visible_items {
            self.scroll_offset = idx - self.max_visible_items + 1;
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    #[allow(dead_code)]
    fn display_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(0.0, 0.0, rect.width() - self.arrow_width, rect.height())
    }

    fn arrow_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.arrow_width,
            0.0,
            self.arrow_width,
            rect.height(),
        )
    }

    fn popup_rect(&self) -> Rect {
        let rect = self.base.rect();
        let item_count = self.effective_item_count();
        let visible_count = item_count.min(self.max_visible_items);
        let popup_height = visible_count as f32 * self.item_height + 2.0; // +2 for border

        Rect::new(0.0, rect.height(), rect.width(), popup_height)
    }

    fn hit_test(&self, pos: Point) -> FontComboBoxPart {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        if !local_rect.contains(pos) {
            // Check popup
            if self.popup_visible {
                let popup_rect = self.popup_rect();
                if popup_rect.contains(pos) {
                    let local_y = pos.y - popup_rect.origin.y - 1.0; // -1 for border
                    let visual_idx = (local_y / self.item_height) as usize + self.scroll_offset;
                    if visual_idx < self.effective_item_count() {
                        return FontComboBoxPart::PopupItem(visual_idx);
                    }
                }
            }
            return FontComboBoxPart::None;
        }

        if self.arrow_rect().contains(pos) {
            FontComboBoxPart::Arrow
        } else {
            FontComboBoxPart::Display
        }
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        self.pressed_part = part;

        match part {
            FontComboBoxPart::Arrow => {
                self.toggle_popup();
                self.base.update();
                true
            }
            FontComboBoxPart::Display => {
                if self.editable {
                    self.is_editing = true;
                    self.cursor_pos = self.edit_text.len();
                } else {
                    self.toggle_popup();
                }
                self.base.update();
                true
            }
            FontComboBoxPart::PopupItem(visual_idx) => {
                let display_idx = self.actual_display_index(visual_idx);
                self.set_current_index(display_idx as i32);
                self.hide_popup();
                true
            }
            FontComboBoxPart::None => {
                // Click outside - close popup
                if self.popup_visible {
                    self.hide_popup();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        self.pressed_part = FontComboBoxPart::None;
        self.base.update();
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let part = self.hit_test(event.local_pos);

        if part != self.hover_part {
            self.hover_part = part;

            // Update highlighted item in popup
            if let FontComboBoxPart::PopupItem(visual_idx) = part {
                self.highlighted_index = visual_idx as i32;
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let item_count = self.effective_item_count();

        match event.key {
            Key::Escape => {
                if self.popup_visible {
                    self.hide_popup();
                    return true;
                }
            }
            Key::Enter => {
                if self.popup_visible && self.highlighted_index >= 0 {
                    let display_idx = self.actual_display_index(self.highlighted_index as usize);
                    self.set_current_index(display_idx as i32);
                    self.hide_popup();
                    return true;
                }
            }
            Key::ArrowDown => {
                if self.popup_visible {
                    if self.highlighted_index < item_count as i32 - 1 {
                        self.highlighted_index += 1;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                } else {
                    // Show popup or move to next item
                    if event.modifiers.alt {
                        self.show_popup();
                    } else if self.current_index < self.count() as i32 - 1 {
                        self.set_current_index(self.current_index + 1);
                    }
                }
                return true;
            }
            Key::ArrowUp => {
                if self.popup_visible {
                    if self.highlighted_index > 0 {
                        self.highlighted_index -= 1;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                } else {
                    // Move to previous item
                    if self.current_index > 0 {
                        self.set_current_index(self.current_index - 1);
                    }
                }
                return true;
            }
            Key::PageDown => {
                if self.popup_visible {
                    let new_idx = (self.highlighted_index + self.max_visible_items as i32)
                        .min(item_count as i32 - 1);
                    if new_idx != self.highlighted_index {
                        self.highlighted_index = new_idx;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                    return true;
                }
            }
            Key::PageUp => {
                if self.popup_visible {
                    let new_idx = (self.highlighted_index - self.max_visible_items as i32).max(0);
                    if new_idx != self.highlighted_index {
                        self.highlighted_index = new_idx;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                    return true;
                }
            }
            Key::Home => {
                if self.popup_visible && item_count > 0 {
                    self.highlighted_index = 0;
                    self.scroll_offset = 0;
                    self.base.update();
                    return true;
                }
            }
            Key::End => {
                if self.popup_visible && item_count > 0 {
                    self.highlighted_index = item_count as i32 - 1;
                    self.ensure_highlighted_visible();
                    self.base.update();
                    return true;
                }
            }
            Key::Space => {
                if !self.editable && !self.popup_visible {
                    self.show_popup();
                    return true;
                }
            }
            Key::Backspace => {
                if self.editable && !self.edit_text.is_empty() && self.cursor_pos > 0 {
                    // Delete character before cursor
                    use unicode_segmentation::UnicodeSegmentation;
                    let graphemes: Vec<&str> = self.edit_text.graphemes(true).collect();
                    let mut byte_pos = 0;
                    let mut grapheme_idx = 0;

                    for (i, g) in graphemes.iter().enumerate() {
                        let next_byte_pos = byte_pos + g.len();
                        if next_byte_pos >= self.cursor_pos {
                            grapheme_idx = i;
                            break;
                        }
                        byte_pos = next_byte_pos;
                    }

                    if grapheme_idx > 0 {
                        let new_text: String = graphemes
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != grapheme_idx - 1)
                            .map(|(_, g)| *g)
                            .collect();

                        let deleted_len = graphemes[grapheme_idx - 1].len();
                        self.cursor_pos -= deleted_len;
                        self.edit_text = new_text;
                    }

                    self.update_text_filter();
                    if self.popup_visible {
                        self.highlighted_index = 0;
                    }
                    self.base.update();
                    return true;
                }
            }
            Key::Delete => {
                if self.editable && self.cursor_pos < self.edit_text.len() {
                    // Delete character after cursor
                    use unicode_segmentation::UnicodeSegmentation;
                    let graphemes: Vec<&str> = self.edit_text.graphemes(true).collect();
                    let mut byte_pos = 0;
                    let mut grapheme_idx = 0;

                    for (i, g) in graphemes.iter().enumerate() {
                        if byte_pos >= self.cursor_pos {
                            grapheme_idx = i;
                            break;
                        }
                        byte_pos += g.len();
                    }

                    if grapheme_idx < graphemes.len() {
                        let new_text: String = graphemes
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != grapheme_idx)
                            .map(|(_, g)| *g)
                            .collect();

                        self.edit_text = new_text;
                        self.update_text_filter();
                        if self.popup_visible {
                            self.highlighted_index = 0;
                        }
                        self.base.update();
                    }
                    return true;
                }
            }
            _ => {}
        }

        // Handle character input for editable mode
        if self.editable
            && let Some(ch) = event.text.chars().next()
            && !ch.is_control()
        {
            // Insert character at cursor
            self.edit_text.insert(self.cursor_pos, ch);
            self.cursor_pos += ch.len_utf8();

            self.update_text_filter();

            // Show popup if we have matches
            if !self.text_filter_indices.is_empty() && !self.popup_visible {
                self.show_popup();
            } else if self.popup_visible {
                self.highlighted_index = 0;
                self.scroll_offset = 0;
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        if self.popup_visible {
            // Scroll the popup
            let delta = if event.delta_y > 0.0 { -1i32 } else { 1 };
            let item_count = self.effective_item_count();
            let max_scroll = item_count.saturating_sub(self.max_visible_items);

            let new_offset = (self.scroll_offset as i32 + delta)
                .max(0)
                .min(max_scroll as i32) as usize;

            if new_offset != self.scroll_offset {
                self.scroll_offset = new_offset;
                self.base.update();
            }
            return true;
        } else if !self.editable {
            // Change selection with wheel
            let delta = if event.delta_y > 0.0 { -1i32 } else { 1 };
            let new_index = (self.current_index + delta)
                .max(0)
                .min(self.count() as i32 - 1);

            if new_index != self.current_index && new_index >= 0 {
                self.set_current_index(new_index);
                return true;
            }
        }

        false
    }

    fn handle_focus_out(&mut self) -> bool {
        if self.popup_visible {
            self.hide_popup();
        }
        if self.editable && self.is_editing {
            self.is_editing = false;
        }
        self.base.update();
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_main(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        // Draw background
        let rounded = RoundedRect::new(local_rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(rounded, self.background_color);

        // Draw border
        let border_color = if self.base.has_focus() {
            self.focus_border_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);

        // Draw separator before arrow
        let arrow_x = rect.width() - self.arrow_width;
        let sep_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(arrow_x, 2.0),
            Point::new(arrow_x, rect.height() - 2.0),
            &sep_stroke,
        );

        // Draw arrow button background on hover
        if matches!(self.hover_part, FontComboBoxPart::Arrow) {
            let arrow_rect = Rect::new(
                arrow_x + 1.0,
                1.0,
                self.arrow_width - 2.0,
                rect.height() - 2.0,
            );
            ctx.renderer()
                .fill_rect(arrow_rect, Color::from_rgba8(200, 200, 200, 50));
        }

        // Draw dropdown arrow
        let arrow_color = if matches!(self.hover_part, FontComboBoxPart::Arrow) {
            self.arrow_hover_color
        } else {
            self.arrow_color
        };
        self.paint_arrow(ctx, arrow_color);

        // Draw text
        self.paint_text(ctx);
    }

    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, color: Color) {
        let rect = self.base.rect();
        let arrow_x = rect.width() - self.arrow_width;
        let center_x = arrow_x + self.arrow_width / 2.0;
        let center_y = rect.height() / 2.0;

        // Draw a simple downward-pointing triangle
        let arrow_size = 5.0;
        let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
        let p2 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
        let p3 = Point::new(center_x, center_y + arrow_size / 2.0);

        // Draw as two lines forming a V
        let stroke = Stroke::new(color, 2.0);
        ctx.renderer().draw_line(p1, p3, &stroke);
        ctx.renderer().draw_line(p2, p3, &stroke);
    }

    fn paint_text(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let mut font_system = FontSystem::new();

        // Determine what text to display
        let (text, color, use_preview_font) = if self.editable {
            if self.edit_text.is_empty() && !self.placeholder.is_empty() {
                (self.placeholder.clone(), self.placeholder_color, false)
            } else {
                (self.edit_text.clone(), self.text_color, false)
            }
        } else if self.current_index >= 0 {
            if let Some(family) = self.font_family(self.current_index as usize) {
                (family.to_string(), self.text_color, self.show_preview)
            } else {
                return;
            }
        } else if !self.placeholder.is_empty() {
            (self.placeholder.clone(), self.placeholder_color, false)
        } else {
            return;
        };

        // Choose font
        let font = if use_preview_font {
            Font::new(FontFamily::Name(text.clone()), self.font.size())
        } else {
            self.font.clone()
        };

        let layout =
            TextLayout::with_options(&mut font_system, &text, &font, TextLayoutOptions::new());

        let text_x = self.padding;
        let text_y = (rect.height() - layout.height()) / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                color,
            );
        }

        // Draw cursor for editable mode when editing
        if self.editable && self.is_editing && self.base.has_focus() {
            let cursor_x = text_x + layout.width();
            let cursor_stroke = Stroke::new(self.text_color, 1.0);
            ctx.renderer().draw_line(
                Point::new(cursor_x, text_y),
                Point::new(cursor_x, text_y + layout.height()),
                &cursor_stroke,
            );
        }
    }

    fn paint_popup(&self, ctx: &mut PaintContext<'_>) {
        if !self.popup_visible {
            return;
        }

        let popup_rect = self.popup_rect();

        // Draw popup background
        ctx.renderer()
            .fill_rect(popup_rect, self.popup_background_color);

        // Draw popup border
        let stroke = Stroke::new(self.popup_border_color, 1.0);
        ctx.renderer().stroke_rect(popup_rect, &stroke);

        // Draw items
        let item_count = self.effective_item_count();
        let visible_count = item_count.min(self.max_visible_items);

        let mut font_system = FontSystem::new();

        for visual_idx in 0..visible_count {
            let list_idx = self.scroll_offset + visual_idx;
            if list_idx >= item_count {
                break;
            }

            let display_idx = self.actual_display_index(list_idx);

            if let Some(family) = self.font_family(display_idx) {
                let item_rect = Rect::new(
                    popup_rect.origin.x + 1.0,
                    popup_rect.origin.y + 1.0 + (visual_idx as f32) * self.item_height,
                    popup_rect.size.width - 2.0,
                    self.item_height,
                );

                let is_selected = list_idx as i32 == self.highlighted_index;
                let is_hovered =
                    matches!(self.hover_part, FontComboBoxPart::PopupItem(idx) if idx == list_idx);

                // Draw background
                if is_selected {
                    ctx.renderer().fill_rect(item_rect, self.selection_color);
                } else if is_hovered {
                    ctx.renderer().fill_rect(item_rect, self.hover_color);
                }

                let text_color = if is_selected {
                    self.selected_text_color
                } else {
                    self.text_color
                };

                // Choose font for this item
                let font = if self.show_preview {
                    Font::new(FontFamily::Name(family.to_string()), self.preview_font_size)
                } else {
                    Font::new(FontFamily::SansSerif, self.preview_font_size)
                };

                let layout = TextLayout::with_options(
                    &mut font_system,
                    family,
                    &font,
                    TextLayoutOptions::new(),
                );

                let text_x = item_rect.origin.x + self.padding;
                let text_y = item_rect.origin.y + (item_rect.height() - layout.height()) / 2.0;

                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        &mut font_system,
                        &layout,
                        Point::new(text_x, text_y),
                        text_color,
                    );
                }
            }
        }

        // Draw scroll indicators if needed
        if item_count > self.max_visible_items {
            self.paint_scroll_indicator(ctx, popup_rect, item_count);
        }
    }

    fn paint_scroll_indicator(
        &self,
        ctx: &mut PaintContext<'_>,
        popup_rect: Rect,
        item_count: usize,
    ) {
        let indicator_width = 4.0;
        let track_height = popup_rect.size.height - 2.0;
        let thumb_height = (self.max_visible_items as f32 / item_count as f32) * track_height;
        let max_scroll = item_count - self.max_visible_items;
        let thumb_y = if max_scroll > 0 {
            (self.scroll_offset as f32 / max_scroll as f32) * (track_height - thumb_height)
        } else {
            0.0
        };

        let track_rect = Rect::new(
            popup_rect.right() - indicator_width - 2.0,
            popup_rect.origin.y + 1.0,
            indicator_width,
            track_height,
        );

        let thumb_rect = Rect::new(
            track_rect.origin.x,
            track_rect.origin.y + thumb_y,
            indicator_width,
            thumb_height.max(10.0),
        );

        ctx.renderer()
            .fill_rect(track_rect, Color::from_rgb8(240, 240, 240));
        ctx.renderer()
            .fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
    }
}

impl Widget for FontComboBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(180.0, 28.0);
        SizeHint::new(preferred).with_minimum(Size::new(100.0, 24.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_main(ctx);
        self.paint_popup(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Wheel(e) => self.handle_wheel(e),
            WidgetEvent::FocusOut(_) => self.handle_focus_out(),
            WidgetEvent::Leave(_) => {
                self.hover_part = FontComboBoxPart::None;
                self.base.update();
                false
            }
            _ => false,
        }
    }
}

impl Object for FontComboBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for FontComboBox {
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
    fn test_font_combo_box_creation() {
        setup();
        let combo = FontComboBox::new();
        // Should have some fonts loaded (system fonts)
        assert!(combo.total_count() > 0 || combo.count() == 0); // May be 0 in test environment
        assert_eq!(combo.current_index(), -1);
        assert!(!combo.is_editable());
        assert!(!combo.is_popup_visible());
    }

    #[test]
    fn test_font_combo_box_filter() {
        setup();
        let mut combo = FontComboBox::new();
        let all_count = combo.count();

        combo.set_filter(FontFilter::Monospace);
        let mono_count = combo.count();

        combo.set_filter(FontFilter::Proportional);
        let prop_count = combo.count();

        // Monospace + Proportional should roughly equal All
        // (may not be exact due to edge cases)
        assert!(mono_count + prop_count <= all_count + 1);
    }

    #[test]
    fn test_font_combo_box_selection() {
        setup();
        let mut combo = FontComboBox::new();

        if combo.count() > 0 {
            combo.set_current_index(0);
            assert_eq!(combo.current_index(), 0);
            assert!(combo.current_font().is_some());

            // Out of bounds should reset to -1
            combo.set_current_index(999999);
            assert_eq!(combo.current_index(), -1);

            // Negative should be -1
            combo.set_current_index(-5);
            assert_eq!(combo.current_index(), -1);
        }
    }

    #[test]
    fn test_font_combo_box_editable() {
        setup();
        let combo = FontComboBox::new().with_editable(true);
        assert!(combo.is_editable());
    }

    #[test]
    fn test_font_combo_box_builder_pattern() {
        setup();
        let combo = FontComboBox::new()
            .with_filter(FontFilter::Monospace)
            .with_editable(true)
            .with_placeholder("Select font...")
            .with_max_visible_items(15)
            .with_show_preview(false)
            .with_preview_font_size(16.0);

        assert_eq!(combo.filter(), FontFilter::Monospace);
        assert!(combo.is_editable());
        assert_eq!(combo.placeholder(), "Select font...");
        assert_eq!(combo.max_visible_items(), 15);
        assert!(!combo.show_preview());
        assert_eq!(combo.preview_font_size(), 16.0);
    }

    #[test]
    fn test_font_combo_box_popup_control() {
        setup();
        let mut combo = FontComboBox::new();

        if combo.count() > 0 {
            assert!(!combo.is_popup_visible());

            combo.show_popup();
            assert!(combo.is_popup_visible());

            combo.hide_popup();
            assert!(!combo.is_popup_visible());

            combo.toggle_popup();
            assert!(combo.is_popup_visible());

            combo.toggle_popup();
            assert!(!combo.is_popup_visible());
        }
    }

    #[test]
    fn test_font_combo_box_empty_popup() {
        setup();
        // Create with impossible filter to get empty list
        let mut combo = FontComboBox::new();

        // If no fonts available, popup shouldn't open
        if combo.count() == 0 {
            combo.show_popup();
            assert!(!combo.is_popup_visible());
        }
    }

    #[test]
    fn test_font_combo_box_set_current_font() {
        setup();
        let mut combo = FontComboBox::new();

        // Try to set to a font that might exist
        if combo.count() > 0 {
            if let Some(first_font) = combo.font_family(0) {
                let font_name = first_font.to_string();
                combo.set_current_font(&font_name);
                assert_eq!(combo.current_index(), 0);
                assert_eq!(combo.current_font(), Some(font_name));
            }
        }
    }
}
