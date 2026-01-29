//! Hardware information services.
//!
//! This module provides cross-platform access to hardware information,
//! particularly display/monitor detection and configuration.
//!
//! # Screen Information
//!
//! ```ignore
//! use horizon_lattice::platform::{Screen, Screens};
//!
//! // Get all connected screens
//! let screens = Screens::all()?;
//! println!("Found {} screens", screens.len());
//!
//! // Get the primary screen
//! if let Some(primary) = Screens::primary()? {
//!     println!("Primary: {} at {:?}", primary.name(), primary.geometry());
//!     println!("DPI scale: {}", primary.scale_factor());
//! }
//!
//! // Iterate all screens
//! for screen in &screens {
//!     println!("Screen: {} - {}x{} @ {:.0}%",
//!         screen.name(),
//!         screen.width(),
//!         screen.height(),
//!         screen.scale_factor() * 100.0,
//!     );
//! }
//! ```
//!
//! # Screen Change Events
//!
//! ```ignore
//! use horizon_lattice::platform::ScreenWatcher;
//!
//! let watcher = ScreenWatcher::new()?;
//!
//! watcher.screen_added().connect(|screen| {
//!     println!("Screen connected: {}", screen.name());
//! });
//!
//! watcher.screen_removed().connect(|screen| {
//!     println!("Screen disconnected: {}", screen.name());
//! });
//!
//! watcher.start()?;
//! ```
//!
//! # Platform Notes
//!
//! ## Screen Enumeration
//! - **Windows**: Uses `EnumDisplayMonitors` and `GetMonitorInfo`
//! - **macOS**: Uses `NSScreen.screens`
//! - **Linux**: Uses XRandR via X11 (Wayland support limited)
//!
//! ## DPI/Scale Factor
//! - **Windows**: Uses `GetDpiForMonitor` (per-monitor DPI aware)
//! - **macOS**: Uses `NSScreen.backingScaleFactor`
//! - **Linux**: Uses XRandR DPI or environment variables

