//! Rich text support with HTML parsing.
//!
//! This module provides rich text capabilities for the Horizon Lattice framework,
//! including parsing a basic HTML subset for styled text.
//!
//! # Supported HTML Tags
//!
//! The following HTML tags are supported:
//!
//! - `<b>`, `<strong>` - Bold text
//! - `<i>`, `<em>` - Italic text
//! - `<u>` - Underlined text
//! - `<s>`, `<del>`, `<strike>` - Strikethrough text
//! - `<br>`, `<br/>` - Line break
//! - `<font size="..." color="...">` - Font size and color
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{RichText, Font, FontFamily, TextLayout, TextLayoutOptions, FontSystem};
//!
//! let mut font_system = FontSystem::new();
//! let base_font = Font::new(FontFamily::SansSerif, 14.0);
//!
//! // Parse HTML into rich text
//! let rich = RichText::from_html("Hello <b>bold</b> and <i>italic</i> world!");
//!
//! // Create spans and layout
//! let spans = rich.to_spans(&base_font);
//! let layout = TextLayout::rich_text(
//!     &mut font_system,
//!     &spans,
//!     &base_font,
//!     TextLayoutOptions::default(),
//! );
//! ```

use super::{Font, FontStyle, FontWeight, TextDecoration, TextSpan};

/// A segment of rich text with styling information.
///
/// Unlike `TextSpan<'a>`, this struct owns its text content, making it
/// suitable for storing parsed HTML results.
#[derive(Debug, Clone, PartialEq)]
pub struct RichTextSpan {
    /// The text content of this span.
    pub text: String,
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
    /// Whether the text has underline.
    pub underline: bool,
    /// Whether the text has strikethrough.
    pub strikethrough: bool,
    /// Optional text color (RGBA).
    pub color: Option<[u8; 4]>,
    /// Optional font size override.
    pub font_size: Option<f32>,
}

impl RichTextSpan {
    /// Create a new span with plain text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            font_size: None,
        }
    }

    /// Create a line break span.
    pub fn line_break() -> Self {
        Self::new("\n")
    }

    /// Check if this span is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Convert to a `TextSpan` using the given base font.
    pub fn to_text_span<'a>(&'a self, base_font: &Font) -> TextSpan<'a> {
        let mut span = TextSpan::new(&self.text);

        // Build font with style overrides
        let mut needs_font = false;
        let mut font = base_font.clone();

        if self.bold {
            font = font.with_weight(FontWeight::BOLD);
            needs_font = true;
        }

        if self.italic {
            font = font.with_style(FontStyle::Italic);
            needs_font = true;
        }

        if let Some(size) = self.font_size {
            font = font.with_size(size);
            needs_font = true;
        }

        if needs_font {
            span = span.with_font(font);
        }

        if let Some(color) = self.color {
            span = span.with_color(color);
        }

        if self.underline {
            span = span.with_decoration(TextDecoration::underline());
        }

        if self.strikethrough {
            span = span.with_decoration(TextDecoration::strikethrough());
        }

        span
    }
}

/// Rich text content parsed from HTML or constructed programmatically.
///
/// `RichText` owns all its text content and can be converted to `TextSpan`
/// references for rendering.
#[derive(Debug, Clone, Default)]
pub struct RichText {
    /// The spans that make up this rich text.
    spans: Vec<RichTextSpan>,
}

