//! DNS configuration types.

use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for DNS resolution.
#[derive(Debug, Clone)]
pub struct DnsConfig {
    /// Use system DNS configuration (reads /etc/resolv.conf on Unix).
    /// If false, uses custom nameservers.
    pub use_system_config: bool,

    /// Custom nameservers to use when `use_system_config` is false.
    /// Format: IP:port (e.g., "8.8.8.8:53")
    pub nameservers: Vec<SocketAddr>,

    /// Maximum number of cached entries.
    pub cache_size: usize,

    /// Maximum TTL for positive responses (caps the TTL from DNS records).
    pub max_positive_ttl: Duration,

    /// Maximum TTL for negative responses (NXDOMAIN).
    pub max_negative_ttl: Duration,

    /// Whether to read from /etc/hosts file.
    pub use_hosts_file: bool,

    /// IP version preference for lookups.
    pub ip_strategy: IpStrategy,

    /// Number of retries for failed lookups.
    pub attempts: usize,

    /// Timeout for each DNS query attempt.
    pub timeout: Duration,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            use_system_config: true,
            nameservers: Vec::new(),
            cache_size: 256,
            max_positive_ttl: Duration::from_secs(86400), // 24 hours
            max_negative_ttl: Duration::from_secs(300),   // 5 minutes
            use_hosts_file: true,
            ip_strategy: IpStrategy::default(),
            attempts: 2,
            timeout: Duration::from_secs(5),
        }
    }
}

impl DnsConfig {
    /// Create a new DNS configuration with system defaults.
    pub fn system() -> Self {
        Self::default()
    }

    /// Create a configuration with custom nameservers.
    pub fn with_nameservers(nameservers: Vec<SocketAddr>) -> Self {
        Self {
            use_system_config: false,
            nameservers,
            ..Default::default()
        }
    }

    /// Use Google's public DNS servers.
    pub fn google() -> Self {
        Self::with_nameservers(vec![
            "8.8.8.8:53".parse().unwrap(),
            "8.8.4.4:53".parse().unwrap(),
        ])
    }

    /// Use Cloudflare's public DNS servers.
    pub fn cloudflare() -> Self {
        Self::with_nameservers(vec![
            "1.1.1.1:53".parse().unwrap(),
            "1.0.0.1:53".parse().unwrap(),
        ])
    }

    /// Set the cache size.
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Set the maximum positive TTL.
    pub fn max_positive_ttl(mut self, ttl: Duration) -> Self {
        self.max_positive_ttl = ttl;
        self
    }

    /// Set the maximum negative TTL.
    pub fn max_negative_ttl(mut self, ttl: Duration) -> Self {
        self.max_negative_ttl = ttl;
        self
    }

    /// Set whether to use the hosts file.
    pub fn use_hosts_file(mut self, use_hosts: bool) -> Self {
        self.use_hosts_file = use_hosts;
        self
    }

    /// Set the IP strategy.
    pub fn ip_strategy(mut self, strategy: IpStrategy) -> Self {
        self.ip_strategy = strategy;
        self
    }

    /// Set the number of retry attempts.
    pub fn attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts;
        self
    }

    /// Set the timeout per attempt.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// IP version lookup strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IpStrategy {
    /// Look up IPv4 addresses only.
    Ipv4Only,
    /// Look up IPv6 addresses only.
    Ipv6Only,
    /// Look up both IPv4 and IPv6, prefer IPv4.
    #[default]
    Ipv4ThenIpv6,
    /// Look up both IPv4 and IPv6, prefer IPv6.
    Ipv6ThenIpv4,
    /// Look up both IPv4 and IPv6 in parallel.
    Ipv4AndIpv6,
}
