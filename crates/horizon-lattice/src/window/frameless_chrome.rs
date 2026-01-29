//! Frameless window chrome and hit-testing.
//!
//! This module provides support for custom window chrome on frameless windows.
//! It defines hit-test regions for window dragging and resizing, allowing
//! applications to create custom title bars while maintaining standard
//! window interactions.
//!
//! # Overview
//!
//! When a window has no native decorations (frameless), the application is
//! responsible for providing the visual chrome (title bar, buttons, etc.).
//! This module handles the interaction logic:
//!
//! - **Resize borders**: Areas at the window edges that trigger resize operations
//! - **Draggable regions**: Areas (like a custom title bar) that drag the window
//! - **Interactive regions**: Areas within draggable regions that should receive
//!   normal mouse events (like buttons in the title bar)
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::window::{FramelessWindowChrome, ChromeHitTestResult};
//!
//! // Create chrome configuration
//! let chrome = FramelessWindowChrome::new()
//!     .with_resize_border(8.0)
//!     .with_title_bar_height(32.0);
//!
//! // In your mouse event handler:
//! let result = chrome.hit_test(mouse_pos, window_size);
//!
//! match result {
//!     ChromeHitTestResult::Caption => {
//!         // Start window drag
//!         window.drag_window()?;
//!     }
//!     ChromeHitTestResult::ResizeBorder(direction) => {
//!         // Start resize operation
//!         window.drag_resize_window(direction)?;
//!     }
//!     ChromeHitTestResult::Client => {
//!         // Normal mouse handling
//!     }
//!     _ => {}
//! }
//! ```

use horizon_lattice_render::{Point, Rect, Size};

/// Re-export of winit's ResizeDirection for convenience.
pub use winit::window::ResizeDirection;

/// Result of hit testing against frameless window chrome.
///
/// This determines what action should be taken when the user clicks
/// or drags in a particular area of a frameless window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHitTestResult {
    /// The point is in the client area - handle as normal widget event.
    Client,

    /// The point is in a draggable region (title bar).
    ///
    /// Clicking and dragging should initiate a window move operation.
    Caption,

    /// The point is in a resize border.
    ///
    /// Clicking and dragging should initiate a window resize operation
    /// in the specified direction.
    ResizeBorder(ResizeDirection),

    /// The point is in the system menu area (top-left corner).
    ///
    /// Clicking should show the system window menu. Double-clicking
    /// typically closes the window.
    SysMenu,

    /// The point is in the minimize button area.
    MinimizeButton,

    /// The point is in the maximize/restore button area.
    MaximizeButton,

    /// The point is in the close button area.
    CloseButton,
}

impl ChromeHitTestResult {
    /// Check if this result indicates a resize operation.
    pub fn is_resize(&self) -> bool {
        matches!(self, Self::ResizeBorder(_))
    }

    /// Check if this result indicates a draggable area.
    pub fn is_draggable(&self) -> bool {
        matches!(self, Self::Caption | Self::SysMenu)
    }

    /// Check if this result is a window button.
    pub fn is_button(&self) -> bool {
        matches!(
            self,
            Self::MinimizeButton | Self::MaximizeButton | Self::CloseButton
        )
    }
}

/// Configuration for frameless window chrome and hit-testing.
///
/// This struct defines the interactive regions of a frameless window,
/// allowing proper window drag and resize behavior with custom chrome.
///
/// # Defaults
///
/// - Resize border: 8 logical pixels
/// - Corner size: 16 logical pixels
/// - Title bar height: 32 logical pixels
/// - No custom draggable or interactive regions
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::FramelessWindowChrome;
///
/// // Basic configuration with title bar
/// let chrome = FramelessWindowChrome::new()
///     .with_title_bar_height(40.0)
///     .with_resize_border(6.0);
///
/// // Configuration with custom regions
/// let chrome = FramelessWindowChrome::new()
///     .with_title_bar_height(0.0)  // Disable default title bar
///     .with_draggable_region(Rect::new(100.0, 0.0, 400.0, 40.0))  // Custom drag region
///     .with_interactive_region(Rect::new(500.0, 0.0, 100.0, 40.0));  // Buttons area
/// ```
#[derive(Debug, Clone)]
pub struct FramelessWindowChrome {
    /// Border thickness for resize detection (in logical pixels).
    ///
    /// This defines the width of the invisible resize borders at
    /// the window edges. Default: 8.0
    resize_border: f32,

