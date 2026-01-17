//! AboutDialog implementation.
//!
//! This module provides [`AboutDialog`], a modal dialog for displaying application
//! information including name, version, description, credits, and license.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::AboutDialog;
//!
//! // Using static helper
//! let about = AboutDialog::about("My App", "1.0.0");
//!
//! // Using builder pattern
//! let mut about = AboutDialog::new()
//!     .with_app_name("My Application")
//!     .with_version("1.2.3")
//!     .with_description("A sample application for demonstration.")
//!     .with_copyright("Copyright © 2025 Example Corp.")
//!     .with_credits("Built by the Example Team")
//!     .with_license("MIT License");
//!
//! about.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Size, Stroke};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;

// ============================================================================
// Button State
// ============================================================================

/// State tracking for clickable text links/buttons.
#[derive(Debug, Clone, Copy, Default)]
struct LinkButtonState {
    hovered: bool,
    pressed: bool,
}

// ============================================================================
// AboutDialog
// ============================================================================

/// A modal dialog for displaying application information.
///
/// AboutDialog provides a standardized way to show information about an application,
/// including:
///
/// - Application name
/// - Version number
/// - Description text
/// - Copyright notice
/// - Credits (expandable section)
/// - License information (expandable section)
///
/// # Static Helpers
///
/// For common use cases, use the static helper methods:
///
/// - [`AboutDialog::about()`]: Simple about dialog with name and version
/// - [`AboutDialog::about_with_description()`]: About dialog with description
pub struct AboutDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// Application name.
    app_name: String,

    /// Application version.
    version: String,

    /// Application description.
    description: String,

    /// Copyright notice.
    copyright: String,

    /// Credits text (authors, contributors, etc.).
    credits: String,

    /// License text.
    license: String,

    /// Optional application icon path (for future use).
    icon_path: Option<String>,

    // Visual styling
    /// Icon/logo area size.
    icon_size: f32,
    /// Content padding.
    content_padding: f32,
    /// Spacing between elements.
    element_spacing: f32,
    /// App name text color.
    name_color: Color,
    /// Version text color.
    version_color: Color,
    /// Description text color.
    description_color: Color,
    /// Copyright text color.
    copyright_color: Color,
    /// Link text color.
    link_color: Color,
    /// Link hover color.
    link_hover_color: Color,

    // Expandable section states
    /// Whether credits section is expanded.
    credits_expanded: bool,
    /// Whether license section is expanded.
    license_expanded: bool,
    /// Credits button state.
    credits_button_state: LinkButtonState,
    /// License button state.
    license_button_state: LinkButtonState,
    /// Expanded section height.
    expanded_section_height: f32,

    // Signals
    /// Signal emitted when the "Credits" link is clicked.
    pub credits_clicked: Signal<()>,

    /// Signal emitted when the "License" link is clicked.
    pub license_clicked: Signal<()>,
}

impl AboutDialog {
    /// Create a new about dialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("About")
            .with_size(400.0, 300.0)
            .with_standard_buttons(StandardButton::CLOSE);

        Self {
            dialog,
            app_name: String::new(),
            version: String::new(),
            description: String::new(),
            copyright: String::new(),
            credits: String::new(),
            license: String::new(),
            icon_path: None,
            icon_size: 64.0,
            content_padding: 24.0,
            element_spacing: 8.0,
            name_color: Color::from_rgb8(32, 32, 32),
            version_color: Color::from_rgb8(96, 96, 96),
            description_color: Color::from_rgb8(64, 64, 64),
            copyright_color: Color::from_rgb8(128, 128, 128),
            link_color: Color::from_rgb8(0, 102, 204),
            link_hover_color: Color::from_rgb8(0, 76, 153),
            credits_expanded: false,
            license_expanded: false,
            credits_button_state: LinkButtonState::default(),
            license_button_state: LinkButtonState::default(),
            expanded_section_height: 120.0,
            credits_clicked: Signal::new(),
            license_clicked: Signal::new(),
        }
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Create a simple about dialog with application name and version.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name
    /// * `version` - The version string
    ///
    /// # Example
    ///
    /// ```ignore
    /// let about = AboutDialog::about("My App", "1.0.0");
    /// about.open();
    /// ```
    pub fn about(app_name: impl Into<String>, version: impl Into<String>) -> Self {
        let name = app_name.into();
        Self::new()
            .with_title(format!("About {}", &name))
            .with_app_name(name)
            .with_version(version)
    }

