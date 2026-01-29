//! Font dialog implementation.
//!
//! This module provides [`FontDialog`], a modal dialog for selecting fonts
//! with family list, style selection, size input, and preview.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::FontDialog;
//! use horizon_lattice_render::Font;
//!
//! // Using static helper
//! let mut dialog = FontDialog::get_font(None, "Select Font");
//! dialog.finished.connect(|result| {
//!     if result.is_accepted() {
//!         // Handle selected font
//!     }
//! });
//! dialog.open();
//!
//! // Using builder pattern
//! let mut dialog = FontDialog::new()
//!     .with_title("Choose Font")
//!     .with_font(Font::new(FontFamily::SansSerif, 14.0))
//!     .with_scalable_fonts_only(true);
//!
//! dialog.font_selected.connect(|font| {
//!     println!("Selected font: {:?}", font);
//! });
//!
//! dialog.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontStyle, FontSystem, FontWeight, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, PaintContext, SizeHint,
    WheelEvent, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;
use super::native_dialogs::{self, NativeFontDesc, NativeFontOptions};

// ============================================================================
// Constants
// ============================================================================

/// Default preview text.
const DEFAULT_PREVIEW_TEXT: &str = "The quick brown fox jumps over the lazy dog. 0123456789";

/// Common font sizes to display in the size list.
const COMMON_SIZES: &[f32] = &[
    8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 32.0, 36.0, 48.0,
    72.0,
];

/// Item height in lists.
const LIST_ITEM_HEIGHT: f32 = 24.0;

/// Maximum visible items in lists.
const MAX_VISIBLE_ITEMS: usize = 10;

// ============================================================================
// FontDialogOptions
// ============================================================================

use std::ops::{BitAnd, BitOr, BitOrAssign};

/// Options that affect FontDialog behavior and appearance.
///
/// These flags can be combined using bitwise OR operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontDialogOptions(u32);

impl FontDialogOptions {
    /// No options.
    pub const NONE: FontDialogOptions = FontDialogOptions(0);

    /// Show only scalable (vector) fonts.
    pub const SCALABLE_FONTS: FontDialogOptions = FontDialogOptions(1 << 0);

    /// Show only monospaced fonts.
    pub const MONOSPACED_FONTS: FontDialogOptions = FontDialogOptions(1 << 1);

    /// Show only proportional (non-monospace) fonts.
    pub const PROPORTIONAL_FONTS: FontDialogOptions = FontDialogOptions(1 << 2);

    /// Don't use native dialog (always use custom).
    pub const DONT_USE_NATIVE: FontDialogOptions = FontDialogOptions(1 << 3);

    /// Check if an option flag is set.
    pub fn contains(&self, option: FontDialogOptions) -> bool {
        (self.0 & option.0) == option.0 && option.0 != 0
    }

    /// Check if no options are set.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Insert an option flag.
    pub fn insert(&mut self, option: FontDialogOptions) {
        self.0 |= option.0;
    }

    /// Remove an option flag.
    pub fn remove(&mut self, option: FontDialogOptions) {
        self.0 &= !option.0;
    }

    /// Create an empty set of options.
    pub fn empty() -> Self {
        Self::NONE
    }
}

impl BitOr for FontDialogOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        FontDialogOptions(self.0 | rhs.0)
    }
}

impl BitOrAssign for FontDialogOptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for FontDialogOptions {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        FontDialogOptions(self.0 & rhs.0)
    }
}

// ============================================================================
// Hit Test Parts
// ============================================================================

/// Identifies which part of the font dialog is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitPart {
    None,
    FamilyList(usize),
    StyleList(usize),
    SizeList(usize),
    SizeInput,
    Preview,
}

// ============================================================================
// FontStyleInfo
// ============================================================================

/// Information about available font styles for a family.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FontStyleInfo {
    /// Display name for the style.
    name: String,
    /// Font weight.
    weight: FontWeight,
    /// Font style.
    style: FontStyle,
}

impl FontStyleInfo {
    fn new(weight: FontWeight, style: FontStyle) -> Self {
        let name = Self::style_name(weight, style);
        Self {
            name,
            weight,
            style,
        }
    }

    fn style_name(weight: FontWeight, style: FontStyle) -> String {
        let weight_str = match weight {
            w if w == FontWeight::THIN => "Thin",
            w if w == FontWeight::EXTRA_LIGHT => "ExtraLight",
            w if w == FontWeight::LIGHT => "Light",
            w if w == FontWeight::NORMAL => "Regular",
            w if w == FontWeight::MEDIUM => "Medium",
            w if w == FontWeight::SEMI_BOLD => "SemiBold",
            w if w == FontWeight::BOLD => "Bold",
            w if w == FontWeight::EXTRA_BOLD => "ExtraBold",
            w if w == FontWeight::BLACK => "Black",
            _ => "Regular",
        };

        match style {
            FontStyle::Normal => weight_str.to_string(),
            FontStyle::Italic => format!("{} Italic", weight_str),
            FontStyle::Oblique => format!("{} Oblique", weight_str),
        }
    }
}

// ============================================================================
// FontDialog
// ============================================================================