    /// Corner size for diagonal resize detection (in logical pixels).
    ///
    /// This defines the size of the corner regions that trigger
    /// diagonal resize (e.g., NorthWest, SouthEast). Default: 16.0
    corner_size: f32,

    /// Title bar height when using the default top area (in logical pixels).
    ///
    /// When non-zero, the top `title_bar_height` pixels of the window
    /// (excluding resize borders) are treated as draggable. Default: 32.0
    ///
    /// Set to 0 to disable the default title bar behavior and use
    /// custom draggable regions instead.
    title_bar_height: f32,

    /// Optional explicit title bar region.
    ///
    /// When set, this overrides the default title bar behavior.
    /// The region is specified in window-local coordinates.
    title_bar_region: Option<Rect>,

    /// Custom draggable regions (in addition to title bar).
    ///
    /// These regions will initiate window drag when clicked.
    /// Useful for side panels or other draggable areas.
    draggable_regions: Vec<Rect>,

    /// Non-draggable regions within draggable areas.
    ///
    /// These regions exclude areas from dragging, allowing widgets
    /// like buttons within the title bar to receive mouse events.
    interactive_regions: Vec<Rect>,

    /// Optional system menu button region (top-left area).
    sys_menu_region: Option<Rect>,

    /// Optional minimize button region.
    minimize_button_region: Option<Rect>,

    /// Optional maximize button region.
    maximize_button_region: Option<Rect>,

    /// Optional close button region.
    close_button_region: Option<Rect>,

    /// Whether resize borders are enabled.
    ///
    /// Set to false to disable edge resizing entirely. Default: true
    resize_enabled: bool,
}

impl Default for FramelessWindowChrome {
    fn default() -> Self {
        Self::new()
    }
}

impl FramelessWindowChrome {
    /// Create a new frameless chrome configuration with default values.
    ///
    /// Defaults:
    /// - Resize border: 8 pixels
    /// - Corner size: 16 pixels
    /// - Title bar height: 32 pixels
    /// - Resize enabled: true
    pub fn new() -> Self {
        Self {
            resize_border: 8.0,
            corner_size: 16.0,
            title_bar_height: 32.0,
            title_bar_region: None,
            draggable_regions: Vec::new(),
            interactive_regions: Vec::new(),
            sys_menu_region: None,
            minimize_button_region: None,
            maximize_button_region: None,
            close_button_region: None,
            resize_enabled: true,
        }
    }

    /// Create a minimal chrome configuration with no title bar.
    ///
    /// Useful for popup windows or custom shapes where you want
    /// resize borders but no default drag behavior.
    pub fn minimal() -> Self {
        Self {
            title_bar_height: 0.0,
            ..Self::new()
        }
    }

