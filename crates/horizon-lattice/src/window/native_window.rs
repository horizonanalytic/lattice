//! Native window wrapper.
//!
//! This module provides `NativeWindow`, a wrapper around the platform's
//! native window (winit::Window) with additional Horizon Lattice functionality.
//!
//! # HiDPI Support and Coordinate Systems
//!
//! Horizon Lattice fully supports HiDPI (High Dots Per Inch) displays, including
//! fractional scaling. Understanding the difference between logical and physical
//! pixels is essential for correct rendering across different display configurations.
//!
//! ## Logical Pixels vs Physical Pixels
//!
//! - **Logical pixels**: Device-independent units used for UI layout and API
//!   consistency. A button that's 100 logical pixels wide will appear roughly
//!   the same physical size on any display, regardless of DPI.
//!
//! - **Physical pixels**: Actual hardware pixels on the display. These are what
//!   the GPU renders to. On a 2x HiDPI display, 100 logical pixels = 200 physical
//!   pixels.
//!
//! ## Scale Factor
//!
//! The scale factor is the ratio of physical pixels to logical pixels:
//!
//! ```text
//! physical_pixels = logical_pixels * scale_factor
//! ```
//!
//! Common scale factors:
//! - **1.0**: Standard DPI display (96 DPI on Windows, 72 DPI on macOS)
//! - **1.25, 1.5**: Common fractional scaling values
//! - **2.0**: Retina/HiDPI display (e.g., 4K on 27" or MacBook Pro)
//! - **3.0**: Very high DPI displays (e.g., iPhone Plus models)
//!
//! ## API Usage
//!
//! ### Window Size
//!
//! - [`NativeWindow::inner_size()`]: Returns physical pixels (for rendering)
//! - [`NativeWindow::inner_size_logical()`]: Returns logical pixels (for layout)
//! - [`NativeWindow::request_inner_size()`]: Takes physical pixels
//! - [`NativeWindow::request_inner_size_logical()`]: Takes logical pixels
//!
//! ### Position
//!
//! - [`NativeWindow::outer_position()`]: Returns physical pixels
//! - [`NativeWindow::set_outer_position()`]: Takes physical pixels
//! - [`NativeWindow::set_outer_position_logical()`]: Takes logical pixels
//!
//! ### Rendering
//!
//! When rendering, always use physical pixels for GPU operations:
//!
//! ```ignore
//! // Get physical size for render target/viewport
//! let physical_size = window.inner_size();
//! surface.configure(physical_size.width, physical_size.height);
//!
//! // Scale factor for coordinate conversion
//! let scale = window.scale_factor();
//!
//! // Convert logical UI coordinates to physical for rendering
//! let physical_x = logical_x * scale;
//! let physical_y = logical_y * scale;
//! ```
//!
//! ## Handling Scale Factor Changes
//!
//! When a window moves between monitors with different DPIs, or when the
//! user changes their display scaling settings, you'll receive a
//! `ScaleFactorChanged` event through [`WindowManager::window_scale_factor_changed()`].
//!
//! ```ignore
//! let manager = WindowManager::instance();
//! manager.window_scale_factor_changed().connect(|(window_id, new_scale)| {
//!     // 1. Update render surface resolution
//!     // 2. Reload images at appropriate resolution (@2x, @3x)
//!     // 3. Recalculate layout if needed
//!     // 4. Request redraw
//! });
//! ```
//!
//! ## Best Practices
//!
//! 1. **Use logical pixels for layout**: Design your UI in logical pixels.
//!    A 100px button should be 100 logical pixels on all displays.
//!
//! 2. **Use physical pixels for rendering**: Configure GPU surfaces and
//!    render targets using physical dimensions.
//!
//! 3. **Provide multi-resolution assets**: For bitmap images, provide @2x
//!    and @3x variants. Use [`ScalableImage`](crate::ScalableImage) for
//!    automatic resolution selection.
//!
//! 4. **Prefer vector graphics**: SVG and procedural drawing scale perfectly.
//!    Use [`SvgImage`](crate::SvgImage) for resolution-independent icons.
//!
//! 5. **Handle scale changes gracefully**: Subscribe to scale factor change
//!    signals and update your rendering accordingly.
//!
//! ## Platform Notes
//!
//! - **macOS**: Reports fractional scale factors. Retina displays are typically 2.0.
//! - **Windows**: Scale factor comes from display settings. Can be 1.0, 1.25, 1.5, 1.75, 2.0, etc.
//! - **Linux/Wayland**: Scale factor is typically an integer (1, 2, 3).
//! - **Linux/X11**: Behavior varies by DE; may report 1.0 with large fonts instead.

