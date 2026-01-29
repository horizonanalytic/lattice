//! MainWindow widget implementation.
//!
//! This module provides [`MainWindow`], the primary application window with
//! support for dock areas, a central widget, menu bar, toolbar, and status bar.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{MainWindow, DockWidget, DockArea, MenuBar, Menu, Action};
//! use std::sync::Arc;
//!
//! // Create a main window
//! let mut main_window = MainWindow::new();
//!
//! // Create and set a menu bar
//! let mut menu_bar = MenuBar::new();
//! let mut file_menu = Menu::new();
//! file_menu.add_action(Arc::new(Action::new("&Open")));
//! menu_bar.add_menu("&File", Arc::new(file_menu));
//! main_window.set_menu_bar(Some(menu_bar));
//!
//! // Set the central widget
//! main_window.set_central_widget(editor_id);
//!
//! // Add dock widgets
//! let properties_dock = DockWidget::new("Properties");
//! main_window.add_dock_widget(DockArea::Right, properties_dock.object_id());
//! ```

use std::collections::HashMap;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer as _, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    FocusPolicy, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext,
    SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

use super::dock_widget::DockArea;
use super::menu_bar::MenuBar;
use super::tool_bar::ToolBarArea;

/// Information about a docked widget.
#[derive(Debug, Clone)]
struct DockedWidget {
    /// The dock widget ID.
    widget_id: ObjectId,
    /// Whether the widget is visible.
    visible: bool,
}

/// A dock area container that manages widgets docked in a single area.
///
/// When multiple widgets are docked in the same area, they can be displayed
/// as tabs or as split panes.
#[derive(Debug)]
struct DockAreaContainer {
    /// The dock area this container represents.
    area: DockArea,
    /// Widgets docked in this area.
    widgets: Vec<DockedWidget>,
    /// Currently active tab index (when using tabbed mode).
    current_index: usize,
    /// Whether to use tabbed mode (true) or split mode (false).
    tabbed: bool,
    /// Allocated size for this dock area.
    size: f32,
    /// Minimum size for this dock area.
    min_size: f32,
    /// Whether this area is collapsed.
    collapsed: bool,
}

impl DockAreaContainer {
    fn new(area: DockArea) -> Self {
        Self {
            area,
            widgets: Vec::new(),
            current_index: 0,
            tabbed: true, // Default to tabbed mode
            size: 200.0,
            min_size: 100.0,
            collapsed: false,
        }
    }

    fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    fn visible_count(&self) -> usize {
        self.widgets.iter().filter(|w| w.visible).count()
    }

    fn add_widget(&mut self, widget_id: ObjectId) {
        self.widgets.push(DockedWidget {
            widget_id,
            visible: true,
        });
    }

    fn remove_widget(&mut self, widget_id: ObjectId) -> bool {
        if let Some(pos) = self.widgets.iter().position(|w| w.widget_id == widget_id) {
            self.widgets.remove(pos);
            if self.current_index >= self.widgets.len() && !self.widgets.is_empty() {
                self.current_index = self.widgets.len() - 1;
            }
            true
        } else {
            false
        }
    }

    fn contains(&self, widget_id: ObjectId) -> bool {
        self.widgets.iter().any(|w| w.widget_id == widget_id)
    }

    fn current_widget(&self) -> Option<ObjectId> {
        if self.tabbed {
            self.widgets
                .iter()
                .filter(|w| w.visible)
                .nth(self.current_index)
                .map(|w| w.widget_id)
        } else {
            // In split mode, all visible widgets are shown
            None
        }
    }
}

// ============================================================================
// ToolbarAreaContainer
// ============================================================================

/// A toolbar area container that manages toolbars in a single area.
#[derive(Debug)]
struct ToolbarAreaContainer {
    /// The toolbar area this container represents.
    area: ToolBarArea,
    /// Toolbars in this area.
    toolbars: Vec<ObjectId>,
    /// Whether there's a break after each toolbar (for multiple rows).
    breaks: Vec<bool>,
    /// Total height/width of this toolbar area (calculated).
    size: f32,
}

impl ToolbarAreaContainer {
    fn new(area: ToolBarArea) -> Self {
        Self {
            area,
            toolbars: Vec::new(),
            breaks: Vec::new(),
            size: 0.0,
        }
    }

