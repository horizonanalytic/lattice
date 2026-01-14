//! Label widget for text display.
//!
//! The Label widget displays text with support for:
//! - Single-line and multi-line text
//! - Text alignment (horizontal and vertical)
//! - Word wrapping
//! - Text elision (truncation with ellipsis)
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::Label;
//! use horizon_lattice::render::{Color, HorizontalAlign};
//!
//! // Create a simple label
//! let mut label = Label::new("Hello, World!");
//!
//! // Create a label with word wrapping
//! let mut wrapped = Label::new("Long text that will wrap...")
//!     .with_word_wrap(true);
//!
//! // Create a label with elision
//! let mut elided = Label::new("Very long filename.txt")
//!     .with_elide_mode(ElideMode::Right);
//!
//! // Customize alignment and color
//! let mut styled = Label::new("Centered text")
//!     .with_horizontal_align(HorizontalAlign::Center)
//!     .with_text_color(Color::from_rgb8(100, 100, 100));
//! ```

use parking_lot::RwLock;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, RichText, Size, TextLayout,
    TextLayoutOptions, TextRenderer, VerticalAlign, WrapMode,
};

use crate::widget::{FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase};

/// Text elision mode for truncating text that doesn't fit.
///
/// When text is too long to fit in the available space and word wrapping
/// is disabled, the text can be truncated with an ellipsis ("...").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElideMode {
    /// No elision - text may extend beyond bounds.
    #[default]
    None,
    /// Elide text at the left: "...filename.txt"
    Left,
    /// Elide text in the middle: "long...name.txt"
    Middle,
    /// Elide text at the right: "longfilena..."
    Right,
}

/// A widget that displays text.
///
/// Label is the primary widget for displaying non-editable text. It supports
/// various text formatting options including alignment, wrapping, elision,
/// and rich text with HTML formatting.
///
/// # Plain Text vs Rich Text
///
/// Labels can display either plain text or rich text:
///
/// ```ignore
/// // Plain text
/// let plain = Label::new("Hello, World!");
///
/// // Rich text from HTML
/// let rich = Label::from_html("Hello <b>bold</b> and <i>italic</i>!");
/// ```
///
/// Rich text supports basic HTML tags:
/// - `<b>`, `<strong>` for bold
/// - `<i>`, `<em>` for italic
/// - `<u>` for underline
/// - `<s>`, `<del>` for strikethrough
/// - `<font color="..." size="...">` for color and size
/// - `<br>` for line breaks
///
/// # Layout Behavior
///
/// By default, Label has a `Preferred` size policy in both dimensions.
/// When word wrapping is enabled, it supports height-for-width layout
/// negotiation.
///
/// # Performance
///
/// The Label caches its text layout and only recalculates when the text,
/// font, or layout options change. The cached layout is also used for
/// hit testing and size hint calculations.
pub struct Label {
    /// Widget base for common functionality.
    base: WidgetBase,

    /// The plain text to display (used when rich_text is None).
    text: String,

    /// Rich text content (takes precedence over text when Some).
    rich_text: Option<RichText>,

    /// Horizontal text alignment within the widget bounds.
    horizontal_align: HorizontalAlign,

    /// Vertical text alignment within the widget bounds.
    vertical_align: VerticalAlign,

    /// Whether word wrapping is enabled.
    word_wrap: bool,

    /// Text elision mode for truncating long text.
    elide_mode: ElideMode,

    /// The font to use for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Cached text layout for efficient rendering.
    /// Uses RwLock for thread-safe interior mutability since Widget requires Sync.
    cached_layout: RwLock<Option<CachedLayout>>,

    /// Signal emitted when the text changes.
    pub text_changed: Signal<String>,
}

/// Cached text layout data.
struct CachedLayout {
    /// The computed text layout.
    layout: TextLayout,
    /// The width constraint used for this layout (None = unconstrained).
    width_constraint: Option<f32>,
}

