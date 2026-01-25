//! Error types for Horizon Lattice.

use std::fmt;

use crate::object::ObjectError;
use crate::property::PropertyError;

/// The main error type for Horizon Lattice operations.
#[derive(Debug)]
pub enum LatticeError {
    /// Application has already been initialized.
    ApplicationAlreadyInitialized,
    /// Application has not been initialized yet.
    ApplicationNotInitialized,
    /// Failed to create the event loop.
    EventLoopCreation(String),
    /// Failed to create a window.
    WindowCreation(String),
    /// Timer-related error.
    Timer(TimerError),
    /// Scheduler-related error.
    Scheduler(SchedulerError),
    /// Object-related error.
    Object(ObjectError),
    /// Property-related error.
    Property(PropertyError),
    /// Signal-related error.
    Signal(SignalError),
    /// Thread pool-related error.
    ThreadPool(ThreadPoolError),
    /// The event loop has already exited.
    EventLoopExited,
}

impl fmt::Display for LatticeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApplicationAlreadyInitialized => {
                write!(f, "Application has already been initialized")
            }
            Self::ApplicationNotInitialized => {
                write!(f, "Application has not been initialized. Call Application::new() first")
            }
            Self::EventLoopCreation(msg) => {
                write!(f, "Failed to create event loop: {msg}")
            }
            Self::WindowCreation(msg) => {
                write!(f, "Failed to create window: {msg}")
            }
            Self::Timer(err) => write!(f, "Timer error: {err}"),
            Self::Scheduler(err) => write!(f, "Scheduler error: {err}"),
            Self::Object(err) => write!(f, "Object error: {err}"),
            Self::Property(err) => write!(f, "Property error: {err}"),
            Self::Signal(err) => write!(f, "Signal error: {err}"),
            Self::ThreadPool(err) => write!(f, "Thread pool error: {err}"),
            Self::EventLoopExited => {
                write!(f, "The event loop has already exited")
            }
        }
    }
}

impl std::error::Error for LatticeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timer(err) => Some(err),
            Self::Scheduler(err) => Some(err),
            Self::Object(err) => Some(err),
            Self::Property(err) => Some(err),
            Self::Signal(err) => Some(err),
            Self::ThreadPool(err) => Some(err),
            _ => None,
        }
    }
}

/// Timer-specific errors.
#[derive(Debug)]
pub enum TimerError {
    /// The timer ID is invalid or has already been removed.
    InvalidTimerId,
    /// Failed to send timer event to the event loop.
    EventDispatchFailed,
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimerId => write!(f, "Invalid or expired timer ID"),
            Self::EventDispatchFailed => write!(f, "Failed to dispatch timer event"),
        }
    }
}

impl std::error::Error for TimerError {}

impl From<TimerError> for LatticeError {
    fn from(err: TimerError) -> Self {
        Self::Timer(err)
    }
}

/// Scheduler-specific errors.
#[derive(Debug)]
pub enum SchedulerError {
    /// The scheduled task ID is invalid or has already been removed.
    InvalidTaskId,
    /// Failed to send scheduler event to the event loop.
    EventDispatchFailed,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTaskId => write!(f, "Invalid or expired scheduled task ID"),
            Self::EventDispatchFailed => write!(f, "Failed to dispatch scheduler event"),
        }
    }
}

impl std::error::Error for SchedulerError {}

impl From<SchedulerError> for LatticeError {
    fn from(err: SchedulerError) -> Self {
        Self::Scheduler(err)
    }
}

impl From<ObjectError> for LatticeError {
    fn from(err: ObjectError) -> Self {
        Self::Object(err)
    }
}

impl From<PropertyError> for LatticeError {
    fn from(err: PropertyError) -> Self {
        Self::Property(err)
    }
}

impl From<SignalError> for LatticeError {
    fn from(err: SignalError) -> Self {
        Self::Signal(err)
    }
}

/// Signal-specific errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalError {
    /// The connection ID is invalid or has already been disconnected.
    InvalidConnection,
    /// The signal has been dropped and is no longer available.
    SignalDropped,
    /// Failed to queue the signal invocation to the event loop.
    QueueFailed,
}

impl fmt::Display for SignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConnection => write!(f, "Invalid or disconnected connection ID"),
            Self::SignalDropped => write!(f, "Signal has been dropped"),
            Self::QueueFailed => write!(f, "Failed to queue signal invocation"),
        }
    }
}

impl std::error::Error for SignalError {}

/// Thread pool-specific errors.
#[derive(Debug)]
pub enum ThreadPoolError {
    /// The thread pool has already been initialized.
    AlreadyInitialized,
    /// Failed to create the thread pool.
    CreationFailed(String),
    /// Task was cancelled.
    TaskCancelled,
    /// Failed to submit task to the pool.
    SubmissionFailed,
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "Thread pool has already been initialized"),
            Self::CreationFailed(msg) => write!(f, "Failed to create thread pool: {msg}"),
            Self::TaskCancelled => write!(f, "Task was cancelled"),
            Self::SubmissionFailed => write!(f, "Failed to submit task to thread pool"),
        }
    }
}

impl std::error::Error for ThreadPoolError {}

impl From<ThreadPoolError> for LatticeError {
    fn from(err: ThreadPoolError) -> Self {
        Self::ThreadPool(err)
    }
}

/// A specialized Result type for Horizon Lattice operations.
pub type Result<T> = std::result::Result<T, LatticeError>;
