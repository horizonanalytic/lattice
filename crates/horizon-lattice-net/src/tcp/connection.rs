//! TCP connection type for server-accepted connections.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

use super::config::TcpSocketConfig;
use crate::error::NetworkError;

/// Unique identifier for a TCP connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Create a new connection ID.
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}

/// Command sent to the connection's async task.
pub(crate) enum ConnectionCommand {
    Send(Vec<u8>),
    Close,
}

/// A TCP connection from an accepted client.
///
/// This represents a single client connection to a TCP server.
/// Use the signals to receive data and detect disconnection.
///
/// # Signals
///
/// - [`data_received`](Self::data_received): Emitted when data is received
/// - [`bytes_written`](Self::bytes_written): Emitted after data is successfully sent
/// - [`disconnected`](Self::disconnected): Emitted when the connection is closed
/// - [`error`](Self::error): Emitted when an error occurs
pub struct TcpConnection {
    id: ConnectionId,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<ConnectionCommand>>>>,
    is_connected: Arc<AtomicBool>,

    /// Signal emitted when data is received.
    pub data_received: Signal<Vec<u8>>,
    /// Signal emitted after data is successfully written.
    pub bytes_written: Signal<usize>,
    /// Signal emitted when the connection is closed.
    pub disconnected: Signal<()>,
    /// Signal emitted when an error occurs.
    pub error: Signal<NetworkError>,
}

impl TcpConnection {
    /// Create a new TCP connection from an accepted stream.
    pub(crate) fn new(
        reader: OwnedReadHalf,
        writer: OwnedWriteHalf,
        local_addr: SocketAddr,
        peer_addr: SocketAddr,
        config: &TcpSocketConfig,
        disconnect_notifier: Option<mpsc::UnboundedSender<ConnectionId>>,
    ) -> Arc<Self> {
        let id = ConnectionId::new();
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let connection = Arc::new(Self {
            id,
            local_addr,
            peer_addr,
            command_tx: Arc::new(Mutex::new(Some(command_tx))),
            is_connected: Arc::new(AtomicBool::new(true)),
            data_received: Signal::new(),
            bytes_written: Signal::new(),
            disconnected: Signal::new(),
            error: Signal::new(),
        });

        // Start the I/O task
        connection.start_io_task(
            reader,
            writer,
            command_rx,
            config.read_buffer_size,
            disconnect_notifier,
        );

        connection
    }

    /// Get the unique connection ID.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Get the local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the peer socket address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Check if the connection is still active.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Send data to the peer.
    ///
    /// Returns `Ok(())` if the data was queued for sending, or an error if disconnected.
    pub fn send(&self, data: impl Into<Vec<u8>>) -> crate::Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(ConnectionCommand::Send(data.into()))
                    .map_err(|_| NetworkError::TcpSocket("Connection closed".into()))?;
                Ok(())
            }
            None => Err(NetworkError::TcpSocket("Connection closed".into())),
        }
    }

    /// Close the connection.
    pub fn close(&self) {
        if let Some(tx) = self.command_tx.lock().take() {
            let _ = tx.send(ConnectionCommand::Close);
        }
        self.is_connected.store(false, Ordering::SeqCst);
    }

    /// Start the I/O task for this connection.
    fn start_io_task(
        &self,
        mut reader: OwnedReadHalf,
        mut writer: OwnedWriteHalf,
        mut command_rx: mpsc::UnboundedReceiver<ConnectionCommand>,
        buffer_size: usize,
        disconnect_notifier: Option<mpsc::UnboundedSender<ConnectionId>>,
    ) {
        let command_tx = self.command_tx.clone();
        let is_connected = self.is_connected.clone();
        let conn_id = self.id;

        // Get signal pointers for use in the async task
        let data_received_ptr = &self.data_received as *const Signal<Vec<u8>> as usize;
        let bytes_written_ptr = &self.bytes_written as *const Signal<usize> as usize;
        let disconnected_ptr = &self.disconnected as *const Signal<()> as usize;
        let error_ptr = &self.error as *const Signal<NetworkError> as usize;

        tokio::spawn(async move {
            // SAFETY: Signal pointers remain valid as long as TcpConnection exists.
            let emit_data_received = |data: Vec<u8>| unsafe {
                let signal = &*(data_received_ptr as *const Signal<Vec<u8>>);
                signal.emit(data);
            };
            let emit_bytes_written = |count: usize| unsafe {
                let signal = &*(bytes_written_ptr as *const Signal<usize>);
                signal.emit(count);
            };
            let emit_disconnected = || unsafe {
                let signal = &*(disconnected_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_error = |err: NetworkError| unsafe {
                let signal = &*(error_ptr as *const Signal<NetworkError>);
                signal.emit(err);
            };

            let mut buffer = vec![0u8; buffer_size];

            loop {
                tokio::select! {
                    // Handle commands from user
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(ConnectionCommand::Send(data)) => {
                                let len = data.len();
                                match writer.write_all(&data).await {
                                    Ok(()) => emit_bytes_written(len),
                                    Err(e) => {
                                        emit_error(NetworkError::TcpSocket(e.to_string()));
                                        break;
                                    }
                                }
                            }
                            Some(ConnectionCommand::Close) | None => {
                                break;
                            }
                        }
                    }

                    // Handle incoming data
                    result = reader.read(&mut buffer) => {
                        match result {
                            Ok(0) => {
                                // EOF - connection closed by peer
                                break;
                            }
                            Ok(n) => {
                                emit_data_received(buffer[..n].to_vec());
                            }
                            Err(e) => {
                                emit_error(NetworkError::TcpSocket(e.to_string()));
                                break;
                            }
                        }
                    }
                }
            }

            // Cleanup
            *command_tx.lock() = None;
            is_connected.store(false, Ordering::SeqCst);

            // Notify server about disconnection (if notifier provided)
            if let Some(notifier) = disconnect_notifier {
                let _ = notifier.send(conn_id);
            }

            emit_disconnected();
        });
    }
}

impl std::fmt::Debug for TcpConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpConnection")
            .field("id", &self.id)
            .field("local_addr", &self.local_addr)
            .field("peer_addr", &self.peer_addr)
            .field("is_connected", &self.is_connected())
            .finish()
    }
}