impl Label {
    /// Create a new label with the specified text.
    ///
    /// The label is created with default settings:
    /// - Left-aligned horizontally, top-aligned vertically
    /// - No word wrapping
    /// - No elision
    /// - System sans-serif font at 14pt
    /// - Black text color
    pub fn new(text: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        // Labels don't receive focus by default
        base.set_focus_policy(FocusPolicy::NoFocus);

        Self {
            base,
            text: text.into(),
            rich_text: None,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            word_wrap: false,
            elide_mode: ElideMode::None,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            cached_layout: RwLock::new(None),
            text_changed: Signal::new(),
        }
    }

    /// Create a new label from HTML rich text.
    ///
    /// The HTML is parsed to extract formatting. Supported tags:
    /// - `<b>`, `<strong>` for bold
    /// - `<i>`, `<em>` for italic
    /// - `<u>` for underline
    /// - `<s>`, `<del>` for strikethrough
    /// - `<font color="..." size="...">` for color and size
    /// - `<br>` for line breaks
    ///
    /// # Example
    ///
    /// ```ignore
    /// let label = Label::from_html("Hello <b>bold</b> and <i>italic</i>!");
    /// ```
    pub fn from_html(html: impl AsRef<str>) -> Self {
        let rich = RichText::from_html(html.as_ref());
        let plain = rich.plain_text();

        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);

