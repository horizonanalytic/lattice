//! WebSocket client with signal-based event delivery.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungsteniteCloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

use super::message::{CloseCode, CloseReason, WebSocketState};
use crate::error::{NetworkError, Result};
use crate::tls::TlsConfig;

/// Type alias for a connected WebSocket stream.
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Configuration for WebSocket connection.
#[derive(Clone, Debug)]
pub struct WebSocketConfig {
    /// The WebSocket URL (ws:// or wss://).
    pub url: String,
    /// Custom headers to send during the handshake.
    pub headers: HashMap<String, String>,
    /// Auto-reconnect configuration. If `None`, auto-reconnect is disabled.
    pub reconnect: Option<ReconnectConfig>,
    /// TLS configuration for secure connections (wss://).
    pub tls: Option<TlsConfig>,
}

impl WebSocketConfig {
    /// Create a new WebSocket configuration with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
            reconnect: None,
            tls: None,
        }
    }

    /// Add a custom header for the WebSocket handshake.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Add multiple headers.
    pub fn headers(mut self, headers: impl IntoIterator<Item = (String, String)>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Enable auto-reconnect with default settings.
    pub fn auto_reconnect(mut self) -> Self {
        self.reconnect = Some(ReconnectConfig::default());
        self
    }

    /// Enable auto-reconnect with custom configuration.
    pub fn reconnect_config(mut self, config: ReconnectConfig) -> Self {
        self.reconnect = Some(config);
        self
    }

    /// Set TLS configuration for secure connections.
    ///
    /// This allows custom CA certificates, client certificates (mTLS),
    /// and TLS version configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice_net::{WebSocketConfig, TlsConfig, Certificate};
    ///
    /// let ca_cert = Certificate::from_pem_file("/path/to/ca.crt")?;
    /// let config = WebSocketConfig::new("wss://example.com/ws")
    ///     .tls_config(TlsConfig::new().add_root_certificate(ca_cert));
    /// ```
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.tls = Some(config);
        self
    }

    /// Accept invalid TLS certificates (DANGEROUS - for testing only).
    ///
    /// # Warning
    ///
    /// This disables certificate verification and makes the connection
    /// vulnerable to man-in-the-middle attacks.
    pub fn danger_accept_invalid_certs(mut self) -> Self {
        let tls = self.tls.get_or_insert_with(TlsConfig::default);
        tls.danger_accept_invalid_certs = true;
        self
    }
}

/// Configuration for automatic reconnection.
#[derive(Clone, Debug)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts. `None` means infinite retries.
    pub max_attempts: Option<u32>,
    /// Initial delay between reconnection attempts.
    pub initial_delay: Duration,
    /// Maximum delay between reconnection attempts.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: None, // Infinite retries by default
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Create a new reconnect configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of reconnection attempts.
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = Some(attempts);
        self
    }

    /// Set the initial delay between reconnection attempts.
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set the maximum delay between reconnection attempts.
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set the backoff multiplier for exponential backoff.
    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate the delay for a given attempt number (0-indexed).
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay_ms = self.initial_delay.as_millis() as f64;
        let delay_ms = base_delay_ms * self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64) as u64;

        // Add jitter (±10%)
        let jitter_range = (delay_ms as f64 * 0.1) as u64;
        let jitter = if jitter_range > 0 {
            rand::random::<u64>() % (jitter_range * 2) - jitter_range
        } else {
            0
        };

        Duration::from_millis((delay_ms as i64 + jitter as i64).max(0) as u64)
    }
}

/// Internal state for the WebSocket connection.
struct WebSocketInner {
    state: WebSocketState,
    reconnect_attempt: u32,
}

/// Command sent to the WebSocket task.
enum Command {
    SendText(String),
    SendBinary(Vec<u8>),
    SendPing(Vec<u8>),
    Close(Option<CloseReason>),
}

