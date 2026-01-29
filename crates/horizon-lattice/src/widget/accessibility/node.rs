//! The Accessible trait for widget accessibility support.

use accesskit::{Action, Node, Toggled};
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::Rect;

use super::object_id_to_node_id;
use super::role::AccessibleRole;

/// Trait for widgets that provide accessibility information.
///
/// This trait allows widgets to expose their semantic information to
/// assistive technologies like screen readers. Widgets that implement
/// this trait can be navigated and interacted with using accessibility APIs.
///
/// # Default Implementations
///
/// Most methods have sensible defaults:
/// - `accessible_role()` returns `AccessibleRole::Unknown`
/// - `accessible_name()` returns `None` (widget name from object system)
/// - `accessible_description()` returns `None`
/// - State methods (`is_accessible_checked()`, etc.) return `None`
///
/// Widgets should override the methods relevant to their functionality.
///
/// # Example
///
/// ```ignore
/// impl Accessible for CheckBox {
///     fn accessible_role(&self) -> AccessibleRole {
///         AccessibleRole::CheckBox
///     }
///
///     fn accessible_name(&self) -> Option<String> {
///         Some(self.text().to_string())
///     }
///
///     fn is_accessible_checked(&self) -> Option<bool> {
///         Some(self.is_checked())
///     }
///
///     fn accessible_actions(&self) -> Vec<Action> {
///         vec![Action::Click, Action::Focus]
///     }
/// }
/// ```
pub trait Accessible {
    /// Get the accessibility role of this widget.
    ///
    /// The role describes the widget's purpose to assistive technologies.
    fn accessible_role(&self) -> AccessibleRole {
        AccessibleRole::Unknown
    }

    /// Get the accessible name of this widget.
    ///
    /// This is the primary label that screen readers announce.
    /// For buttons, this is typically the button text.
    /// For inputs, this is the associated label text.
    ///
    /// Returns `None` to use the widget's object name.
    fn accessible_name(&self) -> Option<String> {
        None
    }

    /// Get the accessible description of this widget.
    ///
    /// This provides additional context beyond the name.
    /// For example, "Press Enter to submit the form".
    fn accessible_description(&self) -> Option<String> {
        None
    }

    /// Get the accessible value as a string.
    ///
    /// For widgets like sliders, progress bars, or text inputs,
    /// this returns the current value as a string.
    fn accessible_value(&self) -> Option<String> {
        None
    }

    /// Get the accessible value as a number.
    ///
    /// For numeric widgets like sliders or spin boxes.
    fn accessible_numeric_value(&self) -> Option<f64> {
        None
    }

    /// Get the minimum numeric value.
    fn accessible_min_value(&self) -> Option<f64> {
        None
    }

    /// Get the maximum numeric value.
    fn accessible_max_value(&self) -> Option<f64> {
        None
    }

    /// Get the numeric value step.
    fn accessible_value_step(&self) -> Option<f64> {
        None
    }

    /// Get the checked/toggled state for checkable widgets.
    ///
    /// Returns `Some(true)` for checked, `Some(false)` for unchecked,
    /// `None` for non-checkable widgets.
    fn is_accessible_checked(&self) -> Option<bool> {
        None
    }

    /// Get the mixed/indeterminate state for tri-state checkboxes.
    ///
    /// Returns `true` if the checkbox is in an indeterminate state.
    fn is_accessible_mixed(&self) -> bool {
        false
    }

    /// Get the expanded state for expandable widgets.
    ///
    /// Returns `Some(true)` if expanded, `Some(false)` if collapsed,
    /// `None` for non-expandable widgets.
    fn is_accessible_expanded(&self) -> Option<bool> {
        None
    }

    /// Get the selected state for selectable widgets.
    ///
    /// Returns `Some(true)` if selected, `Some(false)` if not selected,
    /// `None` for non-selectable widgets.
    fn is_accessible_selected(&self) -> Option<bool> {
        None
    }

    /// Get the placeholder text for input widgets.
    fn accessible_placeholder(&self) -> Option<String> {
        None
    }

    /// Check if the widget is a password field (should hide content).
    fn is_accessible_password(&self) -> bool {
        false
    }

    /// Check if the widget is read-only.
    fn is_accessible_read_only(&self) -> bool {
        false
    }

