//! Configuration types for UDP sockets.

use std::net::Ipv4Addr;

/// Configuration for a UDP socket.
#[derive(Clone, Debug)]
pub struct UdpSocketConfig {
    /// The address to bind to.
    pub bind_address: String,
    /// The port to bind to. Use 0 for an OS-assigned port.
    pub port: u16,
    /// Enable broadcast mode.
    pub broadcast: bool,
    /// Receive buffer size in bytes.
    pub recv_buffer_size: usize,
    /// Multicast configuration.
    pub multicast: MulticastConfig,
}

impl Default for UdpSocketConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".into(),
            port: 0,
            broadcast: false,
            recv_buffer_size: 65535,
            multicast: MulticastConfig::default(),
        }
    }
}

impl UdpSocketConfig {
    /// Create a new configuration that binds to the specified address and port.
    pub fn new(bind_address: impl Into<String>, port: u16) -> Self {
        Self {
            bind_address: bind_address.into(),
            port,
            ..Default::default()
        }
    }

    /// Create a configuration that binds to any address on the specified port.
    pub fn any_address(port: u16) -> Self {
        Self::new("0.0.0.0", port)
    }

    /// Enable broadcast mode.
    pub fn broadcast(mut self, enabled: bool) -> Self {
        self.broadcast = enabled;
        self
    }

    /// Set the receive buffer size.
    pub fn recv_buffer_size(mut self, size: usize) -> Self {
        self.recv_buffer_size = size;
        self
    }

    /// Set multicast configuration.
    pub fn multicast_config(mut self, config: MulticastConfig) -> Self {
        self.multicast = config;
        self
    }

    /// Get the bind address string (address:port).
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

/// Configuration for multicast sockets.
#[derive(Clone, Debug, Default)]
pub struct MulticastConfig {
    /// Groups to join on bind. Each entry is (multicast_addr, interface_addr).
    /// If interface_addr is None, uses INADDR_ANY.
    pub groups: Vec<(Ipv4Addr, Option<Ipv4Addr>)>,
    /// Whether to receive own multicast messages.
    pub loopback: bool,
    /// TTL for multicast packets.
    pub ttl: u32,
}

impl MulticastConfig {
    /// Create a new empty multicast configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a multicast group to join.
    pub fn join_group(mut self, multicast_addr: Ipv4Addr) -> Self {
        self.groups.push((multicast_addr, None));
        self
    }

    /// Add a multicast group with a specific interface.
    pub fn join_group_on(mut self, multicast_addr: Ipv4Addr, interface: Ipv4Addr) -> Self {
        self.groups.push((multicast_addr, Some(interface)));
        self
    }

    /// Enable or disable multicast loopback.
    pub fn loopback(mut self, enabled: bool) -> Self {
        self.loopback = enabled;
        self
    }

    /// Set the multicast TTL.
    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = ttl;
        self
    }
}

/// A received datagram with its source address.
#[derive(Clone, Debug)]
pub struct Datagram {
    /// The datagram payload.
    pub data: Vec<u8>,
    /// The source address of the datagram.
    pub source: std::net::SocketAddr,
}

impl Datagram {
    /// Create a new datagram.
    pub fn new(data: Vec<u8>, source: std::net::SocketAddr) -> Self {
        Self { data, source }
    }
}
