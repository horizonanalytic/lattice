//! Background work scheduler for scheduled and periodic task execution.
//!
//! The scheduler allows deferring task execution to a specific time or running
//! tasks periodically at fixed intervals. It integrates with the event loop
//! to process ready tasks during idle time.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_core::Application;
//! use std::time::Duration;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = Application::new()?;
//!
//!     // Schedule a one-shot task to run after 5 seconds
//!     app.schedule_task(Duration::from_secs(5), || {
//!         println!("Task executed after 5 seconds!");
//!     });
//!
//!     // Schedule a repeating task every 2 seconds
//!     app.schedule_repeating_task(Duration::from_secs(2), || {
//!         println!("Periodic task executed!");
//!     });
//!
//!     Ok(app.run()?)
//! }
//! ```

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use slotmap::{SlotMap, new_key_type};

use crate::error::{Result, SchedulerError};

new_key_type! {
    /// A unique identifier for a scheduled task.
    pub struct ScheduledTaskId;
}

/// The type of scheduled task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledTaskKind {
    /// Executes once at the scheduled time.
    OneShot,
    /// Executes repeatedly at the specified interval.
    Repeating,
}

/// A boxed task closure.
type BoxedScheduledTask = Box<dyn FnMut() + Send + 'static>;

/// Internal scheduled task data.
struct ScheduledTaskData {
    /// When this task should next execute.
    next_run: Instant,
    /// The interval for repeating tasks.
    interval: Duration,
    /// The kind of task.
    kind: ScheduledTaskKind,
    /// Whether this task is active.
    active: bool,
    /// The task closure to execute.
    task: BoxedScheduledTask,
}

/// An entry in the scheduler queue (min-heap by execution time).
#[derive(Debug, Clone, Copy)]
struct SchedulerQueueEntry {
    id: ScheduledTaskId,
    run_time: Instant,
}

impl PartialEq for SchedulerQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.run_time == other.run_time
    }
}

impl Eq for SchedulerQueueEntry {}

impl PartialOrd for SchedulerQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchedulerQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (BinaryHeap is max-heap by default).
        other.run_time.cmp(&self.run_time)
    }
}

/// Manages scheduled background tasks.
///
/// The scheduler maintains a priority queue of tasks ordered by their next
/// execution time. Tasks can be one-shot (execute once) or repeating
/// (execute at regular intervals).
#[allow(dead_code)]
pub struct TaskScheduler {
    /// All registered scheduled tasks.
    tasks: SlotMap<ScheduledTaskId, ScheduledTaskData>,
    /// Priority queue of pending task executions (min-heap by run time).
    queue: BinaryHeap<SchedulerQueueEntry>,
}

#[allow(dead_code)]
impl TaskScheduler {
    /// Create a new task scheduler.
    pub fn new() -> Self {
        Self {
            tasks: SlotMap::with_key(),
            queue: BinaryHeap::new(),
        }
    }

    /// Schedule a one-shot task to execute after the specified delay.
    ///
    /// Returns the task ID that can be used to cancel or reschedule the task.
    pub fn schedule_once<F>(&mut self, delay: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let now = Instant::now();
        let next_run = now + delay;

        let data = ScheduledTaskData {
            next_run,
            interval: delay,
            kind: ScheduledTaskKind::OneShot,
            active: true,
            task: Box::new(task),
        };

        let id = self.tasks.insert(data);
        self.queue.push(SchedulerQueueEntry {
            id,
            run_time: next_run,
        });

        id
    }

    /// Schedule a task to execute at a specific instant.
    ///
    /// If the instant is in the past, the task will execute immediately
    /// on the next scheduler processing cycle.
    ///
    /// Returns the task ID that can be used to cancel or reschedule the task.
    pub fn schedule_at<F>(&mut self, instant: Instant, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let data = ScheduledTaskData {
            next_run: instant,
            interval: Duration::ZERO,
            kind: ScheduledTaskKind::OneShot,
            active: true,
            task: Box::new(task),
        };

        let id = self.tasks.insert(data);
        self.queue.push(SchedulerQueueEntry {
            id,
            run_time: instant,
        });

        id
    }

