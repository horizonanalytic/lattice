//! Computed style with all values resolved.

use crate::types::{BorderStyle, Cursor, TextAlign};
use horizon_lattice_render::{
    BoxShadow, Color, CornerRadii, Paint, Rect,
    text::{FontFamily, FontStretch, FontStyle, FontWeight, TextDecoration},
};

/// Fully resolved style with concrete values.
///
/// This is what widgets use for painting. All relative units are resolved
/// to pixels, and all special values are resolved to actual values.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // === Box Model (resolved to pixels) ===
    /// Top margin in pixels.
    pub margin_top: f32,
    /// Right margin in pixels.
    pub margin_right: f32,
    /// Bottom margin in pixels.
    pub margin_bottom: f32,
    /// Left margin in pixels.
    pub margin_left: f32,
    /// Top padding in pixels.
    pub padding_top: f32,
    /// Right padding in pixels.
    pub padding_right: f32,
    /// Bottom padding in pixels.
    pub padding_bottom: f32,
    /// Left padding in pixels.
    pub padding_left: f32,
    /// Top border width in pixels.
    pub border_top_width: f32,
    /// Right border width in pixels.
    pub border_right_width: f32,
    /// Bottom border width in pixels.
    pub border_bottom_width: f32,
    /// Left border width in pixels.
    pub border_left_width: f32,
    /// Border color.
    pub border_color: Color,
    /// Border line style.
    pub border_style: BorderStyle,
    /// Border corner radii.
    pub border_radius: CornerRadii,

    // === Background ===
    /// Background paint (solid color or gradient).
    pub background: Paint,

    // === Size Constraints ===
    /// Minimum width constraint in pixels, if set.
    pub min_width: Option<f32>,
    /// Minimum height constraint in pixels, if set.
    pub min_height: Option<f32>,
    /// Maximum width constraint in pixels, if set.
    pub max_width: Option<f32>,
    /// Maximum height constraint in pixels, if set.
    pub max_height: Option<f32>,
    /// Explicit width in pixels, if set.
    pub width: Option<f32>,
    /// Explicit height in pixels, if set.
    pub height: Option<f32>,

    // === Typography ===
    /// Font family stack.
    pub font_family: Vec<FontFamily>,
    /// Font size in pixels.
    pub font_size: f32,
    /// Font weight (boldness).
    pub font_weight: FontWeight,
    /// Font style (normal, italic, oblique).
    pub font_style: FontStyle,
    /// Font stretch (condensed, expanded, etc.).
    pub font_stretch: FontStretch,
    /// Text foreground color.
    pub color: Color,
    /// Text alignment.
    pub text_align: TextAlign,
    /// Line height in pixels.
    pub line_height: f32,
    /// Letter spacing in pixels.
    pub letter_spacing: f32,
    /// Text decoration (underline, strikethrough, etc.).
    pub text_decoration: Option<TextDecoration>,

    // === Effects ===
    /// Opacity (0.0 = transparent, 1.0 = opaque).
    pub opacity: f32,
    /// Box shadows to render.
    pub box_shadow: Vec<BoxShadow>,

    // === Interaction ===
    /// Mouse cursor style.
    pub cursor: Cursor,
    /// Whether pointer events are enabled.
    pub pointer_events: bool,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            // Box model - all zero
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            border_color: Color::TRANSPARENT,
            border_style: BorderStyle::None,
            border_radius: CornerRadii::uniform(0.0),

            // Background
            background: Paint::Solid(Color::TRANSPARENT),

            // Size
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            width: None,
            height: None,

            // Typography
            font_family: vec![FontFamily::SansSerif],
            font_size: 14.0,
            font_weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
            font_stretch: FontStretch::Normal,
            color: Color::BLACK,
            text_align: TextAlign::Start,
            line_height: 1.2,
            letter_spacing: 0.0,
            text_decoration: None,

            // Effects
            opacity: 1.0,
            box_shadow: vec![],

            // Interaction
            cursor: Cursor::Default,
            pointer_events: true,
        }
    }
}

