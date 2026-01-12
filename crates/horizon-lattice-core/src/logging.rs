//! Logging and debugging facilities for Horizon Lattice.
//!
//! This module provides:
//! - Integration with the `tracing` crate for structured logging
//! - Debug visualization for object trees
//! - Performance tracing hooks for profiling
//!
//! # Tracing Integration
//!
//! Horizon Lattice uses the `tracing` crate for instrumentation. To see logs,
//! you need to install a tracing subscriber in your application:
//!
//! ```ignore
//! use tracing_subscriber;
//!
//! fn main() {
//!     // Initialize tracing (you can customize this)
//!     tracing_subscriber::fmt::init();
//!
//!     // Your application code...
//! }
//! ```
//!
//! # Debug Visualization
//!
//! Use [`ObjectTreeDebug`] to get detailed views of the object hierarchy:
//!
//! ```ignore
//! use horizon_lattice_core::logging::ObjectTreeDebug;
//!
//! let debug = ObjectTreeDebug::new();
//! println!("{}", debug.format_tree());
//! ```

use std::fmt::{self, Write as FmtWrite};

use crate::object::{global_registry, ObjectId, ObjectResult};

/// Span names used throughout Horizon Lattice for tracing.
///
/// These constants can be used to filter traces for specific subsystems.
pub mod span_names {
    /// Event loop processing span.
    pub const EVENT_LOOP: &str = "horizon_lattice::event_loop";
    /// Timer processing span.
    pub const TIMER: &str = "horizon_lattice::timer";
    /// Signal emission span.
    pub const SIGNAL: &str = "horizon_lattice::signal";
    /// Property change span.
    pub const PROPERTY: &str = "horizon_lattice::property";
    /// Object lifecycle span.
    pub const OBJECT: &str = "horizon_lattice::object";
    /// Task queue processing span.
    pub const TASK: &str = "horizon_lattice::task";
}

/// Target names for log filtering.
///
/// Use these with `tracing` directives to filter logs by subsystem.
pub mod targets {
    /// Core framework target.
    pub const CORE: &str = "horizon_lattice_core";
    /// Event loop target.
    pub const EVENT_LOOP: &str = "horizon_lattice_core::event_loop";
    /// Timer system target.
    pub const TIMER: &str = "horizon_lattice_core::timer";
    /// Signal/slot system target.
    pub const SIGNAL: &str = "horizon_lattice_core::signal";
    /// Property system target.
    pub const PROPERTY: &str = "horizon_lattice_core::property";
    /// Object model target.
    pub const OBJECT: &str = "horizon_lattice_core::object";
}

/// Style options for object tree visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeStyle {
    /// ASCII characters for tree branches.
    Ascii,
    /// Unicode box-drawing characters.
    Unicode,
    /// Compact single-line representation.
    Compact,
}

impl Default for TreeStyle {
    fn default() -> Self {
        Self::Unicode
    }
}

/// Configuration for object tree debug output.
#[derive(Debug, Clone)]
pub struct TreeFormatOptions {
    /// The style of tree visualization.
    pub style: TreeStyle,
    /// Whether to show object IDs.
    pub show_ids: bool,
    /// Whether to show type names.
    pub show_types: bool,
    /// Whether to show dynamic property names.
    pub show_properties: bool,
    /// Maximum depth to traverse (None for unlimited).
    pub max_depth: Option<usize>,
    /// Indent size for each level.
    pub indent_size: usize,
}

impl Default for TreeFormatOptions {
    fn default() -> Self {
        Self {
            style: TreeStyle::default(),
            show_ids: true,
            show_types: true,
            show_properties: false,
            max_depth: None,
            indent_size: 2,
        }
    }
}

impl TreeFormatOptions {
    /// Create options for detailed debugging output.
    pub fn detailed() -> Self {
        Self {
            show_properties: true,
            ..Default::default()
        }
    }

    /// Create options for minimal output.
    pub fn minimal() -> Self {
        Self {
            show_ids: false,
            show_types: false,
            show_properties: false,
            ..Default::default()
        }
    }
}

/// Debug utility for visualizing object trees.
///
/// This provides various methods for inspecting and displaying the
/// object hierarchy in a human-readable format.
#[derive(Debug, Clone)]
pub struct ObjectTreeDebug {
    options: TreeFormatOptions,
}

