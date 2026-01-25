//! The main Application struct and event loop.

use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::error::{LatticeError, Result};
use crate::event::{LatticeEvent, PrioritizedEvent};
use crate::invocation::invocation_registry;
use crate::object::init_global_registry;
use crate::scheduler::{ScheduledTaskId, SharedTaskScheduler};
use crate::task::{SharedTaskQueue, TaskId};
use crate::timer::{SharedTimerManager, TimerId};

/// Global application instance.
static APPLICATION: OnceLock<Application> = OnceLock::new();

/// The main application struct, managing the event loop and global state.
///
/// This is a singleton - only one `Application` can exist per process.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_core::Application;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let app = Application::new()?;
///     // Set up windows, connect signals, etc.
///     Ok(app.run()?)
/// }
/// ```
/// Type alias for window event handler callback.
///
/// The callback receives the window ID and window event, and returns whether
/// the event was handled.
pub type WindowEventHandler = Box<dyn Fn(WindowId, &WindowEvent) -> bool + Send + Sync>;

pub struct Application {
    /// The event loop proxy for sending events from other threads.
    proxy: EventLoopProxy<LatticeEvent>,
    /// Timer manager (thread-safe).
    timers: SharedTimerManager,
    /// Background task scheduler (thread-safe).
    scheduler: SharedTaskScheduler,
    /// Deferred task queue (thread-safe).
    tasks: SharedTaskQueue,
    /// Internal event queue with priorities.
    event_queue: Mutex<BinaryHeap<PrioritizedEvent>>,
    /// Sequence counter for event ordering.
    event_sequence: AtomicU64,
    /// Flag indicating the application should quit.
    should_quit: AtomicBool,
    /// User-provided event handler.
    event_handler: RwLock<Option<Box<dyn Fn(&LatticeEvent) + Send + Sync>>>,
    /// User-provided window event handler.
    ///
    /// This is called for raw window events (keyboard, mouse, etc.) before
    /// any default processing occurs.
    window_event_handler: RwLock<Option<WindowEventHandler>>,
    /// Current keyboard modifier state.
    ///
    /// This tracks the state of Shift, Control, Alt, and Meta keys globally.
    modifiers: Mutex<Modifiers>,
}

impl Application {
    /// Create a new application instance.
    ///
    /// This must be called from the main thread before any other Horizon Lattice
    /// operations. Only one `Application` can exist per process.
    ///
    /// # Errors
    ///
    /// Returns an error if an `Application` has already been initialized, or if
    /// the event loop could not be created.
    ///
    /// # Panics
    ///
    /// May panic on some platforms if not called from the main thread.
    pub fn new() -> Result<&'static Application> {
        // Initialize the global object registry.
        init_global_registry();

        // Create the event loop first to get the proxy.
        let event_loop: EventLoop<LatticeEvent> =
            EventLoop::with_user_event()
                .build()
                .map_err(|e| LatticeError::EventLoopCreation(e.to_string()))?;

        let proxy = event_loop.create_proxy();

        let app = Application {
            proxy,
            timers: SharedTimerManager::new(),
            scheduler: SharedTaskScheduler::new(),
            tasks: SharedTaskQueue::new(),
            event_queue: Mutex::new(BinaryHeap::new()),
            event_sequence: AtomicU64::new(0),
            should_quit: AtomicBool::new(false),
            event_handler: RwLock::new(None),
            window_event_handler: RwLock::new(None),
            modifiers: Mutex::new(Modifiers::default()),
        };

        // Try to set the global instance.
        APPLICATION
            .set(app)
            .map_err(|_| LatticeError::ApplicationAlreadyInitialized)?;

        // Store the event loop in thread-local storage for run().
        EVENT_LOOP.with(|cell| {
            *cell.borrow_mut() = Some(event_loop);
        });