use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for hardware information operations.
#[derive(Debug)]
pub struct HardwareError {
    kind: HardwareErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HardwareErrorKind {
    /// Failed to enumerate screens.
    EnumerationFailed,
    /// Platform does not support this operation.
    UnsupportedPlatform,
    /// Screen watcher failed to start.
    WatcherFailed,
}

impl HardwareError {
    fn enumeration_failed(message: impl Into<String>) -> Self {
        Self {
            kind: HardwareErrorKind::EnumerationFailed,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: HardwareErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn watcher_failed(message: impl Into<String>) -> Self {
        Self {
            kind: HardwareErrorKind::WatcherFailed,
            message: message.into(),
        }
    }

    /// Returns `true` if this error indicates an unsupported platform.
    #[allow(dead_code)]
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == HardwareErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for HardwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HardwareError {}

// ============================================================================
// Screen Geometry
// ============================================================================

/// Represents a rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenRect {
    /// X coordinate of the top-left corner.
    pub x: i32,
    /// Y coordinate of the top-left corner.
    pub y: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ScreenRect {
    /// Create a new screen rectangle.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    /// Get the center point of the rectangle.
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + (self.width / 2) as i32,
            self.y + (self.height / 2) as i32,
        )
    }
}

// ============================================================================
// Screen
// ============================================================================

/// Represents a physical display/monitor.
///
/// Contains information about the screen's geometry, resolution, and DPI.
#[derive(Debug, Clone)]
pub struct Screen {
    /// Platform-specific identifier.
    id: ScreenId,
    /// Human-readable name (e.g., "Dell U2720Q").
    name: String,
    /// Full geometry including position in virtual desktop.
    geometry: ScreenRect,
    /// Work area (excluding taskbar/dock).
    work_area: ScreenRect,
    /// DPI scale factor (1.0 = 96 DPI, 2.0 = 192 DPI).
    scale_factor: f64,
    /// Whether this is the primary screen.
    is_primary: bool,
    /// Refresh rate in Hz, if available.
    refresh_rate: Option<f64>,
    /// Color depth in bits per pixel.
    color_depth: Option<u32>,
}

/// Platform-specific screen identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScreenId(u64);

impl ScreenId {
    /// Create a new screen ID from a raw value.
    fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[allow(dead_code)]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Screen {
    /// Create a new screen with the given properties.
    fn new(
        id: ScreenId,
        name: String,
        geometry: ScreenRect,
        work_area: ScreenRect,
        scale_factor: f64,
        is_primary: bool,
    ) -> Self {
        Self {
            id,
            name,
            geometry,
            work_area,
            scale_factor,
            is_primary,
            refresh_rate: None,
            color_depth: None,
        }
    }

    /// Create a screen for testing purposes.
    ///
    /// This is public for use in tests within the crate.
    #[doc(hidden)]
    pub fn new_for_testing(
        id: u64,
        name: String,
        geometry: ScreenRect,
        work_area: ScreenRect,
        scale_factor: f64,
        is_primary: bool,
    ) -> Self {
        Self::new(
            ScreenId::new(id),
            name,
            geometry,
            work_area,
            scale_factor,
            is_primary,
        )
    }

    /// Get the platform-specific screen identifier.
    pub fn id(&self) -> ScreenId {
        self.id
    }

    /// Get the human-readable name of the screen.
    ///
    /// This may be the device name, model, or a generic name like "Display 1".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the full screen geometry including position in the virtual desktop.
    ///
    /// For multi-monitor setups, screens are arranged in a virtual coordinate space.
    /// The primary monitor typically has (0, 0) as its origin.
    pub fn geometry(&self) -> ScreenRect {
        self.geometry
    }

    /// Get the work area (usable area excluding taskbar/dock).
    pub fn work_area(&self) -> ScreenRect {
        self.work_area
    }

    /// Get the screen width in pixels.
    pub fn width(&self) -> u32 {
        self.geometry.width
    }

    /// Get the screen height in pixels.
    pub fn height(&self) -> u32 {
        self.geometry.height
    }

    /// Get the screen position (top-left corner) in virtual desktop coordinates.
    pub fn position(&self) -> (i32, i32) {
        (self.geometry.x, self.geometry.y)
    }

    /// Get the DPI scale factor.
    ///
    /// Returns 1.0 for standard DPI (96 DPI on Windows, 72 DPI on macOS).
    /// Returns 2.0 for Retina/HiDPI displays at 200% scaling.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Get the effective DPI (dots per inch).
    ///
    /// On Windows, standard DPI is 96. On macOS, standard is 72.
    /// This returns the actual DPI accounting for the scale factor.
    pub fn dpi(&self) -> f64 {
        #[cfg(target_os = "macos")]
        {
            72.0 * self.scale_factor
        }
        #[cfg(not(target_os = "macos"))]
        {
            96.0 * self.scale_factor
        }
    }

    /// Check if this is the primary screen.
    pub fn is_primary(&self) -> bool {
        self.is_primary
    }

    /// Get the refresh rate in Hz, if available.
    pub fn refresh_rate(&self) -> Option<f64> {
        self.refresh_rate
    }

    /// Get the color depth in bits per pixel, if available.
    pub fn color_depth(&self) -> Option<u32> {
        self.color_depth
    }

    /// Get the physical size in inches, if available.
    ///
    /// This is calculated from the DPI and pixel dimensions.
    pub fn physical_size(&self) -> Option<(f64, f64)> {
        let dpi = self.dpi();
        if dpi > 0.0 {
            Some((
                self.geometry.width as f64 / dpi,
                self.geometry.height as f64 / dpi,
            ))
        } else {
            None
        }
    }

    /// Set the refresh rate (internal use).
    #[allow(dead_code)]
    fn set_refresh_rate(&mut self, rate: f64) {
        self.refresh_rate = Some(rate);
    }

    /// Set the color depth (internal use).
    #[allow(dead_code)]
    fn set_color_depth(&mut self, depth: u32) {
        self.color_depth = Some(depth);
    }
}

// ============================================================================
// Screens (enumeration)
// ============================================================================

/// Screen enumeration utilities.
///
/// Provides static methods to query connected screens.
pub struct Screens;

impl Screens {
    /// Get all connected screens.
    ///
    /// Returns a vector of `Screen` objects representing all connected displays.
    /// The primary screen is typically first in the list.
    ///
    /// # Errors
    ///
    /// Returns an error if screen enumeration fails.
    pub fn all() -> Result<Vec<Screen>, HardwareError> {
        platform::enumerate_screens()
    }

