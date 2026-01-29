//! UDP socket with signal-based event delivery.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::sync::mpsc;

use super::config::{Datagram, UdpSocketConfig};
use super::state::UdpSocketState;
use crate::Result;
use crate::error::NetworkError;

/// Internal state for the UDP socket.
struct UdpSocketInner {
    state: UdpSocketState,
    local_addr: Option<SocketAddr>,
}

/// Command sent to the UDP socket's async task.
enum Command {
    SendTo(Vec<u8>, SocketAddr),
    JoinMulticast(Ipv4Addr, Option<Ipv4Addr>),
    LeaveMulticast(Ipv4Addr, Option<Ipv4Addr>),
    SetBroadcast(bool),
    Close,
}

/// A UDP socket with signal-based event delivery.
///
/// The socket provides connectionless datagram communication and emits
/// signals for received data and errors.
///
/// # Signals
///
/// - [`bound`](Self::bound): Emitted when the socket is bound successfully
/// - [`datagram_received`](Self::datagram_received): Emitted when a datagram is received
/// - [`datagram_sent`](Self::datagram_sent): Emitted after a datagram is sent
/// - [`closed`](Self::closed): Emitted when the socket is closed
/// - [`error`](Self::error): Emitted when an error occurs
///
/// # Example
///
/// ```ignore
/// let config = UdpSocketConfig::new("0.0.0.0", 8080);
/// let socket = UdpSocket::new(config);
///
/// socket.bound.connect(|addr| {
///     println!("Socket bound to {}", addr);
/// });
///
/// socket.datagram_received.connect(|datagram| {
///     println!("Received {} bytes from {}", datagram.data.len(), datagram.source);
/// });
///
/// socket.bind();
/// socket.send_to(b"Hello!", "127.0.0.1:9000".parse().unwrap())?;
/// ```
pub struct UdpSocket {
    config: UdpSocketConfig,
    inner: Arc<Mutex<UdpSocketInner>>,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Command>>>>,
    is_running: Arc<AtomicBool>,

    /// Signal emitted when the socket is bound successfully.
    pub bound: Signal<SocketAddr>,
    /// Signal emitted when a datagram is received.
    pub datagram_received: Signal<Datagram>,
    /// Signal emitted after a datagram is successfully sent.
    pub datagram_sent: Signal<usize>,
    /// Signal emitted when the socket is closed.
    pub closed: Signal<()>,
    /// Signal emitted when an error occurs.
    pub error: Signal<NetworkError>,
}