use std::sync::Arc;

use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId, WindowLevel};

use super::frameless_chrome::{ChromeHitTestResult, FramelessWindowChrome};
use super::window_config::WindowConfig;
use super::window_effects::{self, WindowEffectError, WindowMask};
use super::window_geometry::WindowGeometry;
use super::window_icon::WindowIcon;
use super::window_type::WindowType;
use crate::widget::widgets::WindowState;
use horizon_lattice_render::{Point, Size};

/// Unique identifier for a native window.
///
/// This wraps winit's `WindowId` and provides additional functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeWindowId(WindowId);

impl NativeWindowId {
    /// Create from a winit WindowId.
    pub fn from_winit(id: WindowId) -> Self {
        Self(id)
    }

    /// Get the underlying winit WindowId.
    pub fn winit_id(&self) -> WindowId {
        self.0
    }
}

impl From<WindowId> for NativeWindowId {
    fn from(id: WindowId) -> Self {
        Self(id)
    }
}

impl From<NativeWindowId> for WindowId {
    fn from(id: NativeWindowId) -> Self {
        id.0
    }
}

/// A native platform window.
///
/// `NativeWindow` wraps a winit window and provides additional functionality
/// for the Horizon Lattice framework, including:
///
/// - High-level window management methods
/// - Window type tracking
/// - Integration with the window manager
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{NativeWindow, WindowConfig, WindowType};
///
/// // Create a window
/// let config = WindowConfig::new("My Window")
///     .with_type(WindowType::Normal)
///     .with_size(800, 600);
///
/// let window = NativeWindow::create(event_loop, config)?;
///
/// // Window operations
/// window.set_title("New Title");
/// window.set_visible(true);
/// window.request_redraw();
/// ```
pub struct NativeWindow {
    /// The underlying winit window.
    window: Arc<Window>,
    /// The window type.
    window_type: WindowType,
    /// The original configuration.
    title: String,
    /// Optional frameless window chrome configuration.
    ///
    /// When set, this defines the hit-test regions for a frameless window,
    /// enabling proper drag and resize behavior with custom chrome.
    chrome: Option<FramelessWindowChrome>,
}

impl NativeWindow {
    /// Create a new native window from a configuration.
    ///
    /// This must be called from within the event loop (typically in `resumed()`).
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop
    /// * `config` - Window configuration
    ///
    /// # Errors
    ///
    /// Returns an error if window creation fails.
    pub fn create(
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<Self, NativeWindowError> {
        let attrs = config.to_window_attributes();
        let window = event_loop
            .create_window(attrs)
            .map_err(|e| NativeWindowError::CreationFailed(e.to_string()))?;

        // Automatically set up chrome for frameless windows
        let chrome = if !config.has_decorations() {
            Some(FramelessWindowChrome::new())
        } else {
            None
        };

        Ok(Self {
            window: Arc::new(window),
            window_type: config.window_type(),
            title: config.title().to_string(),
            chrome,
        })
    }

    /// Get the unique window identifier.
    pub fn id(&self) -> NativeWindowId {
        NativeWindowId(self.window.id())
    }

    /// Get the winit window ID.
    pub fn winit_id(&self) -> WindowId {
        self.window.id()
    }

    /// Get the window type.
    pub fn window_type(&self) -> WindowType {
        self.window_type
    }

    /// Get a reference to the underlying winit window.
    ///
    /// This is provided for advanced use cases that need direct access
    /// to the winit window.
    pub fn winit_window(&self) -> &Window {
        &self.window
    }

    /// Get an Arc reference to the underlying winit window.
    ///
    /// This is useful when you need to share the window across threads
    /// or store it for later use (e.g., for surface creation).
    pub fn winit_window_arc(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the window title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        let title = title.into();
        self.window.set_title(&title);
        self.title = title;
    }

    // =========================================================================
    // Visibility
    // =========================================================================

    /// Check if the window is visible.
    pub fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(true)
    }