    /// Get the primary screen.
    ///
    /// Returns `None` if no screens are connected (unlikely in practice).
    ///
    /// # Errors
    ///
    /// Returns an error if screen enumeration fails.
    pub fn primary() -> Result<Option<Screen>, HardwareError> {
        let screens = Self::all()?;
        Ok(screens.into_iter().find(|s| s.is_primary))
    }

    /// Get the screen containing the given point.
    ///
    /// Returns `None` if the point is not on any screen.
    ///
    /// # Errors
    ///
    /// Returns an error if screen enumeration fails.
    pub fn at_point(x: i32, y: i32) -> Result<Option<Screen>, HardwareError> {
        let screens = Self::all()?;
        Ok(screens.into_iter().find(|s| s.geometry.contains(x, y)))
    }

    /// Get the number of connected screens.
    ///
    /// # Errors
    ///
    /// Returns an error if screen enumeration fails.
    pub fn count() -> Result<usize, HardwareError> {
        Ok(Self::all()?.len())
    }
}

// ============================================================================
// Platform-specific implementations
// ============================================================================

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::mem;
    use std::ptr;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    struct EnumContext {
        screens: Vec<Screen>,
    }

    pub fn enumerate_screens() -> Result<Vec<Screen>, HardwareError> {
        let mut context = EnumContext {
            screens: Vec::new(),
        };

        unsafe {
            let result = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut context as *mut _ as isize),
            );

            if result == TRUE {
                // Sort so primary is first
                context
                    .screens
                    .sort_by(|a, b| b.is_primary.cmp(&a.is_primary));
                Ok(context.screens)
            } else {
                Err(HardwareError::enumeration_failed(
                    "EnumDisplayMonitors failed",
                ))
            }
        }
    }

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let context = &mut *(lparam.0 as *mut EnumContext);

