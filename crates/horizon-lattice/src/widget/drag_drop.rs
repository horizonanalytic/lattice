//! Drag and drop support for the widget system.
//!
//! This module provides drag and drop functionality including:
//!
//! - Internal drag/drop between widgets within the application
//! - Receiving file drops from external applications (via winit)
//!
//! # Internal Drag and Drop
//!
//! Widgets can act as drag sources and/or drop targets. To initiate a drag:
//!
//! ```ignore
//! use horizon_lattice::widget::drag_drop::{DragData, DropAction};
//!
//! // In a mouse press handler:
//! let mut data = DragData::new();
//! data.set_text("Hello, world!");
//! data.set_source_widget(self.object_id());
//!
//! // Start the drag operation
//! drag_drop_manager.start_drag(data, DropAction::COPY | DropAction::MOVE);
//! ```
//!
//! To accept drops, configure the widget as a drop target:
//!
//! ```ignore
//! // In widget construction:
//! widget.set_accepts_drops(true);
//!
//! // In the event handler:
//! fn event(&mut self, event: &mut WidgetEvent) -> bool {
//!     match event {
//!         WidgetEvent::DragEnter(e) => {
//!             if e.data().has_text() {
//!                 e.accept_proposed_action();
//!                 true
//!             } else {
//!                 false
//!             }
//!         }
//!         WidgetEvent::Drop(e) => {
//!             if let Some(text) = e.data().text() {
//!                 println!("Dropped: {}", text);
//!                 e.accept();
//!             }
//!             true
//!         }
//!         _ => false,
//!     }
//! }
//! ```
//!
//! # External File Drops
//!
//! Files dragged from the operating system are automatically converted to
//! drop events. The file paths are available via [`DragData::urls`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::Point;

use super::events::EventBase;

/// Standard MIME types used in drag and drop operations.
pub mod mime {
    /// Plain text MIME type.
    pub const TEXT_PLAIN: &str = "text/plain";
    /// HTML MIME type.
    pub const TEXT_HTML: &str = "text/html";
    /// URI list MIME type (for file paths and URLs).
    pub const TEXT_URI_LIST: &str = "text/uri-list";
    /// Custom application data prefix.
    pub const APPLICATION_PREFIX: &str = "application/x-horizon-lattice-";
}

/// Actions that can be performed during a drop operation.
///
/// These flags indicate what actions are supported by the drag source
/// and what action was performed by the drop target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DropAction(u8);

impl DropAction {
    /// No action (drop not allowed).
    pub const NONE: Self = Self(0);
    /// Copy the data.
    pub const COPY: Self = Self(1 << 0);
    /// Move the data (source should delete original).
    pub const MOVE: Self = Self(1 << 1);
    /// Create a link/reference to the data.
    pub const LINK: Self = Self(1 << 2);
    /// All standard actions (copy, move, and link).
    pub const ALL: Self = Self(Self::COPY.0 | Self::MOVE.0 | Self::LINK.0);

    /// Returns true if this action set contains the Copy action.
    pub fn can_copy(self) -> bool {
        self.contains(Self::COPY)
    }

    /// Returns true if this action set contains the Move action.
    pub fn can_move(self) -> bool {
        self.contains(Self::MOVE)
    }

    /// Returns true if this action set contains the Link action.
    pub fn can_link(self) -> bool {
        self.contains(Self::LINK)
    }

    /// Returns true if this action set contains another action.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns the preferred action from this set.
    ///
    /// Priority: Copy > Move > Link > None
    pub fn preferred(self) -> Self {
        if self.can_copy() {
            Self::COPY
        } else if self.can_move() {
            Self::MOVE
        } else if self.can_link() {
            Self::LINK
        } else {
            Self::NONE
        }
    }
}

