//! TCP client and server with signal-based event delivery.
//!
//! This module provides TCP networking capabilities:
//! - **TcpClient**: Connect to TCP servers with auto-reconnect support
//! - **TcpServer**: Accept incoming TCP connections
//! - **TcpConnection**: Handle individual accepted connections
//!
//! # Client Example
//!
//! ```ignore
//! use horizon_lattice_net::tcp::{TcpClient, TcpClientConfig};
//!
//! let config = TcpClientConfig::new("127.0.0.1", 8080)
//!     .no_delay(true)
//!     .auto_reconnect();
//!
//! let client = TcpClient::new(config);
//!
//! // Connect to events
//! client.connected.connect(|| {
//!     println!("Connected to server!");
//! });
//!
//! client.data_received.connect(|data| {
//!     println!("Received {} bytes", data.len());
//! });
//!
//! // Connect and send data
//! client.connect();
//! client.send(b"Hello, Server!");
//! ```
//!
//! # Server Example
//!
//! ```ignore
//! use horizon_lattice_net::tcp::{TcpServer, TcpServerConfig};
//!
//! let config = TcpServerConfig::new("0.0.0.0", 8080);
//! let server = TcpServer::new(config);
//!
//! server.new_connection.connect(|conn| {
//!     println!("New connection from {:?}", conn.peer_addr());
//!
//!     conn.data_received.connect(|data| {
//!         println!("Received: {:?}", data);
//!     });
//! });
//!
//! server.start();
//! ```

mod client;
mod config;
mod connection;
mod server;
mod state;

pub use client::TcpClient;
pub use config::{TcpClientConfig, TcpServerConfig, TcpSocketConfig};
pub use connection::{ConnectionId, TcpConnection};
pub use server::TcpServer;
pub use state::{TcpConnectionState, TcpServerState};
