//! Tests for the widget system.

#[cfg(test)]
mod tests {
    use horizon_lattice_core::{init_global_registry, Object, ObjectId};
    use horizon_lattice_render::{Color, Rect, Size};

    use crate::widget::{PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase};

    /// A simple test widget for verification.
    struct TestWidget {
        base: WidgetBase,
        #[allow(dead_code)] // Would be used in paint()
        color: Color,
    }

    impl TestWidget {
        fn new(color: Color) -> Self {
            Self {
                base: WidgetBase::new::<Self>(),
                color,
            }
        }
    }

    impl Object for TestWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for TestWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::from_dimensions(100.0, 50.0)
                .with_minimum_dimensions(50.0, 25.0)
                .with_maximum_dimensions(200.0, 100.0)
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {
            // Would draw a colored rectangle in real implementation
        }
    }

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_widget_creation() {
        setup();

        let widget = TestWidget::new(Color::RED);
        assert!(widget.is_visible());
        assert!(widget.is_enabled());
        assert!(!widget.has_focus());
        assert!(!widget.is_focusable());
    }

    #[test]
    fn test_widget_geometry() {
        setup();

        let mut widget = TestWidget::new(Color::BLUE);

        // Default geometry
        assert_eq!(widget.pos().x, 0.0);
        assert_eq!(widget.pos().y, 0.0);
        assert_eq!(widget.size().width, 0.0);
        assert_eq!(widget.size().height, 0.0);

        // Set geometry
        widget.set_geometry(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert_eq!(widget.pos().x, 10.0);
        assert_eq!(widget.pos().y, 20.0);
        assert_eq!(widget.width(), 100.0);
        assert_eq!(widget.height(), 50.0);

        // Local rect always starts at origin
        let local_rect = widget.rect();
        assert_eq!(local_rect.origin.x, 0.0);
        assert_eq!(local_rect.origin.y, 0.0);
        assert_eq!(local_rect.size.width, 100.0);
        assert_eq!(local_rect.size.height, 50.0);
    }

    #[test]
    fn test_widget_visibility() {
        setup();

        let mut widget = TestWidget::new(Color::GREEN);
        assert!(widget.is_visible());

        widget.hide();
        assert!(!widget.is_visible());

        widget.show();
        assert!(widget.is_visible());

        widget.set_visible(false);
        assert!(!widget.is_visible());
    }

    #[test]
    fn test_widget_enabled() {
        setup();

        let mut widget = TestWidget::new(Color::YELLOW);
        assert!(widget.is_enabled());

        widget.set_enabled(false);
        assert!(!widget.is_enabled());

        widget.set_enabled(true);
        assert!(widget.is_enabled());
    }

    #[test]
    fn test_widget_focusable() {
        setup();

        let mut widget = TestWidget::new(Color::CYAN);

        // Not focusable by default
        assert!(!widget.is_focusable());

        // Make focusable
        widget.set_focusable(true);
        assert!(widget.is_focusable());

        // Focusable is affected by enabled and visible
        widget.set_enabled(false);
        assert!(!widget.is_focusable()); // Disabled widgets can't be focused

        widget.set_enabled(true);
        widget.set_visible(false);
        assert!(!widget.is_focusable()); // Hidden widgets can't be focused
    }

    #[test]
    fn test_widget_size_hint() {
        setup();

        let widget = TestWidget::new(Color::MAGENTA);
        let hint = widget.size_hint();

        assert_eq!(hint.preferred.width, 100.0);
        assert_eq!(hint.preferred.height, 50.0);
        assert_eq!(hint.minimum.unwrap().width, 50.0);
        assert_eq!(hint.minimum.unwrap().height, 25.0);
        assert_eq!(hint.maximum.unwrap().width, 200.0);
        assert_eq!(hint.maximum.unwrap().height, 100.0);

        // Test constraining
        let constrained = hint.constrain(Size::new(30.0, 10.0));
        assert_eq!(constrained.width, 50.0); // Clamped to minimum
        assert_eq!(constrained.height, 25.0); // Clamped to minimum

        let constrained = hint.constrain(Size::new(300.0, 200.0));
        assert_eq!(constrained.width, 200.0); // Clamped to maximum
        assert_eq!(constrained.height, 100.0); // Clamped to maximum
    }

    #[test]
    fn test_widget_size_policy() {
        setup();

        let mut widget = TestWidget::new(Color::WHITE);

        // Default policy is Preferred for both dimensions
        let policy = widget.size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Preferred);
        assert_eq!(policy.vertical, SizePolicy::Preferred);

        // Set a custom policy
        widget.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed));
        let policy = widget.size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Expanding);
        assert_eq!(policy.vertical, SizePolicy::Fixed);
    }

    #[test]
    fn test_widget_coordinate_mapping() {
        setup();

        let mut widget = TestWidget::new(Color::BLACK);
        widget.set_geometry(Rect::new(100.0, 200.0, 50.0, 30.0));

        // Map local to parent
        let local_point = horizon_lattice_render::Point::new(10.0, 15.0);
        let parent_point = widget.map_to_parent(local_point);
        assert_eq!(parent_point.x, 110.0);
        assert_eq!(parent_point.y, 215.0);

        // Map parent to local
        let back_to_local = widget.map_from_parent(parent_point);
        assert_eq!(back_to_local.x, 10.0);
        assert_eq!(back_to_local.y, 15.0);
    }

    #[test]
    fn test_widget_contains_point() {
        setup();

        let mut widget = TestWidget::new(Color::GRAY);
        widget.set_geometry(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Point inside
        assert!(widget.contains_point(horizon_lattice_render::Point::new(50.0, 25.0)));
        assert!(widget.contains_point(horizon_lattice_render::Point::new(0.0, 0.0)));

        // Point outside
        assert!(!widget.contains_point(horizon_lattice_render::Point::new(100.0, 50.0))); // Right/bottom edge exclusive
        assert!(!widget.contains_point(horizon_lattice_render::Point::new(-1.0, 25.0)));
        assert!(!widget.contains_point(horizon_lattice_render::Point::new(50.0, 51.0)));
    }

    #[test]
    fn test_widget_repaint_flag() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Initially needs repaint
        assert!(widget.needs_repaint());

        // Update requests repaint
        widget.update();
        assert!(widget.needs_repaint());
    }

    #[test]
    fn test_widget_naming() {
        setup();

        let widget = TestWidget::new(Color::BLUE);
        widget.widget_base().set_name("test_button");
        assert_eq!(widget.widget_base().name(), "test_button");
    }

    // =========================================================================
    // Pressed State Tests
    // =========================================================================

    #[test]
    fn test_widget_pressed_state() {
        setup();

        let widget = TestWidget::new(Color::RED);

        // Not pressed by default
        assert!(!widget.is_pressed());
        assert!(!widget.is_hovered());
    }

    // =========================================================================
    // State Propagation Tests
    // =========================================================================

    #[test]
    fn test_effectively_visible_no_parent() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Widget with no parent: effective visibility = own visibility
        assert!(widget.is_visible());
        assert!(widget.is_effectively_visible());

        widget.hide();
        assert!(!widget.is_visible());
        assert!(!widget.is_effectively_visible());
    }

    #[test]
    fn test_effectively_visible_with_parent() {
        setup();

        let mut parent = TestWidget::new(Color::RED);
        let child = TestWidget::new(Color::BLUE);

        // Set up parent-child relationship
        child
            .widget_base()
            .set_parent(Some(parent.object_id()))
            .unwrap();

        // Both visible: child is effectively visible
        assert!(parent.is_visible());
        assert!(child.is_visible());
        assert!(child.is_effectively_visible());

        // Hide parent: child's own visibility unchanged, but effectively hidden
        parent.hide();
        assert!(!parent.is_visible());
        assert!(child.is_visible()); // Own flag unchanged
        assert!(!child.is_effectively_visible()); // But effectively hidden

        // Show parent again: child becomes effectively visible
        parent.show();
        assert!(child.is_effectively_visible());
    }

    #[test]
    fn test_effectively_enabled_no_parent() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Widget with no parent: effective enabled = own enabled
        assert!(widget.is_enabled());
        assert!(widget.is_effectively_enabled());

        widget.set_enabled(false);
        assert!(!widget.is_enabled());
        assert!(!widget.is_effectively_enabled());
    }

    #[test]
    fn test_effectively_enabled_with_parent() {
        setup();

        let mut parent = TestWidget::new(Color::RED);
        let child = TestWidget::new(Color::BLUE);

        // Set up parent-child relationship
        child
            .widget_base()
            .set_parent(Some(parent.object_id()))
            .unwrap();

        // Both enabled: child is effectively enabled
        assert!(parent.is_enabled());
        assert!(child.is_enabled());
        assert!(child.is_effectively_enabled());

        // Disable parent: child's own enabled unchanged, but effectively disabled
        parent.set_enabled(false);
        assert!(!parent.is_enabled());
        assert!(child.is_enabled()); // Own flag unchanged
        assert!(!child.is_effectively_enabled()); // But effectively disabled

        // Enable parent again: child becomes effectively enabled
        parent.set_enabled(true);
        assert!(child.is_effectively_enabled());
    }

    #[test]
    fn test_deeply_nested_state_propagation() {
        setup();

        let mut grandparent = TestWidget::new(Color::RED);
        let parent = TestWidget::new(Color::GREEN);
        let child = TestWidget::new(Color::BLUE);

        // Build hierarchy: grandparent -> parent -> child
        parent
            .widget_base()
            .set_parent(Some(grandparent.object_id()))
            .unwrap();
        child
            .widget_base()
            .set_parent(Some(parent.object_id()))
            .unwrap();

        // All visible: child is effectively visible
        assert!(child.is_effectively_visible());
        assert!(child.is_effectively_enabled());

        // Hide grandparent: affects all descendants
        grandparent.hide();
        assert!(parent.is_visible()); // Own flag unchanged
        assert!(!parent.is_effectively_visible()); // But effectively hidden
        assert!(child.is_visible()); // Own flag unchanged
        assert!(!child.is_effectively_visible()); // But effectively hidden

        // Show grandparent: all visible again
        grandparent.show();
        assert!(child.is_effectively_visible());

        // Disable grandparent: affects all descendants
        grandparent.set_enabled(false);
        assert!(child.is_enabled()); // Own flag unchanged
        assert!(!child.is_effectively_enabled()); // But effectively disabled
    }

    #[test]
    fn test_reparenting_updates_effective_state() {
        setup();

        let visible_parent = TestWidget::new(Color::RED);
        let mut hidden_parent = TestWidget::new(Color::GREEN);
        let child = TestWidget::new(Color::BLUE);

        hidden_parent.hide();

        // Initially under visible parent
        child
            .widget_base()
            .set_parent(Some(visible_parent.object_id()))
            .unwrap();
        assert!(child.is_effectively_visible());

        // Move to hidden parent
        child
            .widget_base()
            .set_parent(Some(hidden_parent.object_id()))
            .unwrap();
        assert!(child.is_visible()); // Own flag unchanged
        assert!(!child.is_effectively_visible()); // Now effectively hidden

        // Move back to visible parent
        child
            .widget_base()
            .set_parent(Some(visible_parent.object_id()))
            .unwrap();
        assert!(child.is_effectively_visible()); // Visible again
    }
}