    /// Set window visibility.
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    /// Show the window.
    pub fn show(&self) {
        self.window.set_visible(true);
    }

    /// Hide the window.
    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    // =========================================================================
    // Size and Position
    // =========================================================================

    /// Get the inner size of the window in physical pixels.
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    /// Get the inner size of the window in logical pixels.
    pub fn inner_size_logical(&self) -> LogicalSize<f64> {
        self.window.inner_size().to_logical(self.scale_factor())
    }

    /// Get the outer size of the window (including decorations) in physical pixels.
    pub fn outer_size(&self) -> PhysicalSize<u32> {
        self.window.outer_size()
    }

    /// Request a new inner size for the window.
    ///
    /// Returns the new size if the request was immediately fulfilled,
    /// or `None` if the request will be processed asynchronously.
    pub fn request_inner_size(&self, size: PhysicalSize<u32>) -> Option<PhysicalSize<u32>> {
        self.window.request_inner_size(size)
    }

    /// Request a new inner size in logical pixels.
    pub fn request_inner_size_logical(&self, width: f64, height: f64) -> Option<PhysicalSize<u32>> {
        let physical: PhysicalSize<u32> =
            LogicalSize::new(width, height).to_physical(self.scale_factor());
        self.window.request_inner_size(physical)
    }

    /// Set the minimum inner size.
    pub fn set_min_inner_size(&self, size: Option<PhysicalSize<u32>>) {
        self.window.set_min_inner_size(size);
    }

    /// Set the maximum inner size.
    pub fn set_max_inner_size(&self, size: Option<PhysicalSize<u32>>) {
        self.window.set_max_inner_size(size);
    }

    /// Get the outer position of the window in physical pixels.
    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NativeWindowError> {
        self.window
            .outer_position()
            .map_err(|_| NativeWindowError::PositionUnavailable)
    }

    /// Set the outer position of the window.
    pub fn set_outer_position(&self, position: PhysicalPosition<i32>) {
        self.window.set_outer_position(position);
    }

    /// Set the outer position in logical pixels.
    pub fn set_outer_position_logical(&self, x: f64, y: f64) {
        let physical: PhysicalPosition<i32> =
            LogicalPosition::new(x, y).to_physical(self.scale_factor());
        self.window.set_outer_position(physical);
    }

    // =========================================================================
    // Window State
    // =========================================================================

    /// Check if the window is minimized.
    pub fn is_minimized(&self) -> Option<bool> {
        self.window.is_minimized()
    }

    /// Set the window minimized state.
    pub fn set_minimized(&self, minimized: bool) {
        self.window.set_minimized(minimized);
    }

    /// Minimize the window.
    pub fn minimize(&self) {
        self.window.set_minimized(true);
    }

    /// Check if the window is maximized.
    pub fn is_maximized(&self) -> bool {
        self.window.is_maximized()
    }

    /// Set the window maximized state.
    pub fn set_maximized(&self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    /// Maximize the window.
    pub fn maximize(&self) {
        self.window.set_maximized(true);
    }

    /// Restore the window from minimized/maximized state.
    pub fn restore(&self) {
        self.window.set_minimized(false);
        self.window.set_maximized(false);
    }

    /// Get the current fullscreen state.
    pub fn fullscreen(&self) -> Option<Fullscreen> {
        self.window.fullscreen()
    }

    /// Set fullscreen mode.
    ///
    /// Pass `Some(Fullscreen::Borderless(None))` for borderless fullscreen
    /// on the current monitor, or `None` to exit fullscreen.
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
        self.window.set_fullscreen(fullscreen);
    }

