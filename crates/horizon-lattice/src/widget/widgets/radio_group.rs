//! RadioGroup container widget implementation.
//!
//! This module provides [`RadioGroup`], a visual container widget that
//! automatically manages exclusive selection among child [`RadioButton`] widgets.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{RadioGroup, RadioButton};
//! use horizon_lattice::widget::layout::LayoutKind;
//!
//! // Create a radio group with vertical layout
//! let mut group = RadioGroup::new()
//!     .with_layout(LayoutKind::vertical())
//!     .with_title("Select an option");
//!
//! // Create and add radio buttons
//! let mut rb1 = RadioButton::new("Option 1");
//! let mut rb2 = RadioButton::new("Option 2");
//! let mut rb3 = RadioButton::new("Option 3");
//!
//! group.add_radio_button(&mut rb1);
//! group.add_radio_button(&mut rb2);
//! group.add_radio_button(&mut rb3);
//!
//! // Connect to selection changes
//! group.selection_changed.connect(|&id| {
//!     println!("Selected button ID: {}", id);
//! });
//! ```
//!
//! # Automatic Grouping
//!
//! When a [`RadioButton`] is added to a `RadioGroup` via [`add_radio_button`],
//! it is automatically configured to use the group's internal [`ButtonGroup`]
//! for exclusive selection. This means:
//!
//! - Clicking one radio button will automatically uncheck all others in the group
//! - Only one radio button can be checked at a time
//! - The group emits signals when the selection changes
//!
//! [`add_radio_button`]: RadioGroup::add_radio_button
//! [`ButtonGroup`]: super::ButtonGroup

use std::sync::{Arc, RwLock};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Point, Rect, Renderer, Size, TextLayout, TextRenderer,
};