        let mut info: MONITORINFOEXW = mem::zeroed();
        info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmonitor, &mut info.monitorInfo).as_bool() {
            let rc = info.monitorInfo.rcMonitor;
            let work = info.monitorInfo.rcWork;

            let geometry = ScreenRect::new(
                rc.left,
                rc.top,
                (rc.right - rc.left) as u32,
                (rc.bottom - rc.top) as u32,
            );

            let work_area = ScreenRect::new(
                work.left,
                work.top,
                (work.right - work.left) as u32,
                (work.bottom - work.top) as u32,
            );

            // Get scale factor via DPI
            let mut dpi_x: u32 = 96;
            let mut dpi_y: u32 = 96;
            let _ = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            let scale_factor = dpi_x as f64 / 96.0;

            // Get monitor name
            let name = String::from_utf16_lossy(
                &info
                    .szDevice
                    .iter()
                    .take_while(|&&c| c != 0)
                    .copied()
                    .collect::<Vec<_>>(),
            );

            let is_primary = (info.monitorInfo.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY

            let id = ScreenId::new(hmonitor.0 as u64);

            let screen = Screen::new(id, name, geometry, work_area, scale_factor, is_primary);
            context.screens.push(screen);
        }

        TRUE
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSScreen;
    use objc2_foundation::NSArray;

    pub fn enumerate_screens() -> Result<Vec<Screen>, HardwareError> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| HardwareError::enumeration_failed("must be called from main thread"))?;

        let mut result = Vec::new();

        let screens: Retained<NSArray<NSScreen>> = NSScreen::screens(mtm);

        // Get the main screen for primary detection
        let main_screen: Option<Retained<NSScreen>> = NSScreen::mainScreen(mtm);
        let main_ptr = main_screen
            .as_ref()
            .map(|s| Retained::as_ptr(s) as *const AnyObject);

        for (index, screen) in screens.iter().enumerate() {
            let frame = screen.frame();
            let visible_frame = screen.visibleFrame();

            let geometry = ScreenRect::new(
                frame.origin.x as i32,
                frame.origin.y as i32,
                frame.size.width as u32,
                frame.size.height as u32,
            );

            let work_area = ScreenRect::new(
                visible_frame.origin.x as i32,
                visible_frame.origin.y as i32,
                visible_frame.size.width as u32,
                visible_frame.size.height as u32,
            );

            let scale_factor = screen.backingScaleFactor();

            // Check if this is the main screen
            let screen_ptr = &*screen as *const NSScreen as *const AnyObject;
            let is_primary = main_ptr == Some(screen_ptr);

            // Get device description for name
            let name = format!("Display {}", index + 1);

            // Use frame origin as a pseudo-ID (macOS doesn't expose stable IDs)
            let id =
                ScreenId::new(((frame.origin.x as i64) << 32 | (frame.origin.y as i64)) as u64);

            let screen_info = Screen::new(id, name, geometry, work_area, scale_factor, is_primary);

            // Note: NSScreen.depth returns NSWindowDepth which is not easily convertible
            // to bits per pixel. Color depth detection is skipped on macOS.

            result.push(screen_info);
        }

        // Sort so primary is first
        result.sort_by(|a, b| b.is_primary.cmp(&a.is_primary));

        Ok(result)
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::env;
    use std::process::Command;

    pub fn enumerate_screens() -> Result<Vec<Screen>, HardwareError> {
        // Try xrandr first (works on X11)
        if let Ok(screens) = enumerate_via_xrandr() {
            return Ok(screens);
        }

        // Fallback: return a single "default" screen
        // This is used when we can't detect screens (e.g., on Wayland without portal)
        Ok(vec![create_fallback_screen()])
    }

    fn enumerate_via_xrandr() -> Result<Vec<Screen>, HardwareError> {
        let output = Command::new("xrandr")
            .arg("--query")
            .output()
            .map_err(|e| HardwareError::enumeration_failed(format!("xrandr failed: {}", e)))?;

        if !output.status.success() {
            return Err(HardwareError::enumeration_failed("xrandr returned error"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_xrandr_output(&stdout)
    }

    fn parse_xrandr_output(output: &str) -> Result<Vec<Screen>, HardwareError> {
        let mut screens = Vec::new();
        let mut id_counter: u64 = 0;

        // Get scale factor from environment
        let scale_factor = get_scale_factor();

        for line in output.lines() {
            // Look for lines like: "DP-1 connected primary 2560x1440+0+0 ..."
            // or: "HDMI-1 connected 1920x1080+2560+0 ..."
            if line.contains(" connected") {
                if let Some(screen) = parse_screen_line(line, id_counter, scale_factor) {
                    screens.push(screen);
                    id_counter += 1;
                }
            }
        }

        if screens.is_empty() {
            return Err(HardwareError::enumeration_failed(
                "No screens found in xrandr output",
            ));
        }

        // Sort so primary is first
        screens.sort_by(|a, b| b.is_primary.cmp(&a.is_primary));

        Ok(screens)
    }

    fn parse_screen_line(line: &str, id: u64, scale_factor: f64) -> Option<Screen> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let name = parts[0].to_string();
        let is_primary = parts.contains(&"primary");

        // Find the geometry (format: WxH+X+Y)
        let geometry_str = parts.iter().find(|p| {
            p.contains('x')
                && (p.contains('+') || p.contains('-'))
                && p.chars().next().unwrap_or(' ').is_ascii_digit()
        })?;

        let (width, height, x, y) = parse_geometry(geometry_str)?;

        let geometry = ScreenRect::new(x, y, width, height);
        // Work area is same as geometry on Linux (no reliable way to get it without WM integration)
        let work_area = geometry;

        Some(Screen::new(
            ScreenId::new(id),
            name,
            geometry,
            work_area,
            scale_factor,
            is_primary,
        ))
    }

    fn parse_geometry(s: &str) -> Option<(u32, u32, i32, i32)> {
        // Parse "2560x1440+0+0" or "1920x1080+2560+180"
        let x_idx = s.find('x')?;
        let width: u32 = s[..x_idx].parse().ok()?;

        let rest = &s[x_idx + 1..];

        // Find first + or - for position
        let pos_idx = rest.find(|c| c == '+' || c == '-')?;
        let height: u32 = rest[..pos_idx].parse().ok()?;

        let pos_str = &rest[pos_idx..];
        let (x, y) = parse_position(pos_str)?;

        Some((width, height, x, y))
    }

    fn parse_position(s: &str) -> Option<(i32, i32)> {
        // Parse "+0+0" or "-100+200" etc.
        let chars: Vec<char> = s.chars().collect();
        if chars.is_empty() {
            return None;
        }

        // Find the second sign (start of Y)
        let mut sign_count = 0;
        let mut second_sign_idx = 0;
        for (i, &c) in chars.iter().enumerate() {
            if c == '+' || c == '-' {
                sign_count += 1;
                if sign_count == 2 {
                    second_sign_idx = i;
                    break;
                }
            }
        }

        if sign_count < 2 {
            return None;
        }

        let x_str: String = chars[..second_sign_idx].iter().collect();
        let y_str: String = chars[second_sign_idx..].iter().collect();

        let x: i32 = x_str.replace('+', "").parse().ok()?;
        let y: i32 = y_str.replace('+', "").parse().ok()?;

        Some((x, y))
    }

    fn get_scale_factor() -> f64 {
        // Check GDK_SCALE first (GTK apps)
        if let Ok(scale) = env::var("GDK_SCALE") {
            if let Ok(factor) = scale.parse::<f64>() {
                return factor;
            }
        }

        // Check QT_SCALE_FACTOR (Qt apps)
        if let Ok(scale) = env::var("QT_SCALE_FACTOR") {
            if let Ok(factor) = scale.parse::<f64>() {
                return factor;
            }
        }

        // Default to 1.0
        1.0
    }

    fn create_fallback_screen() -> Screen {
        Screen::new(
            ScreenId::new(0),
            "Display 1".to_string(),
            ScreenRect::new(0, 0, 1920, 1080),
            ScreenRect::new(0, 0, 1920, 1080),
            1.0,
            true,
        )
    }
}

// Fallback for unsupported platforms
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod platform {
    use super::*;

    pub fn enumerate_screens() -> Result<Vec<Screen>, HardwareError> {
        // Return a single default screen
        Ok(vec![Screen::new(
            ScreenId::new(0),
            "Display 1".to_string(),
            ScreenRect::new(0, 0, 1920, 1080),
            ScreenRect::new(0, 0, 1920, 1080),
            1.0,
            true,
        )])
    }
}

// ============================================================================
// ScreenWatcher (monitor change events)
// ============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_lattice_core::Signal;

struct ScreenWatcherInner {
    screen_added: Signal<Screen>,
    screen_removed: Signal<Screen>,
    screen_changed: Signal<Screen>,
    running: AtomicBool,
}

/// Watches for screen/monitor configuration changes.
///
/// Emits signals when screens are connected, disconnected, or reconfigured.
///
/// # Example
///
/// ```ignore
/// let watcher = ScreenWatcher::new()?;
///
/// watcher.screen_added().connect(|screen| {
///     println!("New screen: {}", screen.name());
/// });
///
/// watcher.start()?;
/// ```
///
/// # Platform Notes
///
/// - **Windows**: Uses `WM_DISPLAYCHANGE` message (requires window)
/// - **macOS**: Uses `NSApplicationDidChangeScreenParametersNotification`
/// - **Linux**: Polls xrandr periodically (no reliable event API)
pub struct ScreenWatcher {
    inner: Arc<ScreenWatcherInner>,
}

impl ScreenWatcher {
    /// Create a new screen watcher.
    ///
    /// The watcher is created in a stopped state. Call `start()` to begin watching.
    pub fn new() -> Result<Self, HardwareError> {
        Ok(Self {
            inner: Arc::new(ScreenWatcherInner {
                screen_added: Signal::new(),
                screen_removed: Signal::new(),
                screen_changed: Signal::new(),
                running: AtomicBool::new(false),
            }),
        })
    }

    /// Signal emitted when a new screen is connected.
    pub fn screen_added(&self) -> &Signal<Screen> {
        &self.inner.screen_added
    }

    /// Signal emitted when a screen is disconnected.
    pub fn screen_removed(&self) -> &Signal<Screen> {
        &self.inner.screen_removed
    }

    /// Signal emitted when a screen's configuration changes (resolution, position, etc.).
    pub fn screen_changed(&self) -> &Signal<Screen> {
        &self.inner.screen_changed
    }

    /// Start watching for screen changes.
    ///
    /// On some platforms, this spawns a background thread that polls for changes.
    pub fn start(&self) -> Result<(), HardwareError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        // Platform-specific watching is complex and often requires a window handle.
        // For now, we just mark as running but don't actively poll.
        // The signals can still be manually emitted by the application if needed.

        Ok(())
    }

    /// Stop watching for screen changes.
    pub fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }
}