impl RichText {
    /// Create empty rich text.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create rich text from plain text (no formatting).
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            spans: vec![RichTextSpan::new(text)],
        }
    }

    /// Parse rich text from an HTML string.
    ///
    /// Supports a basic HTML subset:
    /// - `<b>`, `<strong>` for bold
    /// - `<i>`, `<em>` for italic
    /// - `<u>` for underline
    /// - `<s>`, `<del>`, `<strike>` for strikethrough
    /// - `<br>`, `<br/>` for line breaks
    /// - `<font size="..." color="...">` for font size and color
    ///
    /// Unsupported tags are ignored (their content is still rendered).
    /// HTML entities `&lt;`, `&gt;`, `&amp;`, `&quot;`, `&nbsp;` are decoded.
    pub fn from_html(html: &str) -> Self {
        HtmlParser::parse(html)
    }

    /// Get the spans in this rich text.
    pub fn spans(&self) -> &[RichTextSpan] {
        &self.spans
    }

    /// Get the plain text content (without formatting).
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Check if this rich text is empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|s| s.is_empty())
    }

    /// Convert to `TextSpan` references for use with `TextLayout::rich_text()`.
    pub fn to_spans<'a>(&'a self, base_font: &Font) -> Vec<TextSpan<'a>> {
        self.spans
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_text_span(base_font))
            .collect()
    }

    /// Add a span to this rich text.
    pub fn push(&mut self, span: RichTextSpan) {
        self.spans.push(span);
    }

    /// Add plain text.
    pub fn push_text(&mut self, text: impl Into<String>) {
        self.spans.push(RichTextSpan::new(text));
    }

    /// Add a line break.
    pub fn push_line_break(&mut self) {
        self.spans.push(RichTextSpan::line_break());
    }
}

/// Current formatting state during HTML parsing.
#[derive(Debug, Clone, Default)]
struct FormatState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: Option<[u8; 4]>,
    font_size: Option<f32>,
}

impl FormatState {
    fn apply_to_span(&self, span: &mut RichTextSpan) {
        span.bold = self.bold;
        span.italic = self.italic;
        span.underline = self.underline;
        span.strikethrough = self.strikethrough;
        span.color = self.color;
        span.font_size = self.font_size;
    }
}

/// Simple HTML parser for rich text.
struct HtmlParser {
    spans: Vec<RichTextSpan>,
    current_text: String,
    format_stack: Vec<FormatState>,
}

impl HtmlParser {
    fn new() -> Self {
        Self {
            spans: Vec::new(),
            current_text: String::new(),
            format_stack: vec![FormatState::default()],
        }
    }

    fn current_format(&self) -> &FormatState {
        // Safe: format_stack is always initialized with at least one element
        self.format_stack.last().expect("format_stack should never be empty")
    }

    fn flush_text(&mut self) {
        if !self.current_text.is_empty() {
            let mut span = RichTextSpan::new(std::mem::take(&mut self.current_text));
            self.current_format().apply_to_span(&mut span);
            self.spans.push(span);
        }
    }

    fn push_format(&mut self, modifier: impl FnOnce(&mut FormatState)) {
        self.flush_text();
        let mut new_format = self.current_format().clone();
        modifier(&mut new_format);
        self.format_stack.push(new_format);
    }

    fn pop_format(&mut self) {
        self.flush_text();
        if self.format_stack.len() > 1 {
            self.format_stack.pop();
        }
    }

    fn add_line_break(&mut self) {
        self.flush_text();
        let mut span = RichTextSpan::line_break();
        self.current_format().apply_to_span(&mut span);
        self.spans.push(span);
    }

    fn parse(html: &str) -> RichText {
        let mut parser = HtmlParser::new();
        let mut chars = html.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '<' {
                // Parse tag
                let mut tag_content = String::new();
                while let Some(&tc) = chars.peek() {
                    if tc == '>' {
                        chars.next();
                        break;
                    }
                    tag_content.push(chars.next().unwrap());
                }
                parser.handle_tag(&tag_content);
            } else if c == '&' {
                // Parse HTML entity
                let mut entity = String::new();
                while let Some(&ec) = chars.peek() {
                    if ec == ';' {
                        chars.next();
                        break;
                    }
                    if ec == '<' || ec == ' ' || entity.len() > 10 {
                        // Not a valid entity, treat as literal
                        parser.current_text.push('&');
                        parser.current_text.push_str(&entity);
                        entity.clear();
                        break;
                    }
                    entity.push(chars.next().unwrap());
                }
                if !entity.is_empty() {
                    parser.current_text.push_str(&decode_entity(&entity));
                }
            } else {
                parser.current_text.push(c);
            }
        }

        parser.flush_text();