    fn is_empty(&self) -> bool {
        self.toolbars.is_empty()
    }

    fn add_toolbar(&mut self, toolbar_id: ObjectId) {
        self.toolbars.push(toolbar_id);
        self.breaks.push(false);
    }

    fn add_toolbar_break(&mut self) {
        if let Some(last_break) = self.breaks.last_mut() {
            *last_break = true;
        }
    }

    fn remove_toolbar(&mut self, toolbar_id: ObjectId) -> bool {
        if let Some(pos) = self.toolbars.iter().position(|&id| id == toolbar_id) {
            self.toolbars.remove(pos);
            self.breaks.remove(pos);
            true
        } else {
            false
        }
    }

    fn contains(&self, toolbar_id: ObjectId) -> bool {
        self.toolbars.contains(&toolbar_id)
    }
}

/// The main application window.
///
/// MainWindow provides the primary window structure for applications, including:
/// - An optional menu bar at the top
/// - A central widget area for the main content
/// - Four dock areas (left, right, top, bottom) for tool panels
/// - Support for floating dock widgets
/// - Resizable dock areas via splitter handles
///
/// # Layout
///
/// The window is divided into regions:
/// ```text
/// +------------------------------------------+
/// |                Menu Bar                  |
/// +------------------------------------------+
/// |              Top Dock Area               |
/// +--------+------------------------+--------+
/// |  Left  |                        | Right  |
/// |  Dock  |     Central Widget     |  Dock  |
/// |  Area  |                        |  Area  |
/// +--------+------------------------+--------+
/// |            Bottom Dock Area              |
/// +------------------------------------------+
/// ```
///
/// # Signals
///
/// - `dock_widget_added(ObjectId)`: Emitted when a dock widget is added
/// - `dock_widget_removed(ObjectId)`: Emitted when a dock widget is removed
pub struct MainWindow {
    /// Widget base.
    base: WidgetBase,

    /// The menu bar widget (optional).
    menu_bar: Option<MenuBar>,

    /// The central widget ID.
    central_widget: Option<ObjectId>,

    /// Dock area containers.
    dock_areas: HashMap<DockArea, DockAreaContainer>,

    /// Floating dock widgets (not in any dock area).
    floating_widgets: Vec<ObjectId>,

    /// Toolbar area containers.
    toolbar_areas: HashMap<ToolBarArea, ToolbarAreaContainer>,

    /// Default toolbar area for new toolbars.
    default_toolbar_area: ToolBarArea,

    /// Toolbar area height (for horizontal areas) or width (for vertical areas).
    toolbar_area_height: f32,

    /// Content margins around the entire layout.
    content_margins: ContentMargins,

    /// Splitter handle width.
    handle_width: f32,

    /// Minimum central widget size.
    min_central_size: Size,

    // Visual styling
    /// Background color.
    background_color: Color,
    /// Splitter handle color.
    handle_color: Color,
    /// Splitter handle hover color.
    handle_hover_color: Color,
    /// Splitter handle pressed color.
    handle_pressed_color: Color,

    // Interaction state
    /// Currently dragging splitter (dock area being resized).
    dragging_splitter: Option<DockArea>,
    /// Drag start position.
    drag_start: f32,
    /// Size at drag start.
    drag_start_size: f32,
    /// Currently hovered splitter.
    hover_splitter: Option<DockArea>,

    // Drag-to-dock preview
    /// Currently dragging dock widget (for repositioning).
    dragging_dock_widget: Option<ObjectId>,
    /// Preview dock area (where widget would dock if released).
    dock_preview_area: Option<DockArea>,

    // Signals
    /// Signal emitted when a dock widget is added.
    pub dock_widget_added: Signal<ObjectId>,
    /// Signal emitted when a dock widget is removed.
    pub dock_widget_removed: Signal<ObjectId>,
}

impl MainWindow {
    /// Create a new main window.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        let mut dock_areas = HashMap::new();
        for area in DockArea::all() {
            dock_areas.insert(area, DockAreaContainer::new(area));
        }

        let mut toolbar_areas = HashMap::new();
        for area in ToolBarArea::all() {
            toolbar_areas.insert(area, ToolbarAreaContainer::new(area));
        }