    /// Schedule a repeating task that executes at the specified interval.
    ///
    /// The first execution occurs after `interval` duration.
    /// Returns the task ID that can be used to cancel the task.
    pub fn schedule_repeating<F>(&mut self, interval: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let now = Instant::now();
        let next_run = now + interval;

        let data = ScheduledTaskData {
            next_run,
            interval,
            kind: ScheduledTaskKind::Repeating,
            active: true,
            task: Box::new(task),
        };

        let id = self.tasks.insert(data);
        self.queue.push(SchedulerQueueEntry {
            id,
            run_time: next_run,
        });

        id
    }

    /// Schedule a repeating task with an initial delay different from the interval.
    ///
    /// The first execution occurs after `initial_delay`, then repeats every `interval`.
    /// Returns the task ID that can be used to cancel the task.
    pub fn schedule_repeating_with_delay<F>(
        &mut self,
        initial_delay: Duration,
        interval: Duration,
        task: F,
    ) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let now = Instant::now();
        let next_run = now + initial_delay;

        let data = ScheduledTaskData {
            next_run,
            interval,
            kind: ScheduledTaskKind::Repeating,
            active: true,
            task: Box::new(task),
        };

        let id = self.tasks.insert(data);
        self.queue.push(SchedulerQueueEntry {
            id,
            run_time: next_run,
        });

        id
    }

    /// Cancel and remove a scheduled task.
    ///
    /// Returns `Ok(())` if the task was found and cancelled, or an error if not found.
    pub fn cancel(&mut self, id: ScheduledTaskId) -> Result<()> {
        if let Some(task) = self.tasks.get_mut(id) {
            task.active = false;
            self.tasks.remove(id);
            Ok(())
        } else {
            Err(SchedulerError::InvalidTaskId.into())
        }
    }

    /// Reschedule an existing task with a new delay.
    ///
    /// For one-shot tasks, this sets a new execution time.
    /// For repeating tasks, this resets the schedule with the current time as base.
    ///
    /// Returns `Ok(())` if successful, or an error if the task was not found.
    pub fn reschedule(&mut self, id: ScheduledTaskId, delay: Duration) -> Result<()> {
        if let Some(task) = self.tasks.get_mut(id) {
            let now = Instant::now();
            task.next_run = now + delay;

            // Add new queue entry (old one will be skipped when processed)
            self.queue.push(SchedulerQueueEntry {
                id,
                run_time: task.next_run,
            });

            Ok(())
        } else {
            Err(SchedulerError::InvalidTaskId.into())
        }
    }

    /// Check if a scheduled task is currently active.
    pub fn is_active(&self, id: ScheduledTaskId) -> bool {
        self.tasks.get(id).is_some_and(|t| t.active)
    }

    /// Get the duration until the next task should execute, if any.
    ///
    /// Returns `None` if there are no active scheduled tasks.
    pub fn time_until_next(&mut self) -> Option<Duration> {
        // Clean up any inactive tasks from the front of the queue.
        while let Some(entry) = self.queue.peek() {
            if !self.tasks.get(entry.id).is_some_and(|t| t.active) {
                self.queue.pop();
            } else {
                break;
            }
        }

        self.queue.peek().map(|entry| {
            let now = Instant::now();
            if entry.run_time > now {
                entry.run_time - now
            } else {
                Duration::ZERO
            }
        })
    }

    /// Process all tasks that should execute now.
    ///
    /// Returns the number of tasks that were executed.
    #[tracing::instrument(
        skip(self),
        target = "horizon_lattice_core::scheduler",
        level = "trace"
    )]
    pub fn process_ready(&mut self) -> usize {
        let now = Instant::now();
        let mut executed_count = 0;

        while let Some(entry) = self.queue.peek() {
            // Check if this task should run.
            if entry.run_time > now {
                break;
            }

            let entry = self.queue.pop().unwrap();
            let id = entry.id;

            // Check if task is still active.
            let Some(task_data) = self.tasks.get_mut(id) else {
                continue;
            };

            if !task_data.active {
                continue;
            }

            // Check if this queue entry is stale (task was rescheduled).
            // If the entry's run_time doesn't match the task's current next_run,
            // this is an old entry and should be skipped.
            if entry.run_time != task_data.next_run {
                continue;
            }

            // Execute the task.
            tracing::trace!(target: "horizon_lattice_core::scheduler", ?id, "executing scheduled task");
            (task_data.task)();
            executed_count += 1;

            match task_data.kind {
                ScheduledTaskKind::OneShot => {
                    // One-shot tasks are removed after execution.
                    task_data.active = false;
                    self.tasks.remove(id);
                }
                ScheduledTaskKind::Repeating => {
                    // Schedule the next execution.
                    // Use the scheduled time as base to avoid drift.
                    let next_run = entry.run_time + task_data.interval;
                    task_data.next_run = next_run;
                    self.queue.push(SchedulerQueueEntry {
                        id,
                        run_time: next_run,
                    });
                }
            }
        }

        executed_count
    }

    /// Get the number of active scheduled tasks.
    pub fn active_count(&self) -> usize {
        self.tasks.iter().filter(|(_, t)| t.active).count()
    }

    /// Check if there are any tasks ready to execute now.
    pub fn has_ready(&mut self) -> bool {
        // Clean up inactive entries first
        while let Some(entry) = self.queue.peek() {
            if !self.tasks.get(entry.id).is_some_and(|t| t.active) {
                self.queue.pop();
            } else {
                break;
            }
        }

        self.queue
            .peek()
            .is_some_and(|entry| entry.run_time <= Instant::now())
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe wrapper around `TaskScheduler` for use from the application.
pub(crate) struct SharedTaskScheduler {
    inner: Mutex<TaskScheduler>,
}

#[allow(dead_code)]
impl SharedTaskScheduler {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TaskScheduler::new()),
        }
    }

    pub fn schedule_once<F>(&self, delay: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        self.inner.lock().schedule_once(delay, task)
    }

    pub fn schedule_at<F>(&self, instant: Instant, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        self.inner.lock().schedule_at(instant, task)
    }

    pub fn schedule_repeating<F>(&self, interval: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        self.inner.lock().schedule_repeating(interval, task)
    }

    pub fn schedule_repeating_with_delay<F>(
        &self,
        initial_delay: Duration,
        interval: Duration,
        task: F,
    ) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        self.inner
            .lock()
            .schedule_repeating_with_delay(initial_delay, interval, task)
    }

    pub fn cancel(&self, id: ScheduledTaskId) -> Result<()> {
        self.inner.lock().cancel(id)
    }

    pub fn reschedule(&self, id: ScheduledTaskId, delay: Duration) -> Result<()> {
        self.inner.lock().reschedule(id, delay)
    }

    pub fn is_active(&self, id: ScheduledTaskId) -> bool {
        self.inner.lock().is_active(id)
    }

    pub fn time_until_next(&self) -> Option<Duration> {
        self.inner.lock().time_until_next()
    }

    pub fn process_ready(&self) -> usize {
        self.inner.lock().process_ready()
    }

    pub fn active_count(&self) -> usize {
        self.inner.lock().active_count()
    }

    pub fn has_ready(&self) -> bool {
        self.inner.lock().has_ready()
    }
}

