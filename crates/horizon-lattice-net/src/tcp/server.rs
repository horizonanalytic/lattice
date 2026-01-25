//! TCP server with signal-based event delivery.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use super::config::TcpServerConfig;
use super::connection::{ConnectionId, TcpConnection};
use super::state::TcpServerState;
use crate::error::NetworkError;

/// Internal state for the TCP server.
struct TcpServerInner {
    state: TcpServerState,
    connections: HashMap<ConnectionId, Arc<TcpConnection>>,
    local_addr: Option<SocketAddr>,
}

/// Command sent to the TCP server's async task.
enum ServerCommand {
    Stop,
    Broadcast(Vec<u8>),
    SendTo(ConnectionId, Vec<u8>),
    DisconnectClient(ConnectionId),
}

/// A TCP server with signal-based event delivery.
///
/// The server listens for incoming connections and emits signals
/// for connection events. Each accepted connection is represented
/// as a `TcpConnection` with its own signals.
///
/// # Signals
///
/// - [`started`](Self::started): Emitted when the server starts listening
/// - [`stopped`](Self::stopped): Emitted when the server stops
/// - [`new_connection`](Self::new_connection): Emitted when a new client connects
/// - [`connection_closed`](Self::connection_closed): Emitted when a client disconnects
/// - [`error`](Self::error): Emitted when an error occurs
///
/// # Example
///
/// ```ignore
/// let config = TcpServerConfig::new("0.0.0.0", 8080)
///     .no_delay(true)
///     .backlog(128);
///
/// let server = TcpServer::new(config);
///
/// server.started.connect(|| println!("Server started!"));
///
/// server.new_connection.connect(|conn| {
///     let peer = conn.peer_addr();
///     println!("New connection from {}", peer);
///
///     conn.data_received.connect(move |data| {
///         println!("Received from {}: {} bytes", peer, data.len());
///     });
/// });
///
/// server.start();
/// ```
pub struct TcpServer {
    config: TcpServerConfig,
    inner: Arc<Mutex<TcpServerInner>>,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<ServerCommand>>>>,
    is_running: Arc<AtomicBool>,

    /// Signal emitted when the server starts listening.
    pub started: Signal<()>,
    /// Signal emitted when the server stops.
    pub stopped: Signal<()>,
    /// Signal emitted when a new client connects.
    pub new_connection: Signal<Arc<TcpConnection>>,
    /// Signal emitted when a client disconnects.
    pub connection_closed: Signal<ConnectionId>,
    /// Signal emitted when an error occurs.
    pub error: Signal<NetworkError>,
}