    /// Check if the widget is required (for form inputs).
    fn is_accessible_required(&self) -> bool {
        false
    }

    /// Check if the widget has an error state.
    fn is_accessible_invalid(&self) -> bool {
        false
    }

    /// Get the error message for invalid state.
    fn accessible_error_message(&self) -> Option<String> {
        None
    }

    /// Get the actions supported by this widget.
    ///
    /// Common actions include:
    /// - `Action::Click` - for clickable widgets
    /// - `Action::Focus` - for focusable widgets
    /// - `Action::Increment` / `Action::Decrement` - for range widgets
    /// - `Action::Expand` / `Action::Collapse` - for expandable widgets
    fn accessible_actions(&self) -> Vec<Action> {
        Vec::new()
    }

    /// Get IDs of widgets that label this widget.
    ///
    /// Used to associate a Label widget with an input field.
    fn accessible_labelled_by(&self) -> Vec<ObjectId> {
        Vec::new()
    }

    /// Get IDs of widgets that describe this widget.
    fn accessible_described_by(&self) -> Vec<ObjectId> {
        Vec::new()
    }

    /// Get the active descendant for composite widgets.
    ///
    /// For widgets like lists or trees, this returns the currently
    /// active/focused child item.
    fn accessible_active_descendant(&self) -> Option<ObjectId> {
        None
    }

    /// Get the row index for table/grid items.
    fn accessible_row_index(&self) -> Option<usize> {
        None
    }

    /// Get the column index for table/grid items.
    fn accessible_column_index(&self) -> Option<usize> {
        None
    }

    /// Get the row count for tables/grids.
    fn accessible_row_count(&self) -> Option<usize> {
        None
    }

    /// Get the column count for tables/grids.
    fn accessible_column_count(&self) -> Option<usize> {
        None
    }

    /// Get the position in set (1-indexed) for list/tree items.
    fn accessible_position_in_set(&self) -> Option<usize> {
        None
    }

    /// Get the set size for list/tree items.
    fn accessible_set_size(&self) -> Option<usize> {
        None
    }

    /// Get the level for hierarchical items (1-indexed).
    fn accessible_level(&self) -> Option<usize> {
        None
    }