impl ObjectTreeDebug {
    /// Create a new debug visualizer with default options.
    pub fn new() -> Self {
        Self {
            options: TreeFormatOptions::default(),
        }
    }

    /// Create a debug visualizer with custom options.
    pub fn with_options(options: TreeFormatOptions) -> Self {
        Self { options }
    }

    /// Format the entire object tree starting from all root objects.
    pub fn format_all(&self) -> ObjectResult<String> {
        let registry = global_registry()?;
        let roots = registry.root_objects();

        let mut output = String::new();
        writeln!(output, "Object Tree ({} total objects):", registry.object_count())
            .expect("write to String");

        if roots.is_empty() {
            writeln!(output, "  (empty)").expect("write to String");
        } else {
            for root_id in roots {
                self.format_subtree_into(root_id, 0, true, &mut output)?;
            }
        }

        Ok(output)
    }

    /// Format a subtree starting from a specific object.
    pub fn format_subtree(&self, root: ObjectId) -> ObjectResult<String> {
        let mut output = String::new();
        self.format_subtree_into(root, 0, true, &mut output)?;
        Ok(output)
    }

    /// Format a subtree into an existing string buffer.
    fn format_subtree_into(
        &self,
        id: ObjectId,
        depth: usize,
        is_last: bool,
        output: &mut String,
    ) -> ObjectResult<()> {
        // Check max depth
        if let Some(max) = self.options.max_depth {
            if depth > max {
                return Ok(());
            }
        }

        let registry = global_registry()?;
        let name = registry.object_name(id)?;
        let type_name = registry.type_name(id)?;
        let children = registry.children(id)?;

        // Build the prefix based on style and depth
        let prefix = self.build_prefix(depth, is_last);
        output.push_str(&prefix);

        // Object name
        let display_name = if name.is_empty() {
            "(unnamed)"
        } else {
            &name
        };
        output.push_str(display_name);

        // Optional ID
        if self.options.show_ids {
            write!(output, " [{:?}]", id).expect("write to String");
        }

        // Optional type
        if self.options.show_types {
            // Extract just the type name without the full path for readability
            let short_type = type_name.rsplit("::").next().unwrap_or(type_name);
            write!(output, " ({})", short_type).expect("write to String");
        }

        output.push('\n');

        // Optional properties
        if self.options.show_properties {
            let prop_names: Vec<String> = registry.with_read(|r| {
                r.dynamic_property_names(id)
                    .map(|names| names.into_iter().map(|s| s.to_string()).collect())
            })?;
            if !prop_names.is_empty() {
                let prop_prefix = self.build_property_prefix(depth, is_last);
                for prop_name in prop_names {
                    writeln!(output, "{}  .{}", prop_prefix, prop_name).expect("write to String");
                }
            }
        }

        // Recursively format children
        let child_count = children.len();
        for (i, child_id) in children.into_iter().enumerate() {
            let child_is_last = i == child_count - 1;
            self.format_subtree_into(child_id, depth + 1, child_is_last, output)?;
        }

        Ok(())
    }

    /// Build the prefix string for a tree node.
    fn build_prefix(&self, depth: usize, is_last: bool) -> String {
        if depth == 0 {
            return String::new();
        }

        let (branch, corner, space) = match self.options.style {
            TreeStyle::Ascii => ("|", "+--", "   "),
            TreeStyle::Unicode => ("\u{2502}", "\u{251c}\u{2500}\u{2500}", "\u{2514}\u{2500}\u{2500}"),
            TreeStyle::Compact => ("", "- ", "- "),
        };

        let mut prefix = String::new();

        // Add indentation for parent levels
        for _ in 0..(depth - 1) {
            prefix.push_str(branch);
            for _ in 0..self.options.indent_size {
                prefix.push(' ');
            }
        }

        // Add the connector for this level
        if is_last {
            prefix.push_str(if self.options.style == TreeStyle::Unicode {
                "\u{2514}\u{2500}\u{2500} "
            } else {
                space
            });
        } else {
            prefix.push_str(corner);
            prefix.push(' ');
        }

        prefix
    }