    /// Enter borderless fullscreen mode.
    pub fn enter_fullscreen(&self) {
        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    /// Exit fullscreen mode.
    pub fn exit_fullscreen(&self) {
        self.window.set_fullscreen(None);
    }

    /// Check if the window is in fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        self.window.fullscreen().is_some()
    }

    // =========================================================================
    // Window Attributes
    // =========================================================================

    /// Check if the window is resizable.
    pub fn is_resizable(&self) -> bool {
        self.window.is_resizable()
    }

    /// Set whether the window is resizable.
    pub fn set_resizable(&self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    /// Check if the window has decorations.
    pub fn is_decorated(&self) -> bool {
        self.window.is_decorated()
    }

    /// Set whether the window has decorations.
    pub fn set_decorations(&self, decorations: bool) {
        self.window.set_decorations(decorations);
    }

    /// Set the window level (z-ordering).
    pub fn set_window_level(&self, level: WindowLevel) {
        self.window.set_window_level(level);
    }

    /// Set the window icon.
    pub fn set_icon(&self, icon: Option<WindowIcon>) {
        let winit_icon = icon.and_then(|i| i.to_winit_icon().ok());
        self.window.set_window_icon(winit_icon);
    }

    // =========================================================================
    // Scale Factor
    // =========================================================================

    /// Get the window's scale factor.
    ///
    /// This is typically 1.0 for standard displays and 2.0 for HiDPI displays.
    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    // =========================================================================
    // Focus
    // =========================================================================

    /// Check if the window has focus.
    pub fn has_focus(&self) -> bool {
        self.window.has_focus()
    }

    /// Request focus for the window.
    pub fn focus(&self) {
        self.window.focus_window();
    }

    // =========================================================================
    // Rendering
    // =========================================================================

    /// Request a redraw of the window content.
    ///
    /// This schedules a `RedrawRequested` event for the window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Mark the window content as needing to be redrawn before the next
    /// display refresh.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    // =========================================================================
    // Drag Operations
    // =========================================================================

    /// Start a window drag operation.
    ///
    /// This allows the user to move the window by dragging from anywhere,
    /// not just the title bar. Typically called in response to a mouse press.
    pub fn drag_window(&self) -> Result<(), NativeWindowError> {
        self.window
            .drag_window()
            .map_err(|_| NativeWindowError::DragFailed)
    }

    /// Start a window resize operation.
    ///
    /// This allows the user to resize the window by dragging from the
    /// current cursor position.
    pub fn drag_resize_window(
        &self,
        direction: winit::window::ResizeDirection,
    ) -> Result<(), NativeWindowError> {
        self.window
            .drag_resize_window(direction)
            .map_err(|_| NativeWindowError::ResizeFailed)
    }

    // =========================================================================
    // Monitor
    // =========================================================================

    /// Get the current monitor that the window is on.
    pub fn current_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.current_monitor()
    }

    /// Get all available monitors.
    pub fn available_monitors(&self) -> impl Iterator<Item = winit::monitor::MonitorHandle> {
        self.window.available_monitors()
    }

