//! Window geometry persistence.
//!
//! This module provides types and utilities for saving and restoring
//! window geometry across application sessions.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::window::{NativeWindow, WindowGeometry};
//!
//! // Save the current window geometry
//! let geometry = window.save_geometry();
//!
//! // Persist to storage (JSON, config file, etc.)
//! let json = serde_json::to_string(&geometry)?;
//! std::fs::write("window_state.json", json)?;
//!
//! // Later, restore the geometry
//! let json = std::fs::read_to_string("window_state.json")?;
//! let geometry: WindowGeometry = serde_json::from_str(&json)?;
//! window.restore_geometry(&geometry);
//! ```
//!
//! # Screen Change Handling
//!
//! When restoring geometry, the module handles cases where the saved
//! screen configuration no longer matches the current setup:
//!
//! - If the saved monitor no longer exists, the window is centered on the primary monitor
//! - If the saved position would place the window off-screen, it is adjusted to be visible
//! - Size is clamped to fit within the available screen bounds

use crate::platform::{Screen, ScreenRect, Screens};
use crate::widget::widgets::WindowState;

/// Saved window geometry for persistence.
///
/// This struct captures all the information needed to restore a window
/// to its previous state, including position, size, and window state.
///
/// # Coordinate System
///
/// All coordinates are stored in logical pixels for DPI independence.
/// When restoring, coordinates are converted to physical pixels using
/// the target screen's scale factor.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowGeometry {
    /// X position in logical pixels (top-left corner).
    pub x: i32,
    /// Y position in logical pixels (top-left corner).
    pub y: i32,
    /// Width in logical pixels.
    pub width: u32,
    /// Height in logical pixels.
    pub height: u32,
    /// Window state when geometry was saved.
    pub state: WindowState,
    /// Name of the screen the window was on (for multi-monitor restoration).
    /// This is used as a hint; if the screen no longer exists, the window
    /// will be placed on the primary screen.
    pub screen_name: Option<String>,
}

