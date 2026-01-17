//! Stub implementation for unsupported platforms.
//!
//! This module provides no-op implementations for platforms that don't have
//! native dialog support. All functions return `None`, causing the framework
//! to fall back to custom implementations.

use std::path::PathBuf;

use horizon_lattice_render::Color;

use super::{
    NativeColorOptions, NativeFileDialogOptions, NativeFontDesc, NativeFontOptions,
    NativeMessageOptions, NativeMessageResult,
};

/// Native dialogs are not available on this platform.
pub fn is_available() -> bool {
    false
}

/// Not available on this platform.
pub fn open_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

/// Not available on this platform.
pub fn open_files(_options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    None
}

/// Not available on this platform.
pub fn save_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

/// Not available on this platform.
pub fn select_directory(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

/// Not available on this platform.
pub fn show_message(_options: NativeMessageOptions) -> Option<NativeMessageResult> {
    None
}

/// Not available on this platform.
pub fn pick_color(_options: NativeColorOptions) -> Option<Color> {
    None
}

/// Not available on this platform.
pub fn pick_font(_options: NativeFontOptions) -> Option<NativeFontDesc> {
    None
}