    /// Create an about dialog with name, version, and description.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name
    /// * `version` - The version string
    /// * `description` - A brief description of the application
    ///
    /// # Example
    ///
    /// ```ignore
    /// let about = AboutDialog::about_with_description(
    ///     "My App",
    ///     "1.0.0",
    ///     "A powerful tool for doing things.",
    /// );
    /// about.open();
    /// ```
    pub fn about_with_description(
        app_name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let name = app_name.into();
        Self::new()
            .with_title(format!("About {}", &name))
            .with_app_name(name)
            .with_version(version)
            .with_description(description)
    }

    /// Create a fully configured about dialog.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name
    /// * `version` - The version string
    /// * `description` - A brief description
    /// * `copyright` - Copyright notice
    /// * `credits` - Credits/authors text (optional)
    /// * `license` - License text (optional)
    pub fn about_full(
        app_name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        copyright: impl Into<String>,
        credits: Option<impl Into<String>>,
        license: Option<impl Into<String>>,
    ) -> Self {
        let name = app_name.into();
        let mut dialog = Self::new()
            .with_title(format!("About {}", &name))
            .with_app_name(name)
            .with_version(version)
            .with_description(description)
            .with_copyright(copyright);

        if let Some(credits) = credits {
            dialog = dialog.with_credits(credits);
        }
        if let Some(license) = license {
            dialog = dialog.with_license(license);
        }

        dialog
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the application name using builder pattern.
    pub fn with_app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self.update_size();
        self
    }

