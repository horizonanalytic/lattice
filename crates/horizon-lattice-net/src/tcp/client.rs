//! TCP client with signal-based event delivery.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::config::TcpClientConfig;
use super::state::TcpConnectionState;
use crate::error::NetworkError;
use crate::websocket::ReconnectConfig;
use crate::Result;

/// Internal state for the TCP client.
struct TcpClientInner {
    state: TcpConnectionState,
    reconnect_attempt: u32,
}

/// Command sent to the TCP client's async task.
enum Command {
    Send(Vec<u8>),
    Close,
}

/// A TCP client with signal-based event delivery.
///
/// The client manages a TCP connection and emits signals for
/// connection events and received data. It supports:
///
/// - Configurable socket options (no-delay, keep-alive)
/// - Optional auto-reconnect with exponential backoff
///
/// # Signals
///
/// - [`connected`](Self::connected): Emitted when the connection is established
/// - [`disconnected`](Self::disconnected): Emitted when the connection is closed
/// - [`data_received`](Self::data_received): Emitted when data is received
/// - [`bytes_written`](Self::bytes_written): Emitted after data is successfully sent
/// - [`error`](Self::error): Emitted when an error occurs
///
/// # Example
///
/// ```ignore
/// let config = TcpClientConfig::new("127.0.0.1", 8080)
///     .no_delay(true)
///     .auto_reconnect();
///
/// let client = TcpClient::new(config);
///
/// client.connected.connect(|| println!("Connected!"));
/// client.data_received.connect(|data| println!("Received {} bytes", data.len()));
///
/// client.connect();
/// client.send(b"Hello, Server!")?;
/// ```
pub struct TcpClient {
    config: TcpClientConfig,
    inner: Arc<Mutex<TcpClientInner>>,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Command>>>>,
    is_running: Arc<AtomicBool>,

    /// Signal emitted when the connection is established.
    pub connected: Signal<()>,
    /// Signal emitted when the connection is closed.
    pub disconnected: Signal<()>,
    /// Signal emitted when data is received.
    pub data_received: Signal<Vec<u8>>,
    /// Signal emitted after data is successfully written.
    pub bytes_written: Signal<usize>,
    /// Signal emitted when an error occurs.
    pub error: Signal<NetworkError>,
}

impl TcpClient {
    /// Create a new TCP client with the given configuration.
    pub fn new(config: TcpClientConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(TcpClientInner {
                state: TcpConnectionState::Disconnected,
                reconnect_attempt: 0,
            })),
            command_tx: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            connected: Signal::new(),
            disconnected: Signal::new(),
            data_received: Signal::new(),
            bytes_written: Signal::new(),
            error: Signal::new(),
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> TcpConnectionState {
        self.inner.lock().state
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.inner.lock().state == TcpConnectionState::Connected
    }

    /// Connect to the TCP server.
    ///
    /// If the client is already connected or connecting, this is a no-op.
    pub fn connect(&self) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let config = self.config.clone();
        let inner = self.inner.clone();
        let command_tx = self.command_tx.clone();
        let is_running = self.is_running.clone();

        // Get signal pointers for use in the async task
        let connected_ptr = &self.connected as *const Signal<()> as usize;
        let disconnected_ptr = &self.disconnected as *const Signal<()> as usize;
        let data_ptr = &self.data_received as *const Signal<Vec<u8>> as usize;
        let bytes_written_ptr = &self.bytes_written as *const Signal<usize> as usize;
        let error_ptr = &self.error as *const Signal<NetworkError> as usize;