impl ComputedStyle {
    /// Get the content box rect given widget bounds.
    ///
    /// The content box is the area inside padding and border.
    pub fn content_rect(&self, widget_rect: Rect) -> Rect {
        Rect::new(
            widget_rect.origin.x + self.padding_left + self.border_left_width,
            widget_rect.origin.y + self.padding_top + self.border_top_width,
            widget_rect.size.width
                - self.padding_left
                - self.padding_right
                - self.border_left_width
                - self.border_right_width,
            widget_rect.size.height
                - self.padding_top
                - self.padding_bottom
                - self.border_top_width
                - self.border_bottom_width,
        )
    }

    /// Get the padding box rect (inside border, including padding).
    pub fn padding_rect(&self, widget_rect: Rect) -> Rect {
        Rect::new(
            widget_rect.origin.x + self.border_left_width,
            widget_rect.origin.y + self.border_top_width,
            widget_rect.size.width - self.border_left_width - self.border_right_width,
            widget_rect.size.height - self.border_top_width - self.border_bottom_width,
        )
    }

    /// Get total horizontal margin + border + padding.
    pub fn horizontal_space(&self) -> f32 {
        self.margin_left
            + self.margin_right
            + self.padding_left
            + self.padding_right
            + self.border_left_width
            + self.border_right_width
    }

    /// Get total vertical margin + border + padding.
    pub fn vertical_space(&self) -> f32 {
        self.margin_top
            + self.margin_bottom
            + self.padding_top
            + self.padding_bottom
            + self.border_top_width
            + self.border_bottom_width
    }

    /// Get total horizontal padding.
    pub fn horizontal_padding(&self) -> f32 {
        self.padding_left + self.padding_right
    }

    /// Get total vertical padding.
    pub fn vertical_padding(&self) -> f32 {
        self.padding_top + self.padding_bottom
    }

    /// Get total horizontal border width.
    pub fn horizontal_border(&self) -> f32 {
        self.border_left_width + self.border_right_width
    }

    /// Get total vertical border width.
    pub fn vertical_border(&self) -> f32 {
        self.border_top_width + self.border_bottom_width
    }

    /// Check if the border should be drawn.
    pub fn has_border(&self) -> bool {
        self.border_style != BorderStyle::None
            && (self.border_top_width > 0.0
                || self.border_right_width > 0.0
                || self.border_bottom_width > 0.0
                || self.border_left_width > 0.0)
            && self.border_color.a > 0.0
    }

    /// Check if there are any box shadows.
    pub fn has_shadows(&self) -> bool {
        !self.box_shadow.is_empty()
    }

    /// Check if the background should be drawn.
    pub fn has_background(&self) -> bool {
        match &self.background {
            Paint::Solid(color) => color.a > 0.0,
            Paint::LinearGradient(_) | Paint::RadialGradient(_) => true,
        }
    }

    /// Check if border radius is non-zero.
    pub fn has_border_radius(&self) -> bool {
        !self.border_radius.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn computed_style_default() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.opacity, 1.0);
        assert!(style.pointer_events);
    }

    #[test]
    fn content_rect_calculation() {
        let mut style = ComputedStyle::default();
        style.padding_left = 10.0;
        style.padding_top = 5.0;
        style.padding_right = 10.0;
        style.padding_bottom = 5.0;
        style.border_left_width = 1.0;
        style.border_top_width = 1.0;
        style.border_right_width = 1.0;
        style.border_bottom_width = 1.0;

        let widget_rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        let content = style.content_rect(widget_rect);

        assert_eq!(content.origin.x, 11.0); // padding + border
        assert_eq!(content.origin.y, 6.0);
        assert_eq!(content.size.width, 78.0); // 100 - 2*(10+1)
        assert_eq!(content.size.height, 38.0); // 50 - 2*(5+1)
    }

    #[test]
    fn horizontal_vertical_space() {
        let mut style = ComputedStyle::default();
        style.margin_left = 5.0;
        style.margin_right = 5.0;
        style.padding_left = 10.0;
        style.padding_right = 10.0;
        style.border_left_width = 1.0;
        style.border_right_width = 1.0;

        assert_eq!(style.horizontal_space(), 32.0); // 5+5+10+10+1+1
    }
}
