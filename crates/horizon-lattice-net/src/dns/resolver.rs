//! DNS resolver implementation.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use hickory_resolver::config::{NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol;
use hickory_resolver::{Resolver, TokioResolver};
use horizon_lattice_core::signal::Signal;

use crate::dns::config::{DnsConfig, IpStrategy};
use crate::error::{NetworkError, Result};

/// Result of a DNS lookup.
#[derive(Debug, Clone)]
pub struct DnsLookupResult {
    /// The hostname that was resolved.
    pub hostname: String,
    /// The resolved IP addresses.
    pub addresses: Vec<IpAddr>,
    /// The time-to-live from the DNS response.
    pub ttl: Duration,
}

/// DNS resolver with caching support.
///
/// This resolver wraps the hickory-resolver library to provide DNS resolution
/// with built-in caching, TTL respect, and async support.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::dns::{DnsResolver, DnsConfig};
///
/// // Create a resolver with system DNS settings
/// let resolver = DnsResolver::system()?;
///
/// // Async resolution
/// let addresses = resolver.resolve("example.com").await?;
/// for addr in addresses {
///     println!("Resolved: {}", addr);
/// }
///
/// // Signal-based async lookup
/// resolver.resolved.connect(|result| {
///     println!("Resolved {}: {:?}", result.hostname, result.addresses);
/// });
/// resolver.lookup("example.com");
/// ```
pub struct DnsResolver {
    /// The underlying hickory resolver (handles caching internally).
    resolver: TokioResolver,

    /// Signal emitted when an async lookup completes successfully.
    pub resolved: Arc<Signal<DnsLookupResult>>,

    /// Signal emitted when an async lookup fails.
    /// The tuple contains (hostname, error).
    pub error: Arc<Signal<(String, NetworkError)>>,
}

impl DnsResolver {
    /// Create a new DNS resolver with the given configuration.
    pub fn new(config: DnsConfig) -> Result<Self> {
        let (resolver_config, resolver_opts) = build_resolver_config(&config)?;

        let resolver =
            Resolver::builder_with_config(resolver_config, TokioConnectionProvider::default())
                .with_options(resolver_opts)
                .build();

        Ok(Self {
            resolver,
            resolved: Arc::new(Signal::new()),
            error: Arc::new(Signal::new()),
        })
    }

    /// Create a DNS resolver using system DNS settings.
    ///
    /// On Unix, this reads `/etc/resolv.conf`.
    /// On Windows, this uses the system's configured DNS servers.
    pub fn system() -> Result<Self> {
        Self::new(DnsConfig::system())
    }

    /// Create a DNS resolver using Google's public DNS servers.
    pub fn google() -> Result<Self> {
        Self::new(DnsConfig::google())
    }

    /// Create a DNS resolver using Cloudflare's public DNS servers.
    pub fn cloudflare() -> Result<Self> {
        Self::new(DnsConfig::cloudflare())
    }

    /// Perform an async lookup and emit signals when complete.
    ///
    /// This method returns immediately. Results are delivered via the
    /// `resolved` signal on success or `error` signal on failure.
    pub fn lookup(&self, hostname: &str) {
        let hostname = hostname.to_string();
        let resolver = self.resolver.clone();
        let resolved_signal = Arc::clone(&self.resolved);
        let error_signal = Arc::clone(&self.error);

        tokio::spawn(async move {
            match resolve_hostname(&resolver, &hostname).await {
                Ok(result) => {
                    resolved_signal.emit(result);
                }
                Err(e) => {
                    error_signal.emit((hostname, e));
                }
            }
        });
    }

    /// Perform an async IPv4-only lookup and emit signals when complete.
    pub fn lookup_v4(&self, hostname: &str) {
        let hostname = hostname.to_string();
        let resolver = self.resolver.clone();
        let resolved_signal = Arc::clone(&self.resolved);
        let error_signal = Arc::clone(&self.error);

        tokio::spawn(async move {
            match resolve_v4(&resolver, &hostname).await {
                Ok(result) => {
                    resolved_signal.emit(result);
                }
                Err(e) => {
                    error_signal.emit((hostname, e));
                }
            }
        });
    }

    /// Perform an async IPv6-only lookup and emit signals when complete.
    pub fn lookup_v6(&self, hostname: &str) {
        let hostname = hostname.to_string();
        let resolver = self.resolver.clone();
        let resolved_signal = Arc::clone(&self.resolved);
        let error_signal = Arc::clone(&self.error);

        tokio::spawn(async move {
            match resolve_v6(&resolver, &hostname).await {
                Ok(result) => {
                    resolved_signal.emit(result);
                }
                Err(e) => {
                    error_signal.emit((hostname, e));
                }
            }
        });
    }

