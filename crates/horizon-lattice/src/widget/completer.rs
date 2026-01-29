//! Completer/autocomplete functionality for text input widgets.
//!
//! This module provides [`Completer`], a component that provides autocomplete
//! functionality for text input widgets like [`LineEdit`](super::widgets::LineEdit).
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::completer::{Completer, StringListModel};
//! use horizon_lattice::widget::widgets::LineEdit;
//!
//! // Create a completer with a list of suggestions
//! let suggestions = vec![
//!     "apple".to_string(),
//!     "application".to_string(),
//!     "banana".to_string(),
//!     "cherry".to_string(),
//! ];
//! let model = StringListModel::new(suggestions);
//! let mut completer = Completer::new(Box::new(model));
//!
//! // Connect to signals
//! completer.activated.connect(|text| {
//!     println!("Selected: {}", text);
//! });
//!
//! // Attach to a LineEdit
//! let mut line_edit = LineEdit::new();
//! line_edit.set_completer(Some(completer));
//! ```

use horizon_lattice_core::Signal;
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, GpuRenderer, Point, Rect, Renderer, Size, Stroke,
    TextLayout, TextLayoutOptions, TextRenderer,
};

// ============================================================================
// Case Sensitivity
// ============================================================================

/// Controls how completion matching handles letter case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseSensitivity {
    /// Case-sensitive matching (e.g., "App" won't match "apple").
    CaseSensitive,
    /// Case-insensitive matching (e.g., "App" will match "apple").
    #[default]
    CaseInsensitive,
}

// ============================================================================
// Completer Model Trait
// ============================================================================

/// Trait for providing completion suggestions.
///
/// Implement this trait to provide custom completion data sources.
/// The model is responsible for filtering and returning relevant completions
/// based on the input prefix.
pub trait CompleterModel: Send + Sync {
    /// Get completions matching the given prefix.
    ///
    /// # Arguments
    /// * `prefix` - The text to match against
    /// * `case_sensitivity` - How to handle letter case when matching
    ///
    /// # Returns
    /// A vector of matching completion strings, ordered by relevance.
    fn completions(&self, prefix: &str, case_sensitivity: CaseSensitivity) -> Vec<String>;

    /// Get the total number of items in the model (before filtering).
    ///
    /// Returns `None` if the count is unknown or expensive to compute.
    fn count(&self) -> Option<usize> {
        None
    }
}

// ============================================================================
// String List Model
// ============================================================================

/// A simple completer model backed by a static list of strings.
///
/// This is the most common model for simple autocomplete scenarios where
/// the list of suggestions is known ahead of time.
#[derive(Debug, Clone)]
pub struct StringListModel {
    items: Vec<String>,
}