impl std::ops::BitOr for DropAction {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for DropAction {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for DropAction {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for DropAction {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// Data being transferred in a drag and drop operation.
///
/// `DragData` can hold multiple representations of the same data,
/// each identified by a MIME type. This allows drop targets to
/// choose the most appropriate format.
#[derive(Debug, Clone, Default)]
pub struct DragData {
    /// Data stored by MIME type.
    data: HashMap<String, Vec<u8>>,
    /// File/URL paths being dragged.
    urls: Vec<PathBuf>,
    /// The widget that initiated the drag (if internal).
    source_widget: Option<ObjectId>,
    /// Custom user data (type-erased).
    user_data: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl DragData {
    /// Creates empty drag data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates drag data from a list of file paths.
    ///
    /// This is typically used for external file drops from the OS.
    pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let urls: Vec<PathBuf> = paths.into_iter().collect();
        let mut data = Self::default();
        data.urls = urls;
        data
    }

    /// Creates drag data with plain text.
    pub fn from_text(text: impl Into<String>) -> Self {
        let mut data = Self::default();
        data.set_text(text);
        data
    }

    /// Returns true if this drag data is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.urls.is_empty()
    }

    /// Returns the available MIME formats.
    pub fn formats(&self) -> impl Iterator<Item = &str> {
        self.data.keys().map(|s| s.as_str())
    }

    /// Checks if data is available for the given MIME type.
    pub fn has_format(&self, mime_type: &str) -> bool {
        self.data.contains_key(mime_type)
    }

    /// Gets raw data for a MIME type.
    pub fn get_data(&self, mime_type: &str) -> Option<&[u8]> {
        self.data.get(mime_type).map(|v| v.as_slice())
    }

    /// Sets raw data for a MIME type.
    pub fn set_data(&mut self, mime_type: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.data.insert(mime_type.into(), data.into());
    }

    // -------------------------------------------------------------------------
    // Text convenience methods
    // -------------------------------------------------------------------------

    /// Returns true if plain text is available.
    pub fn has_text(&self) -> bool {
        self.has_format(mime::TEXT_PLAIN)
    }

    /// Gets the plain text content, if available.
    pub fn text(&self) -> Option<String> {
        self.get_data(mime::TEXT_PLAIN)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    /// Sets the plain text content.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.set_data(mime::TEXT_PLAIN, text.into_bytes());
    }

    // -------------------------------------------------------------------------
    // HTML convenience methods
    // -------------------------------------------------------------------------

    /// Returns true if HTML is available.
    pub fn has_html(&self) -> bool {
        self.has_format(mime::TEXT_HTML)
    }

    /// Gets the HTML content, if available.
    pub fn html(&self) -> Option<String> {
        self.get_data(mime::TEXT_HTML)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    /// Sets the HTML content.
    pub fn set_html(&mut self, html: impl Into<String>) {
        let html = html.into();
        self.set_data(mime::TEXT_HTML, html.into_bytes());
    }

    // -------------------------------------------------------------------------
    // URL/file path methods
    // -------------------------------------------------------------------------

    /// Returns true if URLs/file paths are available.
    pub fn has_urls(&self) -> bool {
        !self.urls.is_empty()
    }

    /// Gets the URLs/file paths.
    pub fn urls(&self) -> &[PathBuf] {
        &self.urls
    }

    /// Sets the URLs/file paths.
    pub fn set_urls(&mut self, urls: impl IntoIterator<Item = PathBuf>) {
        self.urls = urls.into_iter().collect();
    }

    /// Adds a single URL/file path.
    pub fn add_url(&mut self, url: PathBuf) {
        self.urls.push(url);
    }

    // -------------------------------------------------------------------------
    // Source widget tracking
    // -------------------------------------------------------------------------

    /// Gets the source widget ID (for internal drags).
    pub fn source_widget(&self) -> Option<ObjectId> {
        self.source_widget
    }

    /// Sets the source widget ID.
    pub fn set_source_widget(&mut self, id: ObjectId) {
        self.source_widget = Some(id);
    }

    /// Returns true if this is an internal drag (has a source widget).
    pub fn is_internal(&self) -> bool {
        self.source_widget.is_some()
    }

    // -------------------------------------------------------------------------
    // Custom user data
    // -------------------------------------------------------------------------

    /// Sets custom user data.
    ///
    /// This allows attaching arbitrary data to a drag operation for
    /// application-specific purposes.
    pub fn set_user_data<T: Send + Sync + 'static>(&mut self, data: T) {
        self.user_data = Some(Arc::new(data));
    }

