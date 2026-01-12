//! Deferred task queue for idle processing.
//!
//! Tasks can be posted to run during idle time when no other events are pending.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

/// A unique identifier for a deferred task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Get the raw u64 value of this task ID.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Global counter for generating unique task IDs.
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

fn next_task_id() -> TaskId {
    TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
}

/// A boxed task closure.
type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

/// Internal task data.
struct TaskData {
    id: TaskId,
    task: BoxedTask,
}

/// Manages the deferred task queue.
#[allow(dead_code)]
pub struct TaskQueue {
    /// Pending tasks to execute.
    tasks: VecDeque<TaskData>,
    /// Maximum number of tasks to process per idle cycle.
    batch_size: usize,
}

#[allow(dead_code)]
impl TaskQueue {
    /// Create a new task queue.
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            batch_size: 10,
        }
    }

    /// Create a new task queue with a custom batch size.
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            tasks: VecDeque::new(),
            batch_size,
        }
    }

    /// Post a task to be executed during idle time.
    ///
    /// Returns the task ID that can be used to cancel the task.
    pub fn post<F>(&mut self, task: F) -> TaskId
    where
        F: FnOnce() + Send + 'static,
    {
        let id = next_task_id();
        self.tasks.push_back(TaskData {
            id,
            task: Box::new(task),
        });
        id
    }

    /// Cancel a pending task.
    ///
    /// Returns `true` if the task was found and cancelled.
    pub fn cancel(&mut self, id: TaskId) -> bool {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            self.tasks.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if there are any pending tasks.
    pub fn has_pending(&self) -> bool {
        !self.tasks.is_empty()
    }

    /// Get the number of pending tasks.
    pub fn pending_count(&self) -> usize {
        self.tasks.len()
    }

    /// Process up to `batch_size` tasks.
    ///
    /// Returns the number of tasks processed.
    pub fn process_batch(&mut self) -> usize {
        let count = self.tasks.len().min(self.batch_size);
        for _ in 0..count {
            if let Some(task_data) = self.tasks.pop_front() {
                (task_data.task)();
            }
        }
        count
    }

    /// Process all pending tasks.
    ///
    /// Returns the number of tasks processed.
    pub fn process_all(&mut self) -> usize {
        let count = self.tasks.len();
        while let Some(task_data) = self.tasks.pop_front() {
            (task_data.task)();
        }
        count
    }

    /// Set the batch size for idle processing.
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe wrapper around `TaskQueue` for use from the application.
pub(crate) struct SharedTaskQueue {
    inner: Mutex<TaskQueue>,
}

#[allow(dead_code)]
impl SharedTaskQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TaskQueue::new()),
        }
    }

    pub fn post<F>(&self, task: F) -> TaskId
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.lock().post(task)
    }

    pub fn cancel(&self, id: TaskId) -> bool {
        self.inner.lock().cancel(id)
    }

    pub fn has_pending(&self) -> bool {
        self.inner.lock().has_pending()
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().pending_count()
    }

    pub fn process_batch(&self) -> usize {
        self.inner.lock().process_batch()
    }

    pub fn process_all(&self) -> usize {
        self.inner.lock().process_all()
    }

    pub fn set_batch_size(&self, size: usize) {
        self.inner.lock().set_batch_size(size);
    }
}

impl Default for SharedTaskQueue {
    fn default() -> Self {
        Self::new()
    }
}