/// A WebSocket client with signal-based event delivery.
///
/// The client manages a WebSocket connection and emits signals for
/// connection events and received messages. It supports:
///
/// - Secure connections (wss://)
/// - Custom headers for authentication
/// - Text and binary messages
/// - Automatic ping/pong handling
/// - Optional auto-reconnect with exponential backoff
///
/// # Signals
///
/// - [`connected`](Self::connected): Emitted when the connection is established
/// - [`disconnected`](Self::disconnected): Emitted when the connection is closed
/// - [`text_message_received`](Self::text_message_received): Emitted when a text message is received
/// - [`binary_message_received`](Self::binary_message_received): Emitted when a binary message is received
/// - [`error`](Self::error): Emitted when an error occurs
///
/// # Example
///
/// ```ignore
/// let config = WebSocketConfig::new("wss://echo.websocket.org")
///     .header("Authorization", "Bearer token")
///     .auto_reconnect();
///
/// let client = WebSocketClient::new(config);
///
/// client.connected.connect(|| println!("Connected!"));
/// client.text_message_received.connect(|msg| println!("Received: {}", msg));
///
/// client.connect();
/// client.send_text("Hello!");
/// ```
pub struct WebSocketClient {
    config: WebSocketConfig,
    inner: Arc<Mutex<WebSocketInner>>,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Command>>>>,
    is_running: Arc<AtomicBool>,

    /// Signal emitted when the connection is established.
    pub connected: Signal<()>,
    /// Signal emitted when the connection is closed.
    pub disconnected: Signal<()>,
    /// Signal emitted when a text message is received.
    pub text_message_received: Signal<String>,
    /// Signal emitted when a binary message is received.
    pub binary_message_received: Signal<Vec<u8>>,
    /// Signal emitted when an error occurs.
    pub error: Signal<NetworkError>,
}

