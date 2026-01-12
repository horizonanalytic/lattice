//! Integration tests for the #[derive(Object)] macro.

use horizon_lattice_core::object::{init_global_registry, ObjectId};
use horizon_lattice_core::property::Property;
use horizon_lattice_core::signal::Signal;
use horizon_lattice_core::{object::ObjectBase, Object};
use horizon_lattice_macros::Object;
use std::any::TypeId;

fn setup() {
    init_global_registry();
}

// Basic test struct using the derive macro
#[derive(Object)]
struct TestButton {
    base: ObjectBase,

    #[property(notify = "text_changed")]
    text: Property<String>,

    #[property(notify = "enabled_changed")]
    enabled: Property<bool>,

    #[signal]
    clicked: Signal<()>,

    #[signal]
    text_changed: Signal<String>,

    #[signal]
    enabled_changed: Signal<bool>,
}

// Manual Default implementation since ObjectBase doesn't implement Default
impl Default for TestButton {
    fn default() -> Self {
        Self {
            base: ObjectBase::new::<Self>(),
            text: Property::new(String::new()),
            enabled: Property::new(true),
            clicked: Signal::new(),
            text_changed: Signal::new(),
            enabled_changed: Signal::new(),
        }
    }
}

impl TestButton {
    fn new() -> Self {
        Self::default()
    }
}

// Test struct with read-only property
#[derive(Object)]
struct TestCounter {
    base: ObjectBase,

    #[property]
    count: Property<i32>,

    #[property(read_only)]
    is_positive: bool,

    #[signal]
    count_changed: Signal<i32>,
}

impl Default for TestCounter {
    fn default() -> Self {
        Self {
            base: ObjectBase::new::<Self>(),
            count: Property::new(0),
            is_positive: false,
            count_changed: Signal::new(),
        }
    }
}

impl TestCounter {
    fn new(initial: i32) -> Self {
        Self {
            base: ObjectBase::new::<Self>(),
            count: Property::new(initial),
            is_positive: initial > 0,
            count_changed: Signal::new(),
        }
    }
}

// Test struct with no factory (no Default requirement)
#[derive(Object)]
#[object(no_factory)]
struct CustomWidget {
    base: ObjectBase,

    #[property]
    value: i32,
}

impl CustomWidget {
    fn new(value: i32) -> Self {
        Self {
            base: ObjectBase::new::<Self>(),
            value,
        }
    }
}

// ============= Tests =============

#[test]
fn test_derive_generates_object_impl() {
    setup();
    let button = TestButton::new();

    // Should have a valid object ID
    let id = button.object_id();
    assert_ne!(id, ObjectId::default());
}

#[test]
fn test_derive_generates_meta_object() {
    setup();
    let button = TestButton::new();

    // Should have a meta-object
    let meta = button.meta_object();
    assert!(meta.is_some());

    let meta = meta.unwrap();
    assert_eq!(meta.type_name, "TestButton");
    assert_eq!(meta.type_id, TypeId::of::<TestButton>());
}

#[test]
fn test_meta_object_has_properties() {
    setup();
    let button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Check property count
    assert_eq!(meta.properties.len(), 2);

    // Find text property
    let text_prop = meta.property("text");
    assert!(text_prop.is_some());
    let text_prop = text_prop.unwrap();
    assert_eq!(text_prop.name, "text");
    assert_eq!(text_prop.type_id, TypeId::of::<String>());
    assert!(!text_prop.read_only);
    assert_eq!(text_prop.notify_signal, Some("text_changed"));

    // Find enabled property
    let enabled_prop = meta.property("enabled");
    assert!(enabled_prop.is_some());
    let enabled_prop = enabled_prop.unwrap();
    assert_eq!(enabled_prop.name, "enabled");
    assert_eq!(enabled_prop.type_id, TypeId::of::<bool>());
}

#[test]
fn test_meta_object_has_signals() {
    setup();
    let button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Check signal count
    assert_eq!(meta.signals.len(), 3);

    // Find clicked signal
    let clicked = meta.signal("clicked");
    assert!(clicked.is_some());
    let clicked = clicked.unwrap();
    assert_eq!(clicked.name, "clicked");
    assert!(clicked.param_types.is_empty());

    // Find text_changed signal
    let text_changed = meta.signal("text_changed");
    assert!(text_changed.is_some());
    let text_changed = text_changed.unwrap();
    assert_eq!(text_changed.name, "text_changed");
    assert_eq!(text_changed.param_types.len(), 1);
}

#[test]
fn test_property_getter_works() {
    setup();
    let button = TestButton::new();
    button.text.set_silent("Hello".to_string());

    let meta = button.meta_object().unwrap();

    // Use dynamic property access
    let text_value = meta.get_property(&button, "text").unwrap();
    let text: &String = text_value.downcast_ref().unwrap();
    assert_eq!(text, "Hello");
}

#[test]
fn test_property_setter_works() {
    setup();
    let mut button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Use dynamic property access to set
    meta.set_property(&mut button, "text", Box::new("World".to_string()))
        .unwrap();

    // Verify it was set
    assert_eq!(button.text.get(), "World");
}

#[test]
fn test_read_only_property() {
    setup();
    let mut counter = TestCounter::new(5);
    let meta = counter.meta_object().unwrap();

    // Find the read-only property
    let is_positive = meta.property("is_positive");
    assert!(is_positive.is_some());
    let is_positive = is_positive.unwrap();
    assert!(is_positive.read_only);
    assert!(is_positive.setter.is_none());

    // Trying to set should fail
    let result = meta.set_property(&mut counter, "is_positive", Box::new(false));
    assert!(result.is_err());
}

