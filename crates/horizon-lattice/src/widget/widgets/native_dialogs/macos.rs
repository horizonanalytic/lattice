//! macOS native dialog implementation using AppKit.
//!
//! This module provides native dialog support on macOS using the objc2-app-kit crate.

use std::path::PathBuf;

use horizon_lattice_render::Color;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertStyle, NSModalResponseOK,
    NSOpenPanel, NSSavePanel,
};
use objc2_foundation::NSString;

use super::{
    NativeColorOptions, NativeFileDialogOptions, NativeFileFilter, NativeFontDesc,
    NativeFontOptions, NativeMessageButtons, NativeMessageLevel, NativeMessageOptions,
    NativeMessageResult,
};

/// Check if native dialogs are available.
pub fn is_available() -> bool {
    // On macOS, native dialogs are always available when running as a GUI app
    true
}

// ============================================================================
// File Dialogs
// ============================================================================

/// Convert our filters to NSArray of UTType extension strings.
fn setup_file_types(panel: &NSSavePanel, filters: &[NativeFileFilter]) {
    if filters.is_empty() {
        return;
    }

    // Collect all extensions from all filters
    let mut all_extensions: Vec<String> = Vec::new();
    for filter in filters {
        for ext in &filter.extensions {
            if ext != "*" && !all_extensions.contains(ext) {
                all_extensions.push(ext.clone());
            }
        }
    }

    if all_extensions.is_empty() {
        return;
    }

    // Create NSArray of allowed file types
    let ns_types: Vec<objc2::rc::Retained<NSString>> = all_extensions
        .iter()
        .map(|ext| NSString::from_str(ext))
        .collect();

    let ns_array = objc2_foundation::NSArray::from_retained_slice(&ns_types);

    // Use the deprecated but functional allowedFileTypes API
    // The newer contentTypes API requires more complex UTType handling
    #[allow(deprecated)]
    panel.setAllowedFileTypes(Some(&ns_array));
}

/// Open a native file open dialog for a single file.
pub fn open_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    // Get main thread marker - required for AppKit operations
    let mtm = MainThreadMarker::new()?;

    let panel = NSOpenPanel::openPanel(mtm);

    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);

    if let Some(title) = &options.title {
        let ns_title = NSString::from_str(title);
        panel.setTitle(Some(&ns_title));
    }

    if let Some(dir) = &options.directory
        && let Some(dir_str) = dir.to_str() {
            let ns_path = NSString::from_str(dir_str);
            let url = objc2_foundation::NSURL::fileURLWithPath(&ns_path);
            panel.setDirectoryURL(Some(&url));
        }

    setup_file_types(&panel, &options.filters);

    let response = panel.runModal();
    if response == NSModalResponseOK {
        panel
            .URL()
            .and_then(|url| url.path().map(|p| PathBuf::from(p.to_string())))
    } else {
        None
    }
}

/// Open a native file open dialog for multiple files.
pub fn open_files(options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    let mtm = MainThreadMarker::new()?;

    let panel = NSOpenPanel::openPanel(mtm);

    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(true);

    if let Some(title) = &options.title {
        let ns_title = NSString::from_str(title);
        panel.setTitle(Some(&ns_title));
    }

    if let Some(dir) = &options.directory
        && let Some(dir_str) = dir.to_str() {
            let ns_path = NSString::from_str(dir_str);
            let url = objc2_foundation::NSURL::fileURLWithPath(&ns_path);
            panel.setDirectoryURL(Some(&url));
        }

    setup_file_types(&panel, &options.filters);

    let response = panel.runModal();
    if response == NSModalResponseOK {
        let urls = panel.URLs();
        let mut paths = Vec::new();
        for url in urls {
            if let Some(path) = url.path() {
                paths.push(PathBuf::from(path.to_string()));
            }
        }
        if paths.is_empty() { None } else { Some(paths) }
    } else {
        None
    }
}

/// Open a native file save dialog.
pub fn save_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    let mtm = MainThreadMarker::new()?;

    let panel = NSSavePanel::savePanel(mtm);

    if let Some(title) = &options.title {
        let ns_title = NSString::from_str(title);
        panel.setTitle(Some(&ns_title));
    }

    if let Some(dir) = &options.directory
        && let Some(dir_str) = dir.to_str() {
            let ns_path = NSString::from_str(dir_str);
            let url = objc2_foundation::NSURL::fileURLWithPath(&ns_path);
            panel.setDirectoryURL(Some(&url));
        }

    if let Some(name) = &options.default_name {
        let ns_name = NSString::from_str(name);
        panel.setNameFieldStringValue(&ns_name);
    }

    setup_file_types(&panel, &options.filters);

    let response = panel.runModal();
    if response == NSModalResponseOK {
        panel
            .URL()
            .and_then(|url| url.path().map(|p| PathBuf::from(p.to_string())))
    } else {
        None
    }
}