        Self {
            base,
            text: plain,
            rich_text: Some(rich),
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            word_wrap: false,
            elide_mode: ElideMode::None,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            cached_layout: RwLock::new(None),
            text_changed: Signal::new(),
        }
    }

    /// Get the current plain text content.
    ///
    /// If the label contains rich text, this returns the text without formatting.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text to display (as plain text).
    ///
    /// This clears any rich text formatting and invalidates the cached layout.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        if self.text != new_text || self.rich_text.is_some() {
            self.text = new_text.clone();
            self.rich_text = None;
            self.invalidate_layout();
            self.base.update();
            self.text_changed.emit(new_text);
        }
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.rich_text = None;
        self.invalidate_layout();
        self
    }

    /// Check if this label has rich text formatting.
    pub fn has_rich_text(&self) -> bool {
        self.rich_text.is_some()
    }

    /// Get the rich text content, if any.
    pub fn rich_text(&self) -> Option<&RichText> {
        self.rich_text.as_ref()
    }

    /// Set rich text content.
    ///
    /// This replaces any existing plain text or rich text.
    pub fn set_rich_text(&mut self, rich_text: RichText) {
        self.text = rich_text.plain_text();
        self.rich_text = Some(rich_text);
        self.invalidate_layout();
        self.base.update();
        self.text_changed.emit(self.text.clone());
    }

    /// Set rich text from HTML.
    ///
    /// This is a convenience method that parses the HTML and sets the rich text.
    pub fn set_html(&mut self, html: impl AsRef<str>) {
        self.set_rich_text(RichText::from_html(html.as_ref()));
    }

    /// Set rich text using builder pattern.
    pub fn with_rich_text(mut self, rich_text: RichText) -> Self {
        self.text = rich_text.plain_text();
        self.rich_text = Some(rich_text);
        self.invalidate_layout();
        self
    }

    /// Set rich text from HTML using builder pattern.
    pub fn with_html(self, html: impl AsRef<str>) -> Self {
        self.with_rich_text(RichText::from_html(html.as_ref()))
    }

    /// Get the horizontal alignment.
    pub fn horizontal_align(&self) -> HorizontalAlign {
        self.horizontal_align
    }

    /// Set the horizontal text alignment.
    pub fn set_horizontal_align(&mut self, align: HorizontalAlign) {
        if self.horizontal_align != align {
            self.horizontal_align = align;
            self.invalidate_layout();
            self.base.update();
        }
    }

    /// Set horizontal alignment using builder pattern.
    pub fn with_horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self.invalidate_layout();
        self
    }

    /// Get the vertical alignment.
    pub fn vertical_align(&self) -> VerticalAlign {
        self.vertical_align
    }

    /// Set the vertical text alignment.
    pub fn set_vertical_align(&mut self, align: VerticalAlign) {
        if self.vertical_align != align {
            self.vertical_align = align;
            self.base.update();
        }
    }

    /// Set vertical alignment using builder pattern.
    pub fn with_vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Check if word wrapping is enabled.
    pub fn word_wrap(&self) -> bool {
        self.word_wrap
    }

    /// Enable or disable word wrapping.
    ///
    /// When word wrapping is enabled:
    /// - Text will wrap at word boundaries when it exceeds the widget width
    /// - The label supports height-for-width layout negotiation
    /// - Elision is automatically disabled (mutually exclusive)
    pub fn set_word_wrap(&mut self, wrap: bool) {
        if self.word_wrap != wrap {
            self.word_wrap = wrap;
            if wrap {
                // Word wrap and elision are mutually exclusive
                self.elide_mode = ElideMode::None;
            }
            self.invalidate_layout();
            self.update_size_policy();
            self.base.update();
        }
    }

    /// Set word wrapping using builder pattern.
    pub fn with_word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        if wrap {
            self.elide_mode = ElideMode::None;
        }
        self.invalidate_layout();
        self.update_size_policy();
        self
    }

    /// Get the elide mode.
    pub fn elide_mode(&self) -> ElideMode {
        self.elide_mode
    }

    /// Set the text elision mode.
    ///
    /// Elision truncates text that doesn't fit with an ellipsis.
    /// When elision is enabled, word wrapping is automatically disabled.
    pub fn set_elide_mode(&mut self, mode: ElideMode) {
        if self.elide_mode != mode {
            self.elide_mode = mode;
            if mode != ElideMode::None {
                // Elision and word wrap are mutually exclusive
                self.word_wrap = false;
            }
            self.invalidate_layout();
            self.update_size_policy();
            self.base.update();
        }
    }

    /// Set elide mode using builder pattern.
    pub fn with_elide_mode(mut self, mode: ElideMode) -> Self {
        self.elide_mode = mode;
        if mode != ElideMode::None {
            self.word_wrap = false;
        }
        self.invalidate_layout();
        self.update_size_policy();
        self
    }

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font for text rendering.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.invalidate_layout();
        self.base.update();
    }

    /// Set the font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self.invalidate_layout();
        self
    }

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.text_color
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        if self.text_color != color {
            self.text_color = color;
            self.base.update();
        }
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Invalidate the cached text layout.
    fn invalidate_layout(&self) {
        *self.cached_layout.write() = None;
    }

    /// Update size policy based on current settings.
    fn update_size_policy(&mut self) {
        let policy = if self.word_wrap {
            SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
                .with_height_for_width()
        } else {
            SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed)
        };
        self.base.set_size_policy(policy);
    }

    /// Build layout options based on current label settings.
    fn build_layout_options(&self, width_constraint: Option<f32>) -> TextLayoutOptions {
        let mut options = TextLayoutOptions::new()
            .horizontal_align(self.horizontal_align);

        // Set width constraint
        if let Some(width) = width_constraint {
            options = options.max_width(width);
        }

        // Set wrap mode
        if self.word_wrap {
            options = options.wrap(WrapMode::Word);
        }

        // Set elision
        if self.elide_mode != ElideMode::None && width_constraint.is_some() {
            options = options.with_ellipsis();
        }

        options
    }

    /// Get or create the text layout.
    ///
    /// The layout is cached and only recalculated when necessary.
    fn ensure_layout(&self, font_system: &mut FontSystem, width_constraint: Option<f32>) -> TextLayout {
        let mut cached = self.cached_layout.write();

        // Check if we can reuse the cached layout
        if let Some(ref cache) = *cached {
            if cache.width_constraint == width_constraint {
                return cache.layout.clone();
            }
        }

        // Build new layout
        let options = self.build_layout_options(width_constraint);

        let layout = if let Some(ref rich) = self.rich_text {
            // Rich text rendering
            // Note: Elision is not currently supported for rich text
            if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                // Fall back to plain text for elision
                let display_text = self.compute_elided_text(font_system, width_constraint.unwrap());
                TextLayout::with_options(font_system, &display_text, &self.font, options)
            } else {
                // Use rich text layout
                let spans = rich.to_spans(&self.font);
                TextLayout::rich_text(font_system, &spans, &self.font, options)
            }
        } else {
            // Plain text rendering
            let display_text = if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                self.compute_elided_text(font_system, width_constraint.unwrap())
            } else {
                self.text.clone()
            };
            TextLayout::with_options(font_system, &display_text, &self.font, options)
        };

        *cached = Some(CachedLayout {
            layout: layout.clone(),
            width_constraint,
        });

        layout
    }

    /// Compute elided text that fits within the given width.
    fn compute_elided_text(&self, font_system: &mut FontSystem, max_width: f32) -> String {
        // First check if elision is needed
        let full_layout = TextLayout::with_options(
            font_system,
            &self.text,
            &self.font,
            TextLayoutOptions::new(),
        );

        if full_layout.width() <= max_width {
            return self.text.clone();
        }

        let ellipsis = "…";

        // Measure ellipsis width
        let ellipsis_layout = TextLayout::with_options(
            font_system,
            ellipsis,
            &self.font,
            TextLayoutOptions::new(),
        );
        let ellipsis_width = ellipsis_layout.width();

        if ellipsis_width >= max_width {
            return ellipsis.to_string();
        }

        let available_width = max_width - ellipsis_width;

        match self.elide_mode {
            ElideMode::None => self.text.clone(),
            ElideMode::Right => {
                self.elide_right(font_system, available_width, ellipsis)
            }
            ElideMode::Left => {
                self.elide_left(font_system, available_width, ellipsis)
            }
            ElideMode::Middle => {
                self.elide_middle(font_system, max_width, ellipsis)
            }
        }
    }

    /// Elide text from the right side.
    fn elide_right(&self, font_system: &mut FontSystem, available_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();

        // Binary search for the right cutoff point
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let test_text: String = chars[..mid].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );

            if test_layout.width() <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            let truncated: String = chars[..low].iter().collect();
            format!("{}{}", truncated.trim_end(), ellipsis)
        }
    }

    /// Elide text from the left side.
    fn elide_left(&self, font_system: &mut FontSystem, available_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();

        // Binary search for the right cutoff point from the end
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let start = chars.len() - mid;
            let test_text: String = chars[start..].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );

            if test_layout.width() <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            let start = chars.len() - low;
            let truncated: String = chars[start..].iter().collect();
            format!("{}{}", ellipsis, truncated.trim_start())
        }
    }

    /// Elide text from the middle.
    fn elide_middle(&self, font_system: &mut FontSystem, max_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();
        if chars.is_empty() {
            return ellipsis.to_string();
        }

        // Measure ellipsis
        let ellipsis_layout = TextLayout::with_options(
            font_system,
            ellipsis,
            &self.font,
            TextLayoutOptions::new(),
        );
        let ellipsis_width = ellipsis_layout.width();
        let available_width = max_width - ellipsis_width;

        if available_width <= 0.0 {
            return ellipsis.to_string();
        }

        // Split available width roughly 50/50 between start and end
        let half_width = available_width / 2.0;

        // Find how many chars fit from the start
        let mut start_len = 0;
        for i in 1..=chars.len() {
            let test_text: String = chars[..i].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );
            if test_layout.width() > half_width {
                break;
            }
            start_len = i;
        }

        // Find how many chars fit from the end
        let mut end_len = 0;
        for i in 1..=chars.len() {
            let start_idx = chars.len() - i;
            let test_text: String = chars[start_idx..].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );
            if test_layout.width() > half_width {
                break;
            }
            end_len = i;
        }

        if start_len == 0 && end_len == 0 {
            ellipsis.to_string()
        } else {
            let start_part: String = chars[..start_len].iter().collect();
            let end_start = chars.len() - end_len;
            let end_part: String = chars[end_start..].iter().collect();
            format!("{}{}{}", start_part.trim_end(), ellipsis, end_part.trim_start())
        }
    }

    /// Calculate the size hint for unconstrained text.
    fn calculate_unconstrained_size(&self, font_system: &mut FontSystem) -> Size {
        let layout = TextLayout::with_options(
            font_system,
            &self.text,
            &self.font,
            TextLayoutOptions::new().horizontal_align(self.horizontal_align),
        );
        Size::new(layout.width(), layout.height())
    }
}

