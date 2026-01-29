//! Linux native dialog implementation using XDG Desktop Portal.
//!
//! This module provides native dialog support on Linux using the ashpd crate
//! for XDG Desktop Portal D-Bus communication.

use std::path::PathBuf;

use horizon_lattice_render::Color;

use super::{
    NativeColorOptions, NativeFileDialogOptions, NativeFileFilter, NativeFontDesc,
    NativeFontOptions, NativeMessageOptions, NativeMessageResult,
};

#[cfg(target_os = "linux")]
use ashpd::desktop::file_chooser::{FileFilter as PortalFileFilter, SelectedFiles};

/// Check if native dialogs are available.
///
/// On Linux, this checks if we can communicate with the XDG Desktop Portal.
pub fn is_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Try to establish a connection to the portal
        // This is a simplified check - full implementation would verify specific portals
        std::env::var("XDG_CURRENT_DESKTOP").is_ok() || std::env::var("DESKTOP_SESSION").is_ok()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

// ============================================================================
// File Dialogs (async internally, sync API)
// ============================================================================

#[cfg(target_os = "linux")]
fn convert_filters(filters: &[NativeFileFilter]) -> Vec<PortalFileFilter> {
    filters
        .iter()
        .map(|f| {
            let mut filter = PortalFileFilter::new(&f.name);
            for ext in &f.extensions {
                if ext != "*" {
                    filter = filter.glob(&format!("*.{}", ext));
                }
            }
            filter
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn run_async<F, T>(future: F) -> Option<T>
where
    F: std::future::Future<Output = Result<T, ashpd::Error>>,
{
    // Use a simple blocking runtime for the async operation
    // In a real application, you'd want to integrate with the event loop
    tokio_or_block(future).ok()
}

#[cfg(target_os = "linux")]
fn tokio_or_block<F, T>(future: F) -> Result<T, ashpd::Error>
where
    F: std::future::Future<Output = Result<T, ashpd::Error>>,
{
    // Use pollster for simple blocking execution
    // This works for portal dialogs since they handle their own event loop
    pollster::block_on(future)
}

/// Open a native file open dialog for a single file.
#[cfg(target_os = "linux")]
pub fn open_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    let result: Result<Option<PathBuf>, ashpd::Error> = pollster::block_on(async {
        let mut request = SelectedFiles::open_file();

        if let Some(title) = &options.title {
            request = request.title(title.as_str());
        }

        request = request.modal(true);
        request = request.multiple(false);

        for filter in convert_filters(&options.filters) {
            request = request.filter(filter);
        }

        let response = request.send().await?.response()?;
        let uris = response.uris();

        if let Some(uri) = uris.first() {
            if let Ok(path) = uri.to_file_path() {
                return Ok(Some(path));
            }
        }

        Ok(None)
    });

    result.ok().flatten()
}

/// Open a native file open dialog for multiple files.
#[cfg(target_os = "linux")]
pub fn open_files(options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    let result: Result<Option<Vec<PathBuf>>, ashpd::Error> = pollster::block_on(async {
        let mut request = SelectedFiles::open_file();

        if let Some(title) = &options.title {
            request = request.title(title.as_str());
        }

        request = request.modal(true);
        request = request.multiple(true);

        for filter in convert_filters(&options.filters) {
            request = request.filter(filter);
        }

        let response = request.send().await?.response()?;
        let uris = response.uris();

        let paths: Vec<PathBuf> = uris
            .iter()
            .filter_map(|uri| uri.to_file_path().ok())
            .collect();

        if paths.is_empty() {
            Ok(None)
        } else {
            Ok(Some(paths))
        }
    });

    result.ok().flatten()
}

/// Open a native file save dialog.
#[cfg(target_os = "linux")]
pub fn save_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    let result: Result<Option<PathBuf>, ashpd::Error> = pollster::block_on(async {
        let mut request = SelectedFiles::save_file();

        if let Some(title) = &options.title {
            request = request.title(title.as_str());
        }

        if let Some(name) = &options.default_name {
            request = request.current_name(name.as_str());
        }

        request = request.modal(true);

        for filter in convert_filters(&options.filters) {
            request = request.filter(filter);
        }

        let response = request.send().await?.response()?;
        let uris = response.uris();

        if let Some(uri) = uris.first() {
            if let Ok(path) = uri.to_file_path() {
                return Ok(Some(path));
            }
        }

        Ok(None)
    });

    result.ok().flatten()
}

/// Open a native directory selection dialog.
#[cfg(target_os = "linux")]
pub fn select_directory(options: NativeFileDialogOptions) -> Option<PathBuf> {
    let result: Result<Option<PathBuf>, ashpd::Error> = pollster::block_on(async {
        let mut request = SelectedFiles::open_file();

        if let Some(title) = &options.title {
            request = request.title(title.as_str());
        }

        request = request.modal(true);
        request = request.multiple(false);
        request = request.directory(true);

        let response = request.send().await?.response()?;
        let uris = response.uris();

        if let Some(uri) = uris.first() {
            if let Ok(path) = uri.to_file_path() {
                return Ok(Some(path));
            }
        }

        Ok(None)
    });

    result.ok().flatten()
}

// ============================================================================
// Message Dialog
// ============================================================================

/// Show a native message dialog.
///
/// Note: XDG Desktop Portal doesn't have a dedicated message dialog portal.
/// We return None to fall back to the custom implementation.
#[cfg(target_os = "linux")]
pub fn show_message(_options: NativeMessageOptions) -> Option<NativeMessageResult> {
    // XDG Desktop Portal doesn't provide a message dialog portal
    // The Notification portal exists but is not modal
    // Return None to fall back to custom implementation
    None
}

// ============================================================================
// Color Dialog
// ============================================================================

/// Show a native color picker dialog.
///
/// Uses the Screenshot portal's PickColor method.
#[cfg(target_os = "linux")]
pub fn pick_color(_options: NativeColorOptions) -> Option<Color> {
    use ashpd::desktop::Color as PortalColor;

    let result: Result<Color, ashpd::Error> = pollster::block_on(async {
        let response = PortalColor::pick().send().await?.response()?;

        let r = response.red() as f32;
        let g = response.green() as f32;
        let b = response.blue() as f32;

        // Portal color picker doesn't support alpha
        Ok(Color::new(r, g, b, 1.0))
    });

    result.ok()
}

// ============================================================================
// Font Dialog
// ============================================================================

/// Show a native font selection dialog.
///
/// Note: XDG Desktop Portal doesn't have a font dialog portal.
/// We return None to fall back to the custom implementation.
#[cfg(target_os = "linux")]
pub fn pick_font(_options: NativeFontOptions) -> Option<NativeFontDesc> {
    // XDG Desktop Portal doesn't provide a font dialog portal
    // Return None to fall back to custom implementation
    None
}

// ============================================================================
// Stub implementations for non-Linux builds
// ============================================================================

#[cfg(not(target_os = "linux"))]
pub fn open_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn open_files(_options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn save_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn select_directory(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn show_message(_options: NativeMessageOptions) -> Option<NativeMessageResult> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn pick_color(_options: NativeColorOptions) -> Option<Color> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn pick_font(_options: NativeFontOptions) -> Option<NativeFontDesc> {
    None
}