    /// Gets custom user data, if it matches the requested type.
    pub fn user_data<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.user_data
            .as_ref()
            .and_then(|d| d.downcast_ref::<T>())
    }
}

/// State of an active drag operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragState {
    /// No drag is active.
    Idle,
    /// A drag is in progress.
    Dragging,
}

/// Manager for tracking drag and drop operations.
///
/// There is typically one `DragDropManager` per window that tracks
/// the current drag state and routes events to appropriate widgets.
#[derive(Debug)]
pub struct DragDropManager {
    /// Current drag state.
    state: DragState,
    /// Data being dragged (if any).
    drag_data: Option<Arc<DragData>>,
    /// Supported actions for the current drag.
    supported_actions: DropAction,
    /// The current proposed action.
    proposed_action: DropAction,
    /// The widget currently under the drag cursor.
    current_target: Option<ObjectId>,
    /// The widget that initiated the drag (for internal drags).
    source_widget: Option<ObjectId>,
    /// Current drag position in window coordinates.
    drag_position: Point,
    /// Position where the drag started.
    start_position: Point,
    /// Minimum distance to move before a drag starts.
    drag_threshold: f32,
    /// Whether we're in the "pending" state (mouse down, waiting for threshold).
    pending_drag: bool,
    /// Pending drag data (before threshold is reached).
    pending_data: Option<DragData>,
    /// Pending supported actions.
    pending_actions: DropAction,
}

impl Default for DragDropManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DragDropManager {
    /// Default drag threshold in pixels.
    pub const DEFAULT_DRAG_THRESHOLD: f32 = 4.0;

    /// Creates a new drag/drop manager.
    pub fn new() -> Self {
        Self {
            state: DragState::Idle,
            drag_data: None,
            supported_actions: DropAction::NONE,
            proposed_action: DropAction::NONE,
            current_target: None,
            source_widget: None,
            drag_position: Point::ZERO,
            start_position: Point::ZERO,
            drag_threshold: Self::DEFAULT_DRAG_THRESHOLD,
            pending_drag: false,
            pending_data: None,
            pending_actions: DropAction::NONE,
        }
    }

    /// Sets the minimum distance (in pixels) the mouse must move to start a drag.
    pub fn set_drag_threshold(&mut self, threshold: f32) {
        self.drag_threshold = threshold;
    }

    /// Returns the current drag state.
    pub fn state(&self) -> DragState {
        self.state
    }

    /// Returns true if a drag is currently active.
    pub fn is_dragging(&self) -> bool {
        self.state == DragState::Dragging
    }

    /// Returns the data being dragged, if any.
    pub fn drag_data(&self) -> Option<&DragData> {
        self.drag_data.as_ref().map(|arc| arc.as_ref())
    }

    /// Returns the supported actions for the current drag.
    pub fn supported_actions(&self) -> DropAction {
        self.supported_actions
    }

    /// Returns the currently proposed action.
    pub fn proposed_action(&self) -> DropAction {
        self.proposed_action
    }

    /// Returns the widget currently under the drag cursor.
    pub fn current_target(&self) -> Option<ObjectId> {
        self.current_target
    }

    /// Returns the widget that initiated the drag (for internal drags).
    pub fn source_widget(&self) -> Option<ObjectId> {
        self.source_widget
    }

    /// Returns the current drag position in window coordinates.
    pub fn drag_position(&self) -> Point {
        self.drag_position
    }

    /// Prepares a drag operation (called on mouse press).
    ///
    /// The actual drag won't start until the mouse moves past the threshold.
    /// This prevents accidental drags from interfering with normal clicks.
    pub fn prepare_drag(
        &mut self,
        data: DragData,
        supported_actions: DropAction,
        start_position: Point,
    ) {
        self.pending_drag = true;
        self.pending_data = Some(data);
        self.pending_actions = supported_actions;
        self.start_position = start_position;
    }

    /// Cancels a pending drag (before threshold is reached).
    pub fn cancel_pending(&mut self) {
        self.pending_drag = false;
        self.pending_data = None;
        self.pending_actions = DropAction::NONE;
    }

