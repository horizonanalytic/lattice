//! Window type definitions.
//!
//! Different window types receive different treatment by the window manager
//! and have different default behaviors.

use crate::widget::widgets::WindowFlags;

/// The type of window, which affects its default behavior and appearance.
///
/// Different window types are treated differently by the platform's window manager:
/// - They may have different decorations (title bar size, buttons)
/// - They may have different z-ordering behavior
/// - They may have different stacking relationships
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::WindowType;
///
/// // A normal application window
/// let window_type = WindowType::Normal;
///
/// // A dialog window (typically modal, centered on parent)
/// let dialog_type = WindowType::Dialog;
///
/// // A tool window (small title bar, stays on top of normal windows)
/// let tool_type = WindowType::Tool;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WindowType {
    /// A normal top-level window.
    ///
    /// This is the default window type for main application windows.
    /// Normal windows have full decorations including title bar, minimize,
    /// maximize, and close buttons. They appear in the taskbar and can
    /// be minimized.
    #[default]
    Normal,

    /// A dialog window.
    ///
    /// Dialogs are typically used for modal interactions with the user.
    /// They have a simplified title bar (usually just title and close button),
    /// and are often centered on their parent window. They may not appear
    /// in the taskbar.
    ///
    /// Dialog windows typically:
    /// - Have a close button but no minimize/maximize
    /// - Are not resizable by default
    /// - Stay above their parent window
    /// - Don't appear in the taskbar (platform-dependent)
    Dialog,

    /// A tool window (palette, inspector, etc.).
    ///
    /// Tool windows have a smaller title bar and typically stay on top
    /// of normal windows. They are often used for floating palettes,
    /// property inspectors, or toolboxes.
    ///
    /// Tool windows typically:
    /// - Have a smaller/thinner title bar
    /// - Stay on top of normal windows (but below other tool windows)
    /// - May not appear in the taskbar
    /// - Are not shown in the Alt+Tab window switcher (platform-dependent)
    Tool,

    /// A popup window (menu, dropdown, tooltip).
    ///
    /// Popup windows have no decorations and are used for menus,
    /// dropdowns, tooltips, and similar transient UI elements.
    ///
    /// Popup windows typically:
    /// - Have no title bar or border (frameless)
    /// - Don't receive focus (may depend on usage)
    /// - Are dismissed when clicking outside
    /// - Stay on top of all other windows
    /// - Don't appear in the taskbar or window switcher
    Popup,

    /// A splash screen window.
    ///
    /// Splash screens are displayed during application startup and
    /// have no decorations. They are typically centered on the screen.
    ///
    /// Splash windows typically:
    /// - Have no title bar or border (frameless)
    /// - Are centered on the screen
    /// - Stay on top of other windows
    /// - Don't appear in the taskbar
    /// - Are not resizable or movable
    Splash,
}

impl WindowType {
    /// Get the default window flags for this window type.
    ///
    /// These are the recommended starting flags; they can be customized
    /// as needed.
    pub fn default_flags(&self) -> WindowFlags {
        match self {
            WindowType::Normal => WindowFlags::DEFAULT,
            WindowType::Dialog => WindowFlags::DIALOG,
            WindowType::Tool => WindowFlags::TOOL,
            WindowType::Popup => WindowFlags::FRAMELESS | WindowFlags::STAYS_ON_TOP,
            WindowType::Splash => WindowFlags::FRAMELESS,
        }
    }

    /// Check if this window type should have decorations by default.
    pub fn has_decorations(&self) -> bool {
        match self {
            WindowType::Normal | WindowType::Dialog | WindowType::Tool => true,
            WindowType::Popup | WindowType::Splash => false,
        }
    }

    /// Check if this window type should stay on top by default.
    pub fn stays_on_top(&self) -> bool {
        match self {
            WindowType::Tool | WindowType::Popup | WindowType::Splash => true,
            WindowType::Normal | WindowType::Dialog => false,
        }
    }

    /// Check if this window type should appear in the taskbar.
    pub fn shows_in_taskbar(&self) -> bool {
        match self {
            WindowType::Normal => true,
            WindowType::Dialog | WindowType::Tool | WindowType::Popup | WindowType::Splash => false,
        }
    }

    /// Check if this window type is typically resizable.
    pub fn is_resizable(&self) -> bool {
        match self {
            WindowType::Normal => true,
            WindowType::Dialog | WindowType::Tool | WindowType::Popup | WindowType::Splash => false,
        }
    }
}

impl std::fmt::Display for WindowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowType::Normal => write!(f, "Normal"),
            WindowType::Dialog => write!(f, "Dialog"),
            WindowType::Tool => write!(f, "Tool"),
            WindowType::Popup => write!(f, "Popup"),
            WindowType::Splash => write!(f, "Splash"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_type_default() {
        assert_eq!(WindowType::default(), WindowType::Normal);
    }

    #[test]
    fn test_window_type_decorations() {
        assert!(WindowType::Normal.has_decorations());
        assert!(WindowType::Dialog.has_decorations());
        assert!(WindowType::Tool.has_decorations());
        assert!(!WindowType::Popup.has_decorations());
        assert!(!WindowType::Splash.has_decorations());
    }

    #[test]
    fn test_window_type_stays_on_top() {
        assert!(!WindowType::Normal.stays_on_top());
        assert!(!WindowType::Dialog.stays_on_top());
        assert!(WindowType::Tool.stays_on_top());
        assert!(WindowType::Popup.stays_on_top());
        assert!(WindowType::Splash.stays_on_top());
    }

    #[test]
    fn test_window_type_taskbar() {
        assert!(WindowType::Normal.shows_in_taskbar());
        assert!(!WindowType::Dialog.shows_in_taskbar());
        assert!(!WindowType::Tool.shows_in_taskbar());
        assert!(!WindowType::Popup.shows_in_taskbar());
        assert!(!WindowType::Splash.shows_in_taskbar());
    }

    #[test]
    fn test_window_type_resizable() {
        assert!(WindowType::Normal.is_resizable());
        assert!(!WindowType::Dialog.is_resizable());
        assert!(!WindowType::Tool.is_resizable());
        assert!(!WindowType::Popup.is_resizable());
        assert!(!WindowType::Splash.is_resizable());
    }

    #[test]
    fn test_window_type_display() {
        assert_eq!(format!("{}", WindowType::Normal), "Normal");
        assert_eq!(format!("{}", WindowType::Dialog), "Dialog");
        assert_eq!(format!("{}", WindowType::Tool), "Tool");
        assert_eq!(format!("{}", WindowType::Popup), "Popup");
        assert_eq!(format!("{}", WindowType::Splash), "Splash");
    }

    #[test]
    fn test_window_type_default_flags() {
        let flags = WindowType::Normal.default_flags();
        assert!(flags.has_minimize_button());
        assert!(flags.has_maximize_button());
        assert!(flags.has_close_button());
        assert!(flags.is_resizable());

        let dialog_flags = WindowType::Dialog.default_flags();
        assert!(dialog_flags.has_close_button());
        assert!(!dialog_flags.has_minimize_button());
        assert!(!dialog_flags.has_maximize_button());
        assert!(!dialog_flags.is_resizable());

        let popup_flags = WindowType::Popup.default_flags();
        assert!(popup_flags.is_frameless());
        assert!(popup_flags.stays_on_top());

        let splash_flags = WindowType::Splash.default_flags();
        assert!(splash_flags.is_frameless());
    }
}