    /// Resolve a hostname to IP addresses.
    ///
    /// This is an async method that returns when resolution completes.
    pub async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        let result = resolve_hostname(&self.resolver, hostname).await?;
        Ok(result.addresses)
    }

    /// Resolve a hostname to IPv4 addresses only.
    pub async fn resolve_v4(&self, hostname: &str) -> Result<Vec<Ipv4Addr>> {
        let result = resolve_v4(&self.resolver, hostname).await?;
        Ok(result
            .addresses
            .into_iter()
            .filter_map(|addr| match addr {
                IpAddr::V4(v4) => Some(v4),
                IpAddr::V6(_) => None,
            })
            .collect())
    }

    /// Resolve a hostname to IPv6 addresses only.
    pub async fn resolve_v6(&self, hostname: &str) -> Result<Vec<Ipv6Addr>> {
        let result = resolve_v6(&self.resolver, hostname).await?;
        Ok(result
            .addresses
            .into_iter()
            .filter_map(|addr| match addr {
                IpAddr::V4(_) => None,
                IpAddr::V6(v6) => Some(v6),
            })
            .collect())
    }

    /// Clear the DNS cache.
    ///
    /// This removes all cached entries, forcing subsequent lookups
    /// to query DNS servers again.
    pub fn clear_cache(&self) {
        self.resolver.clear_cache();
    }
}

/// Build hickory resolver configuration from our DnsConfig.
fn build_resolver_config(config: &DnsConfig) -> Result<(ResolverConfig, ResolverOpts)> {
    let resolver_config = if config.use_system_config {
        // Use system configuration
        ResolverConfig::default()
    } else if config.nameservers.is_empty() {
        return Err(NetworkError::Dns("No nameservers configured".to_string()));
    } else {
        // Build custom configuration from nameservers
        let mut resolver_config = ResolverConfig::new();
        for addr in &config.nameservers {
            resolver_config.add_name_server(NameServerConfig::new(*addr, Protocol::Udp));
            resolver_config.add_name_server(NameServerConfig::new(*addr, Protocol::Tcp));
        }
        resolver_config
    };

    let mut opts = ResolverOpts::default();

    // Configure cache settings
    opts.cache_size = config.cache_size;
    opts.positive_max_ttl = Some(config.max_positive_ttl);
    opts.negative_max_ttl = Some(config.max_negative_ttl);

    // Configure lookup behavior
    opts.use_hosts_file = if config.use_hosts_file {
        ResolveHosts::Auto
    } else {
        ResolveHosts::Never
    };
    opts.attempts = config.attempts;
    opts.timeout = config.timeout;

    // Configure IP strategy
    opts.ip_strategy = match config.ip_strategy {
        IpStrategy::Ipv4Only => hickory_resolver::config::LookupIpStrategy::Ipv4Only,
        IpStrategy::Ipv6Only => hickory_resolver::config::LookupIpStrategy::Ipv6Only,
        IpStrategy::Ipv4ThenIpv6 => hickory_resolver::config::LookupIpStrategy::Ipv4thenIpv6,
        IpStrategy::Ipv6ThenIpv4 => hickory_resolver::config::LookupIpStrategy::Ipv6thenIpv4,
        IpStrategy::Ipv4AndIpv6 => hickory_resolver::config::LookupIpStrategy::Ipv4AndIpv6,
    };

    Ok((resolver_config, opts))
}

/// Resolve a hostname to IP addresses.
async fn resolve_hostname(resolver: &TokioResolver, hostname: &str) -> Result<DnsLookupResult> {
    let response = resolver
        .lookup_ip(hostname)
        .await
        .map_err(|e| NetworkError::Dns(e.to_string()))?;

    let addresses: Vec<IpAddr> = response.iter().collect();

    if addresses.is_empty() {
        return Err(NetworkError::Dns(format!(
            "No addresses found for hostname: {}",
            hostname
        )));
    }

    // Get the remaining TTL from the response
    let now = std::time::Instant::now();
    let valid_until = response.valid_until();
    let ttl = if valid_until > now {
        valid_until.duration_since(now)
    } else {
        Duration::ZERO
    };

    Ok(DnsLookupResult {
        hostname: hostname.to_string(),
        addresses,
        ttl,
    })
}

/// Resolve a hostname to IPv4 addresses only.
async fn resolve_v4(resolver: &TokioResolver, hostname: &str) -> Result<DnsLookupResult> {
    let response = resolver
        .ipv4_lookup(hostname)
        .await
        .map_err(|e| NetworkError::Dns(e.to_string()))?;

    let addresses: Vec<IpAddr> = response.iter().map(|r| IpAddr::V4(r.0)).collect();

    if addresses.is_empty() {
        return Err(NetworkError::Dns(format!(
            "No IPv4 addresses found for hostname: {}",
            hostname
        )));
    }

    let now = std::time::Instant::now();
    let valid_until = response.valid_until();
    let ttl = if valid_until > now {
        valid_until.duration_since(now)
    } else {
        Duration::ZERO
    };

    Ok(DnsLookupResult {
        hostname: hostname.to_string(),
        addresses,
        ttl,
    })
}

/// Resolve a hostname to IPv6 addresses only.
async fn resolve_v6(resolver: &TokioResolver, hostname: &str) -> Result<DnsLookupResult> {
    let response = resolver
        .ipv6_lookup(hostname)
        .await
        .map_err(|e| NetworkError::Dns(e.to_string()))?;

    let addresses: Vec<IpAddr> = response.iter().map(|r| IpAddr::V6(r.0)).collect();

    if addresses.is_empty() {
        return Err(NetworkError::Dns(format!(
            "No IPv6 addresses found for hostname: {}",
            hostname
        )));
    }

    let now = std::time::Instant::now();
    let valid_until = response.valid_until();
    let ttl = if valid_until > now {
        valid_until.duration_since(now)
    } else {
        Duration::ZERO
    };

    Ok(DnsLookupResult {
        hostname: hostname.to_string(),
        addresses,
        ttl,
    })
}
