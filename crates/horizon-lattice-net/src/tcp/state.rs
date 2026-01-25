//! State enums for TCP connections and servers.

/// Current state of a TCP connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpConnectionState {
    /// Not connected to any server.
    Disconnected,
    /// Currently attempting to connect.
    Connecting,
    /// Connected and ready to send/receive data.
    Connected,
    /// Connection lost, attempting to reconnect (if auto-reconnect is enabled).
    Reconnecting,
    /// Connection is being closed.
    Closing,
}

impl Default for TcpConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl std::fmt::Display for TcpConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Closing => write!(f, "Closing"),
        }
    }
}

/// Current state of a TCP server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpServerState {
    /// Server is not running.
    Stopped,
    /// Server is starting up.
    Starting,
    /// Server is listening for connections.
    Listening,
    /// Server is shutting down.
    Stopping,
}

impl Default for TcpServerState {
    fn default() -> Self {
        Self::Stopped
    }
}

impl std::fmt::Display for TcpServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "Stopped"),
            Self::Starting => write!(f, "Starting"),
            Self::Listening => write!(f, "Listening"),
            Self::Stopping => write!(f, "Stopping"),
        }
    }
}
