//! DNS resolution tests.

use std::net::IpAddr;
use std::time::Duration;

use horizon_lattice_net::dns::{DnsConfig, DnsResolver, IpStrategy};

#[tokio::test]
async fn test_system_resolver_creation() {
    // Should be able to create a system resolver
    let resolver = DnsResolver::system();
    assert!(resolver.is_ok(), "Failed to create system resolver");
}

#[tokio::test]
async fn test_google_resolver_creation() {
    let resolver = DnsResolver::google();
    assert!(resolver.is_ok(), "Failed to create Google DNS resolver");
}

#[tokio::test]
async fn test_cloudflare_resolver_creation() {
    let resolver = DnsResolver::cloudflare();
    assert!(resolver.is_ok(), "Failed to create Cloudflare DNS resolver");
}

#[tokio::test]
async fn test_resolve_localhost() {
    let resolver = DnsResolver::system().expect("Failed to create resolver");

    // localhost should always resolve
    let result = resolver.resolve("localhost").await;
    assert!(
        result.is_ok(),
        "Failed to resolve localhost: {:?}",
        result.err()
    );

    let addresses = result.unwrap();
    assert!(
        !addresses.is_empty(),
        "localhost should resolve to at least one address"
    );

    // localhost typically resolves to 127.0.0.1 or ::1
    let has_loopback = addresses.iter().any(|addr| {
        matches!(addr, IpAddr::V4(v4) if v4.is_loopback())
            || matches!(addr, IpAddr::V6(v6) if v6.is_loopback())
    });
    assert!(
        has_loopback,
        "localhost should resolve to a loopback address"
    );
}

#[tokio::test]
async fn test_config_builder() {
    let config = DnsConfig::cloudflare()
        .cache_size(512)
        .max_positive_ttl(Duration::from_secs(3600))
        .max_negative_ttl(Duration::from_secs(60))
        .ip_strategy(IpStrategy::Ipv4ThenIpv6)
        .attempts(3)
        .timeout(Duration::from_secs(10));

    assert_eq!(config.cache_size, 512);
    assert_eq!(config.max_positive_ttl, Duration::from_secs(3600));
    assert_eq!(config.max_negative_ttl, Duration::from_secs(60));
    assert_eq!(config.ip_strategy, IpStrategy::Ipv4ThenIpv6);
    assert_eq!(config.attempts, 3);
    assert_eq!(config.timeout, Duration::from_secs(10));

    // Should be able to create a resolver with custom config
    let resolver = DnsResolver::new(config);
    assert!(resolver.is_ok());
}

#[tokio::test]
async fn test_custom_nameservers() {
    let config = DnsConfig::with_nameservers(vec![
        "8.8.8.8:53".parse().unwrap(),
        "8.8.4.4:53".parse().unwrap(),
    ]);

    assert!(!config.use_system_config);
    assert_eq!(config.nameservers.len(), 2);

    let resolver = DnsResolver::new(config);
    assert!(resolver.is_ok());
}

#[tokio::test]
async fn test_empty_nameservers_error() {
    let config = DnsConfig::with_nameservers(vec![]);
    let resolver = DnsResolver::new(config);

    assert!(resolver.is_err(), "Should fail with empty nameservers");
}

#[tokio::test]
async fn test_clear_cache() {
    let resolver = DnsResolver::system().expect("Failed to create resolver");

    // This should not panic
    resolver.clear_cache();
}

#[tokio::test]
async fn test_ip_strategy_variants() {
    // Test all IP strategy variants
    let strategies = [
        IpStrategy::Ipv4Only,
        IpStrategy::Ipv6Only,
        IpStrategy::Ipv4ThenIpv6,
        IpStrategy::Ipv6ThenIpv4,
        IpStrategy::Ipv4AndIpv6,
    ];

    for strategy in strategies {
        let config = DnsConfig::system().ip_strategy(strategy);
        let resolver = DnsResolver::new(config);
        assert!(
            resolver.is_ok(),
            "Failed to create resolver with strategy {:?}",
            strategy
        );
    }
}
