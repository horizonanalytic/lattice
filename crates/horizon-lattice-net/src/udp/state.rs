//! State enumerations for UDP sockets.

/// State of a UDP socket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UdpSocketState {
    /// Socket is not bound.
    #[default]
    Unbound,
    /// Socket is binding to an address.
    Binding,
    /// Socket is bound and ready.
    Bound,
    /// Socket is closing.
    Closing,
    /// Socket is closed.
    Closed,
}

impl std::fmt::Display for UdpSocketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UdpSocketState::Unbound => write!(f, "Unbound"),
            UdpSocketState::Binding => write!(f, "Binding"),
            UdpSocketState::Bound => write!(f, "Bound"),
            UdpSocketState::Closing => write!(f, "Closing"),
            UdpSocketState::Closed => write!(f, "Closed"),
        }
    }
}