impl UdpSocket {
    /// Create a new UDP socket with the given configuration.
    pub fn new(config: UdpSocketConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(UdpSocketInner {
                state: UdpSocketState::Unbound,
                local_addr: None,
            })),
            command_tx: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            bound: Signal::new(),
            datagram_received: Signal::new(),
            datagram_sent: Signal::new(),
            closed: Signal::new(),
            error: Signal::new(),
        }
    }

    /// Get the current socket state.
    pub fn state(&self) -> UdpSocketState {
        self.inner.lock().state
    }

    /// Check if the socket is bound.
    pub fn is_bound(&self) -> bool {
        self.inner.lock().state == UdpSocketState::Bound
    }

    /// Get the local address after binding.
    /// Returns `None` if the socket is not bound.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.inner.lock().local_addr
    }

    /// Bind the socket to the configured address.
    ///
    /// If the socket is already bound, this is a no-op.
    pub fn bind(&self) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let config = self.config.clone();
        let inner = self.inner.clone();
        let command_tx = self.command_tx.clone();
        let is_running = self.is_running.clone();

        // Get signal pointers for use in the async task
        let bound_ptr = &self.bound as *const Signal<SocketAddr> as usize;
        let datagram_received_ptr = &self.datagram_received as *const Signal<Datagram> as usize;
        let datagram_sent_ptr = &self.datagram_sent as *const Signal<usize> as usize;
        let closed_ptr = &self.closed as *const Signal<()> as usize;
        let error_ptr = &self.error as *const Signal<NetworkError> as usize;

        tokio::spawn(async move {
            // SAFETY: Signal pointers remain valid as long as UdpSocket exists.
            // The is_running flag ensures we don't outlive the socket.
            let emit_bound = |addr: SocketAddr| unsafe {
                let signal = &*(bound_ptr as *const Signal<SocketAddr>);
                signal.emit(addr);
            };
            let emit_datagram_received = |datagram: Datagram| unsafe {
                let signal = &*(datagram_received_ptr as *const Signal<Datagram>);
                signal.emit(datagram);
            };
            let emit_datagram_sent = |count: usize| unsafe {
                let signal = &*(datagram_sent_ptr as *const Signal<usize>);
                signal.emit(count);
            };
            let emit_closed = || unsafe {
                let signal = &*(closed_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_error = |err: NetworkError| unsafe {
                let signal = &*(error_ptr as *const Signal<NetworkError>);
                signal.emit(err);
            };

            // Update state to binding
            inner.lock().state = UdpSocketState::Binding;

            // Bind the socket
            let socket = match TokioUdpSocket::bind(config.bind_addr()).await {
                Ok(s) => s,
                Err(e) => {
                    emit_error(NetworkError::UdpSocket(format!("Failed to bind: {}", e)));
                    inner.lock().state = UdpSocketState::Unbound;
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Get local address
            let local_addr = match socket.local_addr() {
                Ok(addr) => addr,
                Err(e) => {
                    emit_error(NetworkError::UdpSocket(format!(
                        "Failed to get local address: {}",
                        e
                    )));
                    inner.lock().state = UdpSocketState::Unbound;
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Apply socket options
            if config.broadcast
                && let Err(e) = socket.set_broadcast(true) {
                    emit_error(NetworkError::UdpSocket(format!(
                        "Failed to enable broadcast: {}",
                        e
                    )));
                }

            // Apply multicast settings
            if config.multicast.ttl > 0
                && let Err(e) = socket.set_multicast_ttl_v4(config.multicast.ttl) {
                    emit_error(NetworkError::UdpSocket(format!(
                        "Failed to set multicast TTL: {}",
                        e
                    )));
                }

            if let Err(e) = socket.set_multicast_loop_v4(config.multicast.loopback) {
                emit_error(NetworkError::UdpSocket(format!(
                    "Failed to set multicast loopback: {}",
                    e
                )));
            }

            // Join configured multicast groups
            for (multicast_addr, interface) in &config.multicast.groups {
                let iface = interface.unwrap_or(Ipv4Addr::UNSPECIFIED);
                if let Err(e) = socket.join_multicast_v4(*multicast_addr, iface) {
                    emit_error(NetworkError::UdpSocket(format!(
                        "Failed to join multicast group {}: {}",
                        multicast_addr, e
                    )));
                }
            }

            // Create command channel
            let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
            *command_tx.lock() = Some(tx);

            // Update state and emit bound signal
            {
                let mut guard = inner.lock();
                guard.state = UdpSocketState::Bound;
                guard.local_addr = Some(local_addr);
            }
            emit_bound(local_addr);

            // Main event loop
            let mut buffer = vec![0u8; config.recv_buffer_size];

            loop {
                tokio::select! {
                    // Handle commands
                    cmd = rx.recv() => {
                        match cmd {
                            Some(Command::SendTo(data, target)) => {
                                match socket.send_to(&data, target).await {
                                    Ok(n) => emit_datagram_sent(n),
                                    Err(e) => {
                                        emit_error(NetworkError::UdpSocket(format!(
                                            "Send error: {}", e
                                        )));
                                    }
                                }
                            }
                            Some(Command::JoinMulticast(multicast_addr, interface)) => {
                                let iface = interface.unwrap_or(Ipv4Addr::UNSPECIFIED);
                                if let Err(e) = socket.join_multicast_v4(multicast_addr, iface) {
                                    emit_error(NetworkError::UdpSocket(format!(
                                        "Failed to join multicast group {}: {}",
                                        multicast_addr, e
                                    )));
                                }
                            }
                            Some(Command::LeaveMulticast(multicast_addr, interface)) => {
                                let iface = interface.unwrap_or(Ipv4Addr::UNSPECIFIED);
                                if let Err(e) = socket.leave_multicast_v4(multicast_addr, iface) {
                                    emit_error(NetworkError::UdpSocket(format!(
                                        "Failed to leave multicast group {}: {}",
                                        multicast_addr, e
                                    )));
                                }
                            }
                            Some(Command::SetBroadcast(enabled)) => {
                                if let Err(e) = socket.set_broadcast(enabled) {
                                    emit_error(NetworkError::UdpSocket(format!(
                                        "Failed to set broadcast: {}", e
                                    )));
                                }
                            }
                            Some(Command::Close) | None => {
                                break;
                            }
                        }
                    }

                    // Receive datagrams
                    result = socket.recv_from(&mut buffer) => {
                        match result {
                            Ok((n, source)) => {
                                let datagram = Datagram::new(buffer[..n].to_vec(), source);
                                emit_datagram_received(datagram);
                            }
                            Err(e) => {
                                emit_error(NetworkError::UdpSocket(format!(
                                    "Receive error: {}", e
                                )));
                            }
                        }
                    }
                }
            }

            // Cleanup
            *command_tx.lock() = None;

            // Update state
            {
                let mut guard = inner.lock();
                guard.state = UdpSocketState::Closed;
                guard.local_addr = None;
            }
            is_running.store(false, Ordering::SeqCst);
            emit_closed();
        });
    }

    /// Send a datagram to the specified address.
    ///
    /// Returns `Ok(())` if the datagram was queued for sending, or an error if not bound.
    pub fn send_to(&self, data: impl Into<Vec<u8>>, target: SocketAddr) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::SendTo(data.into(), target))
                    .map_err(|_| NetworkError::UdpSocket("Socket not bound".into()))?;
                Ok(())
            }
            None => Err(NetworkError::UdpSocket("Socket not bound".into())),
        }
    }

    /// Join a multicast group.
    pub fn join_multicast(&self, multicast_addr: Ipv4Addr) -> Result<()> {
        self.join_multicast_on(multicast_addr, None)
    }

    /// Join a multicast group on a specific interface.
    pub fn join_multicast_on(
        &self,
        multicast_addr: Ipv4Addr,
        interface: Option<Ipv4Addr>,
    ) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::JoinMulticast(multicast_addr, interface))
                    .map_err(|_| NetworkError::UdpSocket("Socket not bound".into()))?;
                Ok(())
            }
            None => Err(NetworkError::UdpSocket("Socket not bound".into())),
        }
    }

    /// Leave a multicast group.
    pub fn leave_multicast(&self, multicast_addr: Ipv4Addr) -> Result<()> {
        self.leave_multicast_on(multicast_addr, None)
    }

    /// Leave a multicast group on a specific interface.
    pub fn leave_multicast_on(
        &self,
        multicast_addr: Ipv4Addr,
        interface: Option<Ipv4Addr>,
    ) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::LeaveMulticast(multicast_addr, interface))
                    .map_err(|_| NetworkError::UdpSocket("Socket not bound".into()))?;
                Ok(())
            }
            None => Err(NetworkError::UdpSocket("Socket not bound".into())),
        }
    }

    /// Enable or disable broadcast mode.
    pub fn set_broadcast(&self, enabled: bool) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::SetBroadcast(enabled))
                    .map_err(|_| NetworkError::UdpSocket("Socket not bound".into()))?;
                Ok(())
            }
            None => Err(NetworkError::UdpSocket("Socket not bound".into())),
        }
    }

    /// Close the socket.
    pub fn close(&self) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(Command::Close);
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Get the configured bind address.
    pub fn bind_addr(&self) -> String {
        self.config.bind_addr()
    }
}

impl std::fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpSocket")
            .field("bind_addr", &self.config.bind_addr())
            .field("state", &self.state())
            .field("local_addr", &self.local_addr())
            .finish()
    }
}