impl WindowGeometry {
    /// Create a new window geometry.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            state: WindowState::Normal,
            screen_name: None,
        }
    }

    /// Set the window state.
    pub fn with_state(mut self, state: WindowState) -> Self {
        self.state = state;
        self
    }

    /// Set the screen name hint.
    pub fn with_screen_name(mut self, name: impl Into<String>) -> Self {
        self.screen_name = Some(name.into());
        self
    }

    /// Validate and adjust the geometry to ensure the window is visible.
    ///
    /// This method:
    /// 1. Finds the appropriate screen (by name, or falls back to primary)
    /// 2. Ensures the window is at least partially visible on that screen
    /// 3. Clamps the size to fit within the screen's work area
    ///
    /// Returns the adjusted geometry.
    pub fn validated(&self) -> Self {
        let screens = Screens::all().unwrap_or_default();
        if screens.is_empty() {
            return self.clone();
        }

        // Find the target screen
        let target_screen = self.find_target_screen(&screens);

        // Validate and adjust position/size for this screen
        self.adjust_for_screen(&target_screen)
    }

    /// Find the appropriate screen to restore the window to.
    fn find_target_screen<'a>(&self, screens: &'a [Screen]) -> &'a Screen {
        // Try to find by name first
        if let Some(ref name) = self.screen_name {
            if let Some(screen) = screens.iter().find(|s| s.name() == name) {
                return screen;
            }
        }

        // Try to find the screen containing the saved position
        if let Some(screen) = screens.iter().find(|s| {
            let g = s.geometry();
            g.contains(self.x, self.y)
        }) {
            return screen;
        }

        // Fall back to primary screen
        screens
            .iter()
            .find(|s| s.is_primary())
            .unwrap_or(&screens[0])
    }

    /// Adjust geometry to fit within the given screen.
    fn adjust_for_screen(&self, screen: &Screen) -> Self {
        let work_area = screen.work_area();
        let mut adjusted = self.clone();

        // Clamp size to work area (with some minimum size)
        const MIN_SIZE: u32 = 100;
        adjusted.width = adjusted
            .width
            .max(MIN_SIZE)
            .min(work_area.width);
        adjusted.height = adjusted
            .height
            .max(MIN_SIZE)
            .min(work_area.height);

        // Ensure window is at least partially visible
        // We require at least 50 pixels of the title bar to be visible
        const MIN_VISIBLE: i32 = 50;

        // Adjust X position
        let max_x = work_area.x + work_area.width as i32 - MIN_VISIBLE;
        let min_x = work_area.x - adjusted.width as i32 + MIN_VISIBLE;
        adjusted.x = adjusted.x.clamp(min_x, max_x);

        // Adjust Y position (ensure title bar is accessible)
        let max_y = work_area.y + work_area.height as i32 - MIN_VISIBLE;
        adjusted.y = adjusted.y.clamp(work_area.y, max_y);

        adjusted
    }

    /// Check if this geometry would be visible on any connected screen.
    pub fn is_visible(&self) -> bool {
        let screens = Screens::all().unwrap_or_default();
        if screens.is_empty() {
            return false;
        }

        // Check if the window overlaps with any screen's work area
        screens.iter().any(|screen| {
            let work_area = screen.work_area();
            self.overlaps_rect(&work_area)
        })
    }

    /// Check if this geometry overlaps with a rectangle.
    fn overlaps_rect(&self, rect: &ScreenRect) -> bool {
        let self_right = self.x + self.width as i32;
        let self_bottom = self.y + self.height as i32;
        let rect_right = rect.x + rect.width as i32;
        let rect_bottom = rect.y + rect.height as i32;

        self.x < rect_right
            && self_right > rect.x
            && self.y < rect_bottom
            && self_bottom > rect.y
    }
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 800,
            height: 600,
            state: WindowState::Normal,
            screen_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_geometry_new() {
        let geom = WindowGeometry::new(100, 200, 800, 600);
        assert_eq!(geom.x, 100);
        assert_eq!(geom.y, 200);
        assert_eq!(geom.width, 800);
        assert_eq!(geom.height, 600);
        assert_eq!(geom.state, WindowState::Normal);
        assert!(geom.screen_name.is_none());
    }

    #[test]
    fn test_window_geometry_builders() {
        let geom = WindowGeometry::new(0, 0, 640, 480)
            .with_state(WindowState::Maximized)
            .with_screen_name("Display 1");

        assert_eq!(geom.state, WindowState::Maximized);
        assert_eq!(geom.screen_name, Some("Display 1".to_string()));
    }

    #[test]
    fn test_window_geometry_default() {
        let geom = WindowGeometry::default();
        assert_eq!(geom.x, 100);
        assert_eq!(geom.y, 100);
        assert_eq!(geom.width, 800);
        assert_eq!(geom.height, 600);
        assert_eq!(geom.state, WindowState::Normal);
    }

    #[test]
    fn test_overlaps_rect() {
        let geom = WindowGeometry::new(100, 100, 200, 200);

        // Overlapping rect
        let rect = ScreenRect::new(150, 150, 100, 100);
        assert!(geom.overlaps_rect(&rect));

        // Non-overlapping rect (to the right)
        let rect = ScreenRect::new(400, 100, 100, 100);
        assert!(!geom.overlaps_rect(&rect));

        // Non-overlapping rect (below)
        let rect = ScreenRect::new(100, 400, 100, 100);
        assert!(!geom.overlaps_rect(&rect));

        // Adjacent but not overlapping
        let rect = ScreenRect::new(300, 100, 100, 100);
        assert!(!geom.overlaps_rect(&rect));
    }

    #[test]
    fn test_adjust_for_screen() {
        // Create a mock screen
        let screen = Screen::new_for_testing(
            0,
            "Test Screen".to_string(),
            ScreenRect::new(0, 0, 1920, 1080),
            ScreenRect::new(0, 0, 1920, 1040), // 40px taskbar
            1.0,
            true,
        );

        // Window completely off-screen to the left
        let geom = WindowGeometry::new(-1000, 100, 400, 300);
        let adjusted = geom.adjust_for_screen(&screen);
        assert!(adjusted.x >= -350); // At least 50px visible

        // Window completely off-screen below
        let geom = WindowGeometry::new(100, 2000, 400, 300);
        let adjusted = geom.adjust_for_screen(&screen);
        assert!(adjusted.y <= 1040 - 50); // At least 50px visible

        // Window too large for screen
        let geom = WindowGeometry::new(0, 0, 3000, 2000);
        let adjusted = geom.adjust_for_screen(&screen);
        assert!(adjusted.width <= 1920);
        assert!(adjusted.height <= 1040);
    }
}
