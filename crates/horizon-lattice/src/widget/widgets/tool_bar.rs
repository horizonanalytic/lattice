//! ToolBar widget implementation.
//!
//! This module provides [`ToolBar`], a container widget for action buttons,
//! separators, and custom widgets. It supports horizontal and vertical
//! orientations, movability, floatability, and overflow handling.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ToolBar, ToolBarArea, Action};
//! use std::sync::Arc;
//!
//! // Create a toolbar
//! let mut toolbar = ToolBar::new("Main");
//!
//! // Add actions
//! let open_action = Arc::new(Action::new("&Open"));
//! let save_action = Arc::new(Action::new("&Save"));
//!
//! toolbar.add_action(open_action.clone());
//! toolbar.add_separator();
//! toolbar.add_action(save_action.clone());
//!
//! // Connect to triggered signal
//! toolbar.action_triggered.connect(|action| {
//!     println!("Action triggered: {}", action.display_text());
//! });
//! ```

use std::collections::HashMap;
use std::ops::{BitAnd, BitOr, BitOrAssign};
use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Font, FontFamily, Point, Rect, Renderer, Size, Stroke};

use crate::widget::{
    FocusPolicy, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext,
    SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

use super::tool_button::ToolButtonStyle;
use super::{Action, Menu, Orientation, PopupPlacement};

// ============================================================================
// ToolBarArea
// ============================================================================

/// Toolbar areas within a MainWindow.
///
/// These define the regions where toolbars can be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ToolBarArea {
    /// Top toolbar area (below menu bar).
    #[default]
    Top,
    /// Bottom toolbar area (above status bar).
    Bottom,
    /// Left toolbar area.
    Left,
    /// Right toolbar area.
    Right,
}

impl ToolBarArea {
    /// Returns all toolbar areas as an iterator.
    pub fn all() -> impl Iterator<Item = ToolBarArea> {
        [
            ToolBarArea::Top,
            ToolBarArea::Bottom,
            ToolBarArea::Left,
            ToolBarArea::Right,
        ]
        .into_iter()
    }

    /// Check if this is a horizontal toolbar area (top or bottom).
    pub fn is_horizontal(&self) -> bool {
        matches!(self, ToolBarArea::Top | ToolBarArea::Bottom)
    }

    /// Check if this is a vertical toolbar area (left or right).
    pub fn is_vertical(&self) -> bool {
        matches!(self, ToolBarArea::Left | ToolBarArea::Right)
    }

    /// Get the natural orientation for toolbars in this area.
    pub fn orientation(&self) -> Orientation {
        if self.is_horizontal() {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        }
    }
}

// ============================================================================
// ToolBarAreas (bitflags)
// ============================================================================

/// A set of toolbar areas represented as bit flags.
///
/// # Example
///
/// ```ignore
/// let areas = ToolBarAreas::TOP | ToolBarAreas::BOTTOM;
/// assert!(areas.contains(ToolBarArea::Top));
/// assert!(!areas.contains(ToolBarArea::Left));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolBarAreas(u8);

impl ToolBarAreas {
    /// No toolbar areas.
    pub const NONE: ToolBarAreas = ToolBarAreas(0);
    /// Top toolbar area.
    pub const TOP: ToolBarAreas = ToolBarAreas(1 << 0);
    /// Bottom toolbar area.
    pub const BOTTOM: ToolBarAreas = ToolBarAreas(1 << 1);
    /// Left toolbar area.
    pub const LEFT: ToolBarAreas = ToolBarAreas(1 << 2);
    /// Right toolbar area.
    pub const RIGHT: ToolBarAreas = ToolBarAreas(1 << 3);
    /// All toolbar areas.
    pub const ALL: ToolBarAreas = ToolBarAreas(0b1111);

    /// Create from a single toolbar area.
    pub fn from_area(area: ToolBarArea) -> Self {
        match area {
            ToolBarArea::Top => Self::TOP,
            ToolBarArea::Bottom => Self::BOTTOM,
            ToolBarArea::Left => Self::LEFT,
            ToolBarArea::Right => Self::RIGHT,
        }
    }

    /// Check if this set contains the specified area.
    pub fn contains(&self, area: ToolBarArea) -> bool {
        let area_flag = Self::from_area(area);
        (self.0 & area_flag.0) != 0
    }

    /// Check if this set contains the specified areas.
    pub fn contains_areas(&self, areas: ToolBarAreas) -> bool {
        (self.0 & areas.0) == areas.0
    }

    /// Check if this set is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get all areas as an iterator.
    pub fn iter(&self) -> impl Iterator<Item = ToolBarArea> + '_ {
        ToolBarArea::all().filter(|&area| self.contains(area))
    }
}

impl BitOr for ToolBarAreas {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        ToolBarAreas(self.0 | rhs.0)
    }
}

impl BitOrAssign for ToolBarAreas {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for ToolBarAreas {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        ToolBarAreas(self.0 & rhs.0)
    }
}

impl BitOr<ToolBarArea> for ToolBarAreas {
    type Output = Self;

    fn bitor(self, rhs: ToolBarArea) -> Self::Output {
        self | Self::from_area(rhs)
    }
}

impl BitOr<ToolBarAreas> for ToolBarArea {
    type Output = ToolBarAreas;

    fn bitor(self, rhs: ToolBarAreas) -> Self::Output {
        ToolBarAreas::from_area(self) | rhs
    }
}

impl BitOr for ToolBarArea {
    type Output = ToolBarAreas;

    fn bitor(self, rhs: Self) -> Self::Output {
        ToolBarAreas::from_area(self) | ToolBarAreas::from_area(rhs)
    }
}

// ============================================================================
// ToolBarFeatures (bitflags)
// ============================================================================

/// Feature flags for toolbars.
///
/// These flags control what operations the user can perform on a toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolBarFeatures(u8);

impl ToolBarFeatures {
    /// No features enabled.
    pub const NONE: ToolBarFeatures = ToolBarFeatures(0);
    /// The toolbar can be moved between areas.
    pub const MOVABLE: ToolBarFeatures = ToolBarFeatures(1 << 0);
    /// The toolbar can float as an independent panel.
    pub const FLOATABLE: ToolBarFeatures = ToolBarFeatures(1 << 1);

    /// All standard features (movable, floatable).
    pub fn all() -> Self {
        Self::MOVABLE | Self::FLOATABLE
    }

    /// Check if a feature is enabled.
    pub fn has(&self, feature: ToolBarFeatures) -> bool {
        (self.0 & feature.0) == feature.0
    }

