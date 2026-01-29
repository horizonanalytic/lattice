//! Network information module for Horizon Lattice.
//!
//! This module provides network interface enumeration and monitoring capabilities
//! with signal-based event delivery for GUI integration.
//!
//! # Features
//!
//! - **Interface enumeration**: List all network interfaces with their properties
//! - **Online/offline detection**: Check and monitor network connectivity state
//! - **Network change monitoring**: Watch for interface and address changes
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::network_info::{NetworkInterface, NetworkMonitor};
//!
//! // List all network interfaces
//! let interfaces = NetworkInterface::list();
//! for iface in &interfaces {
//!     println!("Interface: {} ({:?})", iface.name, iface.interface_type);
//!     for addr in &iface.ipv4_addresses {
//!         println!("  IPv4: {}/{}", addr.address, addr.prefix_len);
//!     }
//! }
//!
//! // Get the default interface used for internet traffic
//! if let Some(default) = NetworkInterface::default_interface() {
//!     println!("Default interface: {}", default.name);
//! }
//!
//! // Monitor for network changes
//! let monitor = NetworkMonitor::new()?;
//!
//! // Connect to online state changes
//! monitor.online_state_changed.connect(|is_online| {
//!     println!("Online state changed: {}", is_online);
//! });
//!
//! // Connect to interface changes
//! monitor.interface_changed.connect(|event| {
//!     println!("Added {} interfaces, removed {}",
//!         event.added.len(), event.removed.len());
//! });
//!
//! // Start monitoring
//! monitor.start()?;
//! ```

mod interface;
mod monitor;

pub use interface::{GatewayInfo, InterfaceType, Ipv4Info, Ipv6Info, MacAddress, NetworkInterface};

pub use monitor::{InterfaceChange, InterfaceEvent, NetworkMonitor, check_connectivity};
