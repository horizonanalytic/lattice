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

    // =========================================================================
    // Event Propagation Tests
    // =========================================================================

    use crate::widget::{
        DispatchResult, EventDispatcher, Key, KeyPressEvent, KeyboardModifiers, MouseButton,
        MousePressEvent, WidgetAccess, WidgetEvent,
    };
    use horizon_lattice_render::Point;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// A widget that tracks events it receives.
    struct EventTrackingWidget {
        base: WidgetBase,
        events_received: Arc<Mutex<Vec<String>>>,
        accept_events: bool,
    }

    impl EventTrackingWidget {
        fn new(name: &str, events: Arc<Mutex<Vec<String>>>, accept_events: bool) -> Self {
            let base = WidgetBase::new::<Self>();
            base.set_name(name);
            Self {
                base,
                events_received: events,
                accept_events,
            }
        }
    }

    impl Object for EventTrackingWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for EventTrackingWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::from_dimensions(100.0, 50.0)
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}

        fn event(&mut self, event: &mut WidgetEvent) -> bool {
            let name = self.base.name();
            let event_type = match event {
                WidgetEvent::MousePress(_) => "MousePress",
                WidgetEvent::MouseRelease(_) => "MouseRelease",
                WidgetEvent::KeyPress(_) => "KeyPress",
                WidgetEvent::KeyRelease(_) => "KeyRelease",
                _ => "Other",
            };
            self.events_received
                .lock()
                .unwrap()
                .push(format!("{}:{}", name, event_type));

            if self.accept_events {
                event.accept();
                true
            } else {
                false
            }
        }

        fn event_filter(&mut self, event: &mut WidgetEvent, _target: ObjectId) -> bool {
            let name = self.base.name();
            let event_type = match event {
                WidgetEvent::MousePress(_) => "MousePress",
                WidgetEvent::KeyPress(_) => "KeyPress",
                _ => "Other",
            };
            self.events_received
                .lock()
                .unwrap()
                .push(format!("{}:filter:{}", name, event_type));

            // Filter blocks the event if this widget accepts events
            self.accept_events
        }
    }

    /// Simple widget storage for testing
    struct TestWidgetStorage {
        widgets: HashMap<ObjectId, Box<dyn Widget>>,
        children: HashMap<ObjectId, Vec<ObjectId>>,
    }

    impl TestWidgetStorage {
        fn new() -> Self {
            Self {
                widgets: HashMap::new(),
                children: HashMap::new(),
            }
        }

        fn add(&mut self, widget: impl Widget + 'static) -> ObjectId {
            let id = widget.object_id();
            self.widgets.insert(id, Box::new(widget));
            id
        }

        fn set_children(&mut self, parent: ObjectId, children: Vec<ObjectId>) {
            self.children.insert(parent, children);
        }
    }

    impl WidgetAccess for TestWidgetStorage {
        fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget> {
            self.widgets.get(&id).map(|w| w.as_ref())
        }

        fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(&id).map(|w| w.as_mut())
        }

        fn get_children(&self, id: ObjectId) -> Vec<ObjectId> {
            self.children.get(&id).cloned().unwrap_or_default()
        }
    }

    #[test]
    fn test_event_dispatch_direct() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget = EventTrackingWidget::new("button", events.clone(), true);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut event = WidgetEvent::MousePress(MousePressEvent::new(
            MouseButton::Left,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            KeyboardModifiers::NONE,
        ));

        let result = EventDispatcher::send_event(&mut storage, widget_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        assert_eq!(events.lock().unwrap().as_slice(), &["button:MousePress"]);
    }

    #[test]
    fn test_event_bubble_up() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        // Create parent (accepts events) and child (doesn't accept)
        let parent = EventTrackingWidget::new("parent", events.clone(), true);
        let child = EventTrackingWidget::new("child", events.clone(), false);

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        // Set up parent-child relationship
        child
            .widget_base()
            .set_parent(Some(parent_id))
            .unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        let mut event = WidgetEvent::MousePress(MousePressEvent::new(
            MouseButton::Left,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            KeyboardModifiers::NONE,
        ));

        // Send to child - should bubble up to parent
        let result = EventDispatcher::send_event(&mut storage, child_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        // Child receives first, then parent (bubble up)
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &["child:MousePress", "parent:MousePress"]
        );
    }

    #[test]
    fn test_event_accepted_stops_propagation() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        // Child accepts events, so parent should NOT receive
        let parent = EventTrackingWidget::new("parent", events.clone(), true);
        let child = EventTrackingWidget::new("child", events.clone(), true); // Accepts!

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        child
            .widget_base()
            .set_parent(Some(parent_id))
            .unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        let mut event = WidgetEvent::MousePress(MousePressEvent::new(
            MouseButton::Left,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            KeyboardModifiers::NONE,
        ));

        let result = EventDispatcher::send_event(&mut storage, child_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        // Only child receives (accepts event, stops propagation)
        assert_eq!(events.lock().unwrap().as_slice(), &["child:MousePress"]);
    }

    #[test]
    fn test_event_filter_blocks_event() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        let filter = EventTrackingWidget::new("filter", events.clone(), true); // Will block
        let mut target = EventTrackingWidget::new("target", events.clone(), true);

        let filter_id = filter.object_id();
        let target_id = target.object_id();

        // Install filter on target
        target
            .widget_base_mut()
            .install_event_filter(filter_id);

        let mut storage = TestWidgetStorage::new();
        storage.add(filter);
        storage.add(target);

        let mut event = WidgetEvent::MousePress(MousePressEvent::new(
            MouseButton::Left,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            KeyboardModifiers::NONE,
        ));

        let result = EventDispatcher::send_event(&mut storage, target_id, &mut event);

        // Filter blocked the event
        assert_eq!(result, DispatchResult::Filtered);
        // Only filter received (target never got the event)
        assert_eq!(events.lock().unwrap().as_slice(), &["filter:filter:MousePress"]);
    }

    #[test]
    fn test_event_filter_passes_event() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        let filter = EventTrackingWidget::new("filter", events.clone(), false); // Won't block
        let mut target = EventTrackingWidget::new("target", events.clone(), true);

        let filter_id = filter.object_id();
        let target_id = target.object_id();

        // Install filter on target
        target
            .widget_base_mut()
            .install_event_filter(filter_id);

        let mut storage = TestWidgetStorage::new();
        storage.add(filter);
        storage.add(target);

        let mut event = WidgetEvent::MousePress(MousePressEvent::new(
            MouseButton::Left,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            KeyboardModifiers::NONE,
        ));

        let result = EventDispatcher::send_event(&mut storage, target_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        // Filter saw it, then target handled it
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &["filter:filter:MousePress", "target:MousePress"]
        );
    }

    #[test]
    fn test_keyboard_event_creation() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget = EventTrackingWidget::new("editor", events.clone(), true);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut event = WidgetEvent::KeyPress(KeyPressEvent::new(
            Key::A,
            KeyboardModifiers::NONE,
            "a",
            false,
        ));

        let result = EventDispatcher::send_event(&mut storage, widget_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        assert_eq!(events.lock().unwrap().as_slice(), &["editor:KeyPress"]);
    }

    #[test]
    fn test_keyboard_event_bubble_up() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        // Child doesn't handle key events, parent does
        let parent = EventTrackingWidget::new("parent", events.clone(), true);
        let child = EventTrackingWidget::new("child", events.clone(), false);

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        child
            .widget_base()
            .set_parent(Some(parent_id))
            .unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        let mut event = WidgetEvent::KeyPress(KeyPressEvent::new(
            Key::Escape,
            KeyboardModifiers::NONE,
            "",
            false,
        ));

        let result = EventDispatcher::send_event(&mut storage, child_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        // Bubbled from child to parent
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &["child:KeyPress", "parent:KeyPress"]
        );
    }

    #[test]
    fn test_event_filter_install_remove() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        let filter1 = TestWidget::new(Color::GREEN);
        let filter2 = TestWidget::new(Color::BLUE);

        let filter1_id = filter1.object_id();
        let filter2_id = filter2.object_id();

        // No filters initially
        assert!(widget.widget_base().event_filters().is_empty());

        // Install filters
        widget.widget_base_mut().install_event_filter(filter1_id);
        widget.widget_base_mut().install_event_filter(filter2_id);

        assert_eq!(widget.widget_base().event_filters().len(), 2);
        assert!(widget.widget_base().has_event_filter(filter1_id));
        assert!(widget.widget_base().has_event_filter(filter2_id));

        // Remove one filter
        widget.widget_base_mut().remove_event_filter(filter1_id);

        assert_eq!(widget.widget_base().event_filters().len(), 1);
        assert!(!widget.widget_base().has_event_filter(filter1_id));
        assert!(widget.widget_base().has_event_filter(filter2_id));

        // Clear all filters
        widget.widget_base_mut().clear_event_filters();
        assert!(widget.widget_base().event_filters().is_empty());
    }

    #[test]
    fn test_event_filter_no_duplicates() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        let filter = TestWidget::new(Color::GREEN);
        let filter_id = filter.object_id();

        widget.widget_base_mut().install_event_filter(filter_id);
        widget.widget_base_mut().install_event_filter(filter_id); // Duplicate
        widget.widget_base_mut().install_event_filter(filter_id); // Duplicate

        // Should only have one
        assert_eq!(widget.widget_base().event_filters().len(), 1);
    }

    #[test]
    fn test_ancestor_chain() {
        setup();

        let grandparent = TestWidget::new(Color::RED);
        let parent = TestWidget::new(Color::GREEN);
        let child = TestWidget::new(Color::BLUE);

        let grandparent_id = grandparent.object_id();
        let parent_id = parent.object_id();
        let child_id = child.object_id();

        parent
            .widget_base()
            .set_parent(Some(grandparent_id))
            .unwrap();
        child
            .widget_base()
            .set_parent(Some(parent_id))
            .unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(grandparent);
        storage.add(parent);
        storage.add(child);

        let ancestors = EventDispatcher::get_ancestor_chain(&storage, child_id);

        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], parent_id); // Immediate parent first
        assert_eq!(ancestors[1], grandparent_id); // Then grandparent
    }

    #[test]
    fn test_hit_test_basic() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_geometry(Rect::new(10.0, 10.0, 100.0, 50.0));
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Point inside widget
        let hit = EventDispatcher::hit_test(&storage, widget_id, Point::new(50.0, 30.0));
        assert_eq!(hit, Some(widget_id));

        // Point outside widget
        let hit = EventDispatcher::hit_test(&storage, widget_id, Point::new(5.0, 5.0));
        assert_eq!(hit, None);
    }

    #[test]
    fn test_hit_test_nested() {
        setup();

        // Parent at (10, 10), size 200x100
        let mut parent = TestWidget::new(Color::RED);
        parent.set_geometry(Rect::new(10.0, 10.0, 200.0, 100.0));
        let parent_id = parent.object_id();

        // Child at (20, 20) relative to parent, size 50x30
        let mut child = TestWidget::new(Color::BLUE);
        child.set_geometry(Rect::new(20.0, 20.0, 50.0, 30.0));
        let child_id = child.object_id();

        child
            .widget_base()
            .set_parent(Some(parent_id))
            .unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);
        storage.set_children(parent_id, vec![child_id]);

        // Point in child (window coords: 10+20+10=40, 10+20+10=40)
        let hit = EventDispatcher::hit_test(&storage, parent_id, Point::new(40.0, 40.0));
        assert_eq!(hit, Some(child_id));

        // Point in parent but outside child
        let hit = EventDispatcher::hit_test(&storage, parent_id, Point::new(150.0, 50.0));
        assert_eq!(hit, Some(parent_id));

        // Point completely outside
        let hit = EventDispatcher::hit_test(&storage, parent_id, Point::new(5.0, 5.0));
        assert_eq!(hit, None);
    }

    #[test]
    fn test_hit_test_hidden_widget() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        widget.hide(); // Hidden!
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Point would be inside, but widget is hidden
        let hit = EventDispatcher::hit_test(&storage, widget_id, Point::new(50.0, 50.0));
        assert_eq!(hit, None);
    }

    // =========================================================================
    // Focus Management Tests
    // =========================================================================

    use crate::widget::{FocusManager, FocusPolicy, FocusReason};

    /// A widget that tracks focus events.
    struct FocusTrackingWidget {
        base: WidgetBase,
        focus_events: Arc<Mutex<Vec<String>>>,
    }

    impl FocusTrackingWidget {
        fn new(name: &str, events: Arc<Mutex<Vec<String>>>, policy: FocusPolicy) -> Self {
            let base = WidgetBase::new::<Self>();
            base.set_name(name);
            let mut widget = Self {
                base,
                focus_events: events,
            };
            widget.base.set_focus_policy(policy);
            widget
        }
    }

    impl Object for FocusTrackingWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for FocusTrackingWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::from_dimensions(100.0, 50.0)
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}

        fn event(&mut self, event: &mut WidgetEvent) -> bool {
            let name = self.base.name();
            match event {
                WidgetEvent::FocusIn(_) => {
                    self.focus_events
                        .lock()
                        .unwrap()
                        .push(format!("{}:FocusIn", name));
                    true
                }
                WidgetEvent::FocusOut(_) => {
                    self.focus_events
                        .lock()
                        .unwrap()
                        .push(format!("{}:FocusOut", name));
                    true
                }
                _ => false,
            }
        }
    }

    #[test]
    fn test_focus_policy_default() {
        setup();

        let widget = TestWidget::new(Color::RED);

        // Default focus policy is NoFocus
        assert_eq!(widget.focus_policy(), FocusPolicy::NoFocus);
        assert!(!widget.is_focusable());
        assert!(!widget.accepts_tab_focus());
        assert!(!widget.accepts_click_focus());
    }

    #[test]
    fn test_focus_policy_strong_focus() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_focus_policy(FocusPolicy::StrongFocus);

        assert_eq!(widget.focus_policy(), FocusPolicy::StrongFocus);
        assert!(widget.is_focusable());
        assert!(widget.accepts_tab_focus());
        assert!(widget.accepts_click_focus());
    }

    #[test]
    fn test_focus_policy_tab_focus() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_focus_policy(FocusPolicy::TabFocus);

        assert_eq!(widget.focus_policy(), FocusPolicy::TabFocus);
        assert!(widget.is_focusable());
        assert!(widget.accepts_tab_focus());
        assert!(!widget.accepts_click_focus());
    }

    #[test]
    fn test_focus_policy_click_focus() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_focus_policy(FocusPolicy::ClickFocus);

        assert_eq!(widget.focus_policy(), FocusPolicy::ClickFocus);
        assert!(widget.is_focusable());
        assert!(!widget.accepts_tab_focus());
        assert!(widget.accepts_click_focus());
    }

    #[test]
    fn test_focusable_requires_enabled_and_visible() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_focus_policy(FocusPolicy::StrongFocus);

        // Initially focusable
        assert!(widget.is_focusable());

        // Disabled widgets cannot be focused
        widget.set_enabled(false);
        assert!(!widget.is_focusable());
        assert!(!widget.accepts_tab_focus());

        // Re-enable
        widget.set_enabled(true);
        assert!(widget.is_focusable());

        // Hidden widgets cannot be focused
        widget.hide();
        assert!(!widget.is_focusable());
        assert!(!widget.accepts_tab_focus());
    }

    #[test]
    fn test_set_focusable_convenience_method() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // set_focusable(true) sets StrongFocus policy
        widget.set_focusable(true);
        assert_eq!(widget.focus_policy(), FocusPolicy::StrongFocus);
        assert!(widget.is_focusable());

        // set_focusable(false) sets NoFocus policy
        widget.set_focusable(false);
        assert_eq!(widget.focus_policy(), FocusPolicy::NoFocus);
        assert!(!widget.is_focusable());
    }

    #[test]
    fn test_focus_manager_set_focus() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget = FocusTrackingWidget::new("button", events.clone(), FocusPolicy::StrongFocus);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut focus_manager = FocusManager::new();

        // No widget focused initially
        assert!(focus_manager.focused_widget().is_none());

        // Set focus to widget
        let result = focus_manager.set_focus(&mut storage, widget_id, FocusReason::Other);
        assert!(result);
        assert_eq!(focus_manager.focused_widget(), Some(widget_id));

        // Widget should have received FocusIn event
        assert_eq!(events.lock().unwrap().as_slice(), &["button:FocusIn"]);
    }

    #[test]
    fn test_focus_manager_change_focus() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget1 = FocusTrackingWidget::new("widget1", events.clone(), FocusPolicy::StrongFocus);
        let widget2 = FocusTrackingWidget::new("widget2", events.clone(), FocusPolicy::StrongFocus);
        let widget1_id = widget1.object_id();
        let widget2_id = widget2.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget1);
        storage.add(widget2);

        let mut focus_manager = FocusManager::new();

        // Focus widget1
        focus_manager.set_focus(&mut storage, widget1_id, FocusReason::Other);
        assert_eq!(focus_manager.focused_widget(), Some(widget1_id));

        // Clear events
        events.lock().unwrap().clear();

        // Change focus to widget2
        focus_manager.set_focus(&mut storage, widget2_id, FocusReason::Tab);
        assert_eq!(focus_manager.focused_widget(), Some(widget2_id));

        // Should see FocusOut on widget1, then FocusIn on widget2
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &["widget1:FocusOut", "widget2:FocusIn"]
        );
    }

    #[test]
    fn test_focus_manager_clear_focus() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget = FocusTrackingWidget::new("button", events.clone(), FocusPolicy::StrongFocus);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut focus_manager = FocusManager::new();
        focus_manager.set_focus(&mut storage, widget_id, FocusReason::Other);

        // Clear events
        events.lock().unwrap().clear();

        // Clear focus
        focus_manager.clear_focus(&mut storage, FocusReason::Other);
        assert!(focus_manager.focused_widget().is_none());

        // Widget should have received FocusOut event
        assert_eq!(events.lock().unwrap().as_slice(), &["button:FocusOut"]);
    }

    #[test]
    fn test_focus_manager_cannot_focus_nofocus_widget() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let widget = FocusTrackingWidget::new("label", events.clone(), FocusPolicy::NoFocus);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut focus_manager = FocusManager::new();

        // Attempting to focus a NoFocus widget should fail
        let result = focus_manager.set_focus(&mut storage, widget_id, FocusReason::Other);
        assert!(!result);
        assert!(focus_manager.focused_widget().is_none());

        // No events should be sent
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn test_focus_manager_cannot_focus_disabled_widget() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut widget = FocusTrackingWidget::new("button", events.clone(), FocusPolicy::StrongFocus);
        widget.set_enabled(false);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        let mut focus_manager = FocusManager::new();

        // Attempting to focus a disabled widget should fail
        let result = focus_manager.set_focus(&mut storage, widget_id, FocusReason::Other);
        assert!(!result);
        assert!(focus_manager.focused_widget().is_none());
    }

    #[test]
    fn test_focus_next_previous() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        // Create a simple widget tree: root -> [child1, child2, child3]
        let root = FocusTrackingWidget::new("root", events.clone(), FocusPolicy::NoFocus);
        let child1 = FocusTrackingWidget::new("child1", events.clone(), FocusPolicy::StrongFocus);
        let child2 = FocusTrackingWidget::new("child2", events.clone(), FocusPolicy::StrongFocus);
        let child3 = FocusTrackingWidget::new("child3", events.clone(), FocusPolicy::StrongFocus);

        let root_id = root.object_id();
        let child1_id = child1.object_id();
        let child2_id = child2.object_id();
        let child3_id = child3.object_id();

        // Set up parent-child relationships
        child1.widget_base().set_parent(Some(root_id)).unwrap();
        child2.widget_base().set_parent(Some(root_id)).unwrap();
        child3.widget_base().set_parent(Some(root_id)).unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(root);
        storage.add(child1);
        storage.add(child2);
        storage.add(child3);
        storage.set_children(root_id, vec![child1_id, child2_id, child3_id]);

        let mut focus_manager = FocusManager::new();

        // focus_next with no current focus should focus first widget
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child1_id));

        // focus_next should move to child2
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child2_id));

        // focus_next should move to child3
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child3_id));

        // focus_next should wrap to child1
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child1_id));

        // focus_previous should go back to child3
        focus_manager.focus_previous(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child3_id));
    }

    #[test]
    fn test_focus_navigation_skips_nofocus_and_hidden() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        let root = FocusTrackingWidget::new("root", events.clone(), FocusPolicy::NoFocus);
        let child1 = FocusTrackingWidget::new("child1", events.clone(), FocusPolicy::StrongFocus);
        let child2 = FocusTrackingWidget::new("child2", events.clone(), FocusPolicy::NoFocus); // NoFocus!
        let mut child3 = FocusTrackingWidget::new("child3", events.clone(), FocusPolicy::StrongFocus);
        child3.hide(); // Hidden!
        let child4 = FocusTrackingWidget::new("child4", events.clone(), FocusPolicy::StrongFocus);

        let root_id = root.object_id();
        let child1_id = child1.object_id();
        let child2_id = child2.object_id();
        let child3_id = child3.object_id();
        let child4_id = child4.object_id();

        child1.widget_base().set_parent(Some(root_id)).unwrap();
        child2.widget_base().set_parent(Some(root_id)).unwrap();
        child3.widget_base().set_parent(Some(root_id)).unwrap();
        child4.widget_base().set_parent(Some(root_id)).unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(root);
        storage.add(child1);
        storage.add(child2);
        storage.add(child3);
        storage.add(child4);
        storage.set_children(root_id, vec![child1_id, child2_id, child3_id, child4_id]);

        let mut focus_manager = FocusManager::new();

        // Focus child1
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child1_id));

        // focus_next should skip child2 (NoFocus) and child3 (hidden), go to child4
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child4_id));

        // focus_next should wrap to child1 (skipping child2 and child3)
        focus_manager.focus_next(&mut storage, root_id);
        assert_eq!(focus_manager.focused_widget(), Some(child1_id));
    }

    #[test]
    fn test_focus_changed_signal() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget.set_focusable(true);

        // Track signal emissions
        let focus_states: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
        let states_clone = focus_states.clone();

        widget.widget_base().focus_changed.connect(move |focused| {
            states_clone.lock().unwrap().push(*focused);
        });

        // Directly set focus (simulating what FocusManager does)
        widget.widget_base_mut().set_focused(true);
        assert!(widget.has_focus());
        assert_eq!(focus_states.lock().unwrap().as_slice(), &[true]);

        widget.widget_base_mut().set_focused(false);
        assert!(!widget.has_focus());
        assert_eq!(focus_states.lock().unwrap().as_slice(), &[true, false]);
    }

    // =========================================================================
    // Widget Destruction Tests
    // =========================================================================

    #[test]
    fn test_destroyed_signal_fires() {
        setup();

        let destroyed_ids: Arc<Mutex<Vec<ObjectId>>> = Arc::new(Mutex::new(Vec::new()));
        let destroyed_clone = destroyed_ids.clone();

        let widget_id;
        {
            let widget = TestWidget::new(Color::RED);
            widget_id = widget.object_id();

            widget.widget_base().destroyed.connect(move |id| {
                destroyed_clone.lock().unwrap().push(*id);
            });

            // Widget is still alive here
            assert!(destroyed_ids.lock().unwrap().is_empty());
        }
        // Widget dropped here - destroyed signal should have fired

        let ids = destroyed_ids.lock().unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], widget_id);
    }

    #[test]
    fn test_destroyed_signal_on_parent_child() {
        setup();

        let destroyed_ids: Arc<Mutex<Vec<ObjectId>>> = Arc::new(Mutex::new(Vec::new()));
        let destroyed_clone = destroyed_ids.clone();
        let destroyed_clone2 = destroyed_ids.clone();

        let parent_id;
        let child_id;
        {
            let parent = TestWidget::new(Color::RED);
            let child = TestWidget::new(Color::BLUE);

            parent_id = parent.object_id();
            child_id = child.object_id();

            child.widget_base().set_parent(Some(parent_id)).unwrap();

            parent.widget_base().destroyed.connect(move |id| {
                destroyed_clone.lock().unwrap().push(*id);
            });
            child.widget_base().destroyed.connect(move |id| {
                destroyed_clone2.lock().unwrap().push(*id);
            });
        }
        // Both widgets dropped - both signals should have fired

        let ids = destroyed_ids.lock().unwrap();
        assert_eq!(ids.len(), 2);
        // Both IDs should be present (order may vary due to drop order)
        assert!(ids.contains(&parent_id));
        assert!(ids.contains(&child_id));
    }

    #[test]
    fn test_destroyed_signal_with_multiple_connections() {
        setup();

        let counter1: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let counter2: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let c1 = counter1.clone();
        let c2 = counter2.clone();

        {
            let widget = TestWidget::new(Color::RED);

            widget.widget_base().destroyed.connect(move |_| {
                *c1.lock().unwrap() += 1;
            });
            widget.widget_base().destroyed.connect(move |_| {
                *c2.lock().unwrap() += 1;
            });
        }
        // Widget dropped

        assert_eq!(*counter1.lock().unwrap(), 1);
        assert_eq!(*counter2.lock().unwrap(), 1);
    }

    // =========================================================================
    // Custom Event Tests
    // =========================================================================

    use crate::widget::CustomEvent;

    /// A custom event payload for testing.
    #[derive(Debug, Clone, PartialEq)]
    struct TestCustomPayload {
        message: String,
        value: i32,
    }

    /// Another custom event payload for testing type differentiation.
    #[derive(Debug, Clone, PartialEq)]
    struct AnotherPayload {
        flag: bool,
    }

    #[test]
    fn test_custom_event_creation() {
        setup();

        let payload = TestCustomPayload {
            message: "Hello".into(),
            value: 42,
        };
        let event = CustomEvent::new(payload.clone());

        assert!(!event.base.is_accepted());
        assert!(event.is::<TestCustomPayload>());
        assert!(!event.is::<AnotherPayload>());
    }

    #[test]
    fn test_custom_event_with_name() {
        setup();

        let event = CustomEvent::with_name(
            TestCustomPayload {
                message: "test".into(),
                value: 1,
            },
            "MyCustomEvent",
        );

        assert_eq!(event.name(), Some("MyCustomEvent"));
    }

    #[test]
    fn test_custom_event_downcast_ref() {
        setup();

        let payload = TestCustomPayload {
            message: "Hello".into(),
            value: 42,
        };
        let event = CustomEvent::new(payload.clone());

        // Correct type
        let retrieved = event.downcast_ref::<TestCustomPayload>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &payload);

        // Wrong type
        let wrong = event.downcast_ref::<AnotherPayload>();
        assert!(wrong.is_none());
    }

    #[test]
    fn test_custom_event_downcast_mut() {
        setup();

        let payload = TestCustomPayload {
            message: "Hello".into(),
            value: 42,
        };
        let mut event = CustomEvent::new(payload);

        // Modify through downcast_mut
        if let Some(data) = event.downcast_mut::<TestCustomPayload>() {
            data.value = 100;
            data.message = "Modified".into();
        }

        // Verify modification
        let retrieved = event.downcast_ref::<TestCustomPayload>();
        assert_eq!(retrieved.unwrap().value, 100);
        assert_eq!(retrieved.unwrap().message, "Modified");
    }

    #[test]
    fn test_custom_event_in_widget_event() {
        setup();

        let payload = TestCustomPayload {
            message: "Test".into(),
            value: 99,
        };
        let mut event = WidgetEvent::Custom(CustomEvent::new(payload.clone()));

        // Check it's a custom event
        assert!(event.as_custom().is_some());
        assert!(event.as_custom().unwrap().is::<TestCustomPayload>());

        // Access the payload
        if let Some(custom) = event.as_custom() {
            let data = custom.downcast_ref::<TestCustomPayload>().unwrap();
            assert_eq!(data, &payload);
        }

        // Accept/ignore works
        assert!(!event.is_accepted());
        event.accept();
        assert!(event.is_accepted());
    }

    #[test]
    fn test_custom_event_dispatch() {
        setup();

        let events = Arc::new(Mutex::new(Vec::new()));

        // A widget that handles custom events
        struct CustomEventWidget {
            base: WidgetBase,
            received_values: Arc<Mutex<Vec<i32>>>,
        }

        impl Object for CustomEventWidget {
            fn object_id(&self) -> ObjectId {
                self.base.object_id()
            }
        }

        impl Widget for CustomEventWidget {
            fn widget_base(&self) -> &WidgetBase {
                &self.base
            }

            fn widget_base_mut(&mut self) -> &mut WidgetBase {
                &mut self.base
            }

            fn size_hint(&self) -> SizeHint {
                SizeHint::from_dimensions(100.0, 50.0)
            }

            fn paint(&self, _ctx: &mut PaintContext<'_>) {}

            fn event(&mut self, event: &mut WidgetEvent) -> bool {
                if let Some(custom) = event.as_custom() {
                    if let Some(payload) = custom.downcast_ref::<TestCustomPayload>() {
                        self.received_values.lock().unwrap().push(payload.value);
                        event.accept();
                        return true;
                    }
                }
                false
            }
        }

        let widget = CustomEventWidget {
            base: WidgetBase::new::<CustomEventWidget>(),
            received_values: events.clone(),
        };
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Dispatch a custom event
        let mut event = WidgetEvent::Custom(CustomEvent::new(TestCustomPayload {
            message: "dispatch test".into(),
            value: 123,
        }));

        let result = EventDispatcher::send_event(&mut storage, widget_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        assert_eq!(events.lock().unwrap().as_slice(), &[123]);
    }

    #[test]
    fn test_custom_event_propagation() {
        setup();

        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        // Parent handles custom events, child does not
        struct PropagationTestWidget {
            base: WidgetBase,
            name: String,
            handle_custom: bool,
            received: Arc<Mutex<Vec<String>>>,
        }

        impl Object for PropagationTestWidget {
            fn object_id(&self) -> ObjectId {
                self.base.object_id()
            }
        }

        impl Widget for PropagationTestWidget {
            fn widget_base(&self) -> &WidgetBase {
                &self.base
            }

            fn widget_base_mut(&mut self) -> &mut WidgetBase {
                &mut self.base
            }

            fn size_hint(&self) -> SizeHint {
                SizeHint::from_dimensions(100.0, 50.0)
            }

            fn paint(&self, _ctx: &mut PaintContext<'_>) {}

            fn event(&mut self, event: &mut WidgetEvent) -> bool {
                if let WidgetEvent::Custom(_) = event {
                    self.received.lock().unwrap().push(self.name.clone());
                    if self.handle_custom {
                        event.accept();
                        return true;
                    }
                }
                false
            }
        }

        let parent = PropagationTestWidget {
            base: WidgetBase::new::<PropagationTestWidget>(),
            name: "parent".into(),
            handle_custom: true,
            received: received.clone(),
        };
        let child = PropagationTestWidget {
            base: WidgetBase::new::<PropagationTestWidget>(),
            name: "child".into(),
            handle_custom: false, // Won't accept, should propagate to parent
            received: received_clone,
        };

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        child.widget_base().set_parent(Some(parent_id)).unwrap();

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        // Dispatch custom event to child - should bubble up to parent
        let mut event = WidgetEvent::Custom(CustomEvent::new(TestCustomPayload {
            message: "propagate".into(),
            value: 1,
        }));

        let result = EventDispatcher::send_event(&mut storage, child_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        // Child received first, then propagated to parent
        assert_eq!(
            received.lock().unwrap().as_slice(),
            &["child", "parent"]
        );
    }

    // =========================================================================
    // Context Menu Tests
    // =========================================================================

    use crate::widget::{ContextMenuEvent, ContextMenuPolicy, ContextMenuReason};

    #[test]
    fn test_context_menu_policy_default() {
        setup();

        let widget = TestWidget::new(Color::RED);

        // Default context menu policy is DefaultContextMenu
        assert_eq!(
            widget.widget_base().context_menu_policy(),
            ContextMenuPolicy::DefaultContextMenu
        );
    }

    #[test]
    fn test_context_menu_policy_set() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Set to CustomContextMenu
        widget
            .widget_base_mut()
            .set_context_menu_policy(ContextMenuPolicy::CustomContextMenu);
        assert_eq!(
            widget.widget_base().context_menu_policy(),
            ContextMenuPolicy::CustomContextMenu
        );

        // Set to NoContextMenu
        widget
            .widget_base_mut()
            .set_context_menu_policy(ContextMenuPolicy::NoContextMenu);
        assert_eq!(
            widget.widget_base().context_menu_policy(),
            ContextMenuPolicy::NoContextMenu
        );
    }

    #[test]
    fn test_context_menu_event_creation() {
        setup();

        // From mouse
        let event = ContextMenuEvent::from_mouse(
            Point::new(10.0, 20.0),
            Point::new(50.0, 60.0),
            Point::new(100.0, 200.0),
        );
        assert_eq!(event.local_pos.x, 10.0);
        assert_eq!(event.local_pos.y, 20.0);
        assert_eq!(event.reason, ContextMenuReason::Mouse);

        // From keyboard
        let event = ContextMenuEvent::from_keyboard(
            Point::new(0.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(100.0, 100.0),
        );
        assert_eq!(event.reason, ContextMenuReason::Keyboard);
    }

    #[test]
    fn test_context_menu_event_dispatch() {
        setup();

        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        // A widget that handles context menu events
        struct ContextMenuWidget {
            base: WidgetBase,
            received: Arc<Mutex<Vec<ContextMenuReason>>>,
        }

        impl Object for ContextMenuWidget {
            fn object_id(&self) -> ObjectId {
                self.base.object_id()
            }
        }

        impl Widget for ContextMenuWidget {
            fn widget_base(&self) -> &WidgetBase {
                &self.base
            }

            fn widget_base_mut(&mut self) -> &mut WidgetBase {
                &mut self.base
            }

            fn size_hint(&self) -> SizeHint {
                SizeHint::from_dimensions(100.0, 50.0)
            }

            fn paint(&self, _ctx: &mut PaintContext<'_>) {}

            fn event(&mut self, event: &mut WidgetEvent) -> bool {
                if let WidgetEvent::ContextMenu(e) = event {
                    self.received.lock().unwrap().push(e.reason);
                    event.accept();
                    return true;
                }
                false
            }
        }

        let widget = ContextMenuWidget {
            base: WidgetBase::new::<ContextMenuWidget>(),
            received: received_clone,
        };
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Dispatch a context menu event
        let mut event = WidgetEvent::ContextMenu(ContextMenuEvent::from_mouse(
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
        ));

        let result = EventDispatcher::send_event(&mut storage, widget_id, &mut event);

        assert_eq!(result, DispatchResult::Accepted);
        assert_eq!(received.lock().unwrap().as_slice(), &[ContextMenuReason::Mouse]);
    }

    #[test]
    fn test_context_menu_requested_signal() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Set policy to CustomContextMenu
        widget
            .widget_base_mut()
            .set_context_menu_policy(ContextMenuPolicy::CustomContextMenu);

        // Track signal emissions
        let positions: Arc<Mutex<Vec<Point>>> = Arc::new(Mutex::new(Vec::new()));
        let positions_clone = positions.clone();

        widget
            .widget_base()
            .context_menu_requested
            .connect(move |pos| {
                positions_clone.lock().unwrap().push(*pos);
            });

        // Emit the signal (simulating what EventDispatcher::trigger_context_menu does)
        widget
            .widget_base()
            .context_menu_requested
            .emit(Point::new(50.0, 75.0));

        let received = positions.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].x, 50.0);
        assert_eq!(received[0].y, 75.0);
    }

    #[test]
    fn test_trigger_context_menu_no_policy() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget
            .widget_base_mut()
            .set_context_menu_policy(ContextMenuPolicy::NoContextMenu);
        let widget_id = widget.object_id();

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Trigger context menu - should be ignored
        let result = EventDispatcher::trigger_context_menu(
            &mut storage,
            widget_id,
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 10.0),
            ContextMenuReason::Mouse,
        );

        assert_eq!(result, DispatchResult::Ignored);
    }

    #[test]
    fn test_trigger_context_menu_custom_policy() {
        setup();

        let mut widget = TestWidget::new(Color::RED);
        widget
            .widget_base_mut()
            .set_context_menu_policy(ContextMenuPolicy::CustomContextMenu);
        let widget_id = widget.object_id();

        // Track signal emissions
        let positions: Arc<Mutex<Vec<Point>>> = Arc::new(Mutex::new(Vec::new()));
        let positions_clone = positions.clone();

        widget
            .widget_base()
            .context_menu_requested
            .connect(move |pos| {
                positions_clone.lock().unwrap().push(*pos);
            });

        let mut storage = TestWidgetStorage::new();
        storage.add(widget);

        // Trigger context menu - should emit signal
        let result = EventDispatcher::trigger_context_menu(
            &mut storage,
            widget_id,
            Point::new(25.0, 35.0),
            Point::new(50.0, 60.0),
            Point::new(100.0, 120.0),
            ContextMenuReason::Mouse,
        );

        assert_eq!(result, DispatchResult::Accepted);

        // Signal should have been emitted with local position
        let received = positions.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].x, 25.0);
        assert_eq!(received[0].y, 35.0);
    }

    // =========================================================================
    // Cursor Tests
    // =========================================================================

    use crate::widget::CursorShape;

    #[test]
    fn test_widget_cursor_default() {
        setup();

        let widget = TestWidget::new(Color::RED);

        // No cursor set by default (inherits from parent)
        assert_eq!(widget.widget_base().cursor(), None);
    }

    #[test]
    fn test_widget_cursor_set_unset() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // Set cursor
        widget.widget_base_mut().set_cursor(CursorShape::Hand);
        assert_eq!(widget.widget_base().cursor(), Some(CursorShape::Hand));

        // Change cursor
        widget.widget_base_mut().set_cursor(CursorShape::IBeam);
        assert_eq!(widget.widget_base().cursor(), Some(CursorShape::IBeam));

        // Unset cursor (inherit from parent)
        widget.widget_base_mut().unset_cursor();
        assert_eq!(widget.widget_base().cursor(), None);
    }

    #[test]
    fn test_cursor_effective_no_parent() {
        setup();

        let mut widget = TestWidget::new(Color::RED);

        // No cursor set, no parent -> default Arrow
        assert_eq!(widget.widget_base().effective_cursor(), CursorShape::Arrow);

        // With cursor set
        widget.widget_base_mut().set_cursor(CursorShape::Wait);
        assert_eq!(widget.widget_base().effective_cursor(), CursorShape::Wait);
    }

    #[test]
    fn test_cursor_resolution_from_widget_tree() {
        setup();

        let mut parent = TestWidget::new(Color::RED);
        let child = TestWidget::new(Color::BLUE);

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        // Set up parent-child relationship
        child.widget_base().set_parent(Some(parent_id)).unwrap();

        // Set cursor on parent
        parent.widget_base_mut().set_cursor(CursorShape::Hand);

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        // Resolve cursor from child - should find parent's cursor
        let cursor = EventDispatcher::get_effective_cursor(&storage, child_id);
        assert_eq!(cursor, CursorShape::Hand);
    }

    #[test]
    fn test_cursor_resolution_child_overrides_parent() {
        setup();

        let mut parent = TestWidget::new(Color::RED);
        let mut child = TestWidget::new(Color::BLUE);

        let parent_id = parent.object_id();
        let child_id = child.object_id();

        // Set up parent-child relationship
        child.widget_base().set_parent(Some(parent_id)).unwrap();

        // Set different cursors
        parent.widget_base_mut().set_cursor(CursorShape::Hand);
        child.widget_base_mut().set_cursor(CursorShape::IBeam);

        let mut storage = TestWidgetStorage::new();
        storage.add(parent);
        storage.add(child);

        // Child's cursor should take precedence
        let cursor = EventDispatcher::get_effective_cursor(&storage, child_id);
        assert_eq!(cursor, CursorShape::IBeam);
    }

    #[test]
    fn test_cursor_resolution_deep_hierarchy() {
        setup();

        let grandparent = TestWidget::new(Color::RED);
        let mut parent = TestWidget::new(Color::GREEN);
        let child = TestWidget::new(Color::BLUE);

        let grandparent_id = grandparent.object_id();
        let parent_id = parent.object_id();
        let child_id = child.object_id();

        // Set up hierarchy
        parent.widget_base().set_parent(Some(grandparent_id)).unwrap();
        child.widget_base().set_parent(Some(parent_id)).unwrap();

        // Only parent has cursor set
        parent.widget_base_mut().set_cursor(CursorShape::Crosshair);

        let mut storage = TestWidgetStorage::new();
        storage.add(grandparent);
        storage.add(parent);
        storage.add(child);

        // Child should inherit parent's cursor
        let cursor = EventDispatcher::get_effective_cursor(&storage, child_id);
        assert_eq!(cursor, CursorShape::Crosshair);
    }

    #[test]
    fn test_cursor_shape_is_resize() {
        // Test the is_resize_cursor helper
        assert!(CursorShape::ResizeEast.is_resize_cursor());
        assert!(CursorShape::ResizeWest.is_resize_cursor());
        assert!(CursorShape::ResizeNorth.is_resize_cursor());
        assert!(CursorShape::ResizeSouth.is_resize_cursor());
        assert!(CursorShape::ResizeHorizontal.is_resize_cursor());
        assert!(CursorShape::ResizeVertical.is_resize_cursor());
        assert!(CursorShape::ResizeNeSw.is_resize_cursor());
        assert!(CursorShape::ResizeNwSe.is_resize_cursor());
        assert!(CursorShape::ResizeColumn.is_resize_cursor());
        assert!(CursorShape::ResizeRow.is_resize_cursor());

        // Non-resize cursors
        assert!(!CursorShape::Arrow.is_resize_cursor());
        assert!(!CursorShape::Hand.is_resize_cursor());
        assert!(!CursorShape::IBeam.is_resize_cursor());
        assert!(!CursorShape::Wait.is_resize_cursor());
    }
}