        Self {
            base,
            menu_bar: None,
            central_widget: None,
            dock_areas,
            floating_widgets: Vec::new(),
            toolbar_areas,
            default_toolbar_area: ToolBarArea::Top,
            toolbar_area_height: 32.0, // Default toolbar height
            content_margins: ContentMargins::uniform(0.0),
            handle_width: 5.0,
            min_central_size: Size::new(100.0, 100.0),
            background_color: Color::from_rgb8(245, 245, 245),
            handle_color: Color::from_rgb8(220, 220, 220),
            handle_hover_color: Color::from_rgb8(180, 180, 200),
            handle_pressed_color: Color::from_rgb8(140, 140, 180),
            dragging_splitter: None,
            drag_start: 0.0,
            drag_start_size: 0.0,
            hover_splitter: None,
            dragging_dock_widget: None,
            dock_preview_area: None,
            dock_widget_added: Signal::new(),
            dock_widget_removed: Signal::new(),
        }
    }

    // =========================================================================
    // Menu Bar
    // =========================================================================

    /// Get a reference to the menu bar, if any.
    pub fn menu_bar(&self) -> Option<&MenuBar> {
        self.menu_bar.as_ref()
    }

    /// Get a mutable reference to the menu bar, if any.
    pub fn menu_bar_mut(&mut self) -> Option<&mut MenuBar> {
        self.menu_bar.as_mut()
    }

    /// Set the menu bar for this window.
    ///
    /// The menu bar appears at the top of the window, above the dock areas.
    pub fn set_menu_bar(&mut self, menu_bar: Option<MenuBar>) {
        self.menu_bar = menu_bar;
        self.base.update();
    }

    /// Set menu bar using builder pattern.
    pub fn with_menu_bar(mut self, menu_bar: MenuBar) -> Self {
        self.menu_bar = Some(menu_bar);
        self
    }

    /// Get the height of the menu bar (0 if no menu bar).
    fn menu_bar_height(&self) -> f32 {
        match &self.menu_bar {
            Some(mb) => mb.style().height,
            None => 0.0,
        }
    }

    // =========================================================================
    // Toolbars
    // =========================================================================

    /// Add a toolbar to the default area (top).
    ///
    /// The toolbar will be added to the top area by default.
    pub fn add_toolbar(&mut self, toolbar_id: ObjectId) {
        self.add_toolbar_to_area(self.default_toolbar_area, toolbar_id);
    }

    /// Add a toolbar to a specific area.
    pub fn add_toolbar_to_area(&mut self, area: ToolBarArea, toolbar_id: ObjectId) {
        // Remove from any other area first
        self.remove_toolbar_from_areas(toolbar_id);

        // Add to the specified area
        if let Some(container) = self.toolbar_areas.get_mut(&area) {
            container.add_toolbar(toolbar_id);
        }

        self.base.update();
    }

    /// Add a toolbar break (starts a new row of toolbars).
    ///
    /// The break is added after the most recently added toolbar in the specified area.
    pub fn add_toolbar_break(&mut self, area: ToolBarArea) {
        if let Some(container) = self.toolbar_areas.get_mut(&area) {
            container.add_toolbar_break();
        }
        self.base.update();
    }

    /// Remove a toolbar from all areas.
    fn remove_toolbar_from_areas(&mut self, toolbar_id: ObjectId) {
        for container in self.toolbar_areas.values_mut() {
            container.remove_toolbar(toolbar_id);
        }
    }

    /// Remove a toolbar completely.
    pub fn remove_toolbar(&mut self, toolbar_id: ObjectId) {
        self.remove_toolbar_from_areas(toolbar_id);
        self.base.update();
    }

    /// Get the area containing a toolbar.
    pub fn toolbar_area(&self, toolbar_id: ObjectId) -> Option<ToolBarArea> {
        for (area, container) in &self.toolbar_areas {
            if container.contains(toolbar_id) {
                return Some(*area);
            }
        }
        None
    }

    /// Get all toolbar IDs in a specific area.
    pub fn toolbars_in_area(&self, area: ToolBarArea) -> Vec<ObjectId> {
        self.toolbar_areas
            .get(&area)
            .map(|c| c.toolbars.clone())
            .unwrap_or_default()
    }

    /// Get the total height of toolbar areas (for layout).
    fn toolbar_area_total_height(&self, area: ToolBarArea) -> f32 {
        self.toolbar_areas
            .get(&area)
            .filter(|c| !c.is_empty())
            .map(|_| self.toolbar_area_height)
            .unwrap_or(0.0)
    }

    /// Calculate the rectangle for a toolbar area.
    pub fn toolbar_area_rect(&self, area: ToolBarArea) -> Rect {
        let rect = self.base.rect();
        let menu_height = self.menu_bar_height();
        let top_toolbar_height = self.toolbar_area_total_height(ToolBarArea::Top);
        let bottom_toolbar_height = self.toolbar_area_total_height(ToolBarArea::Bottom);
        let left_toolbar_width = self.toolbar_area_total_height(ToolBarArea::Left);
        let right_toolbar_width = self.toolbar_area_total_height(ToolBarArea::Right);

        match area {
            ToolBarArea::Top => {
                if top_toolbar_height > 0.0 {
                    Rect::new(0.0, menu_height, rect.width(), top_toolbar_height)
                } else {
                    Rect::ZERO
                }
            }
            ToolBarArea::Bottom => {
                if bottom_toolbar_height > 0.0 {
                    Rect::new(
                        0.0,
                        rect.height() - bottom_toolbar_height,
                        rect.width(),
                        bottom_toolbar_height,
                    )
                } else {
                    Rect::ZERO
                }
            }
            ToolBarArea::Left => {
                if left_toolbar_width > 0.0 {
                    Rect::new(
                        0.0,
                        menu_height + top_toolbar_height,
                        left_toolbar_width,
                        rect.height() - menu_height - top_toolbar_height - bottom_toolbar_height,
                    )
                } else {
                    Rect::ZERO
                }
            }
            ToolBarArea::Right => {
                if right_toolbar_width > 0.0 {
                    Rect::new(
                        rect.width() - right_toolbar_width,
                        menu_height + top_toolbar_height,
                        right_toolbar_width,
                        rect.height() - menu_height - top_toolbar_height - bottom_toolbar_height,
                    )
                } else {
                    Rect::ZERO
                }
            }
        }
    }

    // =========================================================================
    // Central Widget
    // =========================================================================

    /// Get the central widget ID.
    pub fn central_widget(&self) -> Option<ObjectId> {
        self.central_widget
    }

    /// Set the central widget.
    pub fn set_central_widget(&mut self, widget_id: ObjectId) {
        self.central_widget = Some(widget_id);
        self.base.update();
    }

    /// Set central widget using builder pattern.
    pub fn with_central_widget(mut self, widget_id: ObjectId) -> Self {
        self.central_widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Dock Widgets
    // =========================================================================

    /// Add a dock widget to a dock area.
    ///
    /// The widget will be added to the specified area. If there are already
    /// widgets in that area, the new widget will be added as a tab.
    pub fn add_dock_widget(&mut self, area: DockArea, widget_id: ObjectId) {
        // Remove from any other area first
        self.remove_dock_widget_from_areas(widget_id);

        // Add to the specified area
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.add_widget(widget_id);
        }

        self.dock_widget_added.emit(widget_id);
        self.base.update();
    }

    /// Remove a dock widget from all dock areas.
    fn remove_dock_widget_from_areas(&mut self, widget_id: ObjectId) {
        for container in self.dock_areas.values_mut() {
            container.remove_widget(widget_id);
        }
        self.floating_widgets.retain(|&id| id != widget_id);
    }

    /// Remove a dock widget completely.
    pub fn remove_dock_widget(&mut self, widget_id: ObjectId) {
        self.remove_dock_widget_from_areas(widget_id);
        self.dock_widget_removed.emit(widget_id);
        self.base.update();
    }

    /// Get the dock area containing a widget.
    pub fn dock_widget_area(&self, widget_id: ObjectId) -> Option<DockArea> {
        for (area, container) in &self.dock_areas {
            if container.contains(widget_id) {
                return Some(*area);
            }
        }
        None
    }

    /// Move a dock widget to a different area.
    pub fn move_dock_widget(&mut self, widget_id: ObjectId, area: DockArea) {
        self.remove_dock_widget_from_areas(widget_id);
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.add_widget(widget_id);
        }
        self.base.update();
    }

    /// Get all dock widget IDs in a specific area.
    pub fn dock_widgets_in_area(&self, area: DockArea) -> Vec<ObjectId> {
        self.dock_areas
            .get(&area)
            .map(|c| c.widgets.iter().map(|w| w.widget_id).collect())
            .unwrap_or_default()
    }

    /// Make a dock widget float.
    pub fn float_dock_widget(&mut self, widget_id: ObjectId) {
        self.remove_dock_widget_from_areas(widget_id);
        if !self.floating_widgets.contains(&widget_id) {
            self.floating_widgets.push(widget_id);
        }
        self.base.update();
    }

    /// Get all floating dock widgets.
    pub fn floating_dock_widgets(&self) -> &[ObjectId] {
        &self.floating_widgets
    }

    // =========================================================================
    // Dock Area Configuration
    // =========================================================================

    /// Set the size of a dock area.
    pub fn set_dock_area_size(&mut self, area: DockArea, size: f32) {
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.size = size.max(container.min_size);
            self.base.update();
        }
    }

    /// Get the size of a dock area.
    pub fn dock_area_size(&self, area: DockArea) -> f32 {
        self.dock_areas.get(&area).map(|c| c.size).unwrap_or(0.0)
    }

    /// Set the minimum size of a dock area.
    pub fn set_dock_area_min_size(&mut self, area: DockArea, min_size: f32) {
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.min_size = min_size;
            if container.size < min_size {
                container.size = min_size;
                self.base.update();
            }
        }
    }

    /// Collapse a dock area.
    pub fn collapse_dock_area(&mut self, area: DockArea) {
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.collapsed = true;
            self.base.update();
        }
    }

    /// Expand a collapsed dock area.
    pub fn expand_dock_area(&mut self, area: DockArea) {
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.collapsed = false;
            self.base.update();
        }
    }

    /// Check if a dock area is collapsed.
    pub fn is_dock_area_collapsed(&self, area: DockArea) -> bool {
        self.dock_areas
            .get(&area)
            .map(|c| c.collapsed)
            .unwrap_or(false)
    }

    /// Set whether a dock area uses tabbed mode.
    pub fn set_dock_area_tabbed(&mut self, area: DockArea, tabbed: bool) {
        if let Some(container) = self.dock_areas.get_mut(&area) {
            container.tabbed = tabbed;
            self.base.update();
        }
    }

    // =========================================================================
    // Styling
    // =========================================================================

    /// Set the splitter handle width.
    pub fn set_handle_width(&mut self, width: f32) {
        self.handle_width = width;
        self.base.update();
    }

    /// Set handle width using builder pattern.
    pub fn with_handle_width(mut self, width: f32) -> Self {
        self.handle_width = width;
        self
    }

    /// Set content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        self.content_margins = margins;
        self.base.update();
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.content_margins = margins;
        self
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Calculate the available content area (inside margins, below menu bar and toolbars).
    fn content_area(&self) -> Rect {
        let rect = self.base.rect();
        let menu_height = self.menu_bar_height();
        let top_toolbar_height = self.toolbar_area_total_height(ToolBarArea::Top);
        let bottom_toolbar_height = self.toolbar_area_total_height(ToolBarArea::Bottom);
        let left_toolbar_width = self.toolbar_area_total_height(ToolBarArea::Left);
        let right_toolbar_width = self.toolbar_area_total_height(ToolBarArea::Right);

        Rect::new(
            self.content_margins.left + left_toolbar_width,
            self.content_margins.top + menu_height + top_toolbar_height,
            rect.width()
                - self.content_margins.horizontal()
                - left_toolbar_width
                - right_toolbar_width,
            rect.height()
                - self.content_margins.vertical()
                - menu_height
                - top_toolbar_height
                - bottom_toolbar_height,
        )
    }

    /// Calculate the menu bar rectangle.
    pub fn menu_bar_rect(&self) -> Rect {
        let rect = self.base.rect();
        let menu_height = self.menu_bar_height();
        if menu_height > 0.0 {
            Rect::new(0.0, 0.0, rect.width(), menu_height)
        } else {
            Rect::ZERO
        }
    }

    /// Calculate the effective size of a dock area (0 if empty or collapsed).
    fn effective_dock_size(&self, area: DockArea) -> f32 {
        self.dock_areas
            .get(&area)
            .filter(|c| !c.is_empty() && !c.collapsed)
            .map(|c| c.size)
            .unwrap_or(0.0)
    }

    /// Calculate the rectangle for a dock area.
    pub fn dock_area_rect(&self, area: DockArea) -> Rect {
        let content = self.content_area();
        let left_size = self.effective_dock_size(DockArea::Left);
        let right_size = self.effective_dock_size(DockArea::Right);
        let top_size = self.effective_dock_size(DockArea::Top);
        let bottom_size = self.effective_dock_size(DockArea::Bottom);

        let has_left = left_size > 0.0;
        let has_right = right_size > 0.0;
        let has_top = top_size > 0.0;
        let has_bottom = bottom_size > 0.0;

        match area {
            DockArea::Left => {
                if !has_left {
                    return Rect::ZERO;
                }
                Rect::new(
                    content.origin.x,
                    content.origin.y
                        + if has_top {
                            top_size + self.handle_width
                        } else {
                            0.0
                        },
                    left_size,
                    content.height()
                        - if has_top {
                            top_size + self.handle_width
                        } else {
                            0.0
                        }
                        - if has_bottom {
                            bottom_size + self.handle_width
                        } else {
                            0.0
                        },
                )
            }
            DockArea::Right => {
                if !has_right {
                    return Rect::ZERO;
                }
                Rect::new(
                    content.origin.x + content.width() - right_size,
                    content.origin.y
                        + if has_top {
                            top_size + self.handle_width
                        } else {
                            0.0
                        },
                    right_size,
                    content.height()
                        - if has_top {
                            top_size + self.handle_width
                        } else {
                            0.0
                        }
                        - if has_bottom {
                            bottom_size + self.handle_width
                        } else {
                            0.0
                        },
                )
            }
            DockArea::Top => {
                if !has_top {
                    return Rect::ZERO;
                }
                Rect::new(
                    content.origin.x,
                    content.origin.y,
                    content.width(),
                    top_size,
                )
            }
            DockArea::Bottom => {
                if !has_bottom {
                    return Rect::ZERO;
                }
                Rect::new(
                    content.origin.x,
                    content.origin.y + content.height() - bottom_size,
                    content.width(),
                    bottom_size,
                )
            }
        }
    }

    /// Calculate the rectangle for the central widget.
    pub fn central_rect(&self) -> Rect {
        let content = self.content_area();
        let left_size = self.effective_dock_size(DockArea::Left);
        let right_size = self.effective_dock_size(DockArea::Right);
        let top_size = self.effective_dock_size(DockArea::Top);
        let bottom_size = self.effective_dock_size(DockArea::Bottom);

        let has_left = left_size > 0.0;
        let has_right = right_size > 0.0;
        let has_top = top_size > 0.0;
        let has_bottom = bottom_size > 0.0;

        let x = content.origin.x
            + if has_left {
                left_size + self.handle_width
            } else {
                0.0
            };
        let y = content.origin.y
            + if has_top {
                top_size + self.handle_width
            } else {
                0.0
            };
        let width = content.width()
            - if has_left {
                left_size + self.handle_width
            } else {
                0.0
            }
            - if has_right {
                right_size + self.handle_width
            } else {
                0.0
            };
        let height = content.height()
            - if has_top {
                top_size + self.handle_width
            } else {
                0.0
            }
            - if has_bottom {
                bottom_size + self.handle_width
            } else {
                0.0
            };

        Rect::new(x, y, width.max(0.0), height.max(0.0))
    }

    /// Calculate the splitter handle rectangle for a dock area.
    fn splitter_rect(&self, area: DockArea) -> Option<Rect> {
        let dock_rect = self.dock_area_rect(area);
        if dock_rect.width() <= 0.0 || dock_rect.height() <= 0.0 {
            return None;
        }

        match area {
            DockArea::Left => Some(Rect::new(
                dock_rect.right(),
                dock_rect.origin.y,
                self.handle_width,
                dock_rect.height(),
            )),
            DockArea::Right => Some(Rect::new(
                dock_rect.origin.x - self.handle_width,
                dock_rect.origin.y,
                self.handle_width,
                dock_rect.height(),
            )),
            DockArea::Top => Some(Rect::new(
                dock_rect.origin.x,
                dock_rect.bottom(),
                dock_rect.width(),
                self.handle_width,
            )),
            DockArea::Bottom => Some(Rect::new(
                dock_rect.origin.x,
                dock_rect.origin.y - self.handle_width,
                dock_rect.width(),
                self.handle_width,
            )),
        }
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    /// Check which splitter handle is at the given position.
    fn hit_test_splitter(&self, pos: Point) -> Option<DockArea> {
        for area in DockArea::all() {
            if let Some(rect) = self.splitter_rect(area) {
                // Expand hit area slightly for easier grabbing
                let expanded = Rect::new(
                    rect.origin.x - 2.0,
                    rect.origin.y - 2.0,
                    rect.width() + 4.0,
                    rect.height() + 4.0,
                );
                if expanded.contains(pos) {
                    return Some(area);
                }
            }
        }
        None
    }

    /// Check which dock area is at the given position (for drag-to-dock preview).
    fn hit_test_dock_area(&self, pos: Point) -> Option<DockArea> {
        let content = self.content_area();

        // Define dock zones at edges of the content area
        let zone_size = 50.0;

        if pos.x < content.origin.x + zone_size {
            return Some(DockArea::Left);
        }
        if pos.x > content.right() - zone_size {
            return Some(DockArea::Right);
        }
        if pos.y < content.origin.y + zone_size {
            return Some(DockArea::Top);
        }
        if pos.y > content.bottom() - zone_size {
            return Some(DockArea::Bottom);
        }

        None
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check splitter press
        if let Some(area) = self.hit_test_splitter(event.local_pos) {
            self.dragging_splitter = Some(area);
            self.drag_start = if area.is_horizontal() {
                event.local_pos.x
            } else {
                event.local_pos.y
            };
            if let Some(container) = self.dock_areas.get(&area) {
                self.drag_start_size = container.size;
            }
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // End splitter drag
        if self.dragging_splitter.is_some() {
            self.dragging_splitter = None;
            return true;
        }

        // End dock widget drag (dock to preview area)
        if let Some(widget_id) = self.dragging_dock_widget.take() {
            if let Some(area) = self.dock_preview_area.take() {
                self.add_dock_widget(area, widget_id);
            }
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Handle splitter drag
        if let Some(area) = self.dragging_splitter {
            let current = if area.is_horizontal() { pos.x } else { pos.y };
            let delta = current - self.drag_start;

            let new_size = match area {
                DockArea::Left | DockArea::Top => self.drag_start_size + delta,
                DockArea::Right | DockArea::Bottom => self.drag_start_size - delta,
            };

            if let Some(container) = self.dock_areas.get_mut(&area) {
                container.size = new_size.max(container.min_size);
            }
            self.base.update();
            return true;
        }

        // Update splitter hover
        let new_hover = self.hit_test_splitter(pos);
        if self.hover_splitter != new_hover {
            self.hover_splitter = new_hover;
            self.base.update();
        }

        // Handle dock widget drag preview
        if self.dragging_dock_widget.is_some() {
            let new_preview = self.hit_test_dock_area(pos);
            if self.dock_preview_area != new_preview {
                self.dock_preview_area = new_preview;
                self.base.update();
            }
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        ctx.renderer().fill_rect(
            Rect::new(0.0, 0.0, rect.width(), rect.height()),
            self.background_color,
        );
    }

    fn paint_menu_bar(&self, ctx: &mut PaintContext<'_>) {
        if let Some(ref menu_bar) = self.menu_bar {
            menu_bar.paint(ctx);
        }
    }

    fn paint_dock_areas(&self, ctx: &mut PaintContext<'_>) {
        // Paint dock area backgrounds
        for area in DockArea::all() {
            let rect = self.dock_area_rect(area);
            if rect.width() > 0.0 && rect.height() > 0.0 {
                // Slightly different background for dock areas
                ctx.renderer()
                    .fill_rect(rect, Color::from_rgb8(250, 250, 250));
            }
        }
    }

    fn paint_splitters(&self, ctx: &mut PaintContext<'_>) {
        for area in DockArea::all() {
            if let Some(rect) = self.splitter_rect(area) {
                let color = if self.dragging_splitter == Some(area) {
                    self.handle_pressed_color
                } else if self.hover_splitter == Some(area) {
                    self.handle_hover_color
                } else {
                    self.handle_color
                };
                ctx.renderer().fill_rect(rect, color);
            }
        }
    }

    fn paint_central_area(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.central_rect();
        if rect.width() > 0.0 && rect.height() > 0.0 {
            ctx.renderer().fill_rect(rect, Color::WHITE);
        }
    }

    fn paint_dock_preview(&self, ctx: &mut PaintContext<'_>) {
        if let Some(area) = self.dock_preview_area {
            let content = self.content_area();
            let zone_size = 100.0;

            let preview_rect = match area {
                DockArea::Left => Rect::new(
                    content.origin.x,
                    content.origin.y,
                    zone_size,
                    content.height(),
                ),
                DockArea::Right => Rect::new(
                    content.right() - zone_size,
                    content.origin.y,
                    zone_size,
                    content.height(),
                ),
                DockArea::Top => Rect::new(
                    content.origin.x,
                    content.origin.y,
                    content.width(),
                    zone_size,
                ),
                DockArea::Bottom => Rect::new(
                    content.origin.x,
                    content.bottom() - zone_size,
                    content.width(),
                    zone_size,
                ),
            };

            // Semi-transparent preview
            ctx.renderer()
                .fill_rect(preview_rect, Color::from_rgba8(100, 150, 255, 100));
            let stroke = Stroke::new(Color::from_rgb8(60, 100, 200), 2.0);
            ctx.renderer().stroke_rect(preview_rect, &stroke);
        }
    }

    // =========================================================================
    // Public API for Drag-to-Dock
    // =========================================================================

    /// Start dragging a dock widget for repositioning.
    ///
    /// Call this when a dock widget's title bar is dragged. The MainWindow
    /// will show dock preview indicators.
    pub fn start_dock_widget_drag(&mut self, widget_id: ObjectId) {
        self.dragging_dock_widget = Some(widget_id);
    }

    /// Update dock preview during drag.
    pub fn update_dock_preview(&mut self, pos: Point) {
        self.dock_preview_area = self.hit_test_dock_area(pos);
        self.base.update();
    }

    /// End dock widget drag and dock if over a valid area.
    pub fn end_dock_widget_drag(&mut self) -> Option<DockArea> {
        let widget_id = self.dragging_dock_widget.take();
        let area = self.dock_preview_area.take();

        if let (Some(widget_id), Some(area)) = (widget_id, area) {
            self.add_dock_widget(area, widget_id);
            self.base.update();
            return Some(area);
        }

        self.base.update();
        None
    }

    /// Cancel dock widget drag without docking.
    pub fn cancel_dock_widget_drag(&mut self) {
        self.dragging_dock_widget = None;
        self.dock_preview_area = None;
        self.base.update();
    }

    // =========================================================================
    // State Save/Restore
    // =========================================================================

    /// Save the current dock area sizes to a vector.
    pub fn save_dock_sizes(&self) -> Vec<(DockArea, f32)> {
        self.dock_areas
            .iter()
            .map(|(area, container)| (*area, container.size))
            .collect()
    }

    /// Restore dock area sizes from a saved state.
    pub fn restore_dock_sizes(&mut self, sizes: &[(DockArea, f32)]) {
        for (area, size) in sizes {
            if let Some(container) = self.dock_areas.get_mut(area) {
                container.size = *size;
            }
        }
        self.base.update();
    }
}

impl Widget for MainWindow {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // MainWindow wants to be as large as possible
        SizeHint::new(Size::new(800.0, 600.0)).with_minimum(Size::new(400.0, 300.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_menu_bar(ctx);
        self.paint_dock_areas(ctx);
        self.paint_central_area(ctx);
        self.paint_splitters(ctx);
        self.paint_dock_preview(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            _ => false,
        }
    }
}

impl Object for MainWindow {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_area_is_horizontal() {
        assert!(DockArea::Left.is_horizontal());
        assert!(DockArea::Right.is_horizontal());
        assert!(!DockArea::Top.is_horizontal());
        assert!(!DockArea::Bottom.is_horizontal());
    }

    #[test]
    fn test_dock_area_is_vertical() {
        assert!(!DockArea::Left.is_vertical());
        assert!(!DockArea::Right.is_vertical());
        assert!(DockArea::Top.is_vertical());
        assert!(DockArea::Bottom.is_vertical());
    }

    #[test]
    fn test_dock_area_container() {
        let container = DockAreaContainer::new(DockArea::Left);
        assert!(container.is_empty());
        // Note: Adding widgets requires real ObjectIds from the registry.
        // See splitter.rs for mock widget patterns.
    }
}
