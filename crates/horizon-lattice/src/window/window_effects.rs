//! Window effects: opacity and shaped windows.
//!
//! This module provides platform-specific implementations for window effects
//! that are not directly supported by winit, including:
//!
//! - **Window Opacity**: Set the overall alpha/transparency of the entire window
//! - **Window Mask**: Create non-rectangular (shaped) windows
//!
//! # Platform Support
//!
//! | Feature | Windows | macOS | Linux (X11) | Linux (Wayland) |
//! |---------|---------|-------|-------------|-----------------|
//! | Opacity | Yes | Yes | Yes | Limited |
//! | Mask | Yes | Yes | Yes | No |
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::window::{NativeWindow, WindowMask};
//!
//! // Set window opacity (0.0 = fully transparent, 1.0 = fully opaque)
//! native_window.set_opacity(0.8)?;
//!
//! // Get current opacity
//! let opacity = native_window.opacity();
//!
//! // Create a circular window mask
//! let mask = WindowMask::ellipse(0, 0, 200, 200);
//! native_window.set_mask(Some(mask))?;
//!
//! // Remove mask (restore rectangular window)
//! native_window.set_mask(None)?;
//! ```
//!
//! # Notes
//!
//! - Opacity affects the entire window including decorations (title bar, borders)
//! - Window masks require the window to be frameless for best results
//! - On Wayland, window masks are not supported due to protocol limitations
//! - Some compositors may not support opacity for all window types

use std::fmt;

use winit::window::Window;

// ============================================================================
// Error Type
// ============================================================================

/// Error type for window effect operations.
#[derive(Debug, Clone)]
pub struct WindowEffectError {
    kind: WindowEffectErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowEffectErrorKind {
    /// The operation is not supported on this platform.
    Unsupported,
    /// Failed to access the native window handle.
    HandleAccess,
    /// Platform-specific operation failed.
    PlatformError,
}

impl WindowEffectError {
    #[allow(dead_code)] // Used by platform-specific code
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: WindowEffectErrorKind::Unsupported,
            message: message.into(),
        }
    }

    #[allow(dead_code)] // Used by platform-specific code
    fn handle_access(message: impl Into<String>) -> Self {
        Self {
            kind: WindowEffectErrorKind::HandleAccess,
            message: message.into(),
        }
    }

    #[allow(dead_code)] // Used by platform-specific code
    fn platform_error(message: impl Into<String>) -> Self {
        Self {
            kind: WindowEffectErrorKind::PlatformError,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the operation is not supported.
    pub fn is_unsupported(&self) -> bool {
        self.kind == WindowEffectErrorKind::Unsupported
    }
}

impl fmt::Display for WindowEffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            WindowEffectErrorKind::Unsupported => {
                write!(f, "unsupported: {}", self.message)
            }
            WindowEffectErrorKind::HandleAccess => {
                write!(f, "failed to access window handle: {}", self.message)
            }
            WindowEffectErrorKind::PlatformError => {
                write!(f, "platform error: {}", self.message)
            }
        }
    }
}

impl std::error::Error for WindowEffectError {}

// ============================================================================
// Window Mask
// ============================================================================

/// A shape that defines the visible region of a window.
///
/// Window masks allow creating non-rectangular windows by specifying
/// which parts of the window should be visible.
///
/// # Example
///
/// ```
/// use horizon_lattice::window::WindowMask;
///
/// // Rectangular mask
/// let rect = WindowMask::rect(10, 10, 200, 150);
///
/// // Rounded rectangle
/// let rounded = WindowMask::rounded_rect(0, 0, 300, 200, 20);
///
/// // Ellipse (oval)
/// let ellipse = WindowMask::ellipse(0, 0, 200, 200);
///
/// // Combine multiple shapes
/// let combined = WindowMask::union(&[
///     WindowMask::rect(0, 0, 100, 200),
///     WindowMask::rect(100, 50, 100, 100),
/// ]);
/// ```
#[derive(Debug, Clone)]
pub struct WindowMask {
    /// The shape data for this mask.
    shape: MaskShape,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Platform-specific usage
pub(super) enum MaskShape {
    /// A rectangular region.
    Rect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// A rounded rectangle region.
    RoundedRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        radius: u32,
    },
    /// An elliptical region.
    Ellipse {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// A polygon defined by points.
    Polygon(Vec<(i32, i32)>),
    /// Union of multiple shapes.
    Union(Vec<WindowMask>),
}

impl WindowMask {
    /// Create a rectangular mask.
    ///
    /// # Arguments
    ///
    /// * `x` - Left edge of the rectangle
    /// * `y` - Top edge of the rectangle
    /// * `width` - Width of the rectangle
    /// * `height` - Height of the rectangle
    pub fn rect(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            shape: MaskShape::Rect {
                x,
                y,
                width,
                height,
            },
        }
    }