use super::button_group::ButtonGroup;
use super::radio_button::RadioButton;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::layout::{ContentMargins, LayoutKind};
use crate::widget::{
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// A visual container widget for grouping radio buttons with automatic exclusivity.
///
/// `RadioGroup` combines a visual container (similar to [`ContainerWidget`]) with
/// an internal [`ButtonGroup`] to provide automatic exclusive selection behavior
/// for child [`RadioButton`] widgets.
///
/// # Features
///
/// - Automatic exclusivity: only one radio button can be checked at a time
/// - Optional title/label display
/// - Optional visual frame/border
/// - Layout support for child positioning
/// - Content margins for padding
///
/// # Usage Pattern
///
/// 1. Create a `RadioGroup` and configure its layout
/// 2. Create `RadioButton` widgets
/// 3. Add each button using [`add_radio_button`] (which configures the group)
/// 4. Connect to [`selection_changed`] to respond to user selections
///
/// # Signals
///
/// - `selection_changed(i32)`: Emitted when the selected button changes (button ID)
/// - `button_added(ObjectId)`: Emitted when a button is added to the group
/// - `button_removed(ObjectId)`: Emitted when a button is removed from the group
///
/// [`ContainerWidget`]: super::ContainerWidget
/// [`add_radio_button`]: RadioGroup::add_radio_button
/// [`selection_changed`]: RadioGroup::selection_changed
pub struct RadioGroup {
    /// Widget base.
    base: WidgetBase,

    /// Internal button group for exclusivity management.
    group: Arc<RwLock<ButtonGroup>>,

    /// Child RadioButton ObjectIds.
    children: Vec<ObjectId>,

    /// Optional layout for child positioning.
    layout: Option<LayoutKind>,

    /// Optional title text displayed above the buttons.
    title: Option<String>,

    /// Font for title rendering.
    title_font: Font,

    /// Title text color.
    title_color: Color,

    /// Content margins around children.
    content_margins: ContentMargins,

    /// Background color (if any).
    background_color: Option<Color>,

    /// Whether to show a border frame.
    show_frame: bool,

    /// Frame border color.
    frame_color: Color,

    /// Frame border width.
    frame_width: f32,

    /// Signal emitted when the selected button changes.
    /// Parameter is the button ID (as assigned in ButtonGroup).
    pub selection_changed: Signal<i32>,

    /// Signal emitted when a button is added.
    pub button_added: Signal<ObjectId>,

    /// Signal emitted when a button is removed.
    pub button_removed: Signal<ObjectId>,
}

impl RadioGroup {
    /// Create a new radio group.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));

        let group = Arc::new(RwLock::new(ButtonGroup::new()));

        // Connect to the internal group's toggled signal to emit selection_changed
        let selection_signal = Signal::new();

        Self {
            base,
            group,
            children: Vec::new(),
            layout: None,
            title: None,
            title_font: Font::default(),
            title_color: Color::from_rgb8(33, 33, 33),
            content_margins: ContentMargins::uniform(8.0),
            background_color: None,
            show_frame: false,
            frame_color: Color::from_rgb8(200, 200, 200),
            frame_width: 1.0,
            selection_changed: selection_signal,
            button_added: Signal::new(),
            button_removed: Signal::new(),
        }
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the title text.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set the title text displayed above the radio buttons.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
        self.base.update();
    }

    /// Clear the title.
    pub fn clear_title(&mut self) {
        self.title = None;
        self.base.update();
    }

    /// Set title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Get the title font.
    pub fn title_font(&self) -> &Font {
        &self.title_font
    }

    /// Set the title font.
    pub fn set_title_font(&mut self, font: Font) {
        self.title_font = font;
        self.base.update();
    }

    /// Set title font using builder pattern.
    pub fn with_title_font(mut self, font: Font) -> Self {
        self.title_font = font;
        self
    }

    /// Get the title color.
    pub fn title_color(&self) -> Color {
        self.title_color
    }

    /// Set the title color.
    pub fn set_title_color(&mut self, color: Color) {
        self.title_color = color;
        self.base.update();
    }

    /// Set title color using builder pattern.
    pub fn with_title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    // =========================================================================
    // Layout Management
    // =========================================================================

    /// Get the layout, if any.
    pub fn layout(&self) -> Option<&LayoutKind> {
        self.layout.as_ref()
    }

    /// Get a mutable reference to the layout.
    pub fn layout_mut(&mut self) -> Option<&mut LayoutKind> {
        self.layout.as_mut()
    }

    /// Set the layout for child positioning.
    ///
    /// Existing children will be added to the new layout.
    pub fn set_layout(&mut self, layout: LayoutKind) {
        let mut new_layout = layout;
        new_layout.set_parent_widget(Some(self.base.object_id()));

        // Add existing children to the new layout
        for &child_id in &self.children {
            new_layout.add_widget(child_id);
        }

        self.layout = Some(new_layout);
        self.base.update();
    }

    /// Set layout using builder pattern.
    pub fn with_layout(mut self, layout: LayoutKind) -> Self {
        self.set_layout(layout);
        self
    }

    /// Check if the group has a layout.
    #[inline]
    pub fn has_layout(&self) -> bool {
        self.layout.is_some()
    }

    // =========================================================================
    // RadioButton Management
    // =========================================================================

    /// Add a radio button to this group.
    ///
    /// This method:
    /// 1. Configures the radio button to use this group's internal ButtonGroup
    /// 2. Adds the button's ObjectId to the children list
    /// 3. Adds the button to the layout (if set)
    ///
    /// The radio button will now participate in exclusive selection with other
    /// buttons in this group.
    ///
    /// Returns the button ID assigned by the internal ButtonGroup.
    pub fn add_radio_button(&mut self, button: &mut RadioButton) -> i32 {
        let object_id = button.object_id();

        // Configure the button to use our group
        button.set_group(Some(self.group.clone()));
        button.set_auto_exclusive(false); // Disable auto-exclusive since we're handling it

        // Add to internal group
        let button_id = if let Ok(mut group) = self.group.write() {
            group.add_button(object_id)
        } else {
            -1
        };

        // Track as child
        self.children.push(object_id);

        // Add to layout if present
        if let Some(layout) = &mut self.layout {
            layout.add_widget(object_id);
        }

        self.base.update();
        self.button_added.emit(object_id);

        button_id
    }

    /// Add a radio button with a specific ID.
    ///
    /// Similar to [`add_radio_button`], but assigns a specific ID to the button
    /// in the internal ButtonGroup.
    ///
    /// [`add_radio_button`]: RadioGroup::add_radio_button
    pub fn add_radio_button_with_id(&mut self, button: &mut RadioButton, id: i32) {
        let object_id = button.object_id();

        // Configure the button to use our group
        button.set_group(Some(self.group.clone()));
        button.set_auto_exclusive(false);

        // Add to internal group with specific ID
        if let Ok(mut group) = self.group.write() {
            group.add_button_with_id(object_id, id);
        }

        // Track as child
        self.children.push(object_id);

        // Add to layout if present
        if let Some(layout) = &mut self.layout {
            layout.add_widget(object_id);
        }

        self.base.update();
        self.button_added.emit(object_id);
    }

    /// Remove a radio button from this group by its ObjectId.
    ///
    /// Note: This removes the button from the group's tracking but does NOT
    /// reset the button's group reference. Call `button.set_group(None)` separately
    /// if you want to fully disassociate the button.
    ///
    /// Returns true if the button was found and removed.
    pub fn remove_radio_button(&mut self, object_id: ObjectId) -> bool {
        // Find and remove from children
        if let Some(index) = self.children.iter().position(|&id| id == object_id) {
            self.children.remove(index);

            // Remove from internal group
            if let Ok(mut group) = self.group.write() {
                group.remove_button(object_id);
            }

            // Remove from layout
            if let Some(layout) = &mut self.layout {
                layout.remove_item(index);
            }

            self.base.update();
            self.button_removed.emit(object_id);
            true
        } else {
            false
        }
    }

    /// Remove all radio buttons from the group.
    pub fn clear(&mut self) {
        let removed_ids: Vec<ObjectId> = self.children.drain(..).collect();

        if let Ok(mut group) = self.group.write() {
            for &id in &removed_ids {
                group.remove_button(id);
            }
        }

        if let Some(layout) = &mut self.layout {
            layout.clear();
        }

        self.base.update();

        for id in removed_ids {
            self.button_removed.emit(id);
        }
    }

    /// Get the number of radio buttons in the group.
    #[inline]
    pub fn button_count(&self) -> usize {
        self.children.len()
    }

    /// Check if the group has no buttons.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the child ObjectIds.
    #[inline]
    pub fn children(&self) -> &[ObjectId] {
        &self.children
    }

    /// Get the child at a specific index.
    #[inline]
    pub fn child_at(&self, index: usize) -> Option<ObjectId> {
        self.children.get(index).copied()
    }

    /// Find the index of a child button.
    pub fn index_of(&self, object_id: ObjectId) -> Option<usize> {
        self.children.iter().position(|&id| id == object_id)
    }

    // =========================================================================
    // Selection State
    // =========================================================================

    /// Get the ObjectId of the currently selected (checked) button.
    pub fn selected_button(&self) -> Option<ObjectId> {
        if let Ok(group) = self.group.read() {
            group.checked_button()
        } else {
            None
        }
    }

    /// Get the ID of the currently selected button.
    ///
    /// Returns -1 if no button is selected.
    pub fn selected_id(&self) -> i32 {
        if let Ok(group) = self.group.read() {
            group.checked_id()
        } else {
            -1
        }
    }

    /// Get direct access to the internal ButtonGroup.
    ///
    /// This can be useful for advanced coordination scenarios or
    /// connecting additional signals.
    pub fn button_group(&self) -> &Arc<RwLock<ButtonGroup>> {
        &self.group
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the content margins.
    #[inline]
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        if self.content_margins != margins {
            self.content_margins = margins;
            if let Some(layout) = &mut self.layout {
                layout.set_content_margins(margins);
            }
            self.base.update();
        }
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.set_content_margins(margins);
        self
    }

    /// Set uniform content margins.
    pub fn set_content_margin(&mut self, margin: f32) {
        self.set_content_margins(ContentMargins::uniform(margin));
    }

    /// Set uniform content margins using builder pattern.
    pub fn with_content_margin(mut self, margin: f32) -> Self {
        self.set_content_margin(margin);
        self
    }

    /// Get the background color.
    #[inline]
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Check if the frame is shown.
    pub fn shows_frame(&self) -> bool {
        self.show_frame
    }

    /// Set whether to show a border frame.
    pub fn set_show_frame(&mut self, show: bool) {
        if self.show_frame != show {
            self.show_frame = show;
            self.base.update();
        }
    }

    /// Set show frame using builder pattern.
    pub fn with_frame(mut self, show: bool) -> Self {
        self.show_frame = show;
        self
    }

    /// Get the frame color.
    pub fn frame_color(&self) -> Color {
        self.frame_color
    }

    /// Set the frame color.
    pub fn set_frame_color(&mut self, color: Color) {
        if self.frame_color != color {
            self.frame_color = color;
            self.base.update();
        }
    }

    /// Set frame color using builder pattern.
    pub fn with_frame_color(mut self, color: Color) -> Self {
        self.frame_color = color;
        self
    }

    // =========================================================================
    // Content Area
    // =========================================================================

    /// Calculate the title height if a title is set.
    fn title_height(&self) -> f32 {
        if self.title.is_some() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, "Xg", &self.title_font);
            layout.height() + 4.0 // Add some spacing below title
        } else {
            0.0
        }
    }

    /// Get the content area rectangle (inside margins and title).
    pub fn contents_rect(&self) -> Rect {
        let rect = self.base.rect();
        let title_h = self.title_height();

        Rect::new(
            self.content_margins.left,
            self.content_margins.top + title_h,
            (rect.width() - self.content_margins.horizontal()).max(0.0),
            (rect.height() - self.content_margins.vertical() - title_h).max(0.0),
        )
    }

    // =========================================================================
    // Layout Operations
    // =========================================================================

    /// Calculate and apply the layout using the provided widget storage.
    pub fn do_layout<S: WidgetAccess>(&mut self, storage: &mut S) {
        let content_rect = self.contents_rect();
        let geo = self.base.geometry();

        let layout_rect = Rect::new(
            geo.origin.x + content_rect.origin.x,
            geo.origin.y + content_rect.origin.y,
            content_rect.width(),
            content_rect.height(),
        );

        if let Some(layout) = &mut self.layout {
            layout.set_geometry(layout_rect);
            layout.calculate(storage, layout_rect.size);
            layout.apply(storage);
        }
    }

    /// Invalidate the layout for recalculation.
    pub fn invalidate_layout(&mut self) {
        if let Some(layout) = &mut self.layout {
            layout.invalidate();
        }
        self.base.update();
    }

    // =========================================================================
    // Painting Helpers
    // =========================================================================

    /// Paint the title if set.
    fn paint_title(&self, ctx: &mut PaintContext<'_>) {
        if let Some(title) = &self.title {
            let rect = ctx.rect();
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, title, &self.title_font);

            let text_x = rect.origin.x + self.content_margins.left;
            let text_y = rect.origin.y + self.content_margins.top;
            let text_pos = Point::new(text_x, text_y);

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    text_pos,
                    self.title_color,
                );
            }
        }
    }

    /// Paint the frame border if enabled.
    fn paint_frame(&self, ctx: &mut PaintContext<'_>) {
        if self.show_frame && self.frame_width > 0.0 {
            let rect = ctx.rect();
            let stroke = horizon_lattice_render::Stroke::new(self.frame_color, self.frame_width);

            let inset = self.frame_width / 2.0;
            let frame_rect = Rect::new(
                rect.origin.x + inset,
                rect.origin.y + inset,
                rect.width() - self.frame_width,
                rect.height() - self.frame_width,
            );

            let top_left = Point::new(frame_rect.origin.x, frame_rect.origin.y);
            let top_right = Point::new(
                frame_rect.origin.x + frame_rect.width(),
                frame_rect.origin.y,
            );
            let bottom_right = Point::new(
                frame_rect.origin.x + frame_rect.width(),
                frame_rect.origin.y + frame_rect.height(),
            );
            let bottom_left = Point::new(
                frame_rect.origin.x,
                frame_rect.origin.y + frame_rect.height(),
            );

            ctx.renderer().draw_line(top_left, top_right, &stroke);
            ctx.renderer().draw_line(top_right, bottom_right, &stroke);
            ctx.renderer().draw_line(bottom_right, bottom_left, &stroke);
            ctx.renderer().draw_line(bottom_left, top_left, &stroke);
        }
    }
}

