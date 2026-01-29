//! Windows native dialog implementation using Windows API.
//!
//! This module provides native dialog support on Windows using the windows-rs crate.

use std::path::PathBuf;

use horizon_lattice_render::Color;

use super::{
    NativeColorOptions, NativeFileDialogOptions, NativeFileFilter, NativeFontDesc,
    NativeFontOptions, NativeMessageButtons, NativeMessageLevel, NativeMessageOptions,
    NativeMessageResult,
};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*, Win32::Graphics::Gdi::*, Win32::System::Com::*,
    Win32::UI::Controls::Dialogs::*, Win32::UI::Shell::Common::*, Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*, core::*,
};

/// Check if native dialogs are available.
pub fn is_available() -> bool {
    // On Windows, native dialogs are always available
    true
}

// ============================================================================
// File Dialogs
// ============================================================================

#[cfg(target_os = "windows")]
fn create_filter_spec(filters: &[NativeFileFilter]) -> Vec<COMDLG_FILTERSPEC> {
    // This creates filter specs for the file dialog
    // Note: The strings need to be kept alive for the duration of the dialog
    filters
        .iter()
        .map(|f| {
            let pattern = if f.extensions.iter().any(|e| e == "*") {
                "*.*".to_string()
            } else {
                f.extensions
                    .iter()
                    .map(|e| format!("*.{}", e))
                    .collect::<Vec<_>>()
                    .join(";")
            };

            COMDLG_FILTERSPEC {
                pszName: PCWSTR::null(), // Will be set with proper lifetime management
                pszSpec: PCWSTR::null(),
            }
        })
        .collect()
}

/// Open a native file open dialog for a single file.
#[cfg(target_os = "windows")]
pub fn open_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    unsafe {
        // Initialize COM
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        // Create the file open dialog
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;

        // Set options
        let mut opts = dialog.GetOptions().ok()?;
        opts |= FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST;
        dialog.SetOptions(opts).ok()?;

        // Set title if provided
        if let Some(title) = &options.title {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            dialog.SetTitle(PCWSTR(wide.as_ptr())).ok()?;
        }

        // Set initial directory if provided
        if let Some(dir) = &options.directory {
            if let Some(dir_str) = dir.to_str() {
                let wide: Vec<u16> = dir_str.encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(folder) = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) {
                    let _ = dialog.SetFolder(&folder);
                }
            }
        }

        // Set file filters if provided
        if !options.filters.is_empty() {
            // Build filter strings with proper lifetime
            let filter_data: Vec<(Vec<u16>, Vec<u16>)> = options
                .filters
                .iter()
                .map(|f| {
                    let name: Vec<u16> = f.name.encode_utf16().chain(std::iter::once(0)).collect();
                    let pattern = if f.extensions.iter().any(|e| e == "*") {
                        "*.*".to_string()
                    } else {
                        f.extensions
                            .iter()
                            .map(|e| format!("*.{}", e))
                            .collect::<Vec<_>>()
                            .join(";")
                    };
                    let spec: Vec<u16> = pattern.encode_utf16().chain(std::iter::once(0)).collect();
                    (name, spec)
                })
                .collect();

            let filter_specs: Vec<COMDLG_FILTERSPEC> = filter_data
                .iter()
                .map(|(name, spec)| COMDLG_FILTERSPEC {
                    pszName: PCWSTR(name.as_ptr()),
                    pszSpec: PCWSTR(spec.as_ptr()),
                })
                .collect();

            let _ = dialog.SetFileTypes(&filter_specs);
        }

        // Show the dialog
        if dialog.Show(HWND::default()).is_ok() {
            if let Ok(result) = dialog.GetResult() {
                if let Ok(path) = result.GetDisplayName(SIGDN_FILESYSPATH) {
                    let path_str = path.to_string().ok()?;
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        None
    }
}

/// Open a native file open dialog for multiple files.
#[cfg(target_os = "windows")]
pub fn open_files(options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;

        let mut opts = dialog.GetOptions().ok()?;
        opts |= FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST | FOS_ALLOWMULTISELECT;
        dialog.SetOptions(opts).ok()?;

        if let Some(title) = &options.title {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            dialog.SetTitle(PCWSTR(wide.as_ptr())).ok()?;
        }

        if let Some(dir) = &options.directory {
            if let Some(dir_str) = dir.to_str() {
                let wide: Vec<u16> = dir_str.encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(folder) = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) {
                    let _ = dialog.SetFolder(&folder);
                }
            }
        }

        if dialog.Show(HWND::default()).is_ok() {
            if let Ok(results) = dialog.GetResults() {
                let count = results.GetCount().ok()?;
                let mut paths = Vec::new();
                for i in 0..count {
                    if let Ok(item) = results.GetItemAt(i) {
                        if let Ok(path) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                            if let Ok(path_str) = path.to_string() {
                                paths.push(PathBuf::from(path_str));
                            }
                        }
                    }
                }
                if !paths.is_empty() {
                    return Some(paths);
                }
            }
        }

        None
    }
}