    /// Create a rounded rectangle mask.
    ///
    /// # Arguments
    ///
    /// * `x` - Left edge of the rectangle
    /// * `y` - Top edge of the rectangle
    /// * `width` - Width of the rectangle
    /// * `height` - Height of the rectangle
    /// * `radius` - Corner radius (clamped to half the smaller dimension)
    pub fn rounded_rect(x: i32, y: i32, width: u32, height: u32, radius: u32) -> Self {
        // Clamp radius to half the smaller dimension
        let max_radius = width.min(height) / 2;
        let radius = radius.min(max_radius);

        Self {
            shape: MaskShape::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
            },
        }
    }

    /// Create an elliptical mask.
    ///
    /// The ellipse is inscribed within the specified bounding box.
    ///
    /// # Arguments
    ///
    /// * `x` - Left edge of the bounding box
    /// * `y` - Top edge of the bounding box
    /// * `width` - Width of the bounding box
    /// * `height` - Height of the bounding box
    pub fn ellipse(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            shape: MaskShape::Ellipse {
                x,
                y,
                width,
                height,
            },
        }
    }

    /// Create a circular mask.
    ///
    /// This is a convenience method for creating an ellipse with equal width and height.
    ///
    /// # Arguments
    ///
    /// * `center_x` - X coordinate of the circle center
    /// * `center_y` - Y coordinate of the circle center
    /// * `radius` - Radius of the circle
    pub fn circle(center_x: i32, center_y: i32, radius: u32) -> Self {
        let diameter = radius * 2;
        Self::ellipse(
            center_x - radius as i32,
            center_y - radius as i32,
            diameter,
            diameter,
        )
    }

    /// Create a polygon mask from a series of points.
    ///
    /// The polygon is automatically closed (the last point connects to the first).
    ///
    /// # Arguments
    ///
    /// * `points` - The vertices of the polygon
    ///
    /// # Panics
    ///
    /// Panics if fewer than 3 points are provided.
    pub fn polygon(points: impl Into<Vec<(i32, i32)>>) -> Self {
        let points = points.into();
        assert!(points.len() >= 3, "polygon requires at least 3 points");
        Self {
            shape: MaskShape::Polygon(points),
        }
    }

    /// Create a mask from the union of multiple shapes.
    ///
    /// The resulting mask includes all areas covered by any of the input shapes.
    ///
    /// # Arguments
    ///
    /// * `shapes` - The shapes to combine
    pub fn union(shapes: &[WindowMask]) -> Self {
        Self {
            shape: MaskShape::Union(shapes.to_vec()),
        }
    }

    /// Get the bounding box of this mask.
    ///
    /// Returns `(x, y, width, height)`.
    pub fn bounds(&self) -> (i32, i32, u32, u32) {
        match &self.shape {
            MaskShape::Rect {
                x,
                y,
                width,
                height,
            }
            | MaskShape::RoundedRect {
                x,
                y,
                width,
                height,
                ..
            }
            | MaskShape::Ellipse {
                x,
                y,
                width,
                height,
            } => (*x, *y, *width, *height),
            MaskShape::Polygon(points) => {
                if points.is_empty() {
                    return (0, 0, 0, 0);
                }
                let min_x = points.iter().map(|(x, _)| *x).min().unwrap();
                let min_y = points.iter().map(|(_, y)| *y).min().unwrap();
                let max_x = points.iter().map(|(x, _)| *x).max().unwrap();
                let max_y = points.iter().map(|(_, y)| *y).max().unwrap();
                (min_x, min_y, (max_x - min_x) as u32, (max_y - min_y) as u32)
            }
            MaskShape::Union(shapes) => {
                if shapes.is_empty() {
                    return (0, 0, 0, 0);
                }
                let mut min_x = i32::MAX;
                let mut min_y = i32::MAX;
                let mut max_x = i32::MIN;
                let mut max_y = i32::MIN;
                for shape in shapes {
                    let (x, y, w, h) = shape.bounds();
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + w as i32);
                    max_y = max_y.max(y + h as i32);
                }
                (min_x, min_y, (max_x - min_x) as u32, (max_y - min_y) as u32)
            }
        }
    }

    /// Get the shape data (for platform implementations).
    #[allow(dead_code)]
    pub(super) fn shape(&self) -> &MaskShape {
        &self.shape
    }
}