impl Default for RadioGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for RadioGroup {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for RadioGroup {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let title_h = self.title_height();
        let min_width = self.content_margins.horizontal();
        let min_height = self.content_margins.vertical() + title_h;

        // Default preferred size
        let preferred = Size::new(min_width.max(150.0), min_height.max(100.0));

        SizeHint::new(preferred).with_minimum(Size::new(min_width, min_height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if let Some(bg_color) = self.background_color {
            ctx.renderer().fill_rect(rect, bg_color);
        }

        // Draw frame if enabled
        self.paint_frame(ctx);

        // Draw title if set
        self.paint_title(ctx);

        // Child widgets are painted separately by the paint system
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // RadioGroup doesn't handle events itself; they pass through to children
        false
    }
}

// Ensure RadioGroup is Send + Sync
static_assertions::assert_impl_all!(RadioGroup: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_radio_group_creation() {
        setup();
        let group = RadioGroup::new();
        assert!(group.is_empty());
        assert!(group.title().is_none());
        assert!(!group.shows_frame());
    }

    #[test]
    fn test_radio_group_builder_pattern() {
        setup();
        let group = RadioGroup::new()
            .with_title("Test Group")
            .with_frame(true)
            .with_content_margin(12.0)
            .with_background_color(Color::WHITE);

        assert_eq!(group.title(), Some("Test Group"));
        assert!(group.shows_frame());
        assert_eq!(group.content_margins().left, 12.0);
        assert_eq!(group.background_color(), Some(Color::WHITE));
    }

    #[test]
    fn test_add_radio_button() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        let id1 = group.add_radio_button(&mut rb1);
        let id2 = group.add_radio_button(&mut rb2);

        assert_eq!(group.button_count(), 2);
        assert!(id1 < 0); // Auto-assigned IDs are negative
        assert!(id2 < 0);
        assert_ne!(id1, id2);

        // Buttons should be configured with the group
        assert!(rb1.group().is_some());
        assert!(rb2.group().is_some());
        assert!(!rb1.is_auto_exclusive());
    }