    /// Checks if a pending drag should start based on mouse movement.
    ///
    /// Returns true if the drag has just started.
    pub fn check_drag_start(&mut self, current_position: Point) -> bool {
        if !self.pending_drag {
            return false;
        }

        let dx = current_position.x - self.start_position.x;
        let dy = current_position.y - self.start_position.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance >= self.drag_threshold {
            // Start the actual drag
            if let Some(data) = self.pending_data.take() {
                self.start_drag_internal(data, self.pending_actions, self.start_position);
                self.pending_drag = false;
                self.pending_actions = DropAction::NONE;
                return true;
            }
        }

        false
    }

    /// Starts a drag operation immediately (for programmatic drags).
    pub fn start_drag(&mut self, data: DragData, supported_actions: DropAction, position: Point) {
        self.cancel_pending();
        self.start_drag_internal(data, supported_actions, position);
    }

    fn start_drag_internal(
        &mut self,
        data: DragData,
        supported_actions: DropAction,
        position: Point,
    ) {
        self.source_widget = data.source_widget();
        self.drag_data = Some(Arc::new(data));
        self.supported_actions = supported_actions;
        self.proposed_action = supported_actions.preferred();
        self.state = DragState::Dragging;
        self.drag_position = position;
        self.start_position = position;
        self.current_target = None;
    }

    /// Starts an external drag operation (e.g., file drop from OS).
    ///
    /// This bypasses the threshold check since the OS has already determined
    /// that a drag is in progress.
    pub fn start_external_drag(&mut self, data: DragData, position: Point) {
        self.cancel_pending();
        self.source_widget = None;
        self.drag_data = Some(Arc::new(data));
        self.supported_actions = DropAction::COPY; // External drags are typically copy-only
        self.proposed_action = DropAction::COPY;
        self.state = DragState::Dragging;
        self.drag_position = position;
        self.start_position = position;
        self.current_target = None;
    }

    /// Updates the drag position and current target.
    ///
    /// Returns the previous target if it changed (for sending DragLeave).
    pub fn update_position(&mut self, position: Point, new_target: Option<ObjectId>) -> Option<ObjectId> {
        self.drag_position = position;

        let previous_target = self.current_target;
        if previous_target != new_target {
            self.current_target = new_target;
            // Reset proposed action when target changes
            self.proposed_action = self.supported_actions.preferred();
            return previous_target;
        }

        None
    }

    /// Sets the proposed action (called by drop target to indicate acceptance).
    pub fn set_proposed_action(&mut self, action: DropAction) {
        // Only allow actions that are supported
        self.proposed_action = action & self.supported_actions;
    }

    /// Ends the drag operation and returns the data if dropped successfully.
    ///
    /// Returns `Some((data, action))` if the drop should be processed,
    /// or `None` if the drag was cancelled.
    pub fn end_drag(&mut self, dropped: bool) -> Option<(Arc<DragData>, DropAction)> {
        let result = if dropped && self.proposed_action != DropAction::NONE {
            self.drag_data.take().map(|data| (data, self.proposed_action))
        } else {
            None
        };

        self.reset();
        result
    }

    /// Cancels the current drag operation.
    pub fn cancel(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.state = DragState::Idle;
        self.drag_data = None;
        self.supported_actions = DropAction::NONE;
        self.proposed_action = DropAction::NONE;
        self.current_target = None;
        self.source_widget = None;
        self.pending_drag = false;
        self.pending_data = None;
        self.pending_actions = DropAction::NONE;
    }

    /// Returns true if there's a pending drag waiting for the threshold.
    pub fn has_pending_drag(&self) -> bool {
        self.pending_drag
    }
}

// =============================================================================
// Drag/Drop Events
// =============================================================================

/// Event sent when a drag enters a widget's bounds.
#[derive(Debug, Clone)]
pub struct DragEnterEvent {
    /// Base event data.
    pub base: EventBase,
    /// The data being dragged.
    data: Arc<DragData>,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Actions supported by the drag source.
    pub supported_actions: DropAction,
    /// The currently proposed action.
    proposed_action: DropAction,
}

impl DragEnterEvent {
    /// Creates a new drag enter event.
    pub fn new(
        data: Arc<DragData>,
        local_pos: Point,
        window_pos: Point,
        supported_actions: DropAction,
    ) -> Self {
        Self {
            base: EventBase::new(),
            data,
            local_pos,
            window_pos,
            supported_actions,
            proposed_action: supported_actions.preferred(),
        }
    }