    /// Get the primary monitor.
    pub fn primary_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.primary_monitor()
    }

    // =========================================================================
    // Window Effects (Opacity & Mask)
    // =========================================================================

    /// Get the current window opacity.
    ///
    /// Returns a value from 0.0 (fully transparent) to 1.0 (fully opaque).
    ///
    /// # Platform Notes
    ///
    /// - **macOS**: Returns the NSWindow's alpha value
    /// - **Windows**: Returns the layered window alpha if set
    /// - **Linux**: Always returns 1.0 (opacity query not reliably supported)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let opacity = window.opacity();
    /// println!("Window opacity: {}%", opacity * 100.0);
    /// ```
    pub fn opacity(&self) -> f32 {
        window_effects::get_window_opacity(&self.window)
    }

    /// Set the window opacity.
    ///
    /// This sets the alpha/transparency of the entire window, including
    /// window decorations (title bar, borders).
    ///
    /// # Arguments
    ///
    /// * `opacity` - Value from 0.0 (fully transparent) to 1.0 (fully opaque).
    ///               Values outside this range are clamped.
    ///
    /// # Platform Notes
    ///
    /// - **macOS**: Uses `NSWindow.setAlphaValue:`
    /// - **Windows**: Uses `SetLayeredWindowAttributes` with `LWA_ALPHA`.
    ///                The window is automatically made a layered window.
    /// - **Linux (X11)**: Uses `_NET_WM_WINDOW_OPACITY` atom property.
    ///                    Requires a compositor that supports transparency.
    /// - **Linux (Wayland)**: Limited support, depends on compositor.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make window 80% opaque
    /// window.set_opacity(0.8)?;
    ///
    /// // Make window fully transparent (invisible but still captures input)
    /// window.set_opacity(0.0)?;
    ///
    /// // Restore full opacity
    /// window.set_opacity(1.0)?;
    /// ```
    pub fn set_opacity(&self, opacity: f32) -> Result<(), WindowEffectError> {
        window_effects::set_window_opacity(&self.window, opacity)
    }

    /// Set a window mask (shaped window).
    ///
    /// Window masks allow creating non-rectangular windows by defining
    /// which parts of the window should be visible.
    ///
    /// # Arguments
    ///
    /// * `mask` - The mask to apply, or `None` to remove the mask and
    ///            restore the rectangular window shape.
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Uses `SetWindowRgn` with GDI regions.
    /// - **macOS**: Uses window transparency with content clipping.
    ///              For best results, use with a frameless window.
    /// - **Linux (X11)**: Uses the XShape extension.
    /// - **Linux (Wayland)**: Not supported (returns an error).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::window::WindowMask;
    ///
    /// // Create a circular window
    /// let mask = WindowMask::circle(100, 100, 100);
    /// window.set_mask(Some(&mask))?;
    ///
    /// // Create a rounded rectangle window
    /// let mask = WindowMask::rounded_rect(0, 0, 400, 300, 20);
    /// window.set_mask(Some(&mask))?;
    ///
    /// // Remove the mask
    /// window.set_mask(None)?;
    /// ```
    pub fn set_mask(&self, mask: Option<&WindowMask>) -> Result<(), WindowEffectError> {
        window_effects::set_window_mask(&self.window, mask)
    }

    // =========================================================================
    // Frameless Window Chrome
    // =========================================================================

    /// Get the frameless window chrome configuration.
    ///
    /// Returns `None` if the window has native decorations or no chrome
    /// configuration has been set.
    pub fn chrome(&self) -> Option<&FramelessWindowChrome> {
        self.chrome.as_ref()
    }

    /// Get a mutable reference to the frameless window chrome configuration.
    pub fn chrome_mut(&mut self) -> Option<&mut FramelessWindowChrome> {
        self.chrome.as_mut()
    }

    /// Set the frameless window chrome configuration.
    ///
    /// Pass `Some(chrome)` to enable custom chrome hit-testing for this window,
    /// or `None` to disable it.
    ///
    /// # Note
    ///
    /// This only affects hit-testing behavior. It does not change whether
    /// the window has native decorations. Use `set_decorations(false)` first
    /// to create a frameless window.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::window::FramelessWindowChrome;
    ///
    /// // Create frameless window
    /// window.set_decorations(false);
    ///
    /// // Configure custom chrome with 40px title bar
    /// let chrome = FramelessWindowChrome::new()
    ///     .with_title_bar_height(40.0)
    ///     .with_resize_border(8.0);
    /// window.set_chrome(Some(chrome));
    /// ```
    pub fn set_chrome(&mut self, chrome: Option<FramelessWindowChrome>) {
        self.chrome = chrome;
    }

    /// Perform hit testing using the frameless chrome configuration.
    ///
    /// Given a point in window coordinates, returns the chrome hit test result
    /// indicating what action (if any) should be taken.
    ///
    /// Returns `ChromeHitTestResult::Client` if:
    /// - The window has no chrome configuration
    /// - The window has native decorations
    /// - The point is in the client area
    ///
    /// # Arguments
    ///
    /// * `point` - The point in window-local coordinates (origin at top-left)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In your mouse press handler:
    /// let result = window.chrome_hit_test(mouse_position);
    ///
    /// match result {
    ///     ChromeHitTestResult::Caption => {
    ///         window.drag_window()?;
    ///     }
    ///     ChromeHitTestResult::ResizeBorder(direction) => {
    ///         window.drag_resize_window(direction)?;
    ///     }
    ///     ChromeHitTestResult::CloseButton => {
    ///         // Handle close button click
    ///     }
    ///     _ => {
    ///         // Normal mouse handling
    ///     }
    /// }
    /// ```
    pub fn chrome_hit_test(&self, point: Point) -> ChromeHitTestResult {
        match &self.chrome {
            Some(chrome) => {
                let size = self.inner_size();
                let window_size = Size::new(size.width as f32, size.height as f32);
                chrome.hit_test(point, window_size)
            }
            None => ChromeHitTestResult::Client,
        }
    }

    /// Show the system window menu at the specified position.
    ///
    /// This shows the context menu that normally appears when right-clicking
    /// the title bar, containing options like Restore, Move, Size, Minimize,
    /// Maximize, and Close.
    ///
    /// This is useful when implementing custom decorations and you want to
    /// provide access to the system window menu.
    ///
    /// # Arguments
    ///
    /// * `position` - The position in window coordinates where the menu should appear
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Shows the standard system menu
    /// - **macOS**: No-op (macOS doesn't have a traditional window menu)
    /// - **Linux**: Behavior varies by window manager
    pub fn show_window_menu(&self, position: Point) {
        use winit::dpi::PhysicalPosition;
        let physical = PhysicalPosition::new(position.x as f64, position.y as f64);
        self.window.show_window_menu(physical);
    }

    // =========================================================================
    // Geometry Save/Restore
    // =========================================================================

    /// Save the current window geometry for later restoration.
    ///
    /// This captures the window's position, size, and state in a format
    /// suitable for persistence across application sessions.
    ///
    /// # Coordinate System
    ///
    /// The saved geometry uses logical pixels for DPI independence.
    /// When restoring, coordinates are automatically adjusted for the
    /// target screen's scale factor.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Save before closing the application
    /// let geometry = window.save_geometry();
    ///
    /// // Serialize to JSON for persistence
    /// let json = serde_json::to_string(&geometry)?;
    /// std::fs::write("window_state.json", json)?;
    /// ```
    ///
    /// # Notes
    ///
    /// - For maximized or fullscreen windows, the "normal" geometry is saved
    ///   (the size/position the window will have when restored)
    /// - On Wayland, position may not be available and defaults to (0, 0)
    pub fn save_geometry(&self) -> WindowGeometry {
        let scale = self.scale_factor();

        // Get position (may not be available on Wayland)
        let (x, y) = self
            .outer_position()
            .map(|pos| {
                let logical: LogicalPosition<i32> = pos.to_logical(scale);
                (logical.x, logical.y)
            })
            .unwrap_or((0, 0));

        // Get size in logical pixels
        let size = self.inner_size_logical();
        let width = size.width as u32;
        let height = size.height as u32;

        // Determine window state
        let state = if self.is_fullscreen() {
            WindowState::Fullscreen
        } else if self.is_maximized() {
            WindowState::Maximized
        } else if self.is_minimized().unwrap_or(false) {
            WindowState::Minimized
        } else {
            WindowState::Normal
        };

        // Get current monitor name if available
        let screen_name = self
            .current_monitor()
            .map(|m| m.name().unwrap_or_else(|| "Unknown".to_string()));

        let mut geometry = WindowGeometry::new(x, y, width, height).with_state(state);

        if let Some(name) = screen_name {
            geometry = geometry.with_screen_name(name);
        }

        geometry
    }

    /// Restore window geometry from a saved state.
    ///
    /// This restores the window's position, size, and state. If the saved
    /// screen configuration no longer matches the current setup, the window
    /// is automatically adjusted to be visible.
    ///
    /// # Screen Change Handling
    ///
    /// - If the saved monitor no longer exists, centers on the primary monitor
    /// - If the saved position would be off-screen, adjusts to be visible
    /// - Clamps the size to fit within available screen bounds
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Restore at application startup
    /// let json = std::fs::read_to_string("window_state.json")?;
    /// let geometry: WindowGeometry = serde_json::from_str(&json)?;
    /// window.restore_geometry(&geometry);
    /// ```
    pub fn restore_geometry(&self, geometry: &WindowGeometry) {
        // Validate and adjust the geometry for current screen configuration
        let adjusted = geometry.validated();

        // Apply position and size (convert from logical to physical)
        let scale = self.scale_factor();

        let physical_pos: PhysicalPosition<i32> =
            LogicalPosition::new(adjusted.x, adjusted.y).to_physical(scale);
        self.set_outer_position(physical_pos);

        let physical_size: PhysicalSize<u32> =
            LogicalSize::new(adjusted.width as f64, adjusted.height as f64).to_physical(scale);
        self.request_inner_size(physical_size);

        // Apply window state
        match adjusted.state {
            WindowState::Normal => {
                self.restore();
            }
            WindowState::Minimized => {
                self.minimize();
            }
            WindowState::Maximized => {
                self.maximize();
            }
            WindowState::Fullscreen => {
                self.enter_fullscreen();
            }
        }
    }

    /// Get the current window state.
    ///
    /// Returns the current state of the window (normal, minimized, maximized,
    /// or fullscreen).
    pub fn window_state(&self) -> WindowState {
        if self.is_fullscreen() {
            WindowState::Fullscreen
        } else if self.is_maximized() {
            WindowState::Maximized
        } else if self.is_minimized().unwrap_or(false) {
            WindowState::Minimized
        } else {
            WindowState::Normal
        }
    }
}