/// A modal dialog for selecting fonts.
///
/// FontDialog provides a comprehensive font selection interface including:
///
/// - Font family list
/// - Style selection (Regular, Bold, Italic, etc.)
/// - Size selection with common sizes
/// - Live preview of the selected font
///
/// # Static Helpers
///
/// For common use cases, use the static helper method:
///
/// - [`FontDialog::get_font()`]: Show a dialog to select a font
///
/// # Signals
///
/// - `font_selected(Font)`: Emitted when dialog is accepted with the final font
/// - `current_font_changed(Font)`: Emitted when the font changes during selection
pub struct FontDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// All font families from the system.
    all_families: Vec<String>,

    /// Monospace flags for all families.
    monospace_flags: Vec<bool>,

    /// Filtered family indices (based on options).
    filtered_families: Vec<usize>,

    /// Currently selected family index (into filtered_families).
    selected_family: i32,

    /// Available styles for the current family.
    available_styles: Vec<FontStyleInfo>,

    /// Currently selected style index.
    selected_style: i32,

    /// Currently selected font size.
    font_size: f32,

    /// Size input text for editing.
    size_text: String,

    /// Whether size input is focused.
    size_input_focused: bool,

    /// Cursor position in size input.
    size_cursor_pos: usize,

    /// Preview text.
    preview_text: String,

    /// Dialog options.
    options: FontDialogOptions,

    // Scroll state
    /// Family list scroll offset.
    family_scroll: usize,
    /// Style list scroll offset.
    style_scroll: usize,
    /// Size list scroll offset.
    size_scroll: usize,

    // Layout constants
    /// Padding between elements.
    padding: f32,
    /// Border radius.
    border_radius: f32,
    /// Border color.
    border_color: Color,
    /// Focus border color.
    focus_color: Color,
    /// Selection background color.
    selection_color: Color,
    /// Hover background color.
    hover_color: Color,

    /// Current hover part.
    hover_part: HitPart,

    // Signals
    /// Signal emitted when the dialog is accepted with the selected font.
    pub font_selected: Signal<Font>,

    /// Signal emitted when the font changes during selection.
    pub current_font_changed: Signal<Font>,
}

impl FontDialog {
    /// Create a new font dialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Select Font")
            .with_size(550.0, 450.0)
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        let mut this = Self {
            dialog,
            all_families: Vec::new(),
            monospace_flags: Vec::new(),
            filtered_families: Vec::new(),
            selected_family: -1,
            available_styles: Vec::new(),
            selected_style: -1,
            font_size: 12.0,
            size_text: "12".to_string(),
            size_input_focused: false,
            size_cursor_pos: 2,
            preview_text: DEFAULT_PREVIEW_TEXT.to_string(),
            options: FontDialogOptions::empty(),
            family_scroll: 0,
            style_scroll: 0,
            size_scroll: 0,
            padding: 8.0,
            border_radius: 4.0,
            border_color: Color::from_rgb8(180, 180, 180),
            focus_color: Color::from_rgb8(51, 153, 255),
            selection_color: Color::from_rgba8(51, 153, 255, 255),
            hover_color: Color::from_rgba8(200, 200, 200, 100),
            hover_part: HitPart::None,
            font_selected: Signal::new(),
            current_font_changed: Signal::new(),
        };

