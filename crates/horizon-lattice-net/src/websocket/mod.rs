//! WebSocket client with real-time bidirectional communication.
//!
//! This module provides a WebSocket client that supports:
//! - Secure connections (ws:// and wss://)
//! - Custom headers for authentication
//! - Text and binary message handling
//! - Automatic ping/pong handling
//! - Optional auto-reconnect with exponential backoff
//! - Signal-based event delivery for GUI integration
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::websocket::{WebSocketClient, WebSocketConfig};
//!
//! let config = WebSocketConfig::new("wss://echo.websocket.org")
//!     .header("Authorization", "Bearer token");
//!
//! let client = WebSocketClient::new(config);
//!
//! // Connect to events
//! client.connected.connect(|| {
//!     println!("Connected to server!");
//! });
//!
//! client.text_message_received.connect(|message| {
//!     println!("Received: {}", message);
//! });
//!
//! client.disconnected.connect(|| {
//!     println!("Disconnected from server");
//! });
//!
//! // Connect and send messages
//! client.connect();
//! client.send_text("Hello, WebSocket!");
//! ```

mod client;
mod message;

pub use client::{ReconnectConfig, WebSocketClient, WebSocketConfig};
pub use message::{CloseCode, CloseReason, WebSocketState};