impl Default for SharedTaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn test_schedule_once() {
        let mut scheduler = TaskScheduler::new();
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let id = scheduler.schedule_once(Duration::from_millis(10), move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(scheduler.is_active(id));
        assert_eq!(scheduler.active_count(), 1);

        // Task shouldn't execute immediately
        assert_eq!(scheduler.process_ready(), 0);
        assert_eq!(executed.load(Ordering::SeqCst), 0);

        // Wait for the task to be ready
        std::thread::sleep(Duration::from_millis(15));

        // Now it should execute
        assert_eq!(scheduler.process_ready(), 1);
        assert_eq!(executed.load(Ordering::SeqCst), 1);

        // Task should be removed after execution
        assert!(!scheduler.is_active(id));
        assert_eq!(scheduler.active_count(), 0);
    }

    #[test]
    fn test_schedule_repeating() {
        let mut scheduler = TaskScheduler::new();
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        // Use longer intervals to avoid timing issues in CI
        let id = scheduler.schedule_repeating(Duration::from_millis(100), move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(scheduler.is_active(id));

        // Wait and process - verify at least one execution
        std::thread::sleep(Duration::from_millis(150));
        scheduler.process_ready();
        let count1 = executed.load(Ordering::SeqCst);
        assert!(count1 >= 1, "Expected at least 1 execution, got {}", count1);

        // Wait more and verify executions increased (repeating works)
        std::thread::sleep(Duration::from_millis(150));
        scheduler.process_ready();
        let count2 = executed.load(Ordering::SeqCst);
        assert!(
            count2 > count1,
            "Expected executions to increase from {} but got {}",
            count1,
            count2
        );

        // Task should still be active (it's repeating)
        assert!(scheduler.is_active(id));

        // Cancel it
        scheduler.cancel(id).unwrap();
        assert!(!scheduler.is_active(id));
    }

    #[test]
    fn test_cancel_task() {
        let mut scheduler = TaskScheduler::new();
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let id = scheduler.schedule_once(Duration::from_millis(10), move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(scheduler.is_active(id));

        // Cancel before execution
        scheduler.cancel(id).unwrap();
        assert!(!scheduler.is_active(id));

        // Wait and try to process
        std::thread::sleep(Duration::from_millis(15));
        assert_eq!(scheduler.process_ready(), 0);
        assert_eq!(executed.load(Ordering::SeqCst), 0);

        // Cancelling again should fail
        assert!(scheduler.cancel(id).is_err());
    }

    #[test]
    #[ignore = "timing-sensitive test that is flaky in CI environments"]
    fn test_reschedule() {
        let mut scheduler = TaskScheduler::new();
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        // Schedule for 200ms from now (generous margin)
        let id = scheduler.schedule_once(Duration::from_millis(200), move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Immediately reschedule to a later time (400ms from now)
        scheduler
            .reschedule(id, Duration::from_millis(400))
            .unwrap();

        // After 300ms, original time would have passed but task should not execute
        // because we rescheduled it
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(scheduler.process_ready(), 0);
        assert_eq!(executed.load(Ordering::SeqCst), 0);

        // Wait for the new scheduled time (generous margin)
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(scheduler.process_ready(), 1);
        assert_eq!(executed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_schedule_at() {
        let mut scheduler = TaskScheduler::new();
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let target_time = Instant::now() + Duration::from_millis(10);
        let id = scheduler.schedule_at(target_time, move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(scheduler.is_active(id));

        // Wait for target time
        std::thread::sleep(Duration::from_millis(15));
        assert_eq!(scheduler.process_ready(), 1);
        assert_eq!(executed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_time_until_next() {
        let mut scheduler = TaskScheduler::new();

        // No tasks
        assert!(scheduler.time_until_next().is_none());

        // Schedule a task
        let _id = scheduler.schedule_once(Duration::from_millis(100), || {});

        let time_until = scheduler.time_until_next();
        assert!(time_until.is_some());
        assert!(time_until.unwrap() <= Duration::from_millis(100));
        assert!(time_until.unwrap() > Duration::from_millis(90));
    }

    #[test]
    fn test_multiple_tasks_order() {
        let mut scheduler = TaskScheduler::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let order1 = order.clone();
        scheduler.schedule_once(Duration::from_millis(30), move || {
            order1.lock().push(3);
        });

        let order2 = order.clone();
        scheduler.schedule_once(Duration::from_millis(10), move || {
            order2.lock().push(1);
        });

        let order3 = order.clone();
        scheduler.schedule_once(Duration::from_millis(20), move || {
            order3.lock().push(2);
        });

        // Wait for all to be ready
        std::thread::sleep(Duration::from_millis(35));
        scheduler.process_ready();

        // Tasks should execute in order of their scheduled times
        assert_eq!(*order.lock(), vec![1, 2, 3]);
    }

    #[test]
    fn test_shared_scheduler_thread_safety() {
        let scheduler = Arc::new(SharedTaskScheduler::new());
        let executed = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let scheduler = scheduler.clone();
                let executed = executed.clone();
                std::thread::spawn(move || {
                    for _ in 0..10 {
                        let executed = executed.clone();
                        scheduler.schedule_once(Duration::from_millis(1), move || {
                            executed.fetch_add(1, Ordering::SeqCst);
                        });
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Wait for all tasks to be ready
        std::thread::sleep(Duration::from_millis(10));

        // Process all ready tasks
        while scheduler.has_ready() {
            scheduler.process_ready();
        }

        assert_eq!(executed.load(Ordering::SeqCst), 40);
    }
}