        tokio::spawn(async move {
            // SAFETY: Signal pointers remain valid as long as TcpClient exists.
            // The is_running flag ensures we don't outlive the client.
            let emit_connected = || unsafe {
                let signal = &*(connected_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_disconnected = || unsafe {
                let signal = &*(disconnected_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_data = |data: Vec<u8>| unsafe {
                let signal = &*(data_ptr as *const Signal<Vec<u8>>);
                signal.emit(data);
            };
            let emit_bytes_written = |count: usize| unsafe {
                let signal = &*(bytes_written_ptr as *const Signal<usize>);
                signal.emit(count);
            };
            let emit_error = |err: NetworkError| unsafe {
                let signal = &*(error_ptr as *const Signal<NetworkError>);
                signal.emit(err);
            };

            let mut reconnect_attempt: u32 = 0;

            loop {
                // Update state to connecting/reconnecting
                {
                    let mut state = inner.lock();
                    state.state = if reconnect_attempt > 0 {
                        TcpConnectionState::Reconnecting
                    } else {
                        TcpConnectionState::Connecting
                    };
                    state.reconnect_attempt = reconnect_attempt;
                }

                // Attempt to connect
                let connect_result = Self::connect_with_config(&config).await;

                match connect_result {
                    Ok(stream) => {
                        // Apply socket options
                        if let Err(e) = stream.set_nodelay(config.socket.no_delay) {
                            emit_error(NetworkError::TcpSocket(format!(
                                "Failed to set TCP_NODELAY: {}",
                                e
                            )));
                        }

                        // Connection successful
                        reconnect_attempt = 0;
                        {
                            let mut state = inner.lock();
                            state.state = TcpConnectionState::Connected;
                            state.reconnect_attempt = 0;
                        }
                        emit_connected();

                        // Create command channel
                        let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
                        *command_tx.lock() = Some(tx);

                        // Split the stream
                        let (mut reader, mut writer) = stream.into_split();

                        // Handle messages and commands
                        let mut closed_normally = false;
                        let mut buffer = vec![0u8; config.socket.read_buffer_size];

                        loop {
                            tokio::select! {
                                // Receive command from user
                                cmd = rx.recv() => {
                                    match cmd {
                                        Some(Command::Send(data)) => {
                                            let len = data.len();
                                            match writer.write_all(&data).await {
                                                Ok(()) => emit_bytes_written(len),
                                                Err(e) => {
                                                    emit_error(NetworkError::TcpSocket(e.to_string()));
                                                    break;
                                                }
                                            }
                                        }
                                        Some(Command::Close) | None => {
                                            closed_normally = true;
                                            break;
                                        }
                                    }
                                }

                                // Receive data from server
                                result = reader.read(&mut buffer) => {
                                    match result {
                                        Ok(0) => {
                                            // EOF - server closed connection
                                            break;
                                        }
                                        Ok(n) => {
                                            emit_data(buffer[..n].to_vec());
                                        }
                                        Err(e) => {
                                            emit_error(NetworkError::TcpSocket(e.to_string()));
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // Clear command channel
                        *command_tx.lock() = None;

                        // Update state
                        inner.lock().state = TcpConnectionState::Disconnected;
                        emit_disconnected();

                        // Check if we should reconnect
                        if closed_normally {
                            is_running.store(false, Ordering::SeqCst);
                            return;
                        }
                    }
                    Err(e) => {
                        emit_error(e);
                    }
                }

                // Check if auto-reconnect is enabled
                let reconnect_config = match &config.reconnect {
                    Some(cfg) => cfg,
                    None => {
                        inner.lock().state = TcpConnectionState::Disconnected;
                        emit_disconnected();
                        is_running.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                // Check max attempts
                if let Some(max) = reconnect_config.max_attempts {
                    if reconnect_attempt >= max {
                        emit_error(NetworkError::Connection(format!(
                            "Max reconnection attempts ({}) reached",
                            max
                        )));
                        inner.lock().state = TcpConnectionState::Disconnected;
                        emit_disconnected();
                        is_running.store(false, Ordering::SeqCst);
                        return;
                    }
                }

                // Wait before reconnecting
                let delay = Self::delay_for_attempt(reconnect_config, reconnect_attempt);
                inner.lock().state = TcpConnectionState::Reconnecting;
                tokio::time::sleep(delay).await;

                // Check if we were told to stop while sleeping
                if !is_running.load(Ordering::SeqCst) {
                    inner.lock().state = TcpConnectionState::Disconnected;
                    return;
                }

                reconnect_attempt += 1;
            }
        });
    }

    /// Connect with the given configuration.
    async fn connect_with_config(config: &TcpClientConfig) -> crate::Result<TcpStream> {
        let addr = config.address();

        match config.socket.connect_timeout {
            Some(timeout_duration) => {
                match timeout(timeout_duration, TcpStream::connect(&addr)).await {
                    Ok(Ok(stream)) => Ok(stream),
                    Ok(Err(e)) => Err(NetworkError::TcpSocket(e.to_string())),
                    Err(_) => Err(NetworkError::Timeout),
                }
            }
            None => TcpStream::connect(&addr)
                .await
                .map_err(|e| NetworkError::TcpSocket(e.to_string())),
        }
    }

    /// Calculate the delay for a given reconnect attempt.
    fn delay_for_attempt(config: &ReconnectConfig, attempt: u32) -> std::time::Duration {
        let base_delay_ms = config.initial_delay.as_millis() as f64;
        let delay_ms = base_delay_ms * config.backoff_multiplier.powi(attempt as i32);
        let delay_ms = delay_ms.min(config.max_delay.as_millis() as f64) as u64;

        // Add jitter (±10%)
        let jitter_range = (delay_ms as f64 * 0.1) as u64;
        let jitter = if jitter_range > 0 {
            rand::random::<u64>() % (jitter_range * 2) - jitter_range
        } else {
            0
        };

        std::time::Duration::from_millis((delay_ms as i64 + jitter as i64).max(0) as u64)
    }

    /// Disconnect from the TCP server.
    pub fn disconnect(&self) {
        self.close();
    }

    /// Close the connection.
    pub fn close(&self) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(Command::Close);
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Send data to the server.
    ///
    /// Returns `Ok(())` if the data was queued for sending, or an error if not connected.
    pub fn send(&self, data: impl Into<Vec<u8>>) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::Send(data.into()))
                    .map_err(|_| NetworkError::Connection("Not connected".into()))?;
                Ok(())
            }
            None => Err(NetworkError::Connection("Not connected".into())),
        }
    }

    /// Get the host this client is configured to connect to.
    pub fn host(&self) -> &str {
        &self.config.host
    }

    /// Get the port this client is configured to connect to.
    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Get the full address (host:port) this client connects to.
    pub fn address(&self) -> String {
        self.config.address()
    }
}

impl std::fmt::Debug for TcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpClient")
            .field("address", &self.config.address())
            .field("state", &self.state())
            .finish()
    }
}
