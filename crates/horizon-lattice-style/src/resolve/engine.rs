//! Main style resolution engine.

use horizon_lattice_core::ObjectId;
use crate::rules::{StyleSheet, StyleRule};
use crate::selector::{Selector, SelectorMatcher, SpecificityWithOrder};
use crate::selector::{WidgetMatchContext, WidgetState, SiblingInfo};
use crate::style::{StyleProperties, ComputedStyle};
use crate::resolve::cascade::cascade_properties;
use crate::resolve::inheritance::resolve_properties;
use crate::resolve::cache::{StyleCache, StyleCacheKey};
use crate::theme::Theme;

/// Context for style resolution.
///
/// This provides all the information needed to match selectors
/// and compute styles for a widget.
#[derive(Debug, Clone)]
pub struct StyleContext<'a> {
    /// Widget type name (e.g., "Button", "Label").
    pub widget_type: &'a str,
    /// Widget name (for ID selector matching).
    pub widget_name: Option<&'a str>,
    /// Widget's CSS classes.
    pub classes: &'a [String],
    /// Widget state for pseudo-class matching.
    pub state: WidgetStyleState,
    /// Parent's computed style (for inheritance).
    pub parent_style: Option<&'a ComputedStyle>,
    /// Root font size (for rem units).
    pub root_font_size: f32,
}

impl<'a> StyleContext<'a> {
    /// Create a match context for selector matching.
    pub fn to_match_context(&self) -> WidgetMatchContext<'a> {
        WidgetMatchContext {
            widget_type: self.widget_type,
            widget_name: self.widget_name,
            classes: self.classes,
            state: WidgetState {
                hovered: self.state.hovered,
                pressed: self.state.pressed,
                focused: self.state.focused,
                enabled: self.state.enabled,
                checked: self.state.checked,
            },
            sibling_info: self.state.sibling_info.map(|(index, count)| {
                SiblingInfo { index, count }
            }),
            child_count: self.state.child_count,
        }
    }
}

/// Widget state for pseudo-class matching and cache keying.
#[derive(Debug, Clone, Copy, Default)]
pub struct WidgetStyleState {
    /// Whether the mouse is hovering over the widget.
    pub hovered: bool,
    /// Whether the widget is being pressed/clicked.
    pub pressed: bool,
    /// Whether the widget has keyboard focus.
    pub focused: bool,
    /// Whether the widget is enabled for interaction.
    pub enabled: bool,
    /// Checked state for checkable widgets (None = not checkable).
    pub checked: Option<bool>,
    /// Sibling info as (index, total_count).
    pub sibling_info: Option<(usize, usize)>,
    /// Number of children this widget has.
    pub child_count: usize,
}

impl WidgetStyleState {
    /// Convert to WidgetState for selector matching.
    pub fn to_widget_state(&self) -> WidgetState {
        WidgetState {
            hovered: self.hovered,
            pressed: self.pressed,
            focused: self.focused,
            enabled: self.enabled,
            checked: self.checked,
        }
    }
}

/// The main style resolution engine.
///
/// The engine manages stylesheets, matches selectors, cascades properties,
/// and resolves final computed styles.
pub struct StyleEngine {
    /// All registered stylesheets, sorted by priority.
    stylesheets: Vec<StyleSheet>,
    /// Style cache for performance.
    cache: StyleCache,
    /// Current theme.
    theme: Theme,
    /// Root font size (for rem units).
    root_font_size: f32,
}

impl StyleEngine {
    /// Create a new style engine with a theme.
    pub fn new(theme: Theme) -> Self {
        Self {
            stylesheets: vec![],
            cache: StyleCache::new(),
            theme,
            root_font_size: 16.0,
        }
    }

    /// Create a style engine with the light theme.
    pub fn light() -> Self {
        Self::new(Theme::light())
    }

    /// Create a style engine with the dark theme.
    pub fn dark() -> Self {
        Self::new(Theme::dark())
    }

    /// Get the root font size.
    pub fn root_font_size(&self) -> f32 {
        self.root_font_size
    }

    /// Set the root font size.
    pub fn set_root_font_size(&mut self, size: f32) {
        self.root_font_size = size;
        self.cache.invalidate_all();
    }

    /// Add a stylesheet.
    pub fn add_stylesheet(&mut self, stylesheet: StyleSheet) {
        self.stylesheets.push(stylesheet);
        self.stylesheets.sort_by_key(|s| s.priority);
        self.cache.invalidate_all();
    }

    /// Remove stylesheets from a specific source file.
    pub fn remove_stylesheet_by_path(&mut self, path: &std::path::Path) {
        self.stylesheets.retain(|s| s.source_path.as_deref() != Some(path));
        self.cache.invalidate_all();
    }

    /// Clear all stylesheets.
    pub fn clear_stylesheets(&mut self) {
        self.stylesheets.clear();
        self.cache.invalidate_all();
    }

