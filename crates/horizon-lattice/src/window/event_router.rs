//! Window event routing module.
//!
//! This module provides utilities for routing winit window events to the
//! Horizon Lattice window system.
//!
//! # Usage
//!
//! Install the standard window event router after creating the application:
//!
//! ```ignore
//! use horizon_lattice::window::install_window_event_router;
//! use horizon_lattice_core::Application;
//!
//! let app = Application::new()?;
//! install_window_event_router();
//! app.run()?;
//! ```
//!
//! This will automatically route window resize, move, and focus events
//! to the `WindowManager`, emitting appropriate signals that you can
//! connect to.

use winit::event::WindowEvent;
use winit::window::WindowId;

use horizon_lattice_core::Application;

use super::native_window::NativeWindowId;
use super::window_manager::WindowManager;

/// Install the standard window event router.
///
/// This sets up an event handler on the application that routes winit
/// window events to the `WindowManager`. The router handles:
///
/// - `Resized`: Emits `WindowManager::window_resized` signal
/// - `Moved`: Emits `WindowManager::window_moved` signal
/// - `Focused(true)`: Emits `WindowManager::window_focused` signal
/// - `Focused(false)`: Emits `WindowManager::window_unfocused` signal
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{install_window_event_router, WindowManager};
///
/// // Install the router
/// install_window_event_router();
///
/// // Connect to window events
/// let manager = WindowManager::instance();
/// manager.window_resized().connect(|(id, width, height)| {
///     println!("Window {:?} resized to {}x{}", id, width, height);
/// });
/// ```
pub fn install_window_event_router() {
    let app = Application::instance();
    app.set_window_event_handler(handle_window_event);
}

/// Handle a single window event and route to WindowManager.
///
/// Returns `true` if the event was handled, `false` to let the application
/// perform default processing.
fn handle_window_event(window_id: WindowId, event: &WindowEvent) -> bool {
    let manager = WindowManager::instance();
    let native_id = NativeWindowId::from_winit(window_id);

    match event {
        WindowEvent::Resized(size) => {
            manager.notify_resize(native_id, size.width, size.height);
            // Return false to allow default processing (e.g., surface resize)
            false
        }
        WindowEvent::Moved(position) => {
            manager.notify_move(native_id, position.x, position.y);
            false
        }
        WindowEvent::Focused(focused) => {
            if *focused {
                manager.notify_focus(native_id);
            } else {
                manager.notify_unfocus(native_id);
            }
            false
        }
        WindowEvent::CloseRequested => {
            // Let the user handler or default behavior handle this
            false
        }
        _ => false,
    }
}

/// Create a custom window event handler that chains with additional logic.
///
/// This allows you to add custom event handling while still routing events
/// to the WindowManager.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::create_window_event_handler;
/// use winit::event::WindowEvent;
///
/// let handler = create_window_event_handler(|window_id, event| {
///     if let WindowEvent::KeyboardInput { .. } = event {
///         // Custom keyboard handling
///         return true;
///     }
///     false
/// });
///
/// Application::instance().set_window_event_handler(handler);
/// ```
pub fn create_window_event_handler<F>(custom_handler: F) -> impl Fn(WindowId, &WindowEvent) -> bool + Send + Sync + 'static
where
    F: Fn(WindowId, &WindowEvent) -> bool + Send + Sync + 'static,
{
    move |window_id: WindowId, event: &WindowEvent| {
        // First try custom handler
        if custom_handler(window_id, event) {
            return true;
        }

        // Then do standard routing
        handle_window_event(window_id, event)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_event_router_compiles() {
        // This is mainly a compilation test since creating real WindowIds
        // requires a window which needs a running event loop
    }
}