impl WebSocketClient {
    /// Create a new WebSocket client with the given configuration.
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(WebSocketInner {
                state: WebSocketState::Disconnected,
                reconnect_attempt: 0,
            })),
            command_tx: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            connected: Signal::new(),
            disconnected: Signal::new(),
            text_message_received: Signal::new(),
            binary_message_received: Signal::new(),
            error: Signal::new(),
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> WebSocketState {
        self.inner.lock().state
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.inner.lock().state == WebSocketState::Connected
    }

    /// Connect to the WebSocket server.
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
        let text_ptr = &self.text_message_received as *const Signal<String> as usize;
        let binary_ptr = &self.binary_message_received as *const Signal<Vec<u8>> as usize;
        let error_ptr = &self.error as *const Signal<NetworkError> as usize;

        tokio::spawn(async move {
            // SAFETY: Signal pointers remain valid as long as WebSocketClient exists.
            // The is_running flag ensures we don't outlive the client.
            let emit_connected = || unsafe {
                let signal = &*(connected_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_disconnected = || unsafe {
                let signal = &*(disconnected_ptr as *const Signal<()>);
                signal.emit(());
            };
            let emit_text = |msg: String| unsafe {
                let signal = &*(text_ptr as *const Signal<String>);
                signal.emit(msg);
            };
            let emit_binary = |data: Vec<u8>| unsafe {
                let signal = &*(binary_ptr as *const Signal<Vec<u8>>);
                signal.emit(data);
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
                        WebSocketState::Reconnecting
                    } else {
                        WebSocketState::Connecting
                    };
                    state.reconnect_attempt = reconnect_attempt;
                }

                // Build the request with custom headers
                let request = match Self::build_request(&config) {
                    Ok(req) => req,
                    Err(e) => {
                        emit_error(e);
                        inner.lock().state = WebSocketState::Disconnected;
                        is_running.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                // Attempt to connect with optional custom TLS configuration
                let connect_result: std::result::Result<(WsStream, _), _> =
                    if let Some(ref tls_config) = config.tls {
                        // Build custom TLS connector
                        let rustls_config = if tls_config.danger_accept_invalid_certs {
                            match tls_config.build_dangerous_rustls_config() {
                                Ok(cfg) => cfg,
                                Err(e) => {
                                    emit_error(e);
                                    inner.lock().state = WebSocketState::Disconnected;
                                    is_running.store(false, Ordering::SeqCst);
                                    return;
                                }
                            }
                        } else {
                            match tls_config.build_rustls_config() {
                                Ok(cfg) => cfg,
                                Err(e) => {
                                    emit_error(e);
                                    inner.lock().state = WebSocketState::Disconnected;
                                    is_running.store(false, Ordering::SeqCst);
                                    return;
                                }
                            }
                        };
                        let connector = Connector::Rustls(rustls_config);
                        tokio_tungstenite::connect_async_tls_with_config(
                            request,
                            None,
                            false,
                            Some(connector),
                        )
                        .await
                    } else {
                        // Use default TLS handling
                        tokio_tungstenite::connect_async(request).await
                    };

                match connect_result {
                    Ok((ws_stream, _response)) => {
                        // Connection successful
                        reconnect_attempt = 0;
                        {
                            let mut state = inner.lock();
                            state.state = WebSocketState::Connected;
                            state.reconnect_attempt = 0;
                        }
                        emit_connected();

                        // Create command channel
                        let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
                        *command_tx.lock() = Some(tx);

                        // Split the stream
                        let (mut write, mut read) = ws_stream.split();

                        // Handle messages and commands
                        let mut closed_normally = false;
                        loop {
                            tokio::select! {
                                // Receive command from user
                                cmd = rx.recv() => {
                                    match cmd {
                                        Some(Command::SendText(text)) => {
                                            if let Err(e) = write.send(Message::Text(text.into())).await {
                                                emit_error(NetworkError::WebSocket(e.to_string()));
                                                break;
                                            }
                                        }
                                        Some(Command::SendBinary(data)) => {
                                            if let Err(e) = write.send(Message::Binary(data.into())).await {
                                                emit_error(NetworkError::WebSocket(e.to_string()));
                                                break;
                                            }
                                        }
                                        Some(Command::SendPing(data)) => {
                                            if let Err(e) = write.send(Message::Ping(data.into())).await {
                                                emit_error(NetworkError::WebSocket(e.to_string()));
                                                break;
                                            }
                                        }
                                        Some(Command::Close(reason)) => {
                                            let close_frame = reason.map(|r| CloseFrame {
                                                code: Self::to_tungstenite_close_code(r.code),
                                                reason: r.reason.unwrap_or_default().into(),
                                            });
                                            let _ = write.send(Message::Close(close_frame)).await;
                                            closed_normally = true;
                                            break;
                                        }
                                        None => {
                                            // Command channel closed, stop
                                            closed_normally = true;
                                            break;
                                        }
                                    }
                                }

                                // Receive message from server
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(text))) => {
                                            emit_text(text.to_string());
                                        }
                                        Some(Ok(Message::Binary(data))) => {
                                            emit_binary(data.to_vec());
                                        }
                                        Some(Ok(Message::Ping(_))) => {
                                            // Pong is sent automatically by tungstenite
                                        }
                                        Some(Ok(Message::Pong(_))) => {
                                            // Received pong response
                                        }
                                        Some(Ok(Message::Close(_frame))) => {
                                            // Server closed connection
                                            break;
                                        }
                                        Some(Ok(Message::Frame(_))) => {
                                            // Raw frame, ignore
                                        }
                                        Some(Err(e)) => {
                                            emit_error(NetworkError::WebSocket(e.to_string()));
                                            break;
                                        }
                                        None => {
                                            // Stream ended
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // Clear command channel
                        *command_tx.lock() = None;

                        // Update state
                        inner.lock().state = WebSocketState::Disconnected;
                        emit_disconnected();

                        // Check if we should reconnect
                        if closed_normally {
                            is_running.store(false, Ordering::SeqCst);
                            return;
                        }
                    }
                    Err(e) => {
                        emit_error(NetworkError::WebSocket(e.to_string()));
                    }
                }

                // Check if auto-reconnect is enabled
                let reconnect_config = match &config.reconnect {
                    Some(cfg) => cfg,
                    None => {
                        inner.lock().state = WebSocketState::Disconnected;
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
                        inner.lock().state = WebSocketState::Disconnected;
                        emit_disconnected();
                        is_running.store(false, Ordering::SeqCst);
                        return;
                    }
                }

                // Wait before reconnecting
                let delay = reconnect_config.delay_for_attempt(reconnect_attempt);
                inner.lock().state = WebSocketState::Reconnecting;
                tokio::time::sleep(delay).await;

                // Check if we were told to stop while sleeping
                if !is_running.load(Ordering::SeqCst) {
                    inner.lock().state = WebSocketState::Disconnected;
                    return;
                }

                reconnect_attempt += 1;
            }
        });
    }

    /// Disconnect from the WebSocket server.
    ///
    /// Sends a close frame with the given reason and shuts down the connection.
    pub fn disconnect(&self) {
        self.close(Some(CloseReason::normal()));
    }

    /// Close the connection with an optional close reason.
    pub fn close(&self, reason: Option<CloseReason>) {
        if let Some(tx) = self.command_tx.lock().as_ref() {
            let _ = tx.send(Command::Close(reason));
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Send a text message.
    ///
    /// Returns `Ok(())` if the message was queued for sending, or an error if not connected.
    pub fn send_text(&self, message: impl Into<String>) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::SendText(message.into()))
                    .map_err(|_| NetworkError::Connection("Not connected".into()))?;
                Ok(())
            }
            None => Err(NetworkError::Connection("Not connected".into())),
        }
    }

    /// Send a binary message.
    ///
    /// Returns `Ok(())` if the message was queued for sending, or an error if not connected.
    pub fn send_binary(&self, data: impl Into<Vec<u8>>) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::SendBinary(data.into()))
                    .map_err(|_| NetworkError::Connection("Not connected".into()))?;
                Ok(())
            }
            None => Err(NetworkError::Connection("Not connected".into())),
        }
    }

    /// Send a ping message with optional payload.
    ///
    /// The server will respond with a pong message (handled automatically by tungstenite).
    pub fn send_ping(&self, payload: impl Into<Vec<u8>>) -> Result<()> {
        let tx = self.command_tx.lock();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(Command::SendPing(payload.into()))
                    .map_err(|_| NetworkError::Connection("Not connected".into()))?;
                Ok(())
            }
            None => Err(NetworkError::Connection("Not connected".into())),
        }
    }

    /// Get the URL this client is configured to connect to.
    pub fn url(&self) -> &str {
        &self.config.url
    }

    /// Build the WebSocket request with custom headers.
    fn build_request(
        config: &WebSocketConfig,
    ) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
        let mut request = config
            .url
            .as_str()
            .into_client_request()
            .map_err(|e| NetworkError::WebSocket(e.to_string()))?;

        // Add custom headers
        let headers = request.headers_mut();
        for (name, value) in &config.headers {
            let header_name = http::header::HeaderName::try_from(name.as_str())
                .map_err(|e| NetworkError::InvalidHeader(e.to_string()))?;
            let header_value = http::header::HeaderValue::try_from(value.as_str())
                .map_err(|e| NetworkError::InvalidHeader(e.to_string()))?;
            headers.insert(header_name, header_value);
        }

        Ok(request)
    }

    /// Convert our CloseCode to tungstenite's CloseCode.
    fn to_tungstenite_close_code(code: CloseCode) -> TungsteniteCloseCode {
        match code {
            CloseCode::Normal => TungsteniteCloseCode::Normal,
            CloseCode::Away => TungsteniteCloseCode::Away,
            CloseCode::Protocol => TungsteniteCloseCode::Protocol,
            CloseCode::Unsupported => TungsteniteCloseCode::Unsupported,
            CloseCode::NoStatus => TungsteniteCloseCode::Status,
            CloseCode::Abnormal => TungsteniteCloseCode::Abnormal,
            CloseCode::Invalid => TungsteniteCloseCode::Invalid,
            CloseCode::Policy => TungsteniteCloseCode::Policy,
            CloseCode::TooBig => TungsteniteCloseCode::Size,
            CloseCode::Extension => TungsteniteCloseCode::Extension,
            CloseCode::Error => TungsteniteCloseCode::Error,
            CloseCode::Restart => TungsteniteCloseCode::Restart,
            CloseCode::Again => TungsteniteCloseCode::Again,
            CloseCode::Custom(code) => TungsteniteCloseCode::from(code),
        }
    }
}

impl std::fmt::Debug for WebSocketClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketClient")
            .field("url", &self.config.url)
            .field("state", &self.state())
            .finish()
    }
}
