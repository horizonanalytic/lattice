//! High contrast mode detection.
//!
//! This module provides detection of high contrast / increased contrast
//! accessibility settings on different platforms.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::platform::HighContrast;
//!
//! if HighContrast::is_enabled() {
//!     // Use high contrast colors
//! }
//! ```

/// High contrast mode detection.
///
/// Provides methods to check if the system is running in high contrast mode,
/// which is an accessibility setting that increases visual contrast.
pub struct HighContrast;

impl HighContrast {
    /// Check if high contrast mode is currently enabled.
    ///
    /// # Platform Behavior
    ///
    /// - **Windows**: Checks the `SPI_GETHIGHCONTRAST` system parameter
    /// - **macOS**: Checks the `accessibilityDisplayShouldIncreaseContrast` setting
    /// - **Linux**: Currently always returns `false` (not yet implemented)
    pub fn is_enabled() -> bool {
        Self::is_enabled_platform()
    }

    #[cfg(target_os = "windows")]
    fn is_enabled_platform() -> bool {
        use windows::Win32::UI::Accessibility::{
            SystemParametersInfoW, HIGHCONTRASTW, SPI_GETHIGHCONTRAST,
        };

        // SAFETY: SystemParametersInfoW is a Windows API call that requires:
        // - A valid HIGHCONTRASTW struct with cbSize correctly set
        // - A pointer to that struct cast to c_void
        // All of these requirements are met: hc is a stack-allocated struct
        // with its size field initialized, and we pass a valid pointer to it.
        // The API only reads/writes within the struct's bounds.
        unsafe {
            let mut hc = HIGHCONTRASTW {
                cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
                ..Default::default()
            };

            let result = SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                hc.cbSize,
                Some(&mut hc as *mut _ as *mut std::ffi::c_void),
                Default::default(),
            );

            if result.is_ok() {
                // HCF_HIGHCONTRASTON = 0x00000001
                (hc.dwFlags.0 & 0x00000001) != 0
            } else {
                false
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn is_enabled_platform() -> bool {
        use objc2_app_kit::NSWorkspace;

        let workspace = NSWorkspace::sharedWorkspace();
        workspace.accessibilityDisplayShouldIncreaseContrast()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn is_enabled_platform() -> bool {
        // TODO: Implement for Linux via gsettings or portal
        // For now, return false as a safe default
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_contrast_detection() {
        // Just verify it doesn't panic
        let _enabled = HighContrast::is_enabled();
    }
}