    /// Check if movable.
    pub fn is_movable(&self) -> bool {
        self.has(Self::MOVABLE)
    }

    /// Check if floatable.
    pub fn is_floatable(&self) -> bool {
        self.has(Self::FLOATABLE)
    }
}

impl BitOr for ToolBarFeatures {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        ToolBarFeatures(self.0 | rhs.0)
    }
}

impl BitOrAssign for ToolBarFeatures {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for ToolBarFeatures {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        ToolBarFeatures(self.0 & rhs.0)
    }
}

// ============================================================================
// ToolBarItem
// ============================================================================

/// An item in a toolbar.
///
/// Toolbar items can be actions (buttons), separators, or custom widgets.
#[derive(Clone)]
pub enum ToolBarItem {
    /// A toolbar button associated with an action.
    Action(Arc<Action>),
    /// A visual separator line.
    Separator,
    /// A custom widget embedded in the toolbar.
    Widget(ObjectId),
}

impl ToolBarItem {
    /// Check if this item is an action.
    pub fn is_action(&self) -> bool {
        matches!(self, ToolBarItem::Action(_))
    }

    /// Check if this item is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, ToolBarItem::Separator)
    }

    /// Check if this item is a custom widget.
    pub fn is_widget(&self) -> bool {
        matches!(self, ToolBarItem::Widget(_))
    }

    /// Get the action, if this is an action item.
    pub fn action(&self) -> Option<&Arc<Action>> {
        match self {
            ToolBarItem::Action(action) => Some(action),
            _ => None,
        }
    }

    /// Get the widget ID, if this is a widget item.
    pub fn widget_id(&self) -> Option<ObjectId> {
        match self {
            ToolBarItem::Widget(id) => Some(*id),
            _ => None,
        }
    }

    /// Check if this item is visible.
    pub fn is_visible(&self) -> bool {
        match self {
            ToolBarItem::Action(action) => action.is_visible(),
            ToolBarItem::Separator => true,
            ToolBarItem::Widget(_) => true, // Widget visibility handled separately
        }
    }
}

// ============================================================================
// ToolBarStyle
// ============================================================================

/// Style configuration for toolbar appearance.
#[derive(Clone)]
pub struct ToolBarStyle {
    /// Background color.
    pub background_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Handle color (for movable toolbars).
    pub handle_color: Color,
    /// Handle hover color.
    pub handle_hover_color: Color,
    /// Separator color.
    pub separator_color: Color,
    /// Icon size for action buttons.
    pub icon_size: Size,
    /// Button style for action buttons.
    pub button_style: ToolButtonStyle,
    /// Spacing between items.
    pub spacing: f32,
    /// Padding around all items.
    pub padding: f32,
    /// Handle width for movable toolbars.
    pub handle_width: f32,
    /// Border width.
    pub border_width: f32,
    /// Overflow button width.
    pub overflow_button_width: f32,
    /// Font for text labels.
    pub font: Font,
}

impl Default for ToolBarStyle {
    fn default() -> Self {
        Self {
            background_color: Color::from_rgb8(245, 245, 245),
            border_color: Color::from_rgb8(200, 200, 200),
            handle_color: Color::from_rgb8(180, 180, 180),
            handle_hover_color: Color::from_rgb8(140, 140, 140),
            separator_color: Color::from_rgb8(200, 200, 200),
            icon_size: Size::new(24.0, 24.0),
            button_style: ToolButtonStyle::IconOnly,
            spacing: 2.0,
            padding: 4.0,
            handle_width: 10.0,
            border_width: 1.0,
            overflow_button_width: 16.0,
            font: Font::new(FontFamily::SansSerif, 12.0),
        }
    }
}

// ============================================================================
// Internal: ActionButton
// ============================================================================

/// Internal state for an action button in the toolbar.
struct ActionButton {
    /// The action this button represents.
    action: Arc<Action>,
    /// The button's rectangle (calculated during layout).
    rect: Rect,
    /// Whether the button is hovered.
    hovered: bool,
    /// Whether the button is pressed.
    pressed: bool,
    /// Whether this button is in the overflow menu (not displayed directly).
    in_overflow: bool,
}

impl ActionButton {
    fn new(action: Arc<Action>) -> Self {
        Self {
            action,
            rect: Rect::ZERO,
            hovered: false,
            pressed: false,
            in_overflow: false,
        }
    }
}

// ============================================================================
// ToolBar
// ============================================================================

/// A toolbar widget for action buttons and widgets.
///
/// ToolBar provides a container for quick-access action buttons, typically
/// displayed at the top or side of an application window. It supports:
///
/// - Action buttons with icons and optional text
/// - Separators for visual grouping
/// - Custom widgets embedded in the toolbar
/// - Horizontal and vertical orientations
/// - Movable and floatable features
/// - Overflow menu for actions that don't fit
///
/// # Signals
///
/// - [`action_triggered`](ToolBar::action_triggered): Emitted when an action is triggered
/// - [`orientation_changed`](ToolBar::orientation_changed): Emitted when orientation changes
/// - [`icon_size_changed`](ToolBar::icon_size_changed): Emitted when icon size changes
/// - [`tool_button_style_changed`](ToolBar::tool_button_style_changed): Emitted when button style changes
/// - [`movable_changed`](ToolBar::movable_changed): Emitted when movable state changes
/// - [`top_level_changed`](ToolBar::top_level_changed): Emitted when floating state changes
/// - [`allowed_areas_changed`](ToolBar::allowed_areas_changed): Emitted when allowed areas change
pub struct ToolBar {
    /// Widget base.
    base: WidgetBase,

    /// Toolbar title (for identification and floating window title).
    title: String,

    /// Items in the toolbar.
    items: Vec<ToolBarItem>,

    /// Action buttons with their state.
    action_buttons: Vec<ActionButton>,

    /// Custom widget sizes (for proper overflow calculation).
    widget_sizes: HashMap<ObjectId, Size>,

    /// Toolbar orientation.
    orientation: Orientation,

    /// Which areas this toolbar can be docked in.
    allowed_areas: ToolBarAreas,

    /// Feature flags.
    features: ToolBarFeatures,

    /// Whether the toolbar is currently floating.
    floating: bool,

    /// Custom icon size override (None = use style default).
    icon_size_override: Option<Size>,

    /// Custom button style override (None = use style default).
    button_style_override: Option<ToolButtonStyle>,

    /// Visual style.
    style: ToolBarStyle,