/// Open a native directory selection dialog.
pub fn select_directory(options: NativeFileDialogOptions) -> Option<PathBuf> {
    let mtm = MainThreadMarker::new()?;

    let panel = NSOpenPanel::openPanel(mtm);

    panel.setCanChooseFiles(false);
    panel.setCanChooseDirectories(true);
    panel.setAllowsMultipleSelection(false);

    if let Some(title) = &options.title {
        let ns_title = NSString::from_str(title);
        panel.setTitle(Some(&ns_title));
    }

    if let Some(dir) = &options.directory
        && let Some(dir_str) = dir.to_str() {
            let ns_path = NSString::from_str(dir_str);
            let url = objc2_foundation::NSURL::fileURLWithPath(&ns_path);
            panel.setDirectoryURL(Some(&url));
        }

    let response = panel.runModal();
    if response == NSModalResponseOK {
        panel
            .URL()
            .and_then(|url| url.path().map(|p| PathBuf::from(p.to_string())))
    } else {
        None
    }
}

// ============================================================================
// Message Dialog
// ============================================================================

/// Show a native message dialog.
pub fn show_message(options: NativeMessageOptions) -> Option<NativeMessageResult> {
    let mtm = MainThreadMarker::new()?;

    let alert = NSAlert::new(mtm);

    // Set message text
    let ns_message = NSString::from_str(&options.message);
    alert.setMessageText(&ns_message);

    // Set informative text if provided
    if let Some(detail) = &options.detail {
        let ns_detail = NSString::from_str(detail);
        alert.setInformativeText(&ns_detail);
    }

    // Set alert style based on level
    let style = match options.level {
        NativeMessageLevel::Info => NSAlertStyle::Informational,
        NativeMessageLevel::Warning => NSAlertStyle::Warning,
        NativeMessageLevel::Error => NSAlertStyle::Critical,
    };
    alert.setAlertStyle(style);

    // Add buttons based on configuration
    match options.buttons {
        NativeMessageButtons::Ok => {
            let btn_title = NSString::from_str("OK");
            let _ = alert.addButtonWithTitle(&btn_title);
        }
        NativeMessageButtons::OkCancel => {
            let ok_title = NSString::from_str("OK");
            let cancel_title = NSString::from_str("Cancel");
            let _ = alert.addButtonWithTitle(&ok_title);
            let _ = alert.addButtonWithTitle(&cancel_title);
        }
        NativeMessageButtons::YesNo => {
            let yes_title = NSString::from_str("Yes");
            let no_title = NSString::from_str("No");
            let _ = alert.addButtonWithTitle(&yes_title);
            let _ = alert.addButtonWithTitle(&no_title);
        }
        NativeMessageButtons::YesNoCancel => {
            let yes_title = NSString::from_str("Yes");
            let no_title = NSString::from_str("No");
            let cancel_title = NSString::from_str("Cancel");
            let _ = alert.addButtonWithTitle(&yes_title);
            let _ = alert.addButtonWithTitle(&no_title);
            let _ = alert.addButtonWithTitle(&cancel_title);
        }
    }

    // Run the alert and get response
    let response = alert.runModal();

    // Map response to our result type
    match options.buttons {
        NativeMessageButtons::Ok => Some(NativeMessageResult::Ok),
        NativeMessageButtons::OkCancel => {
            if response == NSAlertFirstButtonReturn {
                Some(NativeMessageResult::Ok)
            } else {
                Some(NativeMessageResult::Cancel)
            }
        }
        NativeMessageButtons::YesNo => {
            if response == NSAlertFirstButtonReturn {
                Some(NativeMessageResult::Yes)
            } else {
                Some(NativeMessageResult::No)
            }
        }
        NativeMessageButtons::YesNoCancel => {
            if response == NSAlertFirstButtonReturn {
                Some(NativeMessageResult::Yes)
            } else if response == NSAlertSecondButtonReturn {
                Some(NativeMessageResult::No)
            } else {
                Some(NativeMessageResult::Cancel)
            }
        }
    }
}

// ============================================================================
// Color Dialog
// ============================================================================

/// Show a native color picker dialog.
///
/// Note: NSColorPanel is a shared panel that requires delegate-based handling
/// which doesn't fit well with a synchronous API. For now, we return None
/// to fall back to the custom implementation.
pub fn pick_color(_options: NativeColorOptions) -> Option<Color> {
    // NSColorPanel requires complex delegate handling for synchronous operation
    // Return None to fall back to custom implementation
    None
}

// ============================================================================
// Font Dialog
// ============================================================================

/// Show a native font selection dialog.
///
/// Note: NSFontPanel is a shared panel that requires delegate-based handling
/// which doesn't fit well with a synchronous API. For now, we return None
/// to fall back to the custom implementation.
pub fn pick_font(_options: NativeFontOptions) -> Option<NativeFontDesc> {
    // NSFontPanel requires complex delegate handling for synchronous operation
    // Return None to fall back to custom implementation
    None
}
