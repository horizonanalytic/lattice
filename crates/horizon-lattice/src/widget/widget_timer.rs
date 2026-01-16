//! Widget timer system.
//!
//! This module provides infrastructure for widgets to own and receive timer events.
//! It bridges the application-level timer system with widget-level event dispatch.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use horizon_lattice_core::{Application, ObjectId, TimerId};
use parking_lot::Mutex;

/// Global mapping from timer IDs to the widgets that own them.
static WIDGET_TIMERS: OnceLock<Mutex<WidgetTimerRegistry>> = OnceLock::new();

/// Registry that tracks which widget owns each timer.
#[derive(Default)]
struct WidgetTimerRegistry {
    /// Maps timer IDs to the widget that owns them.
    timer_to_widget: HashMap<TimerId, ObjectId>,
}

/// Get the global widget timer registry, initializing it if necessary.
fn get_registry() -> &'static Mutex<WidgetTimerRegistry> {
    WIDGET_TIMERS.get_or_init(|| Mutex::new(WidgetTimerRegistry::default()))
}

/// Start a one-shot timer owned by a widget.
///
/// When the timer fires, the widget will receive a `WidgetEvent::Timer` event.
///
/// # Arguments
///
/// * `widget_id` - The ObjectId of the widget that will own this timer
/// * `duration` - How long until the timer fires
///
/// # Returns
///
/// The TimerId that can be used to stop the timer.
pub fn start_widget_timer(widget_id: ObjectId, duration: Duration) -> TimerId {
    let app = Application::instance();
    let timer_id = app.start_timer(duration);

    let mut registry = get_registry().lock();
    registry.timer_to_widget.insert(timer_id, widget_id);

    timer_id
}

/// Start a repeating timer owned by a widget.
///
/// When the timer fires, the widget will receive a `WidgetEvent::Timer` event.
/// The timer will continue firing at the specified interval until stopped.
///
/// # Arguments
///
/// * `widget_id` - The ObjectId of the widget that will own this timer
/// * `interval` - The interval between timer firings
///
/// # Returns
///
/// The TimerId that can be used to stop the timer.
pub fn start_widget_repeating_timer(widget_id: ObjectId, interval: Duration) -> TimerId {
    let app = Application::instance();
    let timer_id = app.start_repeating_timer(interval);

    let mut registry = get_registry().lock();
    registry.timer_to_widget.insert(timer_id, widget_id);

    timer_id
}

/// Stop a widget-owned timer.
///
/// # Arguments
///
/// * `timer_id` - The timer to stop
///
/// # Returns
///
/// `true` if the timer was found and stopped, `false` otherwise.
pub fn stop_widget_timer(timer_id: TimerId) -> bool {
    let app = Application::instance();

    // Remove from registry
    {
        let mut registry = get_registry().lock();
        registry.timer_to_widget.remove(&timer_id);
    }

    // Stop the underlying timer
    app.stop_timer(timer_id).is_ok()
}

/// Check if a timer is active.
pub fn is_widget_timer_active(timer_id: TimerId) -> bool {
    Application::instance().is_timer_active(timer_id)
}

/// Look up which widget owns a timer.
///
/// This is used by the event dispatch system to route timer events
/// to the appropriate widget.
///
/// # Arguments
///
/// * `timer_id` - The timer to look up
///
/// # Returns
///
/// The ObjectId of the widget that owns the timer, or `None` if the timer
/// is not registered to any widget.
pub fn get_timer_owner(timer_id: TimerId) -> Option<ObjectId> {
    let registry = get_registry().lock();
    registry.timer_to_widget.get(&timer_id).copied()
}

/// Remove all timers owned by a widget.
///
/// This should be called when a widget is destroyed to clean up its timers.
///
/// # Arguments
///
/// * `widget_id` - The widget whose timers should be removed
pub fn remove_timers_for_widget(widget_id: ObjectId) {
    // Get the application if available (may not be initialized in tests)
    let app = Application::try_instance();

    let mut registry = get_registry().lock();

    // Find all timers owned by this widget
    let timer_ids: Vec<TimerId> = registry
        .timer_to_widget
        .iter()
        .filter(|(_, owner)| **owner == widget_id)
        .map(|(id, _)| *id)
        .collect();

    // Remove them
    for timer_id in timer_ids {
        registry.timer_to_widget.remove(&timer_id);
        if let Some(app) = app {
            let _ = app.stop_timer(timer_id);
        }
    }
}