    /// Build an AccessKit Node from this widget's accessibility info.
    ///
    /// This is called by the AccessibilityManager to build the accessibility tree.
    /// Widgets typically don't need to override this.
    fn build_accessible_node(&self, bounds: Rect, children: &[ObjectId]) -> Node {
        let role = self.accessible_role().to_accesskit_role();
        let mut node = Node::new(role);

        // Set bounds
        node.set_bounds(accesskit::Rect {
            x0: bounds.origin.x as f64,
            y0: bounds.origin.y as f64,
            x1: (bounds.origin.x + bounds.size.width) as f64,
            y1: (bounds.origin.y + bounds.size.height) as f64,
        });

        // Set name
        if let Some(name) = self.accessible_name() {
            node.set_label(name);
        }

        // Set description
        if let Some(desc) = self.accessible_description() {
            node.set_description(desc);
        }

        // Set value
        if let Some(value) = self.accessible_value() {
            node.set_value(value);
        }

        // Set numeric value
        if let Some(value) = self.accessible_numeric_value() {
            node.set_numeric_value(value);
        }
        if let Some(min) = self.accessible_min_value() {
            node.set_min_numeric_value(min);
        }
        if let Some(max) = self.accessible_max_value() {
            node.set_max_numeric_value(max);
        }
        if let Some(step) = self.accessible_value_step() {
            node.set_numeric_value_step(step);
        }

        // Set checked/toggled state
        if let Some(checked) = self.is_accessible_checked() {
            if self.is_accessible_mixed() {
                node.set_toggled(Toggled::Mixed);
            } else if checked {
                node.set_toggled(Toggled::True);
            } else {
                node.set_toggled(Toggled::False);
            }
        }

        // Set expanded state
        if let Some(expanded) = self.is_accessible_expanded() {
            node.set_expanded(expanded);
        }

        // Set selected state
        if let Some(selected) = self.is_accessible_selected() {
            node.set_selected(selected);
        }

        // Set placeholder
        if let Some(placeholder) = self.accessible_placeholder() {
            node.set_placeholder(placeholder);
        }

        // Note: AccessKit doesn't have a direct password field flag.
        // Password fields should use Role::TextInput and be marked appropriately
        // by the platform accessibility layer.

        // Set read-only state
        if self.is_accessible_read_only() {
            node.set_read_only();
        }

        // Set required state
        if self.is_accessible_required() {
            node.set_required();
        }

        // Set invalid state
        if self.is_accessible_invalid() {
            node.set_invalid(accesskit::Invalid::True);
            if let Some(msg) = self.accessible_error_message() {
                // Error message would be in described_by relationship
                let _ = msg; // Use in future if AccessKit adds error_message
            }
        }

        // Set actions
        for action in self.accessible_actions() {
            node.add_action(action);
        }

        // Set labelled_by relationship
        let labelled_by: Vec<_> = self
            .accessible_labelled_by()
            .into_iter()
            .map(object_id_to_node_id)
            .collect();
        if !labelled_by.is_empty() {
            node.set_labelled_by(labelled_by);
        }

        // Set described_by relationship
        let described_by: Vec<_> = self
            .accessible_described_by()
            .into_iter()
            .map(object_id_to_node_id)
            .collect();
        if !described_by.is_empty() {
            node.set_described_by(described_by);
        }

        // Set active descendant
        if let Some(active) = self.accessible_active_descendant() {
            node.set_active_descendant(object_id_to_node_id(active));
        }

        // Set table/grid properties
        if let Some(row_index) = self.accessible_row_index() {
            node.set_row_index(row_index);
        }
        if let Some(col_index) = self.accessible_column_index() {
            node.set_column_index(col_index);
        }
        if let Some(row_count) = self.accessible_row_count() {
            node.set_row_count(row_count);
        }
        if let Some(col_count) = self.accessible_column_count() {
            node.set_column_count(col_count);
        }

        // Set list/tree properties
        if let Some(pos) = self.accessible_position_in_set() {
            node.set_position_in_set(pos);
        }
        if let Some(size) = self.accessible_set_size() {
            node.set_size_of_set(size);
        }
        if let Some(level) = self.accessible_level() {
            node.set_level(level);
        }

        // Set children
        let child_ids: Vec<_> = children
            .iter()
            .map(|id| object_id_to_node_id(*id))
            .collect();
        if !child_ids.is_empty() {
            node.set_children(child_ids);
        }

        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    /// A simple test widget for accessibility testing.
    struct TestAccessibleWidget {
        name: String,
        checked: bool,
    }

    impl Accessible for TestAccessibleWidget {
        fn accessible_role(&self) -> AccessibleRole {
            AccessibleRole::CheckBox
        }

        fn accessible_name(&self) -> Option<String> {
            Some(self.name.clone())
        }

        fn is_accessible_checked(&self) -> Option<bool> {
            Some(self.checked)
        }

        fn accessible_actions(&self) -> Vec<Action> {
            vec![Action::Click, Action::Focus]
        }
    }

    #[test]
    fn test_accessible_trait_defaults() {
        init_global_registry();

        struct MinimalWidget;
        impl Accessible for MinimalWidget {}

        let widget = MinimalWidget;
        assert_eq!(widget.accessible_role(), AccessibleRole::Unknown);
        assert!(widget.accessible_name().is_none());
        assert!(widget.accessible_description().is_none());
        assert!(widget.is_accessible_checked().is_none());
        assert!(widget.accessible_actions().is_empty());
    }

    #[test]
    fn test_accessible_implementation() {
        init_global_registry();

        let widget = TestAccessibleWidget {
            name: "Accept terms".to_string(),
            checked: true,
        };

        assert_eq!(widget.accessible_role(), AccessibleRole::CheckBox);
        assert_eq!(widget.accessible_name(), Some("Accept terms".to_string()));
        assert_eq!(widget.is_accessible_checked(), Some(true));
        assert_eq!(widget.accessible_actions().len(), 2);
    }

    #[test]
    fn test_build_accessible_node() {
        init_global_registry();

        let widget = TestAccessibleWidget {
            name: "Test Checkbox".to_string(),
            checked: false,
        };

        let bounds = Rect::new(10.0, 20.0, 100.0, 30.0);
        let node = widget.build_accessible_node(bounds, &[]);

        assert_eq!(node.role(), accesskit::Role::CheckBox);
    }
}