        Ok(APPLICATION.get().unwrap())
    }

    /// Get the global application instance.
    ///
    /// # Panics
    ///
    /// Panics if `Application::new()` has not been called yet.
    pub fn instance() -> &'static Application {
        APPLICATION
            .get()
            .expect("Application not initialized. Call Application::new() first.")
    }

    /// Try to get the global application instance.
    ///
    /// Returns `None` if `Application::new()` has not been called yet.
    pub fn try_instance() -> Option<&'static Application> {
        APPLICATION.get()
    }

    /// Run the main event loop.
    ///
    /// This method takes ownership of the calling thread and runs until
    /// `quit()` is called. On most platforms, this will not return.
    ///
    /// # Errors
    ///
    /// Returns an error if the event loop has already been consumed or exited.
    #[tracing::instrument(skip(self), target = "horizon_lattice_core::event_loop", level = "debug")]
    pub fn run(&self) -> Result<()> {
        tracing::info!(target: "horizon_lattice_core::event_loop", "starting event loop");
        let event_loop = EVENT_LOOP.with(|cell| cell.borrow_mut().take());

        let Some(event_loop) = event_loop else {
            return Err(LatticeError::EventLoopExited);
        };

        let mut handler = AppHandler::new(self);

        event_loop
            .run_app(&mut handler)
            .map_err(|e| LatticeError::EventLoopCreation(e.to_string()))?;

        Ok(())
    }

    /// Request the application to quit.
    ///
    /// This sends a quit event to the event loop, which will cause `run()` to
    /// return on the next iteration. The quit is not immediate.
    pub fn quit(&self) {
        tracing::info!(target: "horizon_lattice_core::event_loop", "quit requested");
        self.should_quit.store(true, Ordering::SeqCst);
        // Wake up the event loop to process the quit.
        let _ = self.proxy.send_event(LatticeEvent::Quit);
    }

    /// Check if a quit has been requested.
    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::SeqCst)
    }

    /// Post a custom event to the event loop.
    ///
    /// This is thread-safe and can be called from any thread.
    pub fn post_event(&self, event: LatticeEvent) -> Result<()> {
        self.proxy
            .send_event(event)
            .map_err(|_| LatticeError::EventLoopExited)
    }

    /// Set a handler for custom events.
    ///
    /// The handler will be called for each `LatticeEvent` that is processed.
    pub fn set_event_handler<F>(&self, handler: F)
    where
        F: Fn(&LatticeEvent) + Send + Sync + 'static,
    {
        *self.event_handler.write() = Some(Box::new(handler));
    }

    /// Clear the event handler.
    pub fn clear_event_handler(&self) {
        *self.event_handler.write() = None;
    }

    /// Set a handler for window events (keyboard, mouse, etc.).
    ///
    /// The handler receives the window ID and raw window event, and should
    /// return `true` if it handled the event.
    ///
    /// This is called before any default processing of window events.
    ///
    /// # Example
    ///
    /// ```ignore
    /// app.set_window_event_handler(|window_id, event| {
    ///     match event {
    ///         WindowEvent::KeyboardInput { event, .. } => {
    ///             // Handle keyboard input
    ///             true
    ///         }
    ///         _ => false,
    ///     }
    /// });
    /// ```
    pub fn set_window_event_handler<F>(&self, handler: F)
    where
        F: Fn(WindowId, &WindowEvent) -> bool + Send + Sync + 'static,
    {
        *self.window_event_handler.write() = Some(Box::new(handler));
    }

    /// Clear the window event handler.
    pub fn clear_window_event_handler(&self) {
        *self.window_event_handler.write() = None;
    }

    /// Get the current keyboard modifier state.
    ///
    /// Returns the state of Shift, Control, Alt, and Meta keys.
    pub fn modifiers(&self) -> Modifiers {
        *self.modifiers.lock()
    }

    // -------------------------------------------------------------------------
    // Timer API
    // -------------------------------------------------------------------------

    /// Start a one-shot timer that fires after the specified duration.
    ///
    /// Returns a `TimerId` that can be used to cancel the timer.
    pub fn start_timer(&self, duration: Duration) -> TimerId {
        let id = self.timers.start_one_shot(duration);
        // Wake up the event loop to recalculate the next timer.
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Start a repeating timer that fires at the specified interval.
    ///
    /// Returns a `TimerId` that can be used to cancel the timer.
    pub fn start_repeating_timer(&self, interval: Duration) -> TimerId {
        let id = self.timers.start_repeating(interval);
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Stop a timer.
    pub fn stop_timer(&self, id: TimerId) -> Result<()> {
        self.timers.stop(id)
    }

    /// Check if a timer is active.
    pub fn is_timer_active(&self, id: TimerId) -> bool {
        self.timers.is_active(id)
    }

    // -------------------------------------------------------------------------
    // Task Queue API (Idle Processing)
    // -------------------------------------------------------------------------

    /// Post a task to be executed during idle time.
    ///
    /// Returns a `TaskId` that can be used to cancel the task.
    pub fn post_task<F>(&self, task: F) -> TaskId
    where
        F: FnOnce() + Send + 'static,
    {
        let id = self.tasks.post(task);
        // Wake up the event loop if it's waiting.
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Cancel a pending task.
    ///
    /// Returns `true` if the task was found and cancelled.
    pub fn cancel_task(&self, id: TaskId) -> bool {
        self.tasks.cancel(id)
    }

    // -------------------------------------------------------------------------
    // Scheduler API (Background Work Scheduling)
    // -------------------------------------------------------------------------

    /// Schedule a one-shot task to execute after the specified delay.
    ///
    /// The task will be executed once during the event loop's idle processing
    /// after the delay has elapsed.
    ///
    /// Returns a `ScheduledTaskId` that can be used to cancel or reschedule the task.
    ///
    /// # Example
    ///
    /// ```ignore
    /// app.schedule_task(Duration::from_secs(5), || {
    ///     println!("Executed after 5 seconds!");
    /// });
    /// ```
    pub fn schedule_task<F>(&self, delay: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let id = self.scheduler.schedule_once(delay, task);
        // Wake up the event loop to recalculate timing.
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Schedule a task to execute at a specific instant.
    ///
    /// If the instant is in the past, the task will execute immediately
    /// on the next scheduler processing cycle.
    ///
    /// Returns a `ScheduledTaskId` that can be used to cancel or reschedule the task.
    pub fn schedule_task_at<F>(&self, instant: std::time::Instant, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let id = self.scheduler.schedule_at(instant, task);
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Schedule a repeating task that executes at the specified interval.
    ///
    /// The first execution occurs after `interval` duration, then repeats
    /// every `interval` thereafter until cancelled.
    ///
    /// Returns a `ScheduledTaskId` that can be used to cancel the task.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let task_id = app.schedule_repeating_task(Duration::from_secs(1), || {
    ///     println!("Executed every second!");
    /// });
    ///
    /// // Later, cancel the task
    /// app.cancel_scheduled_task(task_id);
    /// ```
    pub fn schedule_repeating_task<F>(&self, interval: Duration, task: F) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let id = self.scheduler.schedule_repeating(interval, task);
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Schedule a repeating task with an initial delay different from the interval.
    ///
    /// The first execution occurs after `initial_delay`, then repeats every `interval`.
    ///
    /// Returns a `ScheduledTaskId` that can be used to cancel the task.
    pub fn schedule_repeating_task_with_delay<F>(
        &self,
        initial_delay: Duration,
        interval: Duration,
        task: F,
    ) -> ScheduledTaskId
    where
        F: FnMut() + Send + 'static,
    {
        let id = self
            .scheduler
            .schedule_repeating_with_delay(initial_delay, interval, task);
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        id
    }

    /// Cancel a scheduled task.
    ///
    /// Returns `Ok(())` if the task was found and cancelled.
    pub fn cancel_scheduled_task(&self, id: ScheduledTaskId) -> Result<()> {
        self.scheduler.cancel(id)
    }

    /// Reschedule an existing task with a new delay.
    ///
    /// The task's next execution will be reset to `delay` from now.
    ///
    /// Returns `Ok(())` if successful.
    pub fn reschedule_task(&self, id: ScheduledTaskId, delay: Duration) -> Result<()> {
        self.scheduler.reschedule(id, delay)?;
        let _ = self.proxy.send_event(LatticeEvent::WakeUp);
        Ok(())
    }

    /// Check if a scheduled task is active.
    pub fn is_scheduled_task_active(&self, id: ScheduledTaskId) -> bool {
        self.scheduler.is_active(id)
    }

    // -------------------------------------------------------------------------
    // Internal methods
    // -------------------------------------------------------------------------

    /// Queue an internal event with priority.
    fn queue_event(&self, event: LatticeEvent) {
        let sequence = self.event_sequence.fetch_add(1, Ordering::Relaxed);
        let prioritized = PrioritizedEvent::new(event, sequence);
        self.event_queue.lock().push(prioritized);
    }

    /// Process queued internal events.
    fn process_queued_events(&self) {
        let handler = self.event_handler.read();

        loop {
            let event = {
                let mut queue = self.event_queue.lock();
                queue.pop()
            };

            let Some(prioritized) = event else {
                break;
            };

            // Dispatch to user handler if set.
            if let Some(ref h) = *handler {
                h(&prioritized.event);
            }
        }
    }

    /// Get time until next timer, for setting ControlFlow.
    fn time_until_next_timer(&self) -> Option<Duration> {
        self.timers.time_until_next()
    }

    /// Process expired timers and return events.
    fn process_timers(&self) -> Vec<LatticeEvent> {
        self.timers.process_expired()
    }

    /// Get time until next scheduled task, for setting ControlFlow.
    fn time_until_next_scheduled(&self) -> Option<Duration> {
        self.scheduler.time_until_next()
    }

    /// Process ready scheduled tasks.
    fn process_scheduled_tasks(&self) -> usize {
        self.scheduler.process_ready()
    }

    /// Check if there are scheduled tasks ready to run.
    fn has_ready_scheduled_tasks(&self) -> bool {
        self.scheduler.has_ready()
    }

    /// Process pending idle tasks.
    fn process_idle_tasks(&self) -> usize {
        self.tasks.process_batch()
    }

    /// Check if there are pending idle tasks.
    fn has_pending_tasks(&self) -> bool {
        self.tasks.has_pending()
    }
}

// Thread-local storage for the event loop (needed because EventLoop cannot be stored in static).
thread_local! {
    static EVENT_LOOP: std::cell::RefCell<Option<EventLoop<LatticeEvent>>> =
        const { std::cell::RefCell::new(None) };
}

/// Internal handler that implements winit's ApplicationHandler.
struct AppHandler<'a> {
    app: &'a Application,
}

impl<'a> AppHandler<'a> {
    fn new(app: &'a Application) -> Self {
        Self { app }
    }

    fn update_control_flow(&self, event_loop: &ActiveEventLoop) {
        if self.app.should_quit() {
            event_loop.exit();
            return;
        }

        // Determine the appropriate control flow based on pending work.
        let control_flow = if self.app.has_pending_tasks() || self.app.has_ready_scheduled_tasks() {
            // We have idle tasks or scheduled tasks ready, so poll to process them.
            ControlFlow::Poll
        } else {
            // Calculate wait duration based on timers and scheduled tasks.
            let timer_wait = self.app.time_until_next_timer();
            let scheduled_wait = self.app.time_until_next_scheduled();

            match (timer_wait, scheduled_wait) {
                (Some(t), Some(s)) => ControlFlow::wait_duration(t.min(s)),
                (Some(t), None) => ControlFlow::wait_duration(t),
                (None, Some(s)) => ControlFlow::wait_duration(s),
                (None, None) => ControlFlow::Wait,
            }
        };

        event_loop.set_control_flow(control_flow);
    }
}

impl ApplicationHandler<LatticeEvent> for AppHandler<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Application resumed (on mobile) or started (on desktop).
        // Windows should be created here.
        self.update_control_flow(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Update modifier state for keyboard input
        if let WindowEvent::ModifiersChanged(modifiers) = &event {
            *self.app.modifiers.lock() = *modifiers;
        }

        // First, try the user-provided window event handler
        let handled = {
            let handler = self.app.window_event_handler.read();
            if let Some(ref h) = *handler {
                h(window_id, &event)
            } else {
                false
            }
        };

        // If not handled by user, do default processing
        if !handled {
            match event {
                WindowEvent::CloseRequested => {
                    // Default behavior: quit the application.
                    //
                    // For proper close handling with the widget system, set a
                    // window event handler that routes CloseRequested to your
                    // Window widget's close() method:
                    //
                    // ```ignore
                    // app.set_window_event_handler(|window_id, event| {
                    //     if let WindowEvent::CloseRequested = event {
                    //         // Find your Window widget for this window_id
                    //         // Call window.close() - returns false if vetoed
                    //         // Only quit if close() succeeded and it was the last window
                    //         return true; // Handled
                    //     }
                    //     false
                    // });
                    // ```
                    self.app.quit();
                }
                WindowEvent::RedrawRequested => {
                    // Rendering will be handled by the graphics backend.
                }
                _ => {}
            }
        }

        self.update_control_flow(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: LatticeEvent) {
        tracing::trace!(target: "horizon_lattice_core::event_loop", ?event, "received user event");
        match &event {
            LatticeEvent::Quit => {
                tracing::debug!(target: "horizon_lattice_core::event_loop", "processing quit event");
                event_loop.exit();
                return;
            }
            LatticeEvent::Timer { .. } => {
                // Dispatch to user handler.
                if let Some(ref handler) = *self.app.event_handler.read() {
                    handler(&event);
                }
            }
            LatticeEvent::QueuedSignal { invocation_id } => {
                // Execute the queued signal invocation.
                tracing::trace!(
                    target: "horizon_lattice_core::event_loop",
                    invocation_id,
                    "executing queued signal invocation"
                );
                if let Some(invocation) = invocation_registry().take(*invocation_id) {
                    invocation.execute();
                } else {
                    tracing::warn!(
                        target: "horizon_lattice_core::event_loop",
                        invocation_id,
                        "queued signal invocation not found (already executed or cancelled)"
                    );
                }
            }
            LatticeEvent::WakeUp => {
                // Just wake up, control flow will be updated below.
            }
            _ => {
                // Queue for priority-based processing.
                self.app.queue_event(event);
            }
        }

        // Process any queued events.
        self.app.process_queued_events();

        self.update_control_flow(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Process expired timers.
        let timer_events = self.app.process_timers();
        if !timer_events.is_empty() {
            tracing::trace!(target: "horizon_lattice_core::event_loop", count = timer_events.len(), "processing timer events");
        }
        for event in timer_events {
            if let Some(ref handler) = *self.app.event_handler.read() {
                handler(&event);
            }
        }

        // Process ready scheduled tasks.
        let scheduled_count = self.app.process_scheduled_tasks();
        if scheduled_count > 0 {
            tracing::trace!(target: "horizon_lattice_core::event_loop", count = scheduled_count, "processed scheduled tasks");
        }

        // Process idle tasks.
        if self.app.has_pending_tasks() {
            self.app.process_idle_tasks();
        }

        self.update_control_flow(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Clean up when the event loop is about to exit.
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    // Note: Most Application tests require running without the event loop
    // since tests can't actually run the GUI event loop.

    #[test]
    fn test_timer_manager_basic() {
        let manager = SharedTimerManager::new();

        let id = manager.start_one_shot(Duration::from_millis(100));
        assert!(manager.is_active(id));

        manager.stop(id).unwrap();
        assert!(!manager.is_active(id));
    }

    #[test]
    fn test_task_queue_basic() {
        let queue = SharedTaskQueue::new();

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let _id = queue.post(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        assert!(queue.has_pending());
        assert_eq!(queue.pending_count(), 1);

        queue.process_all();

        assert!(!queue.has_pending());
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_task_cancellation() {
        let queue = SharedTaskQueue::new();

        let id = queue.post(|| {});
        assert!(queue.has_pending());

        assert!(queue.cancel(id));
        assert!(!queue.has_pending());

        // Cancelling again should return false.
        assert!(!queue.cancel(id));
    }
}