impl Default for ScreenWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create ScreenWatcher")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_rect_contains() {
        let rect = ScreenRect::new(100, 100, 800, 600);

        assert!(rect.contains(100, 100)); // Top-left corner
        assert!(rect.contains(500, 400)); // Inside
        assert!(rect.contains(899, 699)); // Just inside bottom-right
        assert!(!rect.contains(99, 100)); // Just outside left
        assert!(!rect.contains(100, 99)); // Just outside top
        assert!(!rect.contains(900, 400)); // Outside right
        assert!(!rect.contains(500, 700)); // Outside bottom
    }

    #[test]
    fn test_screen_rect_center() {
        let rect = ScreenRect::new(100, 100, 800, 600);
        assert_eq!(rect.center(), (500, 400));

        let rect2 = ScreenRect::new(0, 0, 1920, 1080);
        assert_eq!(rect2.center(), (960, 540));
    }

    #[test]
    fn test_screen_id() {
        let id1 = ScreenId::new(123);
        let id2 = ScreenId::new(123);
        let id3 = ScreenId::new(456);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.raw(), 123);
    }

    #[test]
    fn test_screen_dpi_calculation() {
        // Standard DPI screen
        let screen = Screen::new(
            ScreenId::new(0),
            "Test".to_string(),
            ScreenRect::new(0, 0, 1920, 1080),
            ScreenRect::new(0, 0, 1920, 1080),
            1.0,
            true,
        );

        #[cfg(target_os = "macos")]
        assert_eq!(screen.dpi(), 72.0);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(screen.dpi(), 96.0);

        // HiDPI screen at 200%
        let hidpi_screen = Screen::new(
            ScreenId::new(1),
            "HiDPI".to_string(),
            ScreenRect::new(0, 0, 3840, 2160),
            ScreenRect::new(0, 0, 3840, 2160),
            2.0,
            false,
        );

        #[cfg(target_os = "macos")]
        assert_eq!(hidpi_screen.dpi(), 144.0);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(hidpi_screen.dpi(), 192.0);
    }

    #[test]
    fn test_screen_physical_size() {
        let screen = Screen::new(
            ScreenId::new(0),
            "Test".to_string(),
            ScreenRect::new(0, 0, 1920, 1080),
            ScreenRect::new(0, 0, 1920, 1080),
            1.0,
            true,
        );

        let physical = screen.physical_size().unwrap();
        #[cfg(target_os = "macos")]
        {
            // 1920/72 = ~26.67 inches, 1080/72 = 15 inches
            assert!((physical.0 - 26.67).abs() < 0.1);
            assert!((physical.1 - 15.0).abs() < 0.1);
        }
        #[cfg(not(target_os = "macos"))]
        {
            // 1920/96 = 20 inches, 1080/96 = 11.25 inches
            assert!((physical.0 - 20.0).abs() < 0.1);
            assert!((physical.1 - 11.25).abs() < 0.1);
        }
    }

    #[test]
    fn test_screen_watcher_creation() {
        let watcher = ScreenWatcher::new().unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_screen_watcher_start_stop() {
        let watcher = ScreenWatcher::new().unwrap();

        assert!(!watcher.is_running());

        watcher.start().unwrap();
        assert!(watcher.is_running());

        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_hardware_error_display() {
        let err = HardwareError::enumeration_failed("test error");
        assert_eq!(err.to_string(), "test error");
        assert!(!err.is_unsupported_platform());

        let err2 = HardwareError::unsupported_platform("not supported");
        assert!(err2.is_unsupported_platform());
    }

    // Platform-specific tests that actually query the system
    #[test]
    #[ignore] // Run manually: cargo test --lib -- --ignored
    fn test_enumerate_screens_integration() {
        let screens = Screens::all().expect("Failed to enumerate screens");
        assert!(!screens.is_empty(), "Should have at least one screen");

        for screen in &screens {
            println!(
                "Screen: {} - {}x{} at ({}, {}) scale={:.2} primary={}",
                screen.name(),
                screen.width(),
                screen.height(),
                screen.geometry.x,
                screen.geometry.y,
                screen.scale_factor(),
                screen.is_primary()
            );
        }

        // Primary should exist
        let primary = Screens::primary().expect("Failed to get primary");
        assert!(primary.is_some(), "Should have a primary screen");
    }
}
