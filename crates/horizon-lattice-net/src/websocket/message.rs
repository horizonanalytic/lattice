//! WebSocket message types and connection state.

/// Current state of a WebSocket connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum WebSocketState {
    /// Not connected to any server.
    #[default]
    Disconnected,
    /// Currently attempting to connect.
    Connecting,
    /// Connected and ready to send/receive messages.
    Connected,
    /// Connection lost, attempting to reconnect (if auto-reconnect is enabled).
    Reconnecting,
}


/// Standard WebSocket close codes as defined in RFC 6455.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum CloseCode {
    /// Normal closure; the connection successfully completed.
    #[default]
    Normal,
    /// Endpoint is going away (e.g., server shutting down).
    Away,
    /// Protocol error occurred.
    Protocol,
    /// Received data type that cannot be accepted.
    Unsupported,
    /// No status code was provided.
    NoStatus,
    /// Connection was closed abnormally (no close frame received).
    Abnormal,
    /// Received data that was not consistent with the message type.
    Invalid,
    /// Policy violation.
    Policy,
    /// Message too big to process.
    TooBig,
    /// Extension negotiation failed.
    Extension,
    /// Unexpected condition prevented the request from being fulfilled.
    Error,
    /// Server is restarting.
    Restart,
    /// Server is too busy; try again later.
    Again,
    /// Custom close code (application-specific, must be in range 4000-4999).
    Custom(u16),
}

impl CloseCode {
    /// Convert to the numeric close code.
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::Normal => 1000,
            Self::Away => 1001,
            Self::Protocol => 1002,
            Self::Unsupported => 1003,
            Self::NoStatus => 1005,
            Self::Abnormal => 1006,
            Self::Invalid => 1007,
            Self::Policy => 1008,
            Self::TooBig => 1009,
            Self::Extension => 1010,
            Self::Error => 1011,
            Self::Restart => 1012,
            Self::Again => 1013,
            Self::Custom(code) => *code,
        }
    }

    /// Create from a numeric close code.
    pub fn from_u16(code: u16) -> Self {
        match code {
            1000 => Self::Normal,
            1001 => Self::Away,
            1002 => Self::Protocol,
            1003 => Self::Unsupported,
            1005 => Self::NoStatus,
            1006 => Self::Abnormal,
            1007 => Self::Invalid,
            1008 => Self::Policy,
            1009 => Self::TooBig,
            1010 => Self::Extension,
            1011 => Self::Error,
            1012 => Self::Restart,
            1013 => Self::Again,
            code => Self::Custom(code),
        }
    }
}


/// Reason for closing a WebSocket connection.
#[derive(Clone, Debug, Default)]
pub struct CloseReason {
    /// The close status code.
    pub code: CloseCode,
    /// Optional human-readable reason string.
    pub reason: Option<String>,
}

impl CloseReason {
    /// Create a close reason with just a code.
    pub fn new(code: CloseCode) -> Self {
        Self { code, reason: None }
    }

    /// Create a close reason with a code and message.
    pub fn with_reason(code: CloseCode, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: Some(reason.into()),
        }
    }

    /// Create a normal close reason.
    pub fn normal() -> Self {
        Self::new(CloseCode::Normal)
    }
}