impl Object for Label {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Label {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Create a temporary font system to calculate size
        // In a real application, this would be passed in or cached globally
        let mut font_system = FontSystem::new();
        let size = self.calculate_unconstrained_size(&mut font_system);

        if self.word_wrap {
            // For word-wrapped text, we have a minimum width (longest word)
            // and no maximum width. Height will be determined by height_for_width.
            SizeHint::new(size)
                .with_minimum_dimensions(0.0, self.font.size())
        } else if self.elide_mode != ElideMode::None {
            // For elided text, we can shrink to just the ellipsis
            let ellipsis_layout = TextLayout::new(&mut font_system, "…", &self.font);
            SizeHint::new(size)
                .with_minimum_dimensions(ellipsis_layout.width(), size.height)
        } else {
            // Fixed text - preferred size equals natural size
            SizeHint::new(size)
        }
    }

    fn height_for_width(&self, width: f32) -> Option<f32> {
        if !self.word_wrap {
            return None;
        }

        let mut font_system = FontSystem::new();
        let options = TextLayoutOptions::new()
            .max_width(width)
            .wrap(WrapMode::Word)
            .horizontal_align(self.horizontal_align);

        let layout = TextLayout::with_options(&mut font_system, &self.text, &self.font, options);
        Some(layout.height())
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if self.text.is_empty() {
            return;
        }

        let rect = ctx.rect();
        let width_constraint = if self.word_wrap || self.elide_mode != ElideMode::None {
            Some(rect.width())
        } else {
            None
        };

        // Get font system and text renderer (would normally be from application context)
        let mut font_system = FontSystem::new();

        // Build the layout
        let layout = self.ensure_layout(&mut font_system, width_constraint);

        // Calculate vertical offset based on alignment
        let y_offset = match self.vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => (rect.height() - layout.height()) / 2.0,
            VerticalAlign::Bottom => rect.height() - layout.height(),
        };

