//! Custom event types for the Horizon Lattice event loop.

use crate::timer::TimerId;

/// Priority levels for internal events.
/// Higher priority events are processed first within the same event loop iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EventPriority {
    /// Lowest priority - idle tasks, background work.
    Low = 0,
    /// Normal priority - most application events.
    Normal = 1,
    /// High priority - user input, timers.
    High = 2,
    /// Critical priority - system events, shutdown.
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Internal events dispatched through the Horizon Lattice event loop.
///
/// These events are sent via `EventLoopProxy` and processed by the `ApplicationHandler`.
#[derive(Debug, Clone)]
pub enum LatticeEvent {
    /// A timer has fired.
    Timer {
        /// The timer that fired.
        id: TimerId,
    },

    /// A queued signal invocation (for cross-thread signal delivery).
    QueuedSignal {
        /// Unique identifier for this queued invocation.
        invocation_id: u64,
    },

    /// Execute a deferred task from the idle queue.
    DeferredTask {
        /// Unique identifier for this task.
        task_id: u64,
    },

    /// Request to quit the application.
    Quit,

    /// Wake up the event loop (for polling changes).
    WakeUp,

    /// User-defined custom event.
    Custom {
        /// User-defined event kind identifier.
        kind: u32,
        /// Optional payload as raw bytes (for simple data).
        payload: Option<Box<[u8]>>,
    },
}

impl LatticeEvent {
    /// Get the priority of this event.
    pub fn priority(&self) -> EventPriority {
        match self {
            Self::Quit => EventPriority::Critical,
            Self::Timer { .. } => EventPriority::High,
            Self::QueuedSignal { .. } => EventPriority::High,
            Self::DeferredTask { .. } => EventPriority::Low,
            Self::WakeUp => EventPriority::Normal,
            Self::Custom { .. } => EventPriority::Normal,
        }
    }

    /// Create a custom event with a kind identifier.
    pub fn custom(kind: u32) -> Self {
        Self::Custom {
            kind,
            payload: None,
        }
    }

    /// Create a custom event with a kind and byte payload.
    pub fn custom_with_payload(kind: u32, payload: Vec<u8>) -> Self {
        Self::Custom {
            kind,
            payload: Some(payload.into_boxed_slice()),
        }
    }
}

/// A wrapper for prioritized events used in the internal queue.
#[derive(Debug)]
pub(crate) struct PrioritizedEvent {
    pub event: LatticeEvent,
    pub priority: EventPriority,
    /// Sequence number for stable ordering of same-priority events.
    pub sequence: u64,
}

impl PrioritizedEvent {
    pub fn new(event: LatticeEvent, sequence: u64) -> Self {
        let priority = event.priority();
        Self {
            event,
            priority,
            sequence,
        }
    }
}

impl PartialEq for PrioritizedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}

impl Eq for PrioritizedEvent {}

impl PartialOrd for PrioritizedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then lower sequence (older) first.
        // Note: BinaryHeap is a max-heap, so we want higher priority to be "greater".
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => {
                // For same priority, process older events first (lower sequence = greater).
                other.sequence.cmp(&self.sequence)
            }
            ord => ord,
        }
    }
}