impl StringListModel {
    /// Create a new string list model with the given items.
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    /// Create an empty string list model.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Get a reference to the items.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Set the items.
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
    }

    /// Add an item to the list.
    pub fn add_item(&mut self, item: String) {
        self.items.push(item);
    }

    /// Remove an item from the list by value.
    pub fn remove_item(&mut self, item: &str) {
        self.items.retain(|i| i != item);
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl CompleterModel for StringListModel {
    fn completions(&self, prefix: &str, case_sensitivity: CaseSensitivity) -> Vec<String> {
        if prefix.is_empty() {
            return self.items.clone();
        }

        match case_sensitivity {
            CaseSensitivity::CaseSensitive => self
                .items
                .iter()
                .filter(|item| item.starts_with(prefix))
                .cloned()
                .collect(),
            CaseSensitivity::CaseInsensitive => {
                let prefix_lower = prefix.to_lowercase();
                self.items
                    .iter()
                    .filter(|item| item.to_lowercase().starts_with(&prefix_lower))
                    .cloned()
                    .collect()
            }
        }
    }

    fn count(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

impl Default for StringListModel {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Vec<String>> for StringListModel {
    fn from(items: Vec<String>) -> Self {
        Self::new(items)
    }
}

impl From<Vec<&str>> for StringListModel {
    fn from(items: Vec<&str>) -> Self {
        Self::new(items.into_iter().map(String::from).collect())
    }
}

// ============================================================================
// Completer Popup State
// ============================================================================

/// Internal state for the completer popup.
#[derive(Debug)]
pub(crate) struct CompleterPopupState {
    /// Whether the popup is currently visible.
    pub visible: bool,
    /// Current filtered completions.
    pub completions: Vec<String>,
    /// Currently selected index (-1 means no selection).
    pub selected_index: i32,
    /// The prefix used for current completions.
    pub current_prefix: String,
    /// Popup position (relative to parent).
    pub position: Point,
    /// Popup size.
    pub size: Size,
    /// Maximum number of visible items.
    pub max_visible_items: usize,
    /// Scroll offset (for when there are more items than max_visible).
    pub scroll_offset: usize,
    /// Item height in pixels.
    pub item_height: f32,
}

impl Default for CompleterPopupState {
    fn default() -> Self {
        Self {
            visible: false,
            completions: Vec::new(),
            selected_index: -1,
            current_prefix: String::new(),
            position: Point::new(0.0, 0.0),
            size: Size::new(200.0, 0.0),
            max_visible_items: 7,
            scroll_offset: 0,
            item_height: 24.0,
        }
    }
}

impl CompleterPopupState {
    /// Calculate the visible range of items.
    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let start = self.scroll_offset;
        let end = (start + self.max_visible_items).min(self.completions.len());
        start..end
    }

    /// Ensure the selected item is visible (scroll if needed).
    pub fn ensure_selected_visible(&mut self) {
        if self.selected_index < 0 {
            return;
        }

        let idx = self.selected_index as usize;
        if idx < self.scroll_offset {
            self.scroll_offset = idx;
        } else if idx >= self.scroll_offset + self.max_visible_items {
            self.scroll_offset = idx - self.max_visible_items + 1;
        }
    }

    /// Calculate popup height based on number of items.
    pub fn calculate_height(&self) -> f32 {
        let visible_count = self.completions.len().min(self.max_visible_items);
        visible_count as f32 * self.item_height + 2.0 // +2 for border
    }
}

// ============================================================================
// Completer
// ============================================================================

/// Provides autocomplete functionality for text input widgets.
///
/// The Completer component filters and displays suggestions as the user types.
/// It can be attached to a [`LineEdit`](super::widgets::LineEdit) to provide
/// autocomplete functionality.
///
/// # Features
///
/// - Prefix-based completion matching
/// - Case sensitivity control
/// - Popup list of suggestions with keyboard navigation
/// - Customizable completion models
///
/// # Signals
///
/// - `activated(String)`: Emitted when a completion is selected (Enter or click)
/// - `highlighted(String)`: Emitted when the highlighted completion changes
pub struct Completer {
    /// The completion model providing suggestions.
    model: Box<dyn CompleterModel>,

    /// Case sensitivity for matching.
    case_sensitivity: CaseSensitivity,

    /// Minimum characters before showing completions.
    min_chars: usize,

    /// Internal popup state.
    pub(crate) popup_state: CompleterPopupState,

    /// Font for rendering items.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Background color.
    background_color: Color,

    /// Selected item background color.
    selection_color: Color,

    /// Hover background color.
    hover_color: Color,

    /// Border color.
    border_color: Color,

    /// Currently hovered item index (for mouse interaction).
    hovered_index: i32,

    // Signals
    /// Signal emitted when a completion is activated (selected).
    pub activated: Signal<String>,

    /// Signal emitted when the highlighted completion changes.
    pub highlighted: Signal<String>,
}

impl Completer {
    /// Create a new completer with the given model.
    pub fn new(model: Box<dyn CompleterModel>) -> Self {
        Self {
            model,
            case_sensitivity: CaseSensitivity::CaseInsensitive,
            min_chars: 1,
            popup_state: CompleterPopupState::default(),
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            background_color: Color::WHITE,
            selection_color: Color::from_rgba8(51, 153, 255, 200),
            hover_color: Color::from_rgba8(200, 200, 200, 100),
            border_color: Color::from_rgb8(180, 180, 180),
            hovered_index: -1,
            activated: Signal::new(),
            highlighted: Signal::new(),
        }
    }

    /// Create a completer with a simple string list model.
    pub fn with_strings(items: Vec<String>) -> Self {
        Self::new(Box::new(StringListModel::new(items)))
    }

    // =========================================================================
    // Configuration
    // =========================================================================

    /// Get the case sensitivity setting.
    pub fn case_sensitivity(&self) -> CaseSensitivity {
        self.case_sensitivity
    }

    /// Set the case sensitivity for matching.
    pub fn set_case_sensitivity(&mut self, sensitivity: CaseSensitivity) {
        self.case_sensitivity = sensitivity;
    }

    /// Set case sensitivity using builder pattern.
    pub fn with_case_sensitivity(mut self, sensitivity: CaseSensitivity) -> Self {
        self.case_sensitivity = sensitivity;
        self
    }

    /// Get the minimum number of characters before showing completions.
    pub fn min_chars(&self) -> usize {
        self.min_chars
    }

    /// Set the minimum number of characters before showing completions.
    pub fn set_min_chars(&mut self, count: usize) {
        self.min_chars = count;
    }

    /// Set minimum characters using builder pattern.
    pub fn with_min_chars(mut self, count: usize) -> Self {
        self.min_chars = count;
        self
    }

    /// Get the maximum number of visible items in the popup.
    pub fn max_visible_items(&self) -> usize {
        self.popup_state.max_visible_items
    }

    /// Set the maximum number of visible items in the popup.
    pub fn set_max_visible_items(&mut self, count: usize) {
        self.popup_state.max_visible_items = count.max(1);
    }

    /// Set maximum visible items using builder pattern.
    pub fn with_max_visible_items(mut self, count: usize) -> Self {
        self.popup_state.max_visible_items = count.max(1);
        self
    }

    // =========================================================================
    // Model Access
    // =========================================================================

    /// Get a reference to the model.
    pub fn model(&self) -> &dyn CompleterModel {
        self.model.as_ref()
    }

    /// Set a new model.
    pub fn set_model(&mut self, model: Box<dyn CompleterModel>) {
        self.model = model;
        // Clear current completions as model has changed
        self.popup_state.completions.clear();
        self.popup_state.selected_index = -1;
    }

    // =========================================================================
    // Popup Control
    // =========================================================================

    /// Check if the popup is currently visible.
    pub fn is_popup_visible(&self) -> bool {
        self.popup_state.visible
    }

    /// Show the popup with completions for the given prefix.
    ///
    /// # Arguments
    /// * `prefix` - The text to match against
    /// * `anchor_rect` - The rectangle to position the popup relative to
    pub fn show_popup(&mut self, prefix: &str, anchor_rect: Rect) {
        self.update_completions(prefix);

        if self.popup_state.completions.is_empty() {
            self.hide_popup();
            return;
        }

        // Position below the anchor
        self.popup_state.position = Point::new(anchor_rect.origin.x, anchor_rect.bottom());
        self.popup_state.size = Size::new(
            anchor_rect.size.width.max(150.0),
            self.popup_state.calculate_height(),
        );
        self.popup_state.visible = true;
        self.popup_state.selected_index = 0;
        self.popup_state.scroll_offset = 0;

        // Emit highlighted signal for first item
        if !self.popup_state.completions.is_empty() {
            let text = self.popup_state.completions[0].clone();
            self.highlighted.emit(text);
        }
    }

    /// Hide the popup.
    pub fn hide_popup(&mut self) {
        self.popup_state.visible = false;
        self.popup_state.selected_index = -1;
        self.hovered_index = -1;
    }

    /// Update completions for the given prefix.
    pub fn update_completions(&mut self, prefix: &str) {
        if prefix.len() < self.min_chars {
            self.popup_state.completions.clear();
            self.popup_state.current_prefix.clear();
            return;
        }

        self.popup_state.current_prefix = prefix.to_string();
        self.popup_state.completions = self.model.completions(prefix, self.case_sensitivity);
        self.popup_state.selected_index = if self.popup_state.completions.is_empty() {
            -1
        } else {
            0
        };
        self.popup_state.scroll_offset = 0;
        self.popup_state.size.height = self.popup_state.calculate_height();
    }

    /// Complete the text and return the selected completion.
    ///
    /// Returns `None` if no completion is selected.
    pub fn complete(&mut self) -> Option<String> {
        if !self.popup_state.visible || self.popup_state.selected_index < 0 {
            return None;
        }

        let idx = self.popup_state.selected_index as usize;
        if idx < self.popup_state.completions.len() {
            let text = self.popup_state.completions[idx].clone();
            self.hide_popup();
            self.activated.emit(text.clone());
            Some(text)
        } else {
            None
        }
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Move selection up in the completion list.
    pub fn move_up(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        if self.popup_state.selected_index > 0 {
            self.popup_state.selected_index -= 1;
        } else {
            // Wrap to bottom
            self.popup_state.selected_index = self.popup_state.completions.len() as i32 - 1;
        }

        self.popup_state.ensure_selected_visible();
        self.emit_highlighted();
    }

    /// Move selection down in the completion list.
    pub fn move_down(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        let max_idx = self.popup_state.completions.len() as i32 - 1;
        if self.popup_state.selected_index < max_idx {
            self.popup_state.selected_index += 1;
        } else {
            // Wrap to top
            self.popup_state.selected_index = 0;
        }

        self.popup_state.ensure_selected_visible();
        self.emit_highlighted();
    }

    /// Page up in the completion list.
    pub fn page_up(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        let page_size = self.popup_state.max_visible_items as i32;
        self.popup_state.selected_index = (self.popup_state.selected_index - page_size).max(0);

        self.popup_state.ensure_selected_visible();
        self.emit_highlighted();
    }

    /// Page down in the completion list.
    pub fn page_down(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        let page_size = self.popup_state.max_visible_items as i32;
        let max_idx = self.popup_state.completions.len() as i32 - 1;
        self.popup_state.selected_index =
            (self.popup_state.selected_index + page_size).min(max_idx);

        self.popup_state.ensure_selected_visible();
        self.emit_highlighted();
    }

    /// Select the first completion.
    pub fn select_first(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        self.popup_state.selected_index = 0;
        self.popup_state.scroll_offset = 0;
        self.emit_highlighted();
    }

    /// Select the last completion.
    pub fn select_last(&mut self) {
        if !self.popup_state.visible || self.popup_state.completions.is_empty() {
            return;
        }

        self.popup_state.selected_index = self.popup_state.completions.len() as i32 - 1;
        self.popup_state.ensure_selected_visible();
        self.emit_highlighted();
    }

    fn emit_highlighted(&self) {
        if self.popup_state.selected_index >= 0 {
            let idx = self.popup_state.selected_index as usize;
            if idx < self.popup_state.completions.len() {
                self.highlighted
                    .emit(self.popup_state.completions[idx].clone());
            }
        }
    }

    // =========================================================================
    // Mouse Handling
    // =========================================================================

    /// Handle mouse movement over the popup.
    ///
    /// Returns true if the mouse is over the popup.
    pub fn handle_mouse_move(&mut self, pos: Point) -> bool {
        if !self.popup_state.visible {
            return false;
        }

        let popup_rect = Rect::new(
            self.popup_state.position.x,
            self.popup_state.position.y,
            self.popup_state.size.width,
            self.popup_state.size.height,
        );

        if !popup_rect.contains(pos) {
            self.hovered_index = -1;
            return false;
        }

        // Calculate which item is hovered
        let local_y = pos.y - self.popup_state.position.y - 1.0; // -1 for border
        let item_idx = (local_y / self.popup_state.item_height) as usize;
        let actual_idx = item_idx + self.popup_state.scroll_offset;

        if actual_idx < self.popup_state.completions.len() {
            self.hovered_index = actual_idx as i32;
        } else {
            self.hovered_index = -1;
        }

        true
    }

    /// Handle mouse click on the popup.
    ///
    /// Returns `Some(completion)` if an item was clicked.
    pub fn handle_mouse_click(&mut self, pos: Point) -> Option<String> {
        if !self.popup_state.visible {
            return None;
        }

        let popup_rect = Rect::new(
            self.popup_state.position.x,
            self.popup_state.position.y,
            self.popup_state.size.width,
            self.popup_state.size.height,
        );

        if !popup_rect.contains(pos) {
            return None;
        }

        // Calculate which item was clicked
        let local_y = pos.y - self.popup_state.position.y - 1.0;
        let item_idx = (local_y / self.popup_state.item_height) as usize;
        let actual_idx = item_idx + self.popup_state.scroll_offset;

        if actual_idx < self.popup_state.completions.len() {
            let text = self.popup_state.completions[actual_idx].clone();
            self.hide_popup();
            self.activated.emit(text.clone());
            Some(text)
        } else {
            None
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    /// Paint the completer popup.
    ///
    /// This should be called during the widget's paint phase if a completer
    /// is attached.
    pub fn paint(&self, renderer: &mut GpuRenderer) {
        if !self.popup_state.visible {
            return;
        }

        let popup_rect = Rect::new(
            self.popup_state.position.x,
            self.popup_state.position.y,
            self.popup_state.size.width,
            self.popup_state.size.height,
        );

        // Draw background
        renderer.fill_rect(popup_rect, self.background_color);

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        renderer.stroke_rect(popup_rect, &stroke);

        // Get font system for text rendering
        let mut font_system = FontSystem::new();

        // Draw items
        let visible_range = self.popup_state.visible_range();
        for (visual_idx, actual_idx) in visible_range.enumerate() {
            let item = &self.popup_state.completions[actual_idx];

            let item_rect = Rect::new(
                popup_rect.origin.x + 1.0,
                popup_rect.origin.y + 1.0 + (visual_idx as f32) * self.popup_state.item_height,
                popup_rect.size.width - 2.0,
                self.popup_state.item_height,
            );

            // Draw selection/hover background
            let is_selected = actual_idx as i32 == self.popup_state.selected_index;
            let is_hovered = actual_idx as i32 == self.hovered_index && !is_selected;

            if is_selected {
                renderer.fill_rect(item_rect, self.selection_color);
            } else if is_hovered {
                renderer.fill_rect(item_rect, self.hover_color);
            }

            // Draw text
            let layout = TextLayout::with_options(
                &mut font_system,
                item,
                &self.font,
                TextLayoutOptions::new(),
            );

            let text_x = item_rect.origin.x + 4.0;
            let text_y =
                item_rect.origin.y + (self.popup_state.item_height - layout.height()) / 2.0;

            let text_color = if is_selected {
                Color::WHITE
            } else {
                self.text_color
            };

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    text_color,
                );
            }
        }

        // Draw scroll indicator if there are more items
        if self.popup_state.completions.len() > self.popup_state.max_visible_items {
            let indicator_width = 4.0;
            let track_height = popup_rect.size.height - 2.0;
            let thumb_height = (self.popup_state.max_visible_items as f32
                / self.popup_state.completions.len() as f32)
                * track_height;
            let thumb_y = (self.popup_state.scroll_offset as f32
                / (self.popup_state.completions.len() - self.popup_state.max_visible_items) as f32)
                * (track_height - thumb_height);

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

            renderer.fill_rect(track_rect, Color::from_rgb8(240, 240, 240));
            renderer.fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
        }
    }

    // =========================================================================
    // Style Setters
    // =========================================================================

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    /// Set the selection color.
    pub fn set_selection_color(&mut self, color: Color) {
        self.selection_color = color;
    }

    /// Set the border color.
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = color;
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
    }

    /// Set the item height.
    pub fn set_item_height(&mut self, height: f32) {
        self.popup_state.item_height = height.max(16.0);
    }
}

impl std::fmt::Debug for Completer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Completer")
            .field("case_sensitivity", &self.case_sensitivity)
            .field("min_chars", &self.min_chars)
            .field("popup_visible", &self.popup_state.visible)
            .field("completions_count", &self.popup_state.completions.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_list_model_basic() {
        let model = StringListModel::new(vec![
            "apple".to_string(),
            "application".to_string(),
            "banana".to_string(),
            "cherry".to_string(),
        ]);

        // Case insensitive prefix match
        let completions = model.completions("app", CaseSensitivity::CaseInsensitive);
        assert_eq!(completions.len(), 2);
        assert!(completions.contains(&"apple".to_string()));
        assert!(completions.contains(&"application".to_string()));

        // Case sensitive prefix match
        let completions = model.completions("App", CaseSensitivity::CaseSensitive);
        assert_eq!(completions.len(), 0);

        // Empty prefix returns all
        let completions = model.completions("", CaseSensitivity::CaseInsensitive);
        assert_eq!(completions.len(), 4);
    }

    #[test]
    fn test_string_list_model_case_insensitive() {
        let model = StringListModel::new(vec!["Apple".to_string(), "Application".to_string()]);

        let completions = model.completions("app", CaseSensitivity::CaseInsensitive);
        assert_eq!(completions.len(), 2);
    }

    #[test]
    fn test_completer_navigation() {
        let model = StringListModel::new(vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ]);
        let mut completer = Completer::new(Box::new(model));

        // Show popup
        let anchor = Rect::new(0.0, 0.0, 100.0, 30.0);
        completer.show_popup("item", anchor);

        assert!(completer.is_popup_visible());
        assert_eq!(completer.popup_state.selected_index, 0);

        // Move down
        completer.move_down();
        assert_eq!(completer.popup_state.selected_index, 1);

        // Move down again
        completer.move_down();
        assert_eq!(completer.popup_state.selected_index, 2);

        // Move down wraps to top
        completer.move_down();
        assert_eq!(completer.popup_state.selected_index, 0);

        // Move up wraps to bottom
        completer.move_up();
        assert_eq!(completer.popup_state.selected_index, 2);
    }

    #[test]
    fn test_completer_complete() {
        let model = StringListModel::new(vec!["apple".to_string(), "apricot".to_string()]);
        let mut completer = Completer::new(Box::new(model));

        let anchor = Rect::new(0.0, 0.0, 100.0, 30.0);
        completer.show_popup("ap", anchor);

        // Complete with first item selected
        let result = completer.complete();
        assert_eq!(result, Some("apple".to_string()));
        assert!(!completer.is_popup_visible());
    }

    #[test]
    fn test_completer_min_chars() {
        let model = StringListModel::new(vec!["apple".to_string(), "apricot".to_string()]);
        let mut completer = Completer::new(Box::new(model));
        completer.set_min_chars(2);

        let anchor = Rect::new(0.0, 0.0, 100.0, 30.0);

        // Single char doesn't trigger completions
        completer.show_popup("a", anchor);
        assert!(!completer.is_popup_visible());

        // Two chars show completions
        completer.show_popup("ap", anchor);
        assert!(completer.is_popup_visible());
    }

    #[test]
    fn test_completer_hide_popup() {
        let model = StringListModel::new(vec!["test".to_string()]);
        let mut completer = Completer::new(Box::new(model));

        let anchor = Rect::new(0.0, 0.0, 100.0, 30.0);
        completer.show_popup("t", anchor);
        assert!(completer.is_popup_visible());

        completer.hide_popup();
        assert!(!completer.is_popup_visible());
        assert_eq!(completer.popup_state.selected_index, -1);
    }

    #[test]
    fn test_popup_state_visible_range() {
        let mut state = CompleterPopupState::default();
        state.completions = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];
        state.max_visible_items = 3;

        assert_eq!(state.visible_range(), 0..3);

        state.scroll_offset = 2;
        assert_eq!(state.visible_range(), 2..5);
    }

    #[test]
    fn test_popup_state_ensure_selected_visible() {
        let mut state = CompleterPopupState::default();
        state.completions = (0..10).map(|i| format!("item{}", i)).collect();
        state.max_visible_items = 3;
        state.selected_index = 5;

        state.ensure_selected_visible();
        assert_eq!(state.scroll_offset, 3); // 5 - 3 + 1 = 3
    }
}