    // Overflow handling
    /// Index of first item in overflow (items after this go to overflow menu).
    overflow_start_index: Option<usize>,
    /// Whether we need an overflow button.
    needs_overflow: bool,
    /// Overflow menu popup.
    overflow_menu: Option<Menu>,
    /// Whether overflow button is hovered.
    overflow_hovered: bool,
    /// Whether overflow button is pressed.
    overflow_pressed: bool,

    // Drag state (for movable toolbars)
    /// Whether dragging the handle.
    dragging: bool,
    /// Drag start position (in global coordinates).
    drag_start: Point,
    /// Toolbar position at drag start.
    drag_start_pos: Point,
    /// Whether handle is hovered.
    handle_hovered: bool,

    // Floating state
    /// Position when floating.
    float_position: Point,

    // Signals
    /// Signal emitted when an action is triggered.
    pub action_triggered: Signal<Arc<Action>>,
    /// Signal emitted when orientation changes.
    pub orientation_changed: Signal<Orientation>,
    /// Signal emitted when icon size changes.
    pub icon_size_changed: Signal<Size>,
    /// Signal emitted when button style changes.
    pub tool_button_style_changed: Signal<ToolButtonStyle>,
    /// Signal emitted when movable state changes.
    pub movable_changed: Signal<bool>,
    /// Signal emitted when floating state changes.
    pub top_level_changed: Signal<bool>,
    /// Signal emitted when allowed areas change.
    pub allowed_areas_changed: Signal<ToolBarAreas>,
}

impl ToolBar {
    /// Create a new toolbar with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed));

        Self {
            base,
            title: title.into(),
            items: Vec::new(),
            action_buttons: Vec::new(),
            widget_sizes: HashMap::new(),
            orientation: Orientation::Horizontal,
            allowed_areas: ToolBarAreas::ALL,
            features: ToolBarFeatures::all(),
            floating: false,
            icon_size_override: None,
            button_style_override: None,
            style: ToolBarStyle::default(),
            overflow_start_index: None,
            needs_overflow: false,
            overflow_menu: None,
            overflow_hovered: false,
            overflow_pressed: false,
            dragging: false,
            drag_start: Point::ZERO,
            drag_start_pos: Point::ZERO,
            handle_hovered: false,
            float_position: Point::ZERO,
            action_triggered: Signal::new(),
            orientation_changed: Signal::new(),
            icon_size_changed: Signal::new(),
            tool_button_style_changed: Signal::new(),
            movable_changed: Signal::new(),
            top_level_changed: Signal::new(),
            allowed_areas_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the toolbar title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the toolbar title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.base.update();
    }

    /// Set title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    // =========================================================================
    // Items - Add
    // =========================================================================

    /// Add an action to the toolbar.
    ///
    /// Creates a tool button for the action and adds it to the end.
    pub fn add_action(&mut self, action: Arc<Action>) {
        let button = ActionButton::new(action.clone());
        self.action_buttons.push(button);
        self.items.push(ToolBarItem::Action(action));
        self.update_layout();
    }

    /// Add a separator to the toolbar.
    pub fn add_separator(&mut self) {
        self.items.push(ToolBarItem::Separator);
        self.update_layout();
    }

    /// Add a custom widget to the toolbar.
    ///
    /// Uses a default size of 50x50 pixels for overflow calculation.
    /// Use [`add_widget_with_size`](Self::add_widget_with_size) to specify custom dimensions.
    pub fn add_widget(&mut self, widget_id: ObjectId) {
        self.add_widget_with_size(widget_id, Size::new(50.0, 50.0));
    }

    /// Add a custom widget to the toolbar with an explicit size.
    ///
    /// The size is used for overflow calculation to determine when widgets
    /// should be moved to the overflow menu. Use the widget's preferred size
    /// from its `size_hint()` for accurate layout.
    ///
    /// # Arguments
    ///
    /// * `widget_id` - The ObjectId of the widget to add
    /// * `size` - The preferred size of the widget for layout purposes
    pub fn add_widget_with_size(&mut self, widget_id: ObjectId, size: Size) {
        self.widget_sizes.insert(widget_id, size);
        self.items.push(ToolBarItem::Widget(widget_id));
        self.update_layout();
    }

    /// Set the size of a widget already in the toolbar.
    ///
    /// Use this to update the size used for overflow calculation after
    /// the widget's size has changed.
    pub fn set_widget_size(&mut self, widget_id: ObjectId, size: Size) {
        self.widget_sizes.insert(widget_id, size);
        self.update_layout();
    }

    /// Get the stored size of a widget in the toolbar.
    ///
    /// Returns the size used for overflow calculation, or `None` if the
    /// widget is not in this toolbar.
    pub fn widget_size(&self, widget_id: ObjectId) -> Option<Size> {
        self.widget_sizes.get(&widget_id).copied()
    }

    // =========================================================================
    // Items - Insert
    // =========================================================================

    /// Insert an action at a specific index.
    pub fn insert_action(&mut self, index: usize, action: Arc<Action>) {
        let index = index.min(self.items.len());
        let button = ActionButton::new(action.clone());

        // Find insert position in action_buttons
        let button_index = self.items[..index]
            .iter()
            .filter(|item| item.is_action())
            .count();

        self.action_buttons.insert(button_index, button);
        self.items.insert(index, ToolBarItem::Action(action));
        self.update_layout();
    }

    /// Insert a separator at a specific index.
    pub fn insert_separator(&mut self, index: usize) {
        let index = index.min(self.items.len());
        self.items.insert(index, ToolBarItem::Separator);
        self.update_layout();
    }

    /// Insert a widget at a specific index.
    ///
    /// Uses a default size of 50x50 pixels for overflow calculation.
    /// Use [`insert_widget_with_size`](Self::insert_widget_with_size) to specify custom dimensions.
    pub fn insert_widget(&mut self, index: usize, widget_id: ObjectId) {
        self.insert_widget_with_size(index, widget_id, Size::new(50.0, 50.0));
    }

    /// Insert a widget at a specific index with an explicit size.
    ///
    /// The size is used for overflow calculation to determine when widgets
    /// should be moved to the overflow menu.
    pub fn insert_widget_with_size(&mut self, index: usize, widget_id: ObjectId, size: Size) {
        let index = index.min(self.items.len());
        self.widget_sizes.insert(widget_id, size);
        self.items.insert(index, ToolBarItem::Widget(widget_id));
        self.update_layout();
    }

    // =========================================================================
    // Items - Remove
    // =========================================================================