    /// Set the version using builder pattern.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self.update_size();
        self
    }

    /// Set the description using builder pattern.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self.update_size();
        self
    }

    /// Set the copyright notice using builder pattern.
    pub fn with_copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = copyright.into();
        self.update_size();
        self
    }

    /// Set the credits text using builder pattern.
    pub fn with_credits(mut self, credits: impl Into<String>) -> Self {
        self.credits = credits.into();
        self.update_size();
        self
    }

    /// Set the license text using builder pattern.
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = license.into();
        self.update_size();
        self
    }

    /// Set the icon path using builder pattern.
    pub fn with_icon_path(mut self, path: impl Into<String>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog = std::mem::take(&mut self.dialog).with_size(width, height);
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the application name.
    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    /// Set the application name.
    pub fn set_app_name(&mut self, name: impl Into<String>) {
        self.app_name = name.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Set the version.
    pub fn set_version(&mut self, version: impl Into<String>) {
        self.version = version.into();
        self.dialog.widget_base_mut().update();
    }

    /// Get the description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Set the description.
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = description.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the copyright notice.
    pub fn copyright(&self) -> &str {
        &self.copyright
    }

    /// Set the copyright notice.
    pub fn set_copyright(&mut self, copyright: impl Into<String>) {
        self.copyright = copyright.into();
        self.dialog.widget_base_mut().update();
    }

    /// Get the credits text.
    pub fn credits(&self) -> &str {
        &self.credits
    }

    /// Set the credits text.
    pub fn set_credits(&mut self, credits: impl Into<String>) {
        self.credits = credits.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the license text.
    pub fn license(&self) -> &str {
        &self.license
    }

    /// Set the license text.
    pub fn set_license(&mut self, license: impl Into<String>) {
        self.license = license.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    /// Check if credits section is expanded.
    pub fn credits_expanded(&self) -> bool {
        self.credits_expanded
    }

    /// Set credits section expanded state.
    pub fn set_credits_expanded(&mut self, expanded: bool) {
        if self.credits_expanded != expanded && !self.credits.is_empty() {
            self.credits_expanded = expanded;
            self.update_size();
            self.dialog.widget_base_mut().update();
        }
    }

    /// Check if license section is expanded.
    pub fn license_expanded(&self) -> bool {
        self.license_expanded
    }

    /// Set license section expanded state.
    pub fn set_license_expanded(&mut self, expanded: bool) {
        if self.license_expanded != expanded && !self.license.is_empty() {
            self.license_expanded = expanded;
            self.update_size();
            self.dialog.widget_base_mut().update();
        }
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the about dialog (non-blocking modal).
    pub fn open(&mut self) {
        self.dialog.open();
    }

    /// Accept the dialog.
    pub fn accept(&mut self) {
        self.dialog.accept();
    }

    /// Reject the dialog.
    pub fn reject(&mut self) {
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    // =========================================================================
    // Signal Access (delegated from dialog)
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
    }

    // =========================================================================
    // Size Calculation
    // =========================================================================

    /// Update the dialog size based on content.
    fn update_size(&mut self) {
        let min_width: f32 = 350.0;
        let max_width: f32 = 450.0;
        let title_bar_height: f32 = 28.0;
        let button_row_height: f32 = 48.0;

        // Calculate content height
        let mut content_height = self.content_padding * 2.0;

        // Icon area (centered at top)
        content_height += self.icon_size + self.element_spacing;

        // App name (larger text)
        if !self.app_name.is_empty() {
            content_height += 28.0 + self.element_spacing;
        }

        // Version
        if !self.version.is_empty() {
            content_height += 18.0 + self.element_spacing;
        }

        // Description (may wrap)
        if !self.description.is_empty() {
            let text_width = max_width - self.content_padding * 2.0;
            let chars_per_line = (text_width / 7.0) as usize;
            let lines = (self.description.len() / chars_per_line).max(1);
            content_height += (lines as f32 * 18.0) + self.element_spacing;
        }

        // Copyright
        if !self.copyright.is_empty() {
            content_height += 16.0 + self.element_spacing;
        }

        // Credits link/button
        if !self.credits.is_empty() {
            content_height += 20.0 + self.element_spacing;
            if self.credits_expanded {
                content_height += self.expanded_section_height + self.element_spacing;
            }
        }

        // License link/button
        if !self.license.is_empty() {
            content_height += 20.0 + self.element_spacing;
            if self.license_expanded {
                content_height += self.expanded_section_height + self.element_spacing;
            }
        }

        let total_height = title_bar_height + content_height + button_row_height;
        let width = min_width.max(max_width.min(400.0));
        let height = total_height.max(250.0).min(600.0);

        self.dialog.widget_base_mut().set_size(Size::new(width, height));
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Get the icon rectangle (centered at top of content area).
    fn icon_rect(&self) -> Rect {
        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();
        let x = (rect.width() - self.icon_size) / 2.0;

        Rect::new(
            x,
            title_bar_height + self.content_padding,
            self.icon_size,
            self.icon_size,
        )
    }

    /// Get the credits link button rectangle.
    fn credits_link_rect(&self) -> Option<Rect> {
        if self.credits.is_empty() {
            return None;
        }

        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();

        // Calculate Y position based on content above
        let mut y = title_bar_height + self.content_padding + self.icon_size + self.element_spacing;

        if !self.app_name.is_empty() {
            y += 28.0 + self.element_spacing;
        }
        if !self.version.is_empty() {
            y += 18.0 + self.element_spacing;
        }
        if !self.description.is_empty() {
            let text_width = rect.width() - self.content_padding * 2.0;
            let chars_per_line = (text_width / 7.0) as usize;
            let lines = (self.description.len() / chars_per_line).max(1);
            y += (lines as f32 * 18.0) + self.element_spacing;
        }
        if !self.copyright.is_empty() {
            y += 16.0 + self.element_spacing;
        }

        Some(Rect::new(
            self.content_padding,
            y,
            80.0, // "Credits" text width approximation
            20.0,
        ))
    }

    /// Get the credits expanded section rectangle.
    fn credits_section_rect(&self) -> Option<Rect> {
        if self.credits.is_empty() || !self.credits_expanded {
            return None;
        }

        let rect = self.dialog.widget_base().rect();

        if let Some(link_rect) = self.credits_link_rect() {
            Some(Rect::new(
                self.content_padding,
                link_rect.origin.y + link_rect.height() + 4.0,
                rect.width() - self.content_padding * 2.0,
                self.expanded_section_height,
            ))
        } else {
            None
        }
    }

    /// Get the license link button rectangle.
    fn license_link_rect(&self) -> Option<Rect> {
        if self.license.is_empty() {
            return None;
        }

        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();

        // Calculate Y position based on content above
        let mut y = title_bar_height + self.content_padding + self.icon_size + self.element_spacing;

        if !self.app_name.is_empty() {
            y += 28.0 + self.element_spacing;
        }
        if !self.version.is_empty() {
            y += 18.0 + self.element_spacing;
        }
        if !self.description.is_empty() {
            let text_width = rect.width() - self.content_padding * 2.0;
            let chars_per_line = (text_width / 7.0) as usize;
            let lines = (self.description.len() / chars_per_line).max(1);
            y += (lines as f32 * 18.0) + self.element_spacing;
        }
        if !self.copyright.is_empty() {
            y += 16.0 + self.element_spacing;
        }
        if !self.credits.is_empty() {
            y += 20.0 + self.element_spacing;
            if self.credits_expanded {
                y += self.expanded_section_height + self.element_spacing;
            }
        }

        Some(Rect::new(
            self.content_padding,
            y,
            80.0, // "License" text width approximation
            20.0,
        ))
    }

    /// Get the license expanded section rectangle.
    fn license_section_rect(&self) -> Option<Rect> {
        if self.license.is_empty() || !self.license_expanded {
            return None;
        }

        let rect = self.dialog.widget_base().rect();

        if let Some(link_rect) = self.license_link_rect() {
            Some(Rect::new(
                self.content_padding,
                link_rect.origin.y + link_rect.height() + 4.0,
                rect.width() - self.content_padding * 2.0,
                self.expanded_section_height,
            ))
        } else {
            None
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check credits link click
        if let Some(credits_rect) = self.credits_link_rect() {
            if credits_rect.contains(event.local_pos) {
                self.credits_button_state.pressed = true;
                self.dialog.widget_base_mut().update();
                return true;
            }
        }

        // Check license link click
        if let Some(license_rect) = self.license_link_rect() {
            if license_rect.contains(event.local_pos) {
                self.license_button_state.pressed = true;
                self.dialog.widget_base_mut().update();
                return true;
            }
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check credits link release
        if self.credits_button_state.pressed {
            self.credits_button_state.pressed = false;

            if let Some(credits_rect) = self.credits_link_rect() {
                if credits_rect.contains(event.local_pos) {
                    self.credits_expanded = !self.credits_expanded;
                    self.credits_clicked.emit(());
                    self.update_size();
                }
            }
            self.dialog.widget_base_mut().update();
            return true;
        }

        // Check license link release
        if self.license_button_state.pressed {
            self.license_button_state.pressed = false;

            if let Some(license_rect) = self.license_link_rect() {
                if license_rect.contains(event.local_pos) {
                    self.license_expanded = !self.license_expanded;
                    self.license_clicked.emit(());
                    self.update_size();
                }
            }
            self.dialog.widget_base_mut().update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let mut updated = false;

        // Update credits link hover state
        if let Some(credits_rect) = self.credits_link_rect() {
            let new_hover = credits_rect.contains(event.local_pos);
            if self.credits_button_state.hovered != new_hover {
                self.credits_button_state.hovered = new_hover;
                updated = true;
            }
        }

        // Update license link hover state
        if let Some(license_rect) = self.license_link_rect() {
            let new_hover = license_rect.contains(event.local_pos);
            if self.license_button_state.hovered != new_hover {
                self.license_button_state.hovered = new_hover;
                updated = true;
            }
        }

        if updated {
            self.dialog.widget_base_mut().update();
        }

        updated
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to close
        if event.key == Key::Escape {
            self.close();
            return true;
        }

        // Enter to close (accept)
        if event.key == Key::Enter && !event.is_repeat {
            self.accept();
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_icon(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.icon_rect();
        let center = Point::new(
            rect.origin.x + rect.width() / 2.0,
            rect.origin.y + rect.height() / 2.0,
        );

        // Draw a placeholder application icon (rounded rectangle with gradient-like effect)
        let icon_bg = Color::from_rgb8(66, 133, 244); // Nice blue
        let rrect = RoundedRect::new(rect, 12.0);
        ctx.renderer().fill_rounded_rect(rrect, icon_bg);

        // Draw an "i" for info/about
        let stroke = Stroke::new(Color::WHITE, 3.0);

        // Dot at top
        let dot_rect = Rect::new(center.x - 3.0, center.y - 16.0, 6.0, 6.0);
        ctx.renderer().fill_rounded_rect(RoundedRect::new(dot_rect, 3.0), Color::WHITE);

        // Vertical line below
        ctx.renderer().draw_line(
            Point::new(center.x, center.y - 6.0),
            Point::new(center.x, center.y + 16.0),
            &stroke,
        );
    }

    fn paint_credits_section(&self, ctx: &mut PaintContext<'_>) {
        // Paint credits link
        if let Some(link_rect) = self.credits_link_rect() {
            let color = if self.credits_button_state.hovered || self.credits_button_state.pressed {
                self.link_hover_color
            } else {
                self.link_color
            };

            // Draw underline for link appearance
            let stroke = Stroke::new(color, 1.0);
            ctx.renderer().draw_line(
                Point::new(link_rect.origin.x, link_rect.origin.y + link_rect.height() - 2.0),
                Point::new(link_rect.origin.x + link_rect.width(), link_rect.origin.y + link_rect.height() - 2.0),
                &stroke,
            );
        }

        // Paint expanded credits section
        if let Some(section_rect) = self.credits_section_rect() {
            let bg_color = Color::from_rgb8(248, 248, 248);
            let rrect = RoundedRect::new(section_rect, 4.0);
            ctx.renderer().fill_rounded_rect(rrect, bg_color);

            let stroke = Stroke::new(Color::from_rgb8(220, 220, 220), 1.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);
        }
    }

    fn paint_license_section(&self, ctx: &mut PaintContext<'_>) {
        // Paint license link
        if let Some(link_rect) = self.license_link_rect() {
            let color = if self.license_button_state.hovered || self.license_button_state.pressed {
                self.link_hover_color
            } else {
                self.link_color
            };

            // Draw underline for link appearance
            let stroke = Stroke::new(color, 1.0);
            ctx.renderer().draw_line(
                Point::new(link_rect.origin.x, link_rect.origin.y + link_rect.height() - 2.0),
                Point::new(link_rect.origin.x + link_rect.width(), link_rect.origin.y + link_rect.height() - 2.0),
                &stroke,
            );
        }

        // Paint expanded license section
        if let Some(section_rect) = self.license_section_rect() {
            let bg_color = Color::from_rgb8(248, 248, 248);
            let rrect = RoundedRect::new(section_rect, 4.0);
            ctx.renderer().fill_rounded_rect(rrect, bg_color);

            let stroke = Stroke::new(Color::from_rgb8(220, 220, 220), 1.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);
        }
    }
}

impl Object for AboutDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for AboutDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.dialog.size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint AboutDialog-specific content
        self.paint_icon(ctx);

        // Paint expandable sections
        if !self.credits.is_empty() {
            self.paint_credits_section(ctx);
        }
        if !self.license.is_empty() {
            self.paint_license_section(ctx);
        }

        // Note: Text rendering (app name, version, description, etc.) would be
        // handled by the text rendering system. The text fields are stored and
        // positions can be calculated from the geometry methods.
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            _ => false,
        };

        if handled {
            return true;
        }

        // Delegate to dialog
        self.dialog.event(event)
    }
}

impl Default for AboutDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_about_dialog_creation() {
        setup();
        let about = AboutDialog::new();
        assert!(about.app_name().is_empty());
        assert!(about.version().is_empty());
        assert!(!about.is_open());
    }

    #[test]
    fn test_about_dialog_builder_pattern() {
        setup();
        let about = AboutDialog::new()
            .with_app_name("Test App")
            .with_version("1.2.3")
            .with_description("A test application")
            .with_copyright("Copyright © 2025")
            .with_credits("John Doe")
            .with_license("MIT License");

        assert_eq!(about.app_name(), "Test App");
        assert_eq!(about.version(), "1.2.3");
        assert_eq!(about.description(), "A test application");
        assert_eq!(about.copyright(), "Copyright © 2025");
        assert_eq!(about.credits(), "John Doe");
        assert_eq!(about.license(), "MIT License");
    }

    #[test]
    fn test_about_static_helper() {
        setup();
        let about = AboutDialog::about("My App", "2.0.0");
        assert_eq!(about.app_name(), "My App");
        assert_eq!(about.version(), "2.0.0");
    }

    #[test]
    fn test_about_with_description_helper() {
        setup();
        let about = AboutDialog::about_with_description(
            "My App",
            "2.0.0",
            "Does useful things",
        );
        assert_eq!(about.app_name(), "My App");
        assert_eq!(about.version(), "2.0.0");
        assert_eq!(about.description(), "Does useful things");
    }

    #[test]
    fn test_about_full_helper() {
        setup();
        let about = AboutDialog::about_full(
            "Full App",
            "3.0.0",
            "Complete description",
            "© 2025 Test Corp",
            Some("Credits here"),
            Some("MIT"),
        );
        assert_eq!(about.app_name(), "Full App");
        assert_eq!(about.version(), "3.0.0");
        assert_eq!(about.description(), "Complete description");
        assert_eq!(about.copyright(), "© 2025 Test Corp");
        assert_eq!(about.credits(), "Credits here");
        assert_eq!(about.license(), "MIT");
    }

    #[test]
    fn test_property_setters() {
        setup();
        let mut about = AboutDialog::new();

        about.set_app_name("Updated App");
        assert_eq!(about.app_name(), "Updated App");

        about.set_version("4.0.0");
        assert_eq!(about.version(), "4.0.0");

        about.set_description("New description");
        assert_eq!(about.description(), "New description");

        about.set_copyright("© 2026");
        assert_eq!(about.copyright(), "© 2026");

        about.set_credits("New credits");
        assert_eq!(about.credits(), "New credits");

        about.set_license("Apache 2.0");
        assert_eq!(about.license(), "Apache 2.0");
    }

    #[test]
    fn test_expanded_sections() {
        setup();
        let mut about = AboutDialog::new()
            .with_credits("Some credits")
            .with_license("Some license");

        assert!(!about.credits_expanded());
        assert!(!about.license_expanded());

        about.set_credits_expanded(true);
        assert!(about.credits_expanded());

        about.set_license_expanded(true);
        assert!(about.license_expanded());
    }

    #[test]
    fn test_dialog_lifecycle() {
        setup();
        let mut about = AboutDialog::about("Test", "1.0");

        assert!(!about.is_open());
        about.open();
        assert!(about.is_open());
        about.close();
        assert!(!about.is_open());
    }
}