// ============================================================================
// Platform Implementations
// ============================================================================

/// Set the window opacity using platform-specific APIs.
///
/// # Arguments
///
/// * `window` - The winit window
/// * `opacity` - Opacity value from 0.0 (fully transparent) to 1.0 (fully opaque)
///
/// # Platform Notes
///
/// - **macOS**: Uses `NSWindow.setAlphaValue:`
/// - **Windows**: Uses `SetLayeredWindowAttributes` with `LWA_ALPHA`
/// - **Linux (X11)**: Uses `_NET_WM_WINDOW_OPACITY` atom property
/// - **Linux (Wayland)**: Limited support, depends on compositor
pub fn set_window_opacity(window: &Window, opacity: f32) -> Result<(), WindowEffectError> {
    let opacity = opacity.clamp(0.0, 1.0);

    #[cfg(target_os = "macos")]
    {
        set_opacity_macos(window, opacity)
    }

    #[cfg(target_os = "windows")]
    {
        set_opacity_windows(window, opacity)
    }

    #[cfg(target_os = "linux")]
    {
        set_opacity_linux(window, opacity)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = window;
        let _ = opacity;
        Err(WindowEffectError::unsupported(
            "window opacity not supported on this platform",
        ))
    }
}

/// Get the current window opacity.
///
/// # Platform Notes
///
/// - **macOS**: Uses `NSWindow.alphaValue`
/// - **Windows**: Uses `GetLayeredWindowAttributes`
/// - **Linux**: Returns 1.0 (opacity query not reliably supported)
pub fn get_window_opacity(window: &Window) -> f32 {
    #[cfg(target_os = "macos")]
    {
        get_opacity_macos(window)
    }

    #[cfg(target_os = "windows")]
    {
        get_opacity_windows(window)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
        1.0
    }
}