    /// Remove an action from the toolbar.
    pub fn remove_action(&mut self, action: &Arc<Action>) {
        let action_id = action.object_id();

        // Remove from items
        self.items.retain(|item| {
            if let ToolBarItem::Action(a) = item {
                a.object_id() != action_id
            } else {
                true
            }
        });

        // Remove from action_buttons
        self.action_buttons.retain(|btn| btn.action.object_id() != action_id);

        self.update_layout();
    }

    /// Clear all items from the toolbar.
    pub fn clear(&mut self) {
        self.items.clear();
        self.action_buttons.clear();
        self.widget_sizes.clear();
        self.update_layout();
    }

    /// Remove a widget from the toolbar.
    pub fn remove_widget(&mut self, widget_id: ObjectId) {
        self.items.retain(|item| {
            if let ToolBarItem::Widget(id) = item {
                *id != widget_id
            } else {
                true
            }
        });
        self.widget_sizes.remove(&widget_id);
        self.update_layout();
    }

    // =========================================================================
    // Items - Query
    // =========================================================================

    /// Get the items in the toolbar.
    pub fn items(&self) -> &[ToolBarItem] {
        &self.items
    }

    /// Get the number of items in the toolbar.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Check if the toolbar is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // =========================================================================
    // Orientation
    // =========================================================================

    /// Get the toolbar orientation.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the toolbar orientation.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;