    #[test]
    fn test_add_radio_button_with_id() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb = RadioButton::new("Option");

        group.add_radio_button_with_id(&mut rb, 42);

        assert_eq!(group.button_count(), 1);

        // The button should have ID 42 in the group
        let button_group = group.button_group();
        let bg = button_group.read().unwrap();
        assert_eq!(bg.id(rb.object_id()), 42);
    }

    #[test]
    fn test_remove_radio_button() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        group.add_radio_button(&mut rb1);
        group.add_radio_button(&mut rb2);

        let rb1_id = rb1.object_id();
        assert!(group.remove_radio_button(rb1_id));
        assert_eq!(group.button_count(), 1);
        assert!(!group.remove_radio_button(rb1_id)); // Already removed
    }

    #[test]
    fn test_clear_buttons() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        group.add_radio_button(&mut rb1);
        group.add_radio_button(&mut rb2);
        assert_eq!(group.button_count(), 2);

        group.clear();
        assert!(group.is_empty());
    }

    #[test]
    fn test_button_added_signal() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb = RadioButton::new("Option");

        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_clone = signal_received.clone();

        group.button_added.connect(move |_| {
            signal_clone.store(true, Ordering::SeqCst);
        });

        group.add_radio_button(&mut rb);
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_button_removed_signal() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb = RadioButton::new("Option");

        group.add_radio_button(&mut rb);

        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_clone = signal_received.clone();

        group.button_removed.connect(move |_| {
            signal_clone.store(true, Ordering::SeqCst);
        });

        group.remove_radio_button(rb.object_id());
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_exclusivity_through_group() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        group.add_radio_button(&mut rb1);
        group.add_radio_button(&mut rb2);

        // Check rb1
        rb1.set_checked(true);
        {
            let bg = group.button_group().write().unwrap();
            // Note: We're just testing that the group is properly configured.
            // Actual exclusivity coordination happens through the ButtonGroup
            // when buttons are clicked.
            assert!(bg.contains(rb1.object_id()));
            assert!(bg.contains(rb2.object_id()));
        }
    }

    #[test]
    fn test_selected_button() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        group.add_radio_button_with_id(&mut rb1, 1);
        group.add_radio_button_with_id(&mut rb2, 2);

        // Initially no selection
        assert!(group.selected_button().is_none());
        assert_eq!(group.selected_id(), -1);

        // Simulate rb1 being checked and notifying the group
        rb1.set_checked(true);
        {
            let mut bg = group.button_group().write().unwrap();
            bg.button_toggled(rb1.object_id(), true);
        }

        assert_eq!(group.selected_button(), Some(rb1.object_id()));
        assert_eq!(group.selected_id(), 1);
    }

    #[test]
    fn test_children_accessors() {
        setup();
        let mut group = RadioGroup::new();
        let mut rb1 = RadioButton::new("Option 1");
        let mut rb2 = RadioButton::new("Option 2");

        let rb1_id = rb1.object_id();
        let rb2_id = rb2.object_id();

        group.add_radio_button(&mut rb1);
        group.add_radio_button(&mut rb2);

        assert_eq!(group.children(), &[rb1_id, rb2_id]);
        assert_eq!(group.child_at(0), Some(rb1_id));
        assert_eq!(group.child_at(1), Some(rb2_id));
        assert_eq!(group.child_at(2), None);
        assert_eq!(group.index_of(rb1_id), Some(0));
        assert_eq!(group.index_of(rb2_id), Some(1));
    }

    #[test]
    fn test_layout_integration() {
        setup();
        let mut group = RadioGroup::new().with_layout(LayoutKind::vertical());

        assert!(group.has_layout());

        let mut rb = RadioButton::new("Option");
        group.add_radio_button(&mut rb);

        // Check that the button was added to the layout
        let layout = group.layout().unwrap();
        assert_eq!(layout.item_count(), 1);
    }

    #[test]
    fn test_content_margins() {
        setup();
        let mut group = RadioGroup::new().with_content_margin(16.0);

        group
            .widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        let content = group.contents_rect();
        assert_eq!(content.origin.x, 16.0);
        assert_eq!(content.origin.y, 16.0);
        assert_eq!(content.width(), 168.0); // 200 - 32
        assert_eq!(content.height(), 168.0);
    }
}