/// Open a native file save dialog.
#[cfg(target_os = "windows")]
pub fn save_file(options: NativeFileDialogOptions) -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        let dialog: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;

        let mut opts = dialog.GetOptions().ok()?;
        opts |= FOS_FORCEFILESYSTEM | FOS_OVERWRITEPROMPT;
        dialog.SetOptions(opts).ok()?;

        if let Some(title) = &options.title {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            dialog.SetTitle(PCWSTR(wide.as_ptr())).ok()?;
        }

        if let Some(dir) = &options.directory {
            if let Some(dir_str) = dir.to_str() {
                let wide: Vec<u16> = dir_str.encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(folder) = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) {
                    let _ = dialog.SetFolder(&folder);
                }
            }
        }

        if let Some(name) = &options.default_name {
            let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            dialog.SetFileName(PCWSTR(wide.as_ptr())).ok()?;
        }

        if dialog.Show(HWND::default()).is_ok() {
            if let Ok(result) = dialog.GetResult() {
                if let Ok(path) = result.GetDisplayName(SIGDN_FILESYSPATH) {
                    let path_str = path.to_string().ok()?;
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        None
    }
}

/// Open a native directory selection dialog.
#[cfg(target_os = "windows")]
pub fn select_directory(options: NativeFileDialogOptions) -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;

        let mut opts = dialog.GetOptions().ok()?;
        opts |= FOS_FORCEFILESYSTEM | FOS_PICKFOLDERS;
        dialog.SetOptions(opts).ok()?;

        if let Some(title) = &options.title {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            dialog.SetTitle(PCWSTR(wide.as_ptr())).ok()?;
        }

        if let Some(dir) = &options.directory {
            if let Some(dir_str) = dir.to_str() {
                let wide: Vec<u16> = dir_str.encode_utf16().chain(std::iter::once(0)).collect();
                if let Ok(folder) = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) {
                    let _ = dialog.SetFolder(&folder);
                }
            }
        }

        if dialog.Show(HWND::default()).is_ok() {
            if let Ok(result) = dialog.GetResult() {
                if let Ok(path) = result.GetDisplayName(SIGDN_FILESYSPATH) {
                    let path_str = path.to_string().ok()?;
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        None
    }
}

// ============================================================================
// Message Dialog
// ============================================================================

/// Show a native message dialog.
#[cfg(target_os = "windows")]
pub fn show_message(options: NativeMessageOptions) -> Option<NativeMessageResult> {
    unsafe {
        let title_wide: Vec<u16> = options
            .title
            .as_deref()
            .unwrap_or("Message")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let message_wide: Vec<u16> = options
            .message
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Combine message and detail if detail is provided
        let full_message = if let Some(detail) = &options.detail {
            format!("{}\n\n{}", options.message, detail)
        } else {
            options.message.clone()
        };
        let full_message_wide: Vec<u16> = full_message
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Determine message box type flags
        let icon_flag = match options.level {
            NativeMessageLevel::Info => MB_ICONINFORMATION,
            NativeMessageLevel::Warning => MB_ICONWARNING,
            NativeMessageLevel::Error => MB_ICONERROR,
        };

        let button_flag = match options.buttons {
            NativeMessageButtons::Ok => MB_OK,
            NativeMessageButtons::OkCancel => MB_OKCANCEL,
            NativeMessageButtons::YesNo => MB_YESNO,
            NativeMessageButtons::YesNoCancel => MB_YESNOCANCEL,
        };

        let result = MessageBoxW(
            HWND::default(),
            PCWSTR(full_message_wide.as_ptr()),
            PCWSTR(title_wide.as_ptr()),
            icon_flag | button_flag,
        );

        match result {
            IDOK => Some(NativeMessageResult::Ok),
            IDCANCEL => Some(NativeMessageResult::Cancel),
            IDYES => Some(NativeMessageResult::Yes),
            IDNO => Some(NativeMessageResult::No),
            _ => None,
        }
    }
}

// ============================================================================
// Color Dialog
// ============================================================================

/// Show a native color picker dialog.
#[cfg(target_os = "windows")]
pub fn pick_color(options: NativeColorOptions) -> Option<Color> {
    unsafe {
        let mut custom_colors: [COLORREF; 16] = [COLORREF(0); 16];

        let initial = options.initial_color.unwrap_or(Color::WHITE);
        let initial_rgb = COLORREF(
            ((initial.r * 255.0) as u32)
                | (((initial.g * 255.0) as u32) << 8)
                | (((initial.b * 255.0) as u32) << 16),
        );

        let mut cc = CHOOSECOLORW {
            lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
            hwndOwner: HWND::default(),
            hInstance: HWND::default(),
            rgbResult: initial_rgb,
            lpCustColors: custom_colors.as_mut_ptr(),
            Flags: CC_FULLOPEN | CC_RGBINIT,
            lCustData: 0,
            lpfnHook: None,
            lpTemplateName: PCWSTR::null(),
        };

        if ChooseColorW(&mut cc).is_ok() {
            let rgb = cc.rgbResult.0;
            let r = (rgb & 0xFF) as f32 / 255.0;
            let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
            let b = ((rgb >> 16) & 0xFF) as f32 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        } else {
            None
        }
    }
}

// ============================================================================
// Font Dialog
// ============================================================================

/// Show a native font selection dialog.
#[cfg(target_os = "windows")]
pub fn pick_font(options: NativeFontOptions) -> Option<NativeFontDesc> {
    unsafe {
        let mut lf = LOGFONTW::default();

        // Set initial font if provided
        if let Some(initial) = &options.initial_font {
            // Set family name (max 32 chars including null)
            let family_wide: Vec<u16> = initial
                .family
                .encode_utf16()
                .take(31)
                .chain(std::iter::once(0))
                .collect();
            for (i, c) in family_wide.iter().enumerate() {
                if i < 32 {
                    lf.lfFaceName[i] = *c;
                }
            }

            // Set height (negative for point size)
            lf.lfHeight = -(initial.size as i32);

            // Set weight
            lf.lfWeight = if initial.bold { 700 } else { 400 };

            // Set italic
            lf.lfItalic = if initial.italic { 1 } else { 0 };
        }

        let mut cf = CHOOSEFONTW {
            lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
            hwndOwner: HWND::default(),
            hDC: HDC::default(),
            lpLogFont: &mut lf,
            iPointSize: 0,
            Flags: CF_SCREENFONTS | CF_EFFECTS | CF_INITTOLOGFONTSTRUCT,
            rgbColors: COLORREF(0),
            lCustData: 0,
            lpfnHook: None,
            lpTemplateName: PCWSTR::null(),
            hInstance: HINSTANCE::default(),
            lpszStyle: PWSTR::null(),
            nFontType: 0,
            ___MISSING_ALIGNMENT__: 0,
            nSizeMin: 0,
            nSizeMax: 0,
        };

        if ChooseFontW(&mut cf).is_ok() {
            // Extract the font family name
            let family_end = lf.lfFaceName.iter().position(|&c| c == 0).unwrap_or(32);
            let family = String::from_utf16_lossy(&lf.lfFaceName[..family_end]);

            // Convert point size
            let size = (cf.iPointSize as f32) / 10.0;

            Some(NativeFontDesc {
                family,
                size,
                bold: lf.lfWeight >= 700,
                italic: lf.lfItalic != 0,
            })
        } else {
            None
        }
    }
}

// ============================================================================
// Stub implementations for non-Windows builds
// ============================================================================

#[cfg(not(target_os = "windows"))]
pub fn open_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn open_files(_options: NativeFileDialogOptions) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn save_file(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn select_directory(_options: NativeFileDialogOptions) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn show_message(_options: NativeMessageOptions) -> Option<NativeMessageResult> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn pick_color(_options: NativeColorOptions) -> Option<Color> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn pick_font(_options: NativeFontOptions) -> Option<NativeFontDesc> {
    None
}