    /// Create a chrome configuration with no interactive regions.
    ///
    /// The entire window is treated as client area (no resize, no drag).
    /// Useful for splash screens or notification windows.
    pub fn none() -> Self {
        Self {
            resize_border: 0.0,
            corner_size: 0.0,
            title_bar_height: 0.0,
            resize_enabled: false,
            ..Self::new()
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Set the resize border thickness.
    pub fn with_resize_border(mut self, border: f32) -> Self {
        self.resize_border = border.max(0.0);
        self
    }

    /// Set the corner size for diagonal resizing.
    pub fn with_corner_size(mut self, size: f32) -> Self {
        self.corner_size = size.max(0.0);
        self
    }

    /// Set the default title bar height.
    ///
    /// Set to 0 to disable the default title bar.
    pub fn with_title_bar_height(mut self, height: f32) -> Self {
        self.title_bar_height = height.max(0.0);
        self
    }

    /// Set an explicit title bar region.
    ///
    /// This overrides the default title bar behavior.
    pub fn with_title_bar_region(mut self, region: Rect) -> Self {
        self.title_bar_region = Some(region);
        self
    }

    /// Add a custom draggable region.
    pub fn with_draggable_region(mut self, region: Rect) -> Self {
        self.draggable_regions.push(region);
        self
    }

    /// Add an interactive (non-draggable) region.
    ///
    /// Interactive regions take precedence over draggable regions,
    /// allowing widgets within the title bar to receive events.
    pub fn with_interactive_region(mut self, region: Rect) -> Self {
        self.interactive_regions.push(region);
        self
    }

    /// Set the system menu button region.
    pub fn with_sys_menu_region(mut self, region: Rect) -> Self {
        self.sys_menu_region = Some(region);
        self
    }

    /// Set the minimize button region.
    pub fn with_minimize_button_region(mut self, region: Rect) -> Self {
        self.minimize_button_region = Some(region);
        self
    }

    /// Set the maximize button region.
    pub fn with_maximize_button_region(mut self, region: Rect) -> Self {
        self.maximize_button_region = Some(region);
        self
    }

    /// Set the close button region.
    pub fn with_close_button_region(mut self, region: Rect) -> Self {
        self.close_button_region = Some(region);
        self
    }

    /// Enable or disable resize borders.
    pub fn with_resize_enabled(mut self, enabled: bool) -> Self {
        self.resize_enabled = enabled;
        self
    }

    // =========================================================================
    // Setters (for runtime modification)
    // =========================================================================

    /// Set the resize border thickness.
    pub fn set_resize_border(&mut self, border: f32) {
        self.resize_border = border.max(0.0);
    }

    /// Set the corner size.
    pub fn set_corner_size(&mut self, size: f32) {
        self.corner_size = size.max(0.0);
    }

    /// Set the title bar height.
    pub fn set_title_bar_height(&mut self, height: f32) {
        self.title_bar_height = height.max(0.0);
    }

    /// Clear and set the title bar region.
    pub fn set_title_bar_region(&mut self, region: Option<Rect>) {
        self.title_bar_region = region;
    }

    /// Add a draggable region.
    pub fn add_draggable_region(&mut self, region: Rect) {
        self.draggable_regions.push(region);
    }

    /// Add an interactive region.
    pub fn add_interactive_region(&mut self, region: Rect) {
        self.interactive_regions.push(region);
    }

    /// Clear all draggable regions.
    pub fn clear_draggable_regions(&mut self) {
        self.draggable_regions.clear();
    }

    /// Clear all interactive regions.
    pub fn clear_interactive_regions(&mut self) {
        self.interactive_regions.clear();
    }

    /// Enable or disable resize.
    pub fn set_resize_enabled(&mut self, enabled: bool) {
        self.resize_enabled = enabled;
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Get the resize border thickness.
    pub fn resize_border(&self) -> f32 {
        self.resize_border
    }

    /// Get the corner size.
    pub fn corner_size(&self) -> f32 {
        self.corner_size
    }

    /// Get the title bar height.
    pub fn title_bar_height(&self) -> f32 {
        self.title_bar_height
    }

    /// Get the explicit title bar region, if any.
    pub fn title_bar_region(&self) -> Option<&Rect> {
        self.title_bar_region.as_ref()
    }

    /// Get the draggable regions.
    pub fn draggable_regions(&self) -> &[Rect] {
        &self.draggable_regions
    }

    /// Get the interactive regions.
    pub fn interactive_regions(&self) -> &[Rect] {
        &self.interactive_regions
    }

    /// Check if resize is enabled.
    pub fn is_resize_enabled(&self) -> bool {
        self.resize_enabled
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    /// Perform hit testing for a point in window coordinates.
    ///
    /// Given a point and the window size, determines what chrome
    /// element (if any) the point is over.
    ///
    /// # Arguments
    ///
    /// * `point` - The point in window-local coordinates (origin at top-left)
    /// * `window_size` - The current size of the window
    ///
    /// # Returns
    ///
    /// A `ChromeHitTestResult` indicating what the point is over.
    ///
    /// # Hit Test Order
    ///
    /// 1. Window buttons (close, maximize, minimize, sys menu)
    /// 2. Interactive regions (excludes from dragging)
    /// 3. Resize borders (edges and corners)
    /// 4. Title bar / draggable regions
    /// 5. Client area (default)
    pub fn hit_test(&self, point: Point, window_size: Size) -> ChromeHitTestResult {
        let x = point.x;
        let y = point.y;
        let width = window_size.width;
        let height = window_size.height;

        // 1. Check window buttons first (they take priority)
        if let Some(result) = self.hit_test_buttons(point) {
            return result;
        }

        // 2. Check interactive regions (they exclude from dragging)
        if self.point_in_interactive_region(point) {
            return ChromeHitTestResult::Client;
        }

        // 3. Check resize borders
        if self.resize_enabled
            && self.resize_border > 0.0
            && let Some(direction) = self.hit_test_resize(x, y, width, height)
        {
            return ChromeHitTestResult::ResizeBorder(direction);
        }

        // 4. Check title bar and draggable regions
        if self.point_in_title_bar(point, window_size) || self.point_in_draggable_region(point) {
            return ChromeHitTestResult::Caption;
        }

        // 5. Client area (default)
        ChromeHitTestResult::Client
    }

    /// Hit test for resize borders.
    ///
    /// Returns the resize direction if the point is in a resize border,
    /// or `None` if it's in the client area.
    fn hit_test_resize(&self, x: f32, y: f32, width: f32, height: f32) -> Option<ResizeDirection> {
        let border = self.resize_border;
        let corner = self.corner_size;

        // Determine position relative to edges
        let on_left = x < border;
        let on_right = x >= width - border;
        let on_top = y < border;
        let on_bottom = y >= height - border;

        // Check corners first (they override edges)
        let in_left_corner = x < corner;
        let in_right_corner = x >= width - corner;
        let in_top_corner = y < corner;
        let in_bottom_corner = y >= height - corner;

        // Corners
        if on_top && in_left_corner || on_left && in_top_corner {
            return Some(ResizeDirection::NorthWest);
        }
        if on_top && in_right_corner || on_right && in_top_corner {
            return Some(ResizeDirection::NorthEast);
        }
        if on_bottom && in_left_corner || on_left && in_bottom_corner {
            return Some(ResizeDirection::SouthWest);
        }
        if on_bottom && in_right_corner || on_right && in_bottom_corner {
            return Some(ResizeDirection::SouthEast);
        }

        // Edges
        if on_top {
            return Some(ResizeDirection::North);
        }
        if on_bottom {
            return Some(ResizeDirection::South);
        }
        if on_left {
            return Some(ResizeDirection::West);
        }
        if on_right {
            return Some(ResizeDirection::East);
        }

        None
    }

    /// Hit test for window buttons.
    fn hit_test_buttons(&self, point: Point) -> Option<ChromeHitTestResult> {
        // Check buttons in order: close, maximize, minimize, sys menu
        if let Some(ref region) = self.close_button_region
            && region.contains(point)
        {
            return Some(ChromeHitTestResult::CloseButton);
        }

        if let Some(ref region) = self.maximize_button_region
            && region.contains(point)
        {
            return Some(ChromeHitTestResult::MaximizeButton);
        }

        if let Some(ref region) = self.minimize_button_region
            && region.contains(point)
        {
            return Some(ChromeHitTestResult::MinimizeButton);
        }

        if let Some(ref region) = self.sys_menu_region
            && region.contains(point)
        {
            return Some(ChromeHitTestResult::SysMenu);
        }

        None
    }

    /// Check if a point is in the default title bar area.
    fn point_in_title_bar(&self, point: Point, window_size: Size) -> bool {
        // If explicit title bar region is set, use that
        if let Some(ref region) = self.title_bar_region {
            return region.contains(point);
        }

        // Otherwise use default: top area minus resize border
        if self.title_bar_height <= 0.0 {
            return false;
        }

        let border = if self.resize_enabled {
            self.resize_border
        } else {
            0.0
        };

        point.x >= border
            && point.x < window_size.width - border
            && point.y >= border
            && point.y < border + self.title_bar_height
    }

    /// Check if a point is in any custom draggable region.
    fn point_in_draggable_region(&self, point: Point) -> bool {
        self.draggable_regions.iter().any(|r| r.contains(point))
    }

    /// Check if a point is in any interactive region.
    fn point_in_interactive_region(&self, point: Point) -> bool {
        self.interactive_regions.iter().any(|r| r.contains(point))
    }

    /// Get the appropriate cursor shape for a hit test result.
    ///
    /// Returns the cursor that should be displayed when hovering over
    /// the given hit test result.
    pub fn cursor_for_result(result: ChromeHitTestResult) -> super::super::widget::CursorShape {
        use super::super::widget::CursorShape;

        match result {
            ChromeHitTestResult::Client => CursorShape::Arrow,
            ChromeHitTestResult::Caption => CursorShape::Arrow,
            ChromeHitTestResult::SysMenu => CursorShape::Arrow,
            ChromeHitTestResult::MinimizeButton => CursorShape::Arrow,
            ChromeHitTestResult::MaximizeButton => CursorShape::Arrow,
            ChromeHitTestResult::CloseButton => CursorShape::Arrow,
            ChromeHitTestResult::ResizeBorder(dir) => match dir {
                ResizeDirection::North | ResizeDirection::South => CursorShape::ResizeVertical,
                ResizeDirection::East | ResizeDirection::West => CursorShape::ResizeHorizontal,
                ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorShape::ResizeNwSe,
                ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorShape::ResizeNeSw,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_chrome() {
        let chrome = FramelessWindowChrome::new();
        assert_eq!(chrome.resize_border(), 8.0);
        assert_eq!(chrome.corner_size(), 16.0);
        assert_eq!(chrome.title_bar_height(), 32.0);
        assert!(chrome.is_resize_enabled());
    }

    #[test]
    fn test_minimal_chrome() {
        let chrome = FramelessWindowChrome::minimal();
        assert_eq!(chrome.title_bar_height(), 0.0);
        assert!(chrome.is_resize_enabled());
    }

    #[test]
    fn test_no_chrome() {
        let chrome = FramelessWindowChrome::none();
        assert_eq!(chrome.resize_border(), 0.0);
        assert_eq!(chrome.title_bar_height(), 0.0);
        assert!(!chrome.is_resize_enabled());
    }

    #[test]
    fn test_hit_test_client_area() {
        let chrome = FramelessWindowChrome::new();
        let size = Size::new(800.0, 600.0);

        // Point in the middle of the window (client area)
        let result = chrome.hit_test(Point::new(400.0, 300.0), size);
        assert_eq!(result, ChromeHitTestResult::Client);
    }

    #[test]
    fn test_hit_test_title_bar() {
        let chrome = FramelessWindowChrome::new();
        let size = Size::new(800.0, 600.0);

        // Point in the title bar area (below resize border, within title bar height)
        let result = chrome.hit_test(Point::new(400.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::Caption);
    }

    #[test]
    fn test_hit_test_resize_borders() {
        let chrome = FramelessWindowChrome::new();
        let size = Size::new(800.0, 600.0);

        // Top edge (within resize border)
        let result = chrome.hit_test(Point::new(400.0, 3.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::North)
        );

        // Bottom edge
        let result = chrome.hit_test(Point::new(400.0, 597.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::South)
        );

        // Left edge
        let result = chrome.hit_test(Point::new(3.0, 300.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::West)
        );

        // Right edge
        let result = chrome.hit_test(Point::new(797.0, 300.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::East)
        );
    }

    #[test]
    fn test_hit_test_corners() {
        let chrome = FramelessWindowChrome::new();
        let size = Size::new(800.0, 600.0);

        // Top-left corner
        let result = chrome.hit_test(Point::new(3.0, 3.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::NorthWest)
        );

        // Top-right corner
        let result = chrome.hit_test(Point::new(797.0, 3.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::NorthEast)
        );

        // Bottom-left corner
        let result = chrome.hit_test(Point::new(3.0, 597.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::SouthWest)
        );

        // Bottom-right corner
        let result = chrome.hit_test(Point::new(797.0, 597.0), size);
        assert_eq!(
            result,
            ChromeHitTestResult::ResizeBorder(ResizeDirection::SouthEast)
        );
    }

    #[test]
    fn test_interactive_region_excludes_dragging() {
        let chrome =
            FramelessWindowChrome::new().with_interactive_region(Rect::new(700.0, 8.0, 92.0, 32.0)); // Buttons area

        let size = Size::new(800.0, 600.0);

        // Point in the interactive region (should be client, not caption)
        let result = chrome.hit_test(Point::new(750.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::Client);

        // Point in title bar but outside interactive region
        let result = chrome.hit_test(Point::new(400.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::Caption);
    }

    #[test]
    fn test_custom_draggable_region() {
        let chrome = FramelessWindowChrome::new()
            .with_title_bar_height(0.0) // Disable default title bar
            .with_draggable_region(Rect::new(100.0, 100.0, 200.0, 50.0));

        let size = Size::new(800.0, 600.0);

        // Point in custom draggable region
        let result = chrome.hit_test(Point::new(200.0, 120.0), size);
        assert_eq!(result, ChromeHitTestResult::Caption);

        // Point outside draggable region
        let result = chrome.hit_test(Point::new(50.0, 120.0), size);
        assert_eq!(result, ChromeHitTestResult::Client);
    }

    #[test]
    fn test_window_buttons() {
        let chrome = FramelessWindowChrome::new()
            .with_close_button_region(Rect::new(768.0, 8.0, 24.0, 24.0))
            .with_maximize_button_region(Rect::new(740.0, 8.0, 24.0, 24.0))
            .with_minimize_button_region(Rect::new(712.0, 8.0, 24.0, 24.0));

        let size = Size::new(800.0, 600.0);

        // Close button
        let result = chrome.hit_test(Point::new(780.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::CloseButton);

        // Maximize button
        let result = chrome.hit_test(Point::new(752.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::MaximizeButton);

        // Minimize button
        let result = chrome.hit_test(Point::new(724.0, 20.0), size);
        assert_eq!(result, ChromeHitTestResult::MinimizeButton);
    }

    #[test]
    fn test_resize_disabled() {
        let chrome = FramelessWindowChrome::new().with_resize_enabled(false);

        let size = Size::new(800.0, 600.0);

        // Point that would be in resize border, but resize is disabled
        let result = chrome.hit_test(Point::new(3.0, 300.0), size);
        // Should be either client or caption depending on whether it's in title bar
        assert!(!result.is_resize());
    }

    #[test]
    fn test_hit_test_result_methods() {
        assert!(ChromeHitTestResult::ResizeBorder(ResizeDirection::North).is_resize());
        assert!(!ChromeHitTestResult::Caption.is_resize());

        assert!(ChromeHitTestResult::Caption.is_draggable());
        assert!(ChromeHitTestResult::SysMenu.is_draggable());
        assert!(!ChromeHitTestResult::Client.is_draggable());

        assert!(ChromeHitTestResult::CloseButton.is_button());
        assert!(ChromeHitTestResult::MaximizeButton.is_button());
        assert!(ChromeHitTestResult::MinimizeButton.is_button());
        assert!(!ChromeHitTestResult::Caption.is_button());
    }

    #[test]
    fn test_builder_chain() {
        let chrome = FramelessWindowChrome::new()
            .with_resize_border(10.0)
            .with_corner_size(20.0)
            .with_title_bar_height(40.0)
            .with_resize_enabled(true)
            .with_draggable_region(Rect::new(0.0, 0.0, 100.0, 100.0))
            .with_interactive_region(Rect::new(50.0, 0.0, 50.0, 50.0));

        assert_eq!(chrome.resize_border(), 10.0);
        assert_eq!(chrome.corner_size(), 20.0);
        assert_eq!(chrome.title_bar_height(), 40.0);
        assert!(chrome.is_resize_enabled());
        assert_eq!(chrome.draggable_regions().len(), 1);
        assert_eq!(chrome.interactive_regions().len(), 1);
    }

    #[test]
    fn test_runtime_modification() {
        let mut chrome = FramelessWindowChrome::new();

        chrome.set_resize_border(12.0);
        chrome.set_corner_size(24.0);
        chrome.set_title_bar_height(48.0);

        assert_eq!(chrome.resize_border(), 12.0);
        assert_eq!(chrome.corner_size(), 24.0);
        assert_eq!(chrome.title_bar_height(), 48.0);

        chrome.add_draggable_region(Rect::new(0.0, 0.0, 100.0, 100.0));
        chrome.add_interactive_region(Rect::new(0.0, 0.0, 50.0, 50.0));

        assert_eq!(chrome.draggable_regions().len(), 1);
        assert_eq!(chrome.interactive_regions().len(), 1);

        chrome.clear_draggable_regions();
        chrome.clear_interactive_regions();

        assert!(chrome.draggable_regions().is_empty());
        assert!(chrome.interactive_regions().is_empty());
    }
}
