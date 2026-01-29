//! Accessibility manager for window-level accessibility tree management.

use std::sync::Arc;

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId,
    Role, Tree, TreeUpdate,
};
use accesskit_winit::Adapter;
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::Rect;
use parking_lot::Mutex;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::{node_id_to_object_id, object_id_to_node_id};

/// Callback for handling accessibility actions.
pub type AccessibilityActionCallback = Box<dyn Fn(ObjectId, ActionRequest) + Send + Sync>;

/// Manager for a window's accessibility tree.
///
/// Each window has its own `AccessibilityManager` that:
/// - Builds the accessibility tree from the widget hierarchy
/// - Handles action requests from assistive technologies
/// - Sends updates when widget state changes
///
/// # Example
///
/// ```ignore
/// // Create manager when window is created
/// let manager = AccessibilityManager::new(event_loop, &window, root_widget_id);
///
/// // When widget state changes, notify the manager
/// manager.update_if_active(|| build_tree_update());
///
/// // Handle actions by setting a callback
/// manager.set_action_callback(|widget_id, request| {
///     // Route action to widget
/// });
/// ```
pub struct AccessibilityManager {
    /// The AccessKit adapter for this window.
    adapter: Adapter,

    /// The root widget ID.
    root_id: ObjectId,

    /// Callback for handling accessibility actions.
    action_callback: Arc<Mutex<Option<AccessibilityActionCallback>>>,
}

impl AccessibilityManager {
    /// Create a new accessibility manager for a window.
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop
    /// * `window` - The winit window
    /// * `root_id` - The root widget's ObjectId
    ///
    /// # Important
    ///
    /// The window must have been created with `visible(false)` initially.
    /// After creating the AccessibilityManager, show the window.
    pub fn new(event_loop: &ActiveEventLoop, window: &Window, root_id: ObjectId) -> Self {
        let action_callback: Arc<Mutex<Option<AccessibilityActionCallback>>> =
            Arc::new(Mutex::new(None));
        let action_callback_clone = action_callback.clone();

        let activation_handler = ActivationHandlerImpl { root_id };
        let action_handler = ActionHandlerImpl {
            action_callback: action_callback_clone,
        };
        let deactivation_handler = DeactivationHandlerImpl;

        let adapter = Adapter::with_direct_handlers(
            event_loop,
            window,
            activation_handler,
            action_handler,
            deactivation_handler,
        );

        Self {
            adapter,
            root_id,
            action_callback,
        }
    }

    /// Set the callback for handling accessibility actions.
    ///
    /// The callback receives the target widget's ObjectId and the action request.
    /// It should route the action to the appropriate widget.
    pub fn set_action_callback<F>(&self, callback: F)
    where
        F: Fn(ObjectId, ActionRequest) + Send + Sync + 'static,
    {
        *self.action_callback.lock() = Some(Box::new(callback));
    }

    /// Clear the action callback.
    pub fn clear_action_callback(&self) {
        *self.action_callback.lock() = None;
    }

    /// Get the root widget ID.
    pub fn root_id(&self) -> ObjectId {
        self.root_id
    }

    /// Process a winit window event for accessibility.
    ///
    /// This should be called for each window event to allow AccessKit
    /// to handle platform-specific accessibility events.
    pub fn process_event(&mut self, window: &Window, event: &WinitWindowEvent) {
        self.adapter.process_event(window, event);
    }

    /// Send a tree update if accessibility is active.
    ///
    /// The updater closure is only called if assistive technology is connected.
    /// This notifies assistive technologies of changes to the widget tree.
    pub fn update_if_active<F>(&mut self, updater: F)
    where
        F: FnOnce() -> TreeUpdate,
    {
        self.adapter.update_if_active(updater);
    }

    /// Build a complete tree update from the widget hierarchy.
    ///
    /// This traverses the widget tree starting from the root and builds
    /// AccessKit nodes for each widget.
    ///
    /// # Arguments
    ///
    /// * `build_node` - Function that builds a node for a given widget ID
    /// * `focused_id` - The currently focused widget's ID
    pub fn build_full_tree<F>(&self, build_node: F, focused_id: Option<ObjectId>) -> TreeUpdate
    where
        F: Fn(ObjectId) -> Option<(Node, Vec<ObjectId>)>,
    {
        let mut nodes = Vec::new();
        let mut stack = vec![self.root_id];

        while let Some(id) = stack.pop() {
            if let Some((node, children)) = build_node(id) {
                nodes.push((object_id_to_node_id(id), node));
                // Add children to stack in reverse order for correct traversal
                for child_id in children.into_iter().rev() {
                    stack.push(child_id);
                }
            }
        }

        let focus = focused_id
            .map(object_id_to_node_id)
            .unwrap_or_else(|| object_id_to_node_id(self.root_id));

        TreeUpdate {
            nodes,
            tree: Some(Tree::new(object_id_to_node_id(self.root_id))),
            focus,
        }
    }