    /// Compute the style for a widget.
    ///
    /// This performs the full style resolution:
    /// 1. Find all matching rules
    /// 2. Sort by specificity
    /// 3. Cascade properties
    /// 4. Apply inline styles
    /// 5. Resolve to computed values
    pub fn compute_style(
        &mut self,
        widget_id: ObjectId,
        context: &StyleContext<'_>,
        inline_style: Option<&StyleProperties>,
    ) -> ComputedStyle {
        // Check cache first (only if no inline style)
        if inline_style.is_none() {
            let cache_key = StyleCacheKey::new(widget_id, &context.state.to_widget_state());
            if let Some(cached) = self.cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // Collect all matching rules with specificity
        let match_context = context.to_match_context();
        let mut matched_rules: Vec<(&StyleRule, SpecificityWithOrder)> = vec![];
        let mut global_order = 0u32;

        for stylesheet in &self.stylesheets {
            let priority_offset = stylesheet.priority.as_order_offset();

            for rule in &stylesheet.rules {
                if SelectorMatcher::matches_subject(&rule.selector, &match_context) {
                    let order = priority_offset | global_order;
                    matched_rules.push((rule, rule.specificity.with_order(order)));
                    global_order += 1;
                }
            }
        }

        // Sort by specificity (lower specificity first, so later ones override)
        matched_rules.sort_by_key(|(_, spec)| *spec);

        // Cascade properties
        let mut cascaded = StyleProperties::default();

        // Apply theme defaults first
        if let Some(theme_props) = self.theme.widget_defaults.get(context.widget_type) {
            cascade_properties(&mut cascaded, theme_props);
        }

        // Apply matched rules in order
        for (rule, _) in &matched_rules {
            cascade_properties(&mut cascaded, &rule.properties);
        }

        // Apply inline styles (highest priority)
        if let Some(inline) = inline_style {
            cascade_properties(&mut cascaded, inline);
        }

        // Resolve to computed style
        let computed = resolve_properties(
            &cascaded,
            context.parent_style,
            context.root_font_size,
        );

        // Cache if no inline style
        if inline_style.is_none() {
            let cache_key = StyleCacheKey::new(widget_id, &context.state.to_widget_state());
            self.cache.insert(cache_key, computed.clone());
        }

        computed
    }

    /// Invalidate cache for a widget and its descendants.
    pub fn invalidate(&mut self, widget_id: ObjectId) {
        self.cache.invalidate(widget_id);
    }

    /// Invalidate all cached styles.
    pub fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }

    /// Set the current theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.invalidate_all();
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Get the number of loaded stylesheets.
    pub fn stylesheet_count(&self) -> usize {
        self.stylesheets.len()
    }

    /// Get the total number of rules across all stylesheets.
    pub fn rule_count(&self) -> usize {
        self.stylesheets.iter().map(|s| s.len()).sum()
    }

    /// Get the number of cached styles.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for StyleEngine {
    fn default() -> Self {
        Self::light()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_render::Color;
    use crate::style::Style;

    fn make_context<'a>(
        widget_type: &'a str,
        classes: &'a [String],
    ) -> StyleContext<'a> {
        StyleContext {
            widget_type,
            widget_name: None,
            classes,
            state: WidgetStyleState {
                enabled: true,
                ..Default::default()
            },
            parent_style: None,
            root_font_size: 16.0,
        }
    }

    #[test]
    fn engine_basic_resolution() {
        let mut engine = StyleEngine::light();

        // Add a stylesheet with a button rule
        let mut sheet = StyleSheet::application();
        sheet.add_rule(
            Selector::type_selector("Button"),
            Style::new()
                .background_color(Color::BLUE)
                .color(Color::WHITE)
                .build(),
        );
        engine.add_stylesheet(sheet);

        let classes = vec![];
        let context = make_context("Button", &classes);
        let computed = engine.compute_style(ObjectId::default(), &context, None);

        assert_eq!(computed.color, Color::WHITE);
    }

    #[test]
    fn engine_specificity_ordering() {
        let mut engine = StyleEngine::light();

        let mut sheet = StyleSheet::application();

        // Type selector (specificity 0,0,1)
        sheet.add_rule(
            Selector::type_selector("Button"),
            Style::new().color(Color::RED).build(),
        );

        // Class selector (specificity 0,1,0) - should win
        sheet.add_rule(
            Selector::class("primary"),
            Style::new().color(Color::BLUE).build(),
        );

        engine.add_stylesheet(sheet);

        let classes = vec!["primary".to_string()];
        let context = make_context("Button", &classes);
        let computed = engine.compute_style(ObjectId::default(), &context, None);

        assert_eq!(computed.color, Color::BLUE); // Class wins
    }

    #[test]
    fn engine_inline_style_priority() {
        let mut engine = StyleEngine::light();

        let mut sheet = StyleSheet::application();
        sheet.add_rule(
            Selector::type_selector("Button"),
            Style::new().color(Color::RED).build(),
        );
        engine.add_stylesheet(sheet);

        let inline = Style::new().color(Color::GREEN).build();

        let classes = vec![];
        let context = make_context("Button", &classes);
        let computed = engine.compute_style(ObjectId::default(), &context, Some(&inline));

        assert_eq!(computed.color, Color::GREEN); // Inline wins
    }

    #[test]
    fn engine_caching() {
        let mut engine = StyleEngine::light();

        let mut sheet = StyleSheet::application();
        sheet.add_rule(
            Selector::type_selector("Button"),
            Style::new().color(Color::RED).build(),
        );
        engine.add_stylesheet(sheet);

        let classes = vec![];
        let context = make_context("Button", &classes);

        // First call - cache miss
        let widget_id = ObjectId::default();
        let _ = engine.compute_style(widget_id, &context, None);
        assert_eq!(engine.cache_size(), 1);

        // Second call - cache hit
        let _ = engine.compute_style(widget_id, &context, None);
        assert_eq!(engine.cache_size(), 1); // Still 1
    }
}
