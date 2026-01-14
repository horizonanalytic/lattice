//! Stylesheet collection and management.

use std::path::{Path, PathBuf};
use crate::rules::StyleRule;
use crate::{Error, Result};

/// Priority level for style sources.
///
/// Higher priority styles override lower priority ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum StylePriority {
    /// Theme defaults (lowest priority).
    Theme = 0,
    /// Application-level stylesheet.
    Application = 1,
    /// Widget-specific styles.
    Widget = 2,
    /// Inline styles (highest priority).
    Inline = 3,
}

impl StylePriority {
    /// Get a numeric value for ordering calculations.
    pub fn as_order_offset(&self) -> u32 {
        (*self as u32) << 24
    }
}

/// A stylesheet containing multiple rules.
#[derive(Debug, Clone)]
pub struct StyleSheet {
    /// The rules in this stylesheet.
    pub rules: Vec<StyleRule>,
    /// Priority level.
    pub priority: StylePriority,
    /// Source file path (for hot-reload tracking).
    pub source_path: Option<PathBuf>,
}

impl StyleSheet {
    /// Create an empty stylesheet.
    pub fn new(priority: StylePriority) -> Self {
        Self {
            rules: vec![],
            priority,
            source_path: None,
        }
    }

    /// Create a theme stylesheet (lowest priority).
    pub fn theme() -> Self {
        Self::new(StylePriority::Theme)
    }

    /// Create an application stylesheet.
    pub fn application() -> Self {
        Self::new(StylePriority::Application)
    }

    /// Create a widget-specific stylesheet.
    pub fn widget() -> Self {
        Self::new(StylePriority::Widget)
    }

    /// Load a stylesheet from a CSS file.
    ///
    /// The file is parsed and rules are extracted. The source path is stored
    /// for hot-reload support.
    pub fn from_file(path: impl AsRef<Path>, priority: StylePriority) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::io(path, e))?;

        let mut sheet = Self::from_css(&content, priority)?;
        sheet.source_path = Some(path.to_path_buf());
        Ok(sheet)
    }

    /// Parse a stylesheet from CSS text.
    pub fn from_css(css: &str, priority: StylePriority) -> Result<Self> {
        let rules = crate::parser::parse_css(css)?;
        Ok(Self {
            rules,
            priority,
            source_path: None,
        })
    }

    /// Add a rule to the stylesheet.
    ///
    /// The rule's order is automatically set based on the current number of rules.
    pub fn add_rule(&mut self, selector: crate::selector::Selector, properties: crate::style::StyleProperties) {
        let order = self.rules.len() as u32;
        self.rules.push(StyleRule::new(selector, properties, order));
    }

    /// Add a pre-built rule to the stylesheet.
    pub fn add_style_rule(&mut self, mut rule: StyleRule) {
        rule.order = self.rules.len() as u32;
        self.rules.push(rule);
    }

    /// Get the number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the stylesheet is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Iterate over rules.
    pub fn iter(&self) -> impl Iterator<Item = &StyleRule> {
        self.rules.iter()
    }

    /// Clear all rules.
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self::application()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::Selector;
    use crate::style::StyleProperties;

    #[test]
    fn stylesheet_creation() {
        let mut sheet = StyleSheet::application();
        assert!(sheet.is_empty());

        sheet.add_rule(
            Selector::type_selector("Button"),
            StyleProperties::default(),
        );

        assert_eq!(sheet.len(), 1);
        assert_eq!(sheet.rules[0].order, 0);
    }

    #[test]
    fn stylesheet_priority() {
        assert!(StylePriority::Inline > StylePriority::Widget);
        assert!(StylePriority::Widget > StylePriority::Application);
        assert!(StylePriority::Application > StylePriority::Theme);
    }

    #[test]
    fn rule_ordering() {
        let mut sheet = StyleSheet::application();

        sheet.add_rule(Selector::type_selector("A"), StyleProperties::default());
        sheet.add_rule(Selector::type_selector("B"), StyleProperties::default());
        sheet.add_rule(Selector::type_selector("C"), StyleProperties::default());

        assert_eq!(sheet.rules[0].order, 0);
        assert_eq!(sheet.rules[1].order, 1);
        assert_eq!(sheet.rules[2].order, 2);
    }
}