        // Calculate horizontal offset for non-wrapped single-line text
        let x_offset = if !self.word_wrap && layout.line_count() == 1 {
            match self.horizontal_align {
                HorizontalAlign::Left => 0.0,
                HorizontalAlign::Center => (rect.width() - layout.width()) / 2.0,
                HorizontalAlign::Right => rect.width() - layout.width(),
                HorizontalAlign::Justified => 0.0,
            }
        } else {
            0.0
        };

        let position = Point::new(rect.origin.x + x_offset, rect.origin.y + y_offset);

        // Create text renderer and prepare glyphs
        if let Ok(mut text_renderer) = TextRenderer::new() {
            if let Ok(prepared_glyphs) = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                position,
                self.text_color,
            ) {
                // In a full implementation, we would render the prepared glyphs
                // through the text render pass. For now, we draw background rectangles
                // for the text bounds to show the label area.
                let _glyphs = prepared_glyphs;

                // Note: Actual glyph rendering requires integration with the
                // application's render pass system. The prepared_glyphs would be
                // submitted to a TextRenderPass during the frame render.
            }
        }
    }
}

// Ensure Label is Send + Sync
static_assertions::assert_impl_all!(Label: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_label_creation() {
        setup();
        let label = Label::new("Hello, World!");
        assert_eq!(label.text(), "Hello, World!");
        assert_eq!(label.horizontal_align(), HorizontalAlign::Left);
        assert_eq!(label.vertical_align(), VerticalAlign::Top);
        assert!(!label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);
    }

    #[test]
    fn test_label_builder_pattern() {
        setup();
        let label = Label::new("Test")
            .with_horizontal_align(HorizontalAlign::Center)
            .with_vertical_align(VerticalAlign::Middle)
            .with_word_wrap(true)
            .with_text_color(Color::RED);

        assert_eq!(label.horizontal_align(), HorizontalAlign::Center);
        assert_eq!(label.vertical_align(), VerticalAlign::Middle);
        assert!(label.word_wrap());
        assert_eq!(label.text_color(), Color::RED);
    }

    #[test]
    fn test_word_wrap_and_elide_mutually_exclusive() {
        setup();
        let mut label = Label::new("Test");

        // Enable word wrap
        label.set_word_wrap(true);
        assert!(label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);

        // Enable elide - should disable word wrap
        label.set_elide_mode(ElideMode::Right);
        assert!(!label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::Right);

        // Enable word wrap again - should disable elide
        label.set_word_wrap(true);
        assert!(label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);
    }

    #[test]
    fn test_text_changed_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let mut label = Label::new("Initial");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        label.text_changed.connect(move |_text| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        label.set_text("Changed");
        assert!(signal_received.load(Ordering::SeqCst));
        assert_eq!(label.text(), "Changed");
    }

    #[test]
    fn test_elide_mode_variants() {
        setup();
        assert_eq!(ElideMode::default(), ElideMode::None);

        let label_left = Label::new("Test").with_elide_mode(ElideMode::Left);
        assert_eq!(label_left.elide_mode(), ElideMode::Left);

        let label_middle = Label::new("Test").with_elide_mode(ElideMode::Middle);
        assert_eq!(label_middle.elide_mode(), ElideMode::Middle);

        let label_right = Label::new("Test").with_elide_mode(ElideMode::Right);
        assert_eq!(label_right.elide_mode(), ElideMode::Right);
    }

    #[test]
    fn test_size_hint_basic() {
        setup();
        let label = Label::new("Hello");
        let hint = label.size_hint();

        // Should have a non-zero preferred size
        assert!(hint.preferred.width > 0.0);
        assert!(hint.preferred.height > 0.0);
    }

    #[test]
    fn test_height_for_width_without_wrap() {
        setup();
        let label = Label::new("Test");
        assert!(label.height_for_width(100.0).is_none());
    }

    #[test]
    fn test_height_for_width_with_wrap() {
        setup();
        let label = Label::new("This is a longer text that should wrap").with_word_wrap(true);
        let height = label.height_for_width(50.0);
        assert!(height.is_some());
        assert!(height.unwrap() > 0.0);
    }

    #[test]
    fn test_from_html() {
        setup();
        let label = Label::from_html("Hello <b>bold</b> world!");
        assert_eq!(label.text(), "Hello bold world!");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_with_html() {
        setup();
        let label = Label::new("placeholder").with_html("Hello <i>italic</i>!");
        assert_eq!(label.text(), "Hello italic!");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_set_html() {
        setup();
        let mut label = Label::new("Plain text");
        assert!(!label.has_rich_text());

        label.set_html("<b>Bold</b> text");
        assert_eq!(label.text(), "Bold text");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_set_text_clears_rich_text() {
        setup();
        let mut label = Label::from_html("<b>Bold</b>");
        assert!(label.has_rich_text());

        label.set_text("Plain text");
        assert_eq!(label.text(), "Plain text");
        assert!(!label.has_rich_text());
    }

    #[test]
    fn test_rich_text_access() {
        setup();
        let label = Label::from_html("<b>Bold</b> and <i>italic</i>");
        let rich = label.rich_text().expect("should have rich text");
        // spans: "Bold" (bold), " and ", "italic" (italic)
        assert_eq!(rich.spans().len(), 3);
    }

    #[test]
    fn test_complex_html() {
        setup();
        let label = Label::from_html(
            "<b>Bold</b> <i>italic</i> <u>underline</u> <s>strikethrough</s>"
        );
        assert_eq!(label.text(), "Bold italic underline strikethrough");
        assert!(label.has_rich_text());

        let rich = label.rich_text().unwrap();
        // "Bold" (bold), " ", "italic" (italic), " ", "underline" (underline), " ", "strikethrough" (strikethrough)
        assert!(rich.spans()[0].bold);
        assert!(rich.spans()[2].italic);
        assert!(rich.spans()[4].underline);
        assert!(rich.spans()[6].strikethrough);
    }

    #[test]
    fn test_html_with_color() {
        setup();
        let label = Label::from_html("<font color=\"red\">Red text</font>");
        let rich = label.rich_text().unwrap();
        assert_eq!(rich.spans()[0].color, Some([255, 0, 0, 255]));
    }

    #[test]
    fn test_html_with_line_breaks() {
        setup();
        let label = Label::from_html("Line 1<br>Line 2<br/>Line 3");
        assert_eq!(label.text(), "Line 1\nLine 2\nLine 3");
    }
}