        this.load_fonts();
        this
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Create a font dialog to select a font.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial font (None for default)
    /// * `title` - The dialog title
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = FontDialog::get_font(None, "Choose Font");
    /// dialog.font_selected.connect(|font| {
    ///     println!("Selected: {:?}", font);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_font(initial: Option<&Font>, title: impl Into<String>) -> Self {
        let mut dialog = Self::new().with_title(title);
        if let Some(font) = initial {
            dialog.set_font(font.clone());
        }
        dialog
    }

    /// Create a font dialog with options.
    pub fn get_font_with_options(
        initial: Option<&Font>,
        title: impl Into<String>,
        options: FontDialogOptions,
    ) -> Self {
        let mut dialog = Self::new().with_title(title).with_options(options);
        if let Some(font) = initial {
            dialog.set_font(font.clone());
        }
        dialog
    }

    // =========================================================================
    // Font Loading
    // =========================================================================

    fn load_fonts(&mut self) {
        let font_system = FontSystem::new();

        // Get all family names
        self.all_families = font_system.family_names();

        // Determine which are monospace
        self.monospace_flags = self
            .all_families
            .iter()
            .map(|family| {
                font_system
                    .faces()
                    .find(|face| face.families.contains(family))
                    .map(|face| face.monospaced)
                    .unwrap_or(false)
            })
            .collect();

        // Apply filter
        self.apply_filter();

        // Select first family if available
        if !self.filtered_families.is_empty() {
            self.select_family(0);
        }
    }

    fn apply_filter(&mut self) {
        self.filtered_families = self
            .all_families
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let is_mono = self.monospace_flags.get(*i).copied().unwrap_or(false);

                if self.options.contains(FontDialogOptions::MONOSPACED_FONTS) {
                    return is_mono;
                }
                if self.options.contains(FontDialogOptions::PROPORTIONAL_FONTS) {
                    return !is_mono;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if invalid
        if self.selected_family >= 0 {
            let idx = self.selected_family as usize;
            if idx >= self.filtered_families.len() {
                self.selected_family = -1;
                self.available_styles.clear();
                self.selected_style = -1;
            }
        }
    }

    fn load_styles_for_family(&mut self, family: &str) {
        let font_system = FontSystem::new();

        // Collect unique style combinations
        let mut seen_styles: Vec<(FontWeight, FontStyle)> = Vec::new();

        for face in font_system.faces() {
            if face.families.contains(&family.to_string()) {
                let combo = (face.weight, face.style);
                if !seen_styles.contains(&combo) {
                    seen_styles.push(combo);
                }
            }
        }

        // Sort by weight then style
        seen_styles.sort_by(|a, b| {
            a.0.value()
                .cmp(&b.0.value())
                .then_with(|| (a.1 as u8).cmp(&(b.1 as u8)))
        });

        // If no styles found, add default
        if seen_styles.is_empty() {
            seen_styles.push((FontWeight::NORMAL, FontStyle::Normal));
        }

        self.available_styles = seen_styles
            .into_iter()
            .map(|(w, s)| FontStyleInfo::new(w, s))
            .collect();

        // Select first style
        if !self.available_styles.is_empty() {
            self.selected_style = 0;
        }
        self.style_scroll = 0;
    }

    fn select_family(&mut self, display_index: usize) {
        if display_index >= self.filtered_families.len() {
            return;
        }

        let old_family = self.selected_family;
        self.selected_family = display_index as i32;

        if self.selected_family != old_family {
            if let Some(&family_idx) = self.filtered_families.get(display_index)
                && let Some(family) = self.all_families.get(family_idx).cloned()
            {
                self.load_styles_for_family(&family);
            }
            self.emit_font_changed();
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the initial font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.set_font(font);
        self
    }

    /// Set dialog options using builder pattern.
    pub fn with_options(mut self, options: FontDialogOptions) -> Self {
        self.set_options(options);
        self
    }

    /// Set scalable fonts only filter using builder pattern.
    pub fn with_scalable_fonts_only(mut self, only: bool) -> Self {
        if only {
            self.options.insert(FontDialogOptions::SCALABLE_FONTS);
        } else {
            self.options.remove(FontDialogOptions::SCALABLE_FONTS);
        }
        self.apply_filter();
        self
    }

    /// Set monospace fonts only filter using builder pattern.
    pub fn with_monospace_fonts_only(mut self, only: bool) -> Self {
        if only {
            self.options.insert(FontDialogOptions::MONOSPACED_FONTS);
            self.options.remove(FontDialogOptions::PROPORTIONAL_FONTS);
        } else {
            self.options.remove(FontDialogOptions::MONOSPACED_FONTS);
        }
        self.apply_filter();
        self
    }

    /// Set proportional fonts only filter using builder pattern.
    pub fn with_proportional_fonts_only(mut self, only: bool) -> Self {
        if only {
            self.options.insert(FontDialogOptions::PROPORTIONAL_FONTS);
            self.options.remove(FontDialogOptions::MONOSPACED_FONTS);
        } else {
            self.options.remove(FontDialogOptions::PROPORTIONAL_FONTS);
        }
        self.apply_filter();
        self
    }

    /// Set the preview text using builder pattern.
    pub fn with_preview_text(mut self, text: impl Into<String>) -> Self {
        self.preview_text = text.into();
        self
    }

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog = std::mem::take(&mut self.dialog).with_size(width, height);
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the currently selected font.
    pub fn font(&self) -> Option<Font> {
        if self.selected_family < 0 {
            return None;
        }

        let display_idx = self.selected_family as usize;
        let family_idx = self.filtered_families.get(display_idx)?;
        let family_name = self.all_families.get(*family_idx)?;

        let (weight, style) = if self.selected_style >= 0 {
            let style_idx = self.selected_style as usize;
            self.available_styles
                .get(style_idx)
                .map(|s| (s.weight, s.style))
                .unwrap_or((FontWeight::NORMAL, FontStyle::Normal))
        } else {
            (FontWeight::NORMAL, FontStyle::Normal)
        };

        Some(
            Font::builder()
                .family(FontFamily::Name(family_name.clone()))
                .size(self.font_size)
                .weight(weight)
                .style(style)
                .build(),
        )
    }

    /// Set the current font.
    pub fn set_font(&mut self, font: Font) {
        // Set size
        self.font_size = font.size();
        self.size_text = format_size(self.font_size);
        self.size_cursor_pos = self.size_text.len();

        // Find and select family
        if let FontFamily::Name(name) = font.family()
            && let Some(pos) = self.filtered_families.iter().position(|&idx| {
                self.all_families
                    .get(idx)
                    .map(|f| f.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            })
        {
            self.select_family(pos);
            self.ensure_family_visible(pos);

            // Select matching style
            let target_weight = font.weight();
            let target_style = font.style();

            if let Some(style_pos) = self
                .available_styles
                .iter()
                .position(|s| s.weight == target_weight && s.style == target_style)
            {
                self.selected_style = style_pos as i32;
                self.ensure_style_visible(style_pos);
            }
        }

        // Scroll size list to show current size
        self.scroll_size_to_current();

        self.dialog.widget_base_mut().update();
    }

    /// Get the current font options.
    pub fn options(&self) -> FontDialogOptions {
        self.options
    }

    /// Set font dialog options.
    pub fn set_options(&mut self, options: FontDialogOptions) {
        if self.options != options {
            self.options = options;
            self.apply_filter();
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the preview text.
    pub fn preview_text(&self) -> &str {
        &self.preview_text
    }

    /// Set the preview text.
    pub fn set_preview_text(&mut self, text: impl Into<String>) {
        self.preview_text = text.into();
        self.dialog.widget_base_mut().update();
    }

    /// Get the title.
    pub fn title(&self) -> &str {
        self.dialog.title()
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.dialog.set_title(title);
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    /// Get the number of available font families.
    pub fn family_count(&self) -> usize {
        self.filtered_families.len()
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the font dialog (non-blocking modal).
    ///
    /// If the `DONT_USE_NATIVE` option is not set and native font dialogs are available,
    /// a native system font picker will be shown instead.
    pub fn open(&mut self) {
        // Try native dialog if not disabled and available
        if !self.options.contains(FontDialogOptions::DONT_USE_NATIVE)
            && native_dialogs::is_available()
        {
            // Prepare native font options
            let mut native_options = NativeFontOptions::new()
                .title(self.dialog.title())
                .monospace_only(self.options.contains(FontDialogOptions::MONOSPACED_FONTS));

            // Set initial font if we have a selection
            if let Some(font) = self.font() {
                // Extract family name from FontFamily enum
                let family_name = match font.family() {
                    FontFamily::Name(name) => name.clone(),
                    FontFamily::Serif => "Serif".to_string(),
                    FontFamily::SansSerif => "Sans Serif".to_string(),
                    FontFamily::Monospace => "Monospace".to_string(),
                    FontFamily::Cursive => "Cursive".to_string(),
                    FontFamily::Fantasy => "Fantasy".to_string(),
                };
                let desc = NativeFontDesc::new(family_name, font.size())
                    .bold(font.weight() == FontWeight::BOLD || font.weight() == FontWeight::BLACK)
                    .italic(matches!(
                        font.style(),
                        FontStyle::Italic | FontStyle::Oblique
                    ));
                native_options = native_options.initial_font(desc);
            }

            match native_dialogs::pick_font(native_options) {
                Some(native_font) => {
                    // Convert native font to our Font type
                    let weight = if native_font.bold {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    };
                    let style = if native_font.italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    };

                    // Create a font from the native selection using builder pattern
                    let family = FontFamily::Name(native_font.family);
                    let font = Font::builder()
                        .family(family)
                        .size(native_font.size)
                        .weight(weight)
                        .style(style)
                        .build();

                    self.font_selected.emit(font);
                    return;
                }
                None => {
                    // Native dialog not implemented or not available - fall through to custom
                }
            }
        }

        // Use custom dialog
        self.dialog.open();
    }

    /// Accept the dialog and emit selected font.
    pub fn accept(&mut self) {
        if let Some(font) = self.font() {
            self.font_selected.emit(font);
        }
        self.dialog.accept();
    }

    /// Reject the dialog.
    pub fn reject(&mut self) {
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    // =========================================================================
    // Signal Access (delegated from dialog)
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

    fn emit_font_changed(&mut self) {
        if let Some(font) = self.font() {
            self.dialog.widget_base_mut().update();
            self.current_font_changed.emit(font);
        }
    }

    // =========================================================================
    // Layout Calculations
    // =========================================================================

    fn content_rect(&self) -> Rect {
        self.dialog.content_rect()
    }

    /// Family list header rect.
    fn family_header_rect(&self) -> Rect {
        let content = self.content_rect();
        Rect::new(content.left(), content.top(), 200.0, 20.0)
    }

    /// Family list rect.
    fn family_list_rect(&self) -> Rect {
        let header = self.family_header_rect();
        let content = self.content_rect();
        let list_height =
            (MAX_VISIBLE_ITEMS as f32 * LIST_ITEM_HEIGHT).min(content.height() * 0.45);
        Rect::new(header.left(), header.bottom() + 4.0, 200.0, list_height)
    }

    /// Style list header rect.
    fn style_header_rect(&self) -> Rect {
        let family = self.family_list_rect();
        Rect::new(
            family.right() + self.padding,
            family.top() - 24.0,
            150.0,
            20.0,
        )
    }

    /// Style list rect.
    fn style_list_rect(&self) -> Rect {
        let header = self.style_header_rect();
        let family = self.family_list_rect();
        Rect::new(header.left(), header.bottom() + 4.0, 150.0, family.height())
    }

    /// Size header rect.
    fn size_header_rect(&self) -> Rect {
        let style = self.style_list_rect();
        Rect::new(
            style.right() + self.padding,
            style.top() - 24.0,
            100.0,
            20.0,
        )
    }

    /// Size input rect.
    fn size_input_rect(&self) -> Rect {
        let header = self.size_header_rect();
        Rect::new(header.left(), header.bottom() + 4.0, 100.0, 28.0)
    }

    /// Size list rect.
    fn size_list_rect(&self) -> Rect {
        let input = self.size_input_rect();
        let family = self.family_list_rect();
        let remaining = family.bottom() - input.bottom() - 4.0;
        Rect::new(input.left(), input.bottom() + 4.0, 100.0, remaining)
    }

    /// Preview area rect.
    fn preview_rect(&self) -> Rect {
        let family = self.family_list_rect();
        let content = self.content_rect();
        let preview_height = content.bottom() - family.bottom() - self.padding * 2.0 - 20.0;
        Rect::new(
            content.left(),
            family.bottom() + self.padding + 20.0,
            content.width(),
            preview_height.max(60.0),
        )
    }

    /// Preview header rect.
    fn preview_header_rect(&self) -> Rect {
        let preview = self.preview_rect();
        Rect::new(preview.left(), preview.top() - 20.0, 100.0, 20.0)
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> HitPart {
        // Check size input
        if self.size_input_rect().contains(pos) {
            return HitPart::SizeInput;
        }

        // Check family list
        let family_rect = self.family_list_rect();
        if family_rect.contains(pos) {
            let local_y = pos.y - family_rect.top();
            let idx = (local_y / LIST_ITEM_HEIGHT) as usize + self.family_scroll;
            if idx < self.filtered_families.len() {
                return HitPart::FamilyList(idx);
            }
        }

        // Check style list
        let style_rect = self.style_list_rect();
        if style_rect.contains(pos) {
            let local_y = pos.y - style_rect.top();
            let idx = (local_y / LIST_ITEM_HEIGHT) as usize + self.style_scroll;
            if idx < self.available_styles.len() {
                return HitPart::StyleList(idx);
            }
        }

        // Check size list
        let size_rect = self.size_list_rect();
        if size_rect.contains(pos) {
            let local_y = pos.y - size_rect.top();
            let idx = (local_y / LIST_ITEM_HEIGHT) as usize + self.size_scroll;
            if idx < COMMON_SIZES.len() {
                return HitPart::SizeList(idx);
            }
        }

        // Check preview
        if self.preview_rect().contains(pos) {
            return HitPart::Preview;
        }

        HitPart::None
    }

    // =========================================================================
    // Scroll Helpers
    // =========================================================================

    fn visible_family_count(&self) -> usize {
        let rect = self.family_list_rect();
        (rect.height() / LIST_ITEM_HEIGHT) as usize
    }

    fn visible_style_count(&self) -> usize {
        let rect = self.style_list_rect();
        (rect.height() / LIST_ITEM_HEIGHT) as usize
    }

    fn visible_size_count(&self) -> usize {
        let rect = self.size_list_rect();
        (rect.height() / LIST_ITEM_HEIGHT) as usize
    }

    fn ensure_family_visible(&mut self, idx: usize) {
        let visible = self.visible_family_count();
        if idx < self.family_scroll {
            self.family_scroll = idx;
        } else if idx >= self.family_scroll + visible {
            self.family_scroll = idx.saturating_sub(visible) + 1;
        }
    }

    fn ensure_style_visible(&mut self, idx: usize) {
        let visible = self.visible_style_count();
        if idx < self.style_scroll {
            self.style_scroll = idx;
        } else if idx >= self.style_scroll + visible {
            self.style_scroll = idx.saturating_sub(visible) + 1;
        }
    }

    fn scroll_size_to_current(&mut self) {
        // Find closest size
        if let Some(pos) = COMMON_SIZES
            .iter()
            .position(|&s| (s - self.font_size).abs() < 0.5)
        {
            let visible = self.visible_size_count();
            if pos < self.size_scroll {
                self.size_scroll = pos;
            } else if pos >= self.size_scroll + visible {
                self.size_scroll = pos.saturating_sub(visible) + 1;
            }
        }
    }

    // =========================================================================
    // Size Input
    // =========================================================================

    fn apply_size_text(&mut self) {
        if let Ok(size) = self.size_text.parse::<f32>() {
            let new_size = size.clamp(1.0, 500.0);
            if (new_size - self.font_size).abs() > 0.01 {
                self.font_size = new_size;
                self.emit_font_changed();
            }
        }
        self.size_text = format_size(self.font_size);
        self.size_cursor_pos = self.size_text.len();
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let part = self.hit_test(pos);

        // Handle size input focus
        if part == HitPart::SizeInput {
            self.size_input_focused = true;
            self.dialog.widget_base_mut().update();
            return true;
        }

        // Unfocus size input when clicking elsewhere
        if self.size_input_focused {
            self.apply_size_text();
            self.size_input_focused = false;
        }

        match part {
            HitPart::FamilyList(idx) => {
                self.select_family(idx);
                self.ensure_family_visible(idx);
                true
            }
            HitPart::StyleList(idx) => {
                if idx < self.available_styles.len() {
                    self.selected_style = idx as i32;
                    self.emit_font_changed();
                }
                true
            }
            HitPart::SizeList(idx) => {
                if let Some(&size) = COMMON_SIZES.get(idx) {
                    self.font_size = size;
                    self.size_text = format_size(size);
                    self.size_cursor_pos = self.size_text.len();
                    self.emit_font_changed();
                }
                true
            }
            _ => false,
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let part = self.hit_test(event.local_pos);
        if part != self.hover_part {
            self.hover_part = part;
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Size input handling
        if self.size_input_focused {
            return self.handle_size_input_key(event);
        }

        match event.key {
            Key::Enter if !event.is_repeat => {
                self.accept();
                true
            }
            Key::ArrowDown => {
                if self.selected_family < self.filtered_families.len() as i32 - 1 {
                    self.select_family((self.selected_family + 1) as usize);
                    self.ensure_family_visible(self.selected_family as usize);
                }
                true
            }
            Key::ArrowUp => {
                if self.selected_family > 0 {
                    self.select_family((self.selected_family - 1) as usize);
                    self.ensure_family_visible(self.selected_family as usize);
                }
                true
            }
            _ => false,
        }
    }

    fn handle_size_input_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Escape => {
                self.size_text = format_size(self.font_size);
                self.size_input_focused = false;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::Enter => {
                self.apply_size_text();
                self.size_input_focused = false;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::Backspace => {
                if self.size_cursor_pos > 0 {
                    self.size_text.remove(self.size_cursor_pos - 1);
                    self.size_cursor_pos -= 1;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::Delete => {
                if self.size_cursor_pos < self.size_text.len() {
                    self.size_text.remove(self.size_cursor_pos);
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::ArrowLeft => {
                if self.size_cursor_pos > 0 {
                    self.size_cursor_pos -= 1;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::ArrowRight => {
                if self.size_cursor_pos < self.size_text.len() {
                    self.size_cursor_pos += 1;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::Home => {
                self.size_cursor_pos = 0;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::End => {
                self.size_cursor_pos = self.size_text.len();
                self.dialog.widget_base_mut().update();
                true
            }
            _ => {
                // Character input
                if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
                    for c in event.text.chars() {
                        if c.is_ascii_digit() || c == '.' {
                            self.size_text.insert(self.size_cursor_pos, c);
                            self.size_cursor_pos += 1;
                        }
                    }
                    self.dialog.widget_base_mut().update();
                    return true;
                }
                false
            }
        }
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let part = self.hit_test(event.local_pos);
        let delta = if event.delta_y > 0.0 { -3i32 } else { 3 };

        match part {
            HitPart::FamilyList(_) => {
                let max_scroll = self
                    .filtered_families
                    .len()
                    .saturating_sub(self.visible_family_count());
                let new_scroll = (self.family_scroll as i32 + delta)
                    .max(0)
                    .min(max_scroll as i32) as usize;
                if new_scroll != self.family_scroll {
                    self.family_scroll = new_scroll;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            HitPart::StyleList(_) => {
                let max_scroll = self
                    .available_styles
                    .len()
                    .saturating_sub(self.visible_style_count());
                let new_scroll = (self.style_scroll as i32 + delta)
                    .max(0)
                    .min(max_scroll as i32) as usize;
                if new_scroll != self.style_scroll {
                    self.style_scroll = new_scroll;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            HitPart::SizeList(_) => {
                let max_scroll = COMMON_SIZES.len().saturating_sub(self.visible_size_count());
                let new_scroll = (self.size_scroll as i32 + delta)
                    .max(0)
                    .min(max_scroll as i32) as usize;
                if new_scroll != self.size_scroll {
                    self.size_scroll = new_scroll;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            _ => false,
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_label(&self, _ctx: &mut PaintContext<'_>, text: &str, rect: Rect) {
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 12.0);
        let layout =
            TextLayout::with_options(&mut font_system, text, &font, TextLayoutOptions::new());

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(rect.left(), rect.top() + 2.0),
                Color::from_rgb8(80, 80, 80),
            );
        }
    }

    fn paint_list_background(&self, ctx: &mut PaintContext<'_>, rect: Rect, focused: bool) {
        let rounded = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().fill_rounded_rect(rounded, Color::WHITE);

        let border_color = if focused {
            self.focus_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);
    }

    fn paint_family_list(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.family_list_rect();
        self.paint_list_background(ctx, rect, false);

        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let visible_count = self.visible_family_count();

        for i in 0..visible_count {
            let idx = self.family_scroll + i;
            if idx >= self.filtered_families.len() {
                break;
            }

            let family_idx = self.filtered_families[idx];
            let family_name = match self.all_families.get(family_idx) {
                Some(name) => name.as_str(),
                None => continue,
            };

            let item_rect = Rect::new(
                rect.left() + 1.0,
                rect.top() + 1.0 + (i as f32) * LIST_ITEM_HEIGHT,
                rect.width() - 2.0,
                LIST_ITEM_HEIGHT,
            );

            let is_selected = idx as i32 == self.selected_family;
            let is_hovered = matches!(self.hover_part, HitPart::FamilyList(h) if h == idx);

            // Background
            if is_selected {
                ctx.renderer().fill_rect(item_rect, self.selection_color);
            } else if is_hovered {
                ctx.renderer().fill_rect(item_rect, self.hover_color);
            }

            let text_color = if is_selected {
                Color::WHITE
            } else {
                Color::BLACK
            };

            let layout = TextLayout::with_options(
                &mut font_system,
                family_name,
                &font,
                TextLayoutOptions::new(),
            );

            let text_x = item_rect.left() + 4.0;
            let text_y = item_rect.top() + (item_rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    text_color,
                );
            }
        }

        // Scrollbar indicator
        if self.filtered_families.len() > visible_count {
            self.paint_scrollbar(
                ctx,
                rect,
                self.family_scroll,
                self.filtered_families.len(),
                visible_count,
            );
        }
    }

    fn paint_style_list(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.style_list_rect();
        self.paint_list_background(ctx, rect, false);

        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let visible_count = self.visible_style_count();

        for i in 0..visible_count {
            let idx = self.style_scroll + i;
            if idx >= self.available_styles.len() {
                break;
            }

            let style_info = &self.available_styles[idx];

            let item_rect = Rect::new(
                rect.left() + 1.0,
                rect.top() + 1.0 + (i as f32) * LIST_ITEM_HEIGHT,
                rect.width() - 2.0,
                LIST_ITEM_HEIGHT,
            );

            let is_selected = idx as i32 == self.selected_style;
            let is_hovered = matches!(self.hover_part, HitPart::StyleList(h) if h == idx);

            if is_selected {
                ctx.renderer().fill_rect(item_rect, self.selection_color);
            } else if is_hovered {
                ctx.renderer().fill_rect(item_rect, self.hover_color);
            }

            let text_color = if is_selected {
                Color::WHITE
            } else {
                Color::BLACK
            };

            let layout = TextLayout::with_options(
                &mut font_system,
                &style_info.name,
                &font,
                TextLayoutOptions::new(),
            );

            let text_x = item_rect.left() + 4.0;
            let text_y = item_rect.top() + (item_rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    text_color,
                );
            }
        }

        if self.available_styles.len() > visible_count {
            self.paint_scrollbar(
                ctx,
                rect,
                self.style_scroll,
                self.available_styles.len(),
                visible_count,
            );
        }
    }

    fn paint_size_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.size_input_rect();

        // Background
        let bg_color = if self.size_input_focused {
            Color::WHITE
        } else {
            Color::from_rgb8(250, 250, 250)
        };
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(rect, self.border_radius), bg_color);

        // Border
        let border_color = if self.size_input_focused {
            self.focus_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(rect, self.border_radius), &stroke);

        // Text
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout = TextLayout::with_options(
            &mut font_system,
            &self.size_text,
            &font,
            TextLayoutOptions::new(),
        );

        let text_x = rect.left() + 6.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                Color::BLACK,
            );
        }

        // Cursor
        if self.size_input_focused {
            let char_width = 8.0;
            let cursor_x = text_x + self.size_cursor_pos as f32 * char_width;
            ctx.renderer().fill_rect(
                Rect::new(cursor_x, text_y, 1.0, layout.height()),
                Color::BLACK,
            );
        }
    }

    fn paint_size_list(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.size_list_rect();
        self.paint_list_background(ctx, rect, false);

        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let visible_count = self.visible_size_count();

        for i in 0..visible_count {
            let idx = self.size_scroll + i;
            if idx >= COMMON_SIZES.len() {
                break;
            }

            let size = COMMON_SIZES[idx];
            let size_str = format_size(size);

            let item_rect = Rect::new(
                rect.left() + 1.0,
                rect.top() + 1.0 + (i as f32) * LIST_ITEM_HEIGHT,
                rect.width() - 2.0,
                LIST_ITEM_HEIGHT,
            );

            let is_selected = (size - self.font_size).abs() < 0.5;
            let is_hovered = matches!(self.hover_part, HitPart::SizeList(h) if h == idx);

            if is_selected {
                ctx.renderer().fill_rect(item_rect, self.selection_color);
            } else if is_hovered {
                ctx.renderer().fill_rect(item_rect, self.hover_color);
            }

            let text_color = if is_selected {
                Color::WHITE
            } else {
                Color::BLACK
            };

            let layout = TextLayout::with_options(
                &mut font_system,
                &size_str,
                &font,
                TextLayoutOptions::new(),
            );

            let text_x = item_rect.left() + 4.0;
            let text_y = item_rect.top() + (item_rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    text_color,
                );
            }
        }

        if COMMON_SIZES.len() > visible_count {
            self.paint_scrollbar(
                ctx,
                rect,
                self.size_scroll,
                COMMON_SIZES.len(),
                visible_count,
            );
        }
    }

    fn paint_scrollbar(
        &self,
        ctx: &mut PaintContext<'_>,
        list_rect: Rect,
        scroll: usize,
        total: usize,
        visible: usize,
    ) {
        let bar_width = 4.0;
        let track_height = list_rect.height() - 4.0;
        let thumb_height = (visible as f32 / total as f32) * track_height;
        let max_scroll = total.saturating_sub(visible);
        let thumb_y = if max_scroll > 0 {
            (scroll as f32 / max_scroll as f32) * (track_height - thumb_height)
        } else {
            0.0
        };

        let track_rect = Rect::new(
            list_rect.right() - bar_width - 2.0,
            list_rect.top() + 2.0,
            bar_width,
            track_height,
        );

        let thumb_rect = Rect::new(
            track_rect.left(),
            track_rect.top() + thumb_y,
            bar_width,
            thumb_height.max(10.0),
        );

        ctx.renderer()
            .fill_rect(track_rect, Color::from_rgb8(240, 240, 240));
        ctx.renderer()
            .fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
    }

    fn paint_preview(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.preview_rect();

        // Background
        let rounded = RoundedRect::new(rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(rounded, Color::from_rgb8(250, 250, 250));

        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);

        // Preview text
        if let Some(font) = self.font() {
            let mut font_system = FontSystem::new();

            // Clamp preview font size for readability
            let preview_size = font.size().clamp(12.0, 36.0);
            let preview_font = font.with_size(preview_size);

            let layout = TextLayout::with_options(
                &mut font_system,
                &self.preview_text,
                &preview_font,
                TextLayoutOptions::new(),
            );

            let text_x = rect.left() + 8.0;
            let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y.max(rect.top() + 4.0)),
                    Color::BLACK,
                );
            }
        }
    }
}

impl Default for FontDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for FontDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for FontDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.dialog.size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint labels
        self.paint_label(ctx, "Family:", self.family_header_rect());
        self.paint_label(ctx, "Style:", self.style_header_rect());
        self.paint_label(ctx, "Size:", self.size_header_rect());
        self.paint_label(ctx, "Preview:", self.preview_header_rect());

        // Paint lists and preview
        self.paint_family_list(ctx);
        self.paint_style_list(ctx);
        self.paint_size_input(ctx);
        self.paint_size_list(ctx);
        self.paint_preview(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Wheel(e) => self.handle_wheel(e),
            _ => false,
        };

        if handled {
            event.accept();
            return true;
        }

        // Delegate to dialog
        self.dialog.event(event)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_size(size: f32) -> String {
    if size.fract() < 0.01 {
        format!("{}", size as i32)
    } else {
        format!("{:.1}", size)
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(FontDialog: Send, Sync);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::Arc;

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_font_dialog_creation() {
        setup();
        let dialog = FontDialog::new();
        assert!(!dialog.is_open());
        assert_eq!(dialog.title(), "Select Font");
    }

    #[test]
    fn test_font_dialog_builder() {
        setup();
        let dialog = FontDialog::new()
            .with_title("Test Font")
            .with_monospace_fonts_only(true)
            .with_preview_text("Sample text");

        assert_eq!(dialog.title(), "Test Font");
        assert!(
            dialog
                .options()
                .contains(FontDialogOptions::MONOSPACED_FONTS)
        );
        assert_eq!(dialog.preview_text(), "Sample text");
    }

    #[test]
    fn test_get_font_helper() {
        setup();
        let dialog = FontDialog::get_font(None, "Select Font");
        assert_eq!(dialog.title(), "Select Font");
    }

    #[test]
    fn test_font_dialog_set_font() {
        setup();
        let mut dialog = FontDialog::new();
        let font = Font::builder()
            .family(FontFamily::SansSerif)
            .size(18.0)
            .weight(FontWeight::BOLD)
            .build();

        dialog.set_font(font);
        // Font size should be updated
        assert!((dialog.font_size - 18.0).abs() < 0.1);
    }

    #[test]
    #[ignore = "requires desktop environment, hangs on Windows CI"]
    fn test_dialog_lifecycle() {
        setup();
        let mut dialog = FontDialog::new();
        assert!(!dialog.is_open());

        dialog.open();
        assert!(dialog.is_open());

        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    #[ignore = "requires desktop environment, hangs on Windows CI"]
    fn test_font_selected_signal() {
        setup();
        let mut dialog = FontDialog::new();

        let selected = Arc::new(std::sync::Mutex::new(false));
        let selected_clone = selected.clone();

        dialog.font_selected.connect(move |_font| {
            *selected_clone.lock().unwrap() = true;
        });

        dialog.open();

        // Select a family if available
        if dialog.family_count() > 0 {
            dialog.select_family(0);
            dialog.accept();

            let was_selected = *selected.lock().unwrap();
            assert!(was_selected);
        }
    }

    #[test]
    fn test_font_dialog_options() {
        setup();
        let dialog = FontDialog::new()
            .with_options(FontDialogOptions::SCALABLE_FONTS | FontDialogOptions::MONOSPACED_FONTS);

        assert!(dialog.options().contains(FontDialogOptions::SCALABLE_FONTS));
        assert!(
            dialog
                .options()
                .contains(FontDialogOptions::MONOSPACED_FONTS)
        );
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(12.0), "12");
        assert_eq!(format_size(12.5), "12.5");
        assert_eq!(format_size(12.35), "12.4");
    }
}