    /// Returns the data being dragged.
    pub fn data(&self) -> &DragData {
        &self.data
    }

    /// Returns the proposed action.
    pub fn proposed_action(&self) -> DropAction {
        self.proposed_action
    }

    /// Sets the proposed action, accepting the drag.
    ///
    /// Call this to indicate that the widget can accept the drop.
    /// The action must be one of the supported actions.
    pub fn set_proposed_action(&mut self, action: DropAction) {
        self.proposed_action = action & self.supported_actions;
    }

    /// Accepts the drag with the default (preferred) action.
    pub fn accept_proposed_action(&mut self) {
        self.base.accept();
    }

    /// Ignores the drag, preventing this widget from being a drop target.
    pub fn ignore(&mut self) {
        self.base.ignore();
        self.proposed_action = DropAction::NONE;
    }
}

/// Event sent when a drag moves within a widget's bounds.
#[derive(Debug, Clone)]
pub struct DragMoveEvent {
    /// Base event data.
    pub base: EventBase,
    /// The data being dragged.
    data: Arc<DragData>,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Actions supported by the drag source.
    pub supported_actions: DropAction,
    /// The currently proposed action.
    proposed_action: DropAction,
}

impl DragMoveEvent {
    /// Creates a new drag move event.
    pub fn new(
        data: Arc<DragData>,
        local_pos: Point,
        window_pos: Point,
        supported_actions: DropAction,
        proposed_action: DropAction,
    ) -> Self {
        Self {
            base: EventBase::new(),
            data,
            local_pos,
            window_pos,
            supported_actions,
            proposed_action,
        }
    }

    /// Returns the data being dragged.
    pub fn data(&self) -> &DragData {
        &self.data
    }

    /// Returns the proposed action.
    pub fn proposed_action(&self) -> DropAction {
        self.proposed_action
    }

    /// Sets the proposed action.
    pub fn set_proposed_action(&mut self, action: DropAction) {
        self.proposed_action = action & self.supported_actions;
    }

    /// Accepts the continued drag.
    pub fn accept(&mut self) {
        self.base.accept();
    }
}

/// Event sent when a drag leaves a widget's bounds.
#[derive(Debug, Clone)]
pub struct DragLeaveEvent {
    /// Base event data.
    pub base: EventBase,
}

impl DragLeaveEvent {
    /// Creates a new drag leave event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for DragLeaveEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Event sent when data is dropped on a widget.
#[derive(Debug, Clone)]
pub struct DropEvent {
    /// Base event data.
    pub base: EventBase,
    /// The dropped data.
    data: Arc<DragData>,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// The action being performed.
    pub action: DropAction,
}

impl DropEvent {
    /// Creates a new drop event.
    pub fn new(data: Arc<DragData>, local_pos: Point, window_pos: Point, action: DropAction) -> Self {
        Self {
            base: EventBase::new(),
            data,
            local_pos,
            window_pos,
            action,
        }
    }

    /// Returns the dropped data.
    pub fn data(&self) -> &DragData {
        &self.data
    }

