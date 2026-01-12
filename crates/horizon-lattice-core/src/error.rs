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
    /// Object-related error.
    Object(ObjectError),
    /// Property-related error.
    Property(PropertyError),
    /// Signal-related error.
    Signal(SignalError),
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
            Self::Object(err) => write!(f, "Object error: {err}"),
            Self::Property(err) => write!(f, "Property error: {err}"),
            Self::Signal(err) => write!(f, "Signal error: {err}"),
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
            Self::Object(err) => Some(err),
            Self::Property(err) => Some(err),
            Self::Signal(err) => Some(err),
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

/// A specialized Result type for Horizon Lattice operations.
pub type Result<T> = std::result::Result<T, LatticeError>;
