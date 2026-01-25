//! Configuration types for TCP client and server.

use std::time::Duration;

use crate::websocket::ReconnectConfig;

/// Socket-level options for TCP connections.
#[derive(Clone, Debug)]
pub struct TcpSocketConfig {
    /// Enable TCP_NODELAY (disable Nagle's algorithm).
    pub no_delay: bool,
    /// Keep-alive interval. `None` disables keep-alive.
    pub keep_alive: Option<Duration>,
    /// Read buffer size in bytes.
    pub read_buffer_size: usize,
    /// Write buffer size in bytes.
    pub write_buffer_size: usize,
    /// Connection timeout.
    pub connect_timeout: Option<Duration>,
    /// Read timeout. `None` means no timeout.
    pub read_timeout: Option<Duration>,
    /// Write timeout. `None` means no timeout.
    pub write_timeout: Option<Duration>,
}

impl Default for TcpSocketConfig {
    fn default() -> Self {
        Self {
            no_delay: false,
            keep_alive: None,
            read_buffer_size: 8192,
            write_buffer_size: 8192,
            connect_timeout: Some(Duration::from_secs(30)),
            read_timeout: None,
            write_timeout: None,
        }
    }
}

impl TcpSocketConfig {
    /// Create a new socket configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable TCP_NODELAY.
    pub fn no_delay(mut self, enabled: bool) -> Self {
        self.no_delay = enabled;
        self
    }

    /// Set the keep-alive interval.
    pub fn keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive = Some(interval);
        self
    }

    /// Disable keep-alive.
    pub fn no_keep_alive(mut self) -> Self {
        self.keep_alive = None;
        self
    }

    /// Set the read buffer size.
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.read_buffer_size = size;
        self
    }

    /// Set the write buffer size.
    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.write_buffer_size = size;
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Disable connection timeout.
    pub fn no_connect_timeout(mut self) -> Self {
        self.connect_timeout = None;
        self
    }

    /// Set the read timeout.
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    /// Set the write timeout.
    pub fn write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = Some(timeout);
        self
    }
}

/// Configuration for a TCP client connection.
#[derive(Clone, Debug)]
pub struct TcpClientConfig {
    /// The host to connect to.
    pub host: String,
    /// The port to connect to.
    pub port: u16,
    /// Socket-level options.
    pub socket: TcpSocketConfig,
    /// Auto-reconnect configuration. If `None`, auto-reconnect is disabled.
    pub reconnect: Option<ReconnectConfig>,
}

impl TcpClientConfig {
    /// Create a new client configuration.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            socket: TcpSocketConfig::default(),
            reconnect: None,
        }
    }

    /// Set socket options.
    pub fn socket_config(mut self, config: TcpSocketConfig) -> Self {
        self.socket = config;
        self
    }

    /// Enable TCP_NODELAY.
    pub fn no_delay(mut self, enabled: bool) -> Self {
        self.socket.no_delay = enabled;
        self
    }

    /// Set keep-alive interval.
    pub fn keep_alive(mut self, interval: Duration) -> Self {
        self.socket.keep_alive = Some(interval);
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.socket.connect_timeout = Some(timeout);
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

    /// Get the address string (host:port).
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Configuration for a TCP server.
#[derive(Clone, Debug)]
pub struct TcpServerConfig {
    /// The address to bind to.
    pub bind_address: String,
    /// The port to listen on.
    pub port: u16,
    /// Socket-level options for accepted connections.
    pub socket: TcpSocketConfig,
    /// Connection backlog size.
    pub backlog: u32,
}

impl TcpServerConfig {
    /// Create a new server configuration.
    pub fn new(bind_address: impl Into<String>, port: u16) -> Self {
        Self {
            bind_address: bind_address.into(),
            port,
            socket: TcpSocketConfig::default(),
            backlog: 128,
        }
    }

    /// Set socket options for accepted connections.
    pub fn socket_config(mut self, config: TcpSocketConfig) -> Self {
        self.socket = config;
        self
    }

    /// Enable TCP_NODELAY for accepted connections.
    pub fn no_delay(mut self, enabled: bool) -> Self {
        self.socket.no_delay = enabled;
        self
    }

    /// Set the connection backlog size.
    pub fn backlog(mut self, size: u32) -> Self {
        self.backlog = size;
        self
    }

    /// Get the bind address string (address:port).
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}