    /// Build the prefix for property lines.
    fn build_property_prefix(&self, depth: usize, _is_last: bool) -> String {
        let (branch, _) = match self.options.style {
            TreeStyle::Ascii => ("|", "   "),
            TreeStyle::Unicode => ("\u{2502}", "   "),
            TreeStyle::Compact => ("", "  "),
        };

        let mut prefix = String::new();
        for _ in 0..depth {
            prefix.push_str(branch);
            for _ in 0..self.options.indent_size {
                prefix.push(' ');
            }
        }
        prefix
    }
}

impl Default for ObjectTreeDebug {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ObjectTreeDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.format_all() {
            Ok(output) => write!(f, "{}", output),
            Err(e) => write!(f, "Error formatting object tree: {}", e),
        }
    }
}

/// A guard that emits a tracing span when dropped.
///
/// This is useful for tracking the duration of operations.
#[derive(Debug)]
pub struct PerfSpan {
    #[allow(dead_code)]
    span: tracing::span::EnteredSpan,
}

impl PerfSpan {
    /// Create a new performance span.
    ///
    /// The span will be active until the guard is dropped.
    pub fn new(name: &'static str) -> Self {
        let span = tracing::info_span!(target: "horizon_lattice::perf", "perf", operation = name);
        Self {
            span: span.entered(),
        }
    }
}

/// Macros for common tracing patterns.
///
/// These are re-exported for convenience but are just wrappers around
/// the `tracing` crate macros with consistent target naming.
#[macro_export]
macro_rules! lattice_trace {
    ($($arg:tt)*) => {
        tracing::trace!(target: "horizon_lattice_core", $($arg)*)
    };
}

#[macro_export]
macro_rules! lattice_debug {
    ($($arg:tt)*) => {
        tracing::debug!(target: "horizon_lattice_core", $($arg)*)
    };
}

#[macro_export]
macro_rules! lattice_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "horizon_lattice_core", $($arg)*)
    };
}

#[macro_export]
macro_rules! lattice_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "horizon_lattice_core", $($arg)*)
    };
}

#[macro_export]
macro_rules! lattice_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "horizon_lattice_core", $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{init_global_registry, Object, ObjectBase};

    struct TestWidget {
        base: ObjectBase,
    }

    impl TestWidget {
        fn new(name: &str) -> Self {
            let widget = Self {
                base: ObjectBase::new::<Self>(),
            };
            widget.base.set_name(name);
            widget
        }
    }

    impl Object for TestWidget {
        fn object_id(&self) -> ObjectId {
            self.base.id()
        }
    }

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_tree_format_empty() {
        setup();
        let debug = ObjectTreeDebug::new();
        let output = debug.format_all().unwrap();
        assert!(output.contains("Object Tree"));
    }

    #[test]
    fn test_tree_format_single() {
        setup();
        let widget = TestWidget::new("root");

        let debug = ObjectTreeDebug::new();
        let output = debug.format_subtree(widget.object_id()).unwrap();

        assert!(output.contains("root"));
        assert!(output.contains("TestWidget"));
    }

    #[test]
    fn test_tree_format_hierarchy() {
        setup();
        let root = TestWidget::new("window");
        let child1 = TestWidget::new("button1");
        let child2 = TestWidget::new("button2");

        child1.base.set_parent(Some(root.object_id())).unwrap();
        child2.base.set_parent(Some(root.object_id())).unwrap();

        let debug = ObjectTreeDebug::new();
        let output = debug.format_subtree(root.object_id()).unwrap();

        assert!(output.contains("window"));
        assert!(output.contains("button1"));
        assert!(output.contains("button2"));
    }

    #[test]
    fn test_tree_format_minimal() {
        setup();
        let widget = TestWidget::new("test");

        let debug = ObjectTreeDebug::with_options(TreeFormatOptions::minimal());
        let output = debug.format_subtree(widget.object_id()).unwrap();

        assert!(output.contains("test"));
        assert!(!output.contains("TestWidget"));
        assert!(!output.contains("["));
    }

    #[test]
    fn test_perf_span() {
        setup();
        // Just ensure it compiles and doesn't panic
        let _span = PerfSpan::new("test_operation");
    }
}