impl TcpServer {
    /// Create a new TCP server with the given configuration.
    pub fn new(config: TcpServerConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(TcpServerInner {
                state: TcpServerState::Stopped,
                connections: HashMap::new(),
                local_addr: None,
            })),
            command_tx: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            started: Signal::new(),
            stopped: Signal::new(),
            new_connection: Signal::new(),
            connection_closed: Signal::new(),
            error: Signal::new(),
        }
    }

    /// Get the current server state.
    pub fn state(&self) -> TcpServerState {
        self.inner.lock().state
    }

    /// Check if the server is listening.
    pub fn is_listening(&self) -> bool {
        self.inner.lock().state == TcpServerState::Listening
    }

    /// Get the number of active connections.
    pub fn connection_count(&self) -> usize {
        self.inner.lock().connections.len()
    }

    /// Get a list of all active connection IDs.
    pub fn connections(&self) -> Vec<ConnectionId> {
        self.inner.lock().connections.keys().copied().collect()
    }

    /// Get a connection by ID.
    pub fn get_connection(&self, id: ConnectionId) -> Option<Arc<TcpConnection>> {
        self.inner.lock().connections.get(&id).cloned()
    }

    /// Start the TCP server.
    ///
    /// If the server is already running, this is a no-op.
    pub fn start(&self) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let config = self.config.clone();
        let inner = self.inner.clone();
        let command_tx = self.command_tx.clone();
        let is_running = self.is_running.clone();

        // Update state
        inner.lock().state = TcpServerState::Starting;

        // Get signal pointers for use in the async task
        let started_ptr = &self.started as *const Signal<()> as usize;
        let stopped_ptr = &self.stopped as *const Signal<()> as usize;
        let new_connection_ptr =
            &self.new_connection as *const Signal<Arc<TcpConnection>> as usize;
        let connection_closed_ptr =
            &self.connection_closed as *const Signal<ConnectionId> as usize;
        let error_ptr = &self.error as *const Signal<NetworkError> as usize;

        tokio::spawn(async move {
            // SAFETY: Signal pointers remain valid as long as TcpServer exists.
            let emit_started = || unsafe {
                let signal = &*(started_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_stopped = || unsafe {
                let signal = &*(stopped_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_new_connection = |conn: Arc<TcpConnection>| unsafe {
                let signal = &*(new_connection_ptr as *const Signal<Arc<TcpConnection>>);
                signal.emit(conn);
            };
            let emit_connection_closed = |id: ConnectionId| unsafe {
                let signal = &*(connection_closed_ptr as *const Signal<ConnectionId>);
                signal.emit(id);
            };
            let emit_error = |err: NetworkError| unsafe {
                let signal = &*(error_ptr as *const Signal<NetworkError>);
                signal.emit(err);
            };

            // Bind the listener
            let listener = match TcpListener::bind(config.bind_addr()).await {
                Ok(l) => l,
                Err(e) => {
                    emit_error(NetworkError::TcpSocket(format!("Failed to bind: {}", e)));
                    inner.lock().state = TcpServerState::Stopped;
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let local_addr = match listener.local_addr() {
                Ok(addr) => addr,
                Err(e) => {
                    emit_error(NetworkError::TcpSocket(format!(
                        "Failed to get local address: {}",
                        e
                    )));
                    inner.lock().state = TcpServerState::Stopped;
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Create command channel
            let (tx, mut rx) = mpsc::unbounded_channel::<ServerCommand>();
            *command_tx.lock() = Some(tx);

            // Create disconnect notification channel
            let (disconnect_tx, mut disconnect_rx) = mpsc::unbounded_channel::<ConnectionId>();

            // Update state and emit started signal
            {
                let mut guard = inner.lock();
                guard.state = TcpServerState::Listening;
                guard.local_addr = Some(local_addr);
            }
            emit_started();

            // Accept loop
            loop {
                tokio::select! {
                    // Handle commands
                    cmd = rx.recv() => {
                        match cmd {
                            Some(ServerCommand::Stop) | None => {
                                break;
                            }
                            Some(ServerCommand::Broadcast(data)) => {
                                let connections: Vec<Arc<TcpConnection>> =
                                    inner.lock().connections.values().cloned().collect();
                                for conn in connections {
                                    let _ = conn.send(data.clone());
                                }
                            }
                            Some(ServerCommand::SendTo(id, data)) => {
                                if let Some(conn) = inner.lock().connections.get(&id) {
                                    let _ = conn.send(data);
                                }
                            }
                            Some(ServerCommand::DisconnectClient(id)) => {
                                if let Some(conn) = inner.lock().connections.get(&id) {
                                    conn.close();
                                }
                            }
                        }
                    }

                    // Handle disconnection notifications from connections
                    Some(conn_id) = disconnect_rx.recv() => {
                        inner.lock().connections.remove(&conn_id);
                        emit_connection_closed(conn_id);
                    }

                    // Accept new connections
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer_addr)) => {
                                // Apply socket options
                                if let Err(e) = stream.set_nodelay(config.socket.no_delay) {
                                    emit_error(NetworkError::TcpSocket(format!(
                                        "Failed to set TCP_NODELAY: {}", e
                                    )));
                                }

                                // Split the stream
                                let (reader, writer) = stream.into_split();

                                // Create connection with disconnect notifier
                                let connection = TcpConnection::new(
                                    reader,
                                    writer,
                                    local_addr,
                                    peer_addr,
                                    &config.socket,
                                    Some(disconnect_tx.clone()),
                                );

                                let conn_id = connection.id();

                                // Store and emit
                                inner.lock().connections.insert(conn_id, connection.clone());
                                emit_new_connection(connection);
                            }
                            Err(e) => {
                                emit_error(NetworkError::TcpSocket(format!(
                                    "Accept error: {}", e
                                )));
                            }
                        }
                    }
                }
            }

            // Cleanup - close all connections
            inner.lock().state = TcpServerState::Stopping;
            let connections: Vec<Arc<TcpConnection>> =
                inner.lock().connections.values().cloned().collect();
            for conn in connections {
                conn.close();
            }
            inner.lock().connections.clear();

            // Clear command channel
            *command_tx.lock() = None;

            // Update state
            inner.lock().state = TcpServerState::Stopped;
            is_running.store(false, Ordering::SeqCst);
            emit_stopped();
        });
    }

    /// Stop the TCP server.
    pub fn stop(&self) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(ServerCommand::Stop);
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Broadcast data to all connected clients.
    pub fn broadcast(&self, data: impl Into<Vec<u8>>) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(ServerCommand::Broadcast(data.into()));
        }
    }

    /// Send data to a specific client.
    pub fn send_to(&self, id: ConnectionId, data: impl Into<Vec<u8>>) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(ServerCommand::SendTo(id, data.into()));
        }
    }

    /// Disconnect a specific client.
    pub fn disconnect_client(&self, id: ConnectionId) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(ServerCommand::DisconnectClient(id));
        }
    }

    /// Get the configured bind address.
    pub fn bind_addr(&self) -> String {
        self.config.bind_addr()
    }

    /// Get the actual local address after the server has started.
    ///
    /// Returns `None` if the server is not listening.
    /// This is useful when binding to port 0 to get the actual assigned port.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.inner.lock().local_addr
    }
}

impl std::fmt::Debug for TcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpServer")
            .field("bind_addr", &self.config.bind_addr())
            .field("state", &self.state())
            .field("connections", &self.connection_count())
            .finish()
    }
}
