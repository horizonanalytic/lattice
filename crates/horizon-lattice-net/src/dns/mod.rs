//! DNS resolution module for Horizon Lattice.
//!
//! This module provides DNS resolution capabilities with built-in caching,
//! TTL respect, and both synchronous and signal-based async APIs.
//!
//! # Features
//!
//! - **Custom DNS resolution**: Hostname to IP resolution with async support
//! - **Built-in caching**: Automatic caching with TTL respect
//! - **Multiple address support**: Returns all resolved addresses for round-robin
//! - **Negative caching**: Caches NXDOMAIN responses to reduce unnecessary lookups
//! - **System configuration**: Can read system DNS settings automatically
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::dns::{DnsResolver, DnsConfig};
//!
//! // Create a resolver with system DNS settings
//! let resolver = DnsResolver::system()?;
//!
//! // Resolve a hostname
//! let addresses = resolver.resolve("example.com").await?;
//! println!("Resolved addresses: {:?}", addresses);
//!
//! // Use signal-based async for GUI integration
//! resolver.resolved.connect(|result| {
//!     println!("Got addresses for {}: {:?}", result.hostname, result.addresses);
//! });
//! resolver.lookup("api.example.com");
//! ```
//!
//! # Configuration
//!
//! ```ignore
//! use horizon_lattice_net::dns::{DnsConfig, IpStrategy};
//! use std::time::Duration;
//!
//! let config = DnsConfig::cloudflare()
//!     .cache_size(512)
//!     .max_positive_ttl(Duration::from_secs(3600))
//!     .ip_strategy(IpStrategy::Ipv4ThenIpv6);
//!
//! let resolver = DnsResolver::new(config)?;
//! ```

mod config;
mod resolver;

pub use config::{DnsConfig, IpStrategy};
pub use resolver::{DnsLookupResult, DnsResolver};