        RichText {
            spans: parser.spans,
        }
    }

    fn handle_tag(&mut self, tag_content: &str) {
        let tag_content = tag_content.trim();

        // Check for self-closing tags
        let is_self_closing = tag_content.ends_with('/');
        let tag_content = tag_content.trim_end_matches('/').trim();

        // Check for closing tag
        let is_closing = tag_content.starts_with('/');
        let tag_content = tag_content.trim_start_matches('/').trim();

        // Extract tag name (everything before first whitespace or end)
        let (tag_name, attrs_str) = match tag_content.find(|c: char| c.is_whitespace()) {
            Some(idx) => (&tag_content[..idx], tag_content[idx..].trim()),
            None => (tag_content, ""),
        };
        let tag_name = tag_name.to_lowercase();

        if is_closing {
            self.handle_closing_tag(&tag_name);
        } else {
            // Parse attributes properly, respecting quotes
            let attrs = parse_attributes(attrs_str);
            self.handle_opening_tag(&tag_name, &attrs, is_self_closing);
        }
    }

    fn handle_opening_tag(&mut self, tag_name: &str, attrs: &[(String, String)], is_self_closing: bool) {
        match tag_name {
            "b" | "strong" => {
                self.push_format(|f| f.bold = true);
            }
            "i" | "em" => {
                self.push_format(|f| f.italic = true);
            }
            "u" => {
                self.push_format(|f| f.underline = true);
            }
            "s" | "del" | "strike" => {
                self.push_format(|f| f.strikethrough = true);
            }
            "br" => {
                self.add_line_break();
            }
            "font" => {
                let (size, color) = parse_font_attrs(attrs);
                self.push_format(|f| {
                    if let Some(s) = size {
                        f.font_size = Some(s);
                    }
                    if let Some(c) = color {
                        f.color = Some(c);
                    }
                });
            }
            _ => {
                // Unknown tag - ignore but process children
                if !is_self_closing {
                    // Push a dummy format state to balance the stack
                    let current = self.current_format().clone();
                    self.format_stack.push(current);
                }
            }
        }
    }

    fn handle_closing_tag(&mut self, tag_name: &str) {
        match tag_name {
            "b" | "strong" | "i" | "em" | "u" | "s" | "del" | "strike" | "font" => {
                self.pop_format();
            }
            _ => {
                // Pop the dummy format state for unknown tags
                self.pop_format();
            }
        }
    }
}

/// Decode common HTML entities.
fn decode_entity(entity: &str) -> String {
    match entity {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => "\u{00A0}".to_string(), // Non-breaking space
        "ndash" => "–".to_string(),
        "mdash" => "—".to_string(),
        "copy" => "©".to_string(),
        "reg" => "®".to_string(),
        "trade" => "™".to_string(),
        "hellip" => "…".to_string(),
        _ => {
            // Try numeric entity
            if let Some(hex) = entity.strip_prefix('#') {
                if let Some(hex_val) = hex.strip_prefix('x').or_else(|| hex.strip_prefix('X')) {
                    // Hex entity like &#x1F600;
                    if let Ok(code_point) = u32::from_str_radix(hex_val, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            return c.to_string();
                        }
                    }
                } else {
                    // Decimal entity like &#128512;
                    if let Ok(code_point) = hex.parse::<u32>() {
                        if let Some(c) = char::from_u32(code_point) {
                            return c.to_string();
                        }
                    }
                }
            }
            // Unknown entity - return as-is with ampersand
            format!("&{};", entity)
        }
    }
}

/// Parse attributes from a string, respecting quoted values.
///
/// Handles attributes like: `size="14" color="rgb(255, 128, 0)"`
fn parse_attributes(attrs_str: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut chars = attrs_str.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Parse key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(chars.next().unwrap());
        }

        if key.is_empty() {
            break;
        }

        // Skip whitespace before =
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Check for =
        if chars.peek() != Some(&'=') {
            // Attribute without value, skip
            continue;
        }
        chars.next(); // consume '='

        // Skip whitespace after =
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Parse value
        let mut value = String::new();
        let quote_char = chars.peek().copied();

        if quote_char == Some('"') || quote_char == Some('\'') {
            chars.next(); // consume opening quote
            let quote = quote_char.unwrap();
            while let Some(c) = chars.next() {
                if c == quote {
                    break;
                }
                value.push(c);
            }
        } else {
            // Unquoted value - read until whitespace
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                value.push(chars.next().unwrap());
            }
        }

        result.push((key.to_lowercase(), value));
    }

    result
}