    /// Accepts the drop.
    pub fn accept(&mut self) {
        self.base.accept();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_action_flags() {
        let actions = DropAction::COPY | DropAction::MOVE;
        assert!(actions.can_copy());
        assert!(actions.can_move());
        assert!(!actions.can_link());
        assert_eq!(actions.preferred(), DropAction::COPY);
    }

    #[test]
    fn test_drop_action_preferred() {
        assert_eq!(DropAction::NONE.preferred(), DropAction::NONE);
        assert_eq!(DropAction::COPY.preferred(), DropAction::COPY);
        assert_eq!(DropAction::MOVE.preferred(), DropAction::MOVE);
        assert_eq!(DropAction::LINK.preferred(), DropAction::LINK);
        assert_eq!((DropAction::MOVE | DropAction::LINK).preferred(), DropAction::MOVE);
    }

    #[test]
    fn test_drag_data_text() {
        let mut data = DragData::new();
        assert!(!data.has_text());

        data.set_text("Hello, world!");
        assert!(data.has_text());
        assert_eq!(data.text(), Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_drag_data_html() {
        let mut data = DragData::new();
        assert!(!data.has_html());

        data.set_html("<b>Bold</b>");
        assert!(data.has_html());
        assert_eq!(data.html(), Some("<b>Bold</b>".to_string()));
    }

    #[test]
    fn test_drag_data_urls() {
        let paths = vec![PathBuf::from("/tmp/file1.txt"), PathBuf::from("/tmp/file2.txt")];
        let data = DragData::from_paths(paths.clone());

        assert!(data.has_urls());
        assert_eq!(data.urls(), &paths);
    }

    #[test]
    fn test_drag_data_user_data() {
        #[derive(Debug, PartialEq)]
        struct MyData {
            value: i32,
        }

        let mut data = DragData::new();
        data.set_user_data(MyData { value: 42 });

        let user_data = data.user_data::<MyData>();
        assert!(user_data.is_some());
        assert_eq!(user_data.unwrap().value, 42);

        // Wrong type returns None
        assert!(data.user_data::<String>().is_none());
    }

    #[test]
    fn test_drag_drop_manager_lifecycle() {
        let mut manager = DragDropManager::new();
        assert_eq!(manager.state(), DragState::Idle);
        assert!(!manager.is_dragging());

        // Start a drag
        let data = DragData::from_text("test");
        manager.start_drag(data, DropAction::COPY | DropAction::MOVE, Point::new(100.0, 100.0));

        assert_eq!(manager.state(), DragState::Dragging);
        assert!(manager.is_dragging());
        assert_eq!(manager.supported_actions(), DropAction::COPY | DropAction::MOVE);
        assert_eq!(manager.proposed_action(), DropAction::COPY);

        // End the drag
        let result = manager.end_drag(true);
        assert!(result.is_some());
        let (data, action) = result.unwrap();
        assert_eq!(data.text(), Some("test".to_string()));
        assert_eq!(action, DropAction::COPY);

        assert_eq!(manager.state(), DragState::Idle);
    }

    #[test]
    fn test_drag_threshold() {
        let mut manager = DragDropManager::new();
        manager.set_drag_threshold(10.0);

        let data = DragData::from_text("test");
        manager.prepare_drag(data, DropAction::COPY, Point::new(100.0, 100.0));

        assert!(manager.has_pending_drag());
        assert!(!manager.is_dragging());

        // Move less than threshold
        assert!(!manager.check_drag_start(Point::new(105.0, 100.0)));
        assert!(!manager.is_dragging());

        // Move past threshold
        assert!(manager.check_drag_start(Point::new(115.0, 100.0)));
        assert!(manager.is_dragging());
    }

    #[test]
    fn test_target_change() {
        let mut manager = DragDropManager::new();
        let data = DragData::from_text("test");
        manager.start_drag(data, DropAction::COPY, Point::new(100.0, 100.0));

        // Initially no target
        assert_eq!(manager.current_target(), None);

        // Move with no target (stays None)
        let prev = manager.update_position(Point::new(110.0, 100.0), None);
        assert!(prev.is_none());
        assert_eq!(manager.current_target(), None);

        // Move to having no target again - no change
        let prev = manager.update_position(Point::new(200.0, 100.0), None);
        assert!(prev.is_none());
        assert_eq!(manager.current_target(), None);
    }

    #[test]
    fn test_cancel_drag() {
        let mut manager = DragDropManager::new();
        let data = DragData::from_text("test");
        manager.start_drag(data, DropAction::COPY, Point::new(100.0, 100.0));

        assert!(manager.is_dragging());

        let result = manager.end_drag(false);
        assert!(result.is_none());
        assert!(!manager.is_dragging());
    }
}