impl std::fmt::Debug for NativeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeWindow")
            .field("id", &self.id())
            .field("title", &self.title)
            .field("window_type", &self.window_type)
            .field("size", &self.inner_size())
            .finish()
    }
}

/// Error type for native window operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWindowError {
    /// Window creation failed.
    CreationFailed(String),
    /// Window position is unavailable (e.g., on Wayland).
    PositionUnavailable,
    /// Window drag operation failed.
    DragFailed,
    /// Window resize operation failed.
    ResizeFailed,
}

impl std::fmt::Display for NativeWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeWindowError::CreationFailed(msg) => {
                write!(f, "window creation failed: {}", msg)
            }
            NativeWindowError::PositionUnavailable => {
                write!(f, "window position is unavailable")
            }
            NativeWindowError::DragFailed => {
                write!(f, "window drag operation failed")
            }
            NativeWindowError::ResizeFailed => {
                write!(f, "window resize operation failed")
            }
        }
    }
}

impl std::error::Error for NativeWindowError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_window_id_conversion() {
        // We can't create a real WindowId in tests, but we can test the type structure
        // This test just verifies the types compile correctly
    }

    #[test]
    fn test_native_window_error_display() {
        let err = NativeWindowError::CreationFailed("test error".to_string());
        assert!(format!("{}", err).contains("test error"));

        let err = NativeWindowError::PositionUnavailable;
        assert!(format!("{}", err).contains("position"));

        let err = NativeWindowError::DragFailed;
        assert!(format!("{}", err).contains("drag"));

        let err = NativeWindowError::ResizeFailed;
        assert!(format!("{}", err).contains("resize"));
    }
}