/// Parse font tag attributes from parsed key-value pairs.
fn parse_font_attrs(attrs: &[(String, String)]) -> (Option<f32>, Option<[u8; 4]>) {
    let mut size = None;
    let mut color = None;

    for (key, value) in attrs {
        match key.as_str() {
            "size" => {
                size = parse_font_size(value);
            }
            "color" => {
                color = parse_color(value);
            }
            _ => {}
        }
    }

    (size, color)
}

/// Parse a single attribute like `key="value"` or `key='value'`.
#[allow(dead_code)]
fn parse_attr(attr: &str) -> Option<(String, String)> {
    let mut parts = attr.splitn(2, '=');
    let key = parts.next()?.trim().to_string();
    let value = parts.next()?.trim();

    // Remove quotes
    let value = value
        .trim_start_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('"')
        .trim_end_matches('\'')
        .to_string();

    Some((key, value))
}

/// Parse font size value.
///
/// Supports:
/// - Absolute sizes: "1" through "7" (maps to ~8pt through ~36pt)
/// - Pixel values: "14px", "16px", etc.
/// - Point values: "12pt", "14pt", etc.
fn parse_font_size(value: &str) -> Option<f32> {
    let value = value.trim();

    // Check for px suffix
    if let Some(px) = value.strip_suffix("px") {
        return px.trim().parse().ok();
    }

    // Check for pt suffix (convert to px: 1pt ≈ 1.333px at 96dpi)
    if let Some(pt) = value.strip_suffix("pt") {
        return pt.trim().parse::<f32>().ok().map(|p| p * 1.333);
    }

    // HTML font size 1-7 mapping
    match value {
        "1" => Some(8.0),
        "2" => Some(10.0),
        "3" => Some(12.0),
        "4" => Some(14.0),
        "5" => Some(18.0),
        "6" => Some(24.0),
        "7" => Some(36.0),
        _ => value.parse().ok(),
    }
}

/// Parse color value.
///
/// Supports:
/// - Hex colors: "#RGB", "#RRGGBB", "#RRGGBBAA"
/// - Named colors: "red", "green", "blue", etc.
/// - RGB function: "rgb(255, 0, 0)"
/// - RGBA function: "rgba(255, 0, 0, 128)"
fn parse_color(value: &str) -> Option<[u8; 4]> {
    let value = value.trim();

    // Hex color
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // RGB/RGBA function
    if value.starts_with("rgb") {
        return parse_rgb_function(value);
    }

    // Named colors
    parse_named_color(value)
}

fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    match hex.len() {
        3 => {
            // #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some([r, g, b, 255])
        }
        4 => {
            // #RGBA -> #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            Some([r, g, b, a])
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some([r, g, b, 255])
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}

fn parse_rgb_function(value: &str) -> Option<[u8; 4]> {
    // Extract content between parentheses
    let start = value.find('(')?;
    let end = value.rfind(')')?;
    let content = &value[start + 1..end];

    let parts: Vec<&str> = content.split(',').collect();

    match parts.len() {
        3 => {
            // rgb(r, g, b)
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            Some([r, g, b, 255])
        }
        4 => {
            // rgba(r, g, b, a)
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            // Alpha can be 0-255 or 0.0-1.0
            let a_str = parts[3].trim();
            let a = if a_str.contains('.') {
                // Floating point alpha
                let a_float: f32 = a_str.parse().ok()?;
                (a_float * 255.0) as u8
            } else {
                a_str.parse().ok()?
            };
            Some([r, g, b, a])
        }
        _ => None,
    }
}

fn parse_named_color(name: &str) -> Option<[u8; 4]> {
    // Common CSS/HTML color names
    match name.to_lowercase().as_str() {
        "black" => Some([0, 0, 0, 255]),
        "white" => Some([255, 255, 255, 255]),
        "red" => Some([255, 0, 0, 255]),
        "green" => Some([0, 128, 0, 255]),
        "blue" => Some([0, 0, 255, 255]),
        "yellow" => Some([255, 255, 0, 255]),
        "cyan" | "aqua" => Some([0, 255, 255, 255]),
        "magenta" | "fuchsia" => Some([255, 0, 255, 255]),
        "gray" | "grey" => Some([128, 128, 128, 255]),
        "silver" => Some([192, 192, 192, 255]),
        "maroon" => Some([128, 0, 0, 255]),
        "olive" => Some([128, 128, 0, 255]),
        "lime" => Some([0, 255, 0, 255]),
        "navy" => Some([0, 0, 128, 255]),
        "purple" => Some([128, 0, 128, 255]),
        "teal" => Some([0, 128, 128, 255]),
        "orange" => Some([255, 165, 0, 255]),
        "pink" => Some([255, 192, 203, 255]),
        "brown" => Some([165, 42, 42, 255]),
        "gold" => Some([255, 215, 0, 255]),
        "coral" => Some([255, 127, 80, 255]),
        "crimson" => Some([220, 20, 60, 255]),
        "darkblue" => Some([0, 0, 139, 255]),
        "darkgreen" => Some([0, 100, 0, 255]),
        "darkred" => Some([139, 0, 0, 255]),
        "indigo" => Some([75, 0, 130, 255]),
        "violet" => Some([238, 130, 238, 255]),
        "transparent" => Some([0, 0, 0, 0]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let rich = RichText::from_html("Hello, World!");
        assert_eq!(rich.plain_text(), "Hello, World!");
        assert_eq!(rich.spans().len(), 1);
        assert!(!rich.spans()[0].bold);
        assert!(!rich.spans()[0].italic);
    }

    #[test]
    fn test_bold_text() {
        let rich = RichText::from_html("Hello <b>bold</b> world!");
        assert_eq!(rich.plain_text(), "Hello bold world!");
        assert_eq!(rich.spans().len(), 3);
        assert!(!rich.spans()[0].bold);
        assert!(rich.spans()[1].bold);
        assert!(!rich.spans()[2].bold);
    }

    #[test]
    fn test_strong_tag() {
        let rich = RichText::from_html("<strong>Strong text</strong>");
        assert_eq!(rich.spans().len(), 1);
        assert!(rich.spans()[0].bold);
    }

    #[test]
    fn test_italic_text() {
        let rich = RichText::from_html("Hello <i>italic</i> world!");
        assert_eq!(rich.plain_text(), "Hello italic world!");
        assert!(rich.spans()[1].italic);
    }

    #[test]
    fn test_em_tag() {
        let rich = RichText::from_html("<em>Emphasized</em>");
        assert!(rich.spans()[0].italic);
    }

    #[test]
    fn test_underline_text() {
        let rich = RichText::from_html("Hello <u>underlined</u> world!");
        assert!(rich.spans()[1].underline);
    }

    #[test]
    fn test_strikethrough_text() {
        let rich = RichText::from_html("Hello <s>deleted</s> world!");
        assert!(rich.spans()[1].strikethrough);
    }

    #[test]
    fn test_del_tag() {
        let rich = RichText::from_html("<del>Deleted</del>");
        assert!(rich.spans()[0].strikethrough);
    }

    #[test]
    fn test_line_break() {
        let rich = RichText::from_html("Line 1<br>Line 2");
        assert_eq!(rich.plain_text(), "Line 1\nLine 2");
    }

    #[test]
    fn test_self_closing_br() {
        let rich = RichText::from_html("Line 1<br/>Line 2");
        assert_eq!(rich.plain_text(), "Line 1\nLine 2");
    }

    #[test]
    fn test_nested_tags() {
        let rich = RichText::from_html("<b><i>Bold and italic</i></b>");
        assert_eq!(rich.spans().len(), 1);
        assert!(rich.spans()[0].bold);
        assert!(rich.spans()[0].italic);
    }

    #[test]
    fn test_font_color_hex() {
        let rich = RichText::from_html("<font color=\"#ff0000\">Red</font>");
        assert_eq!(rich.spans()[0].color, Some([255, 0, 0, 255]));
    }

    #[test]
    fn test_font_color_named() {
        let rich = RichText::from_html("<font color=\"blue\">Blue</font>");
        assert_eq!(rich.spans()[0].color, Some([0, 0, 255, 255]));
    }

    #[test]
    fn test_font_size() {
        let rich = RichText::from_html("<font size=\"5\">Large</font>");
        assert_eq!(rich.spans()[0].font_size, Some(18.0));
    }

    #[test]
    fn test_font_size_px() {
        let rich = RichText::from_html("<font size=\"20px\">20 pixels</font>");
        assert_eq!(rich.spans()[0].font_size, Some(20.0));
    }

    #[test]
    fn test_html_entities() {
        let rich = RichText::from_html("&lt;tag&gt; &amp; &quot;quoted&quot;");
        assert_eq!(rich.plain_text(), "<tag> & \"quoted\"");
    }

    #[test]
    fn test_numeric_entities() {
        let rich = RichText::from_html("&#60;&#62;"); // < >
        assert_eq!(rich.plain_text(), "<>");
    }

    #[test]
    fn test_hex_entities() {
        let rich = RichText::from_html("&#x3C;&#x3E;"); // < >
        assert_eq!(rich.plain_text(), "<>");
    }

    #[test]
    fn test_complex_html() {
        let rich = RichText::from_html(
            "<b>Bold</b> normal <i>italic <b>bold italic</b></i> end"
        );
        assert_eq!(rich.plain_text(), "Bold normal italic bold italic end");

        // Check formatting
        assert!(rich.spans()[0].bold);      // "Bold"
        assert!(!rich.spans()[1].bold);     // " normal "
        assert!(rich.spans()[2].italic);    // "italic "
        assert!(rich.spans()[3].bold);      // "bold italic"
        assert!(rich.spans()[3].italic);
        assert!(!rich.spans()[4].bold);     // " end"
        assert!(!rich.spans()[4].italic);
    }

    #[test]
    fn test_unclosed_tags() {
        // Unclosed tags should still work
        let rich = RichText::from_html("<b>Bold text");
        assert!(rich.spans()[0].bold);
    }

    #[test]
    fn test_unknown_tags() {
        let rich = RichText::from_html("<span>Unknown tag content</span>");
        assert_eq!(rich.plain_text(), "Unknown tag content");
    }

    #[test]
    fn test_empty_input() {
        let rich = RichText::from_html("");
        assert!(rich.is_empty());
    }

    #[test]
    fn test_hex_color_short() {
        assert_eq!(parse_hex_color("f00"), Some([255, 0, 0, 255]));
        assert_eq!(parse_hex_color("0f0"), Some([0, 255, 0, 255]));
    }

    #[test]
    fn test_rgb_function() {
        let rich = RichText::from_html("<font color=\"rgb(255, 128, 0)\">Orange</font>");
        assert_eq!(rich.spans()[0].color, Some([255, 128, 0, 255]));
    }

    #[test]
    fn test_rgba_function() {
        let rich = RichText::from_html("<font color=\"rgba(255, 0, 0, 128)\">Semi-transparent</font>");
        assert_eq!(rich.spans()[0].color, Some([255, 0, 0, 128]));
    }

    #[test]
    fn test_case_insensitive_tags() {
        let rich = RichText::from_html("<B>BOLD</B> <I>ITALIC</I>");
        // spans: "BOLD" (bold), " ", "ITALIC" (italic)
        assert!(rich.spans()[0].bold);
        assert!(rich.spans()[2].italic);
    }

    #[test]
    fn test_whitespace_in_tags() {
        let rich = RichText::from_html("< b >Bold</ b >");
        assert!(rich.spans()[0].bold);
    }

    #[test]
    fn test_nbsp_entity() {
        let rich = RichText::from_html("Hello&nbsp;World");
        assert_eq!(rich.plain_text(), "Hello\u{00A0}World");
    }
}