            // Update size policy based on orientation
            let policy = match orientation {
                Orientation::Horizontal => {
                    SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed)
                }
                Orientation::Vertical => {
                    SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Preferred)
                }
            };
            self.base.set_size_policy(policy);

            self.orientation_changed.emit(orientation);
            self.update_layout();
        }
    }

    /// Set orientation using builder pattern.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.set_orientation(orientation);
        self
    }

    // =========================================================================
    // Icon Size
    // =========================================================================

    /// Get the icon size.
    pub fn icon_size(&self) -> Size {
        self.icon_size_override.unwrap_or(self.style.icon_size)
    }

    /// Set the icon size.
    pub fn set_icon_size(&mut self, size: Size) {
        if self.icon_size_override != Some(size) {
            self.icon_size_override = Some(size);
            self.icon_size_changed.emit(size);
            self.update_layout();
        }
    }

    /// Set icon size using builder pattern.
    pub fn with_icon_size(mut self, size: Size) -> Self {
        self.icon_size_override = Some(size);
        self
    }

    // =========================================================================
    // Button Style
    // =========================================================================

    /// Get the tool button style.
    pub fn tool_button_style(&self) -> ToolButtonStyle {
        self.button_style_override.unwrap_or(self.style.button_style)
    }

    /// Set the tool button style.
    pub fn set_tool_button_style(&mut self, style: ToolButtonStyle) {
        if self.button_style_override != Some(style) {
            self.button_style_override = Some(style);
            self.tool_button_style_changed.emit(style);
            self.update_layout();
        }
    }

    /// Set button style using builder pattern.
    pub fn with_tool_button_style(mut self, style: ToolButtonStyle) -> Self {
        self.button_style_override = Some(style);
        self
    }

    // =========================================================================
    // Allowed Areas
    // =========================================================================

    /// Get the allowed toolbar areas.
    pub fn allowed_areas(&self) -> ToolBarAreas {
        self.allowed_areas
    }

    /// Set the allowed toolbar areas.
    pub fn set_allowed_areas(&mut self, areas: ToolBarAreas) {
        if self.allowed_areas != areas {
            self.allowed_areas = areas;
            self.allowed_areas_changed.emit(areas);
        }
    }

    /// Set allowed areas using builder pattern.
    pub fn with_allowed_areas(mut self, areas: ToolBarAreas) -> Self {
        self.allowed_areas = areas;
        self
    }

    /// Check if a specific area is allowed.
    pub fn is_area_allowed(&self, area: ToolBarArea) -> bool {
        self.allowed_areas.contains(area)
    }

    // =========================================================================
    // Features
    // =========================================================================

    /// Get the enabled features.
    pub fn features(&self) -> ToolBarFeatures {
        self.features
    }

    /// Set the enabled features.
    pub fn set_features(&mut self, features: ToolBarFeatures) {
        if self.features != features {
            let was_movable = self.features.is_movable();
            self.features = features;

            if was_movable != features.is_movable() {
                self.movable_changed.emit(features.is_movable());
            }

            self.update_layout();
        }
    }

    /// Set features using builder pattern.
    pub fn with_features(mut self, features: ToolBarFeatures) -> Self {
        self.features = features;
        self
    }

    /// Check if the toolbar is movable.
    pub fn is_movable(&self) -> bool {
        self.features.is_movable()
    }

    /// Set whether the toolbar is movable.
    pub fn set_movable(&mut self, movable: bool) {
        let mut features = self.features;
        if movable {
            features |= ToolBarFeatures::MOVABLE;
        } else {
            features = ToolBarFeatures(features.0 & !ToolBarFeatures::MOVABLE.0);
        }
        self.set_features(features);
    }

    /// Check if the toolbar is floatable.
    pub fn is_floatable(&self) -> bool {
        self.features.is_floatable()
    }

    /// Set whether the toolbar is floatable.
    pub fn set_floatable(&mut self, floatable: bool) {
        let mut features = self.features;
        if floatable {
            features |= ToolBarFeatures::FLOATABLE;
        } else {
            features = ToolBarFeatures(features.0 & !ToolBarFeatures::FLOATABLE.0);
        }
        self.set_features(features);
    }

    // =========================================================================
    // Floating
    // =========================================================================

    /// Check if the toolbar is floating.
    pub fn is_floating(&self) -> bool {
        self.floating
    }

    /// Set the floating state.
    pub fn set_floating(&mut self, floating: bool) {
        if !self.features.is_floatable() && floating {
            return;
        }

        if self.floating != floating {
            self.floating = floating;
            if floating {
                // Save current position as float position
                if self.float_position == Point::ZERO {
                    self.float_position = self.base.pos();
                }
            }
            self.top_level_changed.emit(floating);
            self.base.update();
        }
    }

    /// Get the floating position.
    pub fn float_position(&self) -> Point {
        self.float_position
    }

    /// Set the floating position.
    pub fn set_float_position(&mut self, position: Point) {
        self.float_position = position;
        if self.floating {
            self.base.set_pos(position);
        }
    }

    // =========================================================================
    // Style
    // =========================================================================

    /// Get the toolbar style.
    pub fn style(&self) -> &ToolBarStyle {
        &self.style
    }

    /// Set the toolbar style.
    pub fn set_style(&mut self, style: ToolBarStyle) {
        self.style = style;
        self.update_layout();
    }

    /// Set style using builder pattern.
    pub fn with_style(mut self, style: ToolBarStyle) -> Self {
        self.style = style;
        self
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    /// Recalculate the layout of items.
    fn update_layout(&mut self) {
        self.calculate_overflow();
        self.base.update();
    }

    /// Calculate which items overflow and update button rects.
    fn calculate_overflow(&mut self) {
        let rect = self.base.rect();
        let is_horizontal = self.orientation == Orientation::Horizontal;

        // Calculate available space
        let handle_space = if self.features.is_movable() {
            self.style.handle_width + self.style.spacing
        } else {
            0.0
        };

        let available = if is_horizontal {
            rect.width() - self.style.padding * 2.0 - handle_space
        } else {
            rect.height() - self.style.padding * 2.0 - handle_space
        };

        // Calculate size of each item
        let button_size = self.calculate_button_size();
        let separator_size = if is_horizontal { 8.0 } else { 8.0 };

        // Position items
        let start_pos = if is_horizontal {
            self.style.padding + handle_space
        } else {
            self.style.padding + handle_space
        };

        let mut pos = start_pos;
        let mut button_idx = 0;
        let mut first_overflow_index = None;

        // First pass: calculate positions and determine overflow
        for (i, item) in self.items.iter().enumerate() {
            if !item.is_visible() {
                continue;
            }

            let item_size = match item {
                ToolBarItem::Action(_) => {
                    if is_horizontal { button_size.width } else { button_size.height }
                }
                ToolBarItem::Separator => separator_size,
                ToolBarItem::Widget(widget_id) => {
                    // Use stored widget size, with fallback to default
                    let size = self.widget_sizes.get(widget_id).copied()
                        .unwrap_or(Size::new(50.0, 50.0));
                    if is_horizontal { size.width } else { size.height }
                }
            };

            // Check if this item would overflow
            let overflow_button_space = if first_overflow_index.is_none() {
                0.0
            } else {
                self.style.overflow_button_width + self.style.spacing
            };

            if pos + item_size + overflow_button_space > available && first_overflow_index.is_none() {
                // We need overflow, recalculate with overflow button space
                let available_with_overflow = available - self.style.overflow_button_width - self.style.spacing;
                if pos > available_with_overflow {
                    first_overflow_index = Some(i);
                }
            }

            // Update button rect if this is an action
            if let ToolBarItem::Action(_) = item {
                if button_idx < self.action_buttons.len() {
                    let btn = &mut self.action_buttons[button_idx];
                    btn.in_overflow = first_overflow_index.is_some_and(|fi| i >= fi);

                    if !btn.in_overflow {
                        if is_horizontal {
                            btn.rect = Rect::new(
                                pos,
                                self.style.padding,
                                button_size.width,
                                button_size.height,
                            );
                        } else {
                            btn.rect = Rect::new(
                                self.style.padding,
                                pos,
                                button_size.width,
                                button_size.height,
                            );
                        }
                    }
                    button_idx += 1;
                }
            }

            if first_overflow_index.is_none() || first_overflow_index.is_some_and(|fi| i < fi) {
                pos += item_size + self.style.spacing;
            }
        }

        self.overflow_start_index = first_overflow_index;
        self.needs_overflow = first_overflow_index.is_some();
    }

    /// Calculate the size of a single button based on style.
    fn calculate_button_size(&self) -> Size {
        let icon_size = self.icon_size();
        let style = self.tool_button_style();
        let padding = 8.0; // Button internal padding

        match style {
            ToolButtonStyle::IconOnly => {
                Size::new(icon_size.width + padding, icon_size.height + padding)
            }
            ToolButtonStyle::TextOnly => {
                // Estimate text size
                Size::new(60.0, 24.0)
            }
            ToolButtonStyle::TextBesideIcon => {
                Size::new(icon_size.width + 60.0 + padding, icon_size.height.max(20.0) + padding)
            }
            ToolButtonStyle::TextUnderIcon => {
                Size::new(icon_size.width.max(50.0) + padding, icon_size.height + 20.0 + padding)
            }
        }
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the handle rectangle (for movable toolbars).
    fn handle_rect(&self) -> Option<Rect> {
        if !self.features.is_movable() {
            return None;
        }

        let rect = self.base.rect();
        let is_horizontal = self.orientation == Orientation::Horizontal;

        if is_horizontal {
            Some(Rect::new(
                self.style.padding,
                self.style.padding,
                self.style.handle_width,
                rect.height() - self.style.padding * 2.0,
            ))
        } else {
            Some(Rect::new(
                self.style.padding,
                self.style.padding,
                rect.width() - self.style.padding * 2.0,
                self.style.handle_width,
            ))
        }
    }

    /// Get the overflow button rectangle.
    fn overflow_rect(&self) -> Option<Rect> {
        if !self.needs_overflow {
            return None;
        }

        let rect = self.base.rect();
        let is_horizontal = self.orientation == Orientation::Horizontal;
        let button_size = self.calculate_button_size();

        if is_horizontal {
            Some(Rect::new(
                rect.width() - self.style.padding - self.style.overflow_button_width,
                self.style.padding,
                self.style.overflow_button_width,
                button_size.height,
            ))
        } else {
            Some(Rect::new(
                self.style.padding,
                rect.height() - self.style.padding - self.style.overflow_button_width,
                button_size.width,
                self.style.overflow_button_width,
            ))
        }
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    /// Find which action button is at the given position.
    fn hit_test_button(&self, pos: Point) -> Option<usize> {
        for (i, btn) in self.action_buttons.iter().enumerate() {
            if !btn.in_overflow && btn.rect.contains(pos) {
                return Some(i);
            }
        }
        None
    }

    /// Check if the position is in the handle area.
    fn is_in_handle(&self, pos: Point) -> bool {
        self.handle_rect().is_some_and(|r| r.contains(pos))
    }

    /// Check if the position is in the overflow button.
    fn is_in_overflow_button(&self, pos: Point) -> bool {
        self.overflow_rect().is_some_and(|r| r.contains(pos))
    }

    // =========================================================================
    // Overflow Menu
    // =========================================================================

    /// Build the overflow menu with overflowed actions.
    fn build_overflow_menu(&mut self) {
        let mut menu = Menu::new();

        if let Some(start) = self.overflow_start_index {
            for item in self.items.iter().skip(start) {
                match item {
                    ToolBarItem::Action(action) => {
                        if action.is_visible() {
                            menu.add_action(action.clone());
                        }
                    }
                    ToolBarItem::Separator => {
                        menu.add_separator();
                    }
                    ToolBarItem::Widget(_) => {
                        // Widgets can't go in overflow menu
                    }
                }
            }
        }

        self.overflow_menu = Some(menu);
    }

    /// Show the overflow menu.
    fn show_overflow_menu(&mut self) {
        self.build_overflow_menu();

        // Calculate the global rect before mutably borrowing the menu
        let global_rect = if let Some(rect) = self.overflow_rect() {
            let base_rect = self.base.rect();
            Some(Rect::new(
                base_rect.origin.x + rect.origin.x,
                base_rect.origin.y + rect.origin.y,
                rect.width(),
                rect.height(),
            ))
        } else {
            None
        };

        if let Some(rect) = global_rect {
            if let Some(menu) = &mut self.overflow_menu {
                menu.popup_relative_to(rect, PopupPlacement::BelowAlignLeft);
            }
        }
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check handle press (for dragging)
        if self.is_in_handle(pos) && (self.features.is_movable() || self.features.is_floatable()) {
            self.dragging = true;
            self.drag_start = event.global_pos;
            self.drag_start_pos = self.base.pos();
            return true;
        }

        // Check overflow button press
        if self.is_in_overflow_button(pos) {
            self.overflow_pressed = true;
            self.base.update();
            return true;
        }

        // Check action button press
        if let Some(idx) = self.hit_test_button(pos) {
            self.action_buttons[idx].pressed = true;
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // End drag
        if self.dragging {
            self.dragging = false;
            return true;
        }

        // Handle overflow button release
        if self.overflow_pressed {
            self.overflow_pressed = false;
            if self.is_in_overflow_button(pos) {
                self.show_overflow_menu();
            }
            self.base.update();
            return true;
        }

        // Handle action button release
        for btn in self.action_buttons.iter_mut() {
            if btn.pressed {
                btn.pressed = false;
                if btn.rect.contains(pos) && btn.action.is_enabled() {
                    btn.action.trigger();
                    self.action_triggered.emit(btn.action.clone());
                }
                self.base.update();
                return true;
            }
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Handle dragging
        if self.dragging {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );

            // If not floating and we've dragged far enough, float the toolbar
            if !self.floating && self.features.is_floatable() {
                let drag_distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
                if drag_distance > 10.0 {
                    self.set_floating(true);
                    self.float_position = self.drag_start_pos;
                }
            }

            if self.floating {
                let new_pos = Point::new(
                    self.drag_start_pos.x + delta.x,
                    self.drag_start_pos.y + delta.y,
                );
                self.float_position = new_pos;
                self.base.set_pos(new_pos);
            }

            return true;
        }

        // Update handle hover state
        let new_handle_hover = self.is_in_handle(pos);
        if self.handle_hovered != new_handle_hover {
            self.handle_hovered = new_handle_hover;
            self.base.update();
        }

        // Update overflow hover state
        let new_overflow_hover = self.is_in_overflow_button(pos);
        if self.overflow_hovered != new_overflow_hover {
            self.overflow_hovered = new_overflow_hover;
            self.base.update();
        }

        // Update button hover states
        let mut any_changed = false;
        for btn in &mut self.action_buttons {
            let was_hovered = btn.hovered;
            btn.hovered = !btn.in_overflow && btn.rect.contains(pos);
            if was_hovered != btn.hovered {
                any_changed = true;
            }
        }

        if any_changed {
            self.base.update();
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        ctx.renderer().fill_rect(rect, self.style.background_color);

        // Draw bottom border
        let stroke = Stroke::new(self.style.border_color, self.style.border_width);
        let is_horizontal = self.orientation == Orientation::Horizontal;

        if is_horizontal {
            ctx.renderer().draw_line(
                Point::new(0.0, rect.height() - self.style.border_width / 2.0),
                Point::new(rect.width(), rect.height() - self.style.border_width / 2.0),
                &stroke,
            );
        } else {
            ctx.renderer().draw_line(
                Point::new(rect.width() - self.style.border_width / 2.0, 0.0),
                Point::new(rect.width() - self.style.border_width / 2.0, rect.height()),
                &stroke,
            );
        }
    }

    fn paint_handle(&self, ctx: &mut PaintContext<'_>) {
        if let Some(rect) = self.handle_rect() {
            let color = if self.dragging {
                self.style.handle_hover_color
            } else if self.handle_hovered {
                self.style.handle_hover_color
            } else {
                self.style.handle_color
            };

            // Draw grip lines
            let is_horizontal = self.orientation == Orientation::Horizontal;
            let num_lines = 3;
            let line_spacing = 3.0;

            if is_horizontal {
                let start_y = rect.origin.y + (rect.height() - (num_lines as f32 - 1.0) * line_spacing) / 2.0;
                for i in 0..num_lines {
                    let y = start_y + i as f32 * line_spacing;
                    let stroke = Stroke::new(color, 1.0);
                    ctx.renderer().draw_line(
                        Point::new(rect.origin.x + 2.0, y),
                        Point::new(rect.origin.x + rect.width() - 2.0, y),
                        &stroke,
                    );
                }
            } else {
                let start_x = rect.origin.x + (rect.width() - (num_lines as f32 - 1.0) * line_spacing) / 2.0;
                for i in 0..num_lines {
                    let x = start_x + i as f32 * line_spacing;
                    let stroke = Stroke::new(color, 1.0);
                    ctx.renderer().draw_line(
                        Point::new(x, rect.origin.y + 2.0),
                        Point::new(x, rect.origin.y + rect.height() - 2.0),
                        &stroke,
                    );
                }
            }
        }
    }

    fn paint_items(&self, ctx: &mut PaintContext<'_>) {
        let button_size = self.calculate_button_size();
        let is_horizontal = self.orientation == Orientation::Horizontal;

        // Paint separators
        let mut pos = if self.features.is_movable() {
            self.style.padding + self.style.handle_width + self.style.spacing
        } else {
            self.style.padding
        };

        let mut button_idx = 0;

        for item in &self.items {
            if !item.is_visible() {
                continue;
            }

            // Check if this item is in overflow
            if let Some(start) = self.overflow_start_index {
                let current_idx = self.items.iter()
                    .position(|i| std::ptr::eq(i, item))
                    .unwrap_or(0);
                if current_idx >= start {
                    break;
                }
            }

            match item {
                ToolBarItem::Separator => {
                    self.paint_separator(ctx, pos);
                    pos += 8.0 + self.style.spacing;
                }
                ToolBarItem::Action(_) => {
                    if button_idx < self.action_buttons.len() {
                        let btn = &self.action_buttons[button_idx];
                        if !btn.in_overflow {
                            self.paint_button(ctx, btn);
                            pos += if is_horizontal { button_size.width } else { button_size.height };
                            pos += self.style.spacing;
                        }
                        button_idx += 1;
                    }
                }
                ToolBarItem::Widget(widget_id) => {
                    // Widget painting is handled by the widget itself
                    let size = self.widget_sizes.get(widget_id).copied()
                        .unwrap_or(Size::new(50.0, 50.0));
                    pos += (if is_horizontal { size.width } else { size.height }) + self.style.spacing;
                }
            }
        }
    }

    fn paint_separator(&self, ctx: &mut PaintContext<'_>, pos: f32) {
        let rect = ctx.rect();
        let is_horizontal = self.orientation == Orientation::Horizontal;
        let stroke = Stroke::new(self.style.separator_color, 1.0);

        if is_horizontal {
            let x = pos + 4.0;
            ctx.renderer().draw_line(
                Point::new(x, self.style.padding + 4.0),
                Point::new(x, rect.height() - self.style.padding - 4.0),
                &stroke,
            );
        } else {
            let y = pos + 4.0;
            ctx.renderer().draw_line(
                Point::new(self.style.padding + 4.0, y),
                Point::new(rect.width() - self.style.padding - 4.0, y),
                &stroke,
            );
        }
    }

    fn paint_button(&self, ctx: &mut PaintContext<'_>, btn: &ActionButton) {
        let is_disabled = !btn.action.is_enabled();
        let is_pressed = btn.pressed;
        let is_hovered = btn.hovered;
        let is_checked = btn.action.is_checkable() && btn.action.is_checked();

        // Button background
        let bg_color = if is_disabled {
            Color::TRANSPARENT
        } else if is_pressed {
            Color::from_rgba8(0, 122, 255, 51)
        } else if is_checked {
            Color::from_rgba8(0, 122, 255, 38)
        } else if is_hovered {
            Color::from_rgba8(0, 122, 255, 26)
        } else {
            Color::TRANSPARENT
        };

        if bg_color.a > 0.0 {
            ctx.renderer().fill_rect(btn.rect, bg_color);
        }

        // Draw border when hovered or checked
        if (is_hovered || is_checked) && !is_disabled {
            let border_color = Color::from_rgb8(200, 200, 200);
            let stroke = Stroke::new(border_color, 1.0);
            ctx.renderer().stroke_rect(btn.rect, &stroke);
        }

        // Draw icon placeholder (actual icon drawing would use ImageRenderer)
        if let Some(_icon) = btn.action.icon() {
            let icon_size = self.icon_size();
            let icon_x = btn.rect.origin.x + (btn.rect.width() - icon_size.width) / 2.0;
            let icon_y = btn.rect.origin.y + (btn.rect.height() - icon_size.height) / 2.0;
            let icon_rect = Rect::new(icon_x, icon_y, icon_size.width, icon_size.height);

            let icon_color = if is_disabled {
                Color::from_rgb8(180, 180, 180)
            } else {
                Color::from_rgb8(80, 80, 80)
            };

            // Draw placeholder for icon
            ctx.renderer().fill_rect(icon_rect, icon_color);
        }

        // Draw text if style includes text
        let style = self.tool_button_style();
        if matches!(style, ToolButtonStyle::TextOnly | ToolButtonStyle::TextBesideIcon | ToolButtonStyle::TextUnderIcon) {
            let text = btn.action.display_text();
            let text_color = if is_disabled {
                Color::from_rgb8(160, 160, 160)
            } else {
                Color::from_rgb8(40, 40, 40)
            };

            // Simple text position (would need TextLayout for proper positioning)
            let text_x = btn.rect.origin.x + 4.0;
            let text_y = btn.rect.origin.y + btn.rect.height() / 2.0 - 6.0;

            // Text rendering would use TextRenderer here
            let _ = (text, text_color, text_x, text_y);
        }
    }

    fn paint_overflow_button(&self, ctx: &mut PaintContext<'_>) {
        if let Some(rect) = self.overflow_rect() {
            // Background
            let bg_color = if self.overflow_pressed {
                Color::from_rgba8(0, 122, 255, 51)
            } else if self.overflow_hovered {
                Color::from_rgba8(0, 122, 255, 26)
            } else {
                Color::TRANSPARENT
            };

            if bg_color.a > 0.0 {
                ctx.renderer().fill_rect(rect, bg_color);
            }

            // Draw chevron/arrow indicator
            let arrow_color = Color::from_rgb8(80, 80, 80);
            let center_x = rect.origin.x + rect.width() / 2.0;
            let center_y = rect.origin.y + rect.height() / 2.0;
            let arrow_size = 4.0;

            let is_horizontal = self.orientation == Orientation::Horizontal;

            let stroke = Stroke::new(arrow_color, 1.5);

            if is_horizontal {
                // Draw double chevron pointing down
                ctx.renderer().draw_line(
                    Point::new(center_x - arrow_size, center_y - arrow_size / 2.0),
                    Point::new(center_x, center_y + arrow_size / 2.0),
                    &stroke,
                );
                ctx.renderer().draw_line(
                    Point::new(center_x, center_y + arrow_size / 2.0),
                    Point::new(center_x + arrow_size, center_y - arrow_size / 2.0),
                    &stroke,
                );
            } else {
                // Draw double chevron pointing right
                ctx.renderer().draw_line(
                    Point::new(center_x - arrow_size / 2.0, center_y - arrow_size),
                    Point::new(center_x + arrow_size / 2.0, center_y),
                    &stroke,
                );
                ctx.renderer().draw_line(
                    Point::new(center_x + arrow_size / 2.0, center_y),
                    Point::new(center_x - arrow_size / 2.0, center_y + arrow_size),
                    &stroke,
                );
            }
        }
    }
}

impl Widget for ToolBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let button_size = self.calculate_button_size();
        let is_horizontal = self.orientation == Orientation::Horizontal;

        // Calculate minimum size
        let handle_space = if self.features.is_movable() {
            self.style.handle_width + self.style.spacing
        } else {
            0.0
        };

        if is_horizontal {
            let height = button_size.height + self.style.padding * 2.0 + self.style.border_width;
            let width = handle_space + self.style.padding * 2.0 + 50.0; // Minimum width
            SizeHint::new(Size::new(width, height)).with_minimum(Size::new(50.0, height))
        } else {
            let width = button_size.width + self.style.padding * 2.0 + self.style.border_width;
            let height = handle_space + self.style.padding * 2.0 + 50.0;
            SizeHint::new(Size::new(width, height)).with_minimum(Size::new(width, 50.0))
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_handle(ctx);
        self.paint_items(ctx);
        self.paint_overflow_button(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::Leave(_) => {
                // Clear hover states
                let mut changed = false;
                if self.handle_hovered {
                    self.handle_hovered = false;
                    changed = true;
                }
                if self.overflow_hovered {
                    self.overflow_hovered = false;
                    changed = true;
                }
                for btn in &mut self.action_buttons {
                    if btn.hovered {
                        btn.hovered = false;
                        changed = true;
                    }
                }
                if changed {
                    self.base.update();
                }
                false
            }
            WidgetEvent::Resize(_) => {
                self.calculate_overflow();
                false
            }
            _ => false,
        }
    }
}

impl Object for ToolBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for ToolBar {
    fn default() -> Self {
        Self::new("ToolBar")
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
    fn test_toolbar_areas_bitflags() {
        let areas = ToolBarAreas::TOP | ToolBarAreas::BOTTOM;
        assert!(areas.contains(ToolBarArea::Top));
        assert!(areas.contains(ToolBarArea::Bottom));
        assert!(!areas.contains(ToolBarArea::Left));
        assert!(!areas.contains(ToolBarArea::Right));
    }

    #[test]
    fn test_toolbar_area_or() {
        let areas = ToolBarArea::Top | ToolBarArea::Left;
        assert!(areas.contains(ToolBarArea::Top));
        assert!(areas.contains(ToolBarArea::Left));
        assert!(!areas.contains(ToolBarArea::Right));
    }

    #[test]
    fn test_toolbar_features() {
        let features = ToolBarFeatures::MOVABLE | ToolBarFeatures::FLOATABLE;
        assert!(features.is_movable());
        assert!(features.is_floatable());
    }

    #[test]
    fn test_toolbar_features_all() {
        let features = ToolBarFeatures::all();
        assert!(features.is_movable());
        assert!(features.is_floatable());
    }

    #[test]
    fn test_toolbar_new() {
        setup();
        let toolbar = ToolBar::new("Test");
        assert_eq!(toolbar.title(), "Test");
        assert!(toolbar.is_empty());
        assert_eq!(toolbar.item_count(), 0);
        assert_eq!(toolbar.orientation(), Orientation::Horizontal);
    }

    #[test]
    fn test_toolbar_add_action() {
        setup();
        let mut toolbar = ToolBar::new("Test");
        let action = Arc::new(Action::new("&Open"));

        toolbar.add_action(action.clone());

        assert_eq!(toolbar.item_count(), 1);
        assert!(!toolbar.is_empty());
    }

    #[test]
    fn test_toolbar_add_separator() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        toolbar.add_separator();

        assert_eq!(toolbar.item_count(), 1);
        assert!(toolbar.items()[0].is_separator());
    }

    #[test]
    fn test_toolbar_orientation() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        assert_eq!(toolbar.orientation(), Orientation::Horizontal);

        toolbar.set_orientation(Orientation::Vertical);
        assert_eq!(toolbar.orientation(), Orientation::Vertical);
    }

    #[test]
    fn test_toolbar_icon_size() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        let default_size = toolbar.icon_size();
        assert_eq!(default_size, Size::new(24.0, 24.0));

        toolbar.set_icon_size(Size::new(32.0, 32.0));
        assert_eq!(toolbar.icon_size(), Size::new(32.0, 32.0));
    }

    #[test]
    fn test_toolbar_features_movable() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        assert!(toolbar.is_movable());

        toolbar.set_movable(false);
        assert!(!toolbar.is_movable());
    }

    #[test]
    fn test_toolbar_features_floatable() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        assert!(toolbar.is_floatable());

        toolbar.set_floatable(false);
        assert!(!toolbar.is_floatable());
    }

    #[test]
    fn test_toolbar_allowed_areas() {
        setup();
        let toolbar = ToolBar::new("Test")
            .with_allowed_areas(ToolBarAreas::TOP | ToolBarAreas::BOTTOM);

        assert!(toolbar.is_area_allowed(ToolBarArea::Top));
        assert!(toolbar.is_area_allowed(ToolBarArea::Bottom));
        assert!(!toolbar.is_area_allowed(ToolBarArea::Left));
        assert!(!toolbar.is_area_allowed(ToolBarArea::Right));
    }

    #[test]
    fn test_toolbar_clear() {
        setup();
        let mut toolbar = ToolBar::new("Test");
        toolbar.add_action(Arc::new(Action::new("Action 1")));
        toolbar.add_separator();
        toolbar.add_action(Arc::new(Action::new("Action 2")));

        assert_eq!(toolbar.item_count(), 3);

        toolbar.clear();

        assert_eq!(toolbar.item_count(), 0);
        assert!(toolbar.is_empty());
    }

    #[test]
    fn test_toolbar_area_orientation() {
        assert_eq!(ToolBarArea::Top.orientation(), Orientation::Horizontal);
        assert_eq!(ToolBarArea::Bottom.orientation(), Orientation::Horizontal);
        assert_eq!(ToolBarArea::Left.orientation(), Orientation::Vertical);
        assert_eq!(ToolBarArea::Right.orientation(), Orientation::Vertical);
    }

    #[test]
    fn test_toolbar_areas_iter() {
        let areas = ToolBarAreas::TOP | ToolBarAreas::RIGHT;
        let collected: Vec<_> = areas.iter().collect();
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&ToolBarArea::Top));
        assert!(collected.contains(&ToolBarArea::Right));
    }

    #[test]
    fn test_toolbar_item_enum() {
        setup();
        let action = Arc::new(Action::new("Test"));

        let item_action = ToolBarItem::Action(action.clone());
        assert!(item_action.is_action());
        assert!(!item_action.is_separator());
        assert!(!item_action.is_widget());

        let item_sep = ToolBarItem::Separator;
        assert!(!item_sep.is_action());
        assert!(item_sep.is_separator());
        assert!(!item_sep.is_widget());
    }

    #[test]
    fn test_toolbar_button_style() {
        setup();
        let mut toolbar = ToolBar::new("Test");

        assert_eq!(toolbar.tool_button_style(), ToolButtonStyle::IconOnly);

        toolbar.set_tool_button_style(ToolButtonStyle::TextBesideIcon);
        assert_eq!(toolbar.tool_button_style(), ToolButtonStyle::TextBesideIcon);
    }
}