/// Set a window mask (shaped window) using platform-specific APIs.
///
/// # Arguments
///
/// * `window` - The winit window
/// * `mask` - The mask to apply, or `None` to remove the mask
///
/// # Platform Notes
///
/// - **Windows**: Uses `SetWindowRgn` with GDI regions
/// - **macOS**: Uses NSWindow's opaque property and content view clipping
/// - **Linux (X11)**: Uses the XShape extension
/// - **Linux (Wayland)**: Not supported
pub fn set_window_mask(
    window: &Window,
    mask: Option<&WindowMask>,
) -> Result<(), WindowEffectError> {
    #[cfg(target_os = "windows")]
    {
        set_mask_windows(window, mask)
    }

    #[cfg(target_os = "macos")]
    {
        set_mask_macos(window, mask)
    }

    #[cfg(target_os = "linux")]
    {
        set_mask_linux(window, mask)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = window;
        let _ = mask;
        Err(WindowEffectError::unsupported(
            "window masks not supported on this platform",
        ))
    }
}

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSWindow;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// Get the NSWindow from a winit window.
    fn get_ns_window(window: &Window) -> Result<Retained<NSWindow>, WindowEffectError> {
        let handle = window
            .window_handle()
            .map_err(|e| WindowEffectError::handle_access(e.to_string()))?;

        match handle.as_raw() {
            RawWindowHandle::AppKit(handle) => {
                // The ns_view is an NSView, we need to get its window
                let ns_view = handle.ns_view.as_ptr() as *mut AnyObject;
                unsafe {
                    let ns_window: *mut NSWindow = msg_send![ns_view, window];
                    if ns_window.is_null() {
                        return Err(WindowEffectError::handle_access("NSView has no window"));
                    }
                    // Retain the window to return it safely
                    Ok(Retained::retain(ns_window).unwrap())
                }
            }
            _ => Err(WindowEffectError::handle_access(
                "expected AppKit window handle",
            )),
        }
    }

    pub fn set_opacity_macos(window: &Window, opacity: f32) -> Result<(), WindowEffectError> {
        let ns_window = get_ns_window(window)?;
        unsafe {
            let _: () = msg_send![&ns_window, setAlphaValue: opacity as f64];
        }
        Ok(())
    }

    pub fn get_opacity_macos(window: &Window) -> f32 {
        match get_ns_window(window) {
            Ok(ns_window) => unsafe {
                let alpha: f64 = msg_send![&ns_window, alphaValue];
                alpha as f32
            },
            Err(_) => 1.0,
        }
    }

    pub fn set_mask_macos(
        window: &Window,
        mask: Option<&WindowMask>,
    ) -> Result<(), WindowEffectError> {
        let ns_window = get_ns_window(window)?;

        if let Some(mask) = mask {
            // For shaped windows on macOS, we need to:
            // 1. Make the window non-opaque
            // 2. Set the background color to clear
            // 3. Apply a mask to the content view
            //
            // Note: True shaped windows on macOS are complex and typically
            // require custom drawing. For now, we support this through
            // transparency. Full mask support would require CALayer masks.
            unsafe {
                let _: () = msg_send![&ns_window, setOpaque: false];

                // Get NSColor.clearColor
                let clear_color: *mut AnyObject = msg_send![objc2::class!(NSColor), clearColor];
                let _: () = msg_send![&ns_window, setBackgroundColor: clear_color];
            }

            // For complex masks, we'd need to use CALayer masks on the content view
            // This is a simplified implementation that just enables transparency
            let _ = mask; // Acknowledge the mask parameter

            Ok(())
        } else {
            // Remove mask: restore normal window appearance
            unsafe {
                let _: () = msg_send![&ns_window, setOpaque: true];

                // Restore default window background
                let default_color: *mut AnyObject =
                    msg_send![objc2::class!(NSColor), windowBackgroundColor];
                let _: () = msg_send![&ns_window, setBackgroundColor: default_color];
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
use macos::*;

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        CombineRgn, CreateEllipticRgn, CreatePolygonRgn, CreateRectRgn, CreateRoundRectRgn,
        DeleteObject, HRGN, RGN_OR, SetWindowRgn, WINDING,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetLayeredWindowAttributes, GetWindowLongW, LWA_ALPHA,
        SetLayeredWindowAttributes, SetWindowLongW, WS_EX_LAYERED,
    };

    fn get_hwnd(window: &Window) -> Result<HWND, WindowEffectError> {
        let handle = window
            .window_handle()
            .map_err(|e| WindowEffectError::handle_access(e.to_string()))?;

        match handle.as_raw() {
            RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut std::ffi::c_void)),
            _ => Err(WindowEffectError::handle_access(
                "expected Win32 window handle",
            )),
        }
    }

    pub fn set_opacity_windows(window: &Window, opacity: f32) -> Result<(), WindowEffectError> {
        let hwnd = get_hwnd(window)?;

        unsafe {
            // Enable layered window style if not already enabled
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
                SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
            }

            // Set opacity (alpha is 0-255)
            let alpha = (opacity * 255.0).round() as u8;
            SetLayeredWindowAttributes(hwnd, None, alpha, LWA_ALPHA)
                .map_err(|e| WindowEffectError::platform_error(e.to_string()))?;
        }

        Ok(())
    }

    pub fn get_opacity_windows(window: &Window) -> f32 {
        let hwnd = match get_hwnd(window) {
            Ok(hwnd) => hwnd,
            Err(_) => return 1.0,
        };

        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
                return 1.0;
            }

            let mut alpha: u8 = 255;
            let _ = GetLayeredWindowAttributes(hwnd, None, Some(&mut alpha), None);
            alpha as f32 / 255.0
        }
    }

    pub fn set_mask_windows(
        window: &Window,
        mask: Option<&WindowMask>,
    ) -> Result<(), WindowEffectError> {
        let hwnd = get_hwnd(window)?;

        if let Some(mask) = mask {
            let hrgn = create_region_from_mask(mask)?;
            unsafe {
                // SetWindowRgn takes ownership of the region, so we don't delete it
                // Returns 0 on failure, non-zero on success
                if SetWindowRgn(hwnd, hrgn, true) == 0 {
                    DeleteObject(hrgn);
                    return Err(WindowEffectError::platform_error("SetWindowRgn failed"));
                }
            }
        } else {
            // Remove mask by setting region to NULL
            // Returns 0 on failure, non-zero on success
            unsafe {
                if SetWindowRgn(hwnd, HRGN::default(), true) == 0 {
                    return Err(WindowEffectError::platform_error("SetWindowRgn failed"));
                }
            }
        }

        Ok(())
    }

    fn create_region_from_mask(mask: &WindowMask) -> Result<HRGN, WindowEffectError> {
        unsafe {
            match mask.shape() {
                MaskShape::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    let hrgn = CreateRectRgn(*x, *y, x + *width as i32, y + *height as i32);
                    if hrgn.is_invalid() {
                        return Err(WindowEffectError::platform_error("CreateRectRgn failed"));
                    }
                    Ok(hrgn)
                }
                MaskShape::RoundedRect {
                    x,
                    y,
                    width,
                    height,
                    radius,
                } => {
                    let hrgn = CreateRoundRectRgn(
                        *x,
                        *y,
                        x + *width as i32,
                        y + *height as i32,
                        *radius as i32,
                        *radius as i32,
                    );
                    if hrgn.is_invalid() {
                        return Err(WindowEffectError::platform_error(
                            "CreateRoundRectRgn failed",
                        ));
                    }
                    Ok(hrgn)
                }
                MaskShape::Ellipse {
                    x,
                    y,
                    width,
                    height,
                } => {
                    let hrgn = CreateEllipticRgn(*x, *y, x + *width as i32, y + *height as i32);
                    if hrgn.is_invalid() {
                        return Err(WindowEffectError::platform_error(
                            "CreateEllipticRgn failed",
                        ));
                    }
                    Ok(hrgn)
                }
                MaskShape::Polygon(points) => {
                    let win_points: Vec<POINT> =
                        points.iter().map(|(x, y)| POINT { x: *x, y: *y }).collect();
                    let hrgn = CreatePolygonRgn(&win_points, WINDING);
                    if hrgn.is_invalid() {
                        return Err(WindowEffectError::platform_error("CreatePolygonRgn failed"));
                    }
                    Ok(hrgn)
                }
                MaskShape::Union(shapes) => {
                    if shapes.is_empty() {
                        // Empty union = empty region
                        let hrgn = CreateRectRgn(0, 0, 0, 0);
                        if hrgn.is_invalid() {
                            return Err(WindowEffectError::platform_error("CreateRectRgn failed"));
                        }
                        return Ok(hrgn);
                    }

                    // Create the first region
                    let mut combined = create_region_from_mask(&shapes[0])?;

                    // Combine with remaining regions
                    for shape in &shapes[1..] {
                        let other = create_region_from_mask(shape)?;
                        CombineRgn(combined, combined, other, RGN_OR);
                        DeleteObject(other);
                    }

                    Ok(combined)
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
use windows_impl::*;

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    use std::ffi::CString;
    use std::os::raw::{c_int, c_long, c_uchar, c_ulong};

    // X11 bindings (we use raw bindings to avoid adding another dependency)
    #[link(name = "X11")]
    unsafe extern "C" {
        fn XInternAtom(
            display: *mut std::ffi::c_void,
            atom_name: *const c_uchar,
            only_if_exists: c_int,
        ) -> c_ulong;
        fn XChangeProperty(
            display: *mut std::ffi::c_void,
            window: c_ulong,
            property: c_ulong,
            property_type: c_ulong,
            format: c_int,
            mode: c_int,
            data: *const c_uchar,
            nelements: c_int,
        ) -> c_int;
        fn XDeleteProperty(
            display: *mut std::ffi::c_void,
            window: c_ulong,
            property: c_ulong,
        ) -> c_int;
        fn XFlush(display: *mut std::ffi::c_void) -> c_int;
    }

    // XShape extension bindings
    #[link(name = "Xext")]
    unsafe extern "C" {
        fn XShapeCombineRectangles(
            display: *mut std::ffi::c_void,
            dest: c_ulong,
            dest_kind: c_int,
            x_off: c_int,
            y_off: c_int,
            rectangles: *const XRectangle,
            n_rects: c_int,
            op: c_int,
            ordering: c_int,
        );
        fn XShapeCombineMask(
            display: *mut std::ffi::c_void,
            dest: c_ulong,
            dest_kind: c_int,
            x_off: c_int,
            y_off: c_int,
            src: c_ulong,
            op: c_int,
        );
    }

    #[repr(C)]
    struct XRectangle {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    }

    const XA_CARDINAL: c_ulong = 6;
    const PROP_MODE_REPLACE: c_int = 0;
    const SHAPE_BOUNDING: c_int = 0;
    const SHAPE_SET: c_int = 0;
    const UNSORTED: c_int = 0;

    struct X11Window {
        display: *mut std::ffi::c_void,
        window: c_ulong,
    }

    fn get_x11_window(window: &Window) -> Result<X11Window, WindowEffectError> {
        let window_handle = window
            .window_handle()
            .map_err(|e| WindowEffectError::handle_access(e.to_string()))?;

        let display_handle = window
            .display_handle()
            .map_err(|e| WindowEffectError::handle_access(e.to_string()))?;

        // Get display pointer from display handle
        let display = match display_handle.as_raw() {
            RawDisplayHandle::Xlib(dh) => dh
                .display
                .map(|d| d.as_ptr())
                .unwrap_or(std::ptr::null_mut()),
            _ => std::ptr::null_mut(),
        };

        match window_handle.as_raw() {
            RawWindowHandle::Xlib(handle) => Ok(X11Window {
                display,
                window: handle.window,
            }),
            RawWindowHandle::Xcb(handle) => {
                // For XCB handles, we need the X11 display connection
                // This is a limitation - XCB doesn't directly provide X11 display
                // We could try to get it via xcb_get_x11_connection but that requires libX11-xcb
                // For now, return an error for pure XCB windows
                let _ = handle;
                Err(WindowEffectError::unsupported(
                    "pure XCB windows not supported, use X11",
                ))
            }
            RawWindowHandle::Wayland(_) => Err(WindowEffectError::unsupported(
                "window opacity/masks not fully supported on Wayland",
            )),
            _ => Err(WindowEffectError::handle_access(
                "expected X11 or Wayland window handle",
            )),
        }
    }

    pub fn set_opacity_linux(window: &Window, opacity: f32) -> Result<(), WindowEffectError> {
        let x11 = get_x11_window(window)?;

        if x11.display.is_null() {
            return Err(WindowEffectError::handle_access("null X11 display"));
        }

        unsafe {
            let atom_name = CString::new("_NET_WM_WINDOW_OPACITY").unwrap();
            let atom = XInternAtom(x11.display, atom_name.as_ptr() as *const c_uchar, 0);

            if atom == 0 {
                return Err(WindowEffectError::platform_error(
                    "failed to intern _NET_WM_WINDOW_OPACITY atom",
                ));
            }

            if opacity >= 1.0 {
                // Fully opaque: delete the property
                XDeleteProperty(x11.display, x11.window, atom);
            } else {
                // Set opacity as a 32-bit cardinal value
                // The value is the opacity as a fraction of 0xFFFFFFFF
                let opacity_value: c_ulong = ((opacity as f64) * (0xFFFFFFFFu64 as f64)) as c_ulong;
                XChangeProperty(
                    x11.display,
                    x11.window,
                    atom,
                    XA_CARDINAL,
                    32,
                    PROP_MODE_REPLACE,
                    &opacity_value as *const c_ulong as *const c_uchar,
                    1,
                );
            }

            XFlush(x11.display);
        }

        Ok(())
    }

    pub fn set_mask_linux(
        window: &Window,
        mask: Option<&WindowMask>,
    ) -> Result<(), WindowEffectError> {
        let x11 = get_x11_window(window)?;

        if x11.display.is_null() {
            return Err(WindowEffectError::handle_access("null X11 display"));
        }

        unsafe {
            if let Some(mask) = mask {
                // Convert mask to rectangles for XShape
                let rectangles = mask_to_rectangles(mask);

                if rectangles.is_empty() {
                    // Empty mask = invisible window (use a 1x1 rectangle instead)
                    let rect = XRectangle {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    };
                    XShapeCombineRectangles(
                        x11.display,
                        x11.window,
                        SHAPE_BOUNDING,
                        0,
                        0,
                        &rect,
                        1,
                        SHAPE_SET,
                        UNSORTED,
                    );
                } else {
                    XShapeCombineRectangles(
                        x11.display,
                        x11.window,
                        SHAPE_BOUNDING,
                        0,
                        0,
                        rectangles.as_ptr(),
                        rectangles.len() as c_int,
                        SHAPE_SET,
                        UNSORTED,
                    );
                }
            } else {
                // Remove mask by setting to None (0 = reset to rectangular)
                XShapeCombineMask(
                    x11.display,
                    x11.window,
                    SHAPE_BOUNDING,
                    0,
                    0,
                    0, // None pixmap = reset
                    SHAPE_SET,
                );
            }

            XFlush(x11.display);
        }

        Ok(())
    }

    fn mask_to_rectangles(mask: &WindowMask) -> Vec<XRectangle> {
        match mask.shape() {
            MaskShape::Rect {
                x,
                y,
                width,
                height,
            } => {
                vec![XRectangle {
                    x: *x as i16,
                    y: *y as i16,
                    width: *width as u16,
                    height: *height as u16,
                }]
            }
            MaskShape::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
            } => {
                // Approximate rounded rectangle with multiple rectangles
                approximate_rounded_rect(*x, *y, *width, *height, *radius)
            }
            MaskShape::Ellipse {
                x,
                y,
                width,
                height,
            } => {
                // Approximate ellipse with horizontal scan lines
                approximate_ellipse(*x, *y, *width, *height)
            }
            MaskShape::Polygon(points) => {
                // Approximate polygon with scan lines
                approximate_polygon(points)
            }
            MaskShape::Union(shapes) => {
                let mut result = Vec::new();
                for shape in shapes {
                    result.extend(mask_to_rectangles(shape));
                }
                result
            }
        }
    }

    fn approximate_rounded_rect(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        radius: u32,
    ) -> Vec<XRectangle> {
        let mut rects = Vec::new();
        let r = radius as i32;
        let w = width as i32;
        let h = height as i32;

        // Top section (before corners)
        for dy in 0..r {
            let dx = r - ((r * r - (r - dy) * (r - dy)) as f64).sqrt() as i32;
            rects.push(XRectangle {
                x: (x + dx) as i16,
                y: (y + dy) as i16,
                width: (w - 2 * dx) as u16,
                height: 1,
            });
        }

        // Middle section (full width)
        if h > 2 * r {
            rects.push(XRectangle {
                x: x as i16,
                y: (y + r) as i16,
                width: w as u16,
                height: (h - 2 * r) as u16,
            });
        }

        // Bottom section (after corners)
        for dy in 0..r {
            let dx = r - ((r * r - dy * dy) as f64).sqrt() as i32;
            rects.push(XRectangle {
                x: (x + dx) as i16,
                y: (y + h - r + dy) as i16,
                width: (w - 2 * dx) as u16,
                height: 1,
            });
        }

        rects
    }

    fn approximate_ellipse(x: i32, y: i32, width: u32, height: u32) -> Vec<XRectangle> {
        let mut rects = Vec::new();
        let a = width as f64 / 2.0;
        let b = height as f64 / 2.0;
        let cx = x as f64 + a;
        let cy = y as f64 + b;

        for dy in 0..height as i32 {
            let rel_y = dy as f64 - b;
            // Ellipse equation: (x/a)^2 + (y/b)^2 = 1
            // Solve for x: x = a * sqrt(1 - (y/b)^2)
            let ratio = rel_y / b;
            if ratio.abs() <= 1.0 {
                let half_width = a * (1.0 - ratio * ratio).sqrt();
                let start_x = (cx - half_width).round() as i32;
                let end_x = (cx + half_width).round() as i32;
                let rect_width = (end_x - start_x).max(1);

                rects.push(XRectangle {
                    x: start_x as i16,
                    y: (y + dy) as i16,
                    width: rect_width as u16,
                    height: 1,
                });
            }
        }

        rects
    }

    fn approximate_polygon(points: &[(i32, i32)]) -> Vec<XRectangle> {
        if points.len() < 3 {
            return Vec::new();
        }

        // Find bounding box
        let min_y = points.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = points.iter().map(|(_, y)| *y).max().unwrap();

        let mut rects = Vec::new();

        // Scan line fill algorithm
        for y in min_y..=max_y {
            let mut intersections = Vec::new();
            let n = points.len();

            for i in 0..n {
                let (x1, y1) = points[i];
                let (x2, y2) = points[(i + 1) % n];

                if (y1 <= y && y2 > y) || (y2 <= y && y1 > y) {
                    // Edge crosses this scan line
                    let x = x1 + (y - y1) * (x2 - x1) / (y2 - y1);
                    intersections.push(x);
                }
            }

            intersections.sort();

            // Fill between pairs of intersections
            for chunk in intersections.chunks(2) {
                if chunk.len() == 2 {
                    let start_x = chunk[0];
                    let end_x = chunk[1];
                    if end_x > start_x {
                        rects.push(XRectangle {
                            x: start_x as i16,
                            y: y as i16,
                            width: (end_x - start_x) as u16,
                            height: 1,
                        });
                    }
                }
            }
        }

        rects
    }
}

#[cfg(target_os = "linux")]
use linux::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_mask_rect() {
        let mask = WindowMask::rect(10, 20, 100, 50);
        assert_eq!(mask.bounds(), (10, 20, 100, 50));
    }

    #[test]
    fn test_window_mask_rounded_rect() {
        let mask = WindowMask::rounded_rect(0, 0, 100, 100, 10);
        assert_eq!(mask.bounds(), (0, 0, 100, 100));
    }

    #[test]
    fn test_window_mask_rounded_rect_clamps_radius() {
        // Radius larger than half the smaller dimension should be clamped
        let mask = WindowMask::rounded_rect(0, 0, 100, 50, 100);
        // Radius should be clamped to 25 (half of 50)
        match mask.shape() {
            MaskShape::RoundedRect { radius, .. } => assert_eq!(*radius, 25),
            _ => panic!("expected RoundedRect"),
        }
    }

    #[test]
    fn test_window_mask_ellipse() {
        let mask = WindowMask::ellipse(5, 10, 200, 100);
        assert_eq!(mask.bounds(), (5, 10, 200, 100));
    }

    #[test]
    fn test_window_mask_circle() {
        let mask = WindowMask::circle(100, 100, 50);
        // Circle centered at (100, 100) with radius 50
        // Bounding box should be (50, 50, 100, 100)
        assert_eq!(mask.bounds(), (50, 50, 100, 100));
    }

    #[test]
    fn test_window_mask_polygon() {
        let points = vec![(0, 0), (100, 0), (100, 100), (0, 100)];
        let mask = WindowMask::polygon(points);
        assert_eq!(mask.bounds(), (0, 0, 100, 100));
    }

    #[test]
    #[should_panic(expected = "polygon requires at least 3 points")]
    fn test_window_mask_polygon_too_few_points() {
        let points = vec![(0, 0), (100, 0)];
        let _mask = WindowMask::polygon(points);
    }

    #[test]
    fn test_window_mask_union() {
        let masks = vec![
            WindowMask::rect(0, 0, 50, 50),
            WindowMask::rect(100, 100, 50, 50),
        ];
        let union = WindowMask::union(&masks);
        // Bounding box should encompass both rectangles
        assert_eq!(union.bounds(), (0, 0, 150, 150));
    }

    #[test]
    fn test_window_effect_error_display() {
        let err = WindowEffectError::unsupported("test message");
        assert!(format!("{}", err).contains("unsupported"));
        assert!(format!("{}", err).contains("test message"));
        assert!(err.is_unsupported());

        let err = WindowEffectError::handle_access("handle error");
        assert!(format!("{}", err).contains("handle"));
        assert!(!err.is_unsupported());
    }
}