    /// Build an incremental update for a single node.
    ///
    /// Use this when a widget's properties change but the tree structure
    /// remains the same.
    pub fn build_node_update(&self, id: ObjectId, node: Node) -> TreeUpdate {
        TreeUpdate {
            nodes: vec![(object_id_to_node_id(id), node)],
            tree: None,
            focus: object_id_to_node_id(id),
        }
    }

    /// Build an update that only changes focus.
    pub fn build_focus_update(&self, focused_id: ObjectId) -> TreeUpdate {
        TreeUpdate {
            nodes: vec![],
            tree: None,
            focus: object_id_to_node_id(focused_id),
        }
    }

    /// Get the underlying AccessKit adapter for advanced usage.
    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    /// Get a mutable reference to the underlying AccessKit adapter.
    pub fn adapter_mut(&mut self) -> &mut Adapter {
        &mut self.adapter
    }
}

/// Internal action handler implementation.
struct ActionHandlerImpl {
    action_callback: Arc<Mutex<Option<AccessibilityActionCallback>>>,
}

impl ActionHandler for ActionHandlerImpl {
    fn do_action(&mut self, request: ActionRequest) {
        if let Some(object_id) = node_id_to_object_id(request.target)
            && let Some(ref callback) = *self.action_callback.lock()
        {
            callback(object_id, request);
        }
    }
}

/// Internal activation handler implementation.
struct ActivationHandlerImpl {
    root_id: ObjectId,
}

impl ActivationHandler for ActivationHandlerImpl {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // Build initial tree when accessibility is first activated
        // This will be called lazily when AT first connects
        let root_node_id = object_id_to_node_id(self.root_id);

        // Create a minimal root node - the full tree will be built
        // by the window's accessibility integration
        let mut root_node = Node::new(Role::Window);
        root_node.set_label("Application Window");

        Some(TreeUpdate {
            nodes: vec![(root_node_id, root_node)],
            tree: Some(Tree::new(root_node_id)),
            focus: root_node_id,
        })
    }
}

/// Internal deactivation handler implementation.
struct DeactivationHandlerImpl;

impl DeactivationHandler for DeactivationHandlerImpl {
    fn deactivate_accessibility(&mut self) {
        // Accessibility has been deactivated (AT disconnected)
        // No cleanup needed for our implementation
    }
}

/// Builder for constructing AccessKit nodes from widget data.
///
/// This helper simplifies building nodes with common patterns.
pub struct NodeBuilder {
    node: Node,
    children: Vec<NodeId>,
}

impl NodeBuilder {
    /// Create a new node builder with the given role.
    pub fn new(role: Role) -> Self {
        Self {
            node: Node::new(role),
            children: Vec::new(),
        }
    }

    /// Set the node's label (accessible name).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.node.set_label(label.into());
        self
    }

    /// Set the node's description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.node.set_description(description.into());
        self
    }

    /// Set the node's bounding rectangle.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.node.set_bounds(accesskit::Rect {
            x0: bounds.origin.x as f64,
            y0: bounds.origin.y as f64,
            x1: (bounds.origin.x + bounds.size.width) as f64,
            y1: (bounds.origin.y + bounds.size.height) as f64,
        });
        self
    }

    /// Add an action that this node supports.
    pub fn action(mut self, action: Action) -> Self {
        self.node.add_action(action);
        self
    }

    /// Add a child node ID.
    pub fn child(mut self, id: ObjectId) -> Self {
        self.children.push(object_id_to_node_id(id));
        self
    }

    /// Add multiple child node IDs.
    pub fn children(mut self, ids: impl IntoIterator<Item = ObjectId>) -> Self {
        for id in ids {
            self.children.push(object_id_to_node_id(id));
        }
        self
    }

    /// Mark the node as focusable.
    pub fn focusable(mut self) -> Self {
        self.node.add_action(Action::Focus);
        self
    }

    /// Mark the node as disabled.
    pub fn disabled(mut self) -> Self {
        self.node.set_disabled();
        self
    }

    /// Mark the node as hidden.
    pub fn hidden(mut self) -> Self {
        self.node.set_hidden();
        self
    }

    /// Build the final node.
    pub fn build(mut self) -> Node {
        if !self.children.is_empty() {
            self.node.set_children(self.children);
        }
        self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    #[test]
    fn test_node_builder() {
        init_global_registry();

        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let node = NodeBuilder::new(Role::Button)
            .label("Click me")
            .bounds(bounds)
            .focusable()
            .action(Action::Click)
            .build();

        // Node was built successfully
        assert!(node.role() == Role::Button);
    }
}
