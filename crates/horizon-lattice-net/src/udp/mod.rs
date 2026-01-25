//! UDP socket with signal-based event delivery.
//!
//! This module provides UDP networking capabilities:
//! - **UdpSocket**: Connectionless datagram communication
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::udp::{UdpSocket, UdpSocketConfig};
//!
//! let config = UdpSocketConfig::new("0.0.0.0", 8080);
//! let socket = UdpSocket::new(config);
//!
//! // Connect to events
//! socket.bound.connect(|addr| {
//!     println!("Socket bound to {}", addr);
//! });
//!
//! socket.datagram_received.connect(|datagram| {
//!     println!("Received {} bytes from {}", datagram.data.len(), datagram.source);
//! });
//!
//! // Bind and send data
//! socket.bind();
//! socket.send_to(b"Hello!", "127.0.0.1:9000".parse().unwrap());
//! ```
//!
//! # Broadcast Example
//!
//! ```ignore
//! let config = UdpSocketConfig::new("0.0.0.0", 0)
//!     .broadcast(true);
//!
//! let socket = UdpSocket::new(config);
//! socket.bind();
//!
//! // Send to broadcast address
//! socket.send_to(b"Discovery", "255.255.255.255:8080".parse().unwrap());
//! ```
//!
//! # Multicast Example
//!
//! ```ignore
//! use std::net::Ipv4Addr;
//! use horizon_lattice_net::udp::{UdpSocket, UdpSocketConfig, MulticastConfig};
//!
//! let multicast_group = "239.255.0.1".parse().unwrap();
//!
//! let config = UdpSocketConfig::new("0.0.0.0", 5000)
//!     .multicast_config(
//!         MulticastConfig::new()
//!             .join_group(multicast_group)
//!             .loopback(true)
//!             .ttl(1)
//!     );
//!
//! let socket = UdpSocket::new(config);
//! socket.bind();
//!
//! // Send to multicast group
//! socket.send_to(b"Multicast message", (multicast_group, 5000).into());
//! ```

mod config;
mod socket;
mod state;

pub use config::{Datagram, MulticastConfig, UdpSocketConfig};
pub use socket::UdpSocket;
pub use state::UdpSocketState;