#[test]
fn test_no_factory_attribute() {
    setup();
    let widget = CustomWidget::new(42);
    let meta = widget.meta_object().unwrap();

    // Should have no factory function
    assert!(meta.create.is_none());
}

#[test]
fn test_factory_generates_default() {
    setup();
    let button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Should have a factory function
    assert!(meta.create.is_some());

    // Create instance via factory
    let factory = meta.create.unwrap();
    let new_obj = factory();

    // Should be the right type
    assert_eq!(new_obj.meta_object().unwrap().type_name, "TestButton");
}

#[test]
fn test_property_type_names() {
    setup();
    let button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Property names should include inherited
    let names = meta.property_names();
    assert!(names.contains(&"text"));
    assert!(names.contains(&"enabled"));
}

#[test]
fn test_signal_type_names() {
    setup();
    let button = TestButton::new();
    let meta = button.meta_object().unwrap();

    // Signal names should include all signals
    let names = meta.signal_names();
    assert!(names.contains(&"clicked"));
    assert!(names.contains(&"text_changed"));
    assert!(names.contains(&"enabled_changed"));
}

#[test]
fn test_multiple_signal_params() {
    setup();

    #[derive(Object)]
    #[object(no_factory)]
    struct MultiParamSignals {
        base: ObjectBase,

        #[signal]
        single_param: Signal<String>,

        #[signal]
        two_params: Signal<(String, i32)>,

        #[signal]
        no_params: Signal<()>,
    }

    impl MultiParamSignals {
        fn new() -> Self {
            Self {
                base: ObjectBase::new::<Self>(),
                single_param: Signal::new(),
                two_params: Signal::new(),
                no_params: Signal::new(),
            }
        }
    }

    let obj = MultiParamSignals::new();
    let meta = obj.meta_object().unwrap();

    // Check signal param types
    let single = meta.signal("single_param").unwrap();
    assert_eq!(single.param_types.len(), 1);

    let two = meta.signal("two_params").unwrap();
    // Two params represented as tuple types
    assert!(two.param_types.len() >= 1);

    let no = meta.signal("no_params").unwrap();
    assert!(no.param_types.is_empty());
}

// ============= TypeRegistry Integration Tests =============

use horizon_lattice_core::TypeRegistry;
use std::sync::Mutex;

// Mutex to serialize TypeRegistry tests that rely on global state
static TYPE_REGISTRY_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn setup_type_registry() -> std::sync::MutexGuard<'static, ()> {
    setup();
    let guard = TYPE_REGISTRY_TEST_MUTEX.lock().unwrap();
    TypeRegistry::clear();
    guard
}

#[test]
fn test_derive_generates_meta_constant() {
    let _guard = setup_type_registry();

    // The derive macro should generate a META constant
    let meta = TestButton::META;
    assert_eq!(meta.type_name, "TestButton");
    assert_eq!(meta.type_id, TypeId::of::<TestButton>());
}

#[test]
fn test_derive_generates_register_type() {
    let _guard = setup_type_registry();

    // Initially not registered
    assert!(!TypeRegistry::contains("TestButton"));

    // Use the generated register_type method
    TestButton::register_type();

    // Now should be registered
    assert!(TypeRegistry::contains("TestButton"));

    let meta = TypeRegistry::get_by_name("TestButton");
    assert!(meta.is_some());
    assert_eq!(meta.unwrap().type_name, "TestButton");
}

#[test]
fn test_type_registry_dynamic_creation() {
    let _guard = setup_type_registry();

    // Register the type
    TestButton::register_type();

    // Create instance dynamically by type name
    let obj = TypeRegistry::create("TestButton");
    assert!(obj.is_some());

    let obj = obj.unwrap();
    let meta = obj.meta_object().unwrap();
    assert_eq!(meta.type_name, "TestButton");
}

#[test]
fn test_type_registry_create_fails_without_factory() {
    let _guard = setup_type_registry();

    // Register a type with no_factory
    TypeRegistry::register(CustomWidget::META);

    // Create should fail because no factory
    let obj = TypeRegistry::create("CustomWidget");
    assert!(obj.is_none());
}

#[test]
fn test_type_registry_get_by_type() {
    let _guard = setup_type_registry();

    TestButton::register_type();

    // Look up using generic get<T>() method
    let meta = TypeRegistry::get::<TestButton>();
    assert!(meta.is_some());
    assert_eq!(meta.unwrap().type_name, "TestButton");
}

#[test]
fn test_multiple_types_registered() {
    let _guard = setup_type_registry();

    TestButton::register_type();
    TestCounter::register_type();

    // Both should be registered
    assert!(TypeRegistry::contains("TestButton"));
    assert!(TypeRegistry::contains("TestCounter"));
    assert_eq!(TypeRegistry::type_count(), 2);

    // Both should be creatable
    let button = TypeRegistry::create("TestButton");
    let counter = TypeRegistry::create("TestCounter");
    assert!(button.is_some());
    assert!(counter.is_some());

    // Verify they're different types
    assert_eq!(button.unwrap().meta_object().unwrap().type_name, "TestButton");
    assert_eq!(counter.unwrap().meta_object().unwrap().type_name, "TestCounter");
}
